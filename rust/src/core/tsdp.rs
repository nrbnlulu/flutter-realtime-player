use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use log::{debug, info, warn};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use super::{
    session::{BaseSession, SessionLifecycle},
    types::{DartEventsStream, TsdpEndpoint},
};

pub struct TsdpSetup {
    pub refresh_tx: mpsc::Sender<()>,
    pub sdp_data: Arc<Vec<u8>>,
}

impl TsdpSetup {
    pub fn cleanup(self) {
        drop(self.refresh_tx);
    }
}

pub struct TsdpSession {
    base: BaseSession,
    refresh_tx: Option<mpsc::Sender<()>>,
}

impl TsdpSession {
    pub fn new(base: BaseSession, setup: TsdpSetup) -> Self {
        Self {
            base,
            refresh_tx: Some(setup.refresh_tx),
        }
    }
}

impl SessionLifecycle for TsdpSession {
    fn session_id(&self) -> i64 {
        self.base.session_id()
    }

    fn engine_handle(&self) -> i64 {
        self.base.engine_handle()
    }

    fn last_alive_mark(&self) -> std::time::SystemTime {
        self.base.last_alive_mark()
    }

    fn make_alive(&mut self) {
        self.base.make_alive();
    }

    fn terminate(&mut self) {
        self.refresh_tx.take();
        self.base.terminate();
    }

    fn set_events_sink(&mut self, sink: DartEventsStream) {
        self.base.set_events_sink(sink);
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        self.base.seek(ts)
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        self.base.resize(width, height)
    }

    fn destroy(self: Box<Self>) {
        let mut session = *self;
        session.terminate();
        session.base.finalize();
    }
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    token: String,
    server_port: u16,
    #[serde(default)]
    udp_holepunch_required: bool,
    #[serde(
        default,
        alias = "refresh_interval_sec",
        alias = "refresh_interval_seconds",
        alias = "keepalive_interval_sec",
        alias = "keepalive_interval_seconds"
    )]
    refresh_interval_secs: Option<u64>,
}

#[derive(Serialize)]
struct RegisterBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    client_port: Option<u16>,
}

#[derive(Serialize)]
struct RefreshBody {
    token: String,
}

pub fn setup_tsdp_session(endpoint: &TsdpEndpoint) -> anyhow::Result<TsdpSetup> {
    let _base_url = reqwest::Url::parse(&endpoint.base_url).context("invalid base_url")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("building http client")?;
    info!(
        "TSDP setup: base_url={}, source_id={}, client_port={:?}",
        endpoint.base_url, endpoint.source_id, endpoint.client_port
    );

    let announce_port = if let Some(port) = endpoint.client_port {
        port
    } else {
        let port = pick_ephemeral_port()?;
        warn!(
            "TSDP client_port not provided; using {} for SDP/UDP holepunch. NAT may require explicit client_port.",
            port
        );
        port
    };
    let register_url = build_url(&endpoint.base_url, &["streams", &endpoint.source_id, "rtp"])?;
    let response: RegisterResponse = client
        .post(register_url)
        .json(&RegisterBody {
            client_port: Some(announce_port),
        })
        .send()
        .context("registering rtp session")?
        .error_for_status()
        .context("register rtp status")?
        .json()
        .context("parsing rtp register response")?;

    let refresh_interval_secs = response.refresh_interval_secs.unwrap_or(10);
    let server_addr =
        resolve_server_addr(&endpoint.base_url, response.server_port).context("resolve server")?;

    if response.udp_holepunch_required {
        if is_loopback_host(&endpoint.base_url) {
            warn!(
                "UDP holepunch requested but base_url is loopback; skipping holepunch for source {}",
                endpoint.source_id
            );
        } else {
            send_udp_holepunch(server_addr, &response.token, announce_port)?;
            info!(
                "Sent UDP holepunch to {} for source {}",
                server_addr, endpoint.source_id
            );
        }
    } else {
        info!(
            "UDP holepunch not required for source {}",
            endpoint.source_id
        );
    }

    let refresh_url = build_url(
        &endpoint.base_url,
        &["streams", &endpoint.source_id, "rtp", "refresh"],
    )?;
    let refresh_body = RefreshBody {
        token: response.token.clone(),
    };
    let (refresh_tx, refresh_rx) = mpsc::channel();
    spawn_refresh_thread(
        client.clone(),
        refresh_url,
        refresh_body,
        refresh_interval_secs,
        refresh_rx,
    );

    let sdp_url = build_url(
        &endpoint.base_url,
        &["streams", &endpoint.source_id, "rtp", "sdp"],
    )?;
    let sdp_text = client
        .get(sdp_url)
        .query(&[("token", response.token.as_str())])
        .send()
        .context("fetching sdp")?
        .error_for_status()
        .context("sdp status")?
        .text()
        .context("reading sdp body")?;
    log_sdp_preview(&sdp_text);

    let sdp_data = Arc::new(ensure_sdp_line_endings(&sdp_text));
    info!("TSDP SDP length: {} bytes", sdp_data.len());
    Ok(TsdpSetup {
        refresh_tx,
        sdp_data,
    })
}

fn ensure_sdp_line_endings(sdp_text: &str) -> Vec<u8> {
    let trimmed = sdp_text.trim_end_matches(['\r', '\n']);
    let mut normalized = trimmed.replace('\n', "\r\n");
    if !normalized.ends_with("\r\n") {
        normalized.push_str("\r\n");
    }
    normalized.into_bytes()
}

fn pick_ephemeral_port() -> anyhow::Result<u16> {
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        return Ok(socket.local_addr().context("get local port")?.port());
    }
    let socket = UdpSocket::bind("[::]:0").context("binding ipv6 udp socket")?;
    Ok(socket.local_addr().context("get local port")?.port())
}

fn is_loopback_host(base_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url) else {
        return false;
    };
    match url.host_str() {
        Some("localhost") | Some("127.0.0.1") | Some("::1") => true,
        _ => false,
    }
}

fn log_sdp_preview(sdp_text: &str) {
    let preview: Vec<&str> = sdp_text.lines().take(8).collect();
    if preview.is_empty() {
        warn!("TSDP SDP preview is empty");
        return;
    }
    info!("TSDP SDP preview:\n{}", preview.join("\n"));
}

fn build_url(base_url: &str, segments: &[&str]) -> Result<reqwest::Url> {
    let mut url = reqwest::Url::parse(base_url).context("invalid base_url")?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("base_url cannot be a base"))?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    Ok(url)
}

fn resolve_server_addr(base_url: &str, port: u16) -> Result<SocketAddr> {
    let url = reqwest::Url::parse(base_url).context("invalid base_url")?;
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

fn send_udp_holepunch(server_addr: SocketAddr, token: &str, client_port: u16) -> Result<()> {
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

fn spawn_refresh_thread(
    client: Client,
    refresh_url: reqwest::Url,
    refresh_body: RefreshBody,
    refresh_interval_secs: u64,
    rx: mpsc::Receiver<()>,
) {
    thread::spawn(move || {
        let interval = Duration::from_secs(refresh_interval_secs.max(1));
        loop {
            match rx.recv_timeout(interval) {
                Ok(_) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            let result = client
                .post(refresh_url.clone())
                .json(&refresh_body)
                .send()
                .and_then(|resp| resp.error_for_status());
            if let Err(err) = result {
                warn!("Failed to refresh TSDP session at {}: {}", refresh_url, err);
            } else {
                debug!("Refreshed TSDP session at {}", refresh_url);
            }
        }
        debug!("Refresh thread exiting for {}", refresh_url);
    });
}
