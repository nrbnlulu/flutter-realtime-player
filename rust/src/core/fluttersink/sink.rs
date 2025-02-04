use crate::core::fluttersink::gltexture::GLTexture;
use crate::core::fluttersink::utils;
use crate::core::gl::TEXTURE;
use crate::core::platform::{LinuxNativeTexture, NativeFrameType, GL_MANAGER};

use super::frame::{GstMappedFrame, ResolvedFrame, TextureCacheId, VideoInfo};
use super::utils::LogErr;
use super::{types, FrameSender, SinkEvent};

use gst::prelude::*;
use gst_gl::prelude::{ContextGLExt, GLContextExt};
use irondash_texture::BoxedGLTexture;
use log::{debug, error, trace, warn};

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
    fl_engine_handle: i64,
    sendable_txt: ArcSendableTexture,
}

impl FlutterConfig {
    pub(crate) fn new(
        fl_txt_id: i64,
        fl_engine_handle: i64,
        sendable_txt: ArcSendableTexture,
    ) -> Self {
        FlutterConfig {
            fl_txt_id,
            fl_engine_handle,
            sendable_txt,
        }
    }
}

pub type ArcSendableTexture =
    Arc<irondash_texture::SendableTexture<irondash_texture::BoxedGLTexture>>;

pub struct FlutterTextureSink {
    appsink: gst_app::AppSink,
    glsink: gst::Element,
    next_frame: Arc<Mutex<Option<NativeFrameType>>>,
    cached_textures: Mutex<HashMap<TextureCacheId, NativeFrameType>>,
}

impl FlutterTextureSink {
    pub fn new() -> Self {
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
            next_frame: Default::default(),
            cached_textures: Default::default(),
        }
    }
    pub fn video_sink(&self) -> gst::Element {
        self.glsink.clone().into()
    }

    pub fn connect(&self, bus: &gst::Bus, fl_config: FlutterConfig) {
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

        let sendable_txt_clone = fl_config.sendable_txt.clone();
        let next_frame_ref = self.next_frame.clone();

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
                        if let Ok(native_frame) = LinuxNativeTexture::from_gst(frame) {
                            let next_frame_ref = next_frame_ref.clone();
                            *next_frame_ref.lock().unwrap() = Some(native_frame);
                            // mark the frame as available before sending it to the main thread
                            sendable_txt_clone.mark_frame_available();
                        }
                    }

                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );
    }

    fn get_current_frame_callback(&self) -> anyhow::Result<BoxedGLTexture> {
        trace!("on get_current_frame_callback");
        let curr_frame = self.next_frame.lock().unwrap();
        curr_frame
            .as_ref()
            .map(|texture| texture.as_texture_provider())
            .or(Some(self.get_fallback_texture()))
            .ok_or(anyhow::anyhow!("coudln't get texture"))
    }
    fn get_fallback_texture(&self) -> BoxedGLTexture {
        unimplemented!("fallback texture")
    }
}

unsafe impl Sync for FlutterTextureSink {}
unsafe impl Send for FlutterTextureSink {}

impl irondash_texture::PayloadProvider<BoxedGLTexture> for FlutterTextureSink {
    fn get_payload(&self) -> BoxedGLTexture {
        self.get_current_frame_callback().log_err().unwrap()
    }
}
