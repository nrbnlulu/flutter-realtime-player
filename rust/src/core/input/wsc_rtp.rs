use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{mpsc, Arc, Mutex, Weak},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use flume::Sender;
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};
use url::Url;

use crate::{
    core::{
        input::{InputCommand, InputCommandReceiver, InputEvent, InputEventSender, VideoInput},
        texture::payload::{RawRgbaFrame, SharedPixelData},
        types::{VideoDimensions, WscSdpEndpoint},
    },
    dart_types::StreamEvent,
    utils::LogErr,
};

// ─── Protocol constants ──────────────────────────────────────────────────────

const HOLEPUNCH_HEADER: &str = "ws-rtp";
const DUMMY_HEADER: &str = "ws-rtp-dummy";
const ACK_HEADER: &str = "ws-rtp-ack";
const UDP_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);

// ─── Public API ──────────────────────────────────────────────────────────────

pub struct WscRtpSetup {
    pub sdp_text: String,
    pub rtp_rx: flume::Receiver<Vec<u8>>,
    pub wsc_rtp_control: WscRtpControl,
    pub cleanup: WscRtpSessionCleanup,
}

impl WscRtpSetup {
    pub fn cleanup(self) {
        self.cleanup.cleanup();
    }
}

#[derive(Clone)]
pub struct WscRtpControl {
    base_url: String,
    source_id: String,
    token: Arc<Mutex<Option<String>>>,
    http_client: reqwest::blocking::Client,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
}

#[derive(Debug, Deserialize)]
struct SessionModeResponse {
    is_live: bool,
    current_time_ms: Option<u64>,
    speed: f64,
}

impl WscRtpControl {
    fn get_control_url(&self, endpoint: &str) -> Result<String> {
        let token_guard = self.token.lock().unwrap();
        let token = token_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WSC-RTP session token not yet available"))?;
        let mut url = Url::parse(&self.base_url)?;
        url.set_path(&format!(
            "/streams/{}/wsc-rtp/{}/{}",
            self.source_id, token, endpoint
        ));
        Ok(url.to_string())
    }

    fn handle_response(&self, response: reqwest::blocking::Response) -> Result<()> {
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "WSC-RTP request failed with status: {}",
                status
            ));
        }
        let mode: SessionModeResponse = response.json().context("parsing WSC-RTP response")?;
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

    pub fn seek(&self, timestamp_ms: i64) -> Result<()> {
        let url = self.get_control_url("seek")?;
        let body = serde_json::json!({ "timestamp": timestamp_ms });
        let response = match self.http_client.post(&url).json(&body).send() {
            Ok(r) => r,
            Err(err) => bail!("WSC-RTP seek request failed for url {}: {}", url, err),
        };
        self.handle_response(response)
            .context("WSC-RTP seek response error")
    }

    pub fn live(&self) -> Result<()> {
        let url = self.get_control_url("live")?;
        let response = self
            .http_client
            .post(url)
            .send()
            .context("WSC-RTP live request failed")?;
        self.handle_response(response)
            .context("WSC-RTP live response error")
    }

    pub fn set_speed(&self, speed: f64) -> Result<()> {
        let url = self.get_control_url("speed")?;
        let body = serde_json::json!({ "speed": speed });
        let response = self
            .http_client
            .post(url)
            .json(&body)
            .send()
            .context("WSC-RTP set_speed request failed")?;
        self.handle_response(response)
            .context("WSC-RTP set_speed response error")
    }
}

pub struct WscRtpSessionCleanup {
    command_tx: Sender<WscRtpCommand>,
}

impl WscRtpSessionCleanup {
    pub fn cleanup(self) {
        let _ = self.command_tx.send(WscRtpCommand::Terminate);
    }
}

// ─── Internal types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Ping,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Init { token: String, holepunch_port: u16 },
    Sdp { sdp: String },
    StreamState { state: String },
    Error { message: String },
    Pong,
}

#[derive(Debug)]
enum WscRtpCommand {
    Terminate,
}

// ─── Session setup ───────────────────────────────────────────────────────────

pub fn setup_wsc_rtp_session(
    endpoint: &WscSdpEndpoint,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
) -> anyhow::Result<WscRtpSetup> {
    let _base_url = Url::parse(&endpoint.base_url).context("invalid base_url")?;
    info!(
        "WSC-RTP setup: base_url={}, source_id={}",
        endpoint.base_url, endpoint.source_id
    );

    let wsc_rtp_url = build_wsc_rtp_url(&endpoint.base_url, &endpoint.source_id)?;
    let (command_tx, command_rx) = flume::unbounded();
    let (sdp_tx, sdp_rx) = mpsc::channel();
    let (rtp_tx, rtp_rx) = flume::unbounded::<Vec<u8>>();

    let token = Arc::new(Mutex::new(None));

    let base_url = endpoint.base_url.clone();
    let source_id = endpoint.source_id.clone();
    let events_sink_clone = Arc::clone(&events_sink);
    let token_clone = Arc::clone(&token);

    thread::spawn(move || {
        if let Err(err) = run_wsc_rtp_session(
            wsc_rtp_url,
            &base_url,
            &source_id,
            command_rx,
            &sdp_tx,
            events_sink_clone,
            token_clone,
            rtp_tx,
        ) {
            warn!(
                "WSC-RTP session thread error: base_url={}, source_id={}, error={}",
                base_url, source_id, err
            );
            let _ = sdp_tx.send(Err(err));
        }
    });

    let sdp_text = sdp_rx
        .recv_timeout(Duration::from_secs(15))
        .context("waiting for WSC-RTP SDP")??;
    log_sdp_preview(&sdp_text);
    info!("WSC-RTP SDP received ({} bytes)", sdp_text.len());

    Ok(WscRtpSetup {
        sdp_text,
        rtp_rx,
        wsc_rtp_control: WscRtpControl {
            base_url: endpoint.base_url.clone(),
            source_id: endpoint.source_id.clone(),
            token,
            http_client: reqwest::blocking::Client::new(),
            events_sink: Arc::clone(&events_sink),
        },
        cleanup: WscRtpSessionCleanup { command_tx },
    })
}

fn run_wsc_rtp_session(
    wsc_rtp_url: Url,
    base_url: &str,
    _source_id: &str,
    command_rx: flume::Receiver<WscRtpCommand>,
    sdp_tx: &mpsc::Sender<anyhow::Result<String>>,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
    token_shared: Arc<Mutex<Option<String>>>,
    rtp_tx: flume::Sender<Vec<u8>>,
) -> Result<()> {
    let (mut socket, _) = connect(wsc_rtp_url.to_string()).context("connecting to WSC-RTP ws")?;
    set_nonblocking(&mut socket);

    let mut sdp_sent = false;
    let mut last_ping = std::time::Instant::now();

    loop {
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                WscRtpCommand::Terminate => {
                    let _ = socket.close(None);
                    return Ok(());
                }
            }
        }

        match socket.read() {
            Ok(message) => match message {
                Message::Ping(payload) => {
                    let _ = socket.send(Message::Pong(payload));
                }
                Message::Binary(data) => {
                    // WebSocket fallback: server sends RTP packets as binary frames
                    // when UDP is blocked
                    let _ = rtp_tx.send(data.to_vec());
                }
                other => {
                    if let Some(server_message) = decode_text_message(other)? {
                        handle_server_message(
                            server_message,
                            base_url,
                            &token_shared,
                            &mut sdp_sent,
                            sdp_tx,
                            &events_sink,
                            &rtp_tx,
                        )?;
                    }
                }
            },
            Err(tungstenite::Error::Io(err)) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(tungstenite::Error::ConnectionClosed) | Err(tungstenite::Error::AlreadyClosed) => {
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        }

        if last_ping.elapsed() >= Duration::from_secs(2) {
            let _ = send_message(&mut socket, ClientMessage::Ping);
            last_ping = std::time::Instant::now();
        }

        thread::sleep(Duration::from_millis(20));
    }
}

fn handle_server_message(
    message: ServerMessage,
    base_url: &str,
    token_shared: &Arc<Mutex<Option<String>>>,
    sdp_sent: &mut bool,
    sdp_tx: &mpsc::Sender<anyhow::Result<String>>,
    events_sink: &Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
    rtp_tx: &flume::Sender<Vec<u8>>,
) -> Result<()> {
    match message {
        ServerMessage::Init {
            token: init_token,
            holepunch_port,
        } => {
            {
                let mut guard = token_shared.lock().unwrap();
                *guard = Some(init_token.clone());
            }
            // Attempt UDP holepunch; if confirmed, spawn UDP receive thread.
            // If UDP is blocked, the server will fall back to binary WebSocket frames
            // which are handled directly in the main loop above.
            try_udp_holepunch(base_url, holepunch_port, &init_token, rtp_tx);
        }
        ServerMessage::Sdp { sdp } => {
            if !*sdp_sent {
                *sdp_sent = true;
                let _ = sdp_tx.send(Ok(sdp));
            }
        }
        ServerMessage::StreamState { state } => {
            push_event(events_sink, StreamEvent::WscRtpStreamState(state));
        }
        ServerMessage::Error { message } => {
            push_event(events_sink, StreamEvent::Error(message.clone()));
            if !*sdp_sent {
                let _ = sdp_tx.send(Err(anyhow::anyhow!("WSC-RTP error: {}", message)));
                *sdp_sent = true;
            }
        }
        ServerMessage::Pong => {}
    }
    Ok(())
}

