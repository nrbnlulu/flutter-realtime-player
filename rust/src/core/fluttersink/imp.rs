use crate::core::fluttersink::frame::Frame;
use crate::core::fluttersink::utils;
use crate::core::platform;

use super::frame::{TextureCacheId, VideoInfo};
use super::gltexture::{GLTexture, GLTextureSource};
use super::utils::{invoke_on_gs_main_thread, make_element};
use super::{frame, types, FrameSender, SinkEvent};

use glib::clone::Downgrade;
use glib::thread_guard::ThreadGuard;

use glib::translate::FromGlibPtrFull;
use gst::{prelude::*, subclass::prelude::*};
use gst_base::subclass::prelude::*;
use gst_gl::prelude::{GLContextExt as _, *};
use gst_video::subclass::prelude::*;
use log::{error, trace, warn};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    atomic::{self, AtomicBool},
    Mutex,
};
use std::sync::{Arc, LazyLock};

// Global GL context that is created by the first sink and kept around until the end of the
// process. This is provided to other elements in the pipeline to make sure they create GL contexts
// that are sharing with the GTK GL context.
enum GLContext {
    Uninitialized,
    Unsupported,
    #[allow(unused)]
    Initialized {
        display: gst_gl::GLDisplay,
        wrapped_context: gst_gl::GLContext,
        glow_context: ThreadGuard<glow::Context>,
    },
}


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
    pending_frame: Mutex<Option<Frame>>,
    cached_caps: Mutex<Option<gst::Caps>>,
    settings: Mutex<Settings>,
    window_resized: AtomicBool,
    playbin3: RefCell<Option<Rc<gst::Element>>>,
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
            // Those are the supported formats by a gdk::Texture
            let mut caps = gst::Caps::new_empty();
            {
                let caps = caps.get_mut().unwrap();

                #[cfg(all(target_os = "linux", feature = "dmabuf"))]
                {
                    for features in [
                        [
                            gst_allocators::CAPS_FEATURE_MEMORY_DMABUF,
                            gst_video::CAPS_FEATURE_META_GST_VIDEO_OVERLAY_COMPOSITION,
                        ]
                        .as_slice(),
                        [gst_allocators::CAPS_FEATURE_MEMORY_DMABUF].as_slice(),
                    ] {
                        let c = gst_video::VideoCapsBuilder::new()
                            .format(gst_video::VideoFormat::DmaDrm)
                            .features(features.iter().copied())
                            .build();
                        caps.append(c);
                    }
                }

                for features in [
                    #[cfg(any(
                        target_os = "macos",
                        target_os = "windows",
                        target_os = "linux"
                    ))]
                    Some(gst::CapsFeatures::new([
                        gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY,
                        gst_video::CAPS_FEATURE_META_GST_VIDEO_OVERLAY_COMPOSITION,
                    ])),
                    #[cfg(any(
                        target_os = "macos",
                        target_os = "windows",
                        target_os = "linux"
                    ))]
                    Some(gst::CapsFeatures::new([
                        gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY,
                    ])),
                    Some(gst::CapsFeatures::new([
                        "memory:SystemMemory",
                        gst_video::CAPS_FEATURE_META_GST_VIDEO_OVERLAY_COMPOSITION,
                    ])),
                    Some(gst::CapsFeatures::new([
                        gst_video::CAPS_FEATURE_META_GST_VIDEO_OVERLAY_COMPOSITION,
                    ])),
                    None,
                ] {
                    {

                        let formats =  &[gst_video::VideoFormat::Rgba];

                        let mut c = gst_video::video_make_raw_caps(formats).build();

                        if let Some(features) = features {
                            let c = c.get_mut().unwrap();

                            if features.contains(gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY) {
                                c.set("texture-target", "2D")
                            }
                            c.set_features_simple(Some(features));
                        }
                        caps.append(c);
                    }
                }
            }

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
    
                trace!("NullToReady");
            }
            _ => (),
        }

        let res = self.parent_change_state(transition);

        match transition {
            gst::StateChange::PausedToReady => {
                *self.config.lock().unwrap() = StreamConfig::default();
                let _ = self.pending_frame.lock().unwrap().take();

                // Flush frames from the GDK paintable but don't wait
                // for this to finish as this can other deadlock.
                // let self_ = self.to_owned();
                // invoke_on_main_thread(move || {
                //     let paintable = self_.paintable.lock().unwrap();
                //     if let Some(paintable) = &*paintable {
                //         paintable.get_ref().handle_flush_frames();
                //     }
                // });
            }
            gst::StateChange::ReadyToNull => {}
            _ => (),
        }

        res
    }
}

