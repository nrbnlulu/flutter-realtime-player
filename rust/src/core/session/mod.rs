pub mod registry;

use std::{
    sync::{Arc, Mutex},
    time::SystemTime,
};

use log::{debug, warn};

use crate::core::{
    input::trtp::{TrtpControl, TsdpSessionCleanup},
    output::flutter_pixelbuffer::{FlutterPixelBufferHandle, OutputCommand},
    types::DartEventsStream,
};

pub trait VideoSession: Send {
    fn session_id(&self) -> i64;
    fn engine_handle(&self) -> i64;
    fn last_alive_mark(&self) -> SystemTime;
    fn make_alive(&mut self);
    fn terminate(&mut self);
    fn set_events_sink(&mut self, sink: DartEventsStream);
    fn seek(&self, ts: i64) -> anyhow::Result<()>;
    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()>;
    /// only valid for TRTP sessions
    fn go_to_live_stream(&self) -> anyhow::Result<()>;
    fn set_speed(&self, speed: f64) -> anyhow::Result<()>;
    fn destroy(self: Box<Self>);
}

struct CommonSession {
    session_id: i64,
    engine_handle: i64,
    output: FlutterPixelBufferHandle,
    last_alive_mark: SystemTime,
    events_sink: Arc<Mutex<Option<DartEventsStream>>>,
}

impl CommonSession {
    fn new(
        session_id: i64,
        engine_handle: i64,
        output: FlutterPixelBufferHandle,
        events_sink: Arc<Mutex<Option<DartEventsStream>>>,
    ) -> Self {
        Self {
            session_id,
            engine_handle,
            output,
            last_alive_mark: SystemTime::now(),
            events_sink,
        }
    }
}

pub struct RawVideoSession {
    common: CommonSession,
}

impl RawVideoSession {
    pub fn new(
        session_id: i64,
        engine_handle: i64,
        output: FlutterPixelBufferHandle,
        events_sink: Arc<Mutex<Option<DartEventsStream>>>,
    ) -> Self {
        Self {
            common: CommonSession::new(session_id, engine_handle, output, events_sink),
        }
    }
}

impl VideoSession for RawVideoSession {
    fn session_id(&self) -> i64 {
        self.common.session_id
    }

    fn engine_handle(&self) -> i64 {
        self.common.engine_handle
    }

    fn last_alive_mark(&self) -> SystemTime {
        self.common.last_alive_mark
    }

    fn make_alive(&mut self) {
        self.common.last_alive_mark = SystemTime::now();
    }

    fn terminate(&mut self) {
        debug!("Terminating raw session {}", self.common.session_id);
        if let Err(err) = self.common.output.send(OutputCommand::Terminate) {
            warn!(
                "Failed to terminate output for session {}: {}",
                self.common.session_id, err
            );
        }
    }

    fn set_events_sink(&mut self, sink: DartEventsStream) {
        if let Ok(mut guard) = self.common.events_sink.lock() {
            *guard = Some(sink);
        }
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        log::debug!("Seeking raw session {} to {}", self.common.session_id, ts);
        self.common.output.send(OutputCommand::Seek { ts })
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        debug!("Resizing raw session {}", self.common.session_id);
        self.common
            .output
            .send(OutputCommand::Resize { width, height })
    }

    fn go_to_live_stream(&self) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("TRTP live not supported for session"))
    }

    fn set_speed(&self, speed: f64) -> anyhow::Result<()> {
        let ts = (speed * 1000.0).round() as i64;
        log::debug!(
            "Mapping speed {} to seek {} on raw session {}",
            speed,
            ts,
            self.common.session_id
        );
        self.common.output.send(OutputCommand::Seek { ts })
    }

    fn destroy(self: Box<Self>) {
        debug!("Destroying raw session {}", self.common.session_id);
        let mut session = *self;
        session.terminate();
    }
}

pub struct TrtpVideoSession {
    common: CommonSession,
    trtp_cleanup: Option<TsdpSessionCleanup>,
    trtp_control: TrtpControl,
}

impl TrtpVideoSession {
    pub fn new(
        session_id: i64,
        engine_handle: i64,
        output: FlutterPixelBufferHandle,
        events_sink: Arc<Mutex<Option<DartEventsStream>>>,
        trtp_cleanup: TsdpSessionCleanup,
        trtp_control: TrtpControl,
    ) -> Self {
        Self {
            common: CommonSession::new(session_id, engine_handle, output, events_sink),
            trtp_cleanup: Some(trtp_cleanup),
            trtp_control,
        }
    }
}

impl VideoSession for TrtpVideoSession {
    fn session_id(&self) -> i64 {
        self.common.session_id
    }

    fn engine_handle(&self) -> i64 {
        self.common.engine_handle
    }

    fn last_alive_mark(&self) -> SystemTime {
        self.common.last_alive_mark
    }

    fn make_alive(&mut self) {
        self.common.last_alive_mark = SystemTime::now();
    }

    fn terminate(&mut self) {
        debug!("Terminating TRTP session {}", self.common.session_id);
        if let Some(cleanup) = self.trtp_cleanup.take() {
            cleanup.cleanup();
        }
        if let Err(err) = self.common.output.send(OutputCommand::Terminate) {
            warn!(
                "Failed to terminate output for session {}: {}",
                self.common.session_id, err
            );
        }
    }

    fn set_events_sink(&mut self, sink: DartEventsStream) {
        if let Ok(mut guard) = self.common.events_sink.lock() {
            *guard = Some(sink);
        }
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        log::debug!("Seeking TRTP session {} to {}", self.common.session_id, ts);
        self.trtp_control.seek(ts)
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        debug!("Resizing TRTP session {}", self.common.session_id);
        self.common
            .output
            .send(OutputCommand::Resize { width, height })
    }

    fn go_to_live_stream(&self) -> anyhow::Result<()> {
        self.trtp_control.live()
    }

    fn set_speed(&self, speed: f64) -> anyhow::Result<()> {
        self.trtp_control.set_speed(speed)
    }

    fn destroy(self: Box<Self>) {
        debug!("Destroying TRTP session {}", self.common.session_id);
        let mut session = *self;
        session.terminate();
    }
}
