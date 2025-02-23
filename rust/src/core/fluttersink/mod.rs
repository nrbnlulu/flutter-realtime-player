pub(super) mod sink;
pub mod utils;
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};

use glib::types::StaticType;
use gst::{
    glib::object::{Cast, ObjectExt},
    prelude::{ElementExt, GstObjectExt},
};
use log::{error, info, trace};
use sink::FlutterTextureSink;

use crate::core::platform::NativeTextureProvider;

use super::platform::NativeRegisteredTexture;

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

pub fn testit(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    trace!("Initializing flutter sink");
    let initialized_sig = Arc::new(AtomicBool::new(false));
    let initialized_sig_clone = initialized_sig.clone();
    let texture_provider;
    let registered_texture;
    let texture_id;
    #[cfg(target_os = "windows")]
    {
        use crate::core::platform::TextureDescriptionProvider2Ext;

        texture_provider = Arc::new(NativeTextureProvider::new());
        let texture_provider_clone = texture_provider.clone();
        registered_texture = utils::invoke_on_platform_main_thread(
            move || -> anyhow::Result<Arc<NativeRegisteredTexture>> {
                irondash_texture::alternative_api::RegisteredTexture::new(
                    texture_provider_clone,
                    engine_handle,
                ).map_err(|e| anyhow::anyhow!("Failed to registered texture: {:?}", e))
            })?;
        texture_id = registered_texture.get_texture_id();

            
    }
    let flutter_sink = Arc::new(FlutterTextureSink::new(
        initialized_sig_clone,
        texture_provider,
        registered_texture,
    ));

    let pipeline = gst::ElementFactory::make("playbin")
        .property(
            "uri",
            "https://gstreamer.freedesktop.org/data/media/sintel_trailer-480p.webm",
        )
        .build()?
        .downcast::<gst::Pipeline>()
        .unwrap();

    pipeline.set_property("video-sink", &flutter_sink.video_sink());

    thread::spawn(move || {
        pipeline
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
        let bus = pipeline.bus().unwrap();
        for msg in bus.iter() {
            trace!("Message: {:?}", msg.view());
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    info!("End of stream");
                    break;
                }
                gst::MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    break;
                }
                _ => (),
            }
        }
    });

    // wait for the sink to be initialized
    while !initialized_sig.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    trace!("Sink initialized; returning texture id: {}", texture_id);
    Ok(texture_id)
}
