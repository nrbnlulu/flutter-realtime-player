// Copyright 2026 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let engine_dir = manifest_dir.join("../.dart_tool/flutter_rs/sdk/lib");
    let engine_library = engine_dir.join("libflutter_rust_engine.so");
    assert!(
        engine_library.is_file(),
        "Flutter Rust engine library is missing: {}. Run `flutter pub get` to install it.",
        engine_library.display()
    );
    println!("cargo:rerun-if-changed={}", engine_library.display());
    println!("cargo:rustc-link-search=native={}", engine_dir.display());
    println!("cargo:rustc-link-lib=dylib=flutter_rust_engine");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", engine_dir.display());
    println!("cargo:rustc-link-arg=-Wl,--export-dynamic");
}
