use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use flume::{bounded, Receiver, Sender};
use glib::types::StaticType;
use irondash_engine_context::EngineContext;
use irondash_run_loop::RunLoop;
use irondash_texture::{
    BoxedPixelData, PayloadProvider, SendableTexture, SimplePixelData, Texture,
};
use log::{error, info};
use simple_logger::SimpleLogger;

use crate::core::{fluttersink::{self, FlutterTextureSink}, get_images, GlSource};

#[flutter_rust_bridge::frb(sync)] // Synchronous mode for simplicity of the demo
pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    SimpleLogger::new().init().unwrap();
    info!("Initializing app");
    fluttersink::init().unwrap();

    // Default utilities - feel free to custom
    flutter_rust_bridge::setup_default_user_utils();
}

lazy_static::lazy_static! {

    static ref TEXTURES_REGISTRY: Mutex<HashMap<i64, Arc<SendableTexture<BoxedPixelData>>>> = Mutex::new(HashMap::new());
}


pub fn get_opengl_texture(engine_handle: i64) -> anyhow::Result<i64> {
    let (tx_texture_id, rx_texture_id) = bounded(1);

    RunLoop::sender_for_main_thread().unwrap().send(move || {

      let a =  create_ogl_texture(engine_handle);
      info!("sending texture id");
      tx_texture_id
          .send(a.unwrap())
          .expect("Failed to send texture");
    }
    );
    info!("waiting for texture id");
    Ok(rx_texture_id.recv().unwrap())

}

