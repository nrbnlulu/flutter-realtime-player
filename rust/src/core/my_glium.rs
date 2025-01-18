use std::{sync::{Arc, Mutex}, thread};

use glow::{HasContext, NativeProgram, TEXTURE_2D};
use irondash_texture::{BoxedGLTexture, GLTextureProvider, PayloadProvider, Texture};
use simple_log::info;

use super::fluttersink::gltexture::{GLTexture, GLTextureSource};


pub fn create_ogl_texture(engine_handle: i64) -> anyhow::Result<i64> {
    let provider = Arc::new(GLTextureSource::init_gl_context().unwrap());
    let texture = Texture::new_with_provider(engine_handle, provider.clone())?;
    let id = texture.id();
    let sendable_texture = texture.into_sendable_texture();
    thread::spawn(move || loop {
        sendable_texture.mark_frame_available();
        thread::sleep(std::time::Duration::from_secs(1));
    });
    Ok(id)
}
