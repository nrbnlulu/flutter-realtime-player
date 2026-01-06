use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::SystemTime,
};

use log::{debug, info};

use crate::{
    core::{
        input::{ffmpeg::FfmpegVideoInput, trtp, VideoInput},
        session::{FlutterVideoSession, SessionLifecycle},
        texture::{
            flutter::{SharedSendableTexture, TextureSession},
            payload::PayloadHolder,
            FlutterTextureSession,
        },
        types::{self, DartStateStream},
    },
    utils::invoke_on_platform_main_thread,
};

pub fn init() -> anyhow::Result<()> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("Failed to initialize ffmpeg: {:?}", e))?;
    info!("ffmpeg initialized: version {}", ffmpeg::version::version());
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Fatal);
    Ok(())
}

pub struct SessionHolder {
    inner: Mutex<Option<Box<dyn SessionLifecycle>>>,
}

impl SessionHolder {
    fn new(session: Box<dyn SessionLifecycle>) -> Self {
        Self {
            inner: Mutex::new(Some(session)),
        }
    }

    fn with_session_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut dyn SessionLifecycle) -> R,
    {
        let mut guard = self.inner.lock().unwrap();
        guard.as_mut().map(|session| f(session.as_mut()))
    }

    fn take(&self) -> Option<Box<dyn SessionLifecycle>> {
        let mut guard = self.inner.lock().unwrap();
        guard.take()
    }
}

lazy_static::lazy_static! {
    static ref SESSION_CACHE: RwLock<HashMap<i64, Arc<SessionHolder>>> =
        RwLock::new(HashMap::new());
}

pub fn get_all_sessions() -> Vec<i64> {
    let session_cache = SESSION_CACHE.read().unwrap();
    session_cache.keys().copied().collect()
}

pub fn get_session(session_id: i64) -> Option<Arc<SessionHolder>> {
    let session_cache = SESSION_CACHE.read().unwrap();
    session_cache.get(&session_id).cloned()
}

pub fn get_session_mut<F, R>(session_id: i64, f: F) -> Option<R>
where
    F: FnOnce(&mut dyn SessionLifecycle) -> R,
{
    let holder = get_session(session_id)?;
    holder.with_session_mut(f)
}

pub fn insert_session(session_id: i64, session: Box<dyn SessionLifecycle>) {
    SESSION_CACHE
        .write()
        .unwrap()
        .insert(session_id, Arc::new(SessionHolder::new(session)));
}

fn remove_session(session_id: i64) -> Option<Arc<SessionHolder>> {
    let mut session_cache = SESSION_CACHE.write().unwrap();
    session_cache.remove(&session_id)
}

fn build_input_and_texture(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    ffmpeg_options: Option<HashMap<String, String>>,
    sdp_data: Option<Arc<Vec<u8>>>,
) -> anyhow::Result<(
    Arc<dyn VideoInput>,
    Arc<dyn FlutterTextureSession>,
    SharedSendableTexture,
    i64,
)> {
    let payload_holder = Arc::new(PayloadHolder::new());
    let payload_holder_weak = Arc::downgrade(&payload_holder);
    let payload_holder_for_texture = Arc::clone(&payload_holder);

    let (sendable_texture, texture_id) =
        invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
            let texture = irondash_texture::Texture::new_with_provider(
                engine_handle,
                payload_holder_for_texture,
            )?;
            let texture_id = texture.id();
            Ok((texture.into_sendable_texture(), texture_id))
        })?;

    let texture_session = Arc::new(TextureSession::new(
        texture_id,
        Arc::downgrade(&sendable_texture),
        payload_holder_weak.clone(),
    ));
    let texture_session: Arc<dyn FlutterTextureSession> = texture_session;

    let input = FfmpegVideoInput::new(
        &video_info,
        session_id,
        ffmpeg_options,
        sdp_data,
        payload_holder_weak,
        Arc::downgrade(&texture_session),
    );
    let input: Arc<dyn VideoInput> = input;

    Ok((input, texture_session, sendable_texture, texture_id))
}

fn spawn_stream_thread(
    input: Arc<dyn VideoInput>,
    update_stream: DartStateStream,
    texture_id: i64,
) {
    thread::spawn(move || {
        let _ = input.execute(update_stream, texture_id);
    });
}

fn apply_trtp_ffmpeg_defaults(options: &mut HashMap<String, String>) {
    options
        .entry("protocol_whitelist".to_string())
        .or_insert_with(|| "file,udp,rtp".to_string());
    options
        .entry("fflags".to_string())
        .or_insert_with(|| "nobuffer".to_string());
    options
        .entry("flags".to_string())
        .or_insert_with(|| "low_delay".to_string());
    options
        .entry("analyzeduration".to_string())
        .or_insert_with(|| "0".to_string());
    options
        .entry("probesize".to_string())
        .or_insert_with(|| "32".to_string());
}

