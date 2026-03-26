pub mod input;
pub mod output;
pub mod session;
pub mod texture;
pub mod types;
use std::sync::Arc;

use log::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

lazy_static::lazy_static! {
    pub static ref IS_INITIALIZED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    static ref WORKER_GUARD: std::sync::Mutex<Option<tracing_appender::non_blocking::WorkerGuard>> = std::sync::Mutex::new(None);
    pub static ref HTTP_CLIENT: Arc<reqwest::Client> = Arc::new(reqwest::Client::new());
}

pub(crate) fn init_logger() {
    let mut is_initialized = IS_INITIALIZED.lock().unwrap();
    if *is_initialized {
        return;
    }

    flutter_rust_bridge::setup_default_user_utils();

    // Use a writable directory for logs on Android
    #[cfg(target_os = "android")] {
        // since we need to know the package name in android in order to get the writable
        // logging path which would be let log_dir = "/data/data/com.example.flutter_realtime_player_example/files/logs";
        // so we just don't enable logging on Android for now, we can get that path from Dart side later.
        *is_initialized = true;
        return;
    } 
    #[cfg(not(target_os = "android"))] {
        let log_dir = "./logs";
        // Try to create the log directory if it doesn't exist
        let _ = std::fs::create_dir_all(log_dir);
        let file_appender = tracing_appender::rolling::daily(log_dir, "flutter_realtime_player.log");
        let (non_blocking_file_writer, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking_file_writer)
            .with_thread_ids(true)
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
            .try_init()
            .unwrap(); // Set as the global default subscriber

        // leak the guard to keep the file writer alive
        WORKER_GUARD.lock().unwrap().replace(guard);
        // Default utilities - feel free to custom
        *is_initialized = true;
        debug!("Done initializing");
    }
}
