// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
use std::{
    alloc::{self, Layout}, cell::RefCell, sync::{atomic::AtomicBool, Arc, Mutex, Weak}
};

use glib::clone::Downgrade;
use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture};
use log::{debug, trace};

use crate::utils::invoke_on_platform_main_thread;

use super::{fluttersink::utils, types};

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


pub struct FFmpegFrameWrapper(ffmpeg::util::frame::Video);


impl irondash_texture::PixelDataProvider for FFmpegFrameWrapper {
    fn get(&self) -> irondash_texture::PixelData {
        irondash_texture::PixelData {
            width: self.0.width() as _,
            height: self.0.height() as _,
            data: self.0.data(0),
        }
    }
}

pub struct PayloadHolder {
    current_frame: Mutex<Option<Box<FFmpegFrameWrapper>>>
    
}
impl PayloadHolder {
    pub fn new() -> Self {
        Self {
            current_frame: Mutex::new(None),
        }
    }
    
    pub fn set_payload(&self, payload: Box<FFmpegFrameWrapper>) {
        let mut curr_frame = self.current_frame.lock().unwrap();
        *curr_frame = Some(payload);
    }
}

impl PayloadProvider<BoxedPixelData> for PayloadHolder {
    fn get_payload(&self) -> BoxedPixelData {
        let mut curr_frame = self.current_frame.lock().unwrap();
        if let Some(frame) = curr_frame.take() {
            frame
        } else {
            debug!("no frame available returning a default");
            // return empty frame
            Box::new(FFmpegFrameWrapper(ffmpeg::util::frame::Video::new(
                ffmpeg::format::Pixel::RGBA,
                640,
                480,
            )))
        }
    }
}
pub struct SoftwareDecoder {
    video_info: types::VideoInfo,
    kill_sig: AtomicBool,
    payload_holder: Weak<PayloadHolder>,
}
unsafe impl Send for SoftwareDecoder {}
unsafe impl Sync for SoftwareDecoder {}
pub type SharedSendableTexture = Arc<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
impl SoftwareDecoder {
    pub fn new(
        video_info: &types::VideoInfo,
        session_id: u32,
        engine_handle: i64,
    ) -> anyhow::Result<(Arc<Self>, i64, SharedSendableTexture)> {
        let payload_holder = Arc::new(PayloadHolder::new());
        let self_ = Arc::new(Self {
            video_info: video_info.clone(),
            kill_sig: AtomicBool::new(false),
            payload_holder: payload_holder.downgrade(),
        });

        let (sendable, texture_id) = invoke_on_platform_main_thread(
            move || -> anyhow::Result<_> {
                let texture =
                    irondash_texture::Texture::new_with_provider(engine_handle, payload_holder)?;
                let texture_id = texture.id();
                Ok((texture.into_sendable_texture(), texture_id))
            },
        )?;
        
        Ok((self_, texture_id, sendable))
    }
    pub fn start(self: Arc<Self>, sendable_texture: SharedSendableTexture) -> anyhow::Result<()> {
        trace!("starting ffmpeg session for {}", &self.video_info.uri);
        
        let mut ictx = ffmpeg::format::input(&self.video_info.uri)?;

        let input = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input.index();
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        
        let mut decoder = context_decoder.decoder().video()?;
        let mut scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )?;
        let sendable_weak = sendable_texture.downgrade();
        drop(sendable_texture);
        let cb = move || {
            if let Some(sendable_weak) = sendable_weak.upgrade() {
                sendable_weak.mark_frame_available();                
            }
        };
        for (stream, packet) in ictx.packets() {
            if self.kill_sig.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            if stream.index() == video_stream_index {
                let mut packet = packet;
                decoder.send_packet(&mut packet)?;
                self.on_new_sample(&mut decoder, &mut scaler, &cb)?;
            }
        }
        self.terminate(& mut decoder)?;
        Ok(())
    }
    fn terminate(&self,decoder: &mut ffmpeg::decoder::Video) -> anyhow::Result<()> {
        decoder.send_eof().map_err(|e| {
            anyhow::anyhow!("Error sending EOF: {:?}", e)
        })
    }
    
    fn on_new_sample<T>(
        &self,
        decoder: &mut ffmpeg::decoder::Video,
        scaler: &mut ffmpeg::software::scaling::Context,
        mark_frame_avb: &T,
    ) -> anyhow::Result<()>
    where
        T: Fn() -> (),
    {
        let mut decoded = ffmpeg::util::frame::Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb_frame = ffmpeg::util::frame::Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;
            match self.payload_holder.upgrade() {
                Some(payload_holder) => {
                    payload_holder.set_payload(Box::new(FFmpegFrameWrapper(rgb_frame)));
                }
                None => {
                   break;
                }
            }
            mark_frame_avb();
            if self.kill_sig.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
        }
        Ok(())
    }

    pub fn destroy_stream(&self) {
        self.kill_sig
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}


