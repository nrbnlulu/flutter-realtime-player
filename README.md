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
- Tell cargo where the NDK and the GStreamer SDK are, in `~/.cargo/config.toml`
  (machine-specific paths, so keep them out of the repo). `rlutter run` invokes cargo
  directly, so it picks these up.

    ```toml
    [env]
    ANDROID_NDK_HOME = "/home/user/Android/Sdk/ndk/29.0.14206865"
    GSTREAMER_ROOT_ANDROID = "/home/user/gstreamer-1.0-android-universal-1.28.1"

    # lets the pkg-config crate cross compile and find the Android .pc files
    PKG_CONFIG_ALLOW_CROSS = "1"
    PKG_CONFIG_LIBDIR_aarch64_linux_android = "/home/user/gstreamer-1.0-android-universal-1.28.1/arm64/lib/pkgconfig"
    # other ABIs: PKG_CONFIG_LIBDIR_armv7_linux_androideabi (.../armv7/...),
    #             PKG_CONFIG_LIBDIR_x86_64_linux_android (.../x86_64/...)

    # NDK toolchain for C/C++ build scripts (cc-rs) and linking; repeat per target
    CC_aarch64_linux_android = "/home/user/Android/Sdk/ndk/29.0.14206865/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
    CXX_aarch64_linux_android = "/home/user/Android/Sdk/ndk/29.0.14206865/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang++"
    AR_aarch64_linux_android = "/home/user/Android/Sdk/ndk/29.0.14206865/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"

    [target.aarch64-linux-android]
    linker = "/home/user/Android/Sdk/ndk/29.0.14206865/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
    ```

  (`24` is the Android API level and should match the app's `minSdk`.)
  The `[env]` variables can also be `export`ed in your shell instead.
- You can use [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk) (eg. `cargo ndk -t x86_64 -P 35`) to make sure the rust side is building successfully
