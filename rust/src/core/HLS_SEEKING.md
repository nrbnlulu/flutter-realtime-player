# HLS Seeking Implementation

This document explains the implementation of HLS (HTTP Live Streaming) seeking functionality in the Flutter Realtime Player.

## Overview

HLS live streams are normally unseekable in FFmpeg because they're designed for real-time playback. However, many HLS streams maintain a "DVR window" (a sliding window of past segments), allowing viewers to seek backward within available content. This implementation enables seeking within that DVR window.

## Problem Statement

HLS presents several challenges for seeking:

1. **Unseekable by Default**: FFmpeg marks live HLS streams with `AVFMT_FLAG_UNSEEKABLE`
2. **Segment-Based Timeline**: The stream is composed of discrete segments (`.ts` or `.m4s` files), each with its own timestamp
3. **Discontinuous Timestamps**: Segment timestamps may not form a continuous timeline
4. **Playlist Updates**: The playlist changes as new segments arrive and old ones expire
5. **Sequence Number Wraparound**: Segment sequence numbers can wrap around

## Solution Architecture

### Trait-Based Design

The implementation uses the `TimelineContext` trait to abstract timeline management:

```rust
trait TimelineContext {
    fn adjust_seek_timestamp(&self, timestamp_us: i64) -> i64;
    fn update_timeline(&mut self);
    fn get_time_offset(&self) -> i64;
    fn is_seekable(&self) -> bool;
}
```

This allows different implementations for HLS vs standard streams without code duplication.

### Two Implementations

1. **`HLSTimelineContext`**: Handles segment-based seeking for HLS streams
2. **`StandardTimelineContext`**: Simple pass-through for regular streams

## HLS Timeline Management

### Virtual Timeline Calculation

The key insight is to create a virtual continuous timeline from discontinuous segments:

```
hls_start_time = current_packet_timestamp - sum_of_previous_segment_durations
```

This creates a reference point that allows us to map between:
- **User's requested time**: Time relative to the start of available content
- **Actual segment time**: FFmpeg's internal timestamp within segments

### Segment Tracking

The implementation tracks:

```rust
struct HLSTimelineContext {
    hls_start_time: i64,      // Virtual start time in microseconds
    hls_cur_duration: i64,    // Duration from playlist start to current segment
    hls_prev_seq_no: i64,     // Previous sequence number for change detection
    hls_ctx: *const HLSContext,      // Direct pointer to HLS context
    playlist: *const playlist,        // Direct pointer to active playlist
    is_initialized: bool,     // Whether timeline is ready
}
```

### Timeline Update Process

Called periodically during playback in `update_timeline()`:

1. **Access Stored Pointers**: Use pre-stored HLS context and playlist pointers
2. **Read Sequence Number**: Check current sequence number from playlist
3. **Detect Changes**: Compare current vs previous sequence numbers
4. **Calculate Duration**: Sum segment durations up to current position using stored playlist
5. **Update Reference**: Recalculate virtual start time if playlist changed

```rust
unsafe fn update_hls_timeline(&mut self) {
    if self.playlist.is_null() {
        return;
    }
    
    let playlist = &*self.playlist;
    let cur_seq_no = playlist.cur_seq_no;
    
    if cur_seq_no != self.hls_prev_seq_no {
        // Playlist updated - recalculate timeline
        self.hls_prev_seq_no = cur_seq_no;
        self.hls_cur_duration = playlist.calculate_duration_until_seq(cur_seq_no);
        // Convert to microseconds
        self.hls_cur_duration = (self.hls_cur_duration * 1000000) / AV_TIME_BASE;
    }
    
    self.is_initialized = true;
}
```

## Seeking Process

### Timestamp Adjustment

When seeking to timestamp T (relative to available content start):

```
adjusted_timestamp = T + hls_start_time - first_timestamp
```

This converts from the user's virtual timeline to FFmpeg's actual segment timestamps.

### Force Seekable

Before seeking, clear the unseekable flag:

```rust
unsafe {
    let fmt_ctx = &mut *fmt_ctx_ptr;
    // Clear AVFMT_FLAG_UNSEEKABLE for HLS
    fmt_ctx.ctx_flags &= !ffi::AVFMT_FLAG_UNSEEKABLE;
}
```

This tricks FFmpeg into allowing seek operations.

### Seek with Fallback

The implementation attempts two seek strategies:

1. **Forward Seek**: `seek(position, ..position)` - seeks to exact position or later
2. **Backward Seek** (fallback): `seek(position, position..)` - seeks to exact position or earlier

```rust
if let Err(e) = ctx.input_ctx.seek(position, ..position) {
    // Try backward seek as fallback
    match ctx.input_ctx.seek(position, position..) {
        Ok(_) => info!("Fallback backward seek succeeded"),
        Err(e2) => return Err(anyhow!("Both seeks failed: {:?}, {:?}", e, e2)),
    }
}
```

### Post-Seek Cleanup

After seeking:
1. Flush the decoder to clear any buffered frames
2. Reset frame synchronizer to avoid timing issues

## FFmpeg Private Structures

### HLS Context Access

The implementation accesses FFmpeg's internal HLS structures defined in `ffmpeg_private.rs`:

- **`HLSContext`**: Main HLS demuxer context (updated to match FFmpeg 6.x/7.x)
- **`playlist`**: Represents a variant or media playlist
- **`segment`**: Individual segment information with duration and URL
- **`rendition`**: Alternative audio/subtitle tracks
- **`variant`**: Different quality levels

### HLSContext Structure (Recent FFmpeg)

The implementation uses the updated HLSContext structure:

