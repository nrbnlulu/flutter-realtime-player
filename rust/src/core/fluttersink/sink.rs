use crate::core::platform::{
     NativeRegisteredTexture, NativeTextureProvider,
};
use gst::prelude::*;
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
