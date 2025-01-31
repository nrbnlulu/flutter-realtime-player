use glow::HasContext;
use gst_video::{prelude::*, VideoFormat, VideoOrientation};

use gst_gl::prelude::*;
use irondash_texture::BoxedGLTexture;
use std::{
    collections::{HashMap, HashSet},
    ops,
    rc::Rc,
    sync::Arc,
};

use crate::core::{
    ffi::gst_egl_ext::egl_image_from_texture,
    platform::{EglImageWrapper, GlCtx, GstNativeFrameType},
};

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
enum MappedFrame {
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

impl MappedFrame {
    fn buffer(&self) -> &gst::BufferRef {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.buffer(),
            MappedFrame::GL { frame, .. } => frame.buffer(),
        }
    }

    fn width(&self) -> u32 {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.width(),
            MappedFrame::GL { frame, .. } => frame.width(),
        }
    }

    fn height(&self) -> u32 {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.height(),
            MappedFrame::GL { frame, .. } => frame.height(),
        }
    }

    fn format_info(&self) -> gst_video::VideoFormatInfo {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.format_info(),
            MappedFrame::GL { frame, .. } => frame.format_info(),
        }
    }

    fn orientation(&self) -> Orientation {
        match self {
            MappedFrame::SysMem { orientation, .. } => *orientation,
            MappedFrame::GL { orientation, .. } => *orientation,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Frame {
    frame: MappedFrame,
    overlays: Vec<Overlay>,
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

fn gl_frame_to_egl_img(
    frame: gst_gl::GLVideoFrame<gst_gl::gl_video_frame::Readable>,
    cached_textures: &mut HashMap<TextureCacheId, GstNativeFrameType>,
    used_textures: &mut HashSet<TextureCacheId>,
    wrapped_context: gst_gl::GLContext,
    orientation: Orientation,
) -> anyhow::Result<(GstNativeFrameType, f64)> {
    let texture_name = frame.texture_id(0).expect("Invalid texture id") as usize;

    let pixel_aspect_ratio =
        (frame.info().par().numer() as f64) / (frame.info().par().denom() as f64);

    if let Some(texture) = cached_textures.get(&TextureCacheId::GL(texture_name)) {
        used_textures.insert(TextureCacheId::GL(texture_name));
        return Ok((texture.clone(), pixel_aspect_ratio));
    }

    let width = frame.width();
    let height = frame.height();

    let sync_meta = frame.buffer().meta::<gst_gl::GLSyncMeta>().unwrap();
    let format = frame.format();

    let egl_image_ptr = egl_image_from_texture(
        wrapped_context.as_ptr(),
        frame
            .memory(0)
            .unwrap()
            .downcast_memory_ref::<gst_gl::GLMemory>()
            .unwrap()
            .as_mut_ptr(),
        &mut [],
    );
    let egl_img_wrapper = EglImageWrapper::new(egl_image_ptr, width, height, format, orientation);
    cached_textures.insert(TextureCacheId::GL(texture_name), egl_img_wrapper.clone());
    used_textures.insert(TextureCacheId::GL(texture_name));
    Ok((egl_img_wrapper, pixel_aspect_ratio))
}

impl Frame {
    pub(crate) fn into_textures(
        self,
        gl_context: &GlCtx,
        cached_textures: &mut HashMap<TextureCacheId, GstNativeFrameType>,
    ) -> anyhow::Result<Vec<GstNativeFrameType>> {
        gl_context.activate(true);

        let mut native_frames = Vec::with_capacity(1 + self.overlays.len());
        let mut used_frames = HashSet::with_capacity(1 + self.overlays.len());
        let width = self.frame.width();
        let height = self.frame.height();
        let has_alpha = self.frame.format_info().has_alpha();
        let orientation = self.frame.orientation();
        let (native_frame, pixel_aspect_ratio) = match self.frame {
            MappedFrame::SysMem { .. } => {
                // this should only be called if we do software rendering
                unimplemented!("memory_frame_to_egl_img");
            }
            MappedFrame::GL {
                frame,
                wrapped_context,
                ..
            } => gl_frame_to_egl_img(
                frame,
                cached_textures,
                &mut used_frames,
                wrapped_context.clone(),
                orientation,
            )
            .unwrap(),
        };

        native_frames.push(native_frame);

        for overlay in self.overlays {
            unimplemented!(
                "This is an in memory frame, we need to implement this using pixel buffer"
            );
        }

        // Remove all textures that were not used
        // gstreamer would call eglDestroyImage when the object is deleted (has no more refs).
        cached_textures.retain(|k, _| used_frames.contains(k));

        Ok(native_frames)
    }
}

impl Frame {
    pub(crate) fn new(
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

                frame = Some(MappedFrame::GL {
                    frame: mapped_frame,
                    wrapped_context: wrapped_context.unwrap().clone(),
                    orientation: orientation.clone(),
                });
            }
        }

        let mut frame = Self {
            frame: match frame {
                Some(frame) => frame,
                None => MappedFrame::SysMem {
                    frame: gst_video::VideoFrame::from_buffer_readable(buffer.clone(), info)
                        .map_err(|_| gst::FlowError::Error)?,
                    orientation,
                },
            },
            overlays: vec![],
        };
        frame.overlays = frame
            .frame
            .buffer()
            .iter_meta::<gst_video::VideoOverlayCompositionMeta>()
            .flat_map(|meta| {
                meta.overlay()
                    .iter()
                    .filter_map(|rect| {
                        let buffer = rect
                            .pixels_unscaled_argb(gst_video::VideoOverlayFormatFlags::GLOBAL_ALPHA);
                        let (x, y, width, height) = rect.render_rectangle();
                        let global_alpha = rect.global_alpha();

                        let vmeta = buffer.meta::<gst_video::VideoMeta>().unwrap();
                        let info = gst_video::VideoInfo::builder(
                            vmeta.format(),
                            vmeta.width(),
                            vmeta.height(),
                        )
                        .build()
                        .unwrap();
                        let frame =
                            gst_video::VideoFrame::from_buffer_readable(buffer, &info).ok()?;

                        Some(Overlay {
                            frame,
                            x,
                            y,
                            width,
                            height,
                            global_alpha,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(frame)
    }
}

// fn memory_frame_to_egl_img(
//     frame: gst_video::VideoFrame<gst_video::video_frame::Readable>,
//     gl: &GlCtx,
//     cached_textures: &mut HashMap<TextureCacheId, GLTexture>,
//     used_textures: &mut HashSet<TextureCacheId>,
// ) -> anyhow::Result<(GstNativeFrameType, f64)> {

//     let ptr = frame.plane_data(0)?.as_ptr() as usize;
//     let pixel_aspect_ratio =
//         (frame.info().par().numer() as f64) / (frame.info().par().denom() as f64); // typos: ignore

//     if let Some(texture) = cached_textures.get(&TextureCacheId::Memory(ptr)) {
//         used_textures.insert(TextureCacheId::Memory(ptr));
//         return Ok((texture.clone(), pixel_aspect_ratio));
//     }

//     let width = frame.width();
//     let height = frame.height();
//     let rowstride = frame.plane_stride()[0] as usize;
//     let texture = unsafe { gl.create_texture().map_err(|e| anyhow::anyhow!(e)) }?;

//     unsafe {
//         gl.bind_texture(glow::TEXTURE_2D, Some(texture));
//         let frame_data: Option<&[u8]> = frame.plane_data(0).ok();
//         gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, (rowstride / 4) as i32);
//         fn map_format_to_glow(gst_fmt: VideoFormat) -> u32 {
//             match gst_fmt {
//                 VideoFormat::Rgba => glow::RGBA,
//                 VideoFormat::Bgra => glow::BGRA,
//                 VideoFormat::Rgb => glow::RGB,
//                 VideoFormat::Bgr => glow::BGR,
//                 _ => unimplemented!("unsupported format"),
//             }
//         }
//         let fmt = map_format_to_glow(frame.format());
//         gl.tex_image_2d(
//             glow::TEXTURE_2D,
//             0,
//             fmt as i32,
//             width as i32,
//             height as i32,
//             0,
//             fmt,
//             glow::UNSIGNED_BYTE,
//             glow::PixelUnpackData::Slice(frame_data),
//         );
//         gl.generate_mipmap(glow::TEXTURE_2D);
//     }
//     let gl_tex = GLTexture::from_glow(texture, width as i32, height as i32, gl.clone());
//     cached_textures.insert(TextureCacheId::Memory(ptr), gl_tex.clone());
//     used_textures.insert(TextureCacheId::Memory(ptr));

//     Ok((gl_tex, pixel_aspect_ratio))
// }
