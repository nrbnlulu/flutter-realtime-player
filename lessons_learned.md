# Lessons Learned - HLS Support with #EXT-X-PROGRAM-DATE-TIME

## Overview
This document captures lessons learned while implementing support for HLS streams that have the `#EXT-X-PROGRAM-DATE-TIME:` extension tag, enabling the ability to show stream start times, current positions, and seek through the stream.

## Key Findings

### 1. HLS Metadata Extraction
- The `#EXT-X-PROGRAM-DATE-TIME:` tag contains absolute time information in ISO 8601 format
- FFmpeg may store this information in various metadata fields, not just standard format metadata
- Different HLS implementations may store the date time in different metadata keys (`program_datetime`, `program_date_time`, `creation_time`, etc.)
- The implementation needed to check multiple metadata sources from both input context and stream metadata

### 2. ISO 8601 Time Parsing
- Multiple formats for date time strings can appear in HLS manifests:
  - `2025-12-04T12:34:56.789Z` (with milliseconds)
  - `2025-12-04T12:34:56Z` (without milliseconds)
  - Standard RFC3339 format
- Proper error handling is crucial for different date formats across HLS implementations

### 3. Time Conversion and Seeking
- For absolute seeking, the target ISO 8601 time must be converted to a relative timestamp by subtracting the stream start time
- Seek operations use AV_TIME_BASE (microsecond precision) for accuracy
- Proper synchronization reset is needed after seeking to prevent timing issues

### 4. API Design Considerations
- Added `getStreamStartTime()` method to retrieve the Unix timestamp of the stream start time when available
- Existing `seekToISO8601()` method was enhanced to work with proper HLS stream metadata
- Time broadcasting for continuous stream time updates was already implemented

### 5. Thread Safety
- Multiple mutexes needed to protect different aspects of the decoder state
- Seeking operations must be thread-safe and properly synchronized with playback
- The decoder state must be reset after seeking to clear buffered frames

## Technical Implementation Details

### Rust Implementation
- Enhanced `extract_program_date_time` function to look for multiple HLS date time metadata keys
- Added `get_stream_start_time` method to expose the stream start time to Dart
- Improved ISO 8601 parsing with fallback formats
- Maintained thread safety with proper mutex usage

### Dart Implementation
- Added `getStreamStartTime()` method to the `VideoController` class
- Maintained backward compatibility with existing APIs
- Added proper error handling for cases where stream start time is not available

## Challenges Encountered

1. **Metadata Variability**: Different HLS implementations store the `#EXT-X-PROGRAM-DATE-TIME:` information in different metadata fields, requiring a comprehensive search approach.

2. **Time Synchronization**: Maintaining proper synchronization after seeking operations required careful management of the frame synchronizer.

3. **FFmpeg Integration**: Leveraging FFmpeg's HLS parsing capabilities while ensuring the metadata is properly exposed through the API.

4. **Thread Safety**: Properly managing concurrent access to decoder state during seeking and playback operations.

## Testing Considerations
- HLS streams with `#EXT-X-PROGRAM-DATE-TIME:` should show accurate stream start times
- ISO 8601 seeking should work correctly by calculating relative timestamps
- Time broadcasting should provide continuous updates during playback
- Seeking should properly reset timing and clear buffered frames

## Future Improvements
- Consider adding more robust HLS manifest parsing for cases where FFmpeg doesn't automatically extract the metadata
- Enhance error reporting when seeking fails due to missing or invalid timestamp data
- Add support for live HLS streams with rolling window seeks