pub mod registry;

use std::time::SystemTime;

use async_trait::async_trait;
use parking_lot::Mutex;

use crate::{
    core::types::{DartEventsStream, DartStateStream},
    dart_types::{StreamEvent, StreamState},
};

#[async_trait]
pub trait VideoSession: Send + Sync {
    fn session_id(&self) -> i64;
    fn engine_handle(&self) -> i64;
    fn last_alive_mark(&self) -> SystemTime;
    fn make_alive(&self);
    /// this should not block at all.
    /// either set a flag or abort a task.
    /// it is expected that a few moments later (or immediately) any actual flutter textures will be destroyed.
    fn terminate(&self);
    fn set_events_sink(&self, sink: DartEventsStream);
    async fn seek(&self, ts: u64) -> anyhow::Result<()>;
    async fn go_to_live_stream(&self) -> anyhow::Result<()>;
    async fn set_speed(&self, speed: f64) -> anyhow::Result<()>;
}

pub struct VideoSessionCommon {
    pub session_id: i64,
    pub engine_handle: i64,
    pub last_alive_mark: Mutex<SystemTime>,
    pub events_sink: Mutex<Option<DartEventsStream>>,
    pub state_sink: DartStateStream,
}

impl VideoSessionCommon {
    pub fn new(session_id: i64, engine_handle: i64, state_sink: DartStateStream) -> Self {
        Self {
            session_id,
            engine_handle,
            last_alive_mark: Mutex::new(SystemTime::now()),
            state_sink,
            events_sink: Mutex::new(None),
        }
    }

    pub fn get_last_alive_mark(&self) -> SystemTime {
        *self.last_alive_mark.lock()
    }

    pub fn mark_alive(&self) {
        *self.last_alive_mark.lock() = SystemTime::now();
    }

    pub fn set_events_sink(&self, sink: DartEventsStream) {
        *self.events_sink.lock() = Some(sink);
    }

    pub fn send_event_msg(&self, msg: StreamEvent) {
        if let Some(sink) = self.events_sink.lock().as_ref() {
            if let Err(e) = sink.add(msg) {
                log::error!("Failed to send event message: {}", e);
            }
        }
    }

    pub fn send_state_msg(&self, msg: StreamState) {
        if let Err(e) = self.state_sink.add(msg) {
            log::error!("Failed to send state message: {}", e);
        }
    }
}
