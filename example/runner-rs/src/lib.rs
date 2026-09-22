// Copyright 2026 The Flutter Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod flutter_plugins;

use flutter_plugin_sdk::{PluginRegistrar, Result};

pub fn register_application(registrar: &mut PluginRegistrar) -> Result<()> {
    flutter_plugins::register_plugins(registrar)
}
