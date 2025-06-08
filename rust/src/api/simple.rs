use flutter_rust_bridge::frb;
use log::{debug, trace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{
    core::{
        fluttersink,
        types::VideoInfo,
    }, dart_types::StreamState, frb_generated::StreamSink, utils::LogErr
};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    let is_initialized = IS_INITIALIZED.lock().unwrap();
    if *is_initialized {
        return;
    }
    let file_appender = tracing_appender::rolling::daily("./logs", "flutter_realtime_player.log");
    let (non_blocking_file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file_writer)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info")) // Default to info level if RUST_LOG is not set
        .unwrap();
    // 5. Combine the layers and initialize the global subscriber
    tracing_subscriber::registry()
        .with(env_filter) // Apply the environment filter
        .with(console_layer) // Add the stdout layer
        .with(file_layer) // Add the file layer
        .try_init().unwrap(); // Set as the global default subscriber
    
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

lazy_static::lazy_static! {
    static ref SESSION_COUNTER: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
}



/// returns a texture id, this id is also used to identify the session
pub fn create_new_playable(
    engine_handle: i64,
    video_info: VideoInfo,
    sink: StreamSink<StreamState>,
) {
    let mut session_counter = SESSION_COUNTER.lock().unwrap();
    *session_counter += 1;
    let session_id = *session_counter;
    sink.add(StreamState::Init { session_id: session_id }).log_err();
    trace!(
        "get_texture was called with engine_handle: {}, video_info: {:?}",
        engine_handle,
        video_info
    );
    crate::core::fluttersink::create_new_playable(
        session_id,
        engine_handle,
        video_info,
        sink.clone(),
    )
    .inspect_err(|e| {
        log::error!("Failed to create new playable: {:?}", e);
        sink.add(StreamState::Error(e.to_string())).log_err();
    })
    .log_err();
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
