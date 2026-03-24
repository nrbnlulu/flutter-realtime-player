# flutter realtime player
GStreamer based player, optimized for realtime streams

## Installation

- Install [rust](https://rustup.rs/)
- Make sure GStramer is available in the system to link against it

## Android

If you want to cross compile the library and run example on Android, follow instructions below:

- make sure you have Rust proper toolchains ready to compile for Android

    ```
    rustup target add \
                aarch64-linux-android \
                armv7-linux-androideabi \
                x86_64-linux-android \
                i686-linux-android
    ```

- Download [GStreamer for Android](https://gstreamer.freedesktop.org/download/#android) and extract it in a directory on your system
- Download Android SDK
- Set `PKG_CONFIG_SYSROOT_DIR` and `GSTREAMER_ROOT_ANDROID` environment variables in the `.env` file in project root directory
- You can use [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) (eg. `cargo ndk -t x86_64 -P 35` for running on emulator with Android 35 API) to make sure the rust side is building successfully
