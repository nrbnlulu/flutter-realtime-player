// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
use std::{
    collections::HashMap,
    fmt, mem,
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
    thread,
    time::Duration,
};

use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture};
use log::{debug, error, info, trace, warn};

use crate::{core::types::DartUpdateStream, dart_types::StreamState, utils::LogErr};

use super::types;

// for raw pixel buffer implementation see https://github.com/nrbnlulu/flutter-realtime-player/blob/fb7d9bd87719b462e7b9e6b32be6e353ba76bcba/rust/src/core/software_decoder.rs#L14

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
    framerate: u32,
}

impl fmt::Debug for DecodingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodingContext")
            .field("video_stream_index", &self.video_stream_index)
            .field("framerate", &self.framerate)
            .field("decoder", &self.decoder.id())
            .finish()
    }
}

#[derive(Debug)]
pub enum StreamExitResult {
    LegalExit,
    EOF,
    Error,
}
pub struct SoftwareDecoder {
    video_info: types::VideoInfo,
    kill_sig: AtomicBool,
    payload_holder: Weak<PayloadHolder>,
    #[allow(unused)]
    session_id: i64,
    decoding_context: Mutex<Option<DecodingContext>>,
    ffmpeg_options: Option<HashMap<String, String>>,
}
unsafe impl Send for SoftwareDecoder {}
unsafe impl Sync for SoftwareDecoder {}
pub type SharedSendableTexture = Arc<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
type WeakSendableTexture = Weak<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
impl SoftwareDecoder {
    /// Validates if a stream is compatible with software scaling
    fn validate_stream_compatibility(
        decoder: &ffmpeg::decoder::Video,
    ) -> Result<(), ffmpeg::Error> {
        let width = decoder.width();
        let height = decoder.height();

        // Validate frame dimensions
        if width == 0 || height == 0 {
            error!("Invalid frame dimensions: {}x{}", width, height);
            return Err(ffmpeg::Error::InvalidData);
        }

        // Warn about potentially problematic dimensions
        if width % 2 != 0 || height % 2 != 0 {
            warn!(
                "Stream has odd dimensions: {}x{}, this might cause issues with some codecs",
                width, height
            );
        }

        // Check for extremely large dimensions that might cause memory issues
        if width > 7680 || height > 4320 {
            warn!(
                "Very large frame dimensions detected: {}x{}, this may cause performance issues",
                width, height
            );
        }

        Ok(())
    }

    /// Creates a scaler with fallback options for maximum compatibility
    fn create_scaler_with_fallbacks(
        src_format: ffmpeg::format::Pixel,
        width: u32,
        height: u32,
        dst_format: ffmpeg::format::Pixel,
    ) -> Result<ffmpeg::software::scaling::Context, ffmpeg::Error> {
        // Try different scaling algorithms in order of preference
        let scaling_flags = [
            ffmpeg::software::scaling::Flags::FAST_BILINEAR,
            ffmpeg::software::scaling::Flags::BILINEAR,
            ffmpeg::software::scaling::Flags::POINT,
            ffmpeg::software::scaling::Flags::AREA,
        ];

        for flag in scaling_flags.iter() {
            match ffmpeg::software::scaling::Context::get(
                src_format, width, height, dst_format, width, height, *flag,
            ) {
                Ok(scaler) => {
                    if !flag.contains(ffmpeg::software::scaling::Flags::FAST_BILINEAR)
                        && !flag.contains(ffmpeg::software::scaling::Flags::BILINEAR)
                    {
                        warn!("Using fallback scaling algorithm: {:?}", flag);
                    }
                    return Ok(scaler);
                }
                Err(e) => {
                    warn!("Scaling with {:?} failed: {}", flag, e);
                }
            }
        }

        Err(ffmpeg::Error::InvalidData)
    }

    pub fn new(
        video_info: &types::VideoInfo,
        session_id: i64,
        ffmpeg_options: Option<HashMap<String, String>>,
    ) -> (Arc<Self>, Arc<PayloadHolder>) {
        let payload_holder = Arc::new(PayloadHolder::new());
        let self_ = Arc::new(Self {
            video_info: video_info.clone(),
            kill_sig: AtomicBool::new(false),
            payload_holder: Arc::downgrade(&payload_holder),
            session_id,
            decoding_context: Mutex::new(None),
            ffmpeg_options: ffmpeg_options,
        });
        (self_, payload_holder)
    }

