# Plan: Add `playbin3` support for normal URI video playback

## Context
The project currently supports only WSC-RTP streams via `WscRtpSession`. Users need a way to play ordinary media files or network streams (e.g. `file:///…`, `http://…`, `rtsp://…`) using GStreamer's `playbin3` element, which handles demuxing, decoding, and buffering automatically.

---

## Files to modify / create

| File | Action |
|------|--------|
| `rust/src/core/types.rs` | Add `PlaybinConfig` struct + `Playbin(PlaybinConfig)` variant to `VideoConfig` |
| `rust/src/core/input/mod.rs` | Add `pub mod playbin;` |
| `rust/src/core/input/playbin.rs` | **New file** — `PlaybinSession` struct + `VideoSession` impl |
| `rust/src/api/simple.rs` | Import `PlaybinSession`, add `VideoConfig::Playbin` arm in `create_playable` |

---

## Step 1 — `core/types.rs`

Add below `WscRtpSessionConfig`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[flutter_rust_bridge::frb(sync)]
pub struct PlaybinConfig {
    pub uri: String,
    pub mute: bool,
}
```

Extend `VideoConfig`:

```rust
pub enum VideoConfig {
    WscRtp(WscRtpSessionConfig),
    Playbin(PlaybinConfig),
}
```

---

## Step 2 — `core/input/mod.rs`

```rust
pub mod playbin;
pub mod wsc_rtp;
```

---

## Step 3 — `core/input/playbin.rs` (new)

### Struct

```rust
pub struct PlaybinSession {
    session_common: VideoSessionCommon,
    config: PlaybinConfig,
    shutdown_sender: tokio::sync::mpsc::Sender<()>,
    active_pipeline: Mutex<Option<Arc<gst::Pipeline>>>,
    current_speed: Mutex<f64>,
}
```

### Constructor

```rust
pub fn new(config: PlaybinConfig, session_common: VideoSessionCommon)
    -> (Arc<Self>, tokio::sync::mpsc::Receiver<()>)
```

### `execute` (same texture setup pattern as `WscRtpSession::execute`)

1. Create `PayloadHolder` + Flutter texture on platform thread via `invoke_on_platform_main_thread`
2. Send `StreamState::Loading`
3. Build `appsink`:
   ```rust
   let caps = gst::Caps::builder("video/x-raw").field("format", "RGBA").build();
   let appsink = gst_app::AppSink::builder().caps(&caps).sync(false).build();
   ```
4. Set appsink callbacks (identical frame-copy logic from `WscRtpSession::run_session_loop` — copy row-by-row to strip stride padding, set `PayloadHolder`, call `mark_frame_available`)
5. Build `playbin3` pipeline:
   ```rust
   let playbin = gst::ElementFactory::make("playbin3").build()?;
   playbin.set_property("uri", &config.uri);
   playbin.set_property("video-sink", &appsink);
   if config.mute {
       let fakesink = gst::ElementFactory::make("fakesink").build()?;
       playbin.set_property("audio-sink", &fakesink);
   }
   let pipeline = playbin.downcast::<gst::Pipeline>().map_err(|_| anyhow!("not a pipeline"))?;
   ```
6. Store `Arc<gst::Pipeline>` in `active_pipeline`
7. Set up bus sync handler sending `GstBusEvent` (Error/Eos/Buffering) over `tokio::sync::mpsc::channel`
8. `pipeline.set_state(gst::State::Playing)?`
9. Send `StreamState::Playing { texture_id, seekable: true }`
10. Select loop watching:
    - `shutdown_rx` → null pipeline, break
    - `gst_event_rx`:
      - `Error(msg)` → send `StreamEvent::Error`, null pipeline, break (return `Err`)
      - `Eos` → null pipeline, send `StreamState::Stopped`, break (return `Ok`)
      - `Buffering(pct)` → pause/play pipeline, emit state (no-op if already right state)
11. Drop texture + payload on platform thread

### `seek` method

```rust
pub fn seek_pipeline(pipeline: &gst::Pipeline, ts_ms: u64) -> anyhow::Result<()> {
    let pos = gst::ClockTime::from_mseconds(ts_ms);
    pipeline.seek_simple(
        gst::Format::Time,
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
        pos,
    ).map_err(|_| anyhow!("seek failed"))
}
```

Called from `VideoSession::seek` on the stored `active_pipeline`.

### `set_speed` method

```rust
// query current pos, then:
pipeline.seek(speed, gst::Format::Time,
    gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
    gst::SeekType::Set, current_pos,
    gst::SeekType::None, gst::ClockTime::NONE)
```

Store new speed in `current_speed` Mutex.

### `go_to_live_stream` — return `Ok(())` (no-op, not applicable)

### `terminate`

```rust
fn terminate(&self) {
    if let Some(p) = self.active_pipeline.lock().take() {
        let _ = p.set_state(gst::State::Null);
    }
    let _ = self.shutdown_sender.blocking_send(());
}
```

---

## Step 4 — `api/simple.rs`

Add import:
```rust
use crate::core::input::playbin::PlaybinSession;
```

Add arm to `create_playable`:
```rust
VideoConfig::Playbin(playbin_config) => {
    let session_common = VideoSessionCommon::new(session_id, engine_handle, sink);
    let (session, shutdown_rx) = PlaybinSession::new(playbin_config, session_common);
    let session_clone = session.clone();
    tokio::spawn(async move { session_clone.execute(shutdown_rx).await });
    insert_session(session_id, session);
}
```

---

## Verification

1. `task codegen` — regenerates Dart bindings (confirms `PlaybinConfig` and new `VideoConfig` variant are exported)
2. Build: `cargo build -p flutter_realtime_player_rust`
3. Smoke-test: create a Dart example that:
   - calls `createNewSession()`
   - calls `createPlayable(sessionId, engineHandle, VideoConfig.playbin(PlaybinConfig(uri: "file:///...", mute: false)), sink)`
   - observes `StreamState.playing` with a valid `textureId`
   - renders the texture — frames should appear
4. Verify seek: call `seekToTimestamp(sessionId, 5000)` — video jumps to 5 s
5. Verify EOS: let a short clip finish — `StreamState.stopped` should be emitted

---

## Example App Updates

### Files modified:
- `example/lib/main.dart` - Added playbin toggle switch, configuration UI, and player widget routing
- `example/lib/playbin_player.dart` (new) - PlaybinPlayerWidget with basic playback controls

### Usage in example app:
1. Launch the example app
2. Toggle "Use Playbin" switch
3. Enter a media URI (e.g., `https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm`)
4. Optionally toggle "Mute Audio"
5. Click "Start Stream"
