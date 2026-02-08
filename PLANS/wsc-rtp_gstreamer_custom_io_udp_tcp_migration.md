
pub mod wsc_rtp {
    /// Header for hole-punch packets sent by the client (UDP or TCP)
    /// Header for UDP hole-punch packets sent by the client
    pub const HOLEPUNCH_HEADER: &str = "ws-rtp";

    /// Header for dummy packets sent by the server to confirm UDP connectivity
    pub const DUMMY_PACKET_INTERVAL_MS: u64 = 100;

    /// Port range start for UDP/TCP holepunch listeners
    /// Port range start for UDP holepunch listeners
    pub const PORT_RANGE_START: u16 = 30000;

    /// Port range end for UDP/TCP holepunch listeners
    /// Port range end for UDP holepunch listeners
    pub const PORT_RANGE_END: u16 = 40000;
}

        message: String,
    },
    /// Sent when UDP transport fails and RTP packets will now be delivered as binary WebSocket frames.
    FallingBackRtpToWs,
    Pong,
}

rand = "0.8"

gstreamer = "0.22"
gstreamer-app = "0.22"
gstreamer-video = "0.22"
gstreamer = "0.24.4"
gstreamer-app = "0.24.4"
gstreamer-video = "0.24.4"

webrtc = "0.9"


WSC-RTP uses a hybrid approach:
- **WebSocket**: For signaling, session management, and keep-alive
- **UDP or TCP**: For RTP packet delivery (video data) — UDP is preferred for lower latency; TCP is used as automatic fallback when UDP is blocked
- **WebSocket**: For signaling, session management, keep-alive, and RTP fallback transport
- **UDP**: For RTP packet delivery (preferred — lowest latency)
- **REST API**: For playback control (seek, go-to-live, speed control)

When UDP is not available (e.g. blocked by firewall or NAT), RTP packets are automatically delivered as binary WebSocket frames instead.

## Architecture

```
│                         Media Server                            │
│                                                                 │
│  ┌─────────────────┐    ┌──────────────────────────────────┐   │
│  │  WebSocket      │    │  Holepunch Port (30000-40000)    │   │
│  │  Endpoint       │    │  ┌───────────┐  ┌─────────────┐ │   │
│  │  /streams/{id}/ │    │  │ UDP       │  │ TCP         │ │   │
│  │  wsc-rtp        │    │  │ Listener  │  │ Listener    │ │   │
│  └────────┬────────┘    │  └───────────┘  └─────────────┘ │   │
│           │             └──────────────────────────────────┘   │
│           │  ┌───────────────────────────────────┘             │
│           │  │                                                  │
│           ▼  ▼                                                  │
│  ┌─────────────────────────────────────┐                       │
│  │         Session Manager             │                       │
│  │  - SDP generation                   │                       │
│  │  - RTP packetization               │                       │
│  │  - DVR/Live switching              │                       │
│  └─────────────────────────────────────┘                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  WebSocket Endpoint: /streams/{id}/wsc-rtp              │   │
│  │  - Signaling (JSON text frames)                         │   │
│  │  - RTP fallback (binary frames, when UDP unavailable)   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────┐                          │
│  │  UDP Holepunch Port (30000-40000) │                          │
│  └──────────────────────────────────┘                          │
│                                                                 │
│  ┌─────────────────┐                                           │
│  │  REST API       │                                           │
└─────────────────────────────────────────────────────────────────┘
         │                      │
         │ WebSocket            │ UDP or TCP (RTP)
         │ (signaling)          │ (video)
         │ WebSocket            │ UDP (preferred)
         │ (signaling +         │ (video)
         │  RTP fallback)       │
         ▼                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Client                                  │
│  ┌─────────────────┐    ┌─────────────────┐                    │
│  │  WebSocket      │    │  UDP or TCP     │                    │
│  │  Handler        │    │  (RTP receiver) │                    │
│  └─────────────────┘    └─────────────────┘                    │
│           │                      │                              │
│           ▼                      ▼                              │
│  ┌─────────────────────────────────────┐                       │
│  │         RTP Decoder (FFmpeg)        │                       │
│  │  - Parse SDP                        │                       │
│  │  - Decode H.264/H.265               │                       │
│  └─────────────────────────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

```

**Parameters:**
- `source_id`: The numeric ID of the stream to subscribe to

### 2. Receive Init Message

Upon connection, the server sends an `init` message:
**Fields:**
- `token`: UUID session identifier (used for all subsequent operations)
- `holepunch_port`: Port allocated for this session — both UDP and TCP listeners are active on this port
- `holepunch_port`: UDP port allocated for this session's hole-punch listener

### 3. Transport Negotiation

The server listens on `holepunch_port` for both UDP and TCP simultaneously. The client should attempt UDP first for lowest latency, then fall back to TCP if UDP packets are not received.
The client should attempt UDP first. If UDP is unavailable, RTP packets will arrive as binary WebSocket frames automatically.

#### UDP Path (preferred)
#### UDP Path (preferred — lowest latency)

**Step 1:** Send a UDP hole-punch packet to `{server_host}:{holepunch_port}`:
**Step 1:** Send a UDP packet to `{server_host}:{holepunch_port}`:

```
ws-rtp {TOKEN}
```

Example: `ws-rtp 550e8400-e29b-41d4-a716-446655440000`

**Step 2:** Server responds with dummy packets (UDP) to confirm connectivity:
**Step 2:** Server responds with dummy UDP packets to confirm connectivity:

```
ws-rtp-dummy {TOKEN}
```

