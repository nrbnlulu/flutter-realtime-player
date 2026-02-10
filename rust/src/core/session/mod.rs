pub mod registry;

use std::{
    sync::{Arc, Mutex},
    time::SystemTime,
};

use log::{debug, warn};

use crate::{
    core::{
        input::{InputEvent, wsc_rtp::{WscRtpSession, WscRtpShutdownHandle}},
        output::flutter_pixelbuffer::{FlutterPixelBufferHandle, OutputCommand},
        types::{DartEventsStream, DartStateStream},
    },
    dart_types::{StreamEvent, StreamState},
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
    /// only valid for WSC-RTP sessions
    fn go_to_live_stream(&self) -> anyhow::Result<()>;
    fn set_speed(&self, speed: f64) -> anyhow::Result<()>;
    fn destroy(self: Box<Self>);
}

pub struct VideoSessionCommon {
    pub session_id: i64,
    pub engine_handle: i64,
    pub output: FlutterPixelBufferHandle,
    pub last_alive_mark: SystemTime,
    pub events_sink: DartEventsStream,
    pub state_sink: Mutex<Option<DartStateStream>>,
}

impl VideoSessionCommon {
    fn new(
        session_id: i64,
        engine_handle: i64,
        output: FlutterPixelBufferHandle,
        events_sink: DartEventsStream,
    ) -> Self {
        Self {
            session_id,
            engine_handle,
            output,
            last_alive_mark: SystemTime::now(),
            events_sink,
            state_sink: Mutex::new(None),
        }
    }

    pub fn send_state_msg(&self, msg: StreamState) {
        if let Some(sink) = self.state_sink.lock().unwrap().as_ref() {
            if let Err(e) = sink.add(msg) {
                log::error!("Failed to send state message: {}", e);
            }
        }
    }

    pub fn send_event_msg(&self, msg: StreamEvent) {
        if let Err(e) = self.events_sink.add(msg) {
            log::error!("Failed to send event message: {}", e);
        }
    }
}

pub struct RawVideoSession {
    common: VideoSessionCommon,
}

impl RawVideoSession {
    pub fn new(
        session_id: i64,
        engine_handle: i64,
        output: FlutterPixelBufferHandle,
        events_sink: DartEventsStream,
    ) -> Self {
        Self {
            common: VideoSessionCommon::new(session_id, engine_handle, output, events_sink),
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
        Err(anyhow::anyhow!("WSC-RTP live not supported for session"))
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

pub struct WscRtpVideoSession {
    common: VideoSessionCommon,
    wsc_rtp_session: Arc<WscRtpSession>,
    wsc_rtp_shutdown: WscRtpShutdownHandle,
}

impl VideoSession for WscRtpVideoSession {
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
        debug!("Terminating WSC-RTP session {}", self.common.session_id);
        self.wsc_rtp_shutdown.shutdown();
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
        log::debug!(
            "Seeking WSC-RTP session {} to {}",
            self.common.session_id,
            ts
        );
        let session = Arc::clone(&self.wsc_rtp_session);
        tokio::runtime::Handle::current().block_on(session.seek(ts))
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        debug!("Resizing WSC-RTP session {}", self.common.session_id);
        self.common
            .output
            .send(OutputCommand::Resize { width, height })
    }

    fn go_to_live_stream(&self) -> anyhow::Result<()> {
        let session = Arc::clone(&self.wsc_rtp_session);
        tokio::runtime::Handle::current().block_on(session.go_live())
    }

    fn set_speed(&self, speed: f64) -> anyhow::Result<()> {
        let session = Arc::clone(&self.wsc_rtp_session);
        tokio::runtime::Handle::current().block_on(session.set_speed(speed))
    }

    fn destroy(self: Box<Self>) {
        debug!("Destroying WSC-RTP session {}", self.common.session_id);
        let mut session = *self;
        session.terminate();
    }
}
