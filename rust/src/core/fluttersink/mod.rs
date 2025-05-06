pub(super) mod sink;
pub mod utils;
use gst::{
    glib::object::Cast,
    prelude::{ElementExtManual, GstBinExt, GstBinExtManual, PadExt},
};
use irondash_engine_context::EngineContext;
use irondash_texture::Texture;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread,
};
use windows::core::Interface;

use gst::prelude::{ElementExt, GstObjectExt};
use log::{error, info, trace};
use sink::FlutterTextureSink;
use utils::LogErr;

use crate::core::platform::NativeTextureProvider;

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
    let appsink_name = format!("sink_{}", engine_handle);
    let pipeline_str = format!(
        "filesrc location={} ! parsebin ! h264parse ! {} ! d3d11upload ! video/x-raw(memory:D3D11Memory) ! appsink name={}",
        video_info.uri,
        "d3d11h264dec",
        appsink_name
    );

    let pipeline = gst::parse::launch(&pipeline_str)?
        .downcast::<gst::Pipeline>()
        .unwrap();

    let pipeline_clone = pipeline.clone();

    let appsink = pipeline
        .by_name(&appsink_name)
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();
    let decoding_engine;

    #[cfg(target_os = "windows")]
    {
        let dimensions_clone = dimensions.clone();
        use crate::core::platform::{GstDecodingEngine, TextureDescriptionProvider2Ext};
        use windows::Win32::Graphics::Direct3D11::ID3D11Device;
        trace!("thread id: {:?}", std::thread::current().id());
        let app_sink_clone = appsink.clone();
        (registered_texture, texture_provider, decoding_engine) =
            utils::invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
                trace!("invoke_on_platform_main_thread called");
                let engine_ctx = EngineContext::get()?;
                trace!("got engine ctx");
                let d3d11_device_raw = EngineContext::get_d3d11_device(engine_ctx, engine_handle)?;
                trace!("d3d11_device_raw: {:?}", d3d11_device_raw);
                let d3d11_device = unsafe {
                    ID3D11Device::from_raw_borrowed(&(d3d11_device_raw as *mut _)).unwrap()
                }
                .clone();
                let decoding_engine = GstDecodingEngine::new(
                    pipeline,
                    &app_sink_clone,
                    d3d11_device,
                    &dimensions_clone,
                )?;
                let texture_provider = NativeTextureProvider::new(decoding_engine.clone())?;

                let tex_provider_clone = texture_provider.clone();
                let reg_texture = irondash_texture::alternative_api::RegisteredTexture::new(
                    texture_provider,
                    engine_handle,
                )
                .map_err(|e| anyhow::anyhow!("Failed to registered texture: {:?}", e))?;

                Ok((reg_texture, tex_provider_clone, decoding_engine))
            })?;
        texture_id = registered_texture.get_texture_id();
    }
    let sink = Arc::new(FlutterTextureSink::new(
        initialized_sig_clone,
        appsink,
        texture_provider.clone(),
        registered_texture.clone(),
    )?);

    let registered_texture_clone = registered_texture.clone();
    let ddecoding_engine_clone = decoding_engine.clone();
    let initialized_sig_clone = initialized_sig.clone();
    decoding_engine.clone().set_callbacks(move || {
        if !initialized_sig_clone.load(std::sync::atomic::Ordering::Relaxed) {
            registered_texture_clone
                .set_current_texture(ddecoding_engine_clone.create_texture_descriptor())
                .log_err();

            initialized_sig_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            let registered_texture_clone = registered_texture_clone.clone();
            thread::spawn(move || {
                thread::sleep(std::time::Duration::from_millis(1000 * 7));

            registered_texture_clone.mark_frame_available().log_err();

            });
        }
    });
    trace!("running {:?} in thread", video_info.uri);
    let _ = decoding_engine.run_in_thread();
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

    // wait for the sink to be initialized
    while !initialized_sig.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    trace!("Sink initialized; returning texture id: {}", texture_id);
    Ok(texture_id)
}
