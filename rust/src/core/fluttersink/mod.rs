mod frame;
pub mod gltexture;
pub(super) mod sink;
pub mod types;
pub mod utils;
use std::{
    sync::{Arc, Mutex},
    thread,
};

use frame::ResolvedFrame;
use glib::types::StaticType;
use gst::{
    glib::object::{Cast, ObjectExt},
    prelude::{ElementExt, GstObjectExt},
    trace,
};
use log::{error, info};
use sink::{ArcSendableTexture, FlutterConfig, FlutterTextureSink};

pub(crate) enum SinkEvent {
    FrameChanged(ResolvedFrame),
}
pub(crate) type FrameSender = flume::Sender<SinkEvent>;

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

pub fn testit(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    let (flutter_sink, sendable_texture, texture_id) =
        utils::invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
            let provider = Arc::new(FlutterTextureSink::new());

            let texture =
                irondash_texture::Texture::new_with_provider(engine_handle, provider.clone())?;
            let texture_id = texture.id();
            Ok((provider, texture.into_sendable_texture(), texture_id))
        })?;
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
        FlutterConfig::new(texture_id, engine_handle, sendable_texture),
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

    Ok(texture_id)
}
