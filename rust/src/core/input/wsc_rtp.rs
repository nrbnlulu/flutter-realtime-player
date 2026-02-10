use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use log::{info, warn};

use tokio::net::{TcpStream, UdpSocket};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::{
    core::{
        input::{InputEvent, InputEventSender},
        session::VideoSessionCommon,
        texture::payload::{RawRgbaFrame, SharedPixelData},
        types::VideoDimensions,
    },
    dart_types::StreamEvent,
    utils::LogErr,
};

use media_server_api_models::{WscRtpClientMessage, WscRtpServerMessage};

const UDP_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const PING_INTERVAL: Duration = Duration::from_secs(2);
const SDP_TIMEOUT: Duration = Duration::from_secs(15);

// ─── Config ──────────────────────────────────────────────────────────────────

pub struct WscRtpSessionConfig {
    pub base_url: String,
    pub source_id: String,
    pub force_websocket_transport: bool,
}

// ─── Session command enum (WS-level commands only) ───────────────────────────

pub enum WscRtpSessionCommand {
    Shutdown,
}

// ─── Session ─────────────────────────────────────────────────────────────────

type WsSink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type WsStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub struct WscRtpSession {
    session_common: VideoSessionCommon,
    initial_sdp: String,
    session_id: String,
    holepunch_port: u16,
    media_server_http_url: Url,
    source_id: String,
    use_udp_transport: bool,
    http_client: reqwest::Client,
}

impl WscRtpSession {
    /// Connect to the WSC-RTP websocket, wait for Init + SDP messages,
    /// and return the session together with the split websocket streams.
    pub async fn new(
        config: &WscRtpSessionConfig,
        session_common: VideoSessionCommon,
        server_url: &str,
        command_rx: tokio::sync::mpsc::Receiver<WscRtpSessionCommand>,
        http_client: Arc<reqwest::Client>,
    ) -> Result<(Arc<Self>, WsSink, WsStream)> {
        let server_url = Url::parse(server_url).context("parsing media server HTTP URL")?;
        let wsc_rtp_url = build_wsc_rtp_handshake_request(
            &server_url,
            &config.source_id,
            config.force_websocket_transport,
        )?;
        info!("WSC-RTP connecting to {}", wsc_rtp_url);

        let (ws, _) = connect_async(wsc_rtp_url.into())
            .await
            .context("connecting to WSC-RTP ws")?;
        let (ws_sink, mut ws_stream) = ws.split();

        let deadline = tokio::time::Instant::now() + SDP_TIMEOUT;
        let mut udp_success: Option<bool> = None;
        let mut init_message = None;
        let mut initial_sdp = None;
        
        // TODO: add timeout
        while init_message.is_none() && initial_sdp.is_none() {
            let msg = tokio::time::timeout_at(deadline, ws_stream.next())
                .await
                .context("timeout waiting for WSC-RTP SDP")?
                .ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before SDP"))?
                .context("WSC-RTP websocket error during handshake")?;
            if let Message::Text(text) = msg {
                if let Ok(parsed) =
                    serde_json::from_str::<media_server_api_models::WscRtpServerMessage>(&text)
                {
                    match parsed {
                        WscRtpServerMessage::Init {
                            token: token,
                            holepunch_port: holepunch_port,
                        } => init_message = Some((token, holepunch_port)),
                        WscRtpServerMessage::Sdp { sdp } => initial_sdp = Some(sdp),
                        _ => {}
                    }
                }
            }
        }
        
        let initial_sdp = initial_sdp.ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before SDP"))?;
        let (session_id, holepunch_port) = init_message
            .ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before initialization"))?;
        
        // the ws wasn't initialized with force_websocket
        // do holepunch and check if we can receive udp packets
        let mut udp_sock_maybe = None;
        if !config.force_websocket_transport {
            let bind_addr = resolve_server_udp_addr(&server_url, holepunch_port)?;

            let mut udp_sock = UdpSocket::bind(bind_addr).await?;
            if let Err(e) = validate_udp_handshare(&session_id, &mut udp_sock).await {
                log::warn!("failed to handshake for udp transport in session {} falling back to websockets", session_id);
            }
            udp_sock_maybe = Some(udp_sock);
        }

        let session = Arc::new(Self {
            session_common,
            initial_sdp,
            session_id: session_id,
            holepunch_port,
            media_server_http_url: server_url,
            source_id: config.source_id.clone(),
            use_udp_transport: udp_sock_maybe.is_some(),
            http_client: reqwest::Client::new(),
        });

        Ok((session, ws_sink, ws_stream))
    }

