use crate::core::fluttersink::utils;
use crate::core::platform::{LinuxNativeTexture, NativeFrameType, GL_MANAGER};

use super::frame::{GstMappedFrame, ResolvedFrame, TextureCacheId, VideoInfo};
use super::utils::LogErr;
use super::{types, FrameSender, SinkEvent};

use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::*;
use gst_base::subclass::prelude::*;
use gst_gl::prelude::{ContextGLExt, GLContextExt};
use gst_video::subclass::prelude::*;
use gst_video::subclass::prelude::*;
use irondash_texture::BoxedGLTexture;
use log::{debug, error, trace, warn};

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{
    atomic::{self, AtomicBool},
    Mutex,
};
use std::sync::{Arc, LazyLock};

pub(crate) static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "fluttertexturesink",
        gst::DebugColorFlags::empty(),
        Some("Flutter texture sink"),
    )
});

struct StreamConfig {
    info: Option<super::frame::VideoInfo>,
    /// Orientation from a global scope tag
    global_orientation: types::Orientation,
    /// Orientation from a stream scope tag
    stream_orientation: Option<types::Orientation>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            info: None,
            global_orientation: types::Orientation::Rotate0,
            stream_orientation: None,
        }
    }
}

pub(crate) struct FlutterConfig {
    fl_txt_id: i64,
    frame_sender: FrameSender,
    fl_engine_handle: i64,
    sendable_txt: ArcSendableTexture,
}

impl FlutterConfig {
    pub(crate) fn new(
        fl_txt_id: i64,
        fl_engine_handle: i64,
        frame_sender: FrameSender,
        sendable_txt: ArcSendableTexture,
    ) -> Self {
        FlutterConfig {
            fl_txt_id,
            fl_engine_handle,
            frame_sender,
            sendable_txt,
        }
    }
}

pub type ArcSendableTexture =
    Arc<irondash_texture::SendableTexture<irondash_texture::BoxedGLTexture>>;

pub struct FlutterTextureSink {
    appsink: gst_app::AppSink,
    glsink: gst::Element,
    texture_rx: flume::Receiver<SinkEvent>,
    texture_tx: flume::Sender<SinkEvent>,
    cached_textures: Mutex<HashMap<TextureCacheId, NativeFrameType>>,
}

impl FlutterTextureSink {
    pub fn new(
        texture_rx: flume::Receiver<SinkEvent>,
        texture_tx: flume::Sender<SinkEvent>,
    ) -> Self {
        let appsink = gst_app::AppSink::builder()
            .caps(
                &gst_video::VideoCapsBuilder::new()
                    .features([gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY])
                    .format(gst_video::VideoFormat::Rgba)
                    .field("texture-target", "2D")
                    .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
                    .build(),
            )
            .enable_last_sample(false)
            .max_buffers(1u32)
            .build();

        let glsink = gst::ElementFactory::make("glsinkbin")
            .property("sink", &appsink)
            .build()
            .expect("Fatal: Unable to create glsink");

        Self {
            appsink: appsink.upcast(),
            glsink,
            texture_rx,
            texture_tx,
            cached_textures: Default::default(),
        }
    }
    pub fn video_sink(&self) -> gst::Element {
        self.glsink.clone().into()
    }

