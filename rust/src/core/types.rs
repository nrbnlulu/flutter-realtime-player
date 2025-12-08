use crate::{dart_types::{StreamEvent, StreamState}, frb_generated::StreamSink};

#[derive(Debug, Clone, PartialEq, Eq)]
#[flutter_rust_bridge::frb(sync)]
pub struct VideoDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[flutter_rust_bridge::frb(sync)]
pub struct VideoInfo {
    pub uri: String,
    pub dimensions: VideoDimensions,
    pub framerate: Option<i32>,
    pub mute: bool,
    pub auto_restart: bool,
}

impl VideoInfo {
    pub fn new(
        uri: String,
        dimensions: VideoDimensions,
        framerate: Option<i32>,
        mute: Option<bool>,
        auto_restart: Option<bool>,
    ) -> Self {
        Self {
            uri,
            dimensions,
            framerate,
            mute: mute.unwrap_or(false),
            auto_restart: auto_restart.unwrap_or(false),
        }
    }
}

pub type DartStateStream = StreamSink<StreamState>;
pub type DartEventsStream = StreamSink<StreamEvent>;
