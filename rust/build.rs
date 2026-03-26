fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os == "android" {
        let mut gstreamer_arch_dir = target_arch.clone();

        println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
        println!("cargo:rerun-if-env-changed=GSTREAMER_ROOT_ANDROID");
        
        let ndk_home = std::env::var("ANDROID_NDK_HOME")
            .expect("ANDROID_NDK_HOME not set");
        if ndk_home.is_empty() {
            panic!("ANDROID_NDK_HOME environment variable is empty");
        }

        if target_arch == "aarch64" {
            // we need libclang_rt.builtins-aarch64-android.a for compiler builtins on Android arm64
            gstreamer_arch_dir = "arm64".to_string();
        
            // we need libclang_rt.builtins-aarch64-nadoird.a for compiler builtins on Android arm64
            let clang_version = "21"; // Standard for r29
            
            let runtime_path = format!(
                "{}/toolchains/llvm/prebuilt/linux-x86_64/lib/clang/{}/lib/linux",
                ndk_home, clang_version
            );

            println!("cargo:rustc-link-search=native={}", runtime_path);
            println!("cargo:rustc-link-lib=static=clang_rt.builtins-{}-android", target_arch);
        }

        let gstreamer_root = std::env::var("GSTREAMER_ROOT_ANDROID")
            .expect("GSTREAMER_ROOT_ANDROID not set");

        if gstreamer_root.is_empty() {
            panic!("GSTREAMER_ROOT_ANDROID environment variable is empty");
        }

        println!("cargo:rustc-link-search=native={}/{}/lib", gstreamer_root, gstreamer_arch_dir);
        println!("cargo:rustc-link-search=native={}/{}/lib/gstreamer-1.0", gstreamer_root, gstreamer_arch_dir);

        // --- Core Transitive Dependencies (The "Infrastructure") ---
        println!("cargo:rustc-link-lib=static=ffi");        // For ffi_type_void
        println!("cargo:rustc-link-lib=static=orc-0.4");    // For SIMD/Orc symbols
        println!("cargo:rustc-link-lib=static=intl");       // For libintl_bindtextdomain
        println!("cargo:rustc-link-lib=static=iconv");      // For libiconv_open
        println!("cargo:rustc-link-lib=static=pcre2-8");    // Required by newer GLib for regex
        
        // gstreamer
        println!("cargo:rustc-link-lib=static=gstreamer-1.0");
        println!("cargo:rustc-link-lib=static=glib-2.0");
        println!("cargo:rustc-link-lib=static=gobject-2.0");

        // gstreamer-app
        println!("cargo:rustc-link-lib=static=gstbase-1.0");
        println!("cargo:rustc-link-lib=static=gstapp-1.0");

        // gstreamer-video
        println!("cargo:rustc-link-lib=static=gstvideo-1.0");

        println!("cargo:rustc-link-lib=static=gmodule-2.0"); // For g_module_open
        
        // --- Common Plugin Support (Add as needed based on your pipeline) ---

        println!("cargo:rustc-link-lib=static=clang_rt.builtins-aarch64-android"); // For compiler builtins on Android arm64
        

        println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition"); // for JNI_OnLoad conflicts
    }

    println!("cargo:rerun-if-changed=build.rs");
}