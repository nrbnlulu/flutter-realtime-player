# WSC-RTP Auto-Restart Support

## Context

WSC-RTP sessions currently have zero error resilience — any WebSocket disconnection, close frame, or GStreamer pipeline failure permanently kills the session. The user must manually destroy and recreate the session from Dart. Additionally, there is no GStreamer bus message handler, so pipeline errors/EOS go undetected.

The `WscRtpSessionConfig` already has an `auto_restart: bool` field that is unused. This plan implements it.

**Goal**: When `auto_restart` is enabled, automatically reconnect the WebSocket, rebuild the GStreamer pipeline, and resume streaming — while preserving the same Dart sinks and Flutter texture (same `texture_id`).

---

## Approach: Retry Loop Inside `execute()`

The retry loop lives **inside `execute()`**, not outside. This means:
- The `Arc<WscRtpSession>` stays stable in `SESSION_CACHE` — no re-insertion needed
- External callers (seek, go_live, set_speed) keep working via the same Arc
- Dart sinks (`state_sink`, `events_sink`) are preserved naturally on `session_common`
- Texture + PayloadHolder are created **once before** the retry loop and dropped **once after** it exits

### Intentional vs unintentional exit
- `shutdown_rx` signal → **never restart** (user/system requested termination)
- WebSocket close/error, GStreamer bus Error/EOS → **restart if `auto_restart` is true**

---

## Implementation Steps

### Step 1: Refactor `WscRtpSession` struct — split config from connection state

**File**: `rust/src/core/input/wsc_rtp.rs`

Remove per-connection fields from the struct. The media-server `session_id` moves behind a lock so HTTP control methods can access it (or error during reconnect).

```rust
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
```

- `initial_sdp`, `holepunch_port` → become locals in the connect function
- `pipeline` → stored in `Mutex<Option<>>` so `terminate()` can null it, and it gets replaced each connection
- `session_id` (media-server) → `RwLock<Option<String>>`, cleared during reconnect

### Step 2: Extract connection logic into `connect_and_setup_pipeline()`

**File**: `rust/src/core/input/wsc_rtp.rs`

Extract the current `WscRtpSession::new()` I/O logic into a standalone async method:

```rust
async fn connect_and_setup_pipeline(&self) -> Result<ConnectionResources>
```

This does: WS connect → handshake (Init + SDP) → UDP holepunch → parse SDP → build GStreamer pipeline → return resources.

```rust
struct ConnectionResources {
    ws_sink: WsSink,
    ws_stream: WsStream,
    udp_sock: Option<UdpSocket>,
    pipeline: gst::Pipeline,
    server_session_id: String,
}
```

### Step 3: Simplify `WscRtpSession::new()` to be sync (config-only)

**File**: `rust/src/core/input/wsc_rtp.rs`

`new()` no longer does any I/O — just builds the struct with config. Returns `(Arc<Self>, mpsc::Receiver<()>)`.

**File**: `rust/src/api/simple.rs`

Update `create_wsc_rtp_playable()`:
```rust
let (session, shutdown_rx) = WscRtpSession::new(config, session_common, HTTP_CLIENT.clone());
let session_clone = session.clone();
tokio::spawn(async move { session_clone.execute(shutdown_rx).await });
insert_session(session_id, session);
```

The function no longer returns `Result` for network errors — those happen inside `execute()`.

### Step 4: Restructure `execute()` with outer retry loop

**File**: `rust/src/core/input/wsc_rtp.rs`

```
execute(shutdown_rx):
  1. Create texture + PayloadHolder on platform main thread (ONCE)
  2. Send StreamState::Loading
  3. OUTER LOOP (retry loop):
     a. Call connect_and_setup_pipeline()
        - On failure: if shutdown requested → break; else backoff, send Loading, continue
     b. Store session_id + pipeline in self locks
     c. Wire appsink callbacks to existing PayloadHolder + texture
     d. Start pipeline, send StreamState::Playing { texture_id, seekable: true }
     e. Run inner select! loop (same as current: shutdown, ping, ws_stream, gst_bus_rx)
     f. On exit from inner loop:
        - Shutdown → break outer loop
        - Error/disconnect → cleanup pipeline, clear session_id lock, backoff, send Loading, continue
  4. Send StreamState::Stopped
  5. Drop texture + PayloadHolder on platform main thread
```

