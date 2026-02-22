use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, Weak},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use gst::prelude::*;
use gst_app::AppSrc;
use irondash_texture::Texture;
use log::{error, info, warn};
use parking_lot::{Mutex, RwLock};
use tokio::net::{TcpStream, UdpSocket};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::{
    core::{
        session::VideoSessionCommon,
        texture::{
            payload::{self, RawRgbaFrame, SharedPixelData},
            FlutterTextureSession,
        },
        types::WscRtpSessionConfig,
    },
    dart_types::{StreamEvent, StreamState},
    utils::invoke_on_platform_main_thread,
};

use media_server_api_models::{
    SeekRequest, SessionMode, SessionModeResponse, SpeedRequest, WscRtpClientMessage,
    WscRtpServerMessage,
};

const UDP_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const PING_INTERVAL: Duration = Duration::from_secs(2);
const SDP_TIMEOUT: Duration = Duration::from_secs(15);
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

// ─── Session ─────────────────────────────────────────────────────────────────

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Resources for a single connection attempt.
/// Returned by `connect_and_setup_pipeline()` and used inside the retry loop.
struct ConnectionResources {
    ws_sink: WsSink,
    ws_stream: WsStream,
    udp_sock: Option<UdpSocket>,
    pipeline: gst::Pipeline,
    wsc_session_id: String,
}

pub struct WscRtpSession {
    session_common: VideoSessionCommon,
    source_id: String,
    media_server_http_url: Url,
    http_client: Arc<reqwest::Client>,
    config: WscRtpSessionConfig,
    shutdown_sender: tokio::sync::mpsc::Sender<()>,
    // Per-connection state (None during reconnect):
    active_session_id: RwLock<Option<String>>,
    active_pipeline: Mutex<Option<Arc<gst::Pipeline>>>,
}

impl WscRtpSession {
    /// Create a new WscRtpSession with config only (no I/O).
    /// Returns the session Arc and a shutdown receiver.
    pub fn new(
        config: WscRtpSessionConfig,
        session_common: VideoSessionCommon,
        http_client: Arc<reqwest::Client>,
    ) -> (Arc<Self>, tokio::sync::mpsc::Receiver<()>) {
        let (shutdown_sender, shutdown_receiver) = tokio::sync::mpsc::channel(1);

        let server_url = Url::parse(&config.base_url)
            .expect("parsing media server HTTP URL should succeed in new()");

        let session = Arc::new(Self {
            session_common,
            source_id: config.source_id.clone(),
            media_server_http_url: server_url,
            http_client,
            config,
            shutdown_sender,
            active_session_id: RwLock::new(None),
            active_pipeline: Mutex::new(None),
        });

        (session, shutdown_receiver)
    }

