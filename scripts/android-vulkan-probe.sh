#!/usr/bin/env bash
set -euo pipefail

# Uses the same SDK configuration as the Android player build.
if [[ -f "$HOME/cross_build.env" ]]; then
    set -a
    source "$HOME/cross_build.env"
    set +a
fi
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME in ~/cross_build.env}"
: "${GSTREAMER_ROOT_ANDROID:?Set GSTREAMER_ROOT_ANDROID in ~/cross_build.env}"

build_only=false
if [[ "${1:-}" == --build-only ]]; then
    build_only=true
    shift
fi
if [[ $# -ne 0 && ( $# -ne 2 || ( "$1" != h264 && "$1" != h265 ) ) ]]; then
    echo "Usage: $0 [--build-only] [h264|h265 ELEMENTARY_STREAM]" >&2
    exit 2
fi
if [[ $# -eq 2 && ! -f "$2" ]]; then
    echo "Input file not found: $2" >&2
    exit 2
fi

case "${ANDROID_ABI:-arm64-v8a}" in
    arm64-v8a) rust_target=aarch64-linux-android; clang_target=aarch64-linux-android; gst_arch=arm64 ;;
    armeabi-v7a) rust_target=armv7-linux-androideabi; clang_target=armv7a-linux-androideabi; gst_arch=armv7 ;;
    x86_64) rust_target=x86_64-linux-android; clang_target=x86_64-linux-android; gst_arch=x86_64 ;;
    *) echo "Unsupported ANDROID_ABI: $ANDROID_ABI" >&2; exit 2 ;;
esac

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
toolchain="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
gst_prefix="$GSTREAMER_ROOT_ANDROID/$gst_arch"
compiler="$toolchain/${clang_target}24-clang"
linker_var="CARGO_TARGET_${rust_target^^}_LINKER"
linker_var=${linker_var//-/_}

[[ -x "$compiler" ]] || { echo "NDK compiler not found: $compiler" >&2; exit 1; }
[[ -d "$gst_prefix/lib/pkgconfig" ]] || { echo "GStreamer SDK not found: $gst_prefix" >&2; exit 1; }

# Explicit target pkg-config paths prevent linking host GStreamer into Android.
env "$linker_var=$compiler" \
    CC="$compiler" CXX="$toolchain/${clang_target}24-clang++" AR="$toolchain/llvm-ar" \
    PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_SYSROOT_DIR=/ \
    PKG_CONFIG_PATH="$gst_prefix/lib/pkgconfig" PKG_CONFIG_LIBDIR="$gst_prefix/lib/pkgconfig" \
    CARGO_TARGET_DIR="$repo_root/target" \
    cargo build --locked --manifest-path "$repo_root/Cargo.toml" \
    --target "$rust_target" --example android_vulkan_probe --features android-vulkan

binary="$repo_root/target/$rust_target/debug/examples/android_vulkan_probe"
if $build_only; then
    echo "Built $binary"
    exit 0
fi

adb get-state >/dev/null
remote_binary=/data/local/tmp/flutter-realtime-player-vulkan-probe
adb push "$binary" "$remote_binary"
adb shell chmod 755 "$remote_binary"
if [[ $# -eq 0 ]]; then
    adb shell "$remote_binary" --list
else
    remote_input="/data/local/tmp/flutter-realtime-player-vulkan-probe.$1"
    adb push "$2" "$remote_input"
    adb shell "$remote_binary" "$1" "$remote_input"
fi
