// Copyright 2026 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// flutter-rust-runner-version: 6

use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    // Cargo sets PROFILE to "debug" or "release"; rlutter provides a matching
    // engine (JIT vs AOT-capable) under the same name so `cargo build
    // --release` links the engine that can actually run an AOT app.
    let profile = env::var("PROFILE").unwrap();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let platform = match (target_os.as_str(), target_arch.as_str()) {
        ("linux", "x86_64") => "linux-x64",
        ("android", "aarch64") => "android-arm64",
        ("android", "x86_64") => "android-x64",
        ("android", "arm") => "android-arm",
        ("android", "riscv64") => "android-riscv64",
        other => panic!("The Flutter Rust shell does not support {other:?} yet."),
    };
    let variant = format!("{platform}-{profile}");
    let engine_dir = manifest_dir.join("../.dart_tool/flutter_rs/sdk/lib").join(&variant);
    let engine_library = engine_dir.join("libflutter_rust_engine.so");
    assert!(
        engine_library.is_file(),
        "Flutter Rust engine library is missing: {}. Run `rlutter build` to install it.",
        engine_library.display()
    );
    println!("cargo:rerun-if-changed={}", engine_library.display());
    println!("cargo:rustc-link-search=native={}", engine_dir.display());
    println!("cargo:rustc-link-lib=dylib=flutter_rust_engine");
    if target_os == "android" {
        // The engine ships inside the APK's jniLibs/<abi>/ next to this
        // cdylib, so resolve it relative to wherever this library is loaded.
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    } else {
        // `$ORIGIN/libs` first so a bundle produced by `rlutter build linux`
        // (engine and native plugin libraries copied into `<exe dir>/libs`; not
        // `lib/`, which plugins such as GStreamer wrappers treat as a bundled
        // third-party library directory) is relocatable; the absolute engine
        // path remains as the in-tree fallback.
        // --disable-new-dtags emits DT_RPATH rather than DT_RUNPATH, so the
        // path also applies to libraries the engine/Dart FFI dlopens by bare name.
        println!("cargo:rustc-link-arg=-Wl,--disable-new-dtags");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/libs");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", engine_dir.display());
        println!("cargo:rustc-link-arg=-Wl,--export-dynamic");
    }
    println!("cargo::rustc-env=FLUTTER_RS_PROFILE={profile}");
    println!("cargo::rustc-env=FLUTTER_RS_ENGINE_VARIANT={variant}");
}
