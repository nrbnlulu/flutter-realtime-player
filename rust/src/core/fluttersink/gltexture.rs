use std::{
    cell::RefCell, collections::HashMap, ops::Deref, rc::Rc, sync::{Arc, Mutex, RwLock}
};

use irondash_texture::{BoxedGLTexture, GLTextureProvider};
use log::error;

use crate::core::{fluttersink::frame, platform::GlCtx};

use super::{ utils, SinkEvent};



#[derive(Debug, Clone)]
pub struct GLTexture {
    pub target: u32,
    pub name_raw: u32,
    pub name: glow::Texture,
    pub width: i32,
    pub height: i32,
    gl_ctx: GlCtx,
}

impl GLTexture {
    pub fn try_new(name_raw: u32, width: i32, height: i32, gl_ctx: GlCtx) -> anyhow::Result<Self> {
        let name = glow::NativeTexture {
            0: std::num::NonZeroU32::new(name_raw).ok_or(anyhow::anyhow!("fdsaf"))?,
        };
        Ok(Self {
            target: glow::TEXTURE_2D,
            name_raw,
            name,
            width,
            height,
            gl_ctx,
        })
    }

    pub fn from_glow(name: glow::Texture, width: i32, height: i32, gl_ctx: GlCtx) -> Self {
        Self {
            target: glow::TEXTURE_2D,
            name_raw: name.0.get(),
            name,
            width,
            height,
            gl_ctx,
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


pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}



pub struct GLTextureSource {
    width: i32,
    height: i32,
    texture_receiver: flume::Receiver<SinkEvent>,
    cached_textures: Mutex<HashMap<frame::TextureCacheId, GLTexture>>,
    gl_context: GlCtx,
    gl: gl::Gl,
}

unsafe impl Sync for GLTextureSource {}
unsafe impl Send for GLTextureSource {}



impl GLTextureSource {
    pub(crate) fn new(texture_receiver: flume::Receiver<SinkEvent>, 
        gl: glutin::api::glx::Glx,
    
    ) -> anyhow::Result<Self> {

        Ok(Self {
            width: 0,
            height: 0,
            texture_receiver,
            cached_textures: Mutex::new(HashMap::new()),
            gl_context: Rc::new(gl_context),
        })
    }

    fn recv_frame(&self) -> anyhow::Result<BoxedGLTexture> {
        match self.texture_receiver.recv() {
            Ok(SinkEvent::FrameChanged(native_frame)) => {
       
                let mut texture_name = 0;
                unsafe { self.gl.GenTextures(1, &mut texture_name);
                
                self.gl.BindTexture(gl::TEXTURE_2D, texture_name);
                self.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                self.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                self.gl.EGLImageTargetTexture2DOES(gl::TEXTURE_2D, native_frame.image_ptr as *const std::ffi::c_void);
                 };

                trace!(unsafe { self.gl.GetError() });
                unsafe { self.gl.BindRenderbuffer(gl::RENDERBUFFER, texture_name) }
                trace!(unsafe { self.gl.GetError() });
                unsafe { self.gl.EGLImageTargetRenderbufferStorageOES(gl::RENDERBUFFER, image) };
                trace!(unsafe { self.gl.GetError() });
                unsafe {
                    self.gl.FramebufferRenderbuffer(
                        gl::FRAMEBUFFER,
                        gl::COLOR_ATTACHMENT0,
                        gl::RENDERBUFFER,
                        texture_name,
                    )
                }
            

                // let mut cached_textures = self
                //     .cached_textures
                //     .lock()
                //     .map_err(|e| anyhow::anyhow!(e.to_string()))?;

                // if let Ok(textures) =
                //     frame.into_textures(self.gl_context.clone(), &mut cached_textures)
                // {
                //     let flip_width_height =
                //         |(width, height, orientation): (u32, u32, frame::Orientation)| {
                //             if orientation.is_flip_width_height() {
                //                 (height, width)
                //             } else {
                //                 (width, height)
                //             }
                //         };

                //     let new_size = textures
                //         .first()
                //         .map(|p| {
                //             flip_width_height((
                //                 f32::round(p.width) as u32,
                //                 f32::round(p.height) as u32,
                //                 p.orientation,
                //             ))
                //         })
                //         .ok_or_else(|| anyhow::anyhow!("Failed to get new size"))?;

                    // let old_paintables = self.paintables.replace(new_paintables);
                    // let old_size = old_paintables.first().map(|p| {
                    //     flip_width_height((
                    //         f32::round(p.width) as u32,
                    //         f32::round(p.height) as u32,
                    //         p.orientation,
                    //     ))
                    // });

                    // if Some(new_size) != old_size {
                    //     debug!("Size changed from {old_size:?} to {new_size:?}",);
                    // }
                    if let Some(first_frame) = textures.first() {
                        return Ok(Box::new(first_frame.texture.clone()));
                    };
                }
                Err(anyhow::anyhow!("Failed to get first frame"))
            }
            Err(e) => {
                Err(anyhow::anyhow!(
                    "Error receiving frame changed event {:?}",
                    e
                ))
            }
        }

    fn get_fallback_texture(&self) -> BoxedGLTexture {
        Box::new(
            GLTexture::try_new(
                self.green_texture_name,
                self.width,
                self.height,
                self.gl_context.clone(),
            )
            .unwrap(),
        )
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
