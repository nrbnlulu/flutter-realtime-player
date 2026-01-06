## TRTP Streaming Protocol

### Flow

1. **Register**: `POST /streams/{source_id}/rtp` with JSON `{ "client_port": <u16 or null> }`.
2. **Holepunch (UDP)**: send a datagram to the returned `server_port`:
   - Preferred: `t5rtp <token> <client_port>`
   - Legacy: `<token>`
3. **Get SDP**: `GET /streams/{source_id}/rtp/sdp?token=<token>`.
4. **Refresh**: `POST /streams/{source_id}/rtp/refresh` with JSON `{ "token": "<token>" }` before `refresh_interval_secs` elapses.
5. **Receive**: raw RTP packets (AVP, payload type 96) sent to the registered destination.

### Edge Cases

- **Token required**: UDP holepunch datagrams are ignored unless the token is already registered.
- **Holepunch required for non-loopback**: `GET /rtp/sdp` fails until a holepunch is received, unless the request is from loopback.
- **Loopback shortcut**: On loopback only, SDP can be returned without holepunch, but `client_port` must be provided at registration.
- **NAT port behavior**: For public IPs, the observed UDP source port is authoritative and `client_port` is ignored.
- **LAN override**: For private/loopback/link-local IPs, `client_port` overrides the observed UDP source port.
- **TTL expiry**: registrations expire after 30s of inactivity and are pruned on the next RTP send.
- **Refresh gating**: `refresh` fails if no holepunch was received (destination unset).
- **Server port is ephemeral**: each stream binds to `0.0.0.0:0`, so `server_port` changes per stream instance.
- **SDP address**: uses `0.0.0.0`/`::` in `c=` and `o=` with unicast RTP to the client port.
- **No RTCP**: RTP is unidirectional UDP only; there is no RTCP feedback channel.
___
### Seeking

    - [x] RTSP(h264/5) support with restarts
- [ ] Publishers
    - [ ] raw rtp with NAT hole punch 
    - [x] raw rtp with NAT hole punch 
        - [x] H264
        - [ ] H265
        - [x] H265
        
    - [x] RTSP->webrtc proxy
        - [ ] GOP caching for rtsp streams (https://github.com/bluenviron/mediamtx/pull/4189)
#[utoipa::path(
    post,
    path = "/streams/{source_id}/rtp/{token}/seek",
    params(
        ("source_id" = u64, Path, description = "Stream unique identifier"),
        ("token" = String, Path, description = "RTP registration token")
    ),
    request_body = SeekRequest,
    responses(
        (status = 200, description = "RTP session seeked successfully", body = SessionModeResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Stream or token not found", body = ErrorResponse)
    ),
    tag = "RTP"
)]
pub async fn seek_rtp_session(
    State(state): State<Arc<AppState>>,
    Path((source_id, token)): Path<(u64, String)>,
    Json(req): Json<SeekRequest>,
) -> Result<Json<SessionModeResponse>, ApiError> {
    state
        .stream_manager
        .seek_rtp_session(source_id, &token, req.timestamp)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Stream not found") || msg.contains("token not registered") {
                ApiError::NotFound(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    let mode = state
        .stream_manager
        .get_rtp_session_mode(source_id, &token)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Stream not found") || msg.contains("token not registered") {
                ApiError::NotFound(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    Ok(Json(mode))
}

#[utoipa::path(
    post,
    path = "/streams/{source_id}/rtp/{token}/speed",
    params(
        ("source_id" = u64, Path, description = "Stream unique identifier"),
        ("token" = String, Path, description = "RTP registration token")
    ),
    request_body = SpeedRequest,
    responses(
        (status = 200, description = "RTP speed updated", body = SessionModeResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Stream or token not found", body = ErrorResponse)
    ),
    tag = "RTP"
)]
pub async fn set_rtp_session_speed(
    State(state): State<Arc<AppState>>,
    Path((source_id, token)): Path<(u64, String)>,
    Json(req): Json<SpeedRequest>,
) -> Result<Json<SessionModeResponse>, ApiError> {
    state
        .stream_manager
        .set_rtp_session_speed(source_id, &token, req.speed)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Stream not found") || msg.contains("token not registered") {
                ApiError::NotFound(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    let mode = state
        .stream_manager
        .get_rtp_session_mode(source_id, &token)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Stream not found") || msg.contains("token not registered") {
                ApiError::NotFound(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    Ok(Json(mode))
}

#[utoipa::path(
    get,
    path = "/streams/{source_id}/rtp/{token}/status",
    params(
        ("source_id" = u64, Path, description = "Stream unique identifier"),
        ("token" = String, Path, description = "RTP registration token")
    ),
    responses(
        (status = 200, description = "RTP session status", body = SessionModeResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Stream or token not found", body = ErrorResponse)
    ),
    tag = "RTP"
)]
pub async fn get_rtp_session_status(
    State(state): State<Arc<AppState>>,
    Path((source_id, token)): Path<(u64, String)>,
) -> Result<Json<SessionModeResponse>, ApiError> {
    let mode = state
        .stream_manager
        .get_rtp_session_mode(source_id, &token)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Stream not found") || msg.contains("token not registered") {
                ApiError::NotFound(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    Ok(Json(mode))
}

#[utoipa::path(
    post,
    path = "/streams/{source_id}/webrtc",
    params(
        ("source_id" = u64, Path, description = "Stream unique identifier")

#[utoipa::path(
    post,
    path = "/streams/{source_id}/webrtc/dvr",
    params(
        ("source_id" = u64, Path, description = "Stream unique identifier")
    ),
    request_body = CreateDvrWebRtcSessionRequest,
    responses(
        (status = 201, description = "DVR WebRTC session created successfully", body = CreateWebRtcSessionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Stream not found", body = ErrorResponse)
    ),
    tag = "WebRTC"
)]
pub async fn create_dvr_webrtc_session(
    State(state): State<Arc<AppState>>,
    Path(source_id): Path<u64>,
    Json(req): Json<CreateDvrWebRtcSessionRequest>,
) -> Result<(StatusCode, Json<CreateWebRtcSessionResponse>), ApiError> {
    let client_handle_id = Uuid::new_v4();

    log::debug!(
        "Creating DVR WebRTC session {} for stream {}",
        client_handle_id,
        source_id
    );

    let answer = state
        .stream_manager
        .create_dvr_webrtc_session(source_id, client_handle_id, req.offer, req.timestamp)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to create DVR WebRTC session {} for stream {}: {}",
                client_handle_id,
                source_id,
                e
            );
            ApiError::BadRequest(e.to_string())
        })?;

    log::info!(
        "Created DVR WebRTC session {} for stream {}",
        client_handle_id,
        source_id
    );

    Ok((
        StatusCode::CREATED,
        Json(CreateWebRtcSessionResponse {
            client_handle_id,
            answer,
        }),
    ))
}

