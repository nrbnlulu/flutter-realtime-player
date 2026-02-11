# Refine GStreamer WSC-RTP Video Rendering

## Context

The WSC-RTP GStreamer pipeline currently has several rendering issues:
1. The pipeline hardcodes output to 640x480 via `videoscale` — losing the original video aspect ratio and resolution.
2. There's a full resize infrastructure (Dart debounce → FFI → Rust trait → registry) that's unnecessary — Flutter should handle scaling via its `Texture` widget, not by re-rendering pixels at a new resolution.
3. Stride/padding from GStreamer decoders isn't handled in the appsink callback — the data is copied raw via `map.as_slice().to_vec()` without checking if row stride matches `width * 4`.
4. The `OriginVideoSize` event variant exists in `StreamEvent` but is never emitted — Dart has no way to know the actual video dimensions for aspect ratio.

## Changes

### 1. Remove `videoscale` + hardcoded dimensions from GStreamer pipeline

**File:** `rust/src/core/input/wsc_rtp.rs`

- Change `build_pipeline_str` to remove `width`/`height` params, remove `videoscale`, and remove `width={w},height={h}` from output caps.
- New pipeline tail: `... ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink sync=false emit-signals=true`
- This lets GStreamer output at the video's native resolution. Flutter handles display scaling.
- Update the call site (currently passes hardcoded `640, 480`) to use the simplified signature.

### 2. Handle stride in appsink callback

**File:** `rust/src/core/input/wsc_rtp.rs`

In the `new_sample` appsink callback, GStreamer buffers may have stride padding. After removing `videoscale`, this becomes more likely since we're no longer forcing specific dimensions.

- Read stride from the `video/x-raw` caps via `gst_video::VideoInfo::from_caps` or from the structure's `stride` field.
- If stride == `width * 4` (RGBA), copy directly (current behavior).
- If stride != `width * 4`, copy row-by-row stripping padding — same logic as `RawRgbaFrame::from_ffmpeg` in `payload.rs`.

Alternatively, use `gst_video::VideoFrame::from_buffer_readable` which handles stride transparently and gives access to pixel data per plane. This is the cleaner GStreamer-native approach.

### 3. Emit `OriginVideoSize` event on first frame

**File:** `rust/src/core/input/wsc_rtp.rs`

- Add a `bool` flag (`first_frame_sent`) in the appsink callback closure.
- On the first sample, extract `width` and `height` from caps and send `StreamEvent::OriginVideoSize { width, height }` via `session_common.send_event_msg(...)`.
- This requires passing a weak ref to `session_common` (or just the events sink) into the callback closure.

### 4. Remove resize infrastructure (GStreamer-only, keep trait for now)

**Rust side:**
- `rust/src/api/simple.rs` — remove `resize_stream_session` function
- `rust/src/core/session/registry.rs` — remove `resize_stream_session` function
- `rust/src/core/session/mod.rs` — remove `fn resize(...)` from `VideoSession` trait
- `rust/src/core/input/wsc_rtp.rs` — remove `fn resize(...)` impl from `WscRtpSession`

**Dart side:**
- `lib/video_player.dart`:
  - Remove `resizeStream` method from `VideoController`
  - In `_VideoPlayerWithSize`: remove `currentDimensions`, `_resizeDebounceTimer`, `_pendingResize`, `_handleResize()`, and the resize detection in `build()`
  - Remove `initialDimensions` field from `_VideoPlayerWithSize` (and constructor params)
  - Simplify `_VideoPlayerWithSizeState` — just listen to state and render `Texture`
- `lib/video_player.dart` (`VideoPlayer.fromConfig`): Remove the `LayoutBuilder` that calculates dimensions for `_createControllerWithSize`. Just use fixed fallback dimensions (or remove `fromConfig` entirely if it becomes trivial — but keep it for backwards compat).
- `VideoController.create` / `VideoController.createWscRtp`: Keep `VideoDimensions` param for now (it's still used by `VideoInfo` which goes to Rust), but the value is informational/initial only — no runtime resize.

**After removing the Rust API function**, run `fvm exec flutter_rust_bridge_codegen generate` to regenerate bindings (this removes `resizeStreamSession` from `lib/rust/api/simple.dart` and `frb_generated.*`).

### 5. (Example app) Remove hardcoded AspectRatio, use OriginVideoSize

**Files:** `example/lib/main.dart`, `example/lib/wsc_rtp_seek_demo.dart`

- Listen to `StreamEvent_OriginVideoSize` from the events stream to get the real aspect ratio.
- Replace hardcoded `AspectRatio(aspectRatio: 16 / 9)` with dynamic aspect ratio from the event.
- Default to 16:9 until the first `OriginVideoSize` event arrives.

## File Summary

| File | Action |
|------|--------|
| `rust/src/core/input/wsc_rtp.rs` | Remove videoscale/hardcoded dims from pipeline, add stride handling, emit OriginVideoSize, remove resize impl |
| `rust/src/api/simple.rs` | Remove `resize_stream_session` |
| `rust/src/core/session/mod.rs` | Remove `resize` from `VideoSession` trait |
| `rust/src/core/session/registry.rs` | Remove `resize_stream_session` |
| `lib/video_player.dart` | Remove resize infra from controller + widget |
| `example/lib/main.dart` | Use dynamic aspect ratio from OriginVideoSize event |
| `example/lib/wsc_rtp_seek_demo.dart` | Use dynamic aspect ratio from OriginVideoSize event |

## Verification

1. `cargo check` — Rust compiles with no new errors
2. `fvm exec flutter_rust_bridge_codegen generate` — regenerate bindings
3. `fvm exec flutter analyze` — no new Dart errors in lib/ or example/
4. `fvm exec flutter build linux` (in example/) — example builds
5. Manual test: connect to a WSC-RTP stream, verify video renders at native resolution with correct aspect ratio and no slanting/distortion
