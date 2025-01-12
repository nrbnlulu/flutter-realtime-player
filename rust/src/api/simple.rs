use std::{sync::Arc, thread};

use flume::{bounded, Receiver, Sender};
use irondash_engine_context::EngineContext;
use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture, SimplePixelData, Texture};
use log::{error, info};
use simple_logger::SimpleLogger;

use crate::core::{get_images, PixelBufferSource};

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    SimpleLogger::new().init().unwrap();
    info!("Initializing app");
    // Default utilities - feel free to custom
    flutter_rust_bridge::setup_default_user_utils();
}

#[flutter_rust_bridge::frb(sync)]
pub fn create_that_texture_please(engine_handle: i64) -> anyhow::Result<i64> {
    info!("Current thread: {:?}", std::thread::current().id());
    info!("Current thread name: {:?}", std::thread::current().name());
    let (tx, rx) = bounded(2);
    if let Err(e) = EngineContext::get(){
        error!("Failed to get engine handle: {:?}", e);
        return Err(anyhow::anyhow!("Failed to set engine handle: {:?}", e));
    }
    let provider = Arc::new(PixelBufferSource { rx });
    info!("engine ptr {:?}", engine_handle);
    let texture = Texture::new_with_provider(engine_handle, provider).unwrap();
    let id = texture.id();
    let texture = texture.into_sendable_texture();
    info!("Texture ID: {}", id);
    
    std::thread::spawn(move || {
        get_images(tx, texture).unwrap();
    });

    Ok(id)
}
