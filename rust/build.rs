fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    if target_os == "android" {
        let gstreamer_root = std::env::var("GSTREAMER_ROOT_ANDROID")
            .expect("GSTREAMER_ROOT_ANDROID not set");

        println!("cargo:rustc-link-search=native={}/{}/lib", gstreamer_root, target_arch);
        println!("cargo:rustc-link-search=native={}/{}/lib/gstreamer-1.0", gstreamer_root, target_arch);

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
        

        println!("cargo:rustc-link-arg=-Wl,--allow-multiple-definition"); // for JNI_OnLoad conflicts
    }

    println!("cargo:rerun-if-changed=build.rs");
}