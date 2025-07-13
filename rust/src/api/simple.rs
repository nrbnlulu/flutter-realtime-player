use std::{collections::HashMap, thread};

use log::{debug, trace};

use crate::{
    core::{fluttersink, types::VideoInfo, IS_INITIALIZED},
    dart_types::StreamState,
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

    fluttersink::init().log_err();
    thread::spawn(|| crate::core::fluttersink::stream_alive_tester_task());
    debug!("Done initializing flutter gstreamer");
    *is_initialized = true;
}

lazy_static::lazy_static! {
    static ref SESSION_COUNTER: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
}
/// updates the counter and returns a session id
/// note that this doesn't create any resources apart from raising the counter
pub fn create_new_session(

) -> i64 {
    let mut session_counter = SESSION_COUNTER.lock().unwrap();
    *session_counter += 1;
     *session_counter
    
}

/// returns a session id that represents a playing session in rust
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
     crate::core::fluttersink::create_new_playable(
        session_id,
        engine_handle,
        video_info,
        sink.clone(),
        ffmpeg_options,
    )?;
     Ok(())
}
/// marks the session as required by the ui
/// if the ui won't call this every 2 seconds 
/// this session will terminate itself.
pub fn mark_session_alive(session_id: i64) {
    let _ = crate::core::fluttersink::mark_session_alive(session_id);
}

pub fn destroy_engine_streams(engine_id: i64) {
    trace!("destroy_playable was called");
    // it is important to call this on the platform main thread
    // because irondash will unregister the texture on Drop, and drop must occur
    // on the platform main thread
    crate::core::fluttersink::destroy_engine_streams(engine_id);
}

pub fn destroy_stream_session(session_id: i64) {
    trace!("destroy_stream_session was called");
    crate::core::fluttersink::destroy_stream_session(session_id)
}