**Step 3:** Client sends an ack to confirm it received the dummy packets:
**Step 3:** Client sends an ack over UDP to confirm receipt:

```
ws-rtp-ack {TOKEN}
```

**Step 4:** Server confirms UDP and begins streaming RTP packets over UDP.

#### TCP Fallback
**Step 4:** Server confirms UDP and streams RTP packets over UDP.

If the client does not receive any `ws-rtp-dummy` packets after a reasonable timeout (~2 seconds), it should fall back to TCP:
#### WebSocket Fallback (automatic)

**Step 1:** Connect a TCP socket to `{server_host}:{holepunch_port}`

**Step 2:** Send the hole-punch over TCP:

```
ws-rtp {TOKEN}
```

**Step 3:** Server confirms TCP transport and begins streaming RTP packets over TCP (length-prefixed frames).
If the server does not receive a UDP ack within 5 seconds of the hole-punch, it automatically switches to sending RTP packets as **binary WebSocket frames**. The client will start receiving binary frames on the existing WebSocket connection — no extra action needed.

### 4. Receive SDP

After transport is established, the server sends the SDP offer over WebSocket:
After transport is established, the server sends the SDP over WebSocket:

```json
{
```

**SDP Contents:**
- Session description with connection info
- Media description for video
- Codec information (H.264 or H.265)
- FMTP parameters (SPS/PPS for H.264, VPS/SPS/PPS for H.265)

**Example SDP:**
```
v=0
o=- 123456 0 IN IP4 0.0.0.0
s=media-server
c=IN IP4 0.0.0.0
t=0 0
m=video 5004 RTP/AVP 96
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1;sprop-parameter-sets=Z0IAKeKQFAe2AtwEBAaQeJEV,aM48gA==
a=sendonly
```

**Important:** The SDP may be sent multiple times if codec parameters change. Always use the latest SDP.

### 5. Start Receiving RTP

Once you have the SDP, configure your decoder and start receiving RTP packets.

#### UDP RTP Packets

Raw RTP packets are sent over UDP — no additional framing.
The SDP may be sent multiple times if codec parameters change. Always use the latest SDP.

#### TCP RTP Packets
### 5. Receiving RTP

Each RTP packet is prefixed with a 2-byte big-endian length:
#### UDP

```
+------------------+----------------------------------+
| Length (2 bytes) | RTP Packet (Length bytes)        |
| Big-endian uint16|                                  |
+------------------+----------------------------------+
```

