pub mod flutter;
pub mod payload;

pub trait FlutterTextureSession: Send + Sync {
    fn mark_frame_available(&self);
    fn terminate(&self);
}
