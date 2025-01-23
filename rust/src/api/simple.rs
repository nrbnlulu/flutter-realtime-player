use flume::bounded;
use irondash_run_loop::RunLoop;
use simple_logger::SimpleLogger;

use crate::core::fluttersink::{self, testit, utils};

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    SimpleLogger::new().init().unwrap();
    fluttersink::init().unwrap();

    // Default utilities - feel free to custom
    flutter_rust_bridge::setup_default_user_utils();
}

pub fn get_opengl_texture(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    return utils::invoke_on_platform_main_thread(move || testit(engine_handle, uri));
}