    /// Connect to the WSC-RTP websocket, wait for Init + SDP messages,
    /// build the GStreamer pipeline, and return connection resources.
    async fn connect_and_setup_pipeline(&self) -> Result<ConnectionResources> {
        let wsc_rtp_url = build_wsc_rtp_handshake_request(
            &self.media_server_http_url,
            &self.source_id,
            self.config.force_websocket_transport,
        )?;
        info!("WSC-RTP connecting to {}", wsc_rtp_url);

        let (ws, _) = connect_async(wsc_rtp_url.to_string())
            .await
            .context(format!("connecting to WSC-RTP ws at {}", wsc_rtp_url))?;
        let (mut ws_sink, mut ws_stream) = ws.split();

        let deadline = tokio::time::Instant::now() + SDP_TIMEOUT;
        let mut init_message = None;
        let mut initial_sdp = None;

        while init_message.is_none() || initial_sdp.is_none() {
            let msg = tokio::time::timeout_at(deadline, ws_stream.next())
                .await
                .context("timeout waiting for WSC-RTP SDP")?
                .ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before SDP"))?
                .context("WSC-RTP websocket error during handshake")?;
            if let Message::Text(text) = msg {
                if let Ok(parsed) = serde_json::from_str::<WscRtpServerMessage>(&text) {
                    match parsed {
                        WscRtpServerMessage::Init {
                            session_id: token,
                            holepunch_port,
                            ..
                        } => {
                            info!(
                                "WSC-RTP Init: session={}, holepunch_port={}",
                                token, holepunch_port
                            );
                            init_message = Some((token, holepunch_port));
                        }
                        WscRtpServerMessage::Sdp { sdp } => initial_sdp = Some(sdp),
                        _ => {}
                    }
                }
            }
        }

        let initial_sdp =
            initial_sdp.ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before SDP"))?;
        let (wsc_session_id, holepunch_port) = init_message
            .ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before initialization"))?;

        // UDP holepunch if not forcing WebSocket transport
        let mut udp_sock_maybe = None;
        if !self.config.force_websocket_transport {
            let bind_addr = resolve_server_udp_addr(&self.media_server_http_url, holepunch_port)?;

            let mut udp_sock = UdpSocket::bind(bind_addr).await?;
            if let Err(e) = validate_udp_handshare(&wsc_session_id, &mut udp_sock).await {
                warn!("failed to handshake for udp transport in session {} due to {} falling back to websockets", wsc_session_id, e);
            }
            udp_sock_maybe = Some(udp_sock);
        }

        let (encoding, pt, clock_rate, sprop) = parse_rtp_caps_from_sdp(&initial_sdp)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse RTP caps from SDP"))?;
        let pipeline_str = build_pipeline_str(&encoding, pt, clock_rate, &sprop);
        info!("WSC-RTP GStreamer pipeline: {}", pipeline_str);

        let pipeline = gst::parse::launch(&pipeline_str)
            .context("GStreamer pipeline launch")?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Not a pipeline"))?;

        Ok(ConnectionResources {
            ws_sink,
            ws_stream,
            udp_sock: udp_sock_maybe,
            pipeline,
            wsc_session_id,
        })
    }

    // ─── HTTP control methods (callable from any thread) ─────────────

    pub async fn seek(&self, timestamp_ms: u64) -> Result<()> {
        self.send_control_request(
            "seek",
            SeekRequest {
                timestamp: timestamp_ms,
            },
        )
        .await
    }

    pub async fn go_live(&self) -> Result<()> {
        self.send_control_request("live", ()).await
    }

    pub async fn set_speed(&self, speed: f64) -> Result<()> {
        self.send_control_request("speed", SpeedRequest { speed })
            .await
    }

    // ─── Execute loop ────────────────────────────────────────────────