    pub fn connect(&mut self, bus: &gst::Bus, fl_config: FlutterConfig) {
        let (gst_gl_display, gst_gl_context) = GL_MANAGER
            .with_borrow(|manager| manager.get_context(fl_config.fl_engine_handle))
            .unwrap();

        gst_gl_context
            .activate(true)
            .expect("could not activate GStreamer GL context");
        gst_gl_context
            .fill_info()
            .expect("failed to fill GL info for wrapped context");

        bus.set_sync_handler({
            let gst_gl_context = gst_gl_context.clone();
            move |_, msg| {
                match msg.view() {
                    gst::MessageView::NeedContext(ctx) => {
                        let ctx_type = ctx.context_type();
                        if ctx_type == *gst_gl::GL_DISPLAY_CONTEXT_TYPE {
                            if let Some(element) = msg
                                .src()
                                .and_then(|source| source.downcast_ref::<gst::Element>())
                            {
                                let gst_context = gst::Context::new(ctx_type, true);
                                gst_context.set_gl_display(&gst_gl_display);
                                element.set_context(&gst_context);
                            }
                        } else if ctx_type == "gst.gl.app_context" {
                            if let Some(element) = msg
                                .src()
                                .and_then(|source| source.downcast_ref::<gst::Element>())
                            {
                                let mut gst_context = gst::Context::new(ctx_type, true);
                                {
                                    let gst_context = gst_context.get_mut().unwrap();
                                    let structure = gst_context.structure_mut();
                                    structure.set("context", &gst_gl_context);
                                }
                                element.set_context(&gst_context);
                            }
                        }
                    }
                    _ => (),
                }

                gst::BusSyncReply::Drop
            }
        });

        self.appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |appsink| {
                    let sample = appsink
                        .pull_sample()
                        .map_err(|_| gst::FlowError::Flushing)?;

                    let mut buffer = sample.buffer_owned().unwrap();
                    {
                        let context = match (buffer.n_memory() > 0)
                            .then(|| buffer.peek_memory(0))
                            .and_then(|m| m.downcast_memory_ref::<gst_gl::GLBaseMemory>())
                            .map(|m| m.context())
                        {
                            Some(context) => context.clone(),
                            None => {
                                eprintln!("Got non-GL memory");
                                return Err(gst::FlowError::Error);
                            }
                        };

                        // Sync point to ensure that the rendering in this context will be complete by the time the
                        // Slint created GL context needs to access the texture.
                        if let Some(meta) = buffer.meta::<gst_gl::GLSyncMeta>() {
                            meta.set_sync_point(&context);
                        } else {
                            let buffer = buffer.make_mut();
                            let meta = gst_gl::GLSyncMeta::add(buffer, &context);
                            meta.set_sync_point(&context);
                        }
                    }

                    let Some(info) = sample
                        .caps()
                        .and_then(|caps| gst_video::VideoInfo::from_caps(caps).ok())
                    else {
                        error!("Got invalid caps");
                        return Err(gst::FlowError::NotNegotiated);
                    };
                    if let Ok(frame) = gst_gl::GLVideoFrame::from_buffer_readable(buffer, &info) {
                        // mark the frame as available before sending it to the main thread
                        fl_config.sendable_txt.mark_frame_available();

                        fl_config
                            .frame_sender
                            .send(SinkEvent::FrameChanged(ResolvedFrame::GL(
                                LinuxNativeTexture::from_gst(frame).log_err().unwrap(),
                            )))
                            .log_err();
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
    }

    fn get_current_frame_callback(&self) -> anyhow::Result<BoxedGLTexture> {
        trace!("Waiting for frame");
        match self
            .texture_receiver
            .recv_timeout(std::time::Duration::from_millis(10))
        {
            Ok(SinkEvent::FrameChanged(resolved_frame)) => match resolved_frame {
                ResolvedFrame::Memory(_) => unimplemented!("Memory"),
                ResolvedFrame::GL((egl_image, pixel_res)) => {
                    let mut texture_name = 0;
                    unsafe {
                        GL.GenTextures(1, &mut texture_name);

                        GL.BindTexture(gl::TEXTURE_2D, texture_name);
                        GL.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                        GL.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                        GL.EGLImageTargetTexture2DOES(
                            gl::TEXTURE_2D,
                            egl_image.texture_id.get_image(),
                        );
                    };
                    Ok(Box::new(GLTexture::new(
                        texture_name,
                        egl_image.width as i32,
                        egl_image.height as i32,
                    )))
                }
            },
            Err(e) => Err(anyhow::anyhow!(
                "Error receiving frame changed event {:?}",
                e
            )),
        }
        self.get_fallback_texture()
    }
    fn get_fallback_texture(&self) -> BoxedGLTexture {
        let tx_name = GL_MANAGER.with(|p| p.borrow().get_fallback_texture_name(self.engine_id));
        Box::new(GLTexture::new(tx_name, self.width, self.height))
    }
}

unsafe impl Sync for FlutterTextureSink {}
unsafe impl Send for FlutterTextureSink {}

impl irondash_texture::PayloadProvider<BoxedGLTexture> for FlutterTextureSink {
    fn get_payload(&self) -> BoxedGLTexture {
        self.get_current_frame_callback().log_err().unwrap()
    }
}
