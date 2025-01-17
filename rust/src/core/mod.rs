use std::{fs::File, sync::Arc, thread};
mod gl;
pub mod fluttersink;
pub mod my_glium;
use flume::{bounded, Receiver, Sender};
use flutter_rust_bridge::BaseAsyncRuntime;
use image::{ImageBuffer, ImageReader, RgbaImage};
use irondash_texture::{
    BoxedPixelData, PayloadProvider, SendableTexture, SimplePixelData, Texture,
};
use log::info;

use crate::frb_generated::FLUTTER_RUST_BRIDGE_HANDLER;

type Frame = (Vec<u8>, u32, u32);

pub struct GlSource {
    pub rx: Receiver<Frame>,
}

impl PayloadProvider<BoxedPixelData> for GlSource {
    fn get_payload(&self) -> BoxedPixelData {
        let (data, width, height) = self.rx.recv().unwrap();
        SimplePixelData::new_boxed(width as i32, height as i32, data)
    }
}

fn get_png_bytes(path: &str) -> anyhow::Result<RgbaImage> {
    let path = std::path::Path::new(path);
    Ok(image::open(path)?.to_rgba8())
}

pub fn get_images(
    tx: Sender<Frame>,
    texture: Arc<SendableTexture<BoxedPixelData>>,
) -> anyhow::Result<()> {
    info!("Starting image stream");

    loop {
        let img1 = get_png_bytes("./assets/1.png").expect("Failed to load image");
        let frame1 = Arc::new((img1.to_vec(), img1.width(), img1.height()));
        tx.send(Arc::try_unwrap(frame1).unwrap())
            .expect("Failed to send frame");
        texture.mark_frame_available();
        info!("Sent frame 1");
        thread::sleep(std::time::Duration::from_secs(1));
        let img2 = get_png_bytes("./assets/2.jpg").expect("Failed to load image");
        let frame2 = Arc::new((img2.to_vec(), img2.width(), img2.height()));
        tx.send(Arc::try_unwrap(frame2).unwrap())
            .expect("Failed to send frame");
        info!("Sent frame 2");
        texture.mark_frame_available();
        thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
}
