pub(super) mod sink;
pub mod utils;
use gst::{
    ffi::{gst_context_get_structure, gst_element_set_context, gst_structure_to_string},
    glib::{self},
    prelude::{ElementExtManual, GstBinExtManual, PadExt},
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};

use gst::{
    glib::object::{Cast, ObjectExt},
    prelude::{ElementExt, GstObjectExt},
};
use log::{error, info, trace};
use sink::FlutterTextureSink;
use utils::LogErr;

use crate::core::platform::{create_gst_d3d_ctx, NativeTextureProvider};

use super::{platform::NativeRegisteredTexture, types};

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}

lazy_static::lazy_static! {
static ref SESSION_CACHE: Mutex<HashMap<i64, (Arc<NativeTextureProvider>, gst::Pipeline, Arc<NativeRegisteredTexture>)>> = Mutex::new(HashMap::new());
}

pub fn create_new_playable(
    engine_handle: i64,
    video_info: types::VideoInfo,
) -> anyhow::Result<i64> {
    trace!("Initializing flutter sink");
    let initialized_sig = Arc::new(AtomicBool::new(false));
    let initialized_sig_clone = initialized_sig.clone();
    let texture_id;
    let registered_texture;
    let texture_provider;
    let dimensions = video_info.dimensions;
    #[cfg(target_os = "windows")]
    {
        use crate::core::platform::TextureDescriptionProvider2Ext;
        trace!("thread id: {:?}", std::thread::current().id());

        (registered_texture, texture_provider) = utils::invoke_on_platform_main_thread(
            move || -> anyhow::Result<(Arc<NativeRegisteredTexture>, Arc<NativeTextureProvider>)> {
                let texture_provider = NativeTextureProvider::new(engine_handle, dimensions)?;

                let tex_provider_clone = texture_provider.clone();
                let reg_texture = irondash_texture::alternative_api::RegisteredTexture::new(
                    texture_provider,
                    engine_handle,
                )
                .map_err(|e| anyhow::anyhow!("Failed to registered texture: {:?}", e))?;

                Ok((reg_texture, tex_provider_clone))
            },
        )?;
        texture_id = registered_texture.get_texture_id();
    }

    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("urisourcebin")
        .property("uri", &video_info.uri)
        .build()?;
    let demux = gst::ElementFactory::make("qtdemux").build()?;
    let parse = gst::ElementFactory::make("h264parse").build()?;
    let decoder = gst::ElementFactory::make("d3d11h264dec").build()?;
    let convert = gst::ElementFactory::make("d3d11convert").build()?;
    let sink = Arc::new(FlutterTextureSink::new(
        initialized_sig_clone,
        &pipeline,
        texture_provider.clone(),
        registered_texture.clone(),
    )?);
    pipeline.add_many(&[&src, &demux, &parse, &decoder, &convert, &sink.video_sink()])?;
    gst::Element::link_many(&[&parse, &decoder, &convert, &sink.video_sink()])?;

    demux.connect_pad_added(move |demux, src_pad| {
        let parse_sink_pad = parse.static_pad("sink").unwrap();
        if !parse_sink_pad.is_linked() {
            demux
                .link_pads(Some(src_pad.name().as_str()), &parse, Some("sink"))
                .unwrap();
        }
    });

    src.connect_pad_added(move |src, src_pad| {
        let demux_sink_pad = demux.static_pad("sink").unwrap();
        if !demux_sink_pad.is_linked() {
            src.link_pads(Some(src_pad.name().as_str()), &demux, Some("sink"))
                .unwrap_or_else(|err| {
                    error!("Failed to link src pad to demux: {:?}", err);
                });
        }
    });

    // let playbin = &pipeline;
    // playbin.connect_closure("element-setup", false,
    // glib::closure!(move |_playbin: &gst::Element, element: &gst::Element | {
    //     if element.name() == "rtspsrc" {
    //         info!("Setting latency to 0 for rtspsrc");
    //         element.set_property("latency", &0u32);
    //     }
    // }));

    // pipeline.set_property("video-sink", &flutter_sink.video_sink());
    SESSION_CACHE
        .lock()
        .unwrap()
        .insert(engine_handle, (texture_provider.clone(), pipeline.clone(), registered_texture.clone()));
    thread::spawn(move || {
        pipeline
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
        let bus = pipeline.bus().unwrap();
        for msg in bus.iter() {
            trace!("Message: {:?}", msg.view());
            match msg.view() {
                gst::MessageView::Buffering(buffering) => {
                    let percent = buffering.percent();
                    info!("Buffering {}%", percent);
                    if percent < 100 {
                        pipeline.set_state(gst::State::Paused).log_err();
                    } else {
                        pipeline.set_state(gst::State::Playing).log_err();
                    }
                }
                gst::MessageView::Eos(..) => {
                    info!("End of stream");
                    break;
                }
                gst::MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.path_string()),
                        err.error(),
                        err.debug()
                    );
                    break;
                }

                _ => (),
            }
        }
    });

    // wait for the sink to be initialized
    while !initialized_sig.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    trace!("Sink initialized; returning texture id: {}", texture_id);
    Ok(texture_id)
}
