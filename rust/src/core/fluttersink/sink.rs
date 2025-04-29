use crate::core::platform::{
    get_texture_from_sample, NativeRegisteredTexture, NativeTextureProvider,
};
use gst::{glib::translate::FromGlibPtrFull, prelude::*};
use irondash_run_loop::platform;
use irondash_texture::TextureDescriptor;
use log::trace;
use std::sync::{atomic::AtomicBool, Arc};

use super::utils::LogErr;

pub struct FlutterTextureSink {
    sink: gst::Element,
    provider: Arc<NativeTextureProvider>,
    registered_texture: Arc<NativeRegisteredTexture>,
}

impl FlutterTextureSink {
    pub fn new(
        initialized_signal: Arc<AtomicBool>,
        appsink: gst_app::AppSink,
        provider: Arc<NativeTextureProvider>,
        registered_texture: Arc<NativeRegisteredTexture>,
    ) -> anyhow::Result<Self> {
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


            let provider_clone = provider.clone();
            appsink.set_callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |sink| {
                        let sample = sink.pull_sample().map_err(|e| gst::FlowError::Flushing)?;

                        if let Ok((handle, video_info)) = get_texture_from_sample(sample, &provider_clone.context.device) {
                            let (width, height) = (video_info.width(), video_info.height());
                            assert!(
                                video_info.format() == gst_video::VideoFormat::Bgra,
                                "Invalid video format: {:?}",
                                video_info.format()
                            );
                 
                            provider_clone
                                .set_current_texture(TextureDescriptor::new(
                                    irondash_texture::DxgiSharedHandle(handle.0 as _),
                                    width as _,
                                    height as _,
                                    width as _,
                                    height as _,
                                    irondash_texture::PixelFormat::BGRA,
                                ))
                                .log_err();
                            if !initialized_sig_clone.load(std::sync::atomic::Ordering::SeqCst) {
                                initialized_sig_clone
                                    .store(true, std::sync::atomic::Ordering::SeqCst);
                            }
                        }

                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );

            let appsink_as_element: gst::Element = appsink.upcast();

            Ok(Self {
                sink: appsink_as_element,
                provider,
                registered_texture,
            })
        }
    }
    pub fn video_sink(&self) -> gst::Element {
        self.sink.clone().into()
    }

    pub fn texture_provider(&self) -> Arc<NativeTextureProvider> {
        self.provider.clone()
    }
}
