# WSC-RTP: Internet Loss Detection & Clean Fallback to Retry

## Problem

When internet connectivity is lost during an active `wsc_rtp` session:

1. **Ping send errors are silently dropped** — `let _ = ws_sink.send(...).await` discards errors.
2. **No pong timeout** — `ws_stream.next()` hangs indefinitely when the OS hasn't yet
   detected the broken TCP connection (e.g., WiFi drops without a TCP RST). This can take
   minutes via the kernel's default TCP keepalive timeout, preventing the retry loop from
   kicking in.

## Fix

### 1. Track last pong time (`last_pong: Instant`)

Initialised to `Instant::now()` at the start of `run_session_loop`. Updated whenever a
`WscRtpServerMessage::Pong` is received.

### 2. Pong timeout check on every ping tick

Before sending each ping, check `last_pong.elapsed() > PONG_TIMEOUT`. If exceeded:
- Abort UDP receiver task
- Set GStreamer pipeline to `Null`
- Return `Err` → outer `execute` loop catches this and retries (with backoff)

`PONG_TIMEOUT = 10s` (5 × PING_INTERVAL of 2s).

### 3. Propagate ping send errors

Replace `let _ = ws_sink.send(...).await` with an explicit error check. A failed send
means the connection is already broken — trigger the same cleanup + retry path
immediately rather than waiting for the stream to signal the error.

## Constants

```
PING_INTERVAL  = 2s   (existing)
PONG_TIMEOUT   = 10s  (new — 5 missed pings before declaring connection lost)
```

## Files Changed

- `rust/src/core/input/wsc_rtp.rs`
