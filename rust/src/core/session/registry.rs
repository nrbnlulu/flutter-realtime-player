use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use log::{debug, error, info};

use crate::core::{
    session::VideoSession,
    types::{self},
};

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize GStreamer: {:?}", e))?;
    info!("GStreamer initialized");
    Ok(())
}

lazy_static::lazy_static! {
    static ref SESSION_CACHE: RwLock<HashMap<i64, Arc<dyn VideoSession>>> =
        RwLock::new(HashMap::new());
}

pub fn get_all_sessions() -> Vec<i64> {
    let session_cache = SESSION_CACHE.read().unwrap();
    session_cache.keys().copied().collect()
}

pub fn get_session(session_id: i64) -> Option<Arc<dyn VideoSession>> {
    let session_cache = SESSION_CACHE.read().unwrap();
    session_cache.get(&session_id).cloned()
}

pub fn insert_session(session_id: i64, session: Arc<dyn VideoSession>) {
    SESSION_CACHE.write().unwrap().insert(session_id, session);
}

fn remove_session(session_id: i64) -> Option<Arc<dyn VideoSession>> {
    let mut session_cache = SESSION_CACHE.write().unwrap();
    session_cache.remove(&session_id)
}

pub async fn stream_alive_tester_task() {
    loop {
        let mut closed_sessions = Vec::new();

        let holders = get_all_sessions()
            .into_iter()
            .filter_map(|session_id| get_session(session_id).map(|holder| (session_id, holder)))
            .collect::<Vec<_>>();
        let now = SystemTime::now();
        for (session_id, holder) in holders {
            let expired = now
                .duration_since(holder.last_alive_mark())
                .map(|dur| dur.as_millis() > 5000)
                .unwrap_or(false);
            if expired {
                closed_sessions.push(session_id);
            }
        }

        if !closed_sessions.is_empty() {
            log::warn!(
                "Closing sessions that was not pinged recently: {:?}",
                closed_sessions
            );
            for session_id in closed_sessions {
                destroy_stream_session(session_id);
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub fn mark_session_alive(session_id: i64) {
    log::trace!("mark_session_alive {}", session_id);
    if let Some(session) = get_session(session_id) {
        session.make_alive();
    }
}

pub fn destroy_engine_streams(engine_handle: i64) {
    log::info!("Destroying streams for engine handle: {}", engine_handle);
    let holders = get_all_sessions()
        .into_iter()
        .filter_map(|session_id| get_session(session_id).map(|holder| (session_id, holder)))
        .collect::<Vec<_>>();
    let to_remove = holders
        .into_iter()
        .filter_map(|(session_id, holder)| {
            if holder.engine_handle() == engine_handle {
                Some(session_id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for texture_id in &to_remove {
        log::debug!("Destroying stream with texture id: {}", texture_id);
    }
    for texture_id in &to_remove {
        destroy_stream_session(*texture_id);
    }
}

pub fn destroy_stream_session(session_id: i64) {
    log::debug!("Destroying stream session : {}", session_id);
    let active_sessions = get_all_sessions();
    debug!("Active sessions at destroy: {:?}", active_sessions);
    let session = remove_session(session_id);
    if let Some(holder) = session {
        log::debug!(
            "Session {} removed from cache, destroying in a new thread",
            session_id
        );
        holder.terminate();
    } else {
        info!(
            "No stream session found for session id: {}, can't remove",
            session_id
        );
    }
}

pub async fn seek_session(session_id: i64, ts: u64) -> anyhow::Result<()> {
    info!(
        "seek_session called for session_id={}, ts={}",
        session_id, ts
    );
    let session = get_session(session_id);
    match session {
        Some(session) => {
            log::trace!("Session found, seeking to {}", ts);
            session.seek(ts).await?;
            log::trace!("Seek completed successfully for session {}", session_id);
            Ok(())
        }
        None => {
            error!("Session {} not found for seek operation", session_id);
            anyhow::bail!("Session {} not found", session_id);
        }
    }
}

pub async fn wsc_rtp_live_session(session_id: i64) -> anyhow::Result<()> {
    log::trace!("wsc_rtp_live_session called for session_id={}", session_id);
    if let Some(session) = get_session(session_id) {
        session.go_to_live_stream().await?;
        log::trace!("Go live completed successfully for session {}", session_id);
        Ok(())
    } else {
        error!("Session {} not found for go_live operation", session_id);
        anyhow::bail!("Session {} not found", session_id);
    }
}

pub async fn set_speed_session(session_id: i64, speed: f64) -> anyhow::Result<()> {
    log::trace!(
        "set_speed_session called for session_id={}, speed={}",
        session_id,
        speed
    );
    if let Some(session) = get_session(session_id) {
        session.set_speed(speed).await?;
        log::trace!(
            "Set speed completed successfully for session {}",
            session_id
        );
        Ok(())
    } else {
        error!("Session {} not found for set_speed operation", session_id);
        anyhow::bail!("Session {} not found", session_id);
    }
}

pub fn register_events_sink(session_id: i64, sink: types::DartEventsStream) {
    if let Some(session) = get_session(session_id) {
        session.set_events_sink(sink);
    }
}