```rust
pub struct HLSContext {
    pub class: *mut AVClass,
    pub ctx: *mut AVFormatContext,
    pub n_variants: c_int,
    pub variants: *mut *mut variant,
    pub n_playlists: c_int,
    pub playlists: *mut *mut playlist,
    pub n_renditions: c_int,
    pub renditions: *mut *mut rendition,
    pub cur_seq_no: c_long,              // Current sequence number
    pub m3u8_hold_counters: c_int,
    pub live_start_index: c_int,
    pub prefer_x_start: c_int,
    pub first_packet: c_int,
    pub first_timestamp: c_long,         // Used for timestamp adjustment
    pub cur_timestamp: c_long,
    pub interrupt_callback: *mut AVIOInterruptCB,
    pub avio_opts: *mut AVDictionary,
    pub seg_format_opts: *mut AVDictionary,
    pub allowed_extensions: *mut c_char,
    pub allowed_segment_extensions: *mut c_char,
    pub extension_picky: c_int,
    pub max_reload: c_int,
    pub http_persistent: c_int,
    pub http_multiple: c_int,
    pub http_seekable: c_int,
    pub seg_max_retry: c_int,
    pub playlist_pb: *mut AVIOContext,
    pub crypto_ctx: HLSCryptoContext,
}
```

Note: The `cur_playlist` field was removed in recent FFmpeg versions. Use `get_first_playlist()` instead.

### Direct Pointer Storage

For efficiency, the implementation stores direct pointers to HLS structures during initialization:

```rust
unsafe fn new(fmt_ctx_ptr: *mut ffi::AVFormatContext) -> Option<Self> {
    let fmt_ctx = &*fmt_ctx_ptr;
    
    // Verify format is "hls"
    let iformat = &*fmt_ctx.iformat;
    let name = std::ffi::CStr::from_ptr(iformat.name);
    if name.to_string_lossy() != "hls" {
        return None;
    }
    
    // Cast priv_data to HLSContext and store pointer
    let hls_ctx = fmt_ctx.priv_data as *const ffmpeg_private::HLSContext;
    let hls_ctx_ref = &*hls_ctx;
    
    // Get and store first playlist pointer
    // For multi-variant streams, this gets the first variant's playlist
    let playlist = hls_ctx_ref
        .get_first_playlist()
        .map(|p| p as *const _)?;
    
    Some(Self {
        hls_start_time: ffi::AV_NOPTS_VALUE,
        hls_cur_duration: 0,
        hls_prev_seq_no: ffi::AV_NOPTS_VALUE,
        hls_ctx,
        playlist,
        is_initialized: false,
    })
}
```

**Important**: 
- These pointers remain valid as long as the parent `DecodingContext` owns the format context
- These structures are version-dependent and may change between FFmpeg releases
- The pointers must not be dereferenced after the format context is destroyed

## Integration with SoftwareDecoder

### Initialization

During stream initialization:

```rust
let timeline_ctx: Box<dyn TimelineContext + Send> = if is_hls_stream {
    unsafe {
        match HLSTimelineContext::new(format_ctx_ptr) {
            Some(ctx) => Box::new(ctx),
            None => {
                // Fallback to standard if HLS context creation fails
                Box::new(StandardTimelineContext::new(start_time))
            }
        }
    }
} else {
    Box::new(StandardTimelineContext::new(start_time))
};
```

### Playback Loop

During packet reading:

```rust
// Update HLS timeline periodically
ctx.timeline_ctx.update_timeline();
```

### Seeking

When user requests seek:

```rust
pub fn seek(&self, timestamp_us: i64) -> anyhow::Result<()> {
    let adjusted_timestamp = ctx.timeline_ctx.adjust_seek_timestamp(timestamp_us);
    
    // Force seekable for HLS
    // ... (clear unseekable flag)
    
    // Perform seek with adjusted timestamp
    ctx.input_ctx.seek(position, ..position)?;
    
    Ok(())
}
```

## Limitations

1. **FFmpeg Version Dependency**: Internal structures may change between FFmpeg versions (currently matches FFmpeg 6.x/7.x)
2. **DVR Window Only**: Can only seek within available segments (typically last 30-60 seconds)
3. **Playlist Format**: Assumes standard HLS playlist format
4. **Single Variant**: Optimized for single-variant streams (most common case)
5. **Pointer Lifetime**: HLS context pointers must remain valid - format context must not be destroyed while timeline context exists

## Testing Recommendations

Test with various HLS streams:

1. **Live HLS with DVR**: Typical use case (e.g., YouTube Live, Twitch)
2. **VOD HLS**: Should work with full timeline
3. **Event HLS**: Growing playlist
4. **Multi-variant HLS**: Different bitrates/qualities
5. **Encrypted HLS**: AES-128 encrypted segments

## References

1. **Flyleaf C# Implementation**: The original inspiration for this design
2. **FFmpeg HLS Demuxer**: `libavformat/hls.c` in FFmpeg source
3. **HLS Specification**: RFC 8216 - HTTP Live Streaming
4. **FFmpeg Seeking Documentation**: `https://ffmpeg.org/doxygen/trunk/group__lavf__decoding.html#ga084a50e7419a8e7e6606cd0a6e75a608`

## Future Improvements

1. **Multi-Variant Support**: Currently uses first playlist; add logic to select optimal variant based on bandwidth
2. **Playlist Refresh**: Detect and handle playlist pointer changes when FFmpeg reloads the playlist
3. **Adaptive Bitrate**: Handle switching between variants during seek
4. **Precise Timing**: Use PTS from packets to refine hls_start_time calculation
5. **Playlist Monitoring**: Actively monitor playlist updates for better accuracy
6. **Version Detection**: Detect FFmpeg version and adapt structure layouts automatically
7. **Segment Metadata**: Utilize individual segment metadata for more accurate seeking
8. **cur_seq_no Tracking**: Use HLSContext's cur_seq_no field for better sequence tracking