#[derive(Debug, Clone)]
pub enum StreamState {
    Init { session_id: i64 },
    Error(String),
    Loading,
    // texture id
    Playing { texture_id: i64 },
    Stopped,
}
