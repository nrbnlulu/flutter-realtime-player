use flume::bounded;
use irondash_engine_context::EngineContext;
use irondash_run_loop::RunLoop;
use log::debug;
use simple_logger::SimpleLogger;

use crate::core::fluttersink::{
    self, testit,
    utils::{self, LogErr},
};

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

pub fn get_opengl_texture(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    testit(engine_handle, uri)
}
