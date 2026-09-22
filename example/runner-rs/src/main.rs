// Copyright 2026 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{env, path::PathBuf, process::ExitCode};

use flutter_shell_winit::{ShellConfig, run_application};
use flutter_realtime_player_example::register_application;

fn main() -> ExitCode {
    let Some(assets_path) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: flutter_realtime_player_example <flutter-assets-directory>");
        return ExitCode::FAILURE;
    };
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let icu_data_path = manifest_dir.join("../.dart_tool/flutter_rs/sdk/lib/icudtl.dat");
    match run_application(
        ShellConfig {
            assets_path: assets_path.to_string_lossy().into_owned(),
            icu_data_path: icu_data_path.to_string_lossy().into_owned(),
            ..ShellConfig::default()
        },
        register_application,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("failed to run flutter_realtime_player_example: {error}");
            ExitCode::FAILURE
        }
    }
}
