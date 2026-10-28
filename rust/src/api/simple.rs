use log::{error, trace};

use crate::{
    core::{
        input::{playbin::PlaybinSession, wsc_rtp::WscRtpSession},
        session::{
            registry::{self, insert_session},
            VideoSessionCommon,
        },
        types::VideoConfig,
        HTTP_CLIENT,
    },
    dart_types::StreamMessage,
    frb_generated::StreamSink,
};

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    crate::core::init_logger();
}

lazy_static::lazy_static! {
    static ref SESSION_COUNTER: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
}
/// updates the counter and returns a session id
/// note that this doesn't create any resources apart from raising the counter
pub fn create_new_session() -> i64 {
    let mut session_counter = SESSION_COUNTER.lock().unwrap();
    *session_counter += 1;
    *session_counter
}

pub async fn create_playable(
    session_id: i64,
    config: VideoConfig,
    combined_sink: StreamSink<StreamMessage>,
) -> anyhow::Result<()> {
    trace!("create_playable was called with session_id: {}", session_id);
    match config {
        VideoConfig::WscRtp(wsc_rtp_config) => {
            trace!("  source_id: {}", wsc_rtp_config.source_id.as_str());
            let session_common = VideoSessionCommon::new(session_id, combined_sink);
            let (session, shutdown_rx) =
                WscRtpSession::new(wsc_rtp_config, session_common, HTTP_CLIENT.clone());
            let session_clone = session.clone();
            tokio::spawn(async move {
                if let Err(e) = session_clone.execute(shutdown_rx).await {
                    log::error!("WscRtp session {} exited with error: {}", session_id, e);
                }
            });
            insert_session(session_id, session);
        }
        VideoConfig::Playbin(playbin_config) => {
            trace!("  uri: {}", playbin_config.uri);
            let session_common = VideoSessionCommon::new(session_id, combined_sink);
            let (session, shutdown_rx) = PlaybinSession::new(playbin_config, session_common);
            let session_clone = session.clone();
            tokio::spawn(async move {
                if let Err(e) = session_clone.execute(shutdown_rx).await {
                    log::error!("Playbin session {} exited with error: {}", session_id, e);
                }
            });
            insert_session(session_id, session);
        }
    }
    Ok(())
}

pub async fn seek_to_timestamp(session_id: i64, ts: u64) -> anyhow::Result<()> {
    log::debug!(
        "seek_to_timestamp called: session_id={}, ts={}",
        session_id,
        ts
    );
    let result = registry::seek_session(session_id, ts).await;
    if let Err(e) = &result {
        log::error!("seek_to_timestamp failed: {}", e);
    }
    result
}

pub async fn wsc_rtp_go_live(session_id: i64) -> anyhow::Result<()> {
    log::debug!("wsc_rtp_go_live called: session_id={}", session_id);
    let result = registry::wsc_rtp_live_session(session_id).await;
    if let Err(e) = &result {
        log::error!("wsc_rtp_go_live failed: {}", e);
    }
    result
}

pub async fn set_speed(session_id: i64, speed: f64) -> anyhow::Result<()> {
    log::debug!(
        "set_speed called: session_id={}, speed={}",
        session_id,
        speed
    );
    let result = registry::set_speed_session(session_id, speed).await;
    if let Err(e) = &result {
        error!("set_speed failed: {}", e);
    }
    result
}

/// marks the session as required by the ui
/// if the ui won't call this every 2 seconds
/// this session will be terminate.
pub fn mark_session_alive(session_id: i64) {
    crate::core::session::registry::mark_session_alive(session_id);
}

pub fn destroy_all_sessions() {
    trace!("destroy_all_sessions was called");
    crate::core::session::registry::destroy_all_sessions();
}

pub fn destroy_stream_session(session_id: i64) {
    trace!("destroy_stream_session was called");
    crate::core::session::registry::destroy_stream_session(session_id)
}
