pub mod flutter;
pub mod payload;

use anyhow::Result;

pub trait FlutterTextureSession: Send + Sync {
    fn mark_frame_available(&self);
    fn terminate(&self);
}
