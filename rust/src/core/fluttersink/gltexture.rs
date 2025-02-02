use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use gst::meta::tags::Memory;
use irondash_texture::{BoxedGLTexture, GLTextureProvider};
use log::{error, trace};

use crate::core::{
    fluttersink::frame,
    gl::{self, GL},
    platform::GlCtx,
};

use super::{frame::ResolvedFrame, utils, SinkEvent};

#[derive(Debug, Clone)]
pub struct GLTexture {
    pub target: u32,
    pub name_raw: u32,
    pub width: i32,
    pub height: i32,
}

impl GLTexture {
    pub fn try_new(name_raw: u32, width: i32, height: i32) -> anyhow::Result<Self> {
        Ok(Self {
            target: gl::TEXTURE_2D,
            name_raw,
            width,
            height,
        })
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

pub struct GLTextureSource {
    width: i32,
    height: i32,
    texture_receiver: flume::Receiver<SinkEvent>,
    cached_textures: Mutex<HashMap<frame::TextureCacheId, GLTexture>>,
}

unsafe impl Sync for GLTextureSource {}
unsafe impl Send for GLTextureSource {}

impl GLTextureSource {
    pub(crate) fn new(texture_receiver: flume::Receiver<SinkEvent>) -> anyhow::Result<Self> {
        Ok(Self {
            width: 0,
            height: 0,
            texture_receiver,
            cached_textures: Mutex::new(HashMap::new()),
        })
    }

    fn recv_frame(&self) -> anyhow::Result<BoxedGLTexture> {
        match self.texture_receiver.recv() {
            Ok(SinkEvent::FrameChanged(resolved_frame)) => match resolved_frame {
                ResolvedFrame::Memory(_) => unimplemented!("Memory"),
                ResolvedFrame::GL((egl_image, pixel_res)) => {
                    let mut texture_name = 0;
                    unsafe {
                        GL.GenTextures(1, &mut texture_name);

                        GL.BindTexture(gl::TEXTURE_2D, texture_name);
                        GL.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                        GL.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                        GL.EGLImageTargetTexture2DOES(gl::TEXTURE_2D, egl_image.image.get_image());
                    };
                    Ok(Box::new(GLTexture::try_new(
                        texture_name,
                        egl_image.width as i32,
                        egl_image.height as i32,
                    )?))
                }
            },
            Err(e) => Err(anyhow::anyhow!(
                "Error receiving frame changed event {:?}",
                e
            )),
        }
    }

    fn get_fallback_texture(&self) -> BoxedGLTexture {
        Box::new(GLTexture::try_new(0, self.width, self.height).unwrap())
    }
}

impl irondash_texture::PayloadProvider<BoxedGLTexture> for GLTextureSource {
    fn get_payload(&self) -> BoxedGLTexture {
        self.recv_frame().unwrap_or_else(|e| {
            error!("Error receiving frame: {:?}", e);
            self.get_fallback_texture()
        })
    }
}

fn init() -> anyhow::Result<()> {
    gl_loader::init_gl();
    Ok(())
}
