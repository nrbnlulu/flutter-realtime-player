//
// Copyright (C) 2021 Bilal Elmoussaoui <bil.elmoussaoui@gmail.com>
// Copyright (C) 2021 Jordan Petridis <jordan@centricular.com>
// Copyright (C) 2021-2024 Sebastian Dröge <sebastian@centricular.com>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use crate::core::fluttersink::frame::Frame;
use crate::core::fluttersink::utils;

use super::gltexture::GLTextureSource;
use super::utils::{invoke_on_gs_main_thread, make_element};
use super::{frame, FrameSender, SinkEvent};

use glib::clone::Downgrade;
use glib::thread_guard::ThreadGuard;

use gst::{prelude::*, subclass::prelude::*};
use gst_base::subclass::prelude::*;
use gst_gl::prelude::{GLContextExt as _, *};
use gst_video::subclass::prelude::*;
use irondash_texture::SendableTexture;
use log::{error, info, warn};

use std::cell::RefCell;
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

static GL_CONTEXT: Mutex<GLContext> = Mutex::new(GLContext::Uninitialized);

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
    global_orientation: frame::Orientation,
    /// Orientation from a stream scope tag
    stream_orientation: Option<frame::Orientation>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            info: None,
            global_orientation: frame::Orientation::Rotate0,
            stream_orientation: None,
        }
    }
}

pub(crate) struct FlutterConfig {
    fl_txt_id: i64,
    frame_sender: FrameSender,
    sendable_txt: ArcSendableTexture
}

impl FlutterConfig {
    pub(crate) fn new(fl_txt_id: i64, frame_sender: FrameSender, sendable_txt: ArcSendableTexture) -> Self {
        FlutterConfig {
            fl_txt_id,
            frame_sender,
            sendable_txt
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
                "Bilal Elmoussaoui <bil.elmoussaoui@gmail.com>, Jordan Petridis <jordan@centricular.com>, Sebastian Dröge <sebastian@centricular.com>, Nir Benlulu <nrbnlulu@gmail.com>",
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
                        feature = "x11egl",
                        feature = "x11glx",
                        feature = "waylandegl",
                        target_os = "macos",
                        target_os = "windows"
                    ))]
                    Some(gst::CapsFeatures::new([
                        gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY,
                        gst_video::CAPS_FEATURE_META_GST_VIDEO_OVERLAY_COMPOSITION,
                    ])),
                    #[cfg(any(
                        feature = "x11egl",
                        feature = "x11glx",
                        feature = "waylandegl",
                        target_os = "macos",
                        target_os = "windows"
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
                        const GL_FORMATS: &[gst_video::VideoFormat] =
                            &[gst_video::VideoFormat::Rgba, gst_video::VideoFormat::Rgb];
                        const NON_GL_FORMATS: &[gst_video::VideoFormat] = &[
                            #[cfg(feature = "gtk_v4_14")]
                            gst_video::VideoFormat::Bgrx,
                            #[cfg(feature = "gtk_v4_14")]
                            gst_video::VideoFormat::Xrgb,
                            #[cfg(feature = "gtk_v4_14")]
                            gst_video::VideoFormat::Rgbx,
                            #[cfg(feature = "gtk_v4_14")]
                            gst_video::VideoFormat::Xbgr,
                            gst_video::VideoFormat::Bgra,
                            gst_video::VideoFormat::Argb,
                            gst_video::VideoFormat::Rgba,
                            gst_video::VideoFormat::Abgr,
                            gst_video::VideoFormat::Rgb,
                            gst_video::VideoFormat::Bgr,
                        ];

                        let formats = if features.as_ref().is_some_and(|features| {
                            features.contains(gst_gl::CAPS_FEATURE_MEMORY_GL_MEMORY)
                        }) {
                            GL_FORMATS
                        } else {
                            NON_GL_FORMATS
                        };

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
                info!("FlutterTextureSink::change_state(NullToReady)");
                // Notify the pipeline about the GL display and wrapped context so that any other
                // elements in the pipeline ideally use the same / create GL contexts that are
                // sharing with this one.
                {
                    let gl_context = GL_CONTEXT.lock().unwrap();
                    if let GLContext::Initialized {
                        display,
                        wrapped_context,
                        ..
                    } = &*gl_context
                    {
                        let display = display.clone();
                        let wrapped_context = wrapped_context.clone();
                        drop(gl_context);

                        gst_gl::gl_element_propagate_display_context(&*self.obj(), &display);
                        let mut ctx = gst::Context::new("gst.gl.app_context", true);
                        {
                            let ctx = ctx.get_mut().unwrap();
                            ctx.structure_mut().set("context", &wrapped_context);
                        }
                        let _ = self.obj().post_message(
                            gst::message::HaveContext::builder(ctx)
                                .src(&*self.obj())
                                .build(),
                        );
                    }
                }
            }
            _ => (),
        }

        let res = self.parent_change_state(transition);

        match transition {
            gst::StateChange::PausedToReady => {
                info!("FlutterTextureSink::change_state(PausedToReady)");
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
            gst::StateChange::ReadyToNull => {
                info!("FlutterTextureSink::change_state(ReadyToNull)");
            }
            _ => (),
        }

        res
    }
}

