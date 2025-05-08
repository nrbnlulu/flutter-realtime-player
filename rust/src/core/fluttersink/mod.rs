pub mod utils;
use gst::{
    glib::object::Cast,
    prelude::{ElementExtManual, GstBinExt, GstBinExtManual, PadExt},
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};

use gst::prelude::{ElementExt, GstObjectExt};
use log::{error, info, trace};
use utils::LogErr;

use crate::core::{platform::NativeTextureProvider, software_renderer::SoftwareDecoder};

use super::{platform::NativeRegisteredTexture, types};

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

lazy_static::lazy_static! {
static ref SESSION_CACHE: Mutex<HashMap<i64, (Arc<SoftwareDecoder>, gst::Pipeline)>> = Mutex::new(HashMap::new());
}

pub fn create_new_playable(
    engine_handle: i64,
    video_info: types::VideoInfo,
) -> anyhow::Result<i64> {
    trace!("Initializing flutter sink");
    let initialized_sig = Arc::new(AtomicBool::new(false));
    let initialized_sig_clone = initialized_sig.clone();
    let pipeline = gst::ElementFactory::make("playbin")
           .property("uri", "https://gstreamer.freedesktop.org/data/media/sintel_trailer-480p.webm")
           .build()?
           .downcast::<gst::Pipeline>()
           .unwrap();
    
    let (decoder, texture_id) = SoftwareDecoder::new(&pipeline, 1, engine_handle)?;
    

    // let playbin = &pipeline;
    // playbin.connect_closure("element-setup", false,
    // glib::closure!(move |_playbin: &gst::Element, element: &gst::Element | {
    //     if element.name() == "rtspsrc" {
    //         info!("Setting latency to 0 for rtspsrc");
    //         element.set_property("latency", &0u32);
    //     }
    // }));

    // pipeline.set_property("video-sink", &flutter_sink.video_sink());
    SESSION_CACHE.lock().unwrap().insert(
        texture_id,
        (
            decoder.clone(),
            pipeline.clone(),
        ),
    );

    // wait for the sink to be initialized
    while !initialized_sig.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    trace!("Sink initialized; returning texture id: {}", texture_id);
    Ok(texture_id)
}
