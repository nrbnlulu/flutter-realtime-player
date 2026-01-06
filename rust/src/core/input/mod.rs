pub mod ffmpeg;
pub mod trtp;

use anyhow::Result;

use crate::core::types::{DartStateStream, VideoDimensions};

pub trait VideoInput: Send + Sync {
    fn execute(&self, update_stream: DartStateStream, texture_id: i64) -> Result<()>;
    fn resize(&self, width: u32, height: u32) -> Result<()>;
    fn terminate(&self);
    fn seek(&self, ts: i64) -> Result<()>;
    fn output_dimensions(&self) -> VideoDimensions;
}
