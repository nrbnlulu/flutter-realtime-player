use glow::HasContext;
use gst_video::{prelude::*, VideoFormat};

use gst_gl::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    ops,
    rc::Rc,
};

use super::{gltexture::GLTexture, types::GlCtx};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VideoInfo {
    VideoInfo(gst_video::VideoInfo),
    #[cfg(all(target_os = "linux", feature = "dmabuf"))]
    DmaDrm(gst_video::VideoInfoDmaDrm),
}

impl From<gst_video::VideoInfo> for VideoInfo {
    fn from(v: gst_video::VideoInfo) -> Self {
        VideoInfo::VideoInfo(v)
    }
}

#[cfg(all(target_os = "linux", feature = "dmabuf"))]
impl From<gst_video::VideoInfoDmaDrm> for VideoInfo {
    fn from(v: gst_video::VideoInfoDmaDrm) -> Self {
        VideoInfo::DmaDrm(v)
    }
}

impl ops::Deref for VideoInfo {
    type Target = gst_video::VideoInfo;

    fn deref(&self) -> &Self::Target {
        match self {
            VideoInfo::VideoInfo(info) => info,
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            VideoInfo::DmaDrm(info) => info,
        }
    }
}

impl VideoInfo {
    #[cfg(all(target_os = "linux", feature = "dmabuf"))]
    fn dma_drm(&self) -> Option<&gst_video::VideoInfoDmaDrm> {
        match self {
            VideoInfo::VideoInfo(..) => None,
            VideoInfo::DmaDrm(info) => Some(info),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TextureCacheId {
    Memory(usize),
    GL(usize),
    #[cfg(all(target_os = "linux", feature = "dmabuf"))]
    DmaBuf([i32; 4]),
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
    #[cfg(all(target_os = "linux", feature = "dmabuf"))]
    DmaBuf {
        buffer: gst::Buffer,
        info: gst_video::VideoInfoDmaDrm,
        n_planes: u32,
        fds: [i32; 4],
        offsets: [usize; 4],
        strides: [usize; 4],
        width: u32,
        height: u32,
        orientation: Orientation,
    },
}

impl MappedFrame {
    fn buffer(&self) -> &gst::BufferRef {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.buffer(),
            MappedFrame::GL { frame, .. } => frame.buffer(),
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            MappedFrame::DmaBuf { buffer, .. } => buffer,
        }
    }

    fn width(&self) -> u32 {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.width(),
            MappedFrame::GL { frame, .. } => frame.width(),
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            MappedFrame::DmaBuf { info, .. } => info.width(),
        }
    }

    fn height(&self) -> u32 {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.height(),
            MappedFrame::GL { frame, .. } => frame.height(),
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            MappedFrame::DmaBuf { info, .. } => info.height(),
        }
    }

    fn format_info(&self) -> gst_video::VideoFormatInfo {
        match self {
            MappedFrame::SysMem { frame, .. } => frame.format_info(),
            MappedFrame::GL { frame, .. } => frame.format_info(),
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            MappedFrame::DmaBuf { info, .. } => info.format_info(),
        }
    }