    pub fn sdp_text(&self) -> &str {
        &self.initial_sdp
    }

    // ─── HTTP control methods (callable from any thread) ─────────────

    pub async fn seek(&self, timestamp_ms: i64) -> Result<()> {
        self.send_control_request(
            "seek",
            Some(serde_json::json!({ "timestamp": timestamp_ms })),
        )
        .await
    }

    pub async fn go_live(&self) -> Result<()> {
        self.send_control_request("live", None).await
    }

    pub async fn set_speed(&self, speed: f64) -> Result<()> {
        self.send_control_request("speed", Some(serde_json::json!({ "speed": speed })))
            .await
    }

    // ─── Execute loop ────────────────────────────────────────────────

    /// Main task: receives RTP packets, feeds GStreamer, sends pings, handles commands.
    ///
    /// Texture creation and `mark_frame_available` are handled externally on the
    /// platform thread via the `event_tx` → output loop path.
    pub async fn execute(
        self: &Arc<Self>,
        mut ws_sink: WsSink,
        mut ws_stream: WsStream,
        mut command_rx: tokio::sync::mpsc::Receiver<WscRtpSessionCommand>,
        event_tx: InputEventSender,
        output_dims: Arc<Mutex<VideoDimensions>>,
        payload_holder: Weak<crate::core::texture::payload::PayloadHolder>,
        texture_id: i64,
    ) {
        let result = self
            .execute_inner(
                &mut ws_sink,
                &mut ws_stream,
                &event_tx,
                output_dims,
                payload_holder,
                texture_id,
            )
            .await;

        let _ = ws_sink.close().await;

        match result {
            Ok(()) => info!("WSC-RTP session finished cleanly"),
            Err(e) => {
                warn!("WSC-RTP session error: {}", e);
                push_event(&self.events_sink, StreamEvent::Error(e.to_string()));
                let _ = event_tx.send(InputEvent::State(crate::dart_types::StreamState::Error(
                    e.to_string(),
                )));
            }
        }

        let _ = event_tx.send(InputEvent::State(crate::dart_types::StreamState::Stopped));
    }

    async fn execute_inner(
        &self,
        ws_sink: &mut WsSink,
        ws_stream: &mut WsStream,
        event_tx: &InputEventSender,
        output_dims: Arc<Mutex<VideoDimensions>>,
        payload_holder: Weak<crate::core::texture::payload::PayloadHolder>,
        texture_id: i64,
    ) -> Result<()> {
        // ── Parse SDP and build GStreamer pipeline ────────────────────────
        let (encoding, pt, clock_rate, sprop) = parse_rtp_caps_from_sdp(&self.initial_sdp)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse RTP caps from SDP"))?;

        let dims = output_dims.lock().unwrap().clone();
        let pipeline_str =
            build_pipeline_str(&encoding, pt, clock_rate, &sprop, dims.width, dims.height);
        info!("WSC-RTP GStreamer pipeline: {}", pipeline_str);

        let pipeline = gstreamer::parse::launch(&pipeline_str)
            .context("GStreamer pipeline launch")?
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Not a pipeline"))?;

        let appsrc = pipeline
            .by_name("src")
            .ok_or_else(|| anyhow::anyhow!("appsrc not found"))?
            .downcast::<AppSrc>()
            .map_err(|_| anyhow::anyhow!("src is not AppSrc"))?;

        let appsink = pipeline
            .by_name("sink")
            .ok_or_else(|| anyhow::anyhow!("appsink not found"))?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("sink is not AppSink"))?;

        let event_tx_clone = event_tx.clone();
        let out_dims = Arc::clone(&output_dims);
        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                    let map = buffer
                        .map_readable()
                        .map_err(|_| gstreamer::FlowError::Error)?;

                    let dims = out_dims.lock().unwrap();
                    let frame = RawRgbaFrame {
                        width: dims.width,
                        height: dims.height,
                        data: map.as_slice().to_vec(),
                    };
                    drop(dims);

                    if let Some(holder) = payload_holder.upgrade() {
                        holder.set_payload(Arc::new(frame) as SharedPixelData);
                        let _ = event_tx_clone.send(InputEvent::FrameAvailable);
                    }
                    Ok(gstreamer::FlowSuccess::Ok)
                })
                .build(),
        );

        pipeline
            .set_state(gstreamer::State::Playing)
            .context("set pipeline Playing")?;

        event_tx
            .send(InputEvent::State(crate::dart_types::StreamState::Playing {
                texture_id,
                seekable: false,
            }))
            .ok();

        // ── Optional UDP transport ───────────────────────────────────────
        let (rtp_tx, rtp_rx) = flume::unbounded::<Vec<u8>>();

        if self.use_udp_transport {
            let base_url = self.base_url.clone();
            let token = self.session_id.clone();
            let holepunch_port = self.holepunch_port;
            let tx = rtp_tx.clone();
            tokio::task::spawn_blocking(move || {
                try_udp_holepunch(&base_url, holepunch_port, &token, &tx);
            });
        } else {
            info!("WSC-RTP: skipping UDP holepunch (force_websocket_transport=true)");
        }

        // ── Main select loop ─────────────────────────────────────────────
        // Take the receiver out of the Mutex so we don't hold a MutexGuard across awaits.
        let mut command_rx = self
            .command_rx
            .lock()
            .unwrap()
            .take()
            .ok_or_else(|| anyhow::anyhow!("WscRtpSession::execute called more than once"))?;
        let mut ping_interval = tokio::time::interval(PING_INTERVAL);
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                // ── Session commands (shutdown only) ──────────────────
                cmd = command_rx.recv() => {
                    match cmd {
                        Some(WscRtpSessionCommand::Shutdown) | None => {
                            info!("WSC-RTP: shutdown command received");
                            pipeline.set_state(gstreamer::State::Null).ok();
                            return Ok(());
                        }
                    }
                }

                // ── Periodic ping ────────────────────────────────────
                _ = ping_interval.tick() => {
                    if let Ok(payload) = serde_json::to_string(&WscRtpClientMessage::Ping) {
                        let _ = ws_sink.send(Message::Text(payload.into())).await;
                    }
                }

                // ── Incoming WebSocket messages ──────────────────────
                msg = ws_stream.next() => {
                    match msg {
                        None => {
                            info!("WSC-RTP: websocket stream closed");
                            pipeline.set_state(gstreamer::State::Null).ok();
                            return Ok(());
                        }
                        Some(Err(e)) => {
                            pipeline.set_state(gstreamer::State::Null).ok();
                            return Err(e.into());
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = ws_sink.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            let _ = rtp_tx.send(data.to_vec());
                        }
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<WscRtpServerMessage>(&text) {
                                Ok(msg) => self.handle_server_message(msg),
                                Err(err) => {
                                    warn!("WSC-RTP: failed to parse server message: {} — raw: {}", err, text);
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WSC-RTP: received close frame");
                            pipeline.set_state(gstreamer::State::Null).ok();
                            return Ok(());
                        }
                        Some(Ok(_)) => {}
                    }
                }

                // ── RTP packets from UDP or WS ───────────────────────
                rtp = tokio::task::spawn_blocking({
                    let rx = rtp_rx.clone();
                    move || rx.recv_timeout(Duration::from_millis(50))
                }) => {
                    match rtp {
                        Ok(Ok(packet)) => {
                            let mut buffer = gstreamer::Buffer::with_size(packet.len())
                                .map_err(|_| anyhow::anyhow!("GStreamer buffer alloc failed"))?;
                            {
                                let buf_mut = buffer.get_mut().unwrap();
                                buf_mut.copy_from_slice(0, &packet).ok();
                            }
                            if appsrc.push_buffer(buffer) != Ok(gstreamer::FlowSuccess::Ok) {
                                warn!("WSC-RTP: appsrc push_buffer failed");
                            }
                        }
                        Ok(Err(flume::RecvTimeoutError::Timeout)) => {}
                        Ok(Err(flume::RecvTimeoutError::Disconnected)) => {
                            info!("WSC-RTP: RTP channel disconnected");
                            pipeline.set_state(gstreamer::State::Null).ok();
                            return Ok(());
                        }
                        Err(e) => {
                            warn!("WSC-RTP: spawn_blocking join error: {}", e);
                        }
                    }
                }
            }
        }
    }

    // ─── Internal helpers ────────────────────────────────────────────

    fn handle_server_message(&self, message: WscRtpServerMessage) {
        match message {
            WscRtpServerMessage::Init { .. } => {}
            WscRtpServerMessage::Sdp { .. } => {}
            WscRtpServerMessage::StreamState { state } => {
                push_event(&self.events_sink, StreamEvent::WscRtpStreamState(state));
            }
            WscRtpServerMessage::SessionMode(_mode) => {}
            WscRtpServerMessage::Error { message } => {
                push_event(&self.events_sink, StreamEvent::Error(message));
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
        body: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut url = Url::parse(&self.base_url)?;
        url.set_path(&format!(
            "/streams/{}/wsc-rtp/{}/{}",
            self.source_id, self.session_id, endpoint
        ));

        let mut req = self.http_client.post(url.as_str());
        if let Some(body) = body {
            req = req.json(&body);
        }

        let response = req.send().await.context("WSC-RTP control request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("WSC-RTP control request failed with status: {}", status);
        }

        let mode: media_server_api_models::SessionModeResponse = response
            .json()
            .await
            .context("parsing WSC-RTP control response")?;
        push_event(
            &self.events_sink,
            StreamEvent::WscRtpSessionMode {
                is_live: mode.is_live,
                current_time_ms: mode.current_time_ms.unwrap_or(0) as i64,
                speed: mode.speed,
            },
        );

        Ok(())
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

#[derive(Clone)]
pub struct WscRtpShutdownHandle {
    tx: tokio::sync::mpsc::Sender<WscRtpSessionCommand>,
}

impl WscRtpShutdownHandle {
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(WscRtpSessionCommand::Shutdown);
    }
}

// ─── Public setup entry point ────────────────────────────────────────────────

/// Creates a WSC-RTP session. Returns the `Arc<WscRtpSession>` (for seek/live/speed
/// calls), a shutdown handle, and a tokio JoinHandle for the execute task.
///
/// Texture creation and frame marking happen on the platform thread via the
/// `event_tx` → output loop path (unchanged from before).
pub async fn setup_wsc_rtp_session(
    config: &WscRtpSessionConfig,
    events_sink: crate::core::types::DartEventsStream,
    event_tx: InputEventSender,
    output_dims: Arc<Mutex<VideoDimensions>>,
    payload_holder: Weak<crate::core::texture::payload::PayloadHolder>,
    texture_id: i64,
) -> Result<(
    Arc<WscRtpSession>,
    WscRtpShutdownHandle,
    tokio::task::JoinHandle<()>,
)> {
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<WscRtpSessionCommand>(32);

    let (session, ws_sink, ws_stream) = WscRtpSession::new(config, events_sink, command_rx).await?;

    let session_clone = Arc::clone(&session);
    let handle = tokio::spawn(async move {
        session_clone
            .execute(
                ws_sink,
                ws_stream,
                event_tx,
                output_dims,
                payload_holder,
                texture_id,
            )
            .await;
    });

    let shutdown = WscRtpShutdownHandle { tx: command_tx };

    Ok((session, shutdown, handle))
}

fn is_timeout_err(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

fn run_udp_receiver(socket: UdpSocket, tx: flume::Sender<Vec<u8>>) {
    let mut buf = vec![0u8; 65536];
    loop {
        match socket.recv(&mut buf) {
            Ok(n) if n > 0 => {
                if tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!("WSC-RTP UDP receive error: {}", e);
                break;
            }
        }
    }
    info!("WSC-RTP UDP receive thread exiting");
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

fn build_pipeline_str(
    encoding: &str,
    pt: u8,
    clock_rate: u32,
    sprop: &Option<String>,
    width: u32,
    height: u32,
) -> String {
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
         ! videoscale \
         ! video/x-raw,format=RGBA,width={width},height={height} \
         ! appsink name=sink sync=false emit-signals=true",
    )
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn push_event(events_sink: &crate::core::types::DartEventsStream, event: StreamEvent) {
    events_sink.add(event).log_err();
}

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

fn log_sdp_preview(sdp_text: &str) {
    let preview: Vec<&str> = sdp_text.lines().take(8).collect();
    if preview.is_empty() {
        warn!("WSC-RTP SDP preview is empty");
        return;
    }
    info!("WSC-RTP SDP preview:\n{}", preview.join("\n"));
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