pub fn stream_alive_tester_task() {
    loop {
        let mut closed_sessions = Vec::new();

        let holders = get_all_sessions()
            .into_iter()
            .filter_map(|session_id| get_session(session_id).map(|holder| (session_id, holder)))
            .collect::<Vec<_>>();
        let now = SystemTime::now();
        for (session_id, holder) in holders {
            let expired = holder
                .with_session_mut(|session| {
                    now.duration_since(session.last_alive_mark())
                        .map(|duration| duration.as_millis() > 5000)
                        .unwrap_or(false)
                })
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
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn create_new_playable(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    update_stream: DartStateStream,
    ffmpeg_options: Option<HashMap<String, String>>,
) -> anyhow::Result<()> {
    info!(
        "Creating new playable: session_id={}, engine_handle={}",
        session_id, engine_handle
    );
    let (input, texture_session, sendable_texture, texture_id) =
        build_input_and_texture(session_id, engine_handle, video_info, ffmpeg_options, None)?;

    let session = FlutterVideoSession::new(
        session_id,
        engine_handle,
        Arc::clone(&input),
        Arc::clone(&texture_session),
        sendable_texture,
        None,
    );
    {
        insert_session(session_id, Box::new(session));
    }

    spawn_stream_thread(input, update_stream, texture_id);

    Ok(())
}

pub fn create_tsdp_playable(
    session_id: i64,
    engine_handle: i64,
    endpoint: types::TsdpEndpoint,
    mut video_info: types::VideoInfo,
    update_stream: DartStateStream,
    ffmpeg_options: Option<HashMap<String, String>>,
) -> anyhow::Result<()> {
    info!(
        "Creating TRTP playable: session_id={}, engine_handle={}, source_id={}",
        session_id, engine_handle, endpoint.source_id
    );
    let tsdp_setup = trtp::setup_tsdp_session(&endpoint)?;

    let sdp_data = Arc::clone(&tsdp_setup.sdp_data);
    video_info.uri = format!("tsdp://{}/{}", endpoint.base_url, endpoint.source_id);
    let mut options = ffmpeg_options.unwrap_or_default();
    apply_trtp_ffmpeg_defaults(&mut options);
    options
        .entry("local_rtpport".to_string())
        .or_insert_with(|| tsdp_setup.client_port.to_string());
    options
        .entry("local_rtcpport".to_string())
        .or_insert_with(|| (tsdp_setup.client_port + 1).to_string());

    let build_result = build_input_and_texture(
        session_id,
        engine_handle,
        video_info,
        Some(options),
        Some(Arc::clone(&sdp_data)),
    );
    let (input, texture_session, sendable_texture, texture_id) = match build_result {
        Ok(value) => value,
        Err(err) => {
            tsdp_setup.cleanup();
            return Err(err);
        }
    };

    let session = FlutterVideoSession::new(
        session_id,
        engine_handle,
        Arc::clone(&input),
        Arc::clone(&texture_session),
        sendable_texture,
        Some(tsdp_setup.refresh_tx),
    );
    {
        insert_session(session_id, Box::new(session));
    }
    spawn_stream_thread(input, update_stream, texture_id);

    Ok(())
}

pub fn mark_session_alive(session_id: i64) {
    log::trace!("mark_session_alive {}", session_id);
    get_session_mut(session_id, |session| session.make_alive());
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
            let matches = holder
                .with_session_mut(|session| session.engine_handle() == engine_handle)
                .unwrap_or(false);
            if matches {
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
        info!("Session {} removed from cache, destroying in a new thread", session_id);
        if let Some(session) = holder.take() {
           thread::spawn(move || session.destroy());
        }
        return;
    }
    info!("No stream session found for session id: {}", session_id);
}

pub fn resize_stream_session(session_id: i64, width: u32, height: u32) -> anyhow::Result<()> {
    get_session_mut(session_id, |session| session.resize(width, height))
        .unwrap_or_else(|| Err(anyhow::anyhow!("Session not found: {}", session_id)))
    
}


pub fn seek_session(session_id: i64, ts: i64) -> anyhow::Result<()> {
    get_session_mut(session_id, |session| session.seek(ts))
        .unwrap_or_else(|| Err(anyhow::anyhow!("Session not found: {}", session_id)))
}

pub fn register_events_sink(session_id: i64, sink: types::DartEventsStream) {
    get_session_mut(session_id, |session| session.set_events_sink(sink));
}