impl BaseSinkImpl for FlutterTextureSink {
    fn caps(&self, filter: Option<&gst::Caps>) -> Option<gst::Caps> {
        let cached_caps = self
            .cached_caps
            .lock()
            .expect("Failed to lock cached caps mutex")
            .clone();

        let mut tmp_caps = cached_caps.unwrap_or_else(|| {
            let templ = Self::pad_templates();
            templ[0].caps().clone()
        });

        gst::debug!(CAT, imp = self, "Advertising our own caps: {tmp_caps:?}");

        if let Some(filter_caps) = filter {
            gst::debug!(
                CAT,
                imp = self,
                "Intersecting with filter caps: {filter_caps:?}",
            );

            tmp_caps = filter_caps.intersect_with_mode(&tmp_caps, gst::CapsIntersectMode::First);
        };

        gst::debug!(CAT, imp = self, "Returning caps: {tmp_caps:?}");
        Some(tmp_caps)
    }

    fn set_caps(&self, caps: &gst::Caps) -> Result<(), gst::LoggableError> {
        gst::debug!(CAT, imp = self, "Setting caps {caps:?}");

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

    fn propose_allocation(
        &self,
        query: &mut gst::query::Allocation,
    ) -> Result<(), gst::LoggableError> {
        gst::debug!(CAT, imp = self, "Proposing Allocation query");

        self.parent_propose_allocation(query)?;

        query.add_allocation_meta::<gst_video::VideoMeta>(None);

        let s = {
            let settings = self.settings.lock().unwrap();
            if (settings.window_width, settings.window_height) != (0, 0) {
                gst::debug!(
                    CAT,
                    imp = self,
                    "answering alloc query with size {}x{}",
                    settings.window_width,
                    settings.window_height
                );

                self.window_resized.store(false, atomic::Ordering::SeqCst);

                Some(
                    gst::Structure::builder("GstVideoOverlayCompositionMeta")
                        .field("width", settings.window_width)
                        .field("height", settings.window_height)
                        .build(),
                )
            } else {
                None
            }
        };

        query.add_allocation_meta::<gst_video::VideoOverlayCompositionMeta>(s.as_deref());

        {
            if let GLContext::Initialized {
                wrapped_context, ..
            } = &*GL_CONTEXT.lock().unwrap()
            {
                if wrapped_context.check_feature("GL_ARB_sync")
                    || wrapped_context.check_feature("GL_EXT_EGL_sync")
                {
                    query.add_allocation_meta::<gst_gl::GLSyncMeta>(None)
                }
            }
        }

        Ok(())
    }

    fn query(&self, query: &mut gst::QueryRef) -> bool {
        gst::log!(CAT, imp = self, "Handling query {:?}", query);

        match query.view_mut() {
            gst::QueryViewMut::Context(q) => {
                // Avoid holding the locks while we respond to the query
                // The objects are ref-counted anyway.
                let mut display_clone = None;
                let mut wrapped_context_clone = None;
                if let GLContext::Initialized {
                    display,
                    wrapped_context,
                    ..
                } = &*GL_CONTEXT.lock().unwrap()
                {
                    display_clone = Some(display.clone());
                    wrapped_context_clone = Some(wrapped_context.clone());
                }

                if let (Some(display), Some(wrapped_context)) =
                    (display_clone, wrapped_context_clone)
                {
                    return gst_gl::functions::gl_handle_context_query(
                        &*self.obj(),
                        q,
                        Some(&display),
                        None::<&gst_gl::GLContext>,
                        Some(&wrapped_context),
                    );
                }

                BaseSinkImplExt::parent_query(self, query)
            }
            _ => BaseSinkImplExt::parent_query(self, query),
        }
    }

    fn event(&self, event: gst::Event) -> bool {
        match event.view() {
            gst::EventView::StreamStart(_) => {
                let mut config = self.config.lock().unwrap();
                config.global_orientation = frame::Orientation::Rotate0;
                config.stream_orientation = None;
            }
            gst::EventView::Tag(ev) => {
                let mut config = self.config.lock().unwrap();
                let tags = ev.tag();
                let scope = tags.scope();
                let orientation = frame::Orientation::from_tags(tags);

                if scope == gst::TagScope::Global {
                    config.global_orientation = orientation.unwrap_or(frame::Orientation::Rotate0);
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
        gst::trace!(CAT, imp = self, "Rendering buffer {:?}", buffer);

        if self.window_resized.swap(false, atomic::Ordering::SeqCst) {
            gst::debug!(CAT, imp = self, "Window size changed, needs to reconfigure");
            let obj = self.obj();
            let sink = obj.sink_pad();
            sink.push_event(gst::event::Reconfigure::builder().build());
        }

        // Empty buffer, nothing to render
        if buffer.n_memory() == 0 {
            gst::trace!(
                CAT,
                imp = self,
                "Empty buffer, nothing to render. Returning."
            );
            return Ok(gst::FlowSuccess::Ok);
        };

        let config = self.config.lock().unwrap();
        let info = config.info.as_ref().ok_or_else(|| {
            gst::error!(CAT, imp = self, "Received no caps yet");
            gst::FlowError::NotNegotiated
        })?;
        let orientation = config
            .stream_orientation
            .unwrap_or(config.global_orientation);

        let wrapped_context = {
            {
                let gl_context = GL_CONTEXT.lock().unwrap();
                if let GLContext::Initialized {
                    wrapped_context, ..
                } = &*gl_context
                {
                    Some(wrapped_context.clone())
                } else {
                    None
                }
            }
        };
        let frame = Frame::new(buffer, info, orientation, wrapped_context.as_ref()).inspect_err(
            |_err| {
                gst::error!(CAT, imp = self, "Failed to map video frame");
            },
        )?;
        self.pending_frame.lock().unwrap().replace(frame);

        let sender = self
            .fl_config
            .borrow()
            .as_ref()
            .map(|wrapper| wrapper.frame_sender.clone());
        let sender = sender.as_ref().ok_or_else(|| {
            error!("has no main thread sender");
            gst::FlowError::Flushing
        })?;

        match sender.try_send(SinkEvent::FrameChanged) {
                gst::warning!(CAT, imp = self, "Have too many pending frames");
            Ok(_) => self.fl_config.borrow().inspect(|p| p.sendable_txt.mark_frame_available()),
            Err(flume::TrySendError::Full(_)) => {
                warn!("Too many pending frames");
            }
            Err(flume::TrySendError::Disconnected(_)) => {
                gst::error!(CAT, imp = self, "Have main thread receiver shut down");
                return Err(gst::FlowError::Flushing);
            }
        }

        Ok(gst::FlowSuccess::Ok)
    }
}

impl FlutterTextureSink {
    fn pending_frame(&self) -> Option<Frame> {
        self.pending_frame.lock().unwrap().take()
    }

    fn handle_frame_change(&self) {
        if let Some(frame) = self.pending_frame() {
            gst::trace!(CAT, imp = self, "Frame changed");
        }
    }

    fn configure_caps(&self) {
        #[allow(unused_mut)]
        let mut tmp_caps = Self::pad_templates()[0].caps().clone();

        #[cfg(all(target_os = "linux", feature = "dmabuf"))]
        {
            let formats = utils::invoke_on_gs_main_thread(move || {
                let Some(display) = gdk::Display::default() else {
                    return vec![];
                };
                let dmabuf_formats = display.dmabuf_formats();

                let mut formats = vec![];
                let n_formats = dmabuf_formats.n_formats();
                for i in 0..n_formats {
                    let (fourcc, modifier) = dmabuf_formats.format(i);

                    if fourcc == 0 || modifier == (u64::MAX >> 8) {
                        continue;
                    }

                    formats.push(gst_video::dma_drm_fourcc_to_string(fourcc, modifier));
                }

                formats
            });

            if formats.is_empty() {
                // Filter out dmabufs caps from the template pads if we have no supported formats
                tmp_caps = tmp_caps
                    .iter_with_features()
                    .filter(|(_, features)| {
                        !features.contains(gst_allocators::CAPS_FEATURE_MEMORY_DMABUF)
                    })
                    .map(|(s, c)| (s.to_owned(), c.to_owned()))
                    .collect::<gst::Caps>();
            } else {
                let tmp_caps = tmp_caps.make_mut();
                for (s, f) in tmp_caps.iter_with_features_mut() {
                    if f.contains(gst_allocators::CAPS_FEATURE_MEMORY_DMABUF) {
                        s.set("drm-format", gst::List::new(&formats));
                    }
                }
            }
        }

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