// ─── UDP holepunch + handshake ───────────────────────────────────────────────

/// Attempts UDP holepunch. If the dummy/ack handshake succeeds, spawns a UDP
/// receive thread. If UDP is blocked (timeout), does nothing — the server will
/// automatically fall back to sending RTP as binary WebSocket frames.
fn try_udp_holepunch(
    base_url: &str,
    holepunch_port: u16,
    token: &str,
    rtp_tx: &flume::Sender<Vec<u8>>,
) {
    let server_addr = match resolve_server_addr(base_url, holepunch_port) {
        Ok(addr) => addr,
        Err(e) => {
            warn!("WSC-RTP: failed to resolve server addr: {}", e);
            return;
        }
    };

    let bind_addr = match server_addr {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    };

    let udp = match UdpSocket::bind(bind_addr) {
        Ok(s) => s,
        Err(e) => {
            warn!("WSC-RTP: failed to bind UDP socket: {}", e);
            return;
        }
    };

    let holepunch = format!("{} {}", HOLEPUNCH_HEADER, token);
    if let Err(e) = udp.send_to(holepunch.as_bytes(), server_addr) {
        warn!("WSC-RTP: UDP holepunch send failed: {}", e);
        return;
    }
    info!("WSC-RTP: sent UDP holepunch to {}", server_addr);

    // Wait for dummy packet
    if let Err(e) = udp.set_read_timeout(Some(UDP_HANDSHAKE_TIMEOUT)) {
        warn!("WSC-RTP: set_read_timeout failed: {}", e);
        return;
    }

    let mut buf = [0u8; 512];
    let expected_dummy = format!("{} {}", DUMMY_HEADER, token);

    match udp.recv_from(&mut buf) {
        Ok((n, _src)) => {
            if let Ok(payload) = std::str::from_utf8(&buf[..n]) {
                if payload.trim() == expected_dummy {
                    let ack = format!("{} {}", ACK_HEADER, token);
                    if let Err(e) = udp.send_to(ack.as_bytes(), server_addr) {
                        warn!("WSC-RTP: UDP ack send failed: {}", e);
                        return;
                    }
                    info!("WSC-RTP: UDP confirmed, spawning UDP receive thread");
                    udp.set_read_timeout(None).ok();
                    let tx = rtp_tx.clone();
                    thread::spawn(move || run_udp_receiver(udp, tx));
                } else {
                    warn!(
                        "WSC-RTP: unexpected UDP payload {:?}, falling back to WS",
                        payload
                    );
                }
            }
        }
        Err(e) if is_timeout_err(&e) => {
            info!("WSC-RTP: UDP dummy timeout — server will use WebSocket binary fallback");
        }
        Err(e) => {
            warn!(
                "WSC-RTP: UDP recv error: {} — server will use WebSocket binary fallback",
                e
            );
        }
    }
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

// ─── GStreamer VideoInput implementation ─────────────────────────────────────

pub struct GstreamerWscRtpInput {
    sdp_text: String,
    rtp_rx: flume::Receiver<Vec<u8>>,
    output_dims: Arc<Mutex<VideoDimensions>>,
    payload_holder: Weak<crate::core::texture::payload::PayloadHolder>,
}

impl GstreamerWscRtpInput {
    pub fn new(
        sdp_text: &str,
        rtp_rx: flume::Receiver<Vec<u8>>,
        output_dims: VideoDimensions,
        payload_holder: Weak<crate::core::texture::payload::PayloadHolder>,
    ) -> Arc<Self> {
        Arc::new(Self {
            sdp_text: sdp_text.to_string(),
            rtp_rx,
            output_dims: Arc::new(Mutex::new(output_dims)),
            payload_holder,
        })
    }

    fn parse_rtp_caps_from_sdp(sdp: &str) -> Option<(String, u8, u32)> {
        for line in sdp.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("a=rtpmap:") {
                let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let pt: u8 = parts[0].parse().ok()?;
                    let codec_parts: Vec<&str> = parts[1].splitn(2, '/').collect();
                    if codec_parts.len() >= 2 {
                        let encoding = codec_parts[0].to_uppercase();
                        let clock_rate: u32 = codec_parts[1].parse().ok()?;
                        return Some((encoding, pt, clock_rate));
                    }
                }
            }
        }
        None
    }

    fn build_pipeline_str(
        encoding: &str,
        pt: u8,
        clock_rate: u32,
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

        format!(
            "appsrc name=src caps=\"application/x-rtp,media=video,payload={pt},clock-rate={clock_rate},encoding-name={encoding}\" format=time is-live=true \
             ! rtpjitterbuffer \
             ! {depay_decode} \
             ! videoconvert \
             ! videoscale \
             ! video/x-raw,format=RGBA,width={width},height={height} \
             ! appsink name=sink sync=false emit-signals=true",
        )
    }
}

impl VideoInput for GstreamerWscRtpInput {
    fn execute(
        &self,
        event_tx: InputEventSender,
        command_rx: InputCommandReceiver,
        texture_id: i64,
    ) -> Result<()> {
        let (encoding, pt, clock_rate) = Self::parse_rtp_caps_from_sdp(&self.sdp_text)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse RTP caps from SDP"))?;

        let dims = self.output_dims.lock().unwrap().clone();
        let pipeline_str =
            Self::build_pipeline_str(&encoding, pt, clock_rate, dims.width, dims.height);
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

        let payload_holder = Weak::clone(&self.payload_holder);
        let event_tx_clone = event_tx.clone();
        let out_dims = Arc::clone(&self.output_dims);
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

        loop {
            while let Ok(cmd) = command_rx.try_recv() {
                match cmd {
                    InputCommand::Terminate => {
                        info!("WSC-RTP GStreamer: terminating pipeline");
                        pipeline.set_state(gstreamer::State::Null).ok();
                        event_tx
                            .send(InputEvent::State(crate::dart_types::StreamState::Stopped))
                            .ok();
                        return Ok(());
                    }
                    InputCommand::Resize { width, height } => {
                        let mut dims = self.output_dims.lock().unwrap();
                        dims.width = width;
                        dims.height = height;
                    }
                    InputCommand::Seek { .. } => {
                        // Seek not applicable for live RTP
                    }
                }
            }

            match self.rtp_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(packet) => {
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
                Err(flume::RecvTimeoutError::Timeout) => {}
                Err(flume::RecvTimeoutError::Disconnected) => {
                    info!("WSC-RTP: RTP channel closed, stopping pipeline");
                    pipeline.set_state(gstreamer::State::Null).ok();
                    event_tx
                        .send(InputEvent::State(crate::dart_types::StreamState::Stopped))
                        .ok();
                    return Ok(());
                }
            }
        }
    }

    fn output_dimensions(&self) -> VideoDimensions {
        self.output_dims.lock().unwrap().clone()
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn push_event(
    events_sink: &Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
    event: StreamEvent,
) {
    if let Ok(guard) = events_sink.lock() {
        if let Some(sink) = guard.as_ref() {
            sink.add(event).log_err();
        }
    }
}

fn decode_text_message(message: Message) -> Result<Option<ServerMessage>> {
    match message {
        Message::Text(text) => Ok(Some(
            serde_json::from_str(&text).context("parsing WSC-RTP message")?,
        )),
        Message::Pong(_) => Ok(Some(ServerMessage::Pong)),
        Message::Close(_) => Ok(None),
        _ => Ok(None),
    }
}

fn send_message(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    message: ClientMessage,
) -> Result<()> {
    let payload = serde_json::to_string(&message).context("serializing WSC-RTP message")?;
    socket
        .send(Message::Text(payload))
        .context("sending WSC-RTP message")?;
    Ok(())
}

fn set_nonblocking(
    socket: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
) {
    use tungstenite::stream::MaybeTlsStream;
    let stream = socket.get_mut();
    match stream {
        MaybeTlsStream::Plain(inner) => {
            let _ = inner.set_nonblocking(true);
        }
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

fn build_wsc_rtp_url(base_url: &str, source_id: &str) -> Result<Url> {
    let mut url = Url::parse(base_url).context("invalid base_url")?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => other,
    }
    .to_string();
    url.set_scheme(&scheme)
        .map_err(|_| anyhow::anyhow!("invalid base_url scheme"))?;
    url.set_path(&format!("/streams/{}/wsc-rtp", source_id));
    url.set_query(None);
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

fn resolve_server_addr(base_url: &str, port: u16) -> Result<SocketAddr> {
    let url = Url::parse(base_url).context("invalid base_url")?;
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
