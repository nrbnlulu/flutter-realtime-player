// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
use std::{
    alloc::{self, Layout},
    cell::RefCell, // add this
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
};

use irondash_texture::{PayloadProvider, PixelData, SendableTexture, SharedPixelData};
use log::{debug, trace};

use crate::utils::invoke_on_platform_main_thread;

use super::types;


pub struct PayloadHolder {
    current_frame: Mutex<Option<ffmpeg::util::frame::Video>>,
    cached_frame: Mutex<Option<SharedPixelData>>,
}
impl PayloadHolder {
    pub fn new() -> Self {
        Self {
            current_frame: Mutex::new(None),
            cached_frame: Mutex::new(None),
        }
    }

    pub fn set_payload(&self, payload: ffmpeg::util::frame::Video) {
        trace!("takin lock for current frame");
        let mut current_frame = self.current_frame.lock().unwrap();
        trace!("setting payload for current frame");
        current_frame.replace(payload);
    }
}
unsafe impl Sync for PayloadHolder {}
unsafe impl Send for PayloadHolder {}

impl PayloadProvider<SharedPixelData> for PayloadHolder {
    fn get_payload(&self) -> SharedPixelData {
        trace!("getting payload from PayloadHolder");
        let mut current_frame = self.current_frame.lock().unwrap();
        if let Some(frame) = current_frame.take() {
            trace!("got current frame, caching it");
            let mut cached_frame = self.cached_frame.lock().unwrap();
            let pixel_data = Arc::new(Mutex::new(PixelData::new(
                1,
                1,
                std::ptr::null(),
            )));
            trace!("caching pixel data: {}x{}", frame.width(), frame.height());
            cached_frame.replace(pixel_data.clone());
            trace!("returning pixel data");
            pixel_data
        } else {
            let cached_frame = self.cached_frame.lock().unwrap();
            if let Some(frame) = cached_frame.as_ref() {
                // return cached frame
                debug!("returning cached frame");
                frame.clone()
            } else {
                // no frame available, return empty frame
                debug!("no cached frame available, returning empty frame");


                Arc::new(Mutex::new(PixelData::new(
                    1 as i32,
                    1 as i32,
                    std::ptr::null(),
                )))
            }
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
                    // not sure we can avoid the clone here.
                    trace!(
                        "setting payload for frame: {}x{}",
                        rgb_frame.width(),
                        rgb_frame.height()
                    );
                    payload_holder.set_payload(decoded.clone());
                }
                None => {
                    break;
                }
            }
            trace!("marking frame as available");
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
