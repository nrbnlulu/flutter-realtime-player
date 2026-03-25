# flutter realtime player
GStreamer based player, optimized for realtime streams

## Installation

- Install [rust](https://rustup.rs/)
- Make sure GStreamer is available in the system to link against it

### Android

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
- We need to set environment variables through a file in your home directory: `$HOME/frtp_build.env`
- Set `ANDROID_NDK_HOME` (eg. `/home/user/Android/Sdk/ndk/29.0.14206865`)
- Set `GSTREAMER_ROOT_ANDROID` (eg. `/home/user/gstreamer-1.0-android-universal-1.28.1`)
- You can use [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) (eg. `cargo ndk -t x86_64 -P 35`) to make sure the rust side is building successfully