    /// Main task: retry loop for connections, receives RTP packets, feeds GStreamer,
    /// sends pings, handles commands.
    ///
    /// Texture creation and `mark_frame_available` are handled externally on the
    /// platform thread via the `event_tx` → output loop path.
    pub async fn execute(
        self: &Arc<Self>,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> anyhow::Result<()> {
        let payload_holder = Arc::new(crate::core::texture::payload::PayloadHolder::new());
        let payload_holder_weak = Arc::downgrade(&payload_holder);
        let payload_holder_for_texture = Arc::clone(&payload_holder);
        let engine_handle = self.session_common.engine_handle;
        let (sendable_texture, texture_id) =
            invoke_on_platform_main_thread(move || -> Result<_> {
                let texture =
                    Texture::new_with_provider(engine_handle, payload_holder_for_texture)?;
                let texture_id = texture.id();
                Ok((texture.into_sendable_texture(), texture_id))
            })?;

        let texture_session = Arc::new(crate::core::texture::flutter::TextureSession::new(
            texture_id,
            Arc::downgrade(&sendable_texture),
            payload_holder_weak.clone(),
        ));
        let texture_session: Arc<dyn FlutterTextureSession> = texture_session;

        self.session_common.send_state_msg(StreamState::Loading);

        let mut backoff = INITIAL_BACKOFF;
        let mut output: anyhow::Result<()> = Ok(());

        loop {
            match self.connect_and_setup_pipeline().await {
                Ok(resources) => {
                    // Reset backoff on successful connection
                    backoff = INITIAL_BACKOFF;

                    // Store session_id and pipeline
                    *self.active_session_id.write() = Some(resources.wsc_session_id.clone());
                    let pipeline_arc = Arc::new(resources.pipeline);
                    *self.active_pipeline.lock() = Some(Arc::clone(&pipeline_arc));

                    // Execute the inner session loop
                    let inner_result = WscRtpSession::run_session_loop(
                        self,
                        resources.ws_sink,
                        resources.ws_stream,
                        resources.udp_sock,
                        pipeline_arc.clone(),
                        texture_session.clone(),
                        payload_holder_weak.clone(),
                        &mut shutdown_rx,
                        texture_id,
                    )
                    .await;

                    *self.active_session_id.write() = None;
                    *self.active_pipeline.lock() = None;

                    match inner_result {
                        Ok(ExitReason::Shutdown) => {
                            output = Ok(());
                            break;
                        }
                        Err(e) => {
                            // Connection lost - will retry if auto_restart is enabled
                            warn!("WSC-RTP session disconnected: {}", e);
                            self.session_common
                                .send_event_msg(StreamEvent::Error(format!(
                                    "Connection lost: {}",
                                    e
                                )));

                            if !self.config.auto_restart {
                                info!("WSC-RTP: auto_restart disabled, stopping");
                                output = Err(e);
                                break;
                            }
                            // Continue to retry
                        }
                    }
                }
                Err(e) => {
                    error!("WSC-RTP connection failed: {}", e);
                    self.session_common
                        .send_event_msg(StreamEvent::Error(format!("Connection failed: {}", e)));

                    if shutdown_rx.try_recv().is_ok() {
                        info!("WSC-RTP: shutdown requested during connection");
                        output = Err(e);
                        break;
                    }

                    if !self.config.auto_restart {
                        info!("WSC-RTP: auto_restart disabled, stopping");
                        output = Err(e);
                        break;
                    }

                    // Backoff before retry
                    self.session_common.send_state_msg(StreamState::Loading);
                    self.session_common
                        .send_event_msg(StreamEvent::Error(format!(
                            "WS connection failed retrying in {}s",
                            backoff.as_secs()
                        )));
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => {}
                        cmd = shutdown_rx.recv() => {
                            if cmd.is_some() {
                                info!("WSC-RTP: shutdown requested during backoff");
                                output = Err(e);
                                break;
                            }
                        }
                    }
                    backoff = std::cmp::min(backoff * 2, MAX_BACKOFF);
                }
            }
        }

        // Send Stopped state
        let _ = self
            .session_common
            .send_state_msg(crate::dart_types::StreamState::Stopped);

        // Texture + payload_holder must be dropped on the platform main thread
        invoke_on_platform_main_thread(move || {
            drop(sendable_texture);
            drop(payload_holder);
        });

        output
    }

    /// Inner session loop - runs while connected.
    /// Returns ExitReason to indicate why the loop exited.
    async fn run_session_loop(
        session: &Arc<WscRtpSession>,
        mut ws_sink: WsSink,
        mut ws_stream: WsStream,
        udp_sock: Option<UdpSocket>,
        pipeline: Arc<gst::Pipeline>,
        texture_session: Arc<dyn FlutterTextureSession>,
        payload_holder_weak: Weak<payload::PayloadHolder>,
        shutdown_rx: &mut tokio::sync::mpsc::Receiver<()>,
        texture_id: i64,
    ) -> Result<ExitReason> {
        let appsrc = pipeline
            .by_name("src")
            .ok_or_else(|| anyhow::anyhow!("appsrc not found"))?
            .downcast::<AppSrc>()
            .map_err(|_| anyhow::anyhow!("src is not AppSrc"))?;

        let appsink = pipeline
            .by_name("sink")
            .ok_or_else(|| anyhow::anyhow!("appsink not found"))?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("sink is not AppSink"))?;

        let session_weak = Arc::downgrade(session);
        let session_weak_for_callbacks = session_weak.clone();
        let mut origin_size_sent = false;
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;
                    let video_info =
                        gst_video::VideoInfo::from_caps(caps).map_err(|_| gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;

                    let width = video_info.width();
                    let height = video_info.height();

                    // Emit OriginVideoSize on first decoded frame
                    if !origin_size_sent {
                        origin_size_sent = true;
                        if let Some(session) = session_weak_for_callbacks.upgrade() {
                            session
                                .session_common
                                .send_event_msg(StreamEvent::OriginVideoSize {
                                    width: width as u64,
                                    height: height as u64,
                                });
                        }
                    }

                    let video_frame =
                        gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &video_info)
                            .map_err(|_| gst::FlowError::Error)?;

                    let stride = video_info.stride()[0] as usize;
                    let expected_stride = (width as usize) * 4; // RGBA
                    let plane_data = video_frame
                        .plane_data(0)
                        .map_err(|_| gst::FlowError::Error)?;

                    let data = if stride == expected_stride {
                        plane_data.to_vec()
                    } else {
                        // Stride mismatch — copy row by row to strip padding
                        let mut buf = Vec::with_capacity(expected_stride * height as usize);
                        for y in 0..height as usize {
                            let row_start = y * stride;
                            buf.extend_from_slice(
                                &plane_data[row_start..row_start + expected_stride],
                            );
                        }
                        buf
                    };

                    let frame = RawRgbaFrame {
                        width,
                        height,
                        data,
                    };

