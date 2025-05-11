use std::{
    alloc::{self, Layout},
    cell::RefCell,
    sync::{Arc, Mutex},
};

use anyhow::bail;
use gst::{glib::{clone::Downgrade, object::ObjectExt}, prelude::ElementExt};
use irondash_texture::{BoxedPixelData, PayloadProvider, SimplePixelData};
use log::error;

struct PixelBuffer {
    ptr: *mut u8,
    size: usize,
}
impl PixelBuffer {
    fn new(size: usize) -> Self {
        // align 1 used for u8 memory allocations.
        let ptr = unsafe { alloc::alloc(Layout::from_size_align(size, 1).unwrap()) };
        Self { ptr, size }
    }

    fn mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

struct VideoFrame {
    width: u32,
    height: u32,
    /// make sure that data outlives the duration where the shared buffer is used.
    pixel_buffer: PixelBuffer,
}
unsafe impl Send for VideoFrame {}
unsafe impl Sync for VideoFrame {}
impl Drop for VideoFrame {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(
                self.pixel_buffer.ptr,
                Layout::from_size_align(self.pixel_buffer.size, 1).unwrap(),
            );
        }
    }
}
impl irondash_texture::PixelDataProvider for VideoFrame {
    fn get(&self) -> irondash_texture::PixelData {
        irondash_texture::PixelData {
            width: self.width as _,
            height: self.height as _,
            data: unsafe {
                std::slice::from_raw_parts(self.pixel_buffer.ptr, self.pixel_buffer.size)
            },
        }
    }
}

pub struct SoftwareDecoder {
    current_frame: Arc<Mutex<Option<Box<VideoFrame>>>>,
    pipeline: gst::Pipeline,
}
unsafe impl Send for SoftwareDecoder {}
unsafe impl Sync for SoftwareDecoder {}

impl SoftwareDecoder {
    pub fn new(
        pipeline: &gst::Pipeline,
        session_id: u32,
        engine_handle: i64,
    ) -> anyhow::Result<(Arc<Self>, i64)> {
        let appsink = Arc::new(
            gst_app::AppSink::builder()
                .caps(
                    &gst_video::VideoCapsBuilder::new()
                        .format(gst_video::VideoFormat::Bgra)
                        .build(),
                )
                .name(format!("appsink-{}", session_id))
                .build(),
        );
        pipeline.set_property("video-sink", &*appsink);

        let self_ = Arc::new(Self {
            current_frame: Arc::new(Mutex::new(None)), pipeline: pipeline.clone(),
        });
        let self_clone = self_.clone();
        let (sendable, texture_id) = super::fluttersink::utils::invoke_on_platform_main_thread(
            move || -> anyhow::Result<_> {
                let texture =
                    irondash_texture::Texture::new_with_provider(engine_handle, self_clone)?;
                let texture_id = texture.id();
                Ok((texture.into_sendable_texture(), texture_id))
            },
        )?;

        let self_weak = self_.downgrade();
        let cb = move || {
            sendable.mark_frame_available();
        };
        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |app| {
                    if let Some(s) = self_weak.upgrade() {
                        s.on_new_sample(&app, &cb);
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        Ok((self_, texture_id))
    }
    fn on_new_sample<T>(
        &self,
        app: &gst_app::AppSink,
        mark_frame_avb: &T,
    ) -> Result<gst::FlowSuccess, gst::FlowError>
    where
        T: Fn() -> (),
    {
        let sample = app.pull_sample().map_err(|_| gst::FlowError::Eos)?;
        let buffer = sample.buffer_owned().unwrap(); // Probably copies!
        let caps = sample.caps().unwrap();
        let video_info = gst_video::VideoInfo::from_caps(caps).expect("couldn't build video info!");
        if video_info.format() != gst_video::VideoFormat::Bgra {
            error!("Unsupported format: {:?}", video_info.format());
            return Err(gst::FlowError::NotSupported);
        }
        let width = video_info.width();
        let height = video_info.height();

        let video_frame = gst_video::VideoFrame::from_buffer_readable(buffer, &video_info).unwrap();
        let video_buffer = video_frame.buffer();
        let mut pixel_buffer = PixelBuffer::new(video_buffer.size());
        video_buffer
            .copy_to_slice(0, pixel_buffer.mut_slice())
            .map_err(|_| gst::FlowError::NotSupported)?;
        let frame = VideoFrame {
            width,
            height,
            pixel_buffer,
        };

        *self.current_frame.lock().unwrap() = Some(Box::new(frame));
        mark_frame_avb();
        Ok(gst::FlowSuccess::Ok)
    }

    pub fn destroy_stream(&self) {
        let mut curr_frame = self.current_frame.lock().unwrap();

        self.pipeline
            .set_state(gst::State::Null)
            .expect("Failed to set pipeline state to Null");

        if let Some(frame) = curr_frame.take() {
            // drop the frame
            drop(frame);
        }
    }
}

impl PayloadProvider<BoxedPixelData> for SoftwareDecoder {
    fn get_payload(&self) -> BoxedPixelData {
        let mut curr_frame = self.current_frame.lock().unwrap();
        if let Some(frame) = curr_frame.take() {
            frame
        } else {
            // return empty frame
            Box::new(VideoFrame {
                width: 0,
                height: 0,
                pixel_buffer: PixelBuffer::new(0),
            })
        }
    }
}
