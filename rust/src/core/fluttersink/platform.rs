use std::sync::Arc;



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
            0: std::num::NonZeroU32::new(name_raw).ok_or(anyhow::anyhow!("couldn't create gl texture"))?,
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



#[cfg(target_os = "linux")]
mod linux {

    pub type IronDashGlTexture = irondash_texture::BoxedGLTexture;
    pub type ArcSendableTexture =
        Arc<irondash_texture::SendableTexture<IronDashGlTexture>>;


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
}
#[cfg(target_os = "windows")]
mod windows {
    use std::sync::Arc;
    pub type IronDashGlTexture = irondash_texture::BoxedTextureDescriptor<irondash_texture::DxgiSharedHandle>;
    pub type ArcSendableTexture =
        Arc<irondash_texture::SendableTexture<IronDashGlTexture>>;
    
    pub fn default_texture() -> IronDashGlTexture{
        
    }
    

}

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "windows")]
pub use windows::*;

use super::types::GlCtx;