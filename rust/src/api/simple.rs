use std::{collections::HashMap, thread};

use log::{debug, info, trace};

use crate::{
    core::{
        session::registry::{self},
        types::{TsdpEndpoint, VideoInfo},
        IS_INITIALIZED,
    },
    dart_types::{StreamEvent, StreamState},
    frb_generated::StreamSink,
    utils::LogErr,
};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    crate::core::init_logger();
}

pub fn flutter_realtime_player_init(ffi_ptr: i64) {
    let mut is_initialized = IS_INITIALIZED.lock().unwrap();
    if *is_initialized {
        return;
    }
    irondash_dart_ffi::irondash_init_ffi(ffi_ptr as *mut std::ffi::c_void);

    registry::init().log_err();
    thread::spawn(crate::core::session::registry::stream_alive_tester_task);
    debug!("Done initializing flutter gstreamer");
    *is_initialized = true;
}

lazy_static::lazy_static! {
    static ref SESSION_COUNTER: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
}
/// updates the counter and returns a session id
/// note that this doesn't create any resources apart from raising the counter
pub fn create_new_session() -> i64 {
    let mut session_counter = SESSION_COUNTER.lock().unwrap();
    *session_counter += 1;
    *session_counter
}

pub fn create_new_playable(
    session_id: i64,
    engine_handle: i64,
    video_info: VideoInfo,
    ffmpeg_options: Option<HashMap<String, String>>,
    sink: StreamSink<StreamState>,
) -> anyhow::Result<()> {
    trace!(
        "get_texture was called with engine_handle: {}, video_info: {:?}, session_id: {}",
        engine_handle,
        video_info,
        session_id
    );
    crate::core::session::registry::create_new_playable(
        session_id,
        engine_handle,
        video_info,
        sink.clone(),
        ffmpeg_options,
    )?;
    Ok(())
}

pub fn create_tsdp_playable(
    session_id: i64,
    engine_handle: i64,
    endpoint: TsdpEndpoint,
    video_info: VideoInfo,
    ffmpeg_options: Option<HashMap<String, String>>,
    sink: StreamSink<StreamState>,
) -> anyhow::Result<()> {
    trace!(
        "create_tsdp_playable was called with engine_handle: {}, source_id: {}, session_id: {}",
        engine_handle,
        endpoint.source_id.as_str(),
        session_id
    );
    crate::core::session::registry::create_tsdp_playable(
        session_id,
        engine_handle,
        endpoint,
        video_info,
        sink.clone(),
        ffmpeg_options,
    )?;
    Ok(())
}

pub fn seek_to_timestamp(session_id: i64, ts: i64) -> anyhow::Result<()> {
    info!("seeking to {ts}");
    registry::seek_session(session_id, ts)
}

pub fn trtp_go_live(session_id: i64) -> anyhow::Result<()> {
    registry::trtp_live_session(session_id)
}

pub fn set_speed(session_id: i64, speed: f64) -> anyhow::Result<()> {
    registry::set_speed_session(session_id, speed)
}

pub fn register_to_stream_events_sink(session_id: i64, sink: StreamSink<StreamEvent>) {
    registry::register_events_sink(session_id, sink);
}

/// marks the session as required by the ui
/// if the ui won't call this every 2 seconds
/// this session will terminate itself.
pub fn mark_session_alive(session_id: i64) {
    crate::core::session::registry::mark_session_alive(session_id);
}

pub fn destroy_engine_streams(engine_id: i64) {
    trace!("destroy_playable was called");
    // it is important to call this on the platform main thread
    // because irondash will unregister the texture on Drop, and drop must occur
    // on the platform main thread
    crate::core::session::registry::destroy_engine_streams(engine_id);
}

pub fn destroy_stream_session(session_id: i64) {
    trace!("destroy_stream_session was called");
    crate::core::session::registry::destroy_stream_session(session_id)
}

pub fn resize_stream_session(session_id: i64, width: u32, height: u32) -> anyhow::Result<()> {
    crate::core::session::registry::resize_stream_session(session_id, width, height)
}
