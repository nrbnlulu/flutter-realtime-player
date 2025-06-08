use crate::{dart_types::StreamState, frb_generated::StreamSink};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoInfo {
    pub uri: String,
    pub dimensions: VideoDimensions,
    pub framerate: Option<i32>,
    pub mute: bool,
}

impl VideoInfo {
    pub fn new(
        uri: String,
        dimensions: VideoDimensions,
        framerate: Option<i32>,
        mute: Option<bool>,
    ) -> Self {
        Self {
            uri,
            dimensions,
            framerate,
            mute: mute.unwrap_or(false),
        }
    }
}

pub type DartUpdateStream = StreamSink<StreamState>;
