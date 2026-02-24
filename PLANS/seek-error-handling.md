# Seek/Go-Live Error Handling Plan

## Problem
Currently, when `seek()` or `go_live()` requests fail in the WSC-RTP session, the errors are returned to the caller but **not emitted as `StreamEvent::Error`** to the event stream. This means:
- The Dart side only knows about the error if it explicitly checks the Result
- Other parts of the UI listening to the event stream won't be notified of the failure
- The controller's mode (live/playback) may become inconsistent with the actual server state

## Solution

### 1. Rust Side: Emit Error Events on Failure
In `rust/src/core/input/wsc_rtp.rs`:
- Update `send_control_request()` to emit `StreamEvent::Error` when requests fail
- The method already updates the session mode on success via `WscRtpSessionMode` event
- On error, we should emit an error event before returning the error

### 2. Controller Mode Tracking
The controller already receives `StreamEvent::WscRtpSessionMode` events which contain:
- `is_live: bool` - indicates live vs playback mode
- `current_time_ms: i64` - current playback position
- `speed: f64` - playback speed

The example demo (`example/lib/wsc_rtp_seek_demo.dart`) already tracks this via `_sessionMode` state.

### 3. Example Demo Updates
The demo already:
- Listens to `StreamEvent::WscRtpSessionMode` events in `_startEventListener()`
- Updates `_sessionMode` state accordingly
- Displays LIVE/DVR badge based on `isLive`
- Shows current time and speed

However, when seek/go-live fails, the demo shows a SnackBar but **doesn't emit an error event**. After the Rust fix, the error event will be automatically received by the event listener.

## Implementation Details

### Changes to `rust/src/core/input/wsc_rtp.rs`

In `send_control_request()`:
```rust
async fn send_control_request(
    &self,
    endpoint: &str,
    body: impl serde::Serialize,
) -> Result<()> {
    let session_id = self.active_session_id.read().clone()
        .ok_or_else(|| anyhow::anyhow!("session is reconnecting, no active server session"))?;

    // ... HTTP request code ...

    let response = self.http_client.post(...).send().await
        .context("WSC-RTP control request failed")?;

    let status = response.status();
    if !status.is_success() {
        let error_msg = format!("WSC-RTP control request failed with status: {}", status);
        // Emit error event before returning
        self.session_common.send_event_msg(StreamEvent::Error(error_msg.clone()));
        anyhow::bail!(error_msg);
    }

    // ... parse response ...
}
```

This ensures that:
1. Any failure in seek/go_live/speed requests emits a `StreamEvent::Error`
2. The error is still returned to the caller (Dart side gets the Result)
3. The event stream listeners also receive the error notification

### No Changes Needed for Mode Tracking
The `WscRtpSessionMode` event is already sent on successful control requests, updating the mode correctly. The example demo already handles this.

## Verification
After implementation:
1. When seek fails, both the Result error AND StreamEvent::Error are triggered
2. When go-live fails, both the Result error AND StreamEvent::Error are triggered
3. The demo's event listener receives the error and can log/handle it
4. The mode tracking remains consistent via `WscRtpSessionMode` events

## Implementation Summary

### Rust Changes (`rust/src/core/input/wsc_rtp.rs`)
Updated `send_control_request()` to emit `StreamEvent::Error` in three failure scenarios:
1. **Session not active**: When the session is reconnecting and has no active server session
2. **HTTP request failure**: Network errors, connection failures, etc.
3. **HTTP error status**: When the server returns a non-success status code (4xx, 5xx)
4. **JSON parsing failure**: When the response cannot be parsed

### Dart Changes (`example/lib/wsc_rtp_seek_demo.dart`)
Enhanced `_startEventListener()` to:
1. Display error events from the stream as SnackBars when the controller is active
2. Distinguish between connection errors (shown via `_errorMessage`) and control operation errors (shown via SnackBar)

### Mode Tracking
The example demo already properly tracks and displays:
- **LIVE/DVR badge**: Red "LIVE" badge when live, blue "DVR" badge when in playback mode
- **Current time**: Formatted time display from `SessionMode.currentTimeMs`
- **Speed**: Current playback speed from `SessionMode.speed`

The `StreamEvent::WscRtpSessionMode` event is sent on successful control requests, keeping the UI in sync with the server state.
