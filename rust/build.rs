fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os == "android" {
        let mut gstreamer_arch_dir = target_arch.clone();

        println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
        println!("cargo:rerun-if-env-changed=GSTREAMER_ROOT_ANDROID");

        let ndk_home = std::env::var("ANDROID_NDK_HOME").expect("ANDROID_NDK_HOME not set");
        if ndk_home.is_empty() {
            panic!("ANDROID_NDK_HOME environment variable is empty");
        }

        if target_arch == "aarch64" {
            // we need libclang_rt.builtins-aarch64-android.a for compiler builtins on Android arm64
            gstreamer_arch_dir = "arm64".to_string();

            // we need libclang_rt.builtins-aarch64-andoird.a for compiler builtins on Android arm64
            let clang_version = "21"; // Standard for r29

            let runtime_path = format!(
                "{}/toolchains/llvm/prebuilt/linux-x86_64/lib/clang/{}/lib/linux",
                ndk_home, clang_version
            );

            println!("cargo:rustc-link-search=native={}", runtime_path);
            println!(
                "cargo:rustc-link-lib=static=clang_rt.builtins-{}-android",
                target_arch
            );
        }

        let gstreamer_root =
            std::env::var("GSTREAMER_ROOT_ANDROID").expect("GSTREAMER_ROOT_ANDROID not set");

        if gstreamer_root.is_empty() {
            panic!("GSTREAMER_ROOT_ANDROID environment variable is empty");
        }

        println!(
            "cargo:rustc-link-search=native={}/{}/lib",
            gstreamer_root, gstreamer_arch_dir
        );
        println!(
            "cargo:rustc-link-search=native={}/{}/lib/gstreamer-1.0",
            gstreamer_root, gstreamer_arch_dir
        );

        // --- Core Transitive Dependencies (The "Infrastructure") ---
        println!("cargo:rustc-link-lib=static=ffi"); // For ffi_type_void
        println!("cargo:rustc-link-lib=static=orc-0.4"); // For SIMD/Orc symbols
        println!("cargo:rustc-link-lib=static=intl"); // For libintl_bindtextdomain
        println!("cargo:rustc-link-lib=static=iconv"); // For libiconv_open
        println!("cargo:rustc-link-lib=static=pcre2-8"); // Required by newer GLib for regex

        // gstreamer
        println!("cargo:rustc-link-lib=static=gstreamer-1.0");
        println!("cargo:rustc-link-lib=static=glib-2.0");
        println!("cargo:rustc-link-lib=static=gobject-2.0");

        // gstreamer-app
        println!("cargo:rustc-link-lib=static=gstbase-1.0");
        println!("cargo:rustc-link-lib=static=gstapp-1.0");

        // gstreamer-video
        println!("cargo:rustc-link-lib=static=gstvideo-1.0");
        println!("cargo:rustc-link-lib=static=gstgl-1.0");
        println!("cargo:rustc-link-lib=static=gstcodecparsers-1.0");
        println!("cargo:rustc-link-lib=static=gstcodecs-1.0");
        println!("cargo:rustc-link-lib=static=gstpbutils-1.0");
        println!("cargo:rustc-link-lib=static=gsttag-1.0");
        println!("cargo:rustc-link-lib=static=gstriff-1.0");
        println!("cargo:rustc-link-lib=static=gstrtp-1.0");
        println!("cargo:rustc-link-lib=static=gstrtsp-1.0");
        println!("cargo:rustc-link-lib=static=gstsdp-1.0");
        println!("cargo:rustc-link-lib=static=gstallocators-1.0");
        println!("cargo:rustc-link-lib=static=gstnet-1.0");
        println!("cargo:rustc-link-lib=static=gstisoff-1.0");

        println!("cargo:rustc-link-lib=static=gmodule-2.0");

        // --- GStreamer plugins (must be statically linked on Android) ---

        // Core: without these, NO elements work at all
        println!("cargo:rustc-link-lib=static=gstcoreelements");
        println!("cargo:rustc-link-lib=static=gsttypefindfunctions");
        println!("cargo:rustc-link-lib=static=gstplayback");

        // Networking / HTTP(S) source
        println!("cargo:rustc-link-lib=static=gstsoup");
        println!("cargo:rustc-link-lib=static=soup-3.0");
        println!("cargo:rustc-link-lib=static=gio-2.0");
        println!("cargo:rustc-link-lib=static=ssl");
        println!("cargo:rustc-link-lib=static=crypto");
        println!("cargo:rustc-link-lib=static=nghttp2");
        println!("cargo:rustc-link-lib=static=psl");
        println!("cargo:rustc-link-lib=static=z");
        println!("cargo:rustc-link-lib=static=bz2");

        // Software decoders (ffmpeg-based, covers VP8/VP9/H264/H265/AAC/etc.)
        println!("cargo:rustc-link-lib=static=gstlibav");
        println!("cargo:rustc-link-lib=static=avcodec");
        println!("cargo:rustc-link-lib=static=avformat");
        println!("cargo:rustc-link-lib=static=avfilter");
        println!("cargo:rustc-link-lib=static=avutil");
        println!("cargo:rustc-link-lib=static=swresample");
        println!("cargo:rustc-link-lib=static=swscale");
        println!("cargo:rustc-link-lib=static=x264");

        // Android hardware decoder (MediaCodec)
        println!("cargo:rustc-link-lib=static=gstandroidmedia");

        // Video color conversion (RGBA output from NV12/I420/etc.)
        println!("cargo:rustc-link-lib=static=gstvideoconvertscale");

        // Audio (needed by playbin3 even if muted)
        println!("cargo:rustc-link-lib=static=gstaudioconvert");
        println!("cargo:rustc-link-lib=static=gstaudioresample");
        println!("cargo:rustc-link-lib=static=gstvolume");
        println!("cargo:rustc-link-lib=static=gstaudiomixer");
        println!("cargo:rustc-link-lib=static=gstaudio-1.0");

        // Autodetect sinks (playbin3 uses these for audio/video sink selection)
        println!("cargo:rustc-link-lib=static=gstautodetect");

        // RTSP/RTP/UDP/TCP/SDP (required for rtsp:// and rtp:// URIs)
        println!("cargo:rustc-link-lib=static=gstrtsp");
        println!("cargo:rustc-link-lib=static=gstrtp");
        println!("cargo:rustc-link-lib=static=gstrtpmanager");
        println!("cargo:rustc-link-lib=static=gstudp");
        println!("cargo:rustc-link-lib=static=gsttcp");
        println!("cargo:rustc-link-lib=static=gstsdpelem");

        // Photography plugin — pulled in by androidmedia at link time
        println!("cargo:rustc-link-lib=static=gstphotography-1.0");

        // Android audio sink
        println!("cargo:rustc-link-lib=static=gstopensles");
        println!("cargo:rustc-link-lib=OpenSLES"); // Android system OpenSL ES
        // Android system libs required by GStreamer (always present on device)
        println!("cargo:rustc-link-lib=atomic");
        println!("cargo:rustc-link-lib=log");

        println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition"); // for JNI_OnLoad conflicts
    }

    if target_os == "linux" {
        // Allow libflutter_realtime_player.so to find bundled GStreamer libs in its own directory.
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
