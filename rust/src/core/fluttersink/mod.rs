pub(super) mod sink;
pub mod utils;
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread,
};
use gst::glib;

use gst::{
    glib::object::{Cast, ObjectExt},
    prelude::{ElementExt, GstObjectExt},
};
use log::{error, info, trace};
use sink::FlutterTextureSink;
use utils::LogErr;

use crate::core::platform::NativeTextureProvider;

use super::{platform::NativeRegisteredTexture, types};

pub fn init() -> anyhow::Result<()> {
    gst::init().map_err(|e| anyhow::anyhow!("Failed to initialize gstreamer: {:?}", e))
}
static   GST_D3D11_DEVICE_HANDLE_CONTEXT_TYPE: &'static str =  "gst.d3d11.device.handle";

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


    let pipeline = gst::ElementFactory::make("playbin")
        .property("uri", &video_info.uri)
        .property("mute", &video_info.mute)
        .build()?
        .downcast::<gst::Pipeline>()
        .unwrap();
    let flutter_sink = Arc::new(FlutterTextureSink::new(
        initialized_sig_clone,
        &pipeline,
        texture_provider,
        registered_texture,
    )?);
    let playbin = &pipeline;
    playbin.connect_closure("element-setup", false, 
    glib::closure!(move |_playbin: &gst::Element, element: &gst::Element | {
        if element.name() == "rtspsrc" {
            info!("Setting latency to 0 for rtspsrc");
            element.set_property("latency", &0u32);
        }
    }));

    pipeline.set_property("video-sink", &flutter_sink.video_sink());

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
                },
                gst::MessageView::NeedContext(msg) => {
                    info!("Need context: {:?}", msg.context_type());
                    #[cfg(target_os = "windows")]
                    if *msg.context_type() == *GST_D3D11_DEVICE_HANDLE_CONTEXT_TYPE {
                        texture_provider.tex
                    }
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
