use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use log::{debug, info};

use crate::{
    core::{software_decoder::SoftwareDecoder, types::DartUpdateStream},
    utils::invoke_on_platform_main_thread,
};

use super::{software_decoder::SharedSendableTexture, types};

pub fn init() -> anyhow::Result<()> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("Failed to initialize ffmpeg: {:?}", e))?;
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Fatal);
    Ok(())
}

pub struct SessionContext {
    pub decoder: Arc<SoftwareDecoder>,
    pub engine_handle: i64,
    pub sendable_texture: SharedSendableTexture,
    pub last_alive_mark: std::time::SystemTime,
}
lazy_static::lazy_static! {
pub static ref SESSION_CACHE: Mutex<HashMap<i64, SessionContext>> = Mutex::new(HashMap::new());
}
pub fn stream_alive_tester_task() {
    loop {
        let mut closed_sessions = Vec::new();

        {
            let session_cache = SESSION_CACHE.lock().unwrap();
            let now = std::time::SystemTime::now();
            for (session_id, ctx) in session_cache.iter() {
                if now.duration_since(ctx.last_alive_mark).unwrap().as_millis() > 5000 {
                    ctx.decoder.destroy_stream();
                    closed_sessions.push(*session_id);
                }
            }
        }
        if !closed_sessions.is_empty() {
            // drop the sessions on platform thread
            invoke_on_platform_main_thread(move || {
                let mut session_cache = SESSION_CACHE.lock().unwrap();
                info!(
                    "Closing sessions that was not pinged recently: {:?}",
                    closed_sessions
                );
                for session_id in closed_sessions {
                    session_cache.remove(&session_id);
                }
            });
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

pub fn create_new_playable(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    update_stream: DartUpdateStream,
    ffmpeg_options: Option<HashMap<String, String>>,
) -> anyhow::Result<()> {
    let (decoding_manager, payload_holder) =
        SoftwareDecoder::new(&video_info, session_id, ffmpeg_options);

    let (sendable_texture, texture_id) =
        invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
            let texture =
                irondash_texture::Texture::new_with_provider(engine_handle, payload_holder)?;
            let texture_id = texture.id();
            Ok((texture.into_sendable_texture(), texture_id))
        })?;

    // Set the sendable texture reference in the decoder for immediate updates during resize
    decoding_manager.set_sendable_texture(Arc::downgrade(&sendable_texture));

    SESSION_CACHE.lock().unwrap().insert(
        session_id,
        SessionContext {
            decoder: decoding_manager.clone(),
            engine_handle,
            sendable_texture: sendable_texture.clone(),
            last_alive_mark: std::time::SystemTime::now(),
        },
    );

    thread::spawn(move || {
        let _ = decoding_manager.stream(sendable_texture, update_stream, texture_id);
    });

    Ok(())
}

pub fn mark_session_alive(session_id: i64) {
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(ctx) = session_cache.get_mut(&session_id) {
        ctx.last_alive_mark = std::time::SystemTime::now();
    }
}

pub fn destroy_engine_streams(engine_handle: i64) {
    info!("Destroying streams for engine handle: {}", engine_handle);
    let session_cache = SESSION_CACHE.lock().unwrap();
    let mut to_remove = vec![];
    for (texture_id, ctx) in session_cache.iter() {
        if ctx.engine_handle == engine_handle {
            info!("Destroying stream with texture id: {}", texture_id);
            ctx.decoder.destroy_stream();
            to_remove.push(*texture_id);
        }
    }
    drop(session_cache); // Release the lock before destroying sessions
    for texture_id in &to_remove {
        destroy_stream_session(*texture_id);
    }
}

pub fn destroy_stream_session(session_id: i64) {
    info!("Destroying stream session : {}", session_id);
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(ctx) = session_cache.remove(&session_id) {
        ctx.decoder.destroy_stream();
        let mut retry_count = 0;
        const MAX_RETRIES: usize = 50;
        while retry_count < MAX_RETRIES {
            if Arc::strong_count(&ctx.decoder) == 1 {
                break;
            }
            debug!(
                "Waiting for all references to be dropped for session id: {}. attempt({})",
                session_id, retry_count
            );
            thread::sleep(std::time::Duration::from_millis(500));
            retry_count += 1;
        }
        if retry_count == MAX_RETRIES {
            log::error!("Forcefully dropped decoder for session id: {}, the texture is held somewhere else and may panic when unregistered if held on the wrong thread.", session_id);
        }
        invoke_on_platform_main_thread(move || {
            drop(ctx.sendable_texture);
            info!("Destroyed stream session for session id: {}", session_id);
        });
    } else {
        info!("No stream session found for session id: {}", session_id);
    }
}

pub fn seek_to_time(session_id: i64, time_seconds: f64) -> anyhow::Result<()> {
    let session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(ctx) = session_cache.get(&session_id) {
        ctx.decoder.seek_to(time_seconds);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Session not found: {}", session_id))
    }
}

pub fn get_current_time(session_id: i64) -> anyhow::Result<f64> {
    let session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(ctx) = session_cache.get(&session_id) {
        ctx.decoder.get_current_time()
    } else {
        Err(anyhow::anyhow!("Session not found: {}", session_id))
    }
}

pub fn resize_stream_session(session_id: i64, width: u32, height: u32) -> anyhow::Result<()> {
    let session_cache = SESSION_CACHE.lock().unwrap();
    if let Some(ctx) = session_cache.get(&session_id) {
        ctx.decoder.resize_stream(width, height)
    } else {
        Err(anyhow::anyhow!("Session not found: {}", session_id))
    }
}
