use std::sync::Mutex;

use glow::HasContext;
use irondash_texture::{BoxedGLTexture, GLTextureProvider};
use log::{debug, info};

use super::SinkEvent;

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
    texture_receiver: flume::Receiver<SinkEvent>,
    green_texture_name: u32,
}

impl GLTextureSource {
    pub fn new(texture_receiver: flume::Receiver<SinkEvent>) -> anyhow::Result<Self> {
        gl_loader::init_gl();
        let gl_context = unsafe {
            glow::Context::from_loader_function(|s| {
                std::mem::transmute(gl_loader::get_proc_address(s))
            })
        };
        let mut texture_name = 0;

        if let Some(texture) = unsafe { gl_context.create_texture().ok() } {
            texture_name = texture.0.get();
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
        }
        Ok(Self {
            width: 0,
            height: 0,
            texture_receiver,
            green_texture_name: texture_name,
        })
    }
}

impl irondash_texture::PayloadProvider<BoxedGLTexture> for GLTextureSource {
    fn get_payload(&self) -> BoxedGLTexture {
        match self.texture_receiver.recv() {
            Ok(SinkEvent::FrameChanged(frame)) => {
                debug!("PayloadProvider: Frame changed");
                let context = self.gl_ctx.borrow();


                let new_paintables = match frame
                    .into_textures(context.as_ref(), &mut self.cached_textures.borrow_mut())
                {
                    Ok(textures) => textures,
                    Err(err) => {
                        gst::element_error!(
                            sink,
                            gst::ResourceError::Failed,
                            ["Failed to transform frame into textures: {err}"]
                        );
                        return;
                    }
                };

                let flip_width_height =
                    |(width, height, orientation): (u32, u32, frame::Orientation)| {
                        if orientation.is_flip_width_height() {
                            (height, width)
                        } else {
                            (width, height)
                        }
                    };

                let new_size = new_paintables
                    .first()
                    .map(|p| {
                        flip_width_height((
                            f32::round(p.width) as u32,
                            f32::round(p.height) as u32,
                            p.orientation,
                        ))
                    })
                    .unwrap();

                let old_paintables = self.paintables.replace(new_paintables);
                let old_size = old_paintables.first().map(|p| {
                    flip_width_height((
                        f32::round(p.width) as u32,
                        f32::round(p.height) as u32,
                        p.orientation,
                    ))
                });

                if Some(new_size) != old_size {
                    gst::debug!(
                        CAT,
                        imp = self,
                        "Size changed from {old_size:?} to {new_size:?}",
                    );
                    self.obj().invalidate_size();
                }

                self.obj().invalidate_contents();
            }
            Err(e) => {
                info!("Error receiving frame changed event: {:?}", e);
            }
        }
        // fallback to a green screen

        Box::new(GLTexture::try_new(self.green_texture_name, self.width, self.height).unwrap())
    }
}

fn init() -> anyhow::Result<()> {
    gl_loader::init_gl();
    Ok(())
}
