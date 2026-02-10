use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use irondash_texture::Texture;
use log::{info, warn};

use tokio::net::{TcpStream, UdpSocket};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::{
    core::{
        input::{InputEvent, InputEventSender},
        output::flutter_pixelbuffer::create_flutter_pixelbuffer,
        session::VideoSessionCommon,
        texture::{
            FlutterTextureSession, payload::{self, RawRgbaFrame, SharedPixelData}
        },
        types::{VideoDimensions, WscRtpSessionConfig},
    },
    dart_types::{StreamEvent, StreamState},
    utils::{LogErr, invoke_on_platform_main_thread},
};

use media_server_api_models::{WscRtpClientMessage, WscRtpServerMessage};

const UDP_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const PING_INTERVAL: Duration = Duration::from_secs(2);
const SDP_TIMEOUT: Duration = Duration::from_secs(15);

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
    http_client: Arc<reqwest::Client>,
    pipeline: Arc<gstreamer::Pipeline>,
    execution_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl WscRtpSession {
    /// Connect to the WSC-RTP websocket, wait for Init + SDP messages,
    /// and return the session together with the split websocket streams.
    pub async fn new(
        config: WscRtpSessionConfig,
        session_common: VideoSessionCommon,
        http_client: Arc<reqwest::Client>,
    ) -> Result<(Arc<Self>, WsSink, WsStream, Option<UdpSocket>)> {
        let server_url = Url::parse(&config.base_url).context("parsing media server HTTP URL")?;
        let wsc_rtp_url = build_wsc_rtp_handshake_request(
            &server_url,
            &config.source_id,
            config.force_websocket_transport,
        )?;
        info!("WSC-RTP connecting to {}", wsc_rtp_url);

        let (ws, _) = connect_async(wsc_rtp_url.to_string())
            .await
            .context("connecting to WSC-RTP ws")?;
        let (ws_sink, mut ws_stream) = ws.split();

        let deadline = tokio::time::Instant::now() + SDP_TIMEOUT;
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

        let initial_sdp =
            initial_sdp.ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before SDP"))?;
        let (session_id, holepunch_port) = init_message
            .ok_or_else(|| anyhow::anyhow!("WSC-RTP websocket closed before initialization"))?;

        // the ws wasn't initialized with force_websocket
        // do holepunch and check if we can receive udp packets
        let mut udp_sock_maybe = None;
        if !config.force_websocket_transport {
            let bind_addr = resolve_server_udp_addr(&server_url, holepunch_port)?;

            let mut udp_sock = UdpSocket::bind(bind_addr).await?;
            if let Err(e) = validate_udp_handshare(&session_id, &mut udp_sock).await {
                log::warn!("failed to handshake for udp transport in session {} due to {} falling back to websockets", session_id, e);
            }
            udp_sock_maybe = Some(udp_sock);
        }

        let (encoding, pt, clock_rate, sprop) = parse_rtp_caps_from_sdp(&initial_sdp)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse RTP caps from SDP"))?;
        // TODO: real dimensions
        let pipeline_str = build_pipeline_str(&encoding, pt, clock_rate, &sprop, 640, 480);
        info!("WSC-RTP GStreamer pipeline: {}", pipeline_str);

        let pipeline = gstreamer::parse::launch(&pipeline_str)
            .context("GStreamer pipeline launch")?
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Not a pipeline"))?;

        let session = Arc::new(Self {
            execution_task: Mutex::new(None),
            pipeline: Arc::new(pipeline),
            session_common,
            initial_sdp,
            session_id: session_id,
            holepunch_port,
            media_server_http_url: server_url,
            source_id: config.source_id.clone(),
            http_client: http_client,
        });

        Ok((session, ws_sink, ws_stream, udp_sock_maybe))
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
        udp_rtp_transport_sock: Option<UdpSocket>,
    ) -> anyhow::Result<()> {
        let appsrc = self
            .pipeline
            .by_name("src")
            .ok_or_else(|| anyhow::anyhow!("appsrc not found"))?
            .downcast::<AppSrc>()
            .map_err(|_| anyhow::anyhow!("src is not AppSrc"))?;

        let appsink = self
            .pipeline
            .by_name("sink")
            .ok_or_else(|| anyhow::anyhow!("appsink not found"))?
            .downcast::<gstreamer_app::AppSink>()
            .map_err(|_| anyhow::anyhow!("sink is not AppSink"))?;

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
        self.session_common.send_state_msg(StreamState::Playing {
            texture_id,
            seekable: true,
        });

        appsink.set_callbacks(
            gstreamer_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
                    let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                    let map = buffer
                        .map_readable()
                        .map_err(|_| gstreamer::FlowError::Error)?;
                    let frame;
                    if let Some((width, height)) = sample.caps().and_then(|caps| {
                        let structure = caps.structure(0)?;

                        let width = structure.get::<i32>("width").ok()?;
                        let height = structure.get::<i32>("height").ok()?;

                        Some((width, height))
                    }) {
                        frame = RawRgbaFrame {
                            width: width as _,
                            height: height as _,
                            data: map.as_slice().to_vec(),
                        };
                    } else {
                        frame = RawRgbaFrame {
                            width: 0,
                            height: 0,
                            data: Vec::new(),
                        };
                    }
                    if let Some(holder) = payload_holder_weak.upgrade() {
                        holder.set_payload(Arc::new(frame) as SharedPixelData);
                        texture_session.mark_frame_available();
                    }
                    Ok(gstreamer::FlowSuccess::Ok)
                })
                .build(),
        );

        async fn udp_packet_receiver(appsrc: AppSrc, udp_sock: UdpSocket) {
            // 1500 is standard MTU size for Ethernet frames
            let mut buf = [0u8; 1500];
            while let Ok((len, _)) = udp_sock.recv_from(&mut buf).await {
                let gst_buffer = gstreamer::Buffer::from_slice(buf[..len].to_vec());
                if let Err(err) = appsrc.push_buffer(gst_buffer) {
                    log::warn!("WSC-RTP: appsrc push_buffer failed: {}", err);
                    break;
                }
            }
        }

        let mut udp_packet_rcv_task = None;
        if let Some(udp_rtp_transport_sock) = udp_rtp_transport_sock {
            udp_packet_rcv_task = Some(tokio::spawn(udp_packet_receiver(
                appsrc.clone(),
                udp_rtp_transport_sock,
            )));
        }

        let mut ping_interval = tokio::time::interval(PING_INTERVAL);
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut output: anyhow::Result<()> = Ok(());
        loop {
            tokio::select! {
                // ── Session commands (shutdown only) ──────────────────
                cmd = command_rx.recv() => {
                    match cmd {
                        Some(WscRtpSessionCommand::Shutdown) | None => {
                            info!("WSC-RTP: shutdown command received");
                            break;
                        }
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
                            output = Err(anyhow::anyhow!("WebSocket stream closed"));
                            break;
                        }
                        Some(Err(e)) => {
                            output = Err(anyhow::anyhow!("WebSocket error: {}", e));
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = ws_sink.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            let buffer = gstreamer::Buffer::from_mut_slice(data.to_vec());
                            if let Err(err) = appsrc.push_buffer(buffer) {
                                log::warn!("WSC-RTP: failed to handle binary message: {}", err);
                            }
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
                            output = Err(anyhow::anyhow!("WSC-RTP: received close frame"));
                            break;
                        }
                        Some(Ok(_)) => {}
                    }
                }
            }
            // cleanup
            if let Some(udp_rcv_task) = udp_packet_rcv_task {
                udp_rcv_task.abort();
            }
            self.pipeline.set_state(gstreamer::State::Null)?;
            return output;
        }

        let _ = self
            .session_common
            .send_state_msg(crate::dart_types::StreamState::Stopped);

        invoke_on_platform_main_thread(move || {
            // make sure it is dropped on the main thread
            // because this would call non thread safe flutter stuff back in irondash
            drop(payload_holder)
        });
        Ok(())
    }
    // ─── Internal helpers ────────────────────────────────────────────

    fn handle_server_message(&self, message: WscRtpServerMessage) {
        match message {
            WscRtpServerMessage::Init { .. } => {}
            WscRtpServerMessage::Sdp { .. } => {}
            WscRtpServerMessage::StreamState { .. } => {}
            WscRtpServerMessage::SessionMode(mode) => {
                let (is_live, current_time_ms) = match mode {
                    media_server_api_models::SessionMode::Live => (true, 0),
                    media_server_api_models::SessionMode::Dvr { timestamp } => {
                        (false, timestamp as i64)
                    }
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
        body: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut url = self.media_server_http_url.clone();
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
        self.session_common
            .send_event_msg(StreamEvent::WscRtpSessionMode {
                is_live: mode.is_live,
                current_time_ms: mode.current_time_ms.unwrap_or(0) as i64,
                speed: mode.speed,
            });
        Ok(())
    }
}

#[async_trait]
impl crate::core::session::VideoSession for WscRtpSession{
    
    async fn seek(&self, ts: i64) -> anyhow::Result<()> {
        Self::seek(&self, ts).await
    }
    async fn go_to_live_stream(&self) -> anyhow::Result<()> {
        Self::go_to_live_stream(&self).await
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
        todo!()
    }

    fn set_events_sink(&mut self, sink: crate::core::types::DartEventsStream) {
        self.session_common.set_events_sink(sink);
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        log::warn!("resize not supported yet for wsc rtp");
        Ok(())
    }

    fn destroy(self: Box<Self>) {
        // no need to do anything here, we handle the flutter texture destroyal in the execute fn.
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

fn is_timeout_err(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
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
