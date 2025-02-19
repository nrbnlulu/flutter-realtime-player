use crate::core::platform::{NativeTextureType, PlatformNativeTextureProvider};
use gst::{glib, prelude::*};
use std::sync::{atomic::AtomicBool, Arc};

pub(crate) struct FlutterConfig {
    fl_txt_id: i64,
    fl_engine_handle: i64,
    sendable_texture: irondash_texture::SendableTexture<NativeTextureType>,
}

impl FlutterConfig {
    pub(crate) fn new(fl_txt_id: i64, fl_engine_handle: i64, sendable_texture: irondash_texture::SendableTexture<NativeTextureType>) -> Self {
        FlutterConfig {
            fl_txt_id,
            fl_engine_handle,
            sendable_texture
        }
    }
}

pub struct FlutterTextureSink {
    appsink: gst_app::AppSink,
    glsink: gst::Element,
    provider: Arc<PlatformNativeTextureProvider>,
    initialized_signal: Arc<AtomicBool>,
}

impl FlutterTextureSink {
    pub fn new(initialized_signal: Arc<AtomicBool>) -> Self {
        let appsink = gst_app::AppSink::builder().build();
        let glsink;

        #[cfg(target_os = "linux")]
        {
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

            #[cfg(target_os = "linux")]
            let glsink = gst::ElementFactory::make("glsinkbin")
                .property("sink", &appsink)
                .build()
                .expect("Fatal: Unable to create glsink");
        }

        // on windows use d3d11upload
        #[cfg(target_os = "windows")]
        {
            // Needs BGRA or RGBA swapchain for D2D interop,
            // and "present" signal must be explicitly enabled
            glsink = gst::ElementFactory::make("d3d11videosink")
                .property("emit-present", true)
                .property_from_str("display-format", "DXGI_FORMAT_R8G8B8A8_UNORM")
                .build()
                .unwrap();
            let provider = Arc::new(PlatformNativeTextureProvider::new());

            let provider_clone = provider.clone();
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
            glsink.connect_closure(
                "present",
                false,
                glib::closure!(move |_sink: &gst::Element,
                                     _device: &gst::Object,
                                     rtv_raw: glib::Pointer| {
                    provider_clone.on_present(_sink, _device, rtv_raw);
                }),
            );

            Self {
                appsink: appsink.upcast(),
                glsink: glsink,
                provider: Arc::new(PlatformNativeTextureProvider::new()),
                initialized_signal,
            }
        }
    }
    pub fn video_sink(&self) -> gst::Element {
        self.glsink.clone().into()
    }
    pub fn texture_provider(&self) -> Arc<PlatformNativeTextureProvider> {
        self.provider.clone()
    }
    pub fn connect(
        &self,
        bus: &gst::Bus,
        config: FlutterConfig,
    ) -> (){


    }
}
