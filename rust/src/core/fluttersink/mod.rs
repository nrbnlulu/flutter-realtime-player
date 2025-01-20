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

use std::sync::Arc;

use glib::{object::Cast, subclass::types::ObjectSubclassIsExt, types::StaticType};
use gltexture::GLTextureSource;
use gst::prelude::GstBinExtManual;
use imp::ArcSendableTexture;
use log::{debug, info};

mod frame;
pub mod gltexture;
pub(super) mod imp;
pub mod utils;
pub(crate) enum SinkEvent {
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
    )
    .map_err(|_| anyhow::anyhow!("Failed to register FlutterTextureSink"))
}

pub fn init() -> anyhow::Result<()> {
    gst::init()?;
    register(None)
}
pub type FrameSender = flume::Sender<SinkEvent>;

fn create_flutter_texture(
    engine_handle: i64,
) -> anyhow::Result<(ArcSendableTexture, i64, FrameSender)> {
    return utils::invoke_on_platform_main_thread(move || {
        debug!("Creating Flutter texture");
        let (tx, rx) = flume::bounded(3);

        let provider = Arc::new(GLTextureSource::new(rx).unwrap());
        let texture =
            irondash_texture::Texture::new_with_provider(engine_handle, provider.clone())?;
        debug!("Created Flutter texture with id {}", texture.id());
        let tx_id = texture.id();
        let sendable_texture = texture.into_sendable_texture();
        Ok((sendable_texture, tx_id, tx))
    });
}

pub fn testit(engine_handle: i64) -> anyhow::Result<i64> {
    let (_, id, tx) = create_flutter_texture(engine_handle)?;
    let src = utils::make_element("videotestsrc", None)?;
    let sink = utils::make_element("fluttertexturesink", None)?;

    let fl_texture_wrapper = imp::FlutterConfig::new(id, tx);
    sink.downcast_ref::<FlutterTextureSink>()
        .unwrap()
        .imp()
        .set_fl_config(fl_texture_wrapper);

    let pipeline = gst::Pipeline::new();
    pipeline.add_many(&[&src, &sink])?;
    gst::Element::link_many(&[&src, &sink])?;

    Ok(id)
}
