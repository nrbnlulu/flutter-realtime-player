use log::{debug, trace};

use crate::{
    core::{
        fluttersink::{self, utils::LogErr},
        types::VideoInfo,
    },
    utils::invoke_on_platform_main_thread,
};


#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    let is_initialized = IS_INITIALIZED.lock().unwrap();
    if *is_initialized {
        return;
    }
    let log_file = tracing_appender::rolling::daily("./logs", "flutter_realtime_player");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(log_file)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
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
/// returns a texture id, this id is also used to identify the session
pub fn create_new_playable(engine_handle: i64, vide_info: VideoInfo) -> i64 {
    debug!("get_texture was called");
    crate::core::fluttersink::create_new_playable(engine_handle, vide_info).unwrap()
}

pub fn destroy_engine_streams(engine_id: i64) {
    trace!("destroy_playable was called");
    // it is important to call this on the platform main thread
    // because irondash will unregister the texture on Drop, and drop must occur
    // on the platform main thread
    invoke_on_platform_main_thread(move || {
        crate::core::fluttersink::destroy_engine_streams(engine_id)
    });
}

pub fn destroy_stream_session(texture_id: i64) {
    trace!("destroy_stream_session was called");
    invoke_on_platform_main_thread(move || {
        crate::core::fluttersink::destroy_stream_session(texture_id)
    });
}
