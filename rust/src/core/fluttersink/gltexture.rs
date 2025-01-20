use std::sync::Mutex;

use glow::HasContext;
use irondash_texture::{BoxedGLTexture, GLTextureProvider};
use log::info;

#[derive(Debug, Clone)]
pub struct GLTexture {
    pub target: u32,
    pub name_raw: u32,
    pub name: glow::Texture,
    pub width: i32,
    pub height: i32,
}

impl GLTexture {
    pub fn try_new(name_raw: u32, width: i32, height: i32) -> anyhow::Result<Self> {
        let name = glow::NativeTexture {
            0: std::num::NonZeroU32::new(name_raw).ok_or(anyhow::anyhow!("fdsaf"))?,
        };
        Ok(Self {
            target: glow::TEXTURE_2D,
            name_raw,
            name,
            width,
            height,
        })
    }
}

impl GLTextureProvider for GLTexture {
    fn get(&self) -> irondash_texture::GLTexture {
        info!("Returning GLTexture");

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
    gl_context: glow::Context,
    texture: Mutex<glow::Texture>,
    texture_name: Option<u32>,
}

impl GLTextureSource {
    pub fn init_gl_context() -> anyhow::Result<Self> {
        gl_loader::init_gl();
        let gl_context = unsafe {
            glow::Context::from_loader_function(|s| {
                std::mem::transmute(gl_loader::get_proc_address(s))
            })
        };
        if let Some(texture) = unsafe { gl_context.create_texture().ok() } {
            let texture_name = texture.0.get();
            unsafe {
                gl_context.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl_context.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    200,
                    500,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(&vec![0, 255, 0, 255].repeat(200 * 500))),
                );
                gl_context.bind_texture(glow::TEXTURE_2D, Some(texture));
            }

            unsafe {
                gdk_sys::gdk_gl_context_clear_current();
            }

            return Ok(Self {
                width: 400,
                height: 400,
                gl_context,
                texture: Mutex::new(texture),
                texture_name: Some(texture_name),
            });
        } else {
            return Err(anyhow::anyhow!("Failed to create texture"));
        }
    }
}

impl irondash_texture::PayloadProvider<BoxedGLTexture> for GLTextureSource {
    fn get_payload(&self) -> BoxedGLTexture {
        //let rng = fastrand::Rng::new();

        Box::new(
            GLTexture::try_new(
                self.texture_name.expect("Texture name not set"),
                self.width,
                self.height,
            )
            .unwrap(),
        )
    }
}

fn init() -> anyhow::Result<()> {
    gl_loader::init_gl();
    Ok(())
}
