use gst_video::{prelude::*, VideoFormat, VideoOrientation};

use std::{
    collections::{HashMap, HashSet},
    ops,
    rc::Rc,
    sync::Arc,
};

use super::types::Orientation;


#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VideoInfo {
    VideoInfo(gst_video::VideoInfo),
}

impl From<gst_video::VideoInfo> for VideoInfo {
    fn from(v: gst_video::VideoInfo) -> Self {
        VideoInfo::VideoInfo(v)
    }
}

impl ops::Deref for VideoInfo {
    type Target = gst_video::VideoInfo;

    fn deref(&self) -> &Self::Target {
        match self {
            VideoInfo::VideoInfo(info) => info,
        }
    }
}


#[derive(Debug)]
pub(crate) struct GlTextureWrapper {
    pub texture: GLTexture,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub global_alpha: f32,
    pub has_alpha: bool,
    pub orientation: VideoOrientation,
}

struct FrameWrapper(gst_video::VideoFrame<gst_video::video_frame::Readable>);
impl AsRef<[u8]> for FrameWrapper {
    fn as_ref(&self) -> &[u8] {
        self.0.plane_data(0).unwrap()
    }
}

/// Convert a video frame to a
fn video_frame_to_pixel_buffer(
    frame: gst_video::VideoFrame<gst_video::video_frame::Readable>,
) -> anyhow::Result<()> {
    unimplemented!("video_frame_to_pixel_buffer")
}

pub enum ResolvedFrame {
    GL(NativeFrameType),
    Memory(Box<[u8]>),
}
