# flutter realtime player
GStreamer based player, optimized for realtime streams

## Installation

- Install [rust](https://rustup.rs/)
- Make sure GStramer is available in the system to link against it

## Android

If you want to cross compile the library and run example on Android, follow instructions below:

- make sure you have rust proper toolchains ready to compile for Android

    ```
    rustup target add \
                aarch64-linux-android \
                armv7-linux-androideabi \
                x86_64-linux-android \
                i686-linux-android
    ```

- Download [GStreamer for Android](https://gstreamer.freedesktop.org/download/#android) and extract it in a directory on your system
- Download Android SDK
- Set `PKG_CONFIG_SYSROOT_DIR` and `GSTREAMER_ROOT_ANDROID` environment variables on your system or if you use VSCode Run option you can set them here in [Cargo config](./.cargo/config.toml) file
- you can use [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) to make sure the rust side is building successfully
