use std::sync::{Arc, Mutex};

use flutter_rust_bridge::DartFnFuture;
use irondash_run_loop::RunLoop;
use log::trace;

use crate::core::types::StreamMessages;


/// Inboke the given function on the flutter engine main thread.
pub(crate) fn invoke_on_platform_main_thread<F, T>(func: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    if RunLoop::is_main_thread().unwrap_or(false) {
        trace!("invoke_on_platform_main_thread: already on main thread");
        return func();
    }

    trace!("invoke_on_platform_main_thread: sending to main thread");
    RunLoop::sender_for_main_thread()
        .expect("failed to get main thread sender")
        .send_and_wait(move || {
            trace!("in main thread");
            func()
        })
}

