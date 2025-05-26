# experiments with realtime video decoding for flutter.

currently there are two branches that are useable.
- `main` uses ffmpeg software decoder. recommended for live streams. sometimes frames are dropped this need to be fixed.
- `software-decoder` which uses GStreamer pipeline and an `appsink`. it is more reliable then `main` though I had an issue with RTSP streaming for low-bandwidth connections.


## Installation

### Linux
install ffmpeg and pkg-config, I haven't needed anything else.

### Windows
1. download https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-7.0.2-full_build-shared.7z
2. extract and add `FFMPEG_LIB_DIR` `FFMPEG_INCLUDE_DIR` `FFMPEG_DIR` to your env vars
3. add the bin directory to `PATH`
4. should be g2g