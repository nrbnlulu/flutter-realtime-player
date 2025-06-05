use std::sync::{Arc, Mutex};

use flutter_rust_bridge::DartFnFuture;
use serde::{Deserialize, Serialize};

use crate::frb_generated::StreamSink;

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

#[derive(Debug, Clone)]
pub enum StreamMessages {
    Error(String),
    Loading,
    Playing,
    Stopped,
    StreamAndTextureReady(i64),
}

pub type DartUpdateStream = StreamSink<StreamMessages>;
