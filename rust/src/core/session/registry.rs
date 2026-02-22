use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use log::{debug, info};

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
            info!(
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
    info!("Destroying streams for engine handle: {}", engine_handle);
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
        info!("Destroying stream with texture id: {}", texture_id);
    }
    for texture_id in &to_remove {
        destroy_stream_session(*texture_id);
    }
}

pub fn destroy_stream_session(session_id: i64) {
    info!("Destroying stream session : {}", session_id);
    let active_sessions = get_all_sessions();
    debug!("Active sessions at destroy: {:?}", active_sessions);
    let session = remove_session(session_id);
    if let Some(holder) = session {
        info!(
            "Session {} removed from cache, destroying in a new thread",
            session_id
        );
        holder.terminate();
    } else {
        info!("No stream session found for session id: {}", session_id);
    }
}

pub async fn seek_session(session_id: i64, ts: i64) -> anyhow::Result<()> {
    let session = get_session(session_id);
    if let Some(session) = session {
        session.seek(ts).await?;
    }
    Ok(())
}

pub async fn wsc_rtp_live_session(session_id: i64) -> anyhow::Result<()> {
    if let Some(session) = get_session(session_id) {
        session.go_to_live_stream().await?;
    }
    Ok(())
}

pub fn set_speed_session(session_id: i64, speed: f64) -> anyhow::Result<()> {
    if let Some(session) = get_session(session_id) {
        session.set_speed(speed);
    }
    Ok(())
}

pub fn register_events_sink(session_id: i64, sink: types::DartEventsStream) {
    if let Some(session) = get_session(session_id) {
        session.set_events_sink(sink);
    }
}
