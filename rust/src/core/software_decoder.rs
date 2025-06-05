// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
use std::{
    alloc::{self, Layout},
    ops::Deref,
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
};

use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture};
use log::{debug, trace};

use crate::{
    core::{fluttersink::utils::LogErr, types::{DartUpdateStream, StreamMessages}},
    utils::invoke_on_platform_main_thread,
};

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

#[derive(Clone)]
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
    current_frame: Mutex<Option<Box<FFmpegFrameWrapper>>>,
    previous_frame: Mutex<Option<Box<FFmpegFrameWrapper>>>,
}
impl PayloadHolder {
    pub fn new() -> Self {
        Self {
            current_frame: Mutex::new(None),
            previous_frame: Mutex::new(None),
        }
    }

    pub fn set_payload(&self, payload: Box<FFmpegFrameWrapper>) {
        let mut curr_frame = self.current_frame.lock().unwrap();
        let mut prev_frame = self.previous_frame.lock().unwrap();
        // Move current to previous before replacing
        *prev_frame = curr_frame.take();
        *curr_frame = Some(payload);
    }
}

impl PayloadProvider<BoxedPixelData> for PayloadHolder {
    fn get_payload(&self) -> BoxedPixelData {
        let mut curr_frame = self.current_frame.lock().unwrap();
        if let Some(frame) = curr_frame.take() {
            frame
        } else {
            // Try to return a clone of the previous frame if it exists
            let prev_frame = self.previous_frame.lock().unwrap();
            if let Some(ref prev) = *prev_frame {
                debug!("returning previous frame");
                prev.clone()
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
}

struct DecodingContext {
    ictx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
}

pub struct SoftwareDecoder {
    video_info: types::VideoInfo,
    kill_sig: AtomicBool,
    payload_holder: Weak<PayloadHolder>,
    session_id: i64,
    decoding_context: Mutex<Option<DecodingContext>>,
}
unsafe impl Send for SoftwareDecoder {}
unsafe impl Sync for SoftwareDecoder {}
pub type SharedSendableTexture = Arc<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
impl SoftwareDecoder {
    pub fn new(video_info: &types::VideoInfo, session_id: i64) -> anyhow::Result<(Arc<Self>, Arc<PayloadHolder>)> {
        let payload_holder = Arc::new(PayloadHolder::new());
        let self_ = Arc::new(Self {
            video_info: video_info.clone(),
            kill_sig: AtomicBool::new(false),
            payload_holder: Arc::downgrade(&payload_holder),
            session_id,
            decoding_context: Mutex::new(None),
        });
        Ok((self_, payload_holder))
    }

    pub fn initialize_stream(self: &Arc<Self>) -> anyhow::Result<()> {
        trace!("starting ffmpeg session for {}", &self.video_info.uri);
        let mut option_dict = ffmpeg::Dictionary::new();
        option_dict.set("rtsp_transport", "tcp");
        option_dict.set("timeout", "1"); // 5 seconds timeout
        let ictx = ffmpeg::format::input_with_dictionary(&self.video_info.uri, option_dict)?;
        trace!("got ictx");

        let input = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        trace!("got input stream: {:?}", input);
        let video_stream_index = input.index();
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;

        let decoder = context_decoder.decoder().video()?;

        let scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )?;
        let context = DecodingContext {
            ictx,
            video_stream_index,
            decoder,
            scaler,
        };
        let mut decoding_context = self.decoding_context.lock().unwrap();
        decoding_context.replace(context);
        trace!("ffmpeg session started for {}", &self.video_info.uri);

        Ok(())
    }

    pub fn stream(
        &self,
        sendable_texture: SharedSendableTexture,
        update_stream: DartUpdateStream,
    ) {
        let mut decoding_context = self.decoding_context.lock().unwrap();
        let mut decoding_context = decoding_context
            .take()
            .expect("Decoding context not initialized");

        let sendable_weak = Arc::downgrade(&sendable_texture);
        drop(sendable_texture);
        let cb = move || {
            if let Some(sendable_weak) = sendable_weak.upgrade() {
                sendable_weak.mark_frame_available();
            }
        };
        for (stream, packet) in decoding_context.ictx.packets() {
            if self.kill_sig.load(std::sync::atomic::Ordering::Relaxed) {
                update_stream.add(StreamMessages::Stopped).log_err();
                break;
            }
            
            if stream.index() == decoding_context.video_stream_index {
                let mut packet = packet;
                decoding_context.decoder.send_packet(&mut packet).log_err();
                self.on_new_sample(
                    &mut decoding_context.decoder,
                    &mut decoding_context.scaler,
                    &cb,
                )
                .log_err();
            }
        }
        self.terminate(&mut decoding_context.decoder).log_err();
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
