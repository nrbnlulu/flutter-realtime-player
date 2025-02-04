use irondash_texture::GLTextureProvider;

use crate::core::gl::{self};

#[derive(Debug, Clone)]
pub struct GLTexture {
    pub target: u32,
    pub name_raw: u32,
    pub width: i32,
    pub height: i32,
}

impl GLTexture {
    pub fn new(name_raw: u32, width: i32, height: i32) -> Self {
        Self {
            target: gl::TEXTURE_2D,
            name_raw,
            width,
            height,
        }
    }
}

impl GLTextureProvider for GLTexture {
    fn get(&self) -> irondash_texture::GLTexture {
        irondash_texture::GLTexture {
            target: self.target,
            name: &self.name_raw,
            width: self.width,
            height: self.height,
        }
    }
}
