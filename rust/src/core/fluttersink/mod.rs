pub(super) mod sink;
pub mod utils;
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread,
};
use gst::{ffi::{gst_context_get_structure, gst_element_set_context, gst_structure_to_string}, glib::{self, translate::ToGlibPtr, value::ToValue}, prelude::{ElementExtManual, GstBinExtManual, PadExt}};

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
        registered_texture,
    )?);
    pipeline.add_many(&[&src, &demux, &parse, &decoder, &convert, &sink.video_sink()])?;
    gst::Element::link_many(&[&parse, &decoder, &convert, &sink.video_sink()])?;

    demux.connect_pad_added(move |demux, src_pad| {
        let parse_sink_pad = parse.static_pad("sink").unwrap();
        if parse_sink_pad.is_linked() {
            return;
        }
        demux.link_pads(Some(src_pad.name().as_str()), &parse, Some("sink"))
            .unwrap();
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
                        let ctx_ = texture_provider.context.texture.read().unwrap();
                        let ctx = ctx_.as_ref().unwrap();
                        unsafe {
                            use gst::prelude::*;

                           let el_raw = msg.src().unwrap().clone().downcast_ref::<gst::Element>().unwrap().as_ptr();

                            gst_element_set_context(
                                el_raw,
                                ctx.gst_d3d_ctx_ptr as *mut _,
                            );
                            info!("Set context for element: {:?}", msg.src().unwrap().name());
                            let ctx = gst_context_get_structure(ctx.gst_d3d_ctx_ptr as *mut _);
                            let ctx = gst::StructureRef::from_glib_borrow(ctx);
                            let ctx_str = gst_structure_to_string(ctx.as_ptr());
                            info!("published context to gstd3d11: {:?}", ctx_str);
                            glib::ffi::g_free(ctx_str as *mut _);
                     } 



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
