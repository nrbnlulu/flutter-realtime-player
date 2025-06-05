#[derive(Debug, Clone)]
pub enum StreamState {
    Error(String),
    Loading,
    // texture id
    Playing { texture_id: i64 },
    Stopped,
}
