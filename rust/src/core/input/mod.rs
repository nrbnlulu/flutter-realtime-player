pub mod wsc_rtp;

use anyhow::Result;

use crate::{core::types::VideoDimensions, dart_types::StreamState};

#[derive(Debug, Clone)]
pub enum InputCommand {
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
