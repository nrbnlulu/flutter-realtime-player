use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use flume::{Receiver, Sender};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tungstenite::{connect, Message};
use url::Url;

use crate::{core::types::TsdpEndpoint, dart_types::StreamEvent, utils::LogErr};

pub struct TsdpSetup {
    pub sdp_data: Arc<Vec<u8>>,
    pub client_port: u16,
    pub trtp_control: TrtpControl,
    pub cleanup: TsdpSessionCleanup,
}

impl TsdpSetup {
    pub fn cleanup(self) {
        self.cleanup.cleanup();
    }
}

#[derive(Clone)]
pub struct TrtpControl {
    command_tx: Sender<TrtpCommand>,
}

impl TrtpControl {
    pub fn seek(&self, timestamp_ms: i64) -> Result<()> {
        self.command_tx
            .send(TrtpCommand::Seek { timestamp_ms })
            .map_err(|err| anyhow::anyhow!("trtp seek send failed: {}", err))
    }

    pub fn live(&self) -> Result<()> {
        self.command_tx
            .send(TrtpCommand::Live)
            .map_err(|err| anyhow::anyhow!("trtp live send failed: {}", err))
    }

    pub fn set_speed(&self, speed: f64) -> Result<()> {
        self.command_tx
            .send(TrtpCommand::SetSpeed { speed })
            .map_err(|err| anyhow::anyhow!("trtp set_speed send failed: {}", err))
    }
}

pub struct TsdpSessionCleanup {
    command_tx: Sender<TrtpCommand>,
}

impl TsdpSessionCleanup {
    pub fn cleanup(self) {
        let _ = self.command_tx.send(TrtpCommand::Terminate);
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Init { client_port: Option<u16> },
    Seek { timestamp: u64 },
    Live,
    SetSpeed { speed: f64 },
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
    SessionMode {
        is_live: bool,
        current_time_ms: u64,
        speed: f64,
    },
    Error {
        message: String,
    },
    Pong,
}

#[derive(Debug)]
enum TrtpCommand {
    Seek { timestamp_ms: i64 },
    Live,
    SetSpeed { speed: f64 },
    Terminate,
}

pub fn setup_tsdp_session(
    endpoint: &TsdpEndpoint,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
) -> anyhow::Result<TsdpSetup> {
    let _base_url = Url::parse(&endpoint.base_url).context("invalid base_url")?;
    info!(
        "TRTP setup: base_url={}, source_id={}, client_port={:?}",
        endpoint.base_url, endpoint.source_id, endpoint.client_port
    );

    let announce_port = if let Some(port) = endpoint.client_port {
        port
    } else {
        let port = pick_ephemeral_rtp_port()?;
        warn!(
            "TRTP client_port not provided; using {} for SDP/UDP holepunch. NAT may require explicit client_port.",
            port
        );
        port
    };

    let trtp_url = build_trtp_url(&endpoint.base_url, &endpoint.source_id)?;
    let (command_tx, command_rx) = flume::unbounded();
    let (sdp_tx, sdp_rx) = mpsc::channel();

    let base_url = endpoint.base_url.clone();
    let source_id = endpoint.source_id.clone();
    let events_sink_clone = Arc::clone(&events_sink);

    thread::spawn(move || {
        if let Err(err) = run_trtp_session(
            trtp_url,
            &base_url,
            &source_id,
            announce_port,
            command_rx,
            sdp_tx,
            events_sink_clone,
        ) {
            warn!(
                "TRTP session thread error: base_url={}, source_id={}, error={}",
                base_url, source_id, err
            );
        }
    });

    let sdp_text = sdp_rx
        .recv_timeout(Duration::from_secs(15))
        .context("waiting for TRTP SDP")??;
    log_sdp_preview(&sdp_text);

    let sdp_data = Arc::new(ensure_sdp_line_endings(&sdp_text));
    info!("TRTP SDP length: {} bytes", sdp_data.len());

    Ok(TsdpSetup {
        sdp_data,
        client_port: announce_port,
        trtp_control: TrtpControl {
            command_tx: command_tx.clone(),
        },
        cleanup: TsdpSessionCleanup { command_tx },
    })
}

fn run_trtp_session(
    trtp_url: Url,
    base_url: &str,
    source_id: &str,
    announce_port: u16,
    command_rx: Receiver<TrtpCommand>,
    sdp_tx: mpsc::Sender<anyhow::Result<String>>,
    events_sink: Arc<Mutex<Option<crate::core::types::DartEventsStream>>>,
) -> Result<()> {
    let (mut socket, _) = connect(trtp_url.to_string()).context("connecting to TRTP ws")?;
    set_nonblocking(&mut socket);

    send_message(
        &mut socket,
        ClientMessage::Init {
            client_port: Some(announce_port),
        },
    )?;

    let mut token: Option<String> = None;
    let mut server_port: Option<u16> = None;
    let mut sdp_sent = false;
    let mut last_ping = Instant::now();

    loop {
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                TrtpCommand::Seek { timestamp_ms } => {
                    send_message(
                        &mut socket,
                        ClientMessage::Seek {
                            timestamp: timestamp_ms.max(0) as u64,
                        },
                    )?;
                }
                TrtpCommand::Live => {
                    send_message(&mut socket, ClientMessage::Live)?;
                }
                TrtpCommand::SetSpeed { speed } => {
                    send_message(&mut socket, ClientMessage::SetSpeed { speed })?;
                }
                TrtpCommand::Terminate => {
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
                            &mut token,
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

        if last_ping.elapsed() >= Duration::from_secs(15) {
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
    token: &mut Option<String>,
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
            *token = Some(init_token.clone());
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
            push_event(events_sink, StreamEvent::TrtpStreamState(state));
        }
        ServerMessage::SessionMode {
            is_live,
            current_time_ms,
            speed,
        } => {
            push_event(
                events_sink,
                StreamEvent::TrtpSessionMode {
                    is_live,
                    current_time_ms: current_time_ms as i64,
                    speed,
                },
            );
        }
        ServerMessage::Error { message } => {
            push_event(events_sink, StreamEvent::Error(message.clone()));
            if !*sdp_sent {
                let _ = sdp_tx.send(Err(anyhow::anyhow!("TRTP error: {}", message)));
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
            serde_json::from_str(&text).context("parsing TRTP message")?,
        )),
        Message::Binary(data) => Ok(Some(
            serde_json::from_slice(&data).context("parsing TRTP binary message")?,
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
    let payload = serde_json::to_string(&message).context("serializing TRTP message")?;
    socket
        .send(Message::Text(payload))
        .context("sending TRTP message")?;
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

fn build_trtp_url(base_url: &str, source_id: &str) -> Result<Url> {
    let mut url = Url::parse(base_url).context("invalid base_url")?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        other => other,
    }
    .to_string();
    url.set_scheme(&scheme)
        .map_err(|_| anyhow::anyhow!("invalid base_url scheme"))?;
    url.set_path(&format!("/streams/{}/trtp", source_id));
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
        warn!("TRTP SDP preview is empty");
        return;
    }
    info!("TRTP SDP preview:\n{}", preview.join("\n"));
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
