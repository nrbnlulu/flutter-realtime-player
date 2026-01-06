pub mod flutter;
pub mod payload;

use anyhow::Result;

pub trait FlutterTextureSession: Send + Sync {
    fn mark_frame_available(&self);
    fn resize(&self, width: u32, height: u32) -> Result<()>;
    fn terminate(&self);
}
