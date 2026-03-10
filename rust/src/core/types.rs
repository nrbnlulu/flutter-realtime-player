use crate::{
    dart_types::{StreamEvent, StreamState},
    frb_generated::StreamSink,
};

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
    pub framerate: Option<i32>,
    pub mute: bool,
    pub auto_restart: bool,
}

impl VideoInfo {
    pub fn new(
        uri: String,
        _: VideoDimensions, // Ignore dimensions parameter for backward compatibility
        framerate: Option<i32>,
        mute: Option<bool>,
        auto_restart: Option<bool>,
    ) -> Self {
        Self {
            uri,
            framerate,
            mute: mute.unwrap_or(false),
            auto_restart: auto_restart.unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[flutter_rust_bridge::frb(sync)]
pub struct WscRtpSessionConfig {
    pub base_url: String,
    pub source_id: String,
    pub client_port: Option<u16>,
    /// Skip UDP negotiation and use WebSocket for RTP delivery from the start.
    pub force_websocket_transport: bool,
    pub auto_restart: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[flutter_rust_bridge::frb(sync)]
pub struct PlaybinConfig {
    pub uri: String,
    pub mute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[flutter_rust_bridge::frb(sync)]
pub enum VideoConfig {
    WscRtp(WscRtpSessionConfig),
    Playbin(PlaybinConfig),
}

pub type DartStateStream = StreamSink<StreamState>;
pub type DartEventsStream = StreamSink<StreamEvent>;
