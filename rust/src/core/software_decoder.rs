// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
use std::{
    alloc::{self, Layout},
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
};

use irondash_texture::{PayloadProvider, PixelData, SendableTexture, SharedPixelData};
use log::{debug, trace};

use crate::utils::invoke_on_platform_main_thread;

use super::types;

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



pub struct PayloadHolder {
    current_frame: Option<SharedPixelData>,
}
impl PayloadHolder {
    pub fn new() -> Self {
        Self {
            current_frame: None,
        }
    }

    pub fn set_payload(&self, payload: SharedPixelData) {
        let mut curr_frame = self.current_frame.replace(payload).unwrap();
    }
}

impl PayloadProvider<SharedPixelData> for PayloadHolder {
    fn get_payload(&self) -> SharedPixelData {
        if let Some(frame) = self.current_frame {
            frame
        } else {
            debug!("no frame available returning a default");
            // return empty frame
            Arc::new(Mutex::new(PixelData::new(0, 0, Vec::new())))
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
pub type SharedSendableTexture = Arc<SendableTexture<SharedPixelData>>;
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
            payload_holder: Arc::downgrade(&payload_holder),
        });

        let (sendable, texture_id) =
            invoke_on_platform_main_thread(move || -> anyhow::Result<_> {
                let texture =
                    irondash_texture::Texture::new_with_provider(engine_handle, payload_holder)?;
                let texture_id = texture.id();
                Ok((texture.into_sendable_texture(), texture_id))
            })?;

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
        let sendable_weak = Arc::downgrade(&sendable_texture);
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
        self.terminate(&mut decoder)?;
        Ok(())
    }
    fn terminate(&self, decoder: &mut ffmpeg::decoder::Video) -> anyhow::Result<()> {
        decoder
            .send_eof()
            .map_err(|e| anyhow::anyhow!("Error sending EOF: {:?}", e))
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
                    let data = Vec::from(rgb_frame.data(0).to_vec());
                    payload_holder.set_payload(
                        Arc::new(
                            PixelData::new(decoded.width(), decoded.height(), data)
                        )
                    );

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
