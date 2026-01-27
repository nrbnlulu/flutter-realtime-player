pub mod ffmpeg;
pub mod wsc_rtp;

use anyhow::Result;

use crate::{core::types::VideoDimensions, dart_types::StreamState};

#[derive(Debug, Clone)]
pub enum InputCommand {
    Resize { width: u32, height: u32 },
    Terminate,
    Seek { ts: i64 },
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    FrameAvailable,
    State(StreamState),
}

pub type InputCommandSender = flume::Sender<InputCommand>;
pub type InputCommandReceiver = flume::Receiver<InputCommand>;
pub type InputEventSender = flume::Sender<InputEvent>;
pub type InputEventReceiver = flume::Receiver<InputEvent>;

pub trait VideoInput: Send + Sync {
    fn execute(
        &self,
        event_tx: InputEventSender,
        command_rx: InputCommandReceiver,
        texture_id: i64,
    ) -> Result<()>;
    fn output_dimensions(&self) -> VideoDimensions;
}
