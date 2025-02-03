use crate::core::fluttersink::utils;
use crate::core::platform::{self, GstNativeFrameType, GL_MANAGER};

use super::frame::{MappedFrame, ResolvedFrame, TextureCacheId, VideoInfo};
use super::gltexture::{GLTexture, GLTextureSource};
use super::utils::{invoke_on_gs_main_thread, make_element};
use super::{frame, types, FrameSender, SinkEvent};

use glib::clone::Downgrade;
use glib::thread_guard::ThreadGuard;

use glib::translate::FromGlibPtrFull;
use gst::Caps;
use gst::{prelude::*, subclass::prelude::*};
use gst_base::subclass::prelude::*;
use gst_gl::prelude::{GLContextExt as _, *};
use gst_video::ffi::GST_VIDEO_SIZE_RANGE;
use gst_video::subclass::prelude::*;
use log::{debug, error, trace, warn};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;
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

#[derive(Default)]
pub struct FlutterTextureSink {
    config: Mutex<StreamConfig>,
    fl_config: RefCell<Option<FlutterConfig>>,
    cached_textures: Mutex<HashMap<TextureCacheId, GstNativeFrameType>>,
    cached_caps: Mutex<Option<gst::Caps>>,
    settings: Mutex<Settings>,
    window_resized: AtomicBool,
    wrapped_gl_ctx: RefCell<Option<gst_gl::GLContext>>,
}

#[derive(Default)]
struct Settings {
    window_width: u32,
    window_height: u32,
}

impl Drop for FlutterTextureSink {
    fn drop(&mut self) {}
}

#[glib::object_subclass]
impl ObjectSubclass for FlutterTextureSink {
    const NAME: &'static str = "FlutterTextureSink";
    type Type = super::FlutterTextureSink;
    type ParentType = gst_video::VideoSink;
}

impl ObjectImpl for FlutterTextureSink {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecUInt::builder("window-width")
                    .nick("Window width")
                    .blurb("the width of the main widget rendering the paintable")
                    .mutable_playing()
                    .build(),
                glib::ParamSpecUInt::builder("window-height")
                    .nick("Window height")
                    .blurb("the height of the main widget rendering the paintable")
                    .mutable_playing()
                    .build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "window-width" => {
                let settings = self.settings.lock().unwrap();
                settings.window_width.to_value()
            }
            "window-height" => {
                let settings = self.settings.lock().unwrap();
                settings.window_height.to_value()
            }
            _ => unimplemented!(),
        }
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "window-width" => {
                let mut settings = self.settings.lock().unwrap();
                let value = value.get().expect("type checked upstream");
                if settings.window_width != value {
                    self.window_resized.store(true, atomic::Ordering::SeqCst);
                }
                settings.window_width = value;
            }
            "window-height" => {
                let mut settings = self.settings.lock().unwrap();
                let value = value.get().expect("type checked upstream");
                if settings.window_height != value {
                    self.window_resized.store(true, atomic::Ordering::SeqCst);
                }
                settings.window_height = value;
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for FlutterTextureSink {}

unsafe impl Send for FlutterTextureSink {}
unsafe impl Sync for FlutterTextureSink {}

impl ElementImpl for FlutterTextureSink {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Flutter texture sink",
                "Sink/Video",
                "A Flutter texture sink",
                "Nir Benlulu <nrbnlulu@gmail.com>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps =  &gst_video::VideoCapsBuilder::new()
            .features([gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY])
            .format(gst_video::VideoFormat::Rgba)
            .field("texture-target", "2D")
            .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
            .build();
            debug!("caps: {:?}", caps);
            vec![gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap()]
        });

        PAD_TEMPLATES.as_ref()
    }

    #[allow(clippy::single_match)]
    fn change_state(
        &self,
        transition: gst::StateChange,
    ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        match transition {
            gst::StateChange::NullToReady => {
                // Notify the pipeline about the GL display and wrapped context so that any other
                // elements in the pipeline ideally use the same / create GL contexts that are
                // sharing with this one.
                debug!("Advertising GL context to gstreamer pipeline");

                let fl_config = self.fl_config.borrow();
                let engine_id = fl_config.as_ref().unwrap().fl_engine_handle;
                drop(fl_config);
                let gl_ctx = utils::invoke_on_gs_main_thread(move || {
                    GL_MANAGER.with_borrow(|manager| manager.get_context(engine_id))
                })
                .unwrap();
                trace!("GL context: {:?}", gl_ctx);
                self.wrapped_gl_ctx.borrow_mut().replace(gl_ctx.clone());

                // let display = display.clone();

                // gst_gl::gl_element_propagate_display_context(&*self.obj(), &display);
                let mut ctx = gst::Context::new("gst.gl.app_context", true);
                {
                    let ctx = ctx.get_mut().unwrap();
                    ctx.structure_mut().set("context", gl_ctx);
                }
                let _ = self.obj().post_message(
                    gst::message::HaveContext::builder(ctx)
                        .src(&*self.obj())
                        .build(),
                );
                trace!("GL context advertised to gstreamer pipeline");
            }
            _ => (),
        }

        let res = self.parent_change_state(transition);

        trace!("transition changed: {:?}", transition);

        res
    }
}

