use crate::core::{
    fluttersink::utils::LogErr,
    platform::{NativeRegisteredTexture, NativeTextureProvider, NativeTextureType},
};
use gst::{glib, prelude::*};
use log::trace;
use std::sync::{atomic::AtomicBool, Arc};

pub struct FlutterTextureSink {
    sinkbin: gst::Bin,
    provider: Arc<NativeTextureProvider>,
    registered_texture: Arc<NativeRegisteredTexture>,
}

impl FlutterTextureSink {
    pub fn new(
        initialized_signal: Arc<AtomicBool>,
        pipeline: &gst::Pipeline,
        provider: Arc<NativeTextureProvider>,
        registered_texture: Arc<NativeRegisteredTexture>,
    ) -> anyhow::Result<Self> {
        let appsink = gst_app::AppSink::builder().build();
        let glsink;
        let initialized_sig_clone = initialized_signal.clone();
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
            use crate::core::platform::TextureDescriptionProvider2Ext;
            use windows::{
                core::*,
                Win32::Graphics::{Direct3D11::*, Dxgi::*},
            };

            let app_sink = gst_app::AppSink::builder()
                .caps(
                    &gst_video::VideoCapsBuilder::new()
                        .features(["memory:D3D11Memory"])
                        .format(gst_video::VideoFormat::Bgra)
                        .field("texture-target", "2D")
                        .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
                        .build(),
                )
                .enable_last_sample(false)
                .max_buffers(1u32)
                .build();

            // see https://gstreamer.freedesktop.org/documentation/d3d11/d3d11videosink.html?gi-language=c#d3d11videosink:draw-on-shared-texture
            glsink = gst::ElementFactory::make("d3d11videosink")
                .property("draw-on-shared-texture", true)
                .property_from_str("display-format", "DXGI_FORMAT_B8G8R8A8_UNORM")
                .build()
                .unwrap();

            let registered_texture_clone = registered_texture.clone();
            let provider_clone = provider.clone();
            
            appsink.set_callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |sink| {
                        let sample = sink.pull_sample().map_err(|e| gst::FlowError::Flushing)?;
                        let mut buffer = sample.buffer_owned().unwrap();
                    {
                        trace!("buffer is {:?}", buffer);
                    }

                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );
            // glsink.connect_closure(
            //     "begin-draw",
            //     false,
            //     glib::closure!(move |sink: &gst::Element| {
            //         provider_clone
            //             .on_begin_draw(sink)
            //             .inspect(|_| {
            //                 registered_texture_clone.mark_frame_available().log_err();
            //             })
            //             .log_err();
            //         initialized_sig_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            //     }),
            // );
            let bin = gst::Bin::new();
            let appsink_as_element: gst::Element = appsink.clone().upcast();
            bin.add_many(&[&appsink_as_element, &glsink])?;
            glsink.link(&appsink_as_element)?;
            // Add ghost pad to bin
            let sink_pad = glsink.static_pad("sink").expect("Failed to get sink pad");
            let ghost_pad = gst::GhostPad::builder(gst::PadDirection::Sink)
            .with_target(&sink_pad)?.build();

            bin.add_pad(&ghost_pad)?;
            Ok(Self {
                sinkbin: bin,
                provider,
                registered_texture,
            })
        }
    }
    pub fn video_sink(&self) -> gst::Element {
        self.sinkbin.clone().into()
    }
    pub fn texture_provider(&self) -> Arc<NativeTextureProvider> {
        self.provider.clone()
    }
}
