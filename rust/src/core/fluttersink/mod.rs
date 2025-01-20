//
// Copyright (C) 2021 Bilal Elmoussaoui <bil.elmoussaoui@gmail.com>
// Copyright (C) 2021 Jordan Petridis <jordan@centricular.com>
// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

// ported from gstreamer-rs-plugins gtk4sink

use glib::types::StaticType;

mod frame;
pub mod gltexture;
pub(super) mod imp;
mod utils;
enum SinkEvent {
    FrameChanged,
}

glib::wrapper! {
    pub struct FlutterTextureSink(ObjectSubclass<imp::FlutterTextureSink>)
    @extends gst_video::VideoSink, gst_base::BaseSink, gst::Element, gst::Object;
}

impl FlutterTextureSink {
    pub fn new(name: Option<&str>) -> Self {
        gst::Object::builder().name_if_some(name).build().unwrap()
    }
}

fn register(plugin: Option<&gst::Plugin>) -> anyhow::Result<()> {
    gst::Element::register(
        plugin,
        "fluttertexturesink",
        gst::Rank::NONE,
        FlutterTextureSink::static_type(),
    ).map_err(|_| anyhow::anyhow!("Failed to register FlutterTextureSink"))
}

pub fn init() -> anyhow::Result<()> {
    gst::init()?;
    register(None)
}