impl BaseSinkImpl for FlutterTextureSink {
    fn set_caps(&self, caps: &gst::Caps) -> Result<(), gst::LoggableError> {
        trace!("set_caps");
        #[allow(unused_mut)]
        let mut video_info = None;

        let video_info = match video_info {
            Some(info) => info,
            None => gst_video::VideoInfo::from_caps(caps)
                .map_err(|_| gst::loggable_error!(CAT, "Invalid caps"))?
                .into(),
        };

        self.config.lock().unwrap().info = Some(video_info);

        Ok(())
    }

    fn event(&self, event: gst::Event) -> bool {
        match event.view() {
            gst::EventView::StreamStart(_) => {
                trace!("Stream start");
                let mut config = self.config.lock().unwrap();
                config.global_orientation = types::Orientation::Rotate0;
                config.stream_orientation = None;
            }
            gst::EventView::Tag(ev) => {
                trace!("Tag event {:?}", ev.tag());
                let mut config = self.config.lock().unwrap();
                let tags = ev.tag();
                let scope = tags.scope();
                let orientation = types::Orientation::from_tags(tags);

                if scope == gst::TagScope::Global {
                    config.global_orientation = orientation.unwrap_or(types::Orientation::Rotate0);
                } else {
                    config.stream_orientation = orientation;
                }
            }
            _ => (),
        }

        self.parent_event(event)
    }
}

impl VideoSinkImpl for FlutterTextureSink {
    /// if new frame could be rendered, send a message to the main thread
    fn show_frame(&self, buffer: &gst::Buffer) -> Result<gst::FlowSuccess, gst::FlowError> {
        trace!("show_frame");
        if self.window_resized.swap(false, atomic::Ordering::SeqCst) {
            let obj = self.obj();
            let sink = obj.sink_pad();
            sink.push_event(gst::event::Reconfigure::builder().build());
        }

        // Empty buffer, nothing to render
        if buffer.n_memory() == 0 {
            return Ok(gst::FlowSuccess::Ok);
        };
        let config = self.config.lock().unwrap();
        let info = config.info.as_ref().ok_or_else(|| {
            error!("Received no caps yet");
            gst::FlowError::NotNegotiated
        })?;
        if info.format() != gst_video::VideoFormat::Rgba {
            unimplemented!("Unsupported format: {:?}", info.format());
            // let sample = self
            //     .playbin3
            //     .borrow()
            //     .as_ref()
            //     .unwrap()
            //     .emit_by_name::<gst::Sample>("convert-sample", &[&self.caps()]);

            // return self.show_frame_from_buffer(&config, &sample.buffer_owned().unwrap(), info);
        }
        self.show_frame_from_buffer(&config, buffer, info)
    }
}

impl FlutterTextureSink {
    fn show_frame_from_buffer(
        &self,
        config: &StreamConfig,
        buffer: &gst::Buffer,
        info: &VideoInfo,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let orientation = config
            .stream_orientation
            .unwrap_or(config.global_orientation);
        let binding = self.wrapped_gl_ctx.borrow();
        let wrapped_context = binding.as_ref().ok_or_else(|| {
            error!("has no GL context");
            gst::FlowError::Error
        })?;
        let frame = MappedFrame::from_gst_buffer(
            &buffer,
            info,
            orientation,
            Some(wrapped_context.as_ref()),
        )
        .inspect_err(|_err| {
            error!("Failed to create frame from buffer");
        })?;
        let cached_textures = &mut self.cached_textures.lock().unwrap();
        let resolved_frame =
            ResolvedFrame::from_mapped_frame(&frame, &wrapped_context, cached_textures).map_err(
                |err| {
                    error!("Failed to resolve frame: {:?}", err);
                    gst::FlowError::Error
                },
            )?;

        let sender = self
            .fl_config
            .borrow()
            .as_ref()
            .map(|wrapper| wrapper.frame_sender.clone());
        let sender = sender.as_ref().ok_or_else(|| {
            error!("has no main thread sender");
            gst::FlowError::Flushing
        })?;
        // we first mark the frame available so that the main thread would listen
        // the frame channel.
        let _ = self
            .fl_config
            .borrow()
            .as_ref()
            .inspect(|p| p.sendable_txt.mark_frame_available());
        match sender.try_send(SinkEvent::FrameChanged(resolved_frame)) {
            Ok(_) => {}
            Err(flume::TrySendError::Full(_)) => warn!("Main thread receiver is full"),
            Err(flume::TrySendError::Disconnected(_)) => {
                error!("Main thread receiver is disconnected");
                return Err(gst::FlowError::Flushing);
            }
        }
        Ok(gst::FlowSuccess::Ok)
    }

    pub(crate) fn set_fl_config(&self, config: FlutterConfig) {
        *self.fl_config.borrow_mut() = Some(config);
    }
}
