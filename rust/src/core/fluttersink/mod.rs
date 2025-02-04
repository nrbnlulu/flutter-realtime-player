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
    glib::object::Cast,
    prelude::{ElementExt, GstObjectExt},
    trace,
};
use log::{error, info};
use sink::{ArcSendableTexture, FlutterTextureSink};

pub(crate) enum SinkEvent {
    FrameChanged(ResolvedFrame),
}
pub(crate) type FrameSender = flume::Sender<SinkEvent>;

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

pub fn testit(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    let (flutter_sink, sendable_texture) = utils::invoke_on_platform_main_thread(move || {
        let provider = Arc::new(FlutterTextureSink::new());

        let texture =
            irondash_texture::Texture::new_with_provider(engine_handle, provider.clone())?;
        Ok((provider, texture.into_sendable_texture()))
    })?;
    
    let pipeline = gst::ElementFactory::make("playbin")
        .property(
            "uri",
            "https://gstreamer.freedesktop.org/data/media/sintel_trailer-480p.webm",
        )
        .build()?
        .downcast::<gst::Pipeline>()
        .unwrap();

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

    Ok(id)
}
