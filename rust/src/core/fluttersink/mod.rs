mod frame;
pub mod gltexture;
pub(super) mod sink;
pub mod types;
pub mod utils;
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

use frame::ResolvedFrame;
use glib::{
    object::{Cast, ObjectExt},
    subclass::types::ObjectSubclassIsExt,
    types::StaticType,
};
use gltexture::GLTextureSource;
use gst::{prelude::{ElementExt, ElementExtManual, GstBinExt, GstBinExtManual, GstObjectExt}, trace};
use log::{error, info};
use sink::ArcSendableTexture;

use super::platform::{GstNativeFrameType, GL_MANAGER};

pub(crate) enum SinkEvent {
    FrameChanged(ResolvedFrame),
}
pub(crate) type FrameSender = flume::Sender<SinkEvent>;

glib::wrapper! {
    pub struct FlutterTextureSink(ObjectSubclass<sink::FlutterTextureSink>)
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

fn create_flutter_texture(
    engine_handle: i64,
) -> anyhow::Result<(ArcSendableTexture, i64, FrameSender)> {
    let (tx, rx) = flume::bounded(1);

    let provider = Arc::new(GLTextureSource::new(rx, engine_handle)?);
    let texture = irondash_texture::Texture::new_with_provider(engine_handle, provider.clone())?;
    let tx_id = texture.id();
    Ok((texture.into_sendable_texture(), tx_id, tx))
}

pub fn testit(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    let (sendable_fl_txt, id, tx) =
        utils::invoke_on_platform_main_thread(move || create_flutter_texture(engine_handle))?;

    let flsink = utils::make_element("fluttertexturesink", None)?;
    let fl_config = sink::FlutterConfig::new(id, engine_handle, tx, sendable_fl_txt);

    let fl_imp = flsink.downcast_ref::<FlutterTextureSink>().unwrap().imp();
    fl_imp.set_fl_config(fl_config);
    let glsink = gst::ElementFactory::make("glsinkbin")
        .property("sink", &flsink)
        .build()
        .expect("Fatal: Unable to create glsink");

    let pipeline = gst::ElementFactory::make("playbin3")
        .property("video-sink", &glsink)
        .property("uri", uri)
        .build()
        .unwrap();

    thread::spawn(move || {
        pipeline
            .set_state(gst::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");
        let bus = pipeline.bus().unwrap();
        for msg in bus.iter() {
            trace!(gst::CAT_DEFAULT, "Message: {:?}", msg.view());
            match msg.view() {
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

    Ok(id)
}