                    if let Some(holder) = payload_holder_weak.upgrade() {
                        holder.set_payload(Arc::new(frame) as SharedPixelData);
                        texture_session.mark_frame_available();
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        // UDP packet receiver task
        async fn udp_packet_receiver(appsrc: AppSrc, udp_sock: UdpSocket) {
            // 1500 is standard MTU size for Ethernet frames
            let mut buf = [0u8; 1500];
            while let Ok((len, _)) = udp_sock.recv_from(&mut buf).await {
                let gst_buffer = gst::Buffer::from_slice(buf[..len].to_vec());
                if let Err(err) = appsrc.push_buffer(gst_buffer) {
                    log::warn!("WSC-RTP: appsrc push_buffer failed: {}", err);
                    break;
                }
            }
        }

        let mut udp_packet_rcv_task = None;
        if let Some(udp_sock) = udp_sock {
            udp_packet_rcv_task = Some(tokio::spawn(udp_packet_receiver(appsrc.clone(), udp_sock)));
        }

        // Set up GStreamer bus error monitoring
        let (gst_err_tx, mut gst_err_rx) = tokio::sync::mpsc::channel::<String>(4);
        let bus = pipeline.bus().unwrap();
        let bus_session_id = session.source_id.clone();
        bus.set_sync_handler(move |_bus, msg| {
            match msg.view() {
                gst::MessageView::Error(err) => {
                    let _ = gst_err_tx.try_send(format!(
                        "GStreamer error [{}]: {}",
                        bus_session_id,
                        err.error()
                    ));
                }
                gst::MessageView::Eos(_) => {
                    let _ = gst_err_tx.try_send(format!("GStreamer EOS [{}]", bus_session_id));
                }
                _ => {}
            }
            gst::BusSyncReply::Drop
        });

        pipeline
            .set_state(gst::State::Playing)
            .context("setting GStreamer pipeline to Playing")?;

        // Send Playing state with texture_id
        if let Some(session) = session_weak.upgrade() {
            session.session_common.send_state_msg(StreamState::Playing {
                texture_id,
                seekable: true,
            });
        }

        let mut ping_interval = tokio::time::interval(PING_INTERVAL);
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Inner select loop
        loop {
            tokio::select! {
                // ── Session commands (shutdown only) ──────────────────
                cmd = shutdown_rx.recv() => {
                    if let Some(cmd) = cmd {
                        info!("WSC-RTP: shutdown command received");
                        // Cleanup before exit
                        if let Some(udp_rcv_task) = udp_packet_rcv_task {
                            udp_rcv_task.abort();
                        }
                        pipeline.set_state(gst::State::Null)?;
                        return Ok(ExitReason::Shutdown);
                    }
                }

                _ = ping_interval.tick() => {
                    if let Ok(payload) = serde_json::to_string(&WscRtpClientMessage::Ping) {
                        let _ = ws_sink.send(Message::Text(payload.into())).await;
                    }
                }
                msg = ws_stream.next() => {
                    match msg {
                        None => {
                            // WebSocket stream closed
                            if let Some(udp_rcv_task) = udp_packet_rcv_task {
                                udp_rcv_task.abort();
                            }
                            pipeline.set_state(gst::State::Null)?;
                            return Err(anyhow::anyhow!("WebSocket stream closed"));
                        }
                        Some(Err(e)) => {
                            // WebSocket error
                            if let Some(udp_rcv_task) = udp_packet_rcv_task {
                                udp_rcv_task.abort();
                            }
                            pipeline.set_state(gst::State::Null)?;
                            return Err(anyhow::anyhow!("WebSocket error: {}", e));
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = ws_sink.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            let buffer = gst::Buffer::from_mut_slice(data.to_vec());
                            if let Err(err) = appsrc.push_buffer(buffer) {
                                log::warn!("WSC-RTP: failed to handle binary message: {}", err);
                            }
                        }
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<WscRtpServerMessage>(&text) {
                                Ok(msg) => {
                                    if let Some(session) = session_weak.upgrade() {
                                        session.handle_server_message(msg);
                                    }
                                },
                                Err(err) => {
                                    warn!("WSC-RTP: failed to parse server message: {} — raw: {}", err, text);
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            // WebSocket close frame
                            if let Some(udp_rcv_task) = udp_packet_rcv_task {
                                udp_rcv_task.abort();
                            }
                            pipeline.set_state(gst::State::Null)?;
                            return Err(anyhow::anyhow!("WSC-RTP: received close frame"));
                        }
                        Some(Ok(_)) => {}
                    }
                }
                // GStreamer bus error
                err_msg = gst_err_rx.recv() => {
                    if let Some(err) = err_msg {
                        error!("{}", err);
                        if let Some(udp_rcv_task) = udp_packet_rcv_task {
                            udp_rcv_task.abort();
                        }
                        pipeline.set_state(gst::State::Null)?;
                        return Err(anyhow::anyhow!(err));
                    }
                }
            }
        }
    }

    // ─── Internal helpers ────────────────────────────────────────────

    fn handle_server_message(&self, message: WscRtpServerMessage) {
        match message {
            WscRtpServerMessage::Init {
                session_id,
                holepunch_port,
                ..
            } => {
                info!(
                    "WSC-RTP session {} initialized, holepunch_port={}",
                    session_id, holepunch_port
                );
            }
            WscRtpServerMessage::Sdp { .. } => {}
            WscRtpServerMessage::SessionMode(mode) => {
                let (is_live, current_time_ms) = match mode {
                    SessionMode::Live => (true, 0),
                    SessionMode::Dvr { timestamp } => (false, timestamp as i64),
                };
                self.session_common
                    .send_event_msg(StreamEvent::WscRtpSessionMode {
                        is_live,
                        current_time_ms,
                        speed: 1.0,
                    });
            }
            WscRtpServerMessage::Error { message } => {
                self.session_common
                    .send_event_msg(StreamEvent::Error(message));
            }
            WscRtpServerMessage::FallingBackRtpToWs => {
                info!("WSC-RTP: server falling back to WebSocket for RTP delivery");
            }
            WscRtpServerMessage::Pong => {}
        }
    }

    async fn send_control_request(
        &self,
        endpoint: &str,
        body: impl serde::Serialize,
    ) -> Result<()> {
        let session_id =
            self.active_session_id.read().clone().ok_or_else(|| {
                anyhow::anyhow!("session is reconnecting, no active server session")
            })?;

        let mut url = self.media_server_http_url.clone();
        url.set_path(&format!(
            "/client-session-control/{}/{}",
            session_id, endpoint
        ));

        let response = self
            .http_client
            .post(url.as_str())
            .json(&body)
            .send()
            .await
            .context("WSC-RTP control request failed")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("WSC-RTP control request failed with status: {}", status);
        }

        let mode: SessionModeResponse = response
            .json()
            .await
            .context("parsing WSC-RTP control response")?;
        self.session_common
            .send_event_msg(StreamEvent::WscRtpSessionMode {
                is_live: mode.is_live,
                current_time_ms: mode.current_time_ms.unwrap_or(0) as i64,
                speed: mode.speed,
            });
        Ok(())
    }
}