#### RTP Packet Structure
Raw RTP bytes — no additional framing.

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|V=2|P|X|  CC   |M|     PT      |       Sequence Number         |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Timestamp                           |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                             SSRC                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         RTP Payload                           |
|                             ...                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```
#### WebSocket Fallback

**Fields:**
- `V`: Version (always 2)
- `P`: Padding (0)
- `X`: Extension (0)
- `CC`: CSRC count (0)
- `M`: Marker bit (1 = end of frame)
- `PT`: Payload type (96 for dynamic)
- `Sequence Number`: 16-bit, monotonically increasing
- `Timestamp`: 90kHz clock (video standard)
- `SSRC`: Synchronization source identifier
RTP packets arrive as binary WebSocket frames. Each frame is exactly one RTP packet.

### 6. Keep-Alive (Ping/Pong)

Send periodic ping messages to keep the session alive:

**Client sends:**
```json
{"type": "ping"}
```

**Server responds:**
```json
{"type": "pong"}
{"type": "ping"}   // Client → Server, every 2-3 seconds
{"type": "pong"}   // Server → Client
```

**Timeout:** The server will close the connection after 5 seconds without a ping.

**Recommended interval:** Send ping every 2-3 seconds.
Server closes connection after 5 seconds without a ping.

### 7. Stream State Notifications

The server sends stream state updates:

```json
{
  "type": "stream_state",
```

**Possible states:**
- `Active`: Stream is receiving video
- `Inactive`: Stream is not receiving video
- `Connecting`: Stream is connecting to source
- `Error`: Stream encountered an error
States: `Active`, `Inactive`, `Connecting`, `Error`

## Holepunch Message Reference

| Message | Sender | Transport | Format |
|---------|--------|-----------|--------|
| Hole-punch | Client→Server | UDP or TCP | `ws-rtp {token}` |
| Hole-punch | Client→Server | UDP | `ws-rtp {token}` |
| Dummy | Server→Client | UDP | `ws-rtp-dummy {token}` |
| Ack | Client→Server | UDP | `ws-rtp-ack {token}` |

## Playback Control (REST API)

All playback control is done via REST API endpoints. The session token from the `init` message is used as the `session_id`.
All playback control is done via REST API. The session token from `init` is used as `session_id`.

### Base URL
```
```

### Get Current Mode

**Request:**
```http
GET /streams/{source_id}/wsc-rtp/{session_id}/mode
```

**Response:**
```json
{
  "is_live": true,
  "current_time_ms": null,
  "speed": 1.0
}
```

Or for DVR mode:
```json
{
  "is_live": false,
  "current_time_ms": 1706369234567,
  "speed": 1.0
}
```

### Seek to Timestamp (DVR)

Seek to a specific timestamp in recorded video.

**Request:**
```http
POST /streams/{source_id}/wsc-rtp/{session_id}/seek
Content-Type: application/json

{
  "timestamp": 1706369234567
}
```

**Parameters:**
- `timestamp`: Unix timestamp in milliseconds

**Response:**
```json
{
  "is_live": false,
  "current_time_ms": 1706369234567,
  "speed": 1.0
}
```

**Notes:**
- Automatically switches from live to DVR mode if needed
- Seeks to the nearest keyframe
- RTP sequence numbers remain continuous (stitching)

### Switch to Live

Return to live streaming from DVR mode.

**Request:**
```http
POST /streams/{source_id}/wsc-rtp/{session_id}/live
```

**Response:**
```json
{
  "is_live": true,
  "current_time_ms": null,
  "speed": 1.0
}
```

**Notes:**
- Stops DVR playback
- Resumes live RTP stream
- RTP sequence numbers remain continuous

### Set Playback Speed (DVR only)

**Request:**
```http
POST /streams/{source_id}/wsc-rtp/{session_id}/speed
Content-Type: application/json

{
  "speed": 2.0
}
```

**Parameters:**
- `speed`: Playback speed multiplier (e.g., 0.5, 1.0, 2.0, 4.0)

**Response:**
```json
{
  "is_live": false,
  "current_time_ms": 1706369234567,
  "speed": 2.0
}
```

**Notes:**
- Only works in DVR mode
- Returns error if currently in live mode
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/mode` | Get current playback mode |
| POST | `/seek` | Seek to timestamp (ms) |
| POST | `/live` | Switch to live mode |
| POST | `/speed` | Set playback speed |

## WebSocket Message Reference

### Client → Server Messages
### Client → Server

| Message Type | Description |
|--------------|-------------|
| Type | Description |
|------|-------------|
| `ping` | Keep-alive heartbeat |

**Example:**
```json
{"type": "ping"}
```

### Server → Client Messages
### Server → Client

| Message Type | Description |
|--------------|-------------|
| `init` | Session initialization with token and holepunch port |
| Type | Description |
|------|-------------|
| `init` | Session init with token and holepunch port |
| `sdp` | SDP offer with codec parameters |
| `stream_state` | Stream status update |
| `pong` | Response to ping |
| `error` | Error message |

**Init Message:**
```json
{
  "type": "init",
  "token": "550e8400-e29b-41d4-a716-446655440000",
  "holepunch_port": 35000
}
```

**SDP Message:**
```json
{
  "type": "sdp",
  "sdp": "v=0\r\n..."
}
```

**Stream State Message:**
```json
{
  "type": "stream_state",
  "state": "Active"
}
```

**Pong Message:**
```json
{"type": "pong"}
```

**Error Message:**
```json
{
  "type": "error",
  "message": "Session not found"
}
```

## RTP Payload Formats

### H.264 (RFC 6184)

**Payload Type:** 96 (dynamic)

**NAL Unit Types in Payload:**
- Single NAL unit: NAL header + data
- FU-A fragmentation for large NALs

**FU-A Header (2 bytes):**
```
+---------------+
|0|1|2|3|4|5|6|7|
+-+-+-+-+-+-+-+-+
|F|NRI|  Type   | FU indicator (Type=28 for FU-A)
+---------------+
|S|E|R|  Type   | FU header
+---------------+
```

### H.265 (RFC 7798)

**Payload Type:** 96 (dynamic)
### H.264 (RFC 6184) — Payload Type 96
- Single NAL unit or FU-A fragmentation for large NALs

**NAL Unit Types in Payload:**
- Single NAL unit: 2-byte NAL header + data
- FU fragmentation for large NALs

**FU Header (3 bytes):**
```
+---------------+---------------+
|0|1|2|3|4|5|6|7|0|1|2|3|4|5|6|7|
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|F|  Type=49 |  LayerId  | TID | FU indicator
+---------------+---------------+
|S|E|    FuType     |           | FU header
+---------------+---------------+
```
### H.265 (RFC 7798) — Payload Type 96
- Single NAL unit or FU fragmentation for large NALs

## Implementation Notes

### Sequence Continuity

The server uses RTP stitching to maintain sequence number continuity when:
- Switching between live and DVR modes
- Seeking within DVR
- Recovering from stream interruptions

Clients should handle sequence gaps gracefully but can rely on mostly continuous sequences.

### Timestamp Handling

- RTP timestamps use 90kHz clock (standard for video)
- Timestamps are adjusted during mode switches for seamless playback
- Client should use timestamps for frame ordering, not wall-clock time

### Error Handling

1. **WebSocket disconnection**: Re-establish connection and get new session
2. **UDP packet loss**: Decoder handles this (P-frame corruption until next I-frame)
3. **SDP changes**: Re-configure decoder with new parameters
4. **REST API errors**: Check response body for error message

### Recommended Client Implementation

1. **Initialization:**
   - Connect WebSocket
   - Parse `init` message to get `token` and `holepunch_port`
   - Create UDP socket and send hole-punch
   - Wait for `ws-rtp-dummy` packet (~2 second timeout)
   - If received: send `ws-rtp-ack`, use UDP for RTP
   - If not received: connect TCP to same `holepunch_port`, send hole-punch over TCP
- RTP timestamps use 90kHz clock
- Sequence numbers are continuous across Live/DVR transitions (server-side stitching)
- SDP changes should trigger decoder reconfiguration

2. **Playback:**
   - Wait for SDP over WebSocket
   - Parse SDP for codec parameters
   - Configure FFmpeg/decoder
   - Start receive loop (UDP or TCP)
   - Feed RTP packets to decoder

3. **Keep-alive:**
   - Send ping every 2-3 seconds
   - Reconnect if no pong received

4. **Controls:**
   - Use REST API for seek/live/speed
   - Handle SDP updates after mode changes

## Example Client Flow (Pseudocode)
## Example Client Flow

```dart
// 1. Connect WebSocket
ws = WebSocket.connect("ws://server:8080/streams/123/wsc-rtp");

// 2. Handle init message
initMsg = await ws.receive();
token = initMsg.token;
holepunchPort = initMsg.holepunch_port;

// 3. Try UDP first
udpSocket = UdpSocket.bind("0.0.0.0", 0);
udpSocket.send("ws-rtp $token", serverAddress, holepunchPort);

// 4. Wait for dummy packet (UDP confirmation)
receivedDummy = await waitForPacket(udpSocket, "ws-rtp-dummy $token", timeout: 2s);

RtpTransport transport;
if (receivedDummy) {
  // UDP works
  udpSocket.send("ws-rtp-ack $token", serverAddress, holepunchPort);
  transport = UdpTransport(udpSocket);
} else {
  // Fall back to TCP
  udpSocket.close();
  tcpSocket = await TcpSocket.connect(serverAddress, holepunchPort);
  tcpSocket.write("ws-rtp $token");
  transport = TcpTransport(tcpSocket);  // reads length-prefixed frames
}

// 5. Wait for SDP
sdpMsg = await ws.receive();
decoder.configure(sdpMsg.sdp);

// 6. Start receive loop
while (true) {
  select {
    case rtpPacket = transport.receive():
      decoder.feedRtpPacket(rtpPacket);
    
    case wsMsg = ws.receive():
      if (wsMsg.type == "sdp") {
        decoder.reconfigure(wsMsg.sdp);
      }
    
    case <-pingTimer:
      ws.send({"type": "ping"});
final ws = WebSocket.connect("ws://server:8080/streams/123/wsc-rtp");

// 2. Receive init
final init = await receiveJson(ws);
final token = init['token'];
final holepunchPort = init['holepunch_port'];

// 3. Try UDP
final udp = await RawDatagramSocket.bind(InternetAddress.anyIPv4, 0);
udp.send(utf8.encode('ws-rtp $token'), serverAddress, holepunchPort);

// 4. Wait for SDP (arrives after transport is established)
//    Meanwhile: if UDP dummy packets arrive, send ack
//    If no UDP activity, RTP will come over WebSocket binary frames

// 5. Receive loop
ws.listen((frame) {
  if (frame is String) {
    final msg = jsonDecode(frame);
    if (msg['type'] == 'sdp') decoder.configure(msg['sdp']);
  } else if (frame is List<int>) {
    // WebSocket fallback RTP
    decoder.feedRtp(frame);
  }
}

// 7. Seek example
http.post("/streams/123/wsc-rtp/$token/seek",
  body: {"timestamp": 1706369234567});

// 8. Go to live
http.post("/streams/123/wsc-rtp/$token/live");
```

## FFmpeg Integration Notes

For Flutter clients using FFmpeg (rffmpeg):

1. **SDP File:** Write the SDP to a temporary file or use a data URI
2. **FFmpeg Input:** Use `-protocol_whitelist file,udp,rtp -i {sdp_path}` for UDP
3. **RTP Demuxer:** FFmpeg's RTP demuxer handles depacketization
4. **Codec Detection:** FFmpeg reads codec from SDP `a=rtpmap` and `a=fmtp` lines

**Example FFmpeg command equivalent:**
```bash
ffmpeg -protocol_whitelist file,udp,rtp \
       -i session.sdp \
       -c:v copy \
       -f rawvideo -
});

udp.listen((event) {
  final datagram = udp.receive();
  if (datagram.data starts with 'ws-rtp-dummy $token') {
    udp.send(utf8.encode('ws-rtp-ack $token'), serverAddress, holepunchPort);
  } else {
    // UDP RTP packet
    decoder.feedRtp(datagram.data);
  }
});
```

## Appendix: Full REST API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/streams/{source_id}/wsc-rtp/{session_id}/mode` | Get current playback mode |
| POST | `/streams/{source_id}/wsc-rtp/{session_id}/seek` | Seek to timestamp |
| POST | `/streams/{source_id}/wsc-rtp/{session_id}/live` | Switch to live mode |
| POST | `/streams/{source_id}/wsc-rtp/{session_id}/speed` | Set playback speed |

use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::net::UdpSocket;
use tokio::sync::oneshot;
use uuid::Uuid;

use sdp::description::session::{Origin, SessionDescription, TimeDescription, Timing};

pub type WsSender = SplitSink<WebSocket, Message>;

pub fn build_unicast_sdp(
    source_id: &VideoSourceId,
    codec: &VideoCodec,
    session.marshal().replace("\r\n", "\n")
}

#[derive(Clone)]
enum Transport {
    Udp(Arc<UdpSocket>),
    Tcp(Arc<TcpStream>),
    WebSocket(tokio::sync::mpsc::UnboundedSender<Vec<u8>>),
}

pub struct WscRtpPublisher {
    id: ClientSessionId,
    }

    /// Allocate a port in the range 30000-40000 and bind both UDP and TCP listeners on it.
    /// Returns the sockets to be passed directly to `run_holepunch_listeners`.
    pub async fn bind_holepunch_port(&self) -> anyhow::Result<(u16, UdpSocket, TcpListener)> {
    /// Allocate a port in the range 30000-40000 and bind a UDP listener on it.
    pub async fn bind_holepunch_port(&self) -> anyhow::Result<(u16, UdpSocket)> {
        for port in proto::PORT_RANGE_START..=proto::PORT_RANGE_END {
            let addr = format!("0.0.0.0:{}", port);
            let udp = UdpSocket::bind(&addr).await;
            let tcp = TcpListener::bind(&addr).await;
            match (udp, tcp) {
                (Ok(udp_sock), Ok(tcp_listener)) => {
                    log::debug!("Session {} allocated port {} (UDP+TCP)", self.id, port);
                    {
                        *self.holepunch_port.lock() = Some(port);
                    }
                    return Ok((port, udp_sock, tcp_listener));
            match UdpSocket::bind(format!("0.0.0.0:{}", port)).await {
                Ok(udp_sock) => {
                    log::debug!("Session {} allocated UDP port {}", self.id, port);
                    *self.holepunch_port.lock() = Some(port);
                    return Ok((port, udp_sock));
                }
                _ => continue,
                Err(_) => continue,
            }
        }

    }

    async fn initiate_transport(
        &self,
        udp_socket: UdpSocket,
        tcp_listener: TcpListener,
    ) -> anyhow::Result<Transport> {
    /// Wait for a UDP hole-punch, confirm with dummy/ack, and return a connected UDP socket.
    /// Returns `None` if no ack received (caller should fall back to WebSocket transport).
    async fn negotiate_udp(&self, udp_socket: UdpSocket) -> anyhow::Result<Option<Arc<UdpSocket>>> {
        let mut buf = [0u8; 1024];

        loop {
            tokio::select! {
                res = udp_socket.recv_from(&mut buf) => {
                    let (len, src) = match res {
                        Ok(v) => v,
                        Err(err) => anyhow::bail!("UDP recv error: {}", err),
                    };

                    let payload = match std::str::from_utf8(&buf[..len]) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    log::info!("Session {}: UDP holepunch from {}: {:?}", self.id, src, payload);

                    if !is_valid_holepunch(&self.id, payload) {
                        continue;
                    }
            let (len, src) = udp_socket.recv_from(&mut buf).await?;

            let payload = match std::str::from_utf8(&buf[..len]) {
                Ok(s) => s,
                Err(_) => continue,
            };

            log::info!(
                "Session {}: UDP holepunch from {}: {:?}",
                self.id,
                src,
                payload
            );

                    let dest_sock = match UdpSocket::bind("0.0.0.0:0").await {
                        Ok(s) => s,
                        Err(err) => {
                            log::error!("Session {}: failed to bind UDP dest socket: {}", self.id, err);
                            continue;
                        }
                    };
                    if let Err(err) = dest_sock.connect(src).await {
                        log::error!("Session {}: failed to connect UDP socket to {}: {}", self.id, src, err);
                        continue;
                    }
            if !is_valid_holepunch(&self.id, payload) {
                continue;
            }

                    if send_dummy_and_wait_ack(&self.id, &dest_sock, &udp_socket).await {
                        log::info!("Session {}: UDP transport confirmed", self.id);
                        return Ok(Transport::Udp(Arc::new(dest_sock)));
                    } else {
                        log::info!("Session {}: UDP ack not received, waiting for TCP fallback", self.id);
                    }
            let dest_sock = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(err) => {
                    log::error!(
                        "Session {}: failed to bind UDP dest socket: {}",
                        self.id,
                        err
                    );
                    continue;
                }
            };
            if let Err(err) = dest_sock.connect(src).await {
                log::error!(
                    "Session {}: failed to connect UDP socket to {}: {}",
                    self.id,
                    src,
                    err
                );
                continue;
            }

                res = tcp_listener.accept() => {
                    let (stream, src) = match res {
                        Ok(v) => v,
                        Err(err) => anyhow::bail!("TCP accept error: {}", err),
                    };

                    log::info!("Session {}: TCP connection from {}", self.id, src);

                    let mut tcp_buf = [0u8; 1024];
                    let len = match stream.readable().await {
                        Ok(()) => match stream.try_read(&mut tcp_buf) {
                            Ok(n) => n,
                            Err(err) => {
                                log::warn!("Session {}: TCP read error: {}", self.id, err);
                                continue;
                            }
                        },
                        Err(err) => {
                            log::warn!("Session {}: TCP readable error: {}", self.id, err);
                            continue;
                        }
                    };

                    let payload = match std::str::from_utf8(&tcp_buf[..len]) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };

                    log::info!("Session {}: TCP holepunch from {}: {:?}", self.id, src, payload);

                    if !is_valid_holepunch(&self.id, payload) {
                        continue;
                    }

                    log::info!("Session {}: TCP transport confirmed", self.id);
                    return Ok(Transport::Tcp(Arc::new(stream)));
                }
            if send_dummy_and_wait_ack(&self.id, &dest_sock, &udp_socket).await {
                log::info!("Session {}: UDP transport confirmed", self.id);
                return Ok(Some(Arc::new(dest_sock)));
            } else {
                log::info!(
                    "Session {}: no UDP ack received, falling back to WebSocket",
                    self.id
                );
                return Ok(None);
            }
        }
    }
            match guard.as_ref() {
                Some(Transport::Udp(sock)) => sock.peer_addr().ok()?,
                Some(Transport::Tcp(stream)) => stream.peer_addr().ok()?,
                // WebSocket transport has no meaningful RTP destination address for SDP;
                // use a placeholder so SDP is still generated.
                Some(Transport::WebSocket(_)) => "0.0.0.0:0".parse().ok()?,
                None => return None,
            }
        };
    let mut parts = payload.split_whitespace();
    matches!(
        (parts.next(), parts.next().and_then(|s| Uuid::parse_str(s).ok())),
        (
            parts.next(),
            parts.next().and_then(|s| Uuid::parse_str(s).ok())
        ),
        (Some(h), Some(ref id)) if h == proto::HOLEPUNCH_HEADER && id == session_id
    )
}
        }

        let transport = {
            let guard = self.transport.lock();
            match guard.as_ref() {
                Some(Transport::Udp(sock)) => Some(Transport::Udp(sock.clone())),
                Some(Transport::Tcp(stream)) => Some(Transport::Tcp(stream.clone())),
                Some(t) => t.clone(),
                None => return,
            }
        };

        let bytes = packet.to_bytes();

        match transport {
            Some(Transport::Udp(sock)) => {
            Transport::Udp(sock) => {
                log::trace!(
                    "Session {}: sending RTP via UDP to {:?}",
                    self.id,
                    sock.peer_addr()
                );
                if let Err(err) = sock.send(&bytes).await {
                if let Err(err) = sock.send(&packet.to_bytes()).await {
                    log::warn!("Session {}: UDP send error: {}", self.id, err);
                }
            }
            Some(Transport::Tcp(stream)) => {
                log::trace!(
                    "Session {}: sending RTP via TCP to {:?}",
                    self.id,
                    stream.peer_addr()
                );
                let len = (bytes.len() as u16).to_be_bytes();
                // Safety: we need write access; use try_write to avoid blocking
                // Since TcpStream write requires &mut self, we use a workaround via
                // the underlying fd or by wrapping in a Mutex. Here we use a BufWriter approach.
                // Actually we need to send atomically: length prefix + data.
                // Use write_all via a temporary owned slice.
                let mut framed = Vec::with_capacity(2 + bytes.len());
                framed.extend_from_slice(&len);
                framed.extend_from_slice(&bytes);
                // TcpStream::try_write may send partial; use writable + write loop
                let mut written = 0;
                loop {
                    match stream.try_write(&framed[written..]) {
                        Ok(n) => {
                            written += n;
                            if written == framed.len() {
                                break;
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            if stream.writable().await.is_err() {
                                log::warn!("Session {}: TCP stream no longer writable", self.id);
                                break;
                            }
                        }
                        Err(err) => {
                            log::warn!("Session {}: TCP send error: {}", self.id, err);
                            break;
                        }
                    }
            Transport::WebSocket(tx) => {
                log::trace!("Session {}: sending RTP via WebSocket", self.id);
                if tx.send(packet.to_bytes()).is_err() {
                    log::warn!("Session {}: WebSocket RTP channel closed", self.id);
                }
            }
            None => {}
        }
    }
}
}

// WebSocket handling
pub type WsSender = SplitSink<WebSocket, Message>;

async fn handle_wsc_rtp_message(
    session_id: &ClientSessionId,
    sender: &mut WsSender,
    let source_id = publisher.source_id().clone();

    let (mut sender, mut receiver) = ws.split();

    // Allocate port and bind listeners
    let (holepunch_port, udp_socket, tcp_listener) = match publisher.bind_holepunch_port().await {
        Ok(v) => v,
        Err(err) => {
            log::error!(
                "Session {}: failed to allocate holepunch port: {}",
                session_id,
                err
            );
            let _ = send_wsc_rtp_message(
                &mut sender,
                WscRtpServerMessage::Error {
                    message: format!("Failed to allocate port: {}", err),
                },
            )
            .await;
            publisher.shutdown();
            return;
        }
    };

    let last_ping = Arc::new(AtomicU64::new(get_current_unix_timestamp()));
    let (ping_timeout_tx, mut ping_timeout_rx) = tokio::sync::oneshot::channel();

    ));

    // Try to allocate a UDP port. On failure, fall straight through to WebSocket transport.
    let udp_port_result = publisher.bind_holepunch_port().await;
    if let Err(ref err) = udp_port_result {
        log::warn!(
            "Session {}: failed to allocate holepunch port ({}), will use WebSocket transport",
            session_id,
            err
        );
    }

    let holepunch_port = udp_port_result.as_ref().map(|(p, _)| *p).unwrap_or(0);

    if send_wsc_rtp_message(
        &mut sender,
        WscRtpServerMessage::Init {
    }

    // Wait for transport negotiation (UDP preferred, TCP fallback), 5-second timeout
    let transport = tokio::select! {
        res = tokio::time::timeout(
            Duration::from_secs(5),
            publisher.initiate_transport(udp_socket, tcp_listener),
        ) => {
            match res {
                Ok(Ok(t)) => t,
                Ok(Err(err)) => {
                    log::error!("Session {}: transport negotiation failed: {}", session_id, err);
                    let _ = send_wsc_rtp_message(
                        &mut sender,
                        WscRtpServerMessage::Error { message: err.to_string() },
                    ).await;
                    publisher.shutdown();
                    return;
                }
                Err(_) => {
                    log::warn!("Session {}: transport negotiation timed out", session_id);
                    let _ = send_wsc_rtp_message(
                        &mut sender,
                        WscRtpServerMessage::Error { message: "Transport negotiation timed out".into() },
                    ).await;
                    publisher.shutdown();
                    return;
    // Attempt UDP transport negotiation with 5-second timeout.
    // Falls back to WebSocket binary frames if UDP port allocation failed,
    // no hole-punch received, or no ack received.
    let udp_result = if let Ok((_, udp_socket)) = udp_port_result {
        tokio::select! {
            res = tokio::time::timeout(
                Duration::from_secs(5),
                publisher.negotiate_udp(udp_socket),
            ) => res.ok().and_then(|r| r.ok()).flatten(),
            msg = receiver.next() => {
                if let Some(Err(err)) = msg {
                    log::info!("Session {}: websocket disconnected during negotiation: {}", session_id, err);
                }
                publisher.shutdown();
                return;
            }
        }
        msg = receiver.next() => {
            if let Some(Err(err)) = msg {
                log::info!("Session {}: websocket disconnected during negotiation: {}", session_id, err);
    } else {
        None
    };

    if let Some(udp_sock) = udp_result {
        *publisher.transport.lock() = Some(Transport::Udp(udp_sock));
        log::info!(
            "Session {}: started on port {} via UDP",
            session_id,
            holepunch_port
        );
    } else {
        // Fall back to WebSocket binary frames for RTP.
        let (ws_rtp_tx, mut ws_rtp_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        *publisher.transport.lock() = Some(Transport::WebSocket(ws_rtp_tx));
        log::info!(
            "Session {}: falling back to WebSocket RTP transport",
            session_id
        );
        let _ = send_wsc_rtp_message(&mut sender, WscRtpServerMessage::FallingBackRtpToWs).await;

        // Drive the WebSocket RTP channel inside the main loop below by merging it in.
        // We use a select arm to forward binary RTP frames.
        let mut last_sdp = None;
        loop {
            tokio::select! {
                _ = &mut ping_timeout_rx => {
                    log::warn!("Session {}: closed due to ping timeout", session_id);
                    break;
                }
                Some(rtp_bytes) = ws_rtp_rx.recv() => {
                    if sender.send(Message::Binary(rtp_bytes.into())).await.is_err() {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(200)) => {
                    if let Some(sdp) = get_sdp() {
                        let new_sdp = Some(sdp.clone());
                        if last_sdp != new_sdp {
                            if send_wsc_rtp_message(&mut sender, WscRtpServerMessage::Sdp { sdp }).await.is_ok() {
                                last_sdp = new_sdp;
                            }
                        }
                    }
                }
                res = receiver.next() => {
                    let msg = match res {
                        Some(Ok(msg)) => msg,
                        Some(Err(err)) => {
                            log::error!("Session {}: error receiving WebSocket message: {}", session_id, err);
                            break;
                        }
                        None => break,
                    };
                    let data: Vec<u8> = match msg {
                        Message::Text(text) => text.into_bytes(),
                        Message::Binary(data) => data.to_vec(),
                        Message::Ping(payload) => {
                            last_ping.store(get_current_unix_timestamp(), Ordering::Relaxed);
                            if sender.send(Message::Pong(payload)).await.is_err() { break; }
                            continue;
                        }
                        Message::Pong(_) => {
                            last_ping.store(get_current_unix_timestamp(), Ordering::Relaxed);
                            continue;
                        }
                        Message::Close(_) => break,
                    };
                    if let Err(err) = handle_wsc_rtp_message(&session_id, &mut sender, &data, &last_ping).await {
                        log::error!("Session {}: error handling message: {}", session_id, err);
                        break;
                    }
                }
            }
            publisher.shutdown();
            return;
        }
    };
    *publisher.transport.lock() = Some(transport);

    log::info!(
        "Session {}: started for stream {} on port {} (UDP+TCP)",
        session_id,
        source_id,
        holepunch_port
    );
        log::info!("Session {}: closed for stream {}", session_id, source_id);
        publisher.shutdown();
        return;
    }

    // UDP path: normal message loop
    let mut last_sdp = None;

    loop {
        tokio::select! {
            _ = &mut ping_timeout_rx => {
                        }
                        continue;
                    },
                    }
                    Message::Pong(_) => {
                        last_ping.store(get_current_unix_timestamp(), Ordering::Relaxed);
                        continue;
                    },
                    }
                    Message::Close(_) => break,
                };

# Flutter Realtime Player — WSC-RTP Migration Guide

This document describes the changes to the WSC-RTP protocol and what needs to be updated in the Flutter/FFmpeg-based player.

## Summary of Changes

| Area | Old | New |
|------|-----|-----|
| Hole-punch header | `t5rtp` | `ws-rtp` |
| Init field | `server_port` | `holepunch_port` |
| UDP confirmation | None (fire-and-forget) | Dummy packet + ack handshake |
| TCP fallback | Not supported | Automatic: RTP arrives as WebSocket binary frames |

---

## 1. Rename Hole-Punch Header

**Old:**
```dart
final holepunchPayload = "t5rtp $token $clientPort";
```

**New:**
```dart
final holepunchPayload = "ws-rtp $token";
```

The client port field has been removed — the server uses the sender's address from the UDP packet directly.

---

## 2. Read `holepunch_port` from Init Message

**Old:**
```dart
final serverPort = initMsg['server_port'] as int;
```

**New:**
```dart
final holepunchPort = initMsg['holepunch_port'] as int;
```

---

## 3. Add UDP Confirmation (Dummy + Ack)

After sending the UDP hole-punch, listen for `ws-rtp-dummy` packets and respond with `ws-rtp-ack`. If no dummy arrives, UDP is blocked — RTP will come over WebSocket binary frames automatically.

```dart
static const _udpConfirmTimeout = Duration(seconds: 2);

Future<bool> _confirmUdp(
  RawDatagramSocket socket,
  InternetAddress serverAddress,
  int holepunchPort,
  String token,
) async {
  final completer = Completer<bool>();
  final timer = Timer(_udpConfirmTimeout, () {
    if (!completer.isCompleted) completer.complete(false);
  });

  socket.listen((event) {
    if (event == RawSocketEvent.read) {
      final datagram = socket.receive();
      if (datagram != null) {
        final data = utf8.decode(datagram.data).trim();
        if (data == 'ws-rtp-dummy $token') {
          // Send ack
          socket.send(
            utf8.encode('ws-rtp-ack $token'),
            serverAddress,
            holepunchPort,
          );
          timer.cancel();
          if (!completer.isCompleted) completer.complete(true);
        }
      }
    }
  });

  return completer.future;
}
```

---

## 4. Handle WebSocket Binary Frame Fallback

When UDP is blocked, the server sends a `falling_back_rtp_to_ws` JSON message to signal the mode switch, then delivers RTP packets as binary WebSocket frames.

```dart
channel.stream.listen((message) {
  if (message is String) {
    // JSON signaling message
    final msg = jsonDecode(message);
    switch (msg['type']) {
      case 'sdp':
        _configureFfmpeg(msg['sdp'] as String);
        break;
      case 'stream_state':
        _onStreamState(msg['state'] as String);
        break;
      case 'falling_back_rtp_to_ws':
        // Server could not establish UDP — RTP will now arrive
        // as binary frames on this WebSocket connection.
        _onFallingBackToWs();
        break;
      case 'pong':
        // keep-alive response
        break;
    }
  } else if (message is List<int>) {
    // WebSocket fallback: raw RTP packet
    _onRtpPacket(Uint8List.fromList(message));
  }
});
```

---

## 5. Full Connection Flow

```dart
Future<void> connect(String wsUrl, InternetAddress serverAddress) async {
  // 1. Connect WebSocket
  final channel = WebSocketChannel.connect(Uri.parse(wsUrl));

  // 2. Receive init
  final initJson = await channel.stream.first;
  final init = jsonDecode(initJson as String);
  final token = init['token'] as String;
  final holepunchPort = init['holepunch_port'] as int;

  // 3. Bind UDP and send hole-punch
  final udpSocket = await RawDatagramSocket.bind(
    InternetAddress.anyIPv4, 0,
  );
  udpSocket.send(
    utf8.encode('ws-rtp $token'),
    serverAddress,
    holepunchPort,
  );

  // 4. Try to confirm UDP (2-second timeout)
  final udpConfirmed = await _confirmUdp(
    udpSocket, serverAddress, holepunchPort, token,
  );

  if (udpConfirmed) {
    // Receive RTP over UDP
    _receiveRtpOverUdp(udpSocket);
  } else {
    // UDP blocked — RTP will arrive as WebSocket binary frames
    udpSocket.close();
  }

  // 5. Listen on WebSocket for SDP, state, and fallback RTP
  channel.stream.listen((message) {
    if (message is String) {
      final msg = jsonDecode(message);
      if (msg['type'] == 'sdp') _configureFfmpeg(msg['sdp'] as String);
    } else if (message is List<int>) {
      _onRtpPacket(Uint8List.fromList(message));
    }
  });

  // 6. Ping loop
  _startPingLoop(channel);
}
```

---

## 6. UDP RTP Reception (unchanged logic, for reference)

```dart
void _receiveRtpOverUdp(RawDatagramSocket socket) {
  socket.listen((event) {
    if (event == RawSocketEvent.read) {
      final datagram = socket.receive();
      if (datagram != null) {
        // Skip any remaining dummy packets
        if (datagram.data.length > 10 &&
            !utf8.decode(datagram.data).startsWith('ws-rtp')) {
          _onRtpPacket(datagram.data);
        }
      }
    }
  });
}
```

---

## Checklist

- [ ] Rename `t5rtp` → `ws-rtp` in hole-punch payload
- [ ] Remove client port from hole-punch payload
- [ ] Read `holepunch_port` instead of `server_port` from init message
- [ ] Implement `_confirmUdp()` with 2-second timeout
- [ ] Send `ws-rtp-ack` after receiving `ws-rtp-dummy`
- [ ] Handle `falling_back_rtp_to_ws` message (log / update UI state)
- [ ] Handle binary WebSocket frames as fallback RTP packets

All done. Here's a summary of what changed:

**Core change:** `Transport::Tcp(Arc<TcpStream>)` → `Transport::WebSocket(mpsc::UnboundedSender<Vec<u8>>)`. No separate TCP listener, no TCP port binding, no length-prefix framing.

**Flow:**
- Server binds only a UDP port (30000-40000)
- Client sends UDP hole-punch → server sends dummy packets → client acks → UDP confirmed
- If no ack within 5 seconds: server creates a channel, stores `Transport::WebSocket(tx)`, and the main WebSocket loop forwards `tx` messages as binary frames to the client
- `on_new_packet` either sends raw UDP bytes or pushes to the channel — the WebSocket loop does the actual send, keeping the sender on one task

**Removed entirely:** `TcpListener`, `TcpStream`, TCP framing, `bind_holepunch_port` returning a `TcpListener`, all the TCP accept/read logic.
**Core change:** `Transport::Tcp(Arc<TcpStream>)` → `Transport::WebSocket(mpsc::UnboundedSender<Vec<u8>>)`. No separate TCP listener, no TCP port binding, no length-prefix framing.

**Flow:**
- Server binds only a UDP port (30000-40000)
- Client sends UDP hole-punch → server sends dummy packets → client acks → UDP confirmed
- If no ack within 5 seconds: server creates a channel, stores `Transport::WebSocket(tx)`, and the main WebSocket loop forwards `tx` messages as binary frames to the client
- `on_new_packet` either sends raw UDP bytes or pushes to the channel — the WebSocket loop does the actual send, keeping the sender on one task

**Removed entirely:** `TcpListener`, `TcpStream`, TCP framing, `bind_holepunch_port` returning a `TcpListener`, all the TCP accept/read logic.