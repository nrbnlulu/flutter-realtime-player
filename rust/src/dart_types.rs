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

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Error(String),
    CurrentTime(i64),
    OriginVideoSize { width: u64, height: u64 },
    WscRtpSessionMode(WscRtpMode),
    WscRtpStreamState(String),
}

#[derive(Debug, Clone)]
pub enum StreamMessage {
    State(StreamState),
    Event(StreamEvent),
}
