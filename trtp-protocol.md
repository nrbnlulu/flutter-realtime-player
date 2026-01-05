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
