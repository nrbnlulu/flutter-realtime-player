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
    trace,
};
use log::{error, info};
use sink::{ArcSendableTexture, FlutterConfig, FlutterTextureSink};

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

pub fn testit(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    let initialized_sig = Arc::new(AtomicBool::new(false));
    let initialized_sig_clone = initialized_sig.clone();
    let texture_id: i64 = utils::invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
        let flutter_sink = Arc::new(FlutterTextureSink::new(initialized_sig_clone));

        let texture = irondash_texture::Texture::new_with_provider(
            engine_handle,
            flutter_sink.get_native_texture_provider(),
        )?;
        let texture_id = texture.id();

        let pipeline = gst::ElementFactory::make("playbin")
            .property(
                "uri",
                "https://gstreamer.freedesktop.org/data/media/sintel_trailer-480p.webm",
            )
            .build()?
            .downcast::<gst::Pipeline>()
            .unwrap();

        pipeline.set_property("video-sink", &flutter_sink.video_sink());
        flutter_sink.connect(
            &pipeline.bus().unwrap(),
            FlutterConfig::new(texture_id, engine_handle, texture.into_sendable_texture()),
        );

        thread::spawn(move || {
            pipeline
                .set_state(gst::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");
            let bus = pipeline.bus().unwrap();
            for msg in bus.iter() {
                trace!(gst::CAT_DEFAULT, "Message: {:?}", msg.view());
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
        Ok((texture_id))
    })?;

    // wait for the sink to be initialized
    while !initialized_sig.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(texture_id)
}