/// Reason why the session loop exited
enum ExitReason {
    /// Intentional shutdown via terminate()
    Shutdown,
}

#[async_trait]
impl crate::core::session::VideoSession for WscRtpSession {
    async fn seek(&self, ts: u64) -> anyhow::Result<()> {
        Self::seek(&self, ts).await
    }
    async fn go_to_live_stream(&self) -> anyhow::Result<()> {
        Self::go_live(&self).await
    }

    async fn set_speed(&self, speed: f64) -> anyhow::Result<()> {
        Self::set_speed(&self, speed).await
    }

    fn session_id(&self) -> i64 {
        self.session_common.session_id
    }
    fn engine_handle(&self) -> i64 {
        self.session_common.engine_handle
    }

    fn last_alive_mark(&self) -> std::time::SystemTime {
        self.session_common.get_last_alive_mark()
    }

    fn make_alive(&self) {
        self.session_common.mark_alive();
    }

    fn terminate(&self) {
        // Stop current pipeline if any
        if let Some(pipeline) = self.active_pipeline.lock().take() {
            let _ = pipeline.set_state(gst::State::Null);
        }
        if let Err(e) = self.shutdown_sender.blocking_send(()) {
            log::error!("Failed to send shutdown signal: {:?}", e);
        }
    }

    fn set_events_sink(&self, sink: crate::core::types::DartEventsStream) {
        self.session_common.set_events_sink(sink);
    }
}

async fn validate_udp_handshare(session_id: &str, udp_sock: &mut UdpSocket) -> anyhow::Result<()> {
    async fn one_try(
        udp_sock: &mut tokio::net::UdpSocket,
        expected_dummy: &str,
        holepunch_msg: &str,
        session_id: &str,
    ) -> anyhow::Result<()> {
        let mut buf = [0u8; 512];
        udp_sock.send(holepunch_msg.as_bytes()).await?;
        let (n, _src) = udp_sock.recv_from(&mut buf).await?;
        let payload = std::str::from_utf8(&buf[..n])?;
        if payload.trim() == expected_dummy {
            let ack = format!(
                "{} {}",
                media_server_api_models::wsc_rtp::ACK_HEADER,
                session_id
            );
            udp_sock.send(ack.as_bytes()).await?;
            info!("WSC-RTP: UDP confirmed, starting UDP receive loop");
        } else {
            bail!("WSC-RTP: unexpected UDP payload {:?}", payload);
        }
        Ok(())
    }
    let holepunch_msg = format!(
        "{} {}",
        media_server_api_models::wsc_rtp::HOLEPUNCH_HEADER,
        session_id
    );

    let mut max_retries = 5;
    let expected_dummy_msg = format!(
        "{} {}",
        media_server_api_models::wsc_rtp::DUMMY_HEADER,
        session_id
    );
    loop {
        if max_retries == 0 {
            return Err(anyhow::anyhow!(
                "Max handshake retries exceeded for session {}",
                session_id
            ));
        }
        max_retries -= 1;
        if let Err(e) = one_try(udp_sock, &holepunch_msg, &expected_dummy_msg, session_id).await {
            log::warn!(
                "failed to handshake udp transport for session {} due to {:?}",
                session_id,
                e
            );
        } else {
            return Ok(());
        }
    }
}