    pub fn initialize_stream(&self) -> Result<(), ffmpeg::Error> {
        trace!("starting ffmpeg session for {}", &self.video_info.uri);
        let mut option_dict = ffmpeg::Dictionary::new();
        if let Some(ref options) = self.ffmpeg_options {
            for (key, value) in options {
                option_dict.set(key, value);
            }
        }
        let ictx = ffmpeg::format::input_with_dictionary(&self.video_info.uri, option_dict)?;
        let input = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input.index();
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let decoder = context_decoder.decoder().video()?;
        trace!(
            "got decoder(
            format={:?},
            width={:?},
            height={:?},
            frame_rate={:?}
            ",
            decoder.format(),
            decoder.width(),
            decoder.height(),
            decoder.frame_rate()
        );

        // Validate stream compatibility
        Self::validate_stream_compatibility(&decoder)?;

        // Create scaler with fallback options
        let src_format = decoder.format();
        let dst_format = ffmpeg::format::Pixel::RGBA;
        let width = decoder.width();
        let height = decoder.height();

        let scaler = Self::create_scaler_with_fallbacks(src_format, width, height, dst_format)
            .map_err(|e| {
                error!(
                    "Failed to create scaler for stream {}: {}",
                    &self.video_info.uri, e
                );
                e
            })?;
        let frame_rate = input.avg_frame_rate().numerator();
        let context = DecodingContext {
            ictx,
            video_stream_index,
            decoder,
            scaler,
            framerate: frame_rate as _,
        };
        debug!(
            "created context {:?} for url: {}",
            context, &self.video_info.uri
        );
        let mut decoding_context = self.decoding_context.lock().unwrap();
        decoding_context.replace(context);
        trace!("ffmpeg session started for {}", &self.video_info.uri);

        Ok(())
    }
    fn asked_for_termination(&self) -> bool {
        self.kill_sig.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn stream(
        &self,
        sendable_texture: SharedSendableTexture,
        dart_update_stream: DartUpdateStream,
        texture_id: i64,
    ) -> Result<(), StreamExitResult> {
        let weak_sendable_texture: WeakSendableTexture = Arc::downgrade(&sendable_texture);
        drop(sendable_texture); // drop the strong reference to allow cleanup
        loop {
            if self.asked_for_termination() {
                return Ok(());
            }
            trace!("ffmpeg session initializing for {}", &self.video_info.uri);
            dart_update_stream.add(StreamState::Loading).log_err();
            if let Err(e) = self.initialize_stream() {
                info!(
                    "Failed to reinitialize stream({}): {}",
                    &self.video_info.uri, e
                );
                dart_update_stream
                    .add(StreamState::Error(format!(
                        "stream({}) initialization failed: {}, reinitializing in 2 seconds",
                        &self.video_info.uri, e
                    )))
                    .log_err();
            } else {
                info!("stream({}) successfully initialized", &self.video_info.uri);
                let res = self.stream_impl(&weak_sendable_texture, &dart_update_stream, texture_id);
                if matches!(res, StreamExitResult::LegalExit | StreamExitResult::EOF) {
                    break; // no need to reinitialize
                }
            }
            thread::sleep(Duration::from_millis(2000));
        }
        let _ = dart_update_stream.add(StreamState::Stopped);
        Ok(())
    }

    fn stream_impl(
        &self,
        sendable_weak: &WeakSendableTexture,
        dart_update_stream: &DartUpdateStream,
        texture_id: i64,
    ) -> StreamExitResult {
        let mut decoding_context = self.decoding_context.lock().unwrap();
        let mut decoding_context = decoding_context
            .take()
            .expect("Decoding context not initialized");

        let cb = move || {
            if let Some(sendable_weak) = sendable_weak.upgrade() {
                sendable_weak.mark_frame_available();
            }
        };
        let mut first_frame = true;

        loop {
            if self.asked_for_termination() {
                let _ = dart_update_stream.add(StreamState::Stopped);
                trace!("stream killed, exiting");
                self.terminate(&mut decoding_context.decoder).log_err();
                return StreamExitResult::LegalExit;
            }
            let mut packet = ffmpeg::Packet::empty();
            match packet.read(&mut decoding_context.ictx) {
                Ok(_) => unsafe {
                    let stream = ffmpeg::format::stream::Stream::wrap(
                        // this somehow gets the raw C ptr to the stream..
                        mem::transmute_copy(&&decoding_context.ictx),
                        packet.stream(),
                    );
                    if packet.is_corrupt() || packet.is_empty() {
                        trace!(
                            "stream({}) received empty or corrupt packet, skipping",
                            self.video_info.uri
                        );
                        continue;
                    }
                    if stream.index() == decoding_context.video_stream_index {
                        match decoding_context.decoder.send_packet(&mut packet) {
                            Ok(_) => {
                                self.on_new_sample(
                                    &mut decoding_context.decoder,
                                    &mut decoding_context.scaler,
                                    &cb,
                                )
                                .log_err();
                                if first_frame {
                                    first_frame = false;
                                    trace!(
                                        "First frame received for stream {}, marking as playing",
                                        &self.video_info.uri
                                    );
                                    let _ =
                                        dart_update_stream.add(StreamState::Playing { texture_id });
                                }
                            }
                            Err(err) => {
                                error!(
                                    "Error sending packet to decoder for stream {}: {}",
                                    &self.video_info.uri, err
                                );
                            }
                        }
                    }
                },

                Err(ffmpeg::Error::Eof) => {
                    info!("Stream {} reached end of file", &self.video_info.uri);
                    self.terminate(&mut decoding_context.decoder).log_err();
                    return StreamExitResult::EOF;
                }
                Err(ffmpeg::Error::Other {
                    errno: ffmpeg::error::EAGAIN,
                }) => {
                    // EAGAIN means try again, so just continue the loop
                    // Decoder is not ready, try again later
                    trace!("EAGAIN received, retrying packet read");
                    continue;
                }
                Err(..) => {
                    info!("Failed to get frame");
                    let _ = dart_update_stream.add(StreamState::Error(
                        "stream corrupted, reinitializing".to_owned(),
                    ));
                    return StreamExitResult::Error;
                }
            }
        }
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
            if self.asked_for_termination() {
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
