pub mod utils;
use utils::LogErr;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use log::{info, trace};

use crate::{
    core::{software_decoder::SoftwareDecoder, types::{DartUpdateStream, StreamMessages}},
    utils::invoke_on_platform_main_thread,
};

use super::{software_decoder::SharedSendableTexture, types};

pub fn init() -> anyhow::Result<()> {
    ffmpeg::init().map_err(|e| anyhow::anyhow!("Failed to initialize ffmpeg: {:?}", e))
}

lazy_static::lazy_static! {
static ref SESSION_CACHE: Mutex<HashMap<i64, (Arc<SoftwareDecoder>, i64, SharedSendableTexture)>> = Mutex::new(HashMap::new());
}

pub fn create_new_playable(
    session_id: i64,
    engine_handle: i64,
    video_info: types::VideoInfo,
    update_stream: DartUpdateStream,
) -> anyhow::Result<()> {
    let (decoding_manager, payload_holder) = SoftwareDecoder::new(&video_info, session_id)?;
    decoding_manager.initialize_stream()?;
    // by now the stream is initialized successfully, we can create a flutter texture
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
    update_stream.add(
        StreamMessages::StreamAndTextureReady(texture_id)
    ).log_err();
    
    thread::spawn(move || {
        trace!("starting to stream on a new thread");
        decoding_manager.stream(sendable_texture, update_stream);
    });
    trace!("initialized; returning texture id: {}", texture_id);

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

pub fn destroy_stream_session(texture_id: i64) {
    info!("Destroying stream session for texture id: {}", texture_id);
    let mut session_cache = SESSION_CACHE.lock().unwrap();
    if let Some((decoder, _, sendable_texture)) = session_cache.remove(&texture_id) {
        decoder.destroy_stream();
        let mut retry_count = 0;
        const MAX_RETRIES: usize = 30;
        while retry_count < MAX_RETRIES {
            if Arc::strong_count(&decoder) == 1 {
                break;
            }
            info!(
                "Waiting for all references to be dropped for texture id: {}. attempt({})",
                texture_id, retry_count
            );
            thread::sleep(std::time::Duration::from_millis(100));
            retry_count += 1;
        }
        if retry_count == MAX_RETRIES {
            log::warn!("Forcefully dropped decoder for texture id: {}, the texture is held somewhere else and may panic when unregistered if held on the wrong thread.", texture_id);
        }
        invoke_on_platform_main_thread(move || {
            drop(sendable_texture);
            info!("Destroyed stream session for texture id: {}", texture_id);
        });
    } else {
        info!("No stream session found for texture id: {}", texture_id);
    }
}
