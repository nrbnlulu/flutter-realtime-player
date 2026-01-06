pub mod registry;

use std::{sync::mpsc, time::SystemTime};

use log::{debug, warn};

use crate::core::{
    output::flutter_pixelbuffer::{FlutterPixelBufferHandle, OutputCommand},
    types::DartEventsStream,
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
    output: FlutterPixelBufferHandle,
    last_alive_mark: SystemTime,
    events_sink: Option<DartEventsStream>,
    refresh_tx: Option<mpsc::Sender<()>>,
}

impl FlutterVideoSession {
    pub fn new(
        session_id: i64,
        engine_handle: i64,
        output: FlutterPixelBufferHandle,
        refresh_tx: Option<mpsc::Sender<()>>,
    ) -> Self {
        Self {
            session_id,
            engine_handle,
            output,
            last_alive_mark: SystemTime::now(),
            events_sink: None,
            refresh_tx,
        }
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
        if let Err(err) = self.output.send(OutputCommand::Terminate) {
            warn!(
                "Failed to terminate output for session {}: {}",
                self.session_id, err
            );
        }
    }

    fn set_events_sink(&mut self, sink: DartEventsStream) {
        self.events_sink = Some(sink);
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        self.output.send(OutputCommand::Seek { ts })
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        debug!("resizing session {}", self.session_id);
        self.output.send(OutputCommand::Resize { width, height })
    }

    fn destroy(self: Box<Self>) {
        debug!("Destroying session {}", self.session_id);
        let mut session = *self;
        session.terminate();
    }
}
