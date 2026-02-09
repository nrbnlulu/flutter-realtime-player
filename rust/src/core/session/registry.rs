use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::SystemTime,
};

use log::{debug, info};

use crate::core::{
    input::{
        ffmpeg::FfmpegVideoInput,
        wsc_rtp::{self, WscRtpSessionConfig},
        InputCommandReceiver, InputCommandSender, InputEventReceiver, InputEventSender, VideoInput,
    },
    output::flutter_pixelbuffer::{create_flutter_pixelbuffer, FlutterPixelBufferHandle},
    session::{RawVideoSession, VideoSession, WscRtpVideoSession},
    types::{self, DartStateStream},
};

pub fn init() -> anyhow::Result<()> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("Failed to initialize ffmpeg: {:?}", e))?;
    info!("ffmpeg initialized: version {}", ffmpeg::version::version());
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Fatal);
    gstreamer::init().map_err(|e| anyhow::anyhow!("Failed to initialize GStreamer: {:?}", e))?;
    info!("GStreamer initialized");
    Ok(())
}

pub struct SessionHolder {
    inner: Mutex<Option<Box<dyn VideoSession>>>,
}

impl SessionHolder {
    fn new(session: Box<dyn VideoSession>) -> Self {
        Self {
            inner: Mutex::new(Some(session)),
        }
    }

    fn with_session_mut<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut dyn VideoSession) -> R,
    {
        let mut guard = self.inner.lock().unwrap();
        guard.as_mut().map(|session| f(session.as_mut()))
    }

    fn take(&self) -> Option<Box<dyn VideoSession>> {
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
    F: FnOnce(&mut dyn VideoSession) -> R,
{
    let holder = get_session(session_id)?;
    holder.with_session_mut(f)
}

pub fn insert_session(session_id: i64, session: Box<dyn VideoSession>) {
    SESSION_CACHE
        .write()
        .unwrap()
        .insert(session_id, Arc::new(SessionHolder::new(session)));
}

fn remove_session(session_id: i64) -> Option<Arc<SessionHolder>> {
    let mut session_cache = SESSION_CACHE.write().unwrap();
    session_cache.remove(&session_id)
}

fn build_input_and_output(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    update_stream: DartStateStream,
    ffmpeg_options: Option<HashMap<String, String>>,
    sdp_data: Option<Arc<Vec<u8>>>,
) -> anyhow::Result<(
    Arc<dyn VideoInput>,
    FlutterPixelBufferHandle,
    InputCommandReceiver,
    InputEventSender,
    i64,
)> {
    let (input_event_tx, input_event_rx): (InputEventSender, InputEventReceiver) =
        flume::unbounded();
    let (input_command_tx, input_command_rx): (InputCommandSender, InputCommandReceiver) =
        flume::unbounded();

    let (output_handle, payload_holder_weak, texture_id) = create_flutter_pixelbuffer(
        session_id,
        engine_handle,
        update_stream,
        input_event_rx,
        input_command_tx,
    )?;

    let input = FfmpegVideoInput::new(
        &video_info,
        session_id,
        ffmpeg_options,
        sdp_data,
        payload_holder_weak,
    );
    let input: Arc<dyn VideoInput> = input;

    Ok((
        input,
        output_handle,
        input_command_rx,
        input_event_tx,
        texture_id,
    ))
}

fn spawn_stream_thread(
    input: Arc<dyn VideoInput>,
    input_event_tx: InputEventSender,
    input_command_rx: InputCommandReceiver,
    texture_id: i64,
) {
    thread::spawn(move || {
        let _ = input.execute(input_event_tx, input_command_rx, texture_id);
    });
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
    let (input, output_handle, input_command_rx, input_event_tx, texture_id) =
        build_input_and_output(
            session_id,
            engine_handle,
            video_info,
            update_stream,
            ffmpeg_options,
            None,
        )?;

    let events_sink = Arc::new(Mutex::new(None));
    let session = RawVideoSession::new(
        session_id,
        engine_handle,
        output_handle,
        Arc::clone(&events_sink),
    );
    {
        insert_session(session_id, Box::new(session));
    }

    spawn_stream_thread(input, input_event_tx, input_command_rx, texture_id);

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
        info!(
            "Session {} removed from cache, destroying in a new thread",
            session_id
        );
        if let Some(session) = holder.take() {
            thread::spawn(move || session.destroy());
        }
        return;
    }
    info!("No stream session found for session id: {}", session_id);
}

pub fn resize_stream_session(session_id: i64, width: u32, height: u32) {
    match get_session_mut(session_id, |session| session.resize(width, height)) {
        Some(Ok(())) => {}
        Some(Err(e)) => log::warn!("Failed to resize session {}: {}", session_id, e),
        None => log::warn!(
            "Resize called for non-existent session {}, ignoring",
            session_id
        ),
    }
}

pub fn seek_session(session_id: i64, ts: i64) -> anyhow::Result<()> {
    get_session_mut(session_id, |session| session.seek(ts))
        .unwrap_or_else(|| Err(anyhow::anyhow!("Session not found: {}", session_id)))
}

pub fn wsc_rtp_live_session(session_id: i64) -> anyhow::Result<()> {
    get_session_mut(session_id, |session| session.go_to_live_stream())
        .unwrap_or_else(|| Err(anyhow::anyhow!("Session not found: {}", session_id)))
}

pub fn set_speed_session(session_id: i64, speed: f64) -> anyhow::Result<()> {
    get_session_mut(session_id, |session| session.set_speed(speed))
        .unwrap_or_else(|| Err(anyhow::anyhow!("Session not found: {}", session_id)))
}

pub fn register_events_sink(session_id: i64, sink: types::DartEventsStream) {
    get_session_mut(session_id, |session| session.set_events_sink(sink));
}