### Step 5: Add GStreamer bus monitoring

**File**: `rust/src/core/input/wsc_rtp.rs`

After pipeline creation in each connection iteration, set up a bus watch that forwards errors into the select loop:

```rust
let (gst_err_tx, mut gst_err_rx) = tokio::sync::mpsc::channel::<String>(4);
let bus = pipeline.bus().unwrap();
bus.set_sync_handler(move |_bus, msg| {
    match msg.view() {
        gst::MessageView::Error(err) => {
            let _ = gst_err_tx.try_send(format!("GStreamer error: {}", err.error()));
        }
        gst::MessageView::Eos(_) => {
            let _ = gst_err_tx.try_send("GStreamer EOS".to_string());
        }
        _ => {}
    }
    gst::BusSyncReply::Drop
});
```

Add `gst_err_rx.recv()` as a branch in the `tokio::select!` loop — on receive, set error output and break inner loop (triggering restart).

### Step 6: Add exponential backoff

**File**: `rust/src/core/input/wsc_rtp.rs`

Constants:
```rust
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
```

Logic inside retry loop:
- On connection failure or disconnect: sleep for `backoff` duration, then double it (capped at MAX_BACKOFF)
- On successful connection + first frame: reset backoff to INITIAL_BACKOFF
- Backoff sleep is **interruptible by shutdown**: use `tokio::select!` with `shutdown_rx.recv()` vs `tokio::time::sleep(backoff)`

No max retry count — retries indefinitely until shutdown or `auto_restart` is disabled. The Dart alive-tester (5s timeout) acts as the safety net if the UI navigates away.

### Step 7: Update `terminate()` and HTTP control methods

**File**: `rust/src/core/input/wsc_rtp.rs`

`terminate()`:
```rust
fn terminate(&self) {
    // Stop current pipeline if any
    if let Some(pipeline) = self.active_pipeline.lock().take() {
        pipeline.set_state(gst::State::Null);
    }
    let _ = self.shutdown_sender.blocking_send(());
}
```

`send_control_request()`:
```rust
async fn send_control_request(&self, endpoint: &str, body: Option<Value>) -> Result<()> {
    let session_id = self.active_session_id.read()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("session is reconnecting, no active server session"))?;
    // ... rest unchanged, using session_id local
}
```

### Step 8: Dart state communication during restart

States sent to Dart:
| Event | StreamState sent |
|-------|-----------------|
| Initial connection attempt | `Loading` |
| Connection established, pipeline playing | `Playing { texture_id, seekable: true }` |
| Disconnect (will retry) | `Loading` |
| Reconnected | `Playing { texture_id, seekable: true }` (same texture_id) |
| Intentional shutdown | `Stopped` |
| auto_restart=false + disconnect | `Stopped` |

Additionally, send `StreamEvent::Error(msg)` on each disconnect so Dart can show transient error info if desired.

---

## Files Modified

| File | Changes |
|------|---------|
| `rust/src/core/input/wsc_rtp.rs` | Major: struct refactor, extract connect fn, retry loop, GStreamer bus, backoff |
| `rust/src/api/simple.rs` | Minor: update `create_wsc_rtp_playable()` signature/call |

No changes needed to `dart_types.rs` (reuse `Loading` state), `session/mod.rs`, `registry.rs`, `types.rs`, or `payload.rs`.

---

## Verification

1. **Build**: `cargo build` in `rust/` — must compile cleanly
2. **Basic flow**: Connect to a WSC-RTP source → verify `Playing` state received by Dart, video renders
3. **WebSocket disconnect**: Kill the media server mid-stream → verify `Loading` sent, then auto-reconnects, `Playing` sent again with same `texture_id`
4. **GStreamer error**: Send malformed RTP data → verify bus error triggers restart
5. **Intentional shutdown**: Call `destroy_stream_session()` → verify `Stopped` sent, no restart attempt
6. **Backoff**: Disconnect server permanently → verify log shows increasing backoff intervals
7. **Control during reconnect**: Call seek while disconnected → verify returns error (not crash)
8. **Alive tester**: Disconnect server, stop calling `mark_session_alive` → verify session is cleaned up after 5s
