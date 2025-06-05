use std::{sync::{Arc, Mutex}, thread};

use flutter_rust_bridge::{frb, DartFnFuture};
use irondash_engine_context::EngineContext;
use log::{debug, trace};

use crate::{
    core::{
        fluttersink::{self, utils::LogErr},
        types::{StreamMessages, VideoInfo},
    }, frb_generated::StreamSink, utils::invoke_on_platform_main_thread
};


#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    let is_initialized = IS_INITIALIZED.lock().unwrap();
    if *is_initialized {
        return;
    }
    let log_file = tracing_appender::rolling::daily("./logs", "flutter_realtime_player.log");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(log_file)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .init();
    // Default utilities - feel free to custom
    flutter_rust_bridge::setup_default_user_utils();
    debug!("Done initializing");
}

lazy_static::lazy_static! {
    static ref IS_INITIALIZED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
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

lazy_static::lazy_static!{
    static ref SESSION_COUNTER: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
}

// To mirror an external struct, you need to define a placeholder type with the same definition
#[frb(mirror(StreamMessages))]
pub enum _StreamMessages {
    Error(String),
    Loading,
    Playing,
    Stopped,
    StreamAndTextureReady(i64), // texture id
}

/// returns a texture id, this id is also used to identify the session
pub fn create_new_playable(engine_handle: i64, video_info: VideoInfo, sink: StreamSink<StreamMessages>) {
    let mut session_counter = SESSION_COUNTER.lock().unwrap();
    *session_counter += 1;
    let session_id = *session_counter;


    trace!("get_texture was called with engine_handle: {}, video_info: {:?}", engine_handle, video_info);
    crate::core::fluttersink::create_new_playable(session_id, 
        engine_handle, video_info, sink.clone())
        .inspect_err(|e| {
            log::error!("Failed to create new playable: {:?}", e);
            sink.add(StreamMessages::Error(e.to_string())).log_err();
        }).log_err();
}

pub fn destroy_engine_streams(engine_id: i64) {
    trace!("destroy_playable was called");
    // it is important to call this on the platform main thread
    // because irondash will unregister the texture on Drop, and drop must occur
    // on the platform main thread    
    crate::core::fluttersink::destroy_engine_streams(engine_id);
}

pub fn destroy_stream_session(texture_id: i64) {
    trace!("destroy_stream_session was called");
        crate::core::fluttersink::destroy_stream_session(texture_id)
}
