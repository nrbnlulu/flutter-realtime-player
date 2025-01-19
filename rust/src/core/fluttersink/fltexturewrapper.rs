use std::sync::Arc;

use glib::subclass::{object::ObjectImpl, types::ObjectSubclass};
use gst::subclass::prelude::GstObjectImpl;
use glib::prelude::*;
use glib::subclass::prelude::*;
use glib_macros::Properties;

use glib::types::StaticType;

mod imp{
#[derive(Default)]
#[properties(wrapper_type = super::FlTextureWrapper)]
pub(crate) struct FlTextureWrapper {
    fl_txt_id: i64,
    fl_texture: Option<ArcSendableTexture>,
}



pub type ArcSendableTexture = Arc<irondash_texture::SendableTexture<irondash_texture::BoxedGLTexture>>;

#[glib::object_subclass]
impl ObjectSubclass for FlTextureWrapper {
    const NAME: &'static str = "FlTextureWrapper";
    type Type = super::FlTextureWrapper;
    type ParentType = gst::Object;
}

#[glib::derived_properties]
impl ObjectImpl for FlTextureWrapper {}

impl GstObjectImpl for FlTextureWrapper {}

}


glib::wrapper! {
    pub struct FlTextureWrapper(ObjectSubclass<imp::FlTextureWrapper>)
    @extends gst::Object;
}

impl FlTextureWrapper {
    pub fn new(name: Option<&str>) -> Self {
        gst::Object::builder().name_if_some(name).build().unwrap()
    }
}

