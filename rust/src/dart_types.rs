#[derive(Debug, Clone)]
pub enum StreamState {
    Error(String),
    Loading,
    // texture id
    Playing { texture_id: i64, seekable: bool },
    Stopped,
}

#[derive(Debug, Clone)]
pub enum WscRtpMode {
    Live,
    Dvr { current_time_ms: i64, speed: f64 },
}

/// Mirror of `media_server_api_models::PlaybackEvent`, kept separate so FRB can generate it.
#[derive(Debug, Clone)]
pub enum WscRtpPlaybackEvent {
    Playing { timestamp: i64 },
    Waiting { timestamp: i64 },
    GapSkipped { from: i64, to: i64 },
    Ended { timestamp: i64 },
    Error { timestamp: i64, message: String },
    RenegotiationRequired { codec: String, message: String },
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Error(String),
    CurrentTime(i64),
    OriginVideoSize { width: u64, height: u64 },
    WscRtpSessionMode(WscRtpMode),
    WscRtpPlayback(WscRtpPlaybackEvent),
}

#[derive(Debug, Clone)]
pub enum StreamMessage {
    State(StreamState),
    Event(StreamEvent),
}
