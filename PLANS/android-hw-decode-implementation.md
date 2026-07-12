# Android hardware decode implementation

## Goal

Implement the zero-copy Android output path described in `android-hw-decode.md` while keeping the existing software RGBA appsink path for non-Android targets.

## Plan

1. Add Android-only GStreamer OpenGL plugin linkage and registration.
2. Export the JNI provider symbols required by GStreamer's `androidmedia` plugin.
3. Add an Android video output backed by `irondash_texture::NativeWindow`.
4. Route Android WSC-RTP and playbin video into `glimagesink` via `GstVideoOverlay`.
5. Add the Android Flutter plugin shell and the GStreamer `GstAmcOnFrameAvailableListener` Java class.
6. Run formatting/checks that are feasible in the local environment.

