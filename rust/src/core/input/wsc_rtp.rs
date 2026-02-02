use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use flume::Sender;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};
use url::Url;

use crate::{core::types::WscRtpEndpoint, dart_types::StreamEvent, utils::LogErr};

pub struct WscRtpSetup {
    pub sdp_data: Arc<Vec<u8>>,
    pub client_port: u16,
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

        // base_url usually looks like "http://host:port" or "https://host:port"
        // We need to construct: http://{host}:{port}/streams/{source_id}/wsc-rtp/{token}/{endpoint}

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
            Ok(response) => response,
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

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Ping,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Init {
        token: String,
        server_port: u16,
        #[serde(default)]
        udp_holepunch_required: bool,
    },
    Sdp {
        sdp: String,
    },
    StreamState {
        state: String,
    },
    Error {
        message: String,
    },
    Pong,
}

#[derive(Debug)]
enum WscRtpCommand {
    Terminate,
}

pub fn setup_wsc_rtp_session(
    endpoint: &WscRtpEndpoint,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
) -> anyhow::Result<WscRtpSetup> {
    let _base_url = Url::parse(&endpoint.base_url).context("invalid base_url")?;
    info!(
        "WSC-RTP setup: base_url={}, source_id={}, client_port={:?}",
        endpoint.base_url, endpoint.source_id, endpoint.client_port
    );

    let announce_port = if let Some(port) = endpoint.client_port {
        port
    } else {
        let port = pick_ephemeral_rtp_port()?;
        warn!(
            "WSC-RTP client_port not provided; using {} for SDP/UDP holepunch. NAT may require explicit client_port.",
            port
        );
        port
    };

    let wsc_rtp_url = build_wsc_rtp_url(&endpoint.base_url, &endpoint.source_id)?;
    let (command_tx, command_rx) = flume::unbounded();
    let (sdp_tx, sdp_rx) = mpsc::channel();

    // Shared token container
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
            announce_port,
            command_rx,
            sdp_tx,
            events_sink_clone,
            token_clone,
        ) {
            warn!(
                "WSC-RTP session thread error: base_url={}, source_id={}, error={}",
                base_url, source_id, err
            );
        }
    });

    let sdp_text = sdp_rx
        .recv_timeout(Duration::from_secs(15))
        .context("waiting for WSC-RTP SDP")??;
    log_sdp_preview(&sdp_text);

    let sdp_data = Arc::new(ensure_sdp_line_endings(&sdp_text));
    info!("WSC-RTP SDP length: {} bytes", sdp_data.len());

    Ok(WscRtpSetup {
        sdp_data,
        client_port: announce_port,
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
    announce_port: u16,
    command_rx: flume::Receiver<WscRtpCommand>,
    sdp_tx: mpsc::Sender<anyhow::Result<String>>,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
    token_shared: Arc<Mutex<Option<String>>>,
) -> Result<()> {
    let (mut socket, _) = connect(wsc_rtp_url.to_string()).context("connecting to WSC-RTP ws")?;
    set_nonblocking(&mut socket);

    // Initial Init message from client is removed per new spec.
    // Client waits for server Init message.

    let mut server_port: Option<u16> = None;
    let mut sdp_sent = false;
    let mut last_ping = Instant::now();

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
                other => {
                    if let Some(server_message) = decode_message(other)? {
                        handle_server_message(
                            server_message,
                            base_url,
                            announce_port,
                            &token_shared,
                            &mut server_port,
                            &mut sdp_sent,
                            &sdp_tx,
                            &events_sink,
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
            last_ping = Instant::now();
        }

        thread::sleep(Duration::from_millis(20));
    }
}

fn handle_server_message(
    message: ServerMessage,
    base_url: &str,
    announce_port: u16,
    token_shared: &Arc<Mutex<Option<String>>>,
    server_port: &mut Option<u16>,
    sdp_sent: &mut bool,
    sdp_tx: &mpsc::Sender<anyhow::Result<String>>,
    events_sink: &Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
) -> Result<()> {
    match message {
        ServerMessage::Init {
            token: init_token,
            server_port: init_server_port,
            udp_holepunch_required,
        } => {
            {
                let mut guard = token_shared.lock().unwrap();
                *guard = Some(init_token.clone());
            }
            *server_port = Some(init_server_port);
            if udp_holepunch_required {
                send_udp_holepunch(base_url, init_server_port, &init_token, announce_port)?;
            }
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

fn decode_message(message: Message) -> Result<Option<ServerMessage>> {
    match message {
        Message::Text(text) => Ok(Some(
            serde_json::from_str(&text).context("parsing WSC-RTP message")?,
        )),
        Message::Binary(data) => Ok(Some(
            serde_json::from_slice(&data).context("parsing WSC-RTP binary message")?,
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

fn ensure_sdp_line_endings(sdp_text: &str) -> Vec<u8> {
    let trimmed = sdp_text.trim_end_matches(['\r', '\n']);
    let mut normalized = trimmed.replace('\n', "\r\n");
    if !normalized.ends_with("\r\n") {
        normalized.push_str("\r\n");
    }
    normalized.into_bytes()
}

fn pick_ephemeral_rtp_port() -> anyhow::Result<u16> {
    for _ in 0..20 {
        let port = pick_ephemeral_port()?;
        if port % 2 == 0 {
            return Ok(port);
        }
    }
    pick_ephemeral_port()
}

fn pick_ephemeral_port() -> anyhow::Result<u16> {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        return Ok(socket.local_addr().context("get local port")?.port());
    }
    let socket = UdpSocket::bind("[::]:0").context("binding ipv6 udp socket")?;
    Ok(socket.local_addr().context("get local port")?.port())
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
        .find(|addr| addr.is_ipv4())
        .cloned()
        .or_else(|| addrs.first().cloned())
        .ok_or_else(|| anyhow::anyhow!("no addresses resolved for {}", host))?;
    Ok(addr)
}

fn send_udp_holepunch(
    base_url: &str,
    server_port: u16,
    token: &str,
    client_port: u16,
) -> Result<()> {
    let server_addr = resolve_server_addr(base_url, server_port).context("resolve server")?;
    let bind_addr = match server_addr {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), client_port),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), client_port),
    };
    let socket = UdpSocket::bind(bind_addr).context("binding udp holepunch socket")?;
    let payload = format!("t5rtp {} {}", token, client_port);
    socket
        .send_to(payload.as_bytes(), server_addr)
        .context("sending udp holepunch")?;
    Ok(())
}
