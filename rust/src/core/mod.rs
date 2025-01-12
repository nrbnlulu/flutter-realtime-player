use std::{sync::Arc, thread};

use flume::{bounded, Receiver, Sender};
use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture, SimplePixelData, Texture};
use log::info;

type Frame = (Vec<u8>, u32, u32);

pub struct PixelBufferSource {
    pub rx: Receiver<Frame>,
}

impl PayloadProvider<BoxedPixelData> for PixelBufferSource {
    fn get_payload(&self) -> BoxedPixelData {
        let (data, width, height) = self.rx.recv().unwrap();
        SimplePixelData::new_boxed(width as i32, height as i32, data)
    }
}
pub fn get_images(tx: Sender<Frame>, texture: Arc<SendableTexture<BoxedPixelData>>) -> anyhow::Result<()> {
    info!("Starting image stream");
    let img1_raw = include_bytes!("../assets/1.png").to_vec();
    let img2_raw = include_bytes!("../assets/2.png").to_vec();

    let rgba_converter = Arc::new(move |input: Vec<u8>| {
        let mut rgba = vec![0; input.len() * 4];
        for i in 0..input.len() {
            rgba[i * 4] = input[i];
            rgba[i * 4 + 1] = input[i];
            rgba[i * 4 + 2] = input[i];
            rgba[i * 4 + 3] = 255;
        }
        rgba
    });

    
    let img1 = rgba_converter(img1_raw);
    let img2 = rgba_converter(img2_raw);
    let (tx1 , rx) = bounded(2);
    tokio::spawn(async move {
        tx1.send((img1, 256, 256)).expect("Failed to send frame");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        tx1.send((img2, 256, 256)).expect("Failed to send frame");
    });


    while let Ok(frame) = rx.recv() {
        tx.send(frame).expect("Failed to send frame");
        texture.mark_frame_available();
    }
    Ok(())
}

