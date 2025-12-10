#[derive(Debug, Clone)]
pub enum StreamState {
    Error(String),
    Loading,
    // texture id
    Playing { texture_id: i64, seekable: bool },
    Stopped,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Error(String),
    CurrentTime(i64),
    OriginVideoSize { width: u64, height: u64 },
}
