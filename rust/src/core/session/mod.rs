pub mod registry;

use std::{
    sync::{mpsc, Arc},
    thread,
    time::SystemTime,
};

use log::{debug, info, warn};

use crate::{
    core::{
        input::VideoInput,
        texture::{flutter::SharedSendableTexture, FlutterTextureSession},
        types::DartEventsStream,
    },
    utils::invoke_on_platform_main_thread,
};

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

pub struct FlutterVideoSession {
    session_id: i64,
    engine_handle: i64,
    input: Arc<dyn VideoInput>,
    texture_session: Arc<dyn FlutterTextureSession>,
    sendable_texture: SharedSendableTexture,
    last_alive_mark: SystemTime,
    events_sink: Option<DartEventsStream>,
    refresh_tx: Option<mpsc::Sender<()>>,
}

impl FlutterVideoSession {
    pub fn new(
        session_id: i64,
        engine_handle: i64,
        input: Arc<dyn VideoInput>,
        texture_session: Arc<dyn FlutterTextureSession>,
        sendable_texture: SharedSendableTexture,
        refresh_tx: Option<mpsc::Sender<()>>,
    ) -> Self {
        Self {
            session_id,
            engine_handle,
            input,
            texture_session,
            sendable_texture,
            last_alive_mark: SystemTime::now(),
            events_sink: None,
            refresh_tx,
        }
    }

    fn finalize(mut self) {
        debug!(
            "Finalizing session {} (engine_handle={})",
            self.session_id, self.engine_handle
        );
        self.terminate();
        debug!("session {} terminated", self.session_id);
        let mut retry_count = 0;
        const MAX_RETRIES: usize = 50;
        while retry_count < MAX_RETRIES {
            let strong_count = Arc::strong_count(&self.sendable_texture);
            debug!(
                "Session {} texture strong_count={} attempt={}",
                self.session_id, strong_count, retry_count
            );
            if strong_count == 1 {
                break;
            }
            debug!(
                "Waiting for texture references to be dropped for session id: {}. attempt({})",
                self.session_id, retry_count
            );
            thread::sleep(std::time::Duration::from_millis(500));
            retry_count += 1;
        }
        if retry_count == MAX_RETRIES {
            warn!(
                "Forcefully dropping texture for session id: {}, texture still held elsewhere.",
                self.session_id
            );
        }
        let sendable_texture = self.sendable_texture;
        let session_id = self.session_id;
        invoke_on_platform_main_thread(move || {
            drop(sendable_texture);
            info!("Destroyed stream session for session id: {}", session_id);
        });
    }
}

impl SessionLifecycle for FlutterVideoSession {
    fn session_id(&self) -> i64 {
        self.session_id
    }

    fn engine_handle(&self) -> i64 {
        self.engine_handle
    }

    fn last_alive_mark(&self) -> SystemTime {
        self.last_alive_mark
    }

    fn make_alive(&mut self) {
        self.last_alive_mark = SystemTime::now();
    }

    fn terminate(&mut self) {
        debug!("Terminating session {}", self.session_id);
        self.refresh_tx.take();
        self.input.terminate();
        self.texture_session.terminate();
    }

    fn set_events_sink(&mut self, sink: DartEventsStream) {
        self.events_sink = Some(sink);
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        self.input.seek(ts)
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        debug!("resizing input for session {}", self.session_id);
        self.input.resize(width, height)?;
        debug!("resizing texture session for session {}", self.session_id);
        self.texture_session.resize(width, height)?;
        Ok(())
    }

    fn destroy(self: Box<Self>) {
        debug!("Destroying session {}", self.session_id);
        self.finalize();
    }
}
