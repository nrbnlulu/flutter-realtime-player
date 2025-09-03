### Flutter realtime player
This is a library that focuses on realtime streams support for flutter using ffmpeg.

### Architecture
- ./rust - utelizes ffmpeg to create video streams, irondash_texture is then used to pass the pixel buffers to flutter.
- ./rust/api - dart-rust interop using `flutter_rust_bridge_codegen`

### commands
- `flutter_rust_bridge_codegen generate` - generates rust bindings to dart, use this when you changed the api. 