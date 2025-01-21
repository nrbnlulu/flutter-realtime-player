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

use frame::Frame;
use glib::{object::Cast, subclass::types::ObjectSubclassIsExt, types::StaticType};
use gltexture::GLTextureSource;
use gst::prelude::{ElementExt, ElementExtManual, GstBinExt, GstBinExtManual, GstObjectExt};
use imp::ArcSendableTexture;
use log::{debug, error, info};

mod frame;
pub mod gltexture;
pub(super) mod imp;
pub mod utils;
pub(crate) enum SinkEvent {
    FrameChanged(Frame),
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
pub(crate) type FrameSender = flume::Sender<SinkEvent>;

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
    let gl = false;

    let pipeline = gst::Pipeline::new();
    let flsink = utils::make_element("fluttertexturesink", None)?;
    let overlay = gst::ElementFactory::make("clockoverlay")
        .property("font-desc", "Monospace 42")
        .build()
        .unwrap();

    let (src, sink) = if gl {
        unimplemented!("GL not supported yet");
        // let src = utils::make_element("gltestsrc", None)?;

        // let sink = gst::ElementFactory::make("glsinkbin")
        //     .property("sink", &flsink)
        //     .build()
        //     .unwrap();
    } else {
        let src = gst::ElementFactory::make("videotestsrc").build().unwrap();

        let sink = gst::Bin::default();
        let convert = gst::ElementFactory::make("videoconvert").build().unwrap();

        sink.add(&convert).unwrap();
        sink.add(&flsink).unwrap();
        convert.link(&flsink).unwrap();

        sink.add_pad(&gst::GhostPad::with_target(&convert.static_pad("sink").unwrap()).unwrap())
            .unwrap();
        (src, sink.upcast())
    };

    let fl_texture_wrapper = imp::FlutterConfig::new(id, tx);
    flsink
        .downcast_ref::<FlutterTextureSink>()
        .unwrap()
        .imp()
        .set_fl_config(fl_texture_wrapper);

    
    pipeline.add_many([&src, &overlay, &sink]).unwrap();
    let caps = gst_video::VideoCapsBuilder::new()
        .width(640)
        .height(480)
        .any_features()
        .build();

    src.link_filtered(&overlay, &caps).unwrap();
    overlay.link(&sink).unwrap();
    let bus = pipeline.bus().unwrap();

    pipeline
        .set_state(gst::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");
    let bus_watch = bus
        .add_watch_local(move |_, msg| {
            use gst::MessageView;

            match msg.view() {
                MessageView::Info(info) => {
                    if let Some(s) = info.structure() {
                        info!("Info: {:?}", s);
                    }
                }
                MessageView::Eos(..) => info!("End of stream"),
                MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                }
                _ => (),
            };

            glib::ControlFlow::Continue
        })
        .expect("Failed to add bus watch");
    Ok(id)
}
