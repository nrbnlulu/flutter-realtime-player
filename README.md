# flutter realtime player
GStreamer based player, optimized for realtime streams

## Installation

- Install [rust](https://rustup.rs/)
- Make sure GStreamer is available in the system to link against it

## Android

Android cross-compilation is currently supported from Linux hosts only.
If you want to cross compile the library and run example on Android, follow instructions below:

- make sure you have Rust proper toolchains ready to compile for Android

    ```
    rustup target add \
                aarch64-linux-android \
                armv7-linux-androideabi \
                x86_64-linux-android
    ```

- Download [GStreamer for Android](https://gstreamer.freedesktop.org/download/#android) and extract it in a directory on your system
- Install Android SDK and Android NDK r29
- We need to set environment variables through a file in your home directory: `$HOME/cross_build.env`
- Set `ANDROID_NDK_HOME` to your Android NDK r29 path (e.g. `/home/user/Android/Sdk/ndk/29.0.14206865`)
- Set `GSTREAMER_ROOT_ANDROID` (eg. `/home/user/gstreamer-1.0-android-universal-1.28.1`)
- You can use [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) (eg. `cargo ndk -t x86_64 -P 35`) to make sure the rust side is building successfully

To run the example through the local Android Rust shell:

```sh
task run-example-rust-android                         # Select the connected device
task run-example-rust-android -- --device SERIAL      # Choose among several devices
task run-example-rust-android -- --build-only         # Build an ARM64 APK without a device
```

This uses the debug Android engine under `LOCAL_ENGINE_SRC_PATH` and the native
GameActivity scaffold, with this player's Rust plugin registered. It requires a
configured `android_debug_unopt_arm64` or `android_debug_unopt_x64` engine. The
task rebuilds the engine before compiling the Rust runner so their native APIs
match, and checks startup logs before reporting success. Use
`--engine NAME` to select another debug engine output. Build files and the APK
are kept under `example/build/rust-android/`.

### Experimental Vulkan Video probe

The first migration step is a standalone decoder probe. It reports Vulkan Video
extensions, codec-specific decode queue families, and GStreamer H.264/H.265
decoder availability. With an elementary stream, it decodes up to 120 frames and
checks both the negotiated caps and every buffer's memory type without mapping
pixels into CPU memory.

Configure `~/cross_build.env` as above and connect an Android device with USB
debugging enabled. The probe defaults to `arm64-v8a`; set `ANDROID_ABI` to
`armeabi-v7a` or `x86_64` for another target. `ANDROID_SERIAL` selects an adb device.

```sh
task android-vulkan-probe                       # List driver and decoder capabilities
task android-vulkan-probe -- h264 sample.h264    # Raw H.264 elementary stream
task android-vulkan-probe -- h265 sample.h265    # Raw H.265 elementary stream
task android-vulkan-probe -- --build-only       # Cross-compile without a device
```

MP4/MKV containers are not accepted by this probe. Driver extension availability
alone does not establish support for a stream's profile, bit depth or resolution;
use representative streams for each codec.

The `android-vulkan` Cargo feature links and registers the Vulkan plugin on
Android. It does **not** switch Flutter playback to Vulkan decoding. The probe
tests decoding into Vulkan images; sharing those images and their synchronization
with the Flutter shell still needs a native image import API. The current shell
API exposes its own RGBA texture slots rather than GStreamer-owned images.

The local GStreamer Android 1.28.3 bundle has
`GST_VULKAN_HAVE_VIDEO_EXTENSIONS=0` in `gst/vulkan/gstvkconfig.h`. Its Vulkan
plugin supports rendering, but omits the video decoders. The build warns about
this and still allows the probe to inspect the device. Rebuilding GStreamer
1.28.3 requires Vulkan headers **1.4.317 or newer**,
`-Dgst-plugins-bad:vulkan=enabled` and
`-Dgst-plugins-bad:vulkan-video=enabled`. Verify that the installed header reports
`GST_VULKAN_HAVE_VIDEO_EXTENSIONS=1` and test decoder availability on the device.
See GStreamer's [Vulkan build checks](https://github.com/GStreamer/gstreamer/blob/1.28.3/subprojects/gst-plugins-bad/gst-libs/gst/vulkan/meson.build)
and [decoder build sources](https://github.com/GStreamer/gstreamer/blob/1.28.3/subprojects/gst-plugins-bad/ext/vulkan/meson.build).
