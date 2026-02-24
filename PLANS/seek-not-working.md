# Seek Not Working - Analysis and Fix Plan

## Problem Summary
Seek functionality in the example app had multiple issues:
1. Seek was called with negative timestamps
2. Server doesn't send `SessionMode` events
3. Server returns `currentTimeMs: 0` after seek
4. Time doesn't advance in DVR mode

## Root Causes Identified

### 1. Negative Timestamps (FIXED)
**Problem:** `_liveEdgeMs` was initialized to 0, causing slider min to be negative.
**Fix:** Initialize `_liveEdgeMs = DateTime.now().millisecondsSinceEpoch` when connection succeeds.

### 2. Server Doesn't Send SessionMode Events (WORKED AROUND)
**Problem:** The WSC-RTP server doesn't send `SessionMode` events, so the client never knows the live edge.
**Fix:** Initialize live edge on connection success instead of waiting for server event.

### 3. Server Returns currentTimeMs=0 After Seek (WORKED AROUND)
**Problem:** After seek, server returns `WscRtpMode_Dvr { currentTimeMs: 0, speed: 1.0 }`, which overwrites the seek target.
**Fix:** Track `_lastSeekTargetMs` and use it when server returns 0.

### 4. Time Doesn't Advance in DVR Mode (FIXED)
**Problem:** Ticker only advanced time when `_isLive=true`.
**Fix:** Always advance `_currentTimeMs` by speed, regardless of live/DVR mode.

## Fixes Applied

### File: `example/lib/wsc_rtp_player.dart`

#### Fix 1: Initialize live edge on connection
```dart
_connect() {
  // ...
  setState(() {
    _liveEdgeMs = DateTime.now().millisecondsSinceEpoch;
    _currentTimeMs = _liveEdgeMs;
  });
}
```

#### Fix 2: Track last seek target
```dart
int? _lastSeekTargetMs;

_seekTo(targetMs) {
  _lastSeekTargetMs = targetMs;
  // ... seek call
}
```

#### Fix 3: Use seek target when server returns 0
```dart
if (mode is WscRtpMode_Dvr) {
  _isLive = false;
  _speed = mode.speed;
  if (mode.currentTimeMs > 0) {
    _currentTimeMs = mode.currentTimeMs;
    _lastSeekTargetMs = null;
  } else if (_lastSeekTargetMs != null) {
    _currentTimeMs = _lastSeekTargetMs!;
  }
}
```

#### Fix 4: Advance time in DVR mode
```dart
_ticker = Timer.periodic(Duration(seconds: 1), (_) {
  setState(() {
    _currentTimeMs += (_speed * 1000).round();
    if (_isLive) {
      _liveEdgeMs = _currentTimeMs;
    }
  });
});
```

## Current Status

### Seeking now works, but slider position might be wrong

**New Issue:** After seek, the slider doesn't show the correct position.

**Possible causes:**
1. `_liveEdgeMs` might be getting overwritten by server events
2. Slider value calculation might be clamping incorrectly
3. State updates might not be triggering rebuilds correctly

### Debug Logging Added

Extensive logging to trace the issue:
- Connection: logs `_liveEdgeMs` initialization
- Slider: logs target, `_currentTimeMs`, `_liveEdgeMs`, `_sliderValue`
- Seek: logs all parameters and results
- Server events: logs mode and timestamps
- Build: logs all state variables

## Next Steps

### Check the new logs for:
1. **After seek, what is `_currentTimeMs`?**
   - Should be the seek target
   - If 0 or wrong, the DVR mode event handler is broken

2. **After seek, what is `_liveEdgeMs`?**
   - Should remain at the live point (not change)
   - If it changes to seek target, something is wrong

3. **What is `_sliderValue`?**
   - Should be `_currentTimeMs` clamped to `[_sliderMinMs, _sliderMaxMs]`
   - If wrong, the slider calculation is broken

4. **Does `_currentTimeMs` advance every second?**
   - Should increase by `speed * 1000` every second
   - If not, the ticker is not working

### Potential fixes based on findings:

#### If `_liveEdgeMs` is being overwritten:
Don't update `_liveEdgeMs` from server events in DVR mode.

#### If slider value is clamped incorrectly:
The slider range might be wrong. In DVR mode, the range should still be `[liveEdge - window, liveEdge]`, but the current position can be anywhere in that range.

#### If state updates aren't triggering rebuilds:
Make sure `setState()` is being called correctly after seek.
