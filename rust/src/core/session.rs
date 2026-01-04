use std::{
    sync::Arc,
    thread,
    time::SystemTime,
};

use log::{debug, info};

use crate::{
    core::{software_decoder::SoftwareDecoder, types::DartEventsStream},
    utils::invoke_on_platform_main_thread,
};

use super::software_decoder::SharedSendableTexture;

pub trait SessionLifecycle: Send {
    fn session_id(&self) -> i64;
    fn engine_handle(&self) -> i64;
    fn last_alive_mark(&self) -> SystemTime;
    fn make_alive(&mut self);
    fn terminate(&mut self);
    fn set_events_sink(&mut self, sink: DartEventsStream);
    fn seek(&self, ts: i64) -> anyhow::Result<()>;
    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()>;
    fn destroy(self: Box<Self>);
}

pub struct BaseSession {
    session_id: i64,
    decoder: Arc<SoftwareDecoder>,
    engine_handle: i64,
    sendable_texture: SharedSendableTexture,
    last_alive_mark: SystemTime,
    events_sink: Option<DartEventsStream>,
}

impl BaseSession {
    pub fn new(
        session_id: i64,
        decoder: Arc<SoftwareDecoder>,
        engine_handle: i64,
        sendable_texture: SharedSendableTexture,
    ) -> Self {
        Self {
            session_id,
            decoder,
            engine_handle,
            sendable_texture,
            last_alive_mark: SystemTime::now(),
            events_sink: None,
        }
    }

    pub fn session_id(&self) -> i64 {
        self.session_id
    }

    pub fn engine_handle(&self) -> i64 {
        self.engine_handle
    }

    pub fn last_alive_mark(&self) -> SystemTime {
        self.last_alive_mark
    }

    pub fn make_alive(&mut self) {
        self.last_alive_mark = SystemTime::now();
    }

    pub fn terminate(&mut self) {
        self.decoder.destroy_stream();
    }

    pub fn set_events_sink(&mut self, sink: DartEventsStream) {
        self.events_sink = Some(sink);
    }

    pub fn seek(&self, ts: i64) -> anyhow::Result<()> {
        self.decoder.seek(ts)
    }

    pub fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        self.decoder.resize_stream(width, height)
    }

    pub fn finalize(self) {
        self.decoder.destroy_stream();
        let mut retry_count = 0;
        const MAX_RETRIES: usize = 50;
        while retry_count < MAX_RETRIES {
            if Arc::strong_count(&self.decoder) == 1 {
                break;
            }
            debug!(
                "Waiting for all references to be dropped for session id: {}. attempt({})",
                self.session_id, retry_count
            );
            thread::sleep(std::time::Duration::from_millis(500));
            retry_count += 1;
        }
        if retry_count == MAX_RETRIES {
            log::error!("Forcefully dropped decoder for session id: {}, the texture is held somewhere else and may panic when unregistered if held on the wrong thread.", self.session_id);
        }
        let sendable_texture = self.sendable_texture;
        let session_id = self.session_id;
        invoke_on_platform_main_thread(move || {
            drop(sendable_texture);
            info!("Destroyed stream session for session id: {}", session_id);
        });
    }
}

impl SessionLifecycle for BaseSession {
    fn session_id(&self) -> i64 {
        self.session_id()
    }

    fn engine_handle(&self) -> i64 {
        self.engine_handle()
    }

    fn last_alive_mark(&self) -> SystemTime {
        self.last_alive_mark()
    }

    fn make_alive(&mut self) {
        BaseSession::make_alive(self);
    }

    fn terminate(&mut self) {
        BaseSession::terminate(self);
    }

    fn set_events_sink(&mut self, sink: DartEventsStream) {
        BaseSession::set_events_sink(self, sink);
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        BaseSession::seek(self, ts)
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        BaseSession::resize(self, width, height)
    }

    fn destroy(self: Box<Self>) {
        self.finalize();
    }
}
