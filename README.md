# flutter realtime player
GStreamer based player, optimized for realtime streams

## Installation

- Install [rust](https://rustup.rs/)
- Make sure GStreamer is available in the system to link against it

### Linux bundle compatibility

Linux release bundles include GStreamer, its plugins, GLib, PCRE2, libmount,
and libblkid from the build machine. Build release artifacts on the oldest
Linux distribution the application supports because glibc cannot safely be
bundled.

For portable release builds, set the oldest supported glibc version in
`$HOME/cross_build.env`. The build hook then rejects libraries requiring a
newer version. Leave it unset for local development and `flutter run`:

```text
FLUTTER_REALTIME_PLAYER_LINUX_GLIBC_MAX=2.35
```

Install `patchelf` on the Linux build machine. The hook adds an `$ORIGIN`
runpath to bundled libraries so GLib loads the matching bundled PCRE2, libmount,
and libblkid instead of ABI-incompatible copies from the target system.

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
