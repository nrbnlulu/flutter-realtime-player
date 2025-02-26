use flume::bounded;
use irondash_engine_context::EngineContext;
use irondash_run_loop::RunLoop;
use log::{debug, trace};
use simple_logger::SimpleLogger;

use crate::core::{fluttersink::{
    self,
    utils::{self, LogErr},
}, types::VideoInfo};

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    SimpleLogger::new().init().unwrap();

    // Default utilities - feel free to custom
    flutter_rust_bridge::setup_default_user_utils();
    debug!("Done initializing app");
}

pub fn flutter_gstreamer_init(ffi_ptr: i64) {
    irondash_dart_ffi::irondash_init_ffi(ffi_ptr as *mut std::ffi::c_void);

    fluttersink::init().log_err();
    debug!("Done initializing flutter gstreamer");
}

pub fn create_new_playable(engine_handle: i64,vide_info: VideoInfo) -> i64 {
    trace!("get_texture was called");
    crate::core::fluttersink::create_new_playable(engine_handle, vide_info).unwrap()
}
