use crate::core::fluttersink::utils;
use crate::core::platform::PlatformNativeTextureProvider;
use crate::core::types;

use super::utils::LogErr;
use gst::{prelude::*, PadProbeReturn, PadProbeType, QueryViewMut};
use log::{debug, error, trace, warn};

use std::collections::HashMap;
use std::sync::{
    atomic::{self, AtomicBool},
    Mutex,
};
use std::sync::{Arc, LazyLock};

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

pub struct FlutterTextureSink {
    appsink: gst_app::AppSink,
    glsink: gst::Element,
    initialized_signal: Arc<AtomicBool>,
}

impl FlutterTextureSink {
    pub fn new(initialized_signal: Arc<AtomicBool>) -> Self {
        let appsink = gst_app::AppSink::builder();
        #[cfg(target_os = "linux")]
        let appsink = appsink
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
        #[cfg(target_os = "windows")]
        let appsink = appsink.build();

        #[cfg(target_os = "linux")]
        let glsink = gst::ElementFactory::make("glsinkbin")
            .property("sink", &appsink)
            .build()
            .expect("Fatal: Unable to create glsink");

        // on windows use d3d11upload
        #[cfg((target_os = "windows"))]
        {
            // Needs BGRA or RGBA swapchain for D2D interop,
            // and "present" signal must be explicitly enabled
            let glsink = gst::ElementFactory::make("d3d11videosink")
                .property("emit-present", true)
                .property_from_str("display-format", "DXGI_FORMAT_B8G8R8A8_UNORM")
                .build()
                .unwrap();

            // Listen "present" signal and draw overlay from the callback
            // Required operations here:
            // 1) Gets IDXGISurface and ID3D11Texture2D interface from
            //    given ID3D11RenderTargetView COM object
            //   - ID3D11Texture2D: To get texture resolution
            //   - IDXGISurface: To create Direct2D render target
            // 2) Creates or reuses IDWriteTextLayout interface
            //   - This object represents text layout we want to draw on render target
            // 3) Draw rectangle (overlay background) and text on render target
            //
            // NOTE: ID2D1Factory, IDWriteFactory, IDWriteTextFormat, and
            // IDWriteTextLayout objects are device-independent. Which can be created
            // earlier instead of creating them in the callback.
            // But ID2D1RenderTarget is a device-dependent resource.
            // The client should not hold the d2d render target object outside of
            // this callback scope because the resource must be cleared before
            // releasing/resizing DXGI swapchain.
            videosink.connect_closure(
                "present",
                false,
                glib::closure!(move |_sink: &gst::Element,
                                     _device: &gst::Object,
                                     rtv_raw: glib::Pointer| {
                    windows_present_callback(rtv_raw);
                }),
            );
        }

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
        let (gst_gl_display, shared_context) = GL_MANAGER
            .with_borrow(|manager| manager.get_context(fl_config.fl_engine_handle).unwrap());
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

    pub fn get_native_texture_provider(&self) -> Arc<PlatformNativeTextureProvider> {
        unimplemented!("get_texture_provider")
    }
}
