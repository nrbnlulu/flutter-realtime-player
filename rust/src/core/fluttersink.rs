use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::SystemTime,
};

use log::info;

use crate::{
    core::{
        session::{BaseSession, SessionLifecycle},
        software_decoder::SoftwareDecoder,
        tsdp,
        types::{self, DartStateStream},
    },
    utils::invoke_on_platform_main_thread,
};

use super::software_decoder::SharedSendableTexture;

pub fn init() -> anyhow::Result<()> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("Failed to initialize ffmpeg: {:?}", e))?;
    info!("ffmpeg initialized: version {}", ffmpeg::version::version());
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Fatal);
    Ok(())
}

lazy_static::lazy_static! {
    pub static ref SESSION_CACHE: Mutex<HashMap<i64, Box<dyn SessionLifecycle>>> =
        Mutex::new(HashMap::new());
}

fn build_decoder(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    ffmpeg_options: Option<HashMap<String, String>>,
    sdp_data: Option<Arc<Vec<u8>>>,
) -> anyhow::Result<(Arc<SoftwareDecoder>, SharedSendableTexture, i64)> {
    let (decoding_manager, payload_holder) =
        SoftwareDecoder::new(&video_info, session_id, ffmpeg_options, sdp_data);

    let (sendable_texture, texture_id) =
        invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
            let texture =
                irondash_texture::Texture::new_with_provider(engine_handle, payload_holder)?;
            let texture_id = texture.id();
            Ok((texture.into_sendable_texture(), texture_id))
        })?;

    // Set the sendable texture reference in the decoder for immediate updates during resize
    decoding_manager.set_sendable_texture(Arc::downgrade(&sendable_texture));

    Ok((decoding_manager, sendable_texture, texture_id))
}

fn spawn_stream_thread(
    decoder: Arc<SoftwareDecoder>,
    sendable_texture: SharedSendableTexture,
    update_stream: DartStateStream,
    texture_id: i64,
) {
    thread::spawn(move || {
        let _ = decoder.stream(sendable_texture, update_stream, texture_id);
    });
}

fn apply_tsdp_ffmpeg_defaults(options: &mut HashMap<String, String>) {
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

        {
            let mut session_cache = SESSION_CACHE.lock().unwrap();
            let now = SystemTime::now();
            for (session_id, session) in session_cache.iter_mut() {
                let expired = now
                    .duration_since(session.last_alive_mark())
                    .map(|duration| duration.as_millis() > 5000)
                    .unwrap_or(false);
                if expired {
                    session.terminate();
                    closed_sessions.push(*session_id);
                }
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
    let (decoder, sendable_texture, texture_id) =
        build_decoder(session_id, engine_handle, video_info, ffmpeg_options, None)?;

    let session = BaseSession::new(
        session_id,
        decoder.clone(),
        engine_handle,
        sendable_texture.clone(),
    );
    SESSION_CACHE
        .lock()
        .unwrap()
        .insert(session_id, Box::new(session));

    spawn_stream_thread(decoder, sendable_texture, update_stream, texture_id);

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
    let tsdp_setup = tsdp::setup_tsdp_session(&endpoint)?;

    let sdp_data = Arc::clone(&tsdp_setup.sdp_data);
    video_info.uri = format!("tsdp://{}/{}", endpoint.base_url, endpoint.source_id);
    let mut options = ffmpeg_options.unwrap_or_default();
    apply_tsdp_ffmpeg_defaults(&mut options);
    options
        .entry("local_rtpport".to_string())
        .or_insert_with(|| tsdp_setup.client_port.to_string());
    options
        .entry("local_rtcpport".to_string())
        .or_insert_with(|| (tsdp_setup.client_port + 1).to_string());

    let (decoder, sendable_texture, texture_id) = match build_decoder(
        session_id,
        engine_handle,
        video_info,
        Some(options),
        Some(Arc::clone(&sdp_data)),
    ) {
        Ok(value) => value,
        Err(err) => {
            tsdp_setup.cleanup();
            return Err(err);
        }
    };

    let base_session = BaseSession::new(
        session_id,
        decoder.clone(),
        engine_handle,
        sendable_texture.clone(),
    );
    let session = tsdp::TsdpSession::new(base_session, tsdp_setup);
    SESSION_CACHE
        .lock()
        .unwrap()
        .insert(session_id, Box::new(session));

    spawn_stream_thread(decoder, sendable_texture, update_stream, texture_id);

    Ok(())
}

pub fn mark_session_alive(session_id: i64) {
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(session) = session_cache.get_mut(&session_id) {
        session.make_alive();
    }
}

pub fn destroy_engine_streams(engine_handle: i64) {
    info!("Destroying streams for engine handle: {}", engine_handle);
    let session_cache = SESSION_CACHE.lock().unwrap();
    let mut to_remove = vec![];
    for (texture_id, session) in session_cache.iter() {
        if session.engine_handle() == engine_handle {
            info!("Destroying stream with texture id: {}", texture_id);
            to_remove.push(*texture_id);
        }
    }
    drop(session_cache);
    for texture_id in &to_remove {
        destroy_stream_session(*texture_id);
    }
}

pub fn destroy_stream_session(session_id: i64) {
    info!("Destroying stream session : {}", session_id);
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(session) = session_cache.remove(&session_id) {
        session.destroy();
    } else {
        info!("No stream session found for session id: {}", session_id);
    }
}

pub fn resize_stream_session(session_id: i64, width: u32, height: u32) -> anyhow::Result<()> {
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(session) = session_cache.get_mut(&session_id) {
        session.resize(width, height)
    } else {
        Err(anyhow::anyhow!("Session not found: {}", session_id))
    }
}
