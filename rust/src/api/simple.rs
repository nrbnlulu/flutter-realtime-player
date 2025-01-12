use std::{sync::Arc, thread};

use flume::{bounded, Receiver, Sender};
use irondash_engine_context::EngineContext;
use irondash_run_loop::RunLoop;
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


pub fn create_that_texture_please(engine_handle: i64) -> anyhow::Result<i64> {
    let (tx_texture_id, rx_texture_id) = bounded(1);


    RunLoop::sender_for_main_thread().unwrap().send(move || {
        if let Err(e) = EngineContext::get(){
            error!("Failed to get engine handle: {:?}", e);
        }
    let (tx, rx) = bounded(2);
    let provider = Arc::new(PixelBufferSource { rx });

    let texture = Texture::new_with_provider(engine_handle, provider).unwrap();
    tx_texture_id.send(texture.id()).expect("Failed to send texture");

    let texture_arc = texture.into_sendable_texture();
    std::thread::spawn(move || {
        get_images(tx, texture_arc).expect("Failed to get images");
    });

    });
    Ok(rx_texture_id.recv().unwrap())
}
