use gst_video::{prelude::*, VideoFormat, VideoOrientation};

use gst_gl::prelude::*;
use irondash_texture::BoxedGLTexture;
use std::{
    collections::{HashMap, HashSet},
    ops,
    rc::Rc,
    sync::Arc,
};

use crate::core::platform::{GlCtx, LinuxNativeTexture, NativeFrameType};

use super::{gltexture::GLTexture, types::Orientation};

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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TextureCacheId {
    Memory(usize),
    GL(usize),
}

#[derive(Debug)]
pub(crate) enum GstMappedFrame {
    SysMem {
        frame: gst_video::VideoFrame<gst_video::video_frame::Readable>,
        orientation: Orientation,
    },
    GL {
        frame: gst_gl::GLVideoFrame<gst_gl::gl_video_frame::Readable>,
        wrapped_context: gst_gl::GLContext,
        orientation: Orientation,
    },
}

impl GstMappedFrame {
    pub(crate) fn from_gst_buffer(
        buffer: &gst::Buffer,
        info: &VideoInfo,
        orientation: Orientation,
        wrapped_context: Option<&gst_gl::GLContext>,
    ) -> Result<Self, gst::FlowError> {
        // Empty buffers get filtered out in show_frame
        debug_assert!(buffer.n_memory() > 0);

        #[allow(unused_mut)]
        let mut frame = None;

        if frame.is_none() {
            // Check we received a buffer with GL memory and if the context of that memory
            // can share with the wrapped context around the GDK GL context.
            //
            // If not it has to be uploaded to the GPU.
            // TODO: this is prob redundant with our architecture
            let memory_ctx = buffer
                .peek_memory(0)
                .downcast_memory_ref::<gst_gl::GLBaseMemory>()
                .and_then(|m| {
                    let ctx = m.context();
                    if wrapped_context.is_some_and(|wrapped_context| wrapped_context.can_share(ctx))
                    {
                        Some(ctx)
                    } else {
                        None
                    }
                });

            if let Some(memory_ctx) = memory_ctx {
                // If there is no GLSyncMeta yet then we need to add one here now, which requires
                // obtaining a writable buffer.
                let mapped_frame = if buffer.meta::<gst_gl::GLSyncMeta>().is_some() {
                    gst_gl::GLVideoFrame::from_buffer_readable(buffer.clone(), info)
                        .map_err(|_| gst::FlowError::Error)?
                } else {
                    let mut buffer = buffer.clone();
                    {
                        let buffer = buffer.make_mut();
                        gst_gl::GLSyncMeta::add(buffer, memory_ctx);
                    }
                    gst_gl::GLVideoFrame::from_buffer_readable(buffer, info)
                        .map_err(|_| gst::FlowError::Error)?
                };

                // Now that it's guaranteed that there is a sync meta and the frame is mapped, set
                // a sync point so we can ensure that the texture is ready later when making use of
                // it as gdk::GLTexture.
                let meta = mapped_frame.buffer().meta::<gst_gl::GLSyncMeta>().unwrap();
                meta.set_sync_point(memory_ctx);

                frame = Some(GstMappedFrame::GL {
                    frame: mapped_frame,
                    wrapped_context: wrapped_context.unwrap().clone(),
                    orientation: orientation.clone(),
                });
            }
        }

        Ok(match frame {
            Some(frame) => frame,
            None => GstMappedFrame::SysMem {
                frame: gst_video::VideoFrame::from_buffer_readable(buffer.clone(), info)
                    .map_err(|_| gst::FlowError::Error)?,
                orientation,
            },
        })
    }

    fn buffer(&self) -> &gst::BufferRef {
        match self {
            GstMappedFrame::SysMem { frame, .. } => frame.buffer(),
            GstMappedFrame::GL { frame, .. } => frame.buffer(),
        }
    }

    fn width(&self) -> u32 {
        match self {
            GstMappedFrame::SysMem { frame, .. } => frame.width(),
            GstMappedFrame::GL { frame, .. } => frame.width(),
        }
    }

    fn height(&self) -> u32 {
        match self {
            GstMappedFrame::SysMem { frame, .. } => frame.height(),
            GstMappedFrame::GL { frame, .. } => frame.height(),
        }
    }

    fn format_info(&self) -> gst_video::VideoFormatInfo {
        match self {
            GstMappedFrame::SysMem { frame, .. } => frame.format_info(),
            GstMappedFrame::GL { frame, .. } => frame.format_info(),
        }
    }

    fn orientation(&self) -> Orientation {
        match self {
            GstMappedFrame::SysMem { orientation, .. } => *orientation,
            GstMappedFrame::GL { orientation, .. } => *orientation,
        }
    }
}

#[derive(Debug)]
struct Overlay {
    frame: gst_video::VideoFrame<gst_video::video_frame::Readable>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    global_alpha: f32,
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
