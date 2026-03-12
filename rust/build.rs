fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "android" {
        let gstreamer_root = std::env::var("GSTREAMER_ROOT_ANDROID")
            .expect("GSTREAMER_ROOT_ANDROID not set");

        println!("cargo:rustc-link-search=native={}/lib", gstreamer_root);

        // --- Core Transitive Dependencies (The "Infrastructure") ---
    println!("cargo:rustc-link-lib=static=ffi");        // For ffi_type_void
    println!("cargo:rustc-link-lib=static=orc-0.4");    // For SIMD/Orc symbols
    println!("cargo:rustc-link-lib=static=intl");       // For libintl_bindtextdomain
    println!("cargo:rustc-link-lib=static=iconv");      // For libiconv_open
    println!("cargo:rustc-link-lib=static=pcre2-8");    // Required by newer GLib for regex
    
    println!("cargo:rustc-link-lib=static=gmodule-2.0"); // For g_module_open
    
    // --- Common Plugin Support (Add as needed based on your pipeline) ---
    println!("cargo:rustc-link-lib=static=gstvideo-1.0");
    println!("cargo:rustc-link-lib=static=gstaudio-1.0");
    println!("cargo:rustc-link-lib=static=gstapp-1.0");
    }

    println!("cargo:rerun-if-changed=build.rs");
}