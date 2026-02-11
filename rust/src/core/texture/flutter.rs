use std::sync::{Arc, Weak};

use anyhow::Result;
use irondash_texture::SendableTexture;

use crate::core::texture::{payload::PayloadHolder, FlutterTextureSession};

pub type SharedSendableTexture = Arc<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
pub type WeakSendableTexture = Weak<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;

pub struct TextureSession {
    texture_id: i64,
    weak_texture: WeakSendableTexture,
    payload_holder: Weak<PayloadHolder>,
}

impl TextureSession {
    pub fn new(
        texture_id: i64,
        weak_texture: WeakSendableTexture,
        payload_holder: Weak<PayloadHolder>,
    ) -> Self {
        Self {
            texture_id,
            weak_texture,
            payload_holder,
        }
    }

    pub fn texture_id(&self) -> i64 {
        self.texture_id
    }

    pub fn payload_holder(&self) -> Weak<PayloadHolder> {
        Weak::clone(&self.payload_holder)
    }
}

impl FlutterTextureSession for TextureSession {
    fn mark_frame_available(&self) {
        if let Some(texture) = self.weak_texture.upgrade() {
            texture.mark_frame_available();
        }
    }

    fn terminate(&self) {
        // nothing to do here now.
    }
}
