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
    )
    .map_err(|_| anyhow::anyhow!("Failed to register FlutterTextureSink"))
}

pub fn init() -> anyhow::Result<()> {
    gst::init()?;
    register(None)
}

fn create_flutter_texture(engine_handle: i64) -> anyhow::Result<(ArcSendableTexture, i64)> {
    utils::invoke_on_platform_main_thread(move || {
        let provider = Arc::new(GLTextureSource::init_gl_context().unwrap());
        let texture =
            irondash_texture::Texture::new_with_provider(engine_handle, provider.clone())?;
        let tx_id = texture.id();
        let sendable_texture = texture.into_sendable_texture();
        Ok((sendable_texture, tx_id))
    })
}

pub fn testit(engine_handle: i64) -> anyhow::Result<i64> {
    let (texture, id) = create_flutter_texture(engine_handle)?;
    let src = utils::make_element("videotestsrc", None)?;
    let sink = utils::make_element("fluttertexturesink", None)?;
    let (tx, rx) = flume::unbounded();

    let fl_texture_wrapper = imp::FlTextureWrapper::new(id, tx);
    sink.downcast_ref::<FlutterTextureSink>()
        .unwrap()
        .imp()
        .set_fl_texture(fl_texture_wrapper);

    let pipeline = gst::Pipeline::new();
    pipeline.add_many(&[&src, &sink])?;
    gst::Element::link_many(&[&src, &sink])?;

    Ok(id)
}