#[utoipa::path(
    delete,
    path = "/streams/{source_id}/webrtc/{client_handle_id}",
    params(
        handlers::refresh_rtp_session,
        handlers::get_rtp_sdp,
        handlers::seek_rtp_session,
        handlers::set_rtp_session_speed,
        handlers::get_rtp_session_status,
        handlers::create_webrtc_session,
        handlers::create_dvr_webrtc_session,
        handlers::delete_webrtc_session,
        handlers::list_webrtc_sessions,
        handlers::webrtc_client,
            models::RefreshRtpSessionRequest,
            models::CreateWebRtcSessionRequest,
            models::CreateDvrWebRtcSessionRequest,
            models::CreateWebRtcSessionResponse,
            models::WebRtcSessionResponse,
            models::ListWebRtcSessionsResponse,
        .route("/streams/:source_id/rtp/sdp", get(handlers::get_rtp_sdp))
        .route(
            "/streams/:source_id/rtp/:token/seek",
            post(handlers::seek_rtp_session),
        )
        .route(
            "/streams/:source_id/rtp/:token/speed",
            post(handlers::set_rtp_session_speed),
        )
        .route(
            "/streams/:source_id/rtp/:token/status",
            get(handlers::get_rtp_session_status),
        )
        .route(
            "/streams/:source_id/webrtc",
            post(handlers::create_webrtc_session),
        )
        .route(
            "/streams/:source_id/webrtc/dvr",
            post(handlers::create_dvr_webrtc_session),
        )
        .route(
            "/streams/:source_id/webrtc/:client_handle_id",
            delete(handlers::delete_webrtc_session),
        )

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateDvrWebRtcSessionRequest {
    pub offer: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWebRtcSessionResponse {
    pub client_handle_id: Uuid,
    pub answer: String,
            }

            async function setupWebRTC(streamId) {
            async function closeActiveSession() {
                if (pc) {
                    pc.close();
                    pc = null;
                }
                if (statusPollInterval) {
                    clearInterval(statusPollInterval);
                    statusPollInterval = null;
                }
                if (currentSession.sessionId) {
                    const endpoint = `/streams/${currentSession.streamId}/webrtc/${currentSession.sessionId}`;
                    await api(endpoint, "DELETE").catch(() => {});
                }
            }

            async function setupWebRTC(
                streamId,
                createPath,
                payload,
                isLiveSession,
            ) {
                const startTime = Date.now();
                console.log(`[${startTime}] Starting WebRTC setup for stream ${streamId}`);
                console.log(
                    `[${startTime}] Starting WebRTC setup for stream ${streamId}`,
                );

                try {
                    if (pc) pc.close();
                    pc.ontrack = (e) => {
                        const trackTime = Date.now();
                        console.log(`[${trackTime}] Video track received, elapsed: ${trackTime - startTime}ms`);
                        console.log(
                            `[${trackTime}] Video track received, elapsed: ${trackTime - startTime}ms`,
                        );
                        video.srcObject = e.streams[0];
                        document
                            .getElementById("video-overlay")
                        video.onloadeddata = () => {
                            const loadedTime = Date.now();
                            console.log(`[${loadedTime}] Video data loaded and playing, elapsed: ${loadedTime - startTime}ms`);
                            console.log(
                                `[${loadedTime}] Video data loaded and playing, elapsed: ${loadedTime - startTime}ms`,
                            );
                        };

                        video.oncanplay = () => {
                            const canPlayTime = Date.now();
                            console.log(`[${canPlayTime}] Video can play, elapsed: ${canPlayTime - startTime}ms`);
                            console.log(
                                `[${canPlayTime}] Video can play, elapsed: ${canPlayTime - startTime}ms`,
                            );
                        };

                        video.onplay = () => {
                            const playTime = Date.now();
                            console.log(`[${playTime}] Video play event fired, elapsed: ${playTime - startTime}ms`);
                            console.log(
                                `[${playTime}] Video play event fired, elapsed: ${playTime - startTime}ms`,
                            );
                        };
                    };

                    pc.onconnectionstatechange = () => {
                        console.log(`WebRTC connection state changed to: ${pc.connectionState}`);
                        console.log(
                            `WebRTC connection state changed to: ${pc.connectionState}`,
                        );
                    };

                    pc.onsignalingstatechange = () => {
                        console.log(`WebRTC signaling state changed to: ${pc.signalingState}`);
                        console.log(
                            `WebRTC signaling state changed to: ${pc.signalingState}`,
                        );
                    };

                    pc.oniceconnectionstatechange = () => {
                        console.log(`WebRTC ICE connection state changed to: ${pc.iceConnectionState}`);
                        console.log(
                            `WebRTC ICE connection state changed to: ${pc.iceConnectionState}`,
                        );
                    };

                    pc.addTransceiver("video", { direction: "recvonly" });
                    console.log(`[${Date.now()}] Added video transceiver, elapsed: ${Date.now() - startTime}ms`);
                    console.log(
                        `[${Date.now()}] Added video transceiver, elapsed: ${Date.now() - startTime}ms`,
                    );

                    const offer = await pc.createOffer();
                    await pc.setLocalDescription(offer);
                    console.log(`[${Date.now()}] Created and set local offer, elapsed: ${Date.now() - startTime}ms`);
                    console.log(
                        `[${Date.now()}] Created and set local offer, elapsed: ${Date.now() - startTime}ms`,
                    );

                    const payload = { offer: offer.sdp };
                    const response = await api(
                        `/streams/${streamId}/webrtc`,
                        "POST",
                        payload,
                    const response = await api(createPath, "POST", {
                        ...payload,
                        offer: offer.sdp,
                    });
                    console.log(
                        `[${Date.now()}] Received server response, elapsed: ${Date.now() - startTime}ms`,
                    );
                    console.log(`[${Date.now()}] Received server response, elapsed: ${Date.now() - startTime}ms`);

                    await pc.setRemoteDescription(
                        new RTCSessionDescription({
                        }),
                    );
                    console.log(`[${Date.now()}] Set remote description, elapsed: ${Date.now() - startTime}ms`);
                    console.log(
                        `[${Date.now()}] Set remote description, elapsed: ${Date.now() - startTime}ms`,
                    );

                    currentSession.streamId = streamId;
                    currentSession.sessionId = response.client_handle_id;
                    currentSession.isLive = true;
                    currentSession.isLive = isLiveSession;
                    currentSession.currentSpeed = 1.0;

                    updateModeIndicator();
                    startPolling();

                    console.log(`[${Date.now()}] WebRTC setup completed, total time: ${Date.now() - startTime}ms`);
                    console.log(
                        `[${Date.now()}] WebRTC setup completed, total time: ${Date.now() - startTime}ms`,
                    );
                } catch (e) {
                    const errorTime = Date.now();
                    console.error(`[${errorTime}] WebRTC setup failed after ${errorTime - startTime}ms:`, e);
                    console.error(
                        `[${errorTime}] WebRTC setup failed after ${errorTime - startTime}ms:`,
                        e,
                    );
                    showToast("Failed to start stream", "error");
                }
            }

            async function startLiveSession(streamId) {
                await closeActiveSession();
                await setupWebRTC(
                    streamId,
                    `/streams/${streamId}/webrtc`,
                    {},
                    true,
                );
            }

            async function startDvrSession(streamId, timestampMs) {
                await closeActiveSession();
                await setupWebRTC(
                    streamId,
                    `/streams/${streamId}/webrtc/dvr`,
                    { timestamp: timestampMs },
                    false,
                );
                currentSession.currentTimeMs = timestampMs;
            }

            function startViewing(streamId) {
                document.getElementById("modal-title").innerText =
                    "Stream Viewer";
                    .classList.remove("hidden");
                document.getElementById("mode-indicator").classList.add("flex");
                setupWebRTC(streamId);
                startLiveSession(streamId);
            }

            function closePlayer() {
                if (pc) pc.close();
                if (statusPollInterval) {
                    clearInterval(statusPollInterval);
                    statusPollInterval = null;
                }
                if (currentSession.sessionId) {
                    const endpoint = `/streams/${currentSession.streamId}/webrtc/${currentSession.sessionId}`;
                    api(endpoint, "DELETE").catch(() => {});
                }
                closeActiveSession().catch(() => {});
                document.getElementById("player-modal").classList.add("hidden");
                document.getElementById("remote-video").srcObject = null;
                currentSession = {
                if (!currentSession.sessionId) return;
                try {
                    const response = await api(
                        `/streams/${currentSession.streamId}/webrtc/${currentSession.sessionId}/live`,
                        "POST",
                    );

                    currentSession.isLive = response.is_live;
                    await startLiveSession(currentSession.streamId);
                    currentSession.isLive = true;
                    currentSession.currentTimeMs = Date.now();
                    updateModeIndicator();
                    showToast("Switched to live mode");
                } catch (e) {

                try {
                    await api(
                        `/streams/${currentSession.streamId}/webrtc/${currentSession.sessionId}/seek`,
                        "POST",
                        {
                            timestamp: timestampMs,
                        },
                    );
                    if (currentSession.isLive) {
                        await startDvrSession(
                            currentSession.streamId,
                            timestampMs,
                        );
                    } else {
                        await api(
                            `/streams/${currentSession.streamId}/webrtc/${currentSession.sessionId}/seek`,
                            "POST",
                            {
                                timestamp: timestampMs,
                            },
                        );
                    }

                    // Optimistic update
                    currentSession.currentTimeMs = timestampMs;
                    const now = Date.now();
                    currentSession.isLive = now - timestampMs < 2000;
                    currentSession.isLive = false;
                    updateModeIndicator();
                    updateTimelineDisplay();
                } catch (e) {
        response: oneshot::Sender<Result<(), String>>,
    },
    StartDelayed {
        start_id: u64,
    },
    Stop,
}

}

impl Clone for DvrSessionHandle {
    fn clone(&self) -> Self {
        Self {
            command_tx: self.command_tx.clone(),
            event_rx: self.event_rx.resubscribe(),
        }
    }
}

struct DvrSessionActor {
    stream_id: u64,
    codec: VideoCodec,
    delayed_start_handle: Option<tokio::task::JoinHandle<()>>,
    time_player_started: chrono::DateTime<chrono::Utc>,
    start_id: u64,
    command_tx: mpsc::Sender<DvrCommand>,
    command_rx: mpsc::Receiver<DvrCommand>,
    event_tx: broadcast::Sender<DvrEvent>,
    eos_tx: broadcast::Sender<()>,
                            let _ = response.send(result);
                        }
                        DvrCommand::StartDelayed { start_id } => {
                            if start_id != self.start_id {
                                continue;
                            }
                            if let Err(e) = self.start_current_player() {
                                log::error!("Failed to start delayed DVR playback: {}", e);
                            }
                        }
                        DvrCommand::Stop => {
                            self.handle_stop();
                            let _ = self.event_tx.send(DvrEvent::Stopped);

    async fn handle_seek(&mut self, timestamp: u64) -> Result<u64, String> {
        if let Some(handle) = self.delayed_start_handle.take() {
            handle.abort();
        }

        match self.current_player.seek_to_timestamp(timestamp, 1.0) {
            Ok(_) => {
                self.invalidate_delayed_start();
                self.start_current_player()?;
                let _ = self.event_tx.send(DvrEvent::SeekComplete(timestamp));
                Ok(timestamp)
            }
        delay: std::time::Duration,
    ) -> Result<u64, String> {
        self.invalidate_delayed_start();

        let player = Self::create_player(
            recording,
            timestamp,

        self.current_player = player;
        self.time_player_started = Utc::now();

        let eos_tx = self.eos_tx.clone();
        let event_tx = self.event_tx.clone();

        self.delayed_start_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(delay).await;
        }));

        if let Some(handle) = self.delayed_start_handle.take() {
            let _ = handle.await;
        let start_id = self.start_id;
        if delay > std::time::Duration::from_millis(0) {
            let command_tx = self.command_tx.clone();
            self.delayed_start_handle = Some(tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let _ = command_tx.send(DvrCommand::StartDelayed { start_id }).await;
            }));
        } else {
            self.start_current_player()?;
        }

        let eos_tx_clone = eos_tx.clone();
        let _ = self.event_tx.send(DvrEvent::SeekComplete(timestamp));
        Ok(timestamp)
    }

    fn start_current_player(&mut self) -> Result<(), String> {
        let eos_tx_clone = self.eos_tx.clone();
        if let Err(e) = self.current_player.start(move || {
            let _ = eos_tx_clone.send(());
        }) {
            let err = format!("Failed to start player: {}", e);
            let _ = event_tx.send(DvrEvent::Error(err.clone()));
            let _ = self.event_tx.send(DvrEvent::Error(err.clone()));
            return Err(err);
        }
        self.time_player_started = Utc::now();
        Ok(())
    }

        let _ = self.event_tx.send(DvrEvent::SeekComplete(timestamp));
        Ok(timestamp)
    fn invalidate_delayed_start(&mut self) {
        if let Some(handle) = self.delayed_start_handle.take() {
            handle.abort();
        }
        self.start_id = self.start_id.wrapping_add(1);
    }

    fn handle_set_speed(&mut self, speed: f64) -> Result<(), String> {

    fn handle_stop(&mut self) {
        if let Some(handle) = self.delayed_start_handle.take() {
            handle.abort();
        }
        self.invalidate_delayed_start();
        if let Err(e) = self.current_player.stop() {
            log::error!("Failed to stop player: {}", e);
        }
            delayed_start_handle: None,
            time_player_started: Utc::now(),
            start_id: 0,
            command_tx: command_tx.clone(),
            command_rx,
            event_tx: event_tx.clone(),
            eos_tx,
use anyhow::{bail, Result};
use rffmpeg as ffmpeg;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
            if parts.len() == 2 {
                if let Ok(start_time) = parts[0].parse::<u64>() {
                    let end_time = if parts[1] == "latest" {
                    let mut end_time = if parts[1] == "latest" {
                        None
                    } else {
                        parts[1].parse::<u64>().ok()
                    let file_size = fs::metadata(&path)?.len();
                    if file_size > 663 {
                        if end_time.is_none() {
                            if let Some(duration_ms) = probe_mp4_duration_ms(&path) {
                                end_time = Some(start_time.saturating_add(duration_ms));
                            }
                        }
                        recordings.push(RecordingMetadata {
                            path,
                            start_time,
    for recording in recordings {
        if recording.start_time > timestamp {
            let duration = Duration::from_secs(recording.start_time - timestamp);
            let duration = Duration::from_millis(recording.start_time - timestamp);
            return Some((recording, duration));
        }
    }
        if let Some(end_time) = recording.end_time {
            if end_time < timestamp {
                let duration = Duration::from_secs(timestamp - end_time);
                let duration = Duration::from_millis(timestamp - end_time);
                return Some((recording.clone(), duration));
            }
        }
    Ok(None)
}

fn probe_mp4_duration_ms(path: &PathBuf) -> Option<u64> {
    ffmpeg::init().ok()?;
    let ictx = ffmpeg::format::input(path.to_str()?).ok()?;
    let video_stream = ictx.streams().best(ffmpeg::media::Type::Video)?;
    let time_base = video_stream.time_base();
    let duration_ts = video_stream.duration();
    if duration_ts <= 0 || time_base.denominator() == 0 {
        return None;
    }
    Some(
        ((duration_ts as u128) * 1000 * time_base.numerator() as u128
            / time_base.denominator() as u128) as u64,
    )
}


use crate::domain::dvr::filesystem::RecordingMetadata;
use crate::domain::rtp_packetizer::{parse_nal_units, RtpPacketizer};
use crate::domain::{RtpPacket, VideoCodec};
use anyhow::Result;
use anyhow::{anyhow, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

enum DvrPlaybackControl {
    Seek(u64),
    Speed(f64),
}

#[derive(Debug, Clone)]
pub struct DvrPlaybackConfig {
    pub recording_metadata: RecordingMetadata,
    packet_tx: broadcast::Sender<RtpPacket>,
    is_playing: AtomicBool,
    shutdown: AtomicBool,
    shutdown: Arc<AtomicBool>,
    control_tx: mpsc::Sender<DvrPlaybackControl>,
    control_rx: Mutex<Option<mpsc::Receiver<DvrPlaybackControl>>>,
    playback_thread: Mutex<Option<thread::JoinHandle<()>>>,
}

        packet_tx: broadcast::Sender<RtpPacket>,
    ) -> Result<Self> {
        let (control_tx, control_rx) = mpsc::channel();
        Ok(Self {
            config: Mutex::new(config),
            codec,
            packet_tx,
            is_playing: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            shutdown: Arc::new(AtomicBool::new(false)),
            control_tx,
            control_rx: Mutex::new(Some(control_rx)),
            playback_thread: Mutex::new(None),
        })
    }
        let codec = self.codec;
        let packet_tx = self.packet_tx.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);
        let shutdown_clone = Arc::clone(&self.shutdown);
        let control_rx = match self.control_rx.lock().unwrap().take() {
            Some(rx) => rx,
            None => return Ok(()),
        };

        let handle = thread::spawn(move || {
            if let Err(e) = run_dvr_playback(config, codec, packet_tx, shutdown_clone, on_eos) {
            if let Err(e) =
                run_dvr_playback(config, codec, packet_tx, shutdown_clone, control_rx, on_eos)
            {
                log::error!("DVR playback error: {}", e);
            }
        });
        }

        self.control_tx
            .send(DvrPlaybackControl::Seek(timestamp))
            .map_err(|_| SeekError::GstError(anyhow!("Playback control channel closed")))?;

        Ok(())
    }

        let mut config = self.config.lock().unwrap();
        config.speed = speed;
        self.control_tx
            .send(DvrPlaybackControl::Speed(speed))
            .map_err(|_| anyhow!("Playback control channel closed"))?;
        Ok(())
    }
}
    packet_tx: broadcast::Sender<RtpPacket>,
    shutdown: Arc<AtomicBool>,
    control_rx: mpsc::Receiver<DvrPlaybackControl>,
    mut on_eos: CB,
) -> Result<()>
where
    CB: FnMut() + Send + 'static,
{
    use rffmpeg as ffmpeg;

    ffmpeg::init()?;
    gst::init()?;

    let path_str = config.recording_metadata.path.to_string_lossy().to_string();
    log::info!("Opening DVR file: {}", path_str);

    let mut ictx = ffmpeg::format::input(&path_str)?;

    let video_stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or_else(|| anyhow::anyhow!("No video stream found in DVR file"))?;

    let video_stream_index = video_stream.index();
    let time_base = video_stream.time_base();

    let duration_ts = video_stream.duration();
    let duration_ms = if duration_ts > 0 && time_base.denominator() > 0 {
        (duration_ts * 1000 * time_base.numerator() as i64) / time_base.denominator() as i64
    } else {
        0
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("filesrc")
        .build()
        .map_err(|_| anyhow::anyhow!("Failed to create filesrc"))?;
    let demux = gst::ElementFactory::make("qtdemux")
        .build()
        .map_err(|_| anyhow::anyhow!("Failed to create qtdemux"))?;
    let queue = gst::ElementFactory::make("queue")
        .build()
        .map_err(|_| anyhow::anyhow!("Failed to create queue"))?;
    let parser = match codec {
        VideoCodec::H264 => gst::ElementFactory::make("h264parse")
            .build()
            .map_err(|_| anyhow::anyhow!("Failed to create h264parse"))?,
        VideoCodec::H265 => gst::ElementFactory::make("h265parse")
            .build()
            .map_err(|_| anyhow::anyhow!("Failed to create h265parse"))?,
    };

    log::info!(
        "DVR file duration: {} ms, time_base: {}/{}",
        duration_ms,
        time_base.numerator(),
        time_base.denominator()
    );
    let pay = match codec {
        VideoCodec::H264 => gst::ElementFactory::make("rtph264pay")
            .build()
            .map_err(|_| anyhow::anyhow!("Failed to create rtph264pay"))?,
        VideoCodec::H265 => gst::ElementFactory::make("rtph265pay")
            .build()
            .map_err(|_| anyhow::anyhow!("Failed to create rtph265pay"))?,
    };
    pay.set_property("pt", 96u32);
    pay.set_property("config-interval", 1i32);

    let appsink = gst::ElementFactory::make("appsink")
        .build()
        .map_err(|_| anyhow::anyhow!("Failed to create appsink"))?;
    let appsink = appsink
        .dynamic_cast::<gst_app::AppSink>()
        .map_err(|_| anyhow::anyhow!("Failed to cast appsink"))?;
    appsink.set_property("emit-signals", false);

    src.set_property("location", path_str);

    pipeline.add_many(&[&src, &demux, &queue, &parser, &pay, appsink.upcast_ref()])?;

    src.link(&demux)
        .map_err(|_| anyhow::anyhow!("Failed to link filesrc to qtdemux"))?;
    queue
        .link(&parser)
        .map_err(|_| anyhow::anyhow!("Failed to link queue to parser"))?;
    parser
        .link(&pay)
        .map_err(|_| anyhow::anyhow!("Failed to link parser to pay"))?;
    pay.link(&appsink)
        .map_err(|_| anyhow::anyhow!("Failed to link pay to appsink"))?;

    let queue_clone = queue.clone();
    demux.connect_pad_added(move |_demux, src_pad| {
        let sink_pad = match queue_clone.static_pad("sink") {
            Some(pad) => pad,
            None => return,
        };
        if sink_pad.is_linked() {
            return;
        }
        if let Some(caps) = src_pad.current_caps() {
            if let Some(structure) = caps.structure(0) {
                if !structure.name().starts_with("video/") {
                    return;
                }
            }
        }
        let _ = src_pad.link(&sink_pad);
    });

    let start_offset_ms = config
        .start_position
        .saturating_sub(config.recording_metadata.start_time) as i64;

    if start_offset_ms > 0 {
        let seek_ts = (start_offset_ms * time_base.denominator() as i64)
            / (1000 * time_base.numerator() as i64);
        log::info!(
            "Seeking to {} ms (ts: {}) in DVR file",
            start_offset_ms,
            seek_ts
        );
        ictx.seek(seek_ts, ..seek_ts)?;
    pipeline
        .set_state(gst::State::Paused)
        .map_err(|_| anyhow::anyhow!("Failed to set pipeline to paused"))?;

    let (state_result, current, pending) = pipeline.state(gst::ClockTime::from_seconds(5));
    state_result.map_err(|_| anyhow::anyhow!("Failed to await paused state"))?;
    log::debug!(
        "DVR pipeline state after pause: result={:?}, current={:?}, pending={:?}",
        state_result,
        current,
        pending
    );

    let mut rate = config.speed;
    if start_offset_ms > 0 || rate != 1.0 {
        let start = gst::ClockTime::from_mseconds(start_offset_ms.max(0) as u64);
        let mut seek_query = gst::query::Seeking::new(gst::Format::Time);
        let seekable = if pipeline.query(&mut seek_query) {
            let (seeks, start_q, end_q) = seek_query.result();
            log::debug!("DVR seekable? {} range: {:?}-{:?}", seeks, start_q, end_q);
            seeks
        } else {
            log::warn!("DVR seek query failed, assuming seekable");
            true
        };

        log::debug!("DVR init seek to {} ms (rate {})", start_offset_ms, rate);
        if seekable {
            pipeline
                .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, start)
                .map_err(|e| anyhow::anyhow!(format!("Failed initial seek: {}", e)))?;
        }
    }

    let ssrc: u32 = rand::random();
    let mut packetizer = RtpPacketizer::new(ssrc, 96);
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|_| anyhow::anyhow!("Failed to set pipeline to playing"))?;

    let mut last_pts: Option<i64> = None;
    let speed = config.speed;
    let bus = pipeline
        .bus()
        .ok_or_else(|| anyhow::anyhow!("Pipeline has no bus"))?;

    for (stream, packet) in ictx.packets() {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            log::info!("DVR playback shutdown requested");
            break;
        }

        if stream.index() != video_stream_index {
            continue;
        }

        let pts = packet.pts().unwrap_or(0);

        if let Some(prev_pts) = last_pts {
            let pts_diff_ms = ((pts - prev_pts) * 1000 * time_base.numerator() as i64)
                / time_base.denominator() as i64;

            if pts_diff_ms > 0 && speed > 0.0 {
                let sleep_duration = Duration::from_millis((pts_diff_ms as f64 / speed) as u64);

                let sleep_start = Instant::now();
                while sleep_start.elapsed() < sleep_duration {
                    if shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(1));
        while let Ok(cmd) = control_rx.try_recv() {
            match cmd {
                DvrPlaybackControl::Seek(timestamp) => {
                    let offset_ms = timestamp.saturating_sub(config.recording_metadata.start_time);
                    let start = gst::ClockTime::from_mseconds(offset_ms);
                    log::debug!("DVR seek to {} (offset_ms={})", timestamp, offset_ms);
                    pipeline
                        .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, start)
                        .map_err(|e| anyhow::anyhow!(format!("Seek failed: {}", e)))?;
                }
                DvrPlaybackControl::Speed(new_speed) => {
                    rate = new_speed;
                    let position = pipeline
                        .query_position::<gst::ClockTime>()
                        .unwrap_or(gst::ClockTime::from_mseconds(0));
                    log::debug!("DVR speed change to {} at {:?}", rate, position);
                    pipeline
                        .seek(
                            rate,
                            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                            gst::SeekType::Set,
                            position,
                            gst::SeekType::None,
                            gst::ClockTime::NONE,
                        )
                        .map_err(|e| anyhow::anyhow!(format!("Speed change seek failed: {}", e)))?;
                }
            }
        }

        last_pts = Some(pts);

        if let Some(data) = packet.data() {
            let rtp_timestamp = packetizer.pts_to_rtp_timestamp(
                pts,
                time_base.numerator(),
                time_base.denominator(),
            );

            let nal_units = parse_nal_units(data, codec);

            if !nal_units.is_empty() {
                let rtp_packets = packetizer.packetize(&nal_units, rtp_timestamp, codec);

                for rtp_data in rtp_packets {
                    let rtp_packet = RtpPacket::from_gstreamer(rtp_data);
        if let Some(sample) = appsink.try_pull_sample(gst::ClockTime::from_mseconds(100)) {
            if let Some(buffer) = sample.buffer() {
                if let Ok(map) = buffer.map_readable() {
                    let rtp_packet = RtpPacket::from_gstreamer(map.as_slice().to_vec());
                    if packet_tx.send(rtp_packet).is_err() {
                        log::debug!("No receivers for DVR packets");
                    }
            }
        }

        if let Some(msg) = bus.timed_pop_filtered(
            gst::ClockTime::from_mseconds(0),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        ) {
            match msg.view() {
                gst::MessageView::Eos(..) => {
                    log::info!("DVR playback reached end of stream");
                    on_eos();
                    break;
                }
                gst::MessageView::Error(err) => {
                    let src = err.src().map(|s| s.path_string()).unwrap_or_default();
                    let dbg = err.debug().unwrap_or_else(|| "".into());
                    return Err(anyhow!(
                        "GStreamer error from {}: {} ({})",
                        src,
                        err.error(),
                        dbg
                    ));
                }
                _ => {}
            }
        }
    }

    log::info!("DVR playback reached end of stream");
    on_eos();
    pipeline
        .set_state(gst::State::Null)
        .map_err(|_| anyhow::anyhow!("Failed to set pipeline to null"))?;

    Ok(())
}
use crate::domain::dvr::dvr_session::DvrSessionHandle;
use crate::domain::RtpPacket;
use crate::utils::get_current_unix_epoch;
use dashmap::DashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
const REGISTRATION_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RtpSessionMode {
    Live,
    Dvr,
}

#[derive(Clone)]
pub struct RtpRegistrationSnapshot {
    pub destination: Option<SocketAddr>,
    pub client_port: Option<u16>,
    pub last_seen: Instant,
    pub mode: RtpSessionMode,
}

pub struct RtpSessionStatus {
    pub is_live: bool,
    pub current_time_ms: u64,
    pub speed: f64,
}

pub struct RtpRegistration {
    pub destination: Option<SocketAddr>,
    pub client_port: Option<u16>,
    pub last_seen: Instant,
    pub mode: RtpSessionMode,
    pub dvr_start_timestamp: Option<u64>,
    pub dvr_started_at: Option<Instant>,
    pub speed: f64,
    pub dvr_handle: Mutex<Option<DvrSessionHandle>>,
    pub dvr_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

pub struct RtpUnicastHandle {
    server_port: u16,
    socket: Arc<UdpSocket>,
    registrations: Arc<DashMap<String, RtpRegistration>>,
    sender_task: tokio::task::JoinHandle<()>,
    receiver_task: tokio::task::JoinHandle<()>,
                                continue;
                            }
                            if reg.mode == RtpSessionMode::Dvr {
                                continue;
                            }
                            if let Some(dest) = reg.destination {
                                let send_addr = match reg.client_port {
                                    Some(port) if should_override_port(dest.ip()) => {

                        for token in stale_tokens {
                            registrations_for_sender.remove(&token);
                            if let Some((_, reg)) = registrations_for_sender.remove(&token) {
                                cleanup_registration(reg);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
        Self {
            server_port,
            socket,
            registrations,
            sender_task,
            receiver_task,
                client_port,
                last_seen: Instant::now(),
                mode: RtpSessionMode::Live,
                dvr_start_timestamp: None,
                dvr_started_at: None,
                speed: 1.0,
                dvr_handle: Mutex::new(None),
                dvr_task: Mutex::new(None),
            },
        );
    }

    pub fn get_registration(&self, token: &str) -> Option<RtpRegistration> {
        self.registrations
            .get(token)
            .map(|entry| entry.value().clone())
    pub fn get_registration(&self, token: &str) -> Option<RtpRegistrationSnapshot> {
        self.registrations.get(token).map(|entry| {
            let reg = entry.value();
            RtpRegistrationSnapshot {
                destination: reg.destination,
                client_port: reg.client_port,
                last_seen: reg.last_seen,
                mode: reg.mode,
            }
        })
    }

    pub fn set_destination(&self, token: &str, destination: SocketAddr) -> bool {
    }

    pub fn get_session_status(&self, token: &str) -> Option<RtpSessionStatus> {
        let entry = self.registrations.get(token)?;
        let reg = entry.value();
        if reg.mode == RtpSessionMode::Live {
            return Some(RtpSessionStatus {
                is_live: true,
                current_time_ms: get_current_unix_epoch(),
                speed: 1.0,
            });
        }
        Some(RtpSessionStatus {
            is_live: false,
            current_time_ms: current_dvr_timestamp(reg),
            speed: reg.speed,
        })
    }

    pub fn get_dvr_handle(&self, token: &str) -> Option<DvrSessionHandle> {
        let entry = self.registrations.get(token)?;
        let reg = entry.value();
        let lock = reg.dvr_handle.lock().ok()?;
        lock.as_ref().cloned()
    }

    pub fn update_dvr_seek(&self, token: &str, timestamp: u64) -> Result<(), String> {
        let mut entry = self
            .registrations
            .get_mut(token)
            .ok_or_else(|| "RTP token not registered".to_string())?;
        let reg = entry.value_mut();
        reg.mode = RtpSessionMode::Dvr;
        reg.dvr_start_timestamp = Some(timestamp);
        reg.dvr_started_at = Some(Instant::now());
        reg.speed = 1.0;
        reg.last_seen = Instant::now();
        Ok(())
    }

    pub fn update_dvr_speed(&self, token: &str, speed: f64) -> Result<(), String> {
        let mut entry = self
            .registrations
            .get_mut(token)
            .ok_or_else(|| "RTP token not registered".to_string())?;
        let reg = entry.value_mut();
        if reg.mode != RtpSessionMode::Dvr {
            return Err("RTP session not in DVR mode".to_string());
        }
        let current_ts = current_dvr_timestamp(reg);
        reg.dvr_start_timestamp = Some(current_ts);
        reg.dvr_started_at = Some(Instant::now());
        reg.speed = speed;
        reg.last_seen = Instant::now();
        Ok(())
    }

    pub fn start_dvr_session(
        &self,
        token: &str,
        dvr_handle: DvrSessionHandle,
        packet_rx: broadcast::Receiver<RtpPacket>,
        timestamp: u64,
    ) -> Result<(), String> {
        let sender_task = spawn_dvr_sender(
            Arc::clone(&self.socket),
            Arc::clone(&self.registrations),
            token.to_string(),
            packet_rx,
        );

        let (old_task, old_handle) = {
            let mut entry = self
                .registrations
                .get_mut(token)
                .ok_or_else(|| "RTP token not registered".to_string())?;
            let reg = entry.value_mut();

            let old_task = reg.dvr_task.lock().ok().and_then(|mut t| t.take());
            let old_handle = reg.dvr_handle.lock().ok().and_then(|mut h| h.take());

            reg.mode = RtpSessionMode::Dvr;
            reg.dvr_start_timestamp = Some(timestamp);
            reg.dvr_started_at = Some(Instant::now());
            reg.speed = 1.0;
            reg.last_seen = Instant::now();

            if let Ok(mut handle_lock) = reg.dvr_handle.lock() {
                *handle_lock = Some(dvr_handle);
            }
            if let Ok(mut task_lock) = reg.dvr_task.lock() {
                *task_lock = Some(sender_task);
            }

            (old_task, old_handle)
        };

        if let Some(task) = old_task {
            task.abort();
        }
        if let Some(handle) = old_handle {
            tokio::spawn(async move {
                let _ = handle.stop().await;
            });
        }
        Ok(())
    }

    pub fn abort(self) {
        self.sender_task.abort();
        self.receiver_task.abort();
    }
}

fn current_dvr_timestamp(reg: &RtpRegistration) -> u64 {
    let start_ts = reg.dvr_start_timestamp.unwrap_or(0);
    let started_at = reg.dvr_started_at.unwrap_or_else(Instant::now);
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    if reg.speed <= 0.0 {
        return start_ts;
    }
    let scaled = (elapsed_ms as f64 * reg.speed) as u64;
    start_ts.saturating_add(scaled)
}

fn cleanup_registration(reg: RtpRegistration) {
    if let Ok(mut task_lock) = reg.dvr_task.lock() {
        if let Some(handle) = task_lock.take() {
            handle.abort();
        }
    }
    if let Ok(mut handle_lock) = reg.dvr_handle.lock() {
        if let Some(handle) = handle_lock.take() {
            tokio::spawn(async move {
                let _ = handle.stop().await;
            });
        }
    }
}

fn spawn_dvr_sender(
    socket: Arc<UdpSocket>,
    registrations: Arc<DashMap<String, RtpRegistration>>,
    token: String,
    mut packet_rx: broadcast::Receiver<RtpPacket>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match packet_rx.recv().await {
                Ok(packet) => {
                    let now = Instant::now();
                    let (dest, client_port, last_seen, mode) = match registrations.get(&token) {
                        Some(entry) => {
                            let reg = entry.value();
                            (reg.destination, reg.client_port, reg.last_seen, reg.mode)
                        }
                        None => break,
                    };

                    if mode != RtpSessionMode::Dvr {
                        break;
                    }
                    if now.duration_since(last_seen) > REGISTRATION_TTL {
                        if let Some((_, reg)) = registrations.remove(&token) {
                            cleanup_registration(reg);
                        }
                        break;
                    }
                    if let Some(dest) = dest {
                        let send_addr = match client_port {
                            Some(port) if should_override_port(dest.ip()) => {
                                SocketAddr::new(dest.ip(), port)
                            }
                            _ => dest,
                        };
                        let _ = socket.send_to(packet.as_slice(), send_addr).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

use crate::domain::{
    dvr::{
        dvr_session::{DvrSession, DvrSessionHandle},
        filesystem,
    },
    dvr::dvr_session::{DvrSession, DvrSessionHandle},
    webrtc::publish::{WebRtcSession, WebRtcStreamer},
    RtpPacket, StreamState, VideoCodec,
};
    dvr_session: Arc<Mutex<Option<DvrSessionHandle>>>,
    codec: VideoCodec,
    mode: StreamHandleMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamHandleMode {
    Live,
    Dvr,
}

impl StreamHandle {
            dvr_session: Arc::new(Mutex::new(None)),
            codec,
            mode: StreamHandleMode::Live,
        };

        handle_self.spawn_loop(packet_rx, state_rx).await;
    }

    pub async fn new_dvr(
        client_handle_id: Uuid,
        source_id: u64,
        offer: String,
        codec: VideoCodec,
        packet_rx: broadcast::Receiver<RtpPacket>,
        state_rx: broadcast::Receiver<StreamState>,
        on_close_tx: tokio::sync::oneshot::Sender<()>,
        timestamp_ms: u64,
    ) -> Result<(Self, String)> {
        let (session, answer, _ice_rx, track) =
            WebRtcSession::new(client_handle_id, &offer, codec, on_close_tx).await?;
        let webrtc_session = Arc::new(session);

        let streamer = WebRtcStreamer::new(Arc::clone(&webrtc_session), track);
        let streamer = Arc::new(Mutex::new(Some(streamer)));

        let (dvr_packet_tx, dvr_packet_rx) = broadcast::channel(1000);
        let dvr_handle =
            DvrSession::new(source_id, codec.clone(), timestamp_ms, 1.0, dvr_packet_tx)?;

        let handle_self = Self {
            client_handle_id,
            source_id,
            webrtc_session: Arc::clone(&webrtc_session),
            streamer: Arc::clone(&streamer),
            packet_rx: packet_rx.resubscribe(),
            state_rx: state_rx.resubscribe(),
            task_handle: Arc::new(Mutex::new(None)),
            dvr_session: Arc::new(Mutex::new(Some(dvr_handle))),
            codec,
            mode: StreamHandleMode::Dvr,
        };

        handle_self.spawn_loop(dvr_packet_rx, state_rx).await;

        Ok((handle_self, answer))
    }

    async fn spawn_loop(
        &self,
        packet_rx: broadcast::Receiver<RtpPacket>,
        }

        if let Some(dvr) = self.dvr_session.lock().await.take() {
        let dvr = { self.dvr_session.lock().await.take() };
        if let Some(dvr) = dvr {
            let _ = dvr.stop().await;
        }


    pub async fn seek(&self, timestamp_ms: u64) -> Result<()> {
        let current_time = crate::utils::get_current_unix_epoch();

        if timestamp_ms > current_time - 2000 {
            return self.switch_to_live().await;
        }

        if filesystem::find_recording_for_timestamp(self.source_id, timestamp_ms).is_err() {
            bail!("No recording available for timestamp {}", timestamp_ms);
        if self.mode == StreamHandleMode::Live {
            bail!("Live sessions cannot seek; create a DVR session");
        }

        log::info!(
        );

        let mut dvr_lock = self.dvr_session.lock().await;
        let existing_dvr = { self.dvr_session.lock().await.as_ref().cloned() };

        if let Some(ref dvr) = *dvr_lock {
        if let Some(dvr) = existing_dvr {
            match dvr.seek(timestamp_ms).await {
                Ok(_) => return Ok(()),
                Err(e) => {
        }

        if let Some(old_dvr) = dvr_lock.take() {
        let old_dvr = { self.dvr_session.lock().await.take() };
        if let Some(old_dvr) = old_dvr {
            let _ = old_dvr.stop().await;
        }

        )?;

        *dvr_lock = Some(dvr_handle);
        drop(dvr_lock);
        {
            let mut dvr_lock = self.dvr_session.lock().await;
            *dvr_lock = Some(dvr_handle);
        }

        self.spawn_loop(packet_rx, self.state_rx.resubscribe())
            .await;

    pub async fn set_speed(&self, speed: f64) -> Result<()> {
        let dvr_lock = self.dvr_session.lock().await;
        if let Some(ref dvr) = *dvr_lock {
        if self.mode == StreamHandleMode::Live {
            bail!("Speed control only available in DVR mode");
        }

        let dvr = { self.dvr_session.lock().await.as_ref().cloned() };
        if let Some(dvr) = dvr {
            dvr.set_speed(speed).await.map_err(|e| anyhow::anyhow!(e))?;
            Ok(())
        } else {

    pub async fn switch_to_live(&self) -> Result<()> {
        if self.mode == StreamHandleMode::Dvr {
            bail!("DVR sessions cannot switch to live; create a new live session");
        }

        log::info!("Switching session {} to LIVE", self.client_handle_id);

        {
            let mut dvr_lock = self.dvr_session.lock().await;
            if let Some(dvr) = dvr_lock.take() {
                let _ = dvr.stop().await;
            }
        let dvr = { self.dvr_session.lock().await.take() };
        if let Some(dvr) = dvr {
            let _ = dvr.stop().await;
        }

        self.spawn_loop(self.packet_rx.resubscribe(), self.state_rx.resubscribe())
    pub async fn get_mode(&self) -> Result<crate::api::models::SessionModeResponse> {
        let dvr_lock = self.dvr_session.lock().await;
        let is_live = dvr_lock.is_none();
        let is_live = self.mode == StreamHandleMode::Live && dvr_lock.is_none();
        let current_time = if is_live {
            crate::utils::get_current_unix_epoch()
        } else {
use crate::domain::{
    dvr::{filesystem, recorder::RecordingFileManager},
    rtp::{run_live_packetizer, unicast::RtpUnicastHandle, CodecParameters},
    dvr::{dvr_session::DvrSession, filesystem, recorder::RecordingFileManager},
    rtp::{
        run_live_packetizer,
        unicast::{RtpSessionStatus, RtpUnicastHandle},
        CodecParameters,
    },
    rtsp::RtspClient,
    stream_handle::StreamHandle,
    RtpPacket, StreamConfig, StreamInfo, StreamState, VideoCodec, WebRtcSessionInfo,
    }

    pub async fn create_dvr_webrtc_session(
        &self,
        source_id: u64,
        client_handle_id: Uuid,
        offer: String,
        timestamp_ms: u64,
    ) -> Result<String> {
        let source = self
            .sources
            .get(&source_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;

        let (on_close_tx, on_close_rx) = tokio::sync::oneshot::channel();

        let codec = loop {
            if let Some(detected_codec) = source.client.get_codec().await {
                break detected_codec;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        };

        let (handle, answer) = StreamHandle::new_dvr(
            client_handle_id,
            source_id,
            offer,
            codec,
            source.packet_tx.subscribe(),
            source.state_tx.subscribe(),
            on_close_tx,
            timestamp_ms,
        )
        .await?;

        self.stream_handles
            .insert(client_handle_id, Arc::new(handle));

        let stream_handles_clone = Arc::clone(&self.stream_handles);
        tokio::spawn(async move {
            let _ = on_close_rx.await;
            if let Some((_, h)) = stream_handles_clone.remove(&client_handle_id) {
                log::info!("Cleaning up closed WebRTC session {}", client_handle_id);
                if let Err(e) = h.close().await {
                    log::warn!("Error closing session {}: {}", client_handle_id, e);
                }
            }
        });

        log::info!(
            "DVR WebRTC session {} created for stream {}",
            client_handle_id,
            source_id
        );
        Ok(answer)
    }

    pub async fn remove_webrtc_session(
        &self,
        source_id: u64,
    pub async fn get_webrtc_sessions(&self, source_id: u64) -> Result<Vec<WebRtcSessionInfo>> {
        let mut sessions = Vec::new();
        for entry in self.stream_handles.iter() {
            if entry.value().source_id() == source_id {
                sessions.push(WebRtcSessionInfo {
                    client_handle_id: *entry.key(),
                    source_id,
                    state: entry.value().state().await,
                });
            }
        let handles: Vec<(Uuid, Arc<StreamHandle>)> = self
            .stream_handles
            .iter()
            .filter(|entry| entry.value().source_id() == source_id)
            .map(|entry| (*entry.key(), Arc::clone(entry.value())))
            .collect();
        for (client_handle_id, handle) in handles {
            sessions.push(WebRtcSessionInfo {
                client_handle_id,
                source_id,
                state: handle.state().await,
            });
        }
        Ok(sessions)
    }
    }

    pub async fn seek_rtp_session(
        &self,
        source_id: u64,
        token: &str,
        timestamp_ms: u64,
    ) -> Result<()> {
        let (client, existing_dvr) = {
            let source = self
                .sources
                .get(&source_id)
                .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
            let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
                anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
            })?;

            if unicast.get_registration(token).is_none() {
                bail!("RTP token not registered");
            }

            (Arc::clone(&source.client), unicast.get_dvr_handle(token))
        };

        if let Some(dvr_handle) = existing_dvr {
            dvr_handle
                .seek(timestamp_ms)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            let source = self
                .sources
                .get(&source_id)
                .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
            let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
                anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
            })?;
            unicast
                .update_dvr_seek(token, timestamp_ms)
                .map_err(|e| anyhow::anyhow!(e))?;
            return Ok(());
        }

        let codec = loop {
            if let Some(detected_codec) = client.get_codec().await {
                break detected_codec;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        };

        let (packet_tx, packet_rx) = broadcast::channel(1000);
        let dvr_handle = DvrSession::new(source_id, codec, timestamp_ms, 1.0, packet_tx)?;
        let source = self
            .sources
            .get(&source_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
        let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
            anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
        })?;
        unicast
            .start_dvr_session(token, dvr_handle, packet_rx, timestamp_ms)
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    pub async fn set_rtp_session_speed(
        &self,
        source_id: u64,
        token: &str,
        speed: f64,
    ) -> Result<()> {
        let dvr_handle = {
            let source = self
                .sources
                .get(&source_id)
                .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
            let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
                anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
            })?;

            if unicast.get_registration(token).is_none() {
                bail!("RTP token not registered");
            }

            unicast
                .get_dvr_handle(token)
                .ok_or_else(|| anyhow::anyhow!("RTP session not in DVR mode"))?
        };
        dvr_handle
            .set_speed(speed)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        let source = self
            .sources
            .get(&source_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
        let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
            anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
        })?;
        unicast
            .update_dvr_speed(token, speed)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub async fn get_rtp_session_mode(
        &self,
        source_id: u64,
        token: &str,
    ) -> Result<crate::api::models::SessionModeResponse> {
        let source = self
            .sources
            .get(&source_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
        let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
            anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
        })?;

        if unicast.get_registration(token).is_none() {
            bail!("RTP token not registered");
        }

        let status = unicast
            .get_session_status(token)
            .ok_or_else(|| anyhow::anyhow!("RTP token not registered"))?;

        Ok(map_rtp_status(status))
    }

    pub async fn seek_session(
        &self,
        source_id: u64,
        timestamp_ms: u64,
    ) -> Result<()> {
        let handle = self
            .stream_handles
            .get(&client_handle_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        let handle = {
            let entry = self
                .stream_handles
                .get(&client_handle_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
            Arc::clone(entry.value())
        };

        if handle.source_id() != source_id {
            bail!(
        speed: f64,
    ) -> Result<()> {
        let handle = self
            .stream_handles
            .get(&client_handle_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        let handle = {
            let entry = self
                .stream_handles
                .get(&client_handle_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
            Arc::clone(entry.value())
        };

        if handle.source_id() != source_id {
            bail!(
        client_handle_id: Uuid,
    ) -> Result<()> {
        let handle = self
            .stream_handles
            .get(&client_handle_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        let handle = {
            let entry = self
                .stream_handles
                .get(&client_handle_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
            Arc::clone(entry.value())
        };

        if handle.source_id() != source_id {
            bail!(
        client_handle_id: Uuid,
    ) -> Result<crate::api::models::SessionModeResponse> {
        let handle = self
            .stream_handles
            .get(&client_handle_id)
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        let handle = {
            let entry = self
                .stream_handles
                .get(&client_handle_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
            Arc::clone(entry.value())
        };

        if handle.source_id() != source_id {
            bail!(
    }
}

fn map_rtp_status(status: RtpSessionStatus) -> crate::api::models::SessionModeResponse {
    crate::api::models::SessionModeResponse {
        is_live: status.is_live,
        current_time_ms: status.current_time_ms,
        speed: status.speed,
    }
}

    log::info!("  POST   /streams/:source_id/rtp/refresh");
    log::info!("  GET    /streams/:source_id/rtp/sdp");
    log::info!("  POST   /streams/:source_id/rtp/:token/seek");
    log::info!("  POST   /streams/:source_id/rtp/:token/speed");
    log::info!("  GET    /streams/:source_id/rtp/:token/status");
    log::info!("  POST   /streams/:source_id/webrtc");
    log::info!("  GET    /streams/:source_id/webrtc");
    log::info!("  DELETE /streams/:source_id/webrtc/:client_handle_id");
# TRTP Protocol Seeking Implementation Guide for Flutter Player

## Overview

This document provides a comprehensive guide for implementing seeking functionality in a Flutter player library that uses the TRTP (T5 RTP) protocol. The TRTP protocol is a custom RTP streaming protocol that enables NAT traversal through UDP hole punching and provides raw RTP streams with SDP negotiation.

## Current TRTP Protocol Architecture

### Server-Side Components

The Rust media server implements TRTP with the following key components:

1. **RTP Unicast Handle** (`domain/rtp/unicast.rs`): Manages RTP packet delivery to registered clients
2. **Registration System**: Uses tokens to track client sessions and destinations
3. **UDP Hole Punching**: Uses `t5rtp <token> <client_port>` format for NAT traversal
4. **SDP Generation**: Creates session descriptions for RTP playback
5. **Session Refresh**: Maintains active sessions with TTL-based expiration

### Current TRTP Flow

1. **Register**: `POST /streams/{source_id}/rtp` with JSON `{ "client_port": <u16 or null> }`
2. **Holepunch (UDP)**: Send datagram to server port with format `t5rtp <token> <client_port>`
3. **Get SDP**: `GET /streams/{source_id}/rtp/sdp?token=<token>`
4. **Refresh**: `POST /streams/{source_id}/rtp/refresh` with JSON `{ "token": "<token>" }`
5. **Receive**: Raw RTP packets (AVP, payload type 96) sent to registered destination

## Seeking Architecture Design

### Missing Components for TRTP Seeking

Currently, the TRTP protocol does not support seeking. To implement seeking functionality, we need to add:

1. **New API Endpoints**: `POST /streams/{source_id}/rtp/{token}/seek`, `POST /streams/{source_id}/rtp/{token}/speed`, `GET /streams/{source_id}/rtp/{token}/status`
2. **Per-Token DVR Session**: Each RTP token can switch between live and DVR playback independently
3. **Flutter Player Updates**: Add seeking controls and status polling for DVR mode

### Proposed TRTP Seeking Flow

1. **Initial Registration**: Same as current flow
2. **Seek Request**: Send HTTP POST to `/streams/{source_id}/rtp/{token}/seek` with timestamp
3. **Server Processing**: Server creates/switches a DVR session for that token and stops live RTP for that token
4. **RTP Continuation**: Server sends DVR RTP packets to the token destination
5. **Refresh**: Keep the token alive via `/streams/{source_id}/rtp/refresh`
6. **Go Live**: Reconnect (new token) to resume live RTP, or seek to current time if recordings are continuous

## Server-Side Implementation

### New API Endpoint

```rust
#[utoipa::path(
    post,
    path = "/streams/{source_id}/rtp/{token}/seek",
    params(
        ("source_id" = u64, Path, description = "Stream unique identifier"),
        ("token" = String, Path, description = "RTP registration token")
    ),
    request_body = SeekRequest,
    responses(
        (status = 200, description = "RTP session seeked successfully", body = SessionModeResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Stream or token not found", body = ErrorResponse)
    ),
    tag = "RTP"
)]
pub async fn seek_rtp_session(
    State(state): State<Arc<AppState>>,
    Path((source_id, token)): Path<(u64, String)>,
    Json(req): Json<SeekRequest>,
) -> Result<Json<SessionModeResponse>, ApiError> {
    state
        .stream_manager
        .seek_rtp_session(source_id, &token, req.timestamp)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Stream not found") || msg.contains("token not registered") {
                ApiError::NotFound(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    let mode = state
        .stream_manager
        .get_rtp_session_mode(source_id, &token)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(Json(mode))
}
```

Additional RTP DVR endpoints to support speed control and status polling:

```
POST /streams/{source_id}/rtp/{token}/speed
GET  /streams/{source_id}/rtp/{token}/status
```

### Stream Manager Extension

```rust
impl GlobalState {
    pub async fn seek_rtp_session(&self, source_id: u64, token: &str, timestamp: u64) -> Result<()> {
        let source = self
            .sources
            .get(&source_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
        let unicast = source.rtp_unicast.as_ref().ok_or_else(|| {
            anyhow::anyhow!("RTP sender not initialized for stream {}", source_id)
        })?;

        if unicast.get_registration(token).is_none() {
            bail!("RTP token not registered");
        }

        if let Some(dvr_handle) = unicast.get_dvr_handle(token) {
            dvr_handle.seek(timestamp).await.map_err(|e| anyhow::anyhow!(e))?;
            unicast.update_dvr_seek(token, timestamp)?;
            return Ok(());
        }

        let codec = loop {
            if let Some(detected_codec) = source.client.get_codec().await {
                break detected_codec;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        };

        let (packet_tx, packet_rx) = broadcast::channel(1000);
        let dvr_handle = DvrSession::new(source_id, codec, timestamp, 1.0, packet_tx)?;
        unicast.start_dvr_session(token, dvr_handle, packet_rx, timestamp)?;

        Ok(())
    }
}
```

### Extended RtpRegistration Structure

```rust
pub struct RtpRegistration {
    pub destination: Option<SocketAddr>,
    pub client_port: Option<u16>,
    pub last_seen: Instant,
    pub mode: RtpSessionMode,
    pub dvr_start_timestamp: Option<u64>,
    pub dvr_started_at: Option<Instant>,
    pub speed: f64,
    pub dvr_handle: Mutex<Option<DvrSessionHandle>>,
    pub dvr_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}
```

## Flutter Player Implementation

### 1. Enhanced TRTP Client Class

```dart
class TrtpClient {
  final String serverUrl;
  final int sourceId;
  String? token;
  int? serverPort;
  Timer? _refreshTimer;
  bool _isConnected = false;
  
  // DVR-specific properties
  bool _isDvrMode = false;
  int _currentTimestamp = 0;
  double _playbackSpeed = 1.0;

  TrtpClient(this.serverUrl, this.sourceId);

  Future<void> connect({int? clientPort, int? seekTimestamp}) async {
    try {
      // Register RTP session
      final response = await http.post(
        Uri.parse('$serverUrl/streams/$sourceId/rtp'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'client_port': clientPort}),
      );
      
      if (response.statusCode == 201) {
        final data = jsonDecode(response.body);
        token = data['token'];
        serverPort = data['server_port'];
        
        // Perform UDP hole punch if required
        if (data['udp_holepunch_required'] && clientPort != null) {
          await _performUdpHolePunch(clientPort);
        }
        
        // If seeking to a specific timestamp, switch this token into DVR mode
        if (seekTimestamp != null) {
          await seekToTimestamp(seekTimestamp);
        }
        
        _isConnected = true;
        
        // Start refresh timer
        _startRefreshTimer(data['refresh_interval_secs']);
      }
    } catch (e) {
      throw Exception('Failed to connect to TRTP server: $e');
    }
  }

  Future<void> _performUdpHolePunch(int clientPort) async {
    if (token == null || serverPort == null) return;
    
    try {
      final socket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, clientPort);
      final serverAddress = Uri.parse(serverUrl).host;
      final message = 't5rtp $token $clientPort';
      
      socket.send(utf8.encode(message), InternetAddress(serverAddress), serverPort!);
      await Future.delayed(Duration(milliseconds: 100));
      socket.close();
    } catch (e) {
      throw Exception('UDP hole punch failed: $e');
    }
  }

  void _startRefreshTimer(int intervalSecs) {
    _refreshTimer?.cancel();
    _refreshTimer = Timer.periodic(Duration(seconds: intervalSecs), (timer) async {
      if (_isConnected && token != null) {
        await _refreshSession();
      }
    });
  }

  Future<void> _refreshSession() async {
    if (token == null) return;
    
    try {
      await http.post(
        Uri.parse('$serverUrl/streams/$sourceId/rtp/refresh'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'token': token}),
      );
    } catch (e) {
      print('Failed to refresh RTP session: $e');
    }
  }

  // SEEKING METHODS
  Future<void> seekToTimestamp(int timestamp) async {
    if (token == null) throw Exception('Not connected to TRTP server');
    
    try {
      final response = await http.post(
        Uri.parse('$serverUrl/streams/$sourceId/rtp/$token/seek'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'timestamp': timestamp}),
      );
      
      if (response.statusCode == 200) {
        _currentTimestamp = timestamp;
        _isDvrMode = true;
      } else {
        throw Exception('Seek request failed: ${response.body}');
      }
    } catch (e) {
      throw Exception('Seek failed: $e');
    }
  }

  Future<void> seekToDateTime(DateTime dateTime) async {
    await seekToTimestamp(dateTime.millisecondsSinceEpoch);
  }

  Future<void> skipTime(int seconds) async {
    final newTimestamp = _currentTimestamp + (seconds * 1000);
    await seekToTimestamp(newTimestamp);
  }

  Future<void> setSpeed(double speed) async {
    if (token == null) throw Exception('Not connected to TRTP server');
    
    try {
      final response = await http.post(
        Uri.parse('$serverUrl/streams/$sourceId/rtp/$token/speed'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'speed': speed}),
      );
      
      if (response.statusCode == 200) {
        _playbackSpeed = speed;
      } else {
        throw Exception('Speed change failed: ${response.body}');
      }
    } catch (e) {
      throw Exception('Speed change failed: $e');
    }
  }

  Future<Map<String, dynamic>> getSessionStatus() async {
    if (token == null) throw Exception('Not connected to TRTP server');
    
    try {
      final response = await http.get(
        Uri.parse('$serverUrl/streams/$sourceId/rtp/$token/status'),
      );
      
      if (response.statusCode == 200) {
        return jsonDecode(response.body);
      } else {
        throw Exception('Status request failed: ${response.body}');
      }
    } catch (e) {
      throw Exception('Get status failed: $e');
    }
  }

  void disconnect() {
    _refreshTimer?.cancel();
    _isConnected = false;
    token = null;
  }
}
```

### 2. Flutter Player Widget with Seeking Controls

```dart
class TrtpPlayerWidget extends StatefulWidget {
  final String serverUrl;
  final int sourceId;
  final int? initialTimestamp; // For DVR mode

  const TrtpPlayerWidget({
    Key? key,
    required this.serverUrl,
    required this.sourceId,
    this.initialTimestamp,
  }) : super(key: key);

  @override
  _TrtpPlayerWidgetState createState() => _TrtpPlayerWidgetState();
}

class _TrtpPlayerWidgetState extends State<TrtpPlayerWidget> {
  late TrtpClient _trtpClient;
  final VideoController _videoController = VideoController();
  bool _isLoading = true;
  bool _isLive = true;
  int _currentTimeMs = 0;
  double _playbackSpeed = 1.0;
  Timer? _statusPollTimer;

  @override
  void initState() {
    super.initState();
    _trtpClient = TrtpClient(widget.serverUrl, widget.sourceId);
    _initializePlayer();
  }

  Future<void> _initializePlayer() async {
    try {
      // Connect to TRTP server
      await _trtpClient.connect(
        clientPort: 5004, // Default RTP port
        seekTimestamp: widget.initialTimestamp,
      );

      // Get SDP and configure video player
      final sdpUrl = '${widget.serverUrl}/streams/${widget.sourceId}/rtp/sdp?token=${_trtpClient.token}';
      final sdpResponse = await http.get(Uri.parse(sdpUrl));
      
      if (sdpResponse.statusCode == 200) {
        final sdp = sdpResponse.body;
        // Configure your video player with the SDP
        // This depends on your specific video player implementation
        await _configureVideoPlayer(sdp);
        
        setState(() {
          _isLoading = false;
          _isLive = widget.initialTimestamp == null;
          _currentTimeMs = widget.initialTimestamp ?? DateTime.now().millisecondsSinceEpoch;
        });
        
        // Start polling for status updates
        _startStatusPolling();
      }
    } catch (e) {
      print('Failed to initialize player: $e');
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _configureVideoPlayer(String sdp) async {
    // Implementation depends on your video player library
    // For example, if using a WebRTC-based player:
    // 1. Create a WebRTC peer connection
    // 2. Set the SDP as remote description
    // 3. Handle RTP packets
  }

  void _startStatusPolling() {
    _statusPollTimer = Timer.periodic(Duration(seconds: 1), (timer) async {
      try {
        final status = await _trtpClient.getSessionStatus();
        setState(() {
          _isLive = status['is_live'] ?? _isLive;
          _currentTimeMs = status['current_time_ms'] ?? _currentTimeMs;
          _playbackSpeed = status['speed'] ?? _playbackSpeed;
        });
      } catch (e) {
        // Status polling failed, but don't interrupt playback
        print('Status polling failed: $e');
      }
    });
  }

  Future<void> _goLive() async {
    try {
      // Reconnect without a specific timestamp to go live
      _trtpClient.disconnect();
      await _trtpClient.connect(clientPort: 5004);
      
      setState(() {
        _isLive = true;
        _currentTimeMs = DateTime.now().millisecondsSinceEpoch;
      });
    } catch (e) {
      print('Failed to go live: $e');
    }
  }

  Future<void> _handleSeek(DateTime dateTime) async {
    try {
      await _trtpClient.seekToDateTime(dateTime);
      setState(() {
        _isLive = false;
        _currentTimeMs = dateTime.millisecondsSinceEpoch;
      });
    } catch (e) {
      print('Seek failed: $e');
    }
  }

  Future<void> _skipTime(int seconds) async {
    try {
      await _trtpClient.skipTime(seconds);
      setState(() {
        _currentTimeMs += seconds * 1000;
        _isLive = false;
      });
    } catch (e) {
      print('Skip failed: $e');
    }
  }

  Future<void> _setSpeed(double speed) async {
    try {
      await _trtpClient.setSpeed(speed);
      setState(() {
        _playbackSpeed = speed;
      });
    } catch (e) {
      print('Speed change failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // Video display area
        Container(
          height: 300,
          width: double.infinity,
          color: Colors.black,
          child: _isLoading
              ? Center(child: CircularProgressIndicator())
              : _buildVideoDisplay(),
        ),
        
        // Playback controls
        _buildPlaybackControls(),
      ],
    );
  }

  Widget _buildVideoDisplay() {
    // Implementation depends on your video player library
    return Center(
      child: Text(
        _isLive ? 'LIVE' : 'DVR: ${DateTime.fromMillisecondsSinceEpoch(_currentTimeMs).toString()}',
        style: TextStyle(color: Colors.white),
      ),
    );
  }

  Widget _buildPlaybackControls() {
    return Padding(
      padding: EdgeInsets.all(16.0),
      child: Column(
        children: [
          // Current time and live indicator
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                'Current: ${DateTime.fromMillisecondsSinceEpoch(_currentTimeMs).toString()}',
                style: TextStyle(fontSize: 12),
              ),
              if (!_isLive)
                ElevatedButton(
                  onPressed: _goLive,
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Container(
                        width: 8,
                        height: 8,
                        decoration: BoxDecoration(
                          color: Colors.red,
                          shape: BoxShape.circle,
                        ),
                      ),
                      SizedBox(width: 4),
                      Text('LIVE'),
                    ],
                  ),
                ),
            ],
          ),
          
          SizedBox(height: 16),
          
          // Timeline slider
          Slider(
            value: 0.0, // This would be calculated based on current time vs total duration
            onChanged: (value) {
              // Handle timeline dragging for seeking
            },
          ),
          
          // Seek controls
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              ElevatedButton(
                onPressed: () => _skipTime(-60),
                child: Text('-1m'),
              ),
              ElevatedButton(
                onPressed: () => _skipTime(-10),
                child: Text('-10s'),
              ),
              ElevatedButton(
                onPressed: () => _skipTime(10),
                child: Text('+10s'),
              ),
              ElevatedButton(
                onPressed: () => _skipTime(60),
                child: Text('+1m'),
              ),
            ],
          ),
          
          SizedBox(height: 16),
          
          // Date/time seek
          Row(
            children: [
              Expanded(
                child: TextField(
                  onTap: () async {
                    final date = await showDatePicker(
                      context: context,
                      initialDate: DateTime.fromMillisecondsSinceEpoch(_currentTimeMs),
                      firstDate: DateTime.now().subtract(Duration(days: 30)),
                      lastDate: DateTime.now(),
                    );
                    if (date != null) {
                      final time = await showTimePicker(
                        context: context,
                        initialTime: TimeOfDay.fromDateTime(
                          DateTime.fromMillisecondsSinceEpoch(_currentTimeMs),
                        ),
                      );
                      if (time != null) {
                        final dateTime = DateTime(
                          date.year,
                          date.month,
                          date.day,
                          time.hour,
                          time.minute,
                        );
                        _handleSeek(dateTime);
                      }
                    }
                  },
                  decoration: InputDecoration(
                    labelText: 'Seek to Time',
                    hintText: DateTime.fromMillisecondsSinceEpoch(_currentTimeMs).toString(),
                  ),
                ),
              ),
              SizedBox(width: 8),
              ElevatedButton(
                onPressed: () async {
                  final now = DateTime.now();
                  _handleSeek(now);
                },
                child: Text('Now'),
              ),
            ],
          ),
          
          SizedBox(height: 16),
          
          // Speed controls
          Wrap(
            spacing: 8,
            children: [0.5, 1.0, 1.5, 2.0, 4.0].map((speed) {
              return FilterChip(
                label: Text('${speed}x'),
                selected: _playbackSpeed == speed,
                onSelected: (selected) {
                  if (selected) {
                    _setSpeed(speed);
                  }
                },
              );
            }).toList(),
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    _statusPollTimer?.cancel();
    _trtpClient.disconnect();
    super.dispose();
  }
}
```

### 3. Usage Example

```dart
class MyApp extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: Text('TRTP Player with Seeking')),
        body: Padding(
          padding: EdgeInsets.all(16.0),
          child: Column(
            children: [
              // Live stream player
              Container(
                height: 300,
                child: TrtpPlayerWidget(
                  serverUrl: 'http://192.168.1.100:8009',
                  sourceId: 1,
                ),
              ),
              
              SizedBox(height: 16),
              
              // DVR player starting at specific time
              Container(
                height: 300,
                child: TrtpPlayerWidget(
                  serverUrl: 'http://192.168.1.100:8009',
                  sourceId: 1,
                  initialTimestamp: DateTime.now().subtract(Duration(hours: 1)).millisecondsSinceEpoch,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
```

## Implementation Considerations

### 1. Server-Side Changes Required

The Rust media server needs the following modifications:

1. **New API endpoints** for seeking and speed control in TRTP sessions
2. **DVR session management** for TRTP clients
3. **State tracking** to maintain playback position for each TRTP session
4. **Integration** between RTP unicast and DVR functionality

### 2. Client-Side Considerations

1. **Buffering strategy** for seeking operations
2. **Error handling** for network issues during seeking
3. **UI feedback** during seek operations
4. **Synchronization** between client state and server state

### 3. Performance Considerations

1. **Seek latency**: The time between requesting a seek and receiving the first frame
2. **Memory usage**: For buffering and caching during playback
3. **Network efficiency**: Minimize unnecessary data transfer during seeking

### 4. Error Handling

1. **Invalid timestamps**: Handle seeks to times outside the recording range
2. **Network timeouts**: Handle cases where seek requests don't complete
3. **Server unavailability**: Graceful degradation when server is unreachable
4. **Format compatibility**: Ensure RTP packets are compatible with the player

## Testing Strategy

### 1. Unit Tests

- Test seeking functionality with various timestamps
- Test speed control changes
- Test session refresh during seeking
- Test error conditions (invalid tokens, timestamps, etc.)

### 2. Integration Tests

- End-to-end seeking from Flutter client to Rust server
- Verify RTP packet delivery after seeking
- Test concurrent seeking by multiple clients
- Validate SDP generation for different seek positions

### 3. Performance Tests

- Measure seek latency under various network conditions
- Test memory usage during extended playback sessions
- Verify stability during rapid seeking operations

## Security Considerations

1. **Token validation**: Ensure only valid tokens can be used for seeking
2. **Rate limiting**: Prevent abuse of seeking endpoints
3. **Access control**: Verify client permissions before allowing seeks
4. **Input validation**: Validate timestamp ranges to prevent errors

This implementation provides a complete framework for adding seeking functionality to the TRTP protocol in a Flutter player library, with proper integration between the client and server components.
