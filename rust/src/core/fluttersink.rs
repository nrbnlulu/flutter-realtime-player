use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use log::{error, info, trace};

use crate::{
    core::{software_decoder::SoftwareDecoder, types::DartUpdateStream},
    utils::{invoke_on_platform_main_thread, LogErr},
};

use super::{software_decoder::SharedSendableTexture, types};

pub fn init() -> anyhow::Result<()> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("Failed to initialize ffmpeg: {:?}", e))?;
    ffmpeg::util::log::set_level(ffmpeg::util::log::Level::Fatal);
    Ok(())
}

lazy_static::lazy_static! {
static ref SESSION_CACHE: Mutex<HashMap<i64, (Arc<SoftwareDecoder>, i64, SharedSendableTexture)>> = Mutex::new(HashMap::new());
}

pub fn create_new_playable(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    update_stream: DartUpdateStream,
    ffmpeg_options: Option<HashMap<String, String>>,
) -> anyhow::Result<()> {
    let (decoding_manager, payload_holder) = SoftwareDecoder::new(&video_info, session_id, ffmpeg_options)?;

    let (sendable_texture, texture_id) =
        invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
            let texture =
                irondash_texture::Texture::new_with_provider(engine_handle, payload_holder)?;
            let texture_id = texture.id();
            Ok((texture.into_sendable_texture(), texture_id))
        })?;

    SESSION_CACHE.lock().unwrap().insert(
        session_id,
        (
            decoding_manager.clone(),
            engine_handle,
            sendable_texture.clone(),
        ),
    );

    thread::spawn(move || {
        if let Err(err) = decoding_manager.stream(sendable_texture, update_stream, texture_id){
            error!("Error streaming video: {}", err);
        }

        
    });

    Ok(())
}

pub fn destroy_engine_streams(engine_handle: i64) {
    info!("Destroying streams for engine handle: {}", engine_handle);
    let session_cache = SESSION_CACHE.lock().unwrap();
    let mut to_remove = vec![];
    for (texture_id, (decoder, handle, _)) in session_cache.iter() {
        if *handle == engine_handle {
            info!("Destroying stream with texture id: {}", texture_id);
            decoder.destroy_stream();
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
    if let Some((decoder, _, sendable_texture)) = session_cache.remove(&session_id) {
        decoder.destroy_stream();
        let mut retry_count = 0;
        const MAX_RETRIES: usize = 30;
        while retry_count < MAX_RETRIES {
            if Arc::strong_count(&decoder) == 1 {
                break;
            }
            info!(
                "Waiting for all references to be dropped for session id: {}. attempt({})",
                session_id, retry_count
            );
            thread::sleep(std::time::Duration::from_millis(100));
            retry_count += 1;
        }
        if retry_count == MAX_RETRIES {
            log::warn!("Forcefully dropped decoder for session id: {}, the texture is held somewhere else and may panic when unregistered if held on the wrong thread.", session_id);
        }
        invoke_on_platform_main_thread(move || {
            drop(sendable_texture);
            info!("Destroyed stream session for session id: {}", session_id);
        });
    } else {
        info!("No stream session found for session id: {}", session_id);
    }
}