    fn orientation(&self) -> Orientation {
        match self {
            MappedFrame::SysMem { orientation, .. } => *orientation,
            MappedFrame::GL { orientation, .. } => *orientation,
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            MappedFrame::DmaBuf { orientation, .. } => *orientation,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Frame {
    frame: MappedFrame,
    overlays: Vec<Overlay>,
}

#[derive(Debug, Default, glib::Enum, PartialEq, Eq, Copy, Clone)]
#[repr(C)]
#[enum_type(name = "FlutterSinkOrientation")]
pub enum Orientation {
    #[default]
    Auto,
    Rotate0,
    Rotate90,
    Rotate180,
    Rotate270,
    FlipRotate0,
    FlipRotate90,
    FlipRotate180,
    FlipRotate270,
}

impl Orientation {
    pub fn from_tags(tags: &gst::TagListRef) -> Option<Orientation> {
        let orientation = tags
            .generic("image-orientation")
            .and_then(|v| v.get::<String>().ok())?;

        Some(match orientation.as_str() {
            "rotate-0" => Orientation::Rotate0,
            "rotate-90" => Orientation::Rotate90,
            "rotate-180" => Orientation::Rotate180,
            "rotate-270" => Orientation::Rotate270,
            "flip-rotate-0" => Orientation::FlipRotate0,
            "flip-rotate-90" => Orientation::FlipRotate90,
            "flip-rotate-180" => Orientation::FlipRotate180,
            "flip-rotate-270" => Orientation::FlipRotate270,
            _ => return None,
        })
    }

    pub fn is_flip_width_height(self) -> bool {
        matches!(
            self,
            Orientation::Rotate90
                | Orientation::Rotate270
                | Orientation::FlipRotate90
                | Orientation::FlipRotate270
        )
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
pub(crate) struct Texture {
    pub texture: GLTexture,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub global_alpha: f32,
    pub has_alpha: bool,
    pub orientation: Orientation,
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

fn video_frame_to_gl_texture(
    frame: gst_gl::GLVideoFrame<gst_gl::gl_video_frame::Readable>,
    cached_textures: &mut HashMap<TextureCacheId, GLTexture>,
    used_textures: &mut HashSet<TextureCacheId>,
    #[allow(unused)] wrapped_context: &gst_gl::GLContext,
    gl_ctx: &GlCtx,
) -> anyhow::Result<(GLTexture, f64)> {
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

    let texture = GLTexture::try_new(
        texture_name as u32,
        width as i32,
        height as i32,
        gl_ctx.clone(),
    )?;

    cached_textures.insert(TextureCacheId::GL(texture_name), texture.clone());
    used_textures.insert(TextureCacheId::GL(texture_name));
    Ok((texture, pixel_aspect_ratio))
}

#[cfg(all(target_os = "linux", feature = "dmabuf"))]
#[allow(clippy::too_many_arguments)]
fn video_frame_to_dmabuf_texture(
    buffer: gst::Buffer,
    cached_textures: &mut HashMap<TextureCacheId, gdk::Texture>,
    used_textures: &mut HashSet<TextureCacheId>,
    info: &gst_video::VideoInfoDmaDrm,
    n_planes: u32,
    fds: &[i32; 4],
    offsets: &[usize; 4],
    strides: &[usize; 4],
    width: u32,
    height: u32,
) -> Result<(gdk::Texture, f64), glib::Error> {
    let pixel_aspect_ratio = (info.par().numer() as f64) / (info.par().denom() as f64);

    if let Some(texture) = cached_textures.get(&TextureCacheId::DmaBuf(*fds)) {
        used_textures.insert(TextureCacheId::DmaBuf(*fds));
        return Ok((texture.clone(), pixel_aspect_ratio));
    }

    let builder = gdk::DmabufTextureBuilder::new();
    builder.set_display(&gdk::Display::default().unwrap());
    builder.set_fourcc(info.fourcc());
    builder.set_modifier(info.modifier());
    builder.set_width(width);
    builder.set_height(height);
    builder.set_n_planes(n_planes);
    for plane in 0..(n_planes as usize) {
        builder.set_fd(plane as u32, fds[plane]);
        builder.set_offset(plane as u32, offsets[plane] as u32);
        builder.set_stride(plane as u32, strides[plane] as u32);
    }

    let texture = unsafe {
        builder.build_with_release_func(move || {
            drop(buffer);
        })?
    };

    cached_textures.insert(TextureCacheId::DmaBuf(*fds), texture.clone());
    used_textures.insert(TextureCacheId::DmaBuf(*fds));

    Ok((texture, pixel_aspect_ratio))
}

impl Frame {
    pub(crate) fn into_textures(
        self,
        gl_context: GlCtx,
        cached_textures: &mut HashMap<TextureCacheId, GLTexture>,
    ) -> anyhow::Result<Vec<Texture>> {
        let mut textures = Vec::with_capacity(1 + self.overlays.len());
        let mut used_textures = HashSet::with_capacity(1 + self.overlays.len());
        let width = self.frame.width();
        let height = self.frame.height();
        let has_alpha = self.frame.format_info().has_alpha();
        let orientation = self.frame.orientation();
        let (texture, pixel_aspect_ratio) = match self.frame {
            MappedFrame::SysMem { frame, .. } => video_frame_to_memory_texture(
                frame,
                &gl_context,
                cached_textures,
                &mut used_textures,
            )?,
            MappedFrame::GL {
                frame,
                wrapped_context,
                ..
            } => {
                // let Some(gdk_context) = gl_context else {
                //     // This will fail badly if the video frame was actually mapped as GL texture
                //     // but this case can't really happen as we only do that if we actually have a
                //     // GDK GL context.
                //     unreachable!();
                // };
                video_frame_to_gl_texture(
                    frame,
                    cached_textures,
                    &mut used_textures,
                    &wrapped_context,
                    &gl_context,
                )
                .unwrap()
            }
            #[cfg(all(target_os = "linux", feature = "dmabuf"))]
            MappedFrame::DmaBuf {
                buffer,
                info,
                n_planes,
                fds,
                offsets,
                strides,
                width,
                height,
                ..
            } => video_frame_to_dmabuf_texture(
                buffer,
                cached_textures,
                &mut used_textures,
                &info,
                n_planes,
                &fds,
                &offsets,
                &strides,
                width,
                height,
            )?,
        };

        textures.push(Texture {
            texture,
            x: 0.0,
            y: 0.0,
            width: width as f32 * pixel_aspect_ratio as f32,
            height: height as f32 * pixel_aspect_ratio as f32,
            global_alpha: 1.0,
            has_alpha,
            orientation,
        });

        for overlay in self.overlays {
            unimplemented!(
                "This is an in memory frame, we need to implement this using pixel buffer"
            );
        }

        let mut unused_textures: HashSet<TextureCacheId> = HashSet::new();
        // Remove all textures that were not used
        for (id, texture) in cached_textures.iter() {
            if !used_textures.contains(id) {
                unsafe { gl_context.delete_texture(texture.name) };
                unused_textures.insert(id.clone());
            }
        }
        cached_textures.retain(|id, _| !unused_textures.contains(id));

        Ok(textures)
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

        // Check we received a buffer with dmabuf memory and if so do some checks before
        // passing it onwards
        #[cfg(all(target_os = "linux", feature = "dmabuf"))]
        if frame.is_none()
            && buffer
                .peek_memory(0)
                .is_memory_type::<gst_allocators::DmaBufMemory>()
        {
            if let Some((vmeta, info)) =
                Option::zip(buffer.meta::<gst_video::VideoMeta>(), info.dma_drm())
            {
                let mut fds = [-1i32; 4];
                let mut offsets = [0; 4];
                let mut strides = [0; 4];
                let n_planes = vmeta.n_planes() as usize;

                let vmeta_offsets = vmeta.offset();
                let vmeta_strides = vmeta.stride();

                for plane in 0..n_planes {
                    let Some((range, skip)) =
                        buffer.find_memory(vmeta_offsets[plane]..(vmeta_offsets[plane] + 1))
                    else {
                        break;
                    };

                    let mem = buffer.peek_memory(range.start);
                    let Some(mem) = mem.downcast_memory_ref::<gst_allocators::DmaBufMemory>()
                    else {
                        break;
                    };

                    let fd = mem.fd();
                    fds[plane] = fd;
                    offsets[plane] = mem.offset() + skip;
                    strides[plane] = vmeta_strides[plane] as usize;
                }

                // All fds valid?
                if fds[0..n_planes].iter().all(|fd| *fd != -1) {
                    frame = Some(MappedFrame::DmaBuf {
                        buffer: buffer.clone(),
                        info: info.clone(),
                        n_planes: n_planes as u32,
                        fds,
                        offsets,
                        strides,
                        width: vmeta.width(),
                        height: vmeta.height(),
                        orientation,
                    });
                }
            }
        }

        if frame.is_none() {
            // Check we received a buffer with GL memory and if the context of that memory
            // can share with the wrapped context around the GDK GL context.
            //
            // If not it has to be uploaded to the GPU.
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
                    orientation,
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

fn video_frame_to_memory_texture(
    frame: gst_video::VideoFrame<gst_video::video_frame::Readable>,
    gl: &GlCtx,
    cached_textures: &mut HashMap<TextureCacheId, GLTexture>,
    used_textures: &mut HashSet<TextureCacheId>,
) -> anyhow::Result<(GLTexture, f64)> {
    let ptr = frame.plane_data(0)?.as_ptr() as usize;
    let pixel_aspect_ratio =
        (frame.info().par().numer() as f64) / (frame.info().par().denom() as f64); // typos: ignore

    if let Some(texture) = cached_textures.get(&TextureCacheId::Memory(ptr)) {
        used_textures.insert(TextureCacheId::Memory(ptr));
        return Ok((texture.clone(), pixel_aspect_ratio));
    }

    let width = frame.width();
    let height = frame.height();
    let rowstride = frame.plane_stride()[0] as usize;
    let texture = unsafe { gl.create_texture().map_err(|e| anyhow::anyhow!(e)) }?;

    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        let frame_data: Option<&[u8]> = frame.plane_data(0).ok();
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, (rowstride / 4) as i32);
        fn map_format_to_glow(gst_fmt: VideoFormat) -> u32 {
            match gst_fmt {
                VideoFormat::Rgba => glow::RGBA,
                VideoFormat::Bgra => glow::BGRA,
                VideoFormat::Rgb => glow::RGB,
                VideoFormat::Bgr => glow::BGR,
                _ => unimplemented!("unsupported format"),
            }
        }
        let fmt = map_format_to_glow(frame.format());
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            fmt as i32,
            width as i32,
            height as i32,
            0,
            fmt,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(frame_data),
        );
        gl.generate_mipmap(glow::TEXTURE_2D);
    }
    let gl_tex = GLTexture::from_glow(texture, width as i32, height as i32, gl.clone());
    cached_textures.insert(TextureCacheId::Memory(ptr), gl_tex.clone());
    used_textures.insert(TextureCacheId::Memory(ptr));

    Ok((gl_tex, pixel_aspect_ratio))
}
