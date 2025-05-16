# expiriments with realtime video decoding for flutter.

currently there are two branches that are useable.
- `main` uses ffmpeg software decoder. recommended for live streams. sometimes frames are dropped this need to be fixed.
- `software-decoder` which uses GStreamer pipeline and an `appsink`. it is more reliable then `main` though I had an issue with RTSP streaming for low-bandwidth connections.
