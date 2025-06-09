use std::collections::HashMap;

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
    debug!("Done initializing flutter gstreamer");
    *is_initialized = true;
}

lazy_static::lazy_static! {
    static ref SESSION_COUNTER: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
}

/// returns a texture id, this id is also used to identify the session
pub fn create_new_playable(
    engine_handle: i64,
    video_info: VideoInfo,
    ffmpeg_options: Option<HashMap<String, String>>,
    sink: StreamSink<StreamState>,
) {
    let mut session_counter = SESSION_COUNTER.lock().unwrap();
    *session_counter += 1;
    let session_id = *session_counter;
    sink.add(StreamState::Init {
        session_id: session_id,
    })
    .log_err();
    trace!(
        "get_texture was called with engine_handle: {}, video_info: {:?}",
        engine_handle,
        video_info
    );
    let _ = crate::core::fluttersink::create_new_playable(
        session_id,
        engine_handle,
        video_info,
        sink.clone(),
        ffmpeg_options,
    )
    .inspect_err(|e| {
        log::debug!("Failed to create new playable: {:?}", e);
        sink.add(StreamState::Error(e.to_string())).log_err();
    });
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
