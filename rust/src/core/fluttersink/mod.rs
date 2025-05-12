pub mod utils;
use gst::prelude::*;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log::{info, trace};

use crate::core::software_decoder::SoftwareDecoder;

use super::types;

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

lazy_static::lazy_static! {
static ref SESSION_CACHE: Mutex<HashMap<i64, (Arc<SoftwareDecoder>, gst::Pipeline, i64)>> = Mutex::new(HashMap::new());
}

pub fn create_new_playable(
    engine_handle: i64,
    video_info: types::VideoInfo,
) -> anyhow::Result<i64> {
    trace!("Initializing flutter sink");
    let pipeline = gst::ElementFactory::make("playbin")
        .property("uri", video_info.uri)
        .build()?
        .downcast::<gst::Pipeline>()
        .unwrap();

    let (decoder, texture_id) = SoftwareDecoder::new(&pipeline, 1, engine_handle)?;
    pipeline.set_state(gst::State::Playing)?;
    pipeline.connect_closure(
        "element-setup",
        false,
        glib::closure!(move |_playbin: &gst::Element, element: &gst::Element| {
            if element.name() == "rtspsrc" {
                info!("Setting latency to 0 for rtspsrc");
                element.set_property("latency", &0u32);
            }
        }),
    );

    // pipeline.set_property("video-sink", &flutter_sink.video_sink());
    SESSION_CACHE.lock().unwrap().insert(
        texture_id,
        (decoder.clone(), pipeline.clone(), engine_handle),
    );

    // // wait for the sink to be initialized
    // while !initialized_sig.load(std::sync::atomic::Ordering::Relaxed) {
    //     std::thread::sleep(std::time::Duration::from_millis(10));
    // }
    trace!("Sink initialized; returning texture id: {}", texture_id);
    Ok(texture_id)
}

pub fn destroy_engine_streams(engine_handle: i64) {
    info!("Destroying streams for engine handle: {}", engine_handle);
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    let mut to_remove = vec![];
    for (texture_id, (decoder, _playbin, handle)) in session_cache.iter() {
        if *handle == engine_handle {
            info!("Destroying stream with texture id: {}", texture_id);
            decoder.destroy_stream();
            to_remove.push(*texture_id);
        }
    }
    for texture_id in &to_remove {
        session_cache.remove(&texture_id);
    }
    info!(
        "Destroyed {} streams for engine handle: {}",
        to_remove.len(),
        engine_handle
    );
}

pub fn destroy_stream_session(texture_id: i64) {
    info!("Destroying stream session for texture id: {}", texture_id);
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some((decoder, _, _)) = session_cache.remove(&texture_id) {
        decoder.destroy_stream();
        info!("Destroyed stream session for texture id: {}", texture_id);
    } else {
        info!("No stream session found for texture id: {}", texture_id);
    }
}