// ─── GStreamer pipeline helpers ───────────────────────────────────────────────

fn parse_rtp_caps_from_sdp(sdp_text: &str) -> Option<(String, u8, u32, Option<String>)> {
    let mut reader = std::io::Cursor::new(sdp_text);
    let sdp = sdp::description::session::SessionDescription::unmarshal(&mut reader).ok()?;

    let media = sdp.media_descriptions.first()?;
    let pt: u8 = media.media_name.formats.first()?.parse().ok()?;

    let rtpmap_val = media.attribute("rtpmap")?.unwrap_or("");
    let codec_part = rtpmap_val.splitn(2, ' ').nth(1)?;
    let mut codec_iter = codec_part.splitn(2, '/');
    let encoding = codec_iter.next()?.to_uppercase();
    let clock_rate: u32 = codec_iter.next()?.parse().ok()?;

    let sprop = media.attribute("fmtp").and_then(|v| v).and_then(|fmtp| {
        let params = fmtp.splitn(2, ' ').nth(1)?;
        params.split(';').find_map(|param| {
            param
                .trim()
                .strip_prefix("sprop-parameter-sets=")
                .filter(|v| !v.is_empty())
                .map(String::from)
        })
    });

    Some((encoding, pt, clock_rate, sprop))
}

fn build_pipeline_str(encoding: &str, pt: u8, clock_rate: u32, sprop: &Option<String>) -> String {
    let depay_decode = match encoding {
        "H264" => "rtph264depay ! h264parse ! avdec_h264",
        "H265" | "HEVC" => "rtph265depay ! h265parse ! avdec_h265",
        "VP8" => "rtpvp8depay ! vp8dec",
        "VP9" => "rtpvp9depay ! vp9dec",
        _ => "rtpjpegdepay ! jpegdec",
    };

    let sprop_cap = match (encoding, sprop) {
        ("H264", Some(s)) => format!(",sprop-parameter-sets=\\\"{}\\\"", s),
        ("H265" | "HEVC", Some(s)) => format!(",sprop-parameter-sets=\\\"{}\\\"", s),
        _ => String::new(),
    };

    format!(
        "appsrc name=src caps=\"application/x-rtp,media=video,payload={pt},clock-rate={clock_rate},encoding-name={encoding}{sprop_cap}\" format=time is-live=true \
         ! rtpjitterbuffer \
         ! {depay_decode} \
         ! videoconvert \
         ! video/x-raw,format=RGBA \
         ! appsink name=sink sync=false emit-signals=true",
    )
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn build_wsc_rtp_handshake_request(
    url: &Url,
    source_id: &str,
    force_websocket_transport: bool,
) -> Result<Url> {
    let mut url = url.clone();
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => other,
    }
    .to_string();
    url.set_scheme(&scheme)
        .map_err(|_| anyhow::anyhow!("invalid base_url scheme"))?;
    url.set_path(&format!("/streams/{}/wsc-rtp", source_id));
    if force_websocket_transport {
        url.set_query(Some("force_websocket_transport=true"));
    } else {
        url.set_query(None);
    }
    Ok(url)
}

fn resolve_server_udp_addr(url: &Url, port: u16) -> Result<SocketAddr> {
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("base_url missing host"))?;
    let addrs: Vec<_> = (host, port)
        .to_socket_addrs()
        .context("resolve server host")?
        .collect();
    let addr = addrs
        .iter()
        .find(|a| a.is_ipv4())
        .cloned()
        .or_else(|| addrs.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("no addresses resolved for {}", host))?;
    Ok(addr)
}
