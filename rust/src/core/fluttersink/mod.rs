pub(super) mod sink;
pub mod utils;
use gst::{
    ffi::{gst_context_get_structure, gst_element_set_context, gst_structure_to_string},
    glib::{self},
    prelude::{ElementExtManual, GstBinExtManual, PadExt},
};
use irondash_engine_context::EngineContext;
use windows::core::Interface;
use std::{
    collections::HashMap, rc::Weak, sync::{atomic::AtomicBool, Arc, Mutex}, thread
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
    let pipeline = gst::Pipeline::new();
    let pipeline_clone = pipeline.clone();

    let appsink = gst_app::AppSink::builder()
    .caps(
        &gst_video::VideoCapsBuilder::new()
            .features(["memory:D3D11Memory"])
            .format(gst_video::VideoFormat::Bgra)
            .field("texture-target", "2D")
            .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
            .build(),
    )
    .enable_last_sample(false)
    .max_buffers(1u32)
    .build();

    let src = gst::ElementFactory::make("urisourcebin")
        .property("uri", &video_info.uri)
        .build()?;
    let demux = gst::ElementFactory::make("qtdemux").build()?;
    let parse = gst::ElementFactory::make("h264parse").build()?;
    let decoder = gst::ElementFactory::make("d3d11h264dec").build()?;
    let convert = gst::ElementFactory::make("d3d11convert").build()?;
  

    #[cfg(target_os = "windows")]
    {
        let dimensions_clone = dimensions.clone();
        use crate::core::platform::{TextureDescriptionProvider2Ext, GstDecodingEngine};
        use windows::Win32::Graphics::Direct3D11::ID3D11Device;
        trace!("thread id: {:?}", std::thread::current().id());
        let app_sink_clone = appsink.clone();
        (registered_texture, texture_provider) = utils::invoke_on_platform_main_thread(
            move || -> anyhow::Result<(Arc<NativeRegisteredTexture>, Arc<NativeTextureProvider>)> {
                trace!("invoke_on_platform_main_thread called");
                let engine_ctx = EngineContext::get()?;
                trace!("got engine ctx");
                let d3d11_device_raw = EngineContext::get_d3d11_device(engine_ctx, engine_handle)?;
                trace!("d3d11_device_raw: {:?}", d3d11_device_raw);
                let d3d11_device =    unsafe { ID3D11Device::from_raw_borrowed(&(d3d11_device_raw as *mut _)).unwrap() }.clone();
                let decoding_engine = GstDecodingEngine::new(
                    pipeline,
                    &app_sink_clone,
                    d3d11_device,
                    &dimensions_clone,
                )?;
                let jh = decoding_engine.run_in_thread();
                
                
                
                
                
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
    let sink = Arc::new(FlutterTextureSink::new(
        initialized_sig_clone,
        appsink,
        texture_provider.clone(),
        registered_texture.clone(),
    )?);
    gst::Element::link_many(&[&parse, &decoder, &convert, &sink.video_sink()])?;
  pipeline_clone.add_many(&[&src, &demux, &parse, &decoder, &convert, &sink.video_sink()])?;
    
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
    SESSION_CACHE.lock().unwrap().insert(
        engine_handle,
        (
            texture_provider.clone(),
            pipeline_clone.clone(),
            registered_texture.clone(),
        ),
    );
    thread::spawn(move || {
        pipeline_clone
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
        let bus = pipeline_clone.bus().unwrap();
        for msg in bus.iter() {
            trace!("Message: {:?}", msg.view());
            match msg.view() {
                gst::MessageView::Buffering(buffering) => {
                    let percent = buffering.percent();
                    info!("Buffering {}%", percent);
                    if percent < 100 {
                        pipeline_clone.set_state(gst::State::Paused).log_err();
                    } else {
                        pipeline_clone.set_state(gst::State::Playing).log_err();
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
