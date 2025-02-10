use crate::core::fluttersink::gltexture::GLTexture;
use crate::core::fluttersink::utils;
use crate::core::gl::TEXTURE;
use crate::core::platform::{LinuxNativeTexture, NativeFrameType, GL_MANAGER};

use super::frame::{GstMappedFrame, ResolvedFrame, TextureCacheId, VideoInfo};
use super::utils::LogErr;
use super::{types, FrameSender, SinkEvent};

use gdkx11::x11::xlib::Atom;
use gst::{prelude::*, PadProbeReturn, PadProbeType, QueryViewMut};
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
    initialized_signal: Arc<AtomicBool>,
    gst_display: Mutex<Option<gst_gl::GLDisplay>>,
    gst_context: Mutex<Option<gst_gl::GLContext>>,
}

impl FlutterTextureSink {
    pub fn new(initialized_signal: Arc<AtomicBool>) -> Self {
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
            initialized_signal,
            gst_context: Default::default(),
            gst_display: Default::default(),
        }
    }
    pub fn video_sink(&self) -> gst::Element {
        self.glsink.clone().into()
    }

    pub fn connect(&self, bus: &gst::Bus, fl_config: FlutterConfig) {
        let (gst_gl_display, shared_context) =
            GL_MANAGER.with_borrow(|manager| manager.get_context(fl_config.fl_engine_handle).unwrap());
        self.gst_display
            .lock()
            .unwrap()
            .replace(gst_gl_display.clone());
        self.gst_context
            .lock()
            .unwrap()
            .replace(shared_context.clone());

        self.appsink
            .static_pad("sink")
            .unwrap()
            .add_probe(PadProbeType::QUERY_DOWNSTREAM, move |pad, probe_info| {
                if let Some(q) = probe_info.query_mut() {
                    if let QueryViewMut::Context(cq) = q.view_mut() {
                        trace!("Setting GL context for appsink");

                        if gst_gl::functions::gl_handle_context_query(
                            &pad.parent_element().unwrap(),
                            cq,
                            Some(&gst_gl_display),
                            Some(&shared_context),
                            None::<&gst_gl::GLContext>,
                        ) {
                            return PadProbeReturn::Handled;
                        }
                    };
                }
                PadProbeReturn::Ok
            })
            .unwrap();
        // bus.set_sync_handler({
        //     move |_, msg| {
        //         match msg.view() {
        //              => trace!("Message: {:?}", msg),
        //         }

        //         gst::BusSyncReply::Drop
        //     }
        // });

        let sendable_txt_clone = fl_config.sendable_txt.clone();
        let next_frame_ref = self.next_frame.clone();

        let initialized_sig_ref = self.initialized_signal.clone();

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
                            trace!("Got a frame");
                            let next_frame_ref = next_frame_ref.clone();
                            *next_frame_ref.lock().unwrap() = Some(native_frame);
                            // mark the frame as available before sending it to the main thread
                            sendable_txt_clone.mark_frame_available();
                            // if not initialized yet, mark it as initialized
                            if !initialized_sig_ref.load(atomic::Ordering::Relaxed) {
                                initialized_sig_ref.store(true, atomic::Ordering::Relaxed);
                            }
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
