pub mod ffmpeg_private;
pub mod fluttersink;
pub mod software_decoder;
pub mod types;
use log::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

lazy_static::lazy_static! {
    pub static ref IS_INITIALIZED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    static ref WORKER_GUARD: std::sync::Mutex<Option<tracing_appender::non_blocking::WorkerGuard>> = std::sync::Mutex::new(None);
}

pub(crate) fn init_logger() {
    let is_initialized = IS_INITIALIZED.lock().unwrap();
    if *is_initialized {
        return;
    }
    let file_appender = tracing_appender::rolling::daily("./logs", "flutter_realtime_player.log");
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
    flutter_rust_bridge::setup_default_user_utils();
    debug!("Done initializing");
}
