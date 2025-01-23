mod frame;
pub mod gltexture;
pub(super) mod imp;
pub mod types;
mod platform;
pub mod utils;
use std::{rc::Rc, sync::Arc};

use frame::Frame;
use glib::{
    object::{Cast, ObjectExt},
    subclass::types::ObjectSubclassIsExt,
    types::StaticType,
};
use gltexture::GLTextureSource;
use gst::prelude::{ElementExt, GstBinExt, GstObjectExt};
use imp::ArcSendableTexture;
use log::{error, info};

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
        let (tx, rx) = flume::bounded(3);

        let provider = Arc::new(GLTextureSource::new(rx)?);
        let texture =
            irondash_texture::Texture::new_with_provider(engine_handle, provider.clone())?;
        let tx_id = texture.id();
        let sendable_texture = texture.into_sendable_texture();
        Ok((sendable_texture, tx_id, tx))
    });
}

pub fn testit(engine_handle: i64, uri: String) -> anyhow::Result<i64> {
    let (sendable_fl_txt, id, tx) = create_flutter_texture(engine_handle)?;

    let flsink = utils::make_element("fluttertexturesink", None)?;
    let fl_config = imp::FlutterConfig::new(id, tx, sendable_fl_txt);

    let fl_imp = flsink.downcast_ref::<FlutterTextureSink>().unwrap().imp();
    fl_imp.set_fl_config(fl_config);

    let pipeline = Rc::new(gst::ElementFactory::make("playbin3")
        .property("video-sink", &flsink)
        .property("uri", uri)
        .build()
        .unwrap());
    fl_imp.set_playbin3(pipeline.clone());
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
