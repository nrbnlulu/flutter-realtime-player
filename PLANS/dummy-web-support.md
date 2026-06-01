# Dummy web support

## Goal

Allow applications using `flutter_realtime_player` to build for Flutter web without
building or loading Rust. The web implementation is intentionally inert and does
not render a real stream.

## Plan

1. Split the public lifecycle functions into native and web implementations.
2. Split the video player implementation by platform and provide an API-compatible
   dummy controller and widget for web.
3. Skip Rust native asset compilation when the build target is web.
4. Format, analyze, and build the example for web to verify the dummy path.
