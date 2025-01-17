use irondash_texture::GLTextureProvider;
use log::info;

#[derive(Debug, Clone)]
pub struct GLTexture {
    pub target: u32,
    pub name: glow::Texture,
    pub width: i32,
    pub height: i32,
}

impl GLTexture {
    pub fn new(name: glow::Texture, width: i32, height: i32) -> Self {
        Self {
            target: glow::TEXTURE_2D,
            name,
            width,
            height,
        }
    }
}

impl GLTextureProvider for GLTexture {
    fn get(&self) -> irondash_texture::GLTexture {
        info!("Returning GLTexture");
        irondash_texture::GLTexture {
            target: self.target,
            name: &self.name,
            width: self.width,
            height: self.height,
        }
    }
}