impl BaseSinkImpl for FlutterTextureSink {


    fn set_caps(&self, caps: &gst::Caps) -> Result<(), gst::LoggableError> {
        #[allow(unused_mut)]
        let mut video_info = None;
        #[cfg(all(target_os = "linux", feature = "dmabuf"))]
        {
            if let Ok(info) = gst_video::VideoInfoDmaDrm::from_caps(caps) {
                video_info = Some(info.into());
            }
        }

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
                let mut config = self.config.lock().unwrap();
                config.global_orientation = types::Orientation::Rotate0;
                config.stream_orientation = None;
            }
            gst::EventView::Tag(ev) => {
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
            let sample = self
                .playbin3
                .borrow()
                .as_ref()
                .unwrap()
                .emit_by_name::<gst::Sample>("convert-sample", &[&self.caps()]);

            return self.show_frame_from_buffer(&config, &sample.buffer_owned().unwrap(), info);
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
        // TODO: upload the buffer EGLImage
        // see https://stackoverflow.com/questions/22063044/how-to-transfer-textures-from-one-opengl-context-to-another
        let orientation = config
            .stream_orientation
            .unwrap_or(config.global_orientation);

        let wrapped_context = gst_gl::GLContext::current().expect("Failed to get current GL context");


        let frame = Frame::new(&buffer, info, orientation, Some(wrapped_context.as_ref())).inspect_err(
            |_err| {
                error!("Failed to create frame from buffer");
            },
        )?;
        let mut cached_textures: HashMap<TextureCacheId, GLTexture> = HashMap::new();
        let res = frame.into_textures(&wrapped_context, &mut cached_textures);


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
        match sender.try_send(SinkEvent::FrameChanged(frame)) {
            Ok(_) => {}
            Err(flume::TrySendError::Full(_)) => warn!("Main thread receiver is full"),            Err(flume::TrySendError::Disconnected(_)) => {
                error!("Main thread receiver is disconnected");
                return Err(gst::FlowError::Flushing);
            }
        }
        Ok(gst::FlowSuccess::Ok)
    }

    fn pending_frame(&self) -> Option<Frame> {
        self.pending_frame.lock().unwrap().take()
    }

    pub fn set_playbin3(&self, playbin3: Rc<gst::Element>) {
        *self.playbin3.borrow_mut() = Some(playbin3);
    }

    fn caps(&self) -> gst::Caps {
        gst::Caps::builder("video/x-raw(memory:GLMemory, meta:GstVideoOverlayComposition)")
            .field("format", &gst_video::VideoFormat::Rgba.to_string())
            .field("width", &640)
            .field("height", &480)
            .field("framerate", &gst::Fraction::new(30, 1))
            .build()
    }

    fn configure_caps(&self) {
        #[allow(unused_mut)]
        let mut tmp_caps = Self::pad_templates()[0].caps().clone();


        {
            // Filter out GL caps from the template pads if we have no context
            if !matches!(&*GL_CONTEXT.lock().unwrap(), GLContext::Initialized { .. }) {
                tmp_caps = tmp_caps
                    .iter_with_features()
                    .filter(|(_, features)| {
                        !features.contains(gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY)
                    })
                    .map(|(s, c)| (s.to_owned(), c.to_owned()))
                    .collect::<gst::Caps>();
            }
        }

        self.cached_caps
            .lock()
            .expect("Failed to lock Mutex")
            .replace(tmp_caps);
    }

    pub(crate) fn set_fl_config(&self, config: FlutterConfig) {
        *self.fl_config.borrow_mut() = Some(config);
    }
}
