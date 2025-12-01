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
pub struct FFmpegFrameWrapper(ffmpeg::util::frame::Video, Option<Vec<u8>>);

impl irondash_texture::PixelDataProvider for FFmpegFrameWrapper {
    fn get(&self) -> irondash_texture::PixelData {
        let width = self.0.width() as usize;
        let height = self.0.height() as usize;

        // If we have a pre-copied contiguous buffer, use it
        if let Some(ref buffer) = self.1 {
            irondash_texture::PixelData {
                width: width as _,
                height: height as _,
                data: buffer.as_slice(),
            }
        } else {
            // No buffer means stride was already correct, use original data
            irondash_texture::PixelData {
                width: width as _,
                height: height as _,
                data: self.0.data(0),
            }
        }
    }
}

pub struct PayloadHolder {
    current_frame: Mutex<Option<Box<FFmpegFrameWrapper>>>,
    previous_frame: Mutex<Option<Box<FFmpegFrameWrapper>>>,
}
impl FFmpegFrameWrapper {
    /// Create a wrapper from a frame, copying data if stride doesn't match width
    fn from_frame(frame: ffmpeg::util::frame::Video) -> Self {
        let width = frame.width() as usize;
        let height = frame.height() as usize;
        let stride = frame.stride(0);
        let expected_stride = width * 4; // RGBA = 4 bytes per pixel

        if stride == expected_stride {
            // No padding, can use frame directly
            Self(frame, None)
        } else {
            // Stride mismatch - copy data row by row without padding
            warn!(
                "Stride mismatch detected! Width: {}, Expected stride: {}, Actual stride: {}. Copying to contiguous buffer.",
                width, expected_stride, stride
            );

            let mut buffer = Vec::with_capacity(width * height * 4);
            let data = frame.data(0);

            for y in 0..height {
                let row_start = y * stride;
                let row_end = row_start + expected_stride;
                buffer.extend_from_slice(&data[row_start..row_end]);
            }

            Self(frame, Some(buffer))
        }
    }
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
        let curr_frame = self.current_frame.lock().unwrap();
        if let Some(ref frame) = *curr_frame {
            // Clone instead of take to keep frame available for resize operations
            frame.clone()
        } else {
            // Try to return a clone of the previous frame if it exists
            let prev_frame = self.previous_frame.lock().unwrap();
            if let Some(ref prev) = *prev_frame {
                debug!("returning previous frame");
                prev.clone()
            } else {
                debug!("no frame available returning a default");
                // return empty frame
                Box::new(FFmpegFrameWrapper::from_frame(
                    ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, 640, 480),
                ))
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
    current_time: Arc<Mutex<f64>>, // Track current playback time in seconds
    seek_request: Arc<Mutex<Option<i64>>>, // Pending seek timestamp in microseconds (AV_TIME_BASE)
    seekable: Arc<Mutex<bool>>,    // Whether the stream is seekable
    output_dimensions: Arc<Mutex<types::VideoDimensions>>, // Track current output dimensions for dynamic resize
    sendable_texture:
        Arc<Mutex<Option<Weak<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>>>>,
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
        src_width: u32,
        src_height: u32,
        dst_format: ffmpeg::format::Pixel,
        dst_width: u32,
        dst_height: u32,
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
                src_format, src_width, src_height, dst_format, dst_width, dst_height, *flag,
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
            ffmpeg_options,
            current_time: Arc::new(Mutex::new(0.0)),
            seek_request: Arc::new(Mutex::new(None)),
            seekable: Arc::new(Mutex::new(false)),
            output_dimensions: Arc::new(Mutex::new(video_info.dimensions.clone())),
            sendable_texture: Arc::new(Mutex::new(None)),
        });
        (self_, payload_holder)
    }

    pub fn set_sendable_texture(
        &self,
        texture: Weak<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>,
    ) {
        let mut texture_ref = self.sendable_texture.lock().unwrap();
        *texture_ref = Some(texture);
    }

    pub fn seek_to(&self, time_seconds: f64) {
        if time_seconds < 0.0 {
            warn!("Seek requested to negative time: {}", time_seconds);
            return; // Invalid seek
        }

        let seekable = *self.seekable.lock().unwrap();
        if !seekable {
            warn!("Seek requested but stream is not seekable");
            return;
        }

        // Check if we're already at the requested time to avoid unnecessary seeks
        let current_time = *self.current_time.lock().unwrap();

        // Check if the stream has a duration limit and validate the seek time
        let decoding_context = self.decoding_context.lock().unwrap();
        if let Some(ref context) = *decoding_context {
            let duration = context.ictx.duration();
            if duration > 0 {
                let duration_seconds = duration as f64 / 1_000_000.0; // Convert from AV_TIME_BASE
                if time_seconds > duration_seconds {
                    warn!(
                        "Seek requested beyond stream duration ({} > {})",
                        time_seconds, duration_seconds
                    );
                    // Seek to end instead of failing
                    let clamped_time = duration_seconds.max(0.0);
                    if (current_time - clamped_time).abs() < 0.1 {
                        info!("Already at end time ({}), skipping seek", clamped_time);
                        return;
                    }
                    let ts = (clamped_time * 1_000_000.0) as i64;
                    let mut seek = self.seek_request.lock().unwrap();
                    *seek = Some(ts);
                    info!("Seek clamped to end ({} seconds, {} us)", clamped_time, ts);
                    return;
                }
            }
        }
        drop(decoding_context); // Release the lock early

        if (current_time - time_seconds).abs() < 0.1 {
            // Within 0.1 second tolerance
            info!(
                "Already at requested time ({}), skipping seek",
                time_seconds
            );
            return;
        }

        let ts = (time_seconds * 1_000_000.0) as i64; // Convert to microseconds (AV_TIME_BASE)
        let mut seek = self.seek_request.lock().unwrap();
        *seek = Some(ts);
        info!(
            "Seek requested to {} seconds ({} us), current time: {}",
            time_seconds, ts, current_time
        );
    }
    pub fn get_current_time(&self) -> anyhow::Result<f64> {
        let current_time = self.current_time.lock().unwrap();
        Ok(*current_time)
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
        let src_width = decoder.width();
        let src_height = decoder.height();
        // Use target dimensions from video_info for output
        let target_width = self.video_info.dimensions.width;
        let target_height = self.video_info.dimensions.height;

        let scaler = Self::create_scaler_with_fallbacks(
            src_format,
            src_width,
            src_height,
            dst_format,
            target_width,
            target_height,
        )
        .inspect_err(|e| {
            error!(
                "Failed to create scaler for stream {}: {}",
                &self.video_info.uri, e
            );
        })?;
        let avg_frame_rate = input.avg_frame_rate();
        let frame_rate = if avg_frame_rate.denominator() != 0 {
            avg_frame_rate.numerator() as f64 / avg_frame_rate.denominator() as f64
        } else {
            0.0
        };
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
        let duration = context.ictx.duration();
        let is_seekable = duration > 0;
        info!("Stream duration: {}, seekable: {}", duration, is_seekable);
        let mut decoding_context = self.decoding_context.lock().unwrap();
        decoding_context.replace(context);
        let mut seekable = self.seekable.lock().unwrap();
        *seekable = is_seekable;
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
                match res {
                    StreamExitResult::LegalExit | StreamExitResult::EOF => {
                        if self.video_info.auto_restart {
                            dart_update_stream.add(StreamState::Stopped).log_err();
                            thread::sleep(Duration::from_millis(800));
                            continue;
                        }
                        break;
                    }
                    _ => continue,
                };
            }
            thread::sleep(Duration::from_millis(2000));
        }
        dart_update_stream.add(StreamState::Stopped).log_err();
        Ok(())
    }

    fn stream_impl(
        &self,
        sendable_weak: &WeakSendableTexture,
        dart_update_stream: &DartUpdateStream,
        texture_id: i64,
    ) -> StreamExitResult {
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
                // Flush decoder on exit
                if let Some(mut ctx_guard) = self.decoding_context.lock().ok() {
                    if let Some(ctx) = &mut *ctx_guard {
                        self.terminate(&mut ctx.decoder).log_err();
                    }
                }
                return StreamExitResult::LegalExit;
            }

            // Lock context for this iteration (per packet/frame processing)
            let mut ctx_guard = match self.decoding_context.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("Failed to lock decoding context: {}", e);
                    return StreamExitResult::Error;
                }
            };

            let ctx = match &mut *ctx_guard {
                Some(ctx) => ctx,
                None => {
                    error!("Decoding context not available");
                    return StreamExitResult::Error;
                }
            };

            // Check for seek request before reading packet
            {
                let mut seek = self.seek_request.lock().unwrap();
                if let Some(ts) = seek.take() {
                    info!("Performing seek to {} us", ts);
                    // Seek the input context (wide range for any direction)
                    match ctx.ictx.seek(ts, 0..5) {
                        Ok(_) => {
                            info!("Seek successful to timestamp: {}", ts);
                            // Flush decoder to clear buffered frames
                            ctx.decoder.flush();
                            // Clear payload holder to avoid stale frames
                            if let Some(holder) = self.payload_holder.upgrade() {
                                holder.current_frame.lock().unwrap().take();
                                holder.previous_frame.lock().unwrap().take();
                            }
                            // Update current_time based on the seek target
                            *self.current_time.lock().unwrap() = (ts as f64) / 1_000_000.0;
                        }
                        Err(e) => {
                            error!("Seek failed to timestamp {}: {}", ts, e);
                            dart_update_stream
                                .add(StreamState::Error(format!("Seek failed: {}", e)))
                                .log_err();
                        }
                    }
                }
            }

            let mut packet = ffmpeg::Packet::empty();
            match packet.read(&mut ctx.ictx) {
                Ok(_) => unsafe {
                    let stream = ffmpeg::format::stream::Stream::wrap(
                        // this somehow gets the raw C ptr to the stream..
                        mem::transmute_copy(&&ctx.ictx),
                        packet.stream(),
                    );
                    if packet.is_corrupt() || packet.is_empty() {
                        trace!(
                            "stream({}) received empty or corrupt packet, skipping",
                            self.video_info.uri
                        );
                        continue;
                    }
                    if stream.index() == ctx.video_stream_index {
                        match ctx.decoder.send_packet(&packet) {
                            Ok(_) => {
                                self.on_new_sample(&mut ctx.decoder, &mut ctx.scaler, &cb)
                                    .log_err();
                                if first_frame {
                                    first_frame = false;
                                    trace!(
                                        "First frame received for stream {}, marking as playing",
                                        &self.video_info.uri
                                    );
                                    let _ = dart_update_stream.add(StreamState::Playing {
                                        texture_id,
                                        seekable: *self.seekable.lock().unwrap(),
                                    });
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
                    self.terminate(&mut ctx.decoder).log_err();
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

            // Drop guard to release lock after processing this packet
            drop(ctx_guard);
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
        T: Fn(),
    {
        let mut decoded = ffmpeg::util::frame::Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
            // Update current time from frame PTS (convert from time_base to seconds)
            if let Some(pts) = decoded.pts() {
                let time_base = decoder.time_base();
                let time_seconds =
                    pts as f64 * (time_base.numerator() as f64 / time_base.denominator() as f64);

                // Only update current time if there's no pending seek (to avoid overwriting seek target)
                let mut current_time = self.current_time.lock().unwrap();
                let pending_seek = self.seek_request.lock().unwrap().is_some();
                if !pending_seek {
                    *current_time = time_seconds;
                }
            }

            let mut rgb_frame = ffmpeg::util::frame::Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;

            // Log scaled frame dimensions for debugging
            let frame_width = rgb_frame.width();
            let frame_height = rgb_frame.height();
            let frame_stride = rgb_frame.stride(0);
            let expected_dims = self.output_dimensions.lock().unwrap().clone();

            if frame_width != expected_dims.width || frame_height != expected_dims.height {
                error!(
                    "Frame dimension mismatch! Scaler produced {}x{} but expected {}x{}",
                    frame_width, frame_height, expected_dims.width, expected_dims.height
                );
            }

            trace!(
                "Scaled frame to {}x{} (stride: {}) for stream {}",
                frame_width,
                frame_height,
                frame_stride,
                &self.video_info.uri
            );

            match self.payload_holder.upgrade() {
                Some(payload_holder) => {
                    payload_holder.set_payload(Box::new(FFmpegFrameWrapper::from_frame(rgb_frame)));
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
        let mut seek = self.seek_request.lock().unwrap();
        *seek = None;
    }

    pub fn resize_stream(&self, mut new_width: u32, mut new_height: u32) -> anyhow::Result<()> {
        // Validate dimensions
        if new_width == 0 || new_height == 0 {
            return Err(anyhow::anyhow!(
                "Invalid resize dimensions: {}x{}",
                new_width,
                new_height
            ));
        }

        // Sanitize dimensions to be even (required by most video codecs)
        // Round down to nearest even number to maintain aspect ratio better
        let original_width = new_width;
        let original_height = new_height;

        if new_width % 2 != 0 {
            new_width = new_width.saturating_sub(1).max(2);
            debug!(
                "Adjusted width from {} to {} (must be even)",
                original_width, new_width
            );
        }

        if new_height % 2 != 0 {
            new_height = new_height.saturating_sub(1).max(2);
            debug!(
                "Adjusted height from {} to {} (must be even)",
                original_height, new_height
            );
        }

        // Validate aspect ratio - warn if extremely stretched
        let aspect_ratio = new_width as f64 / new_height as f64;
        if aspect_ratio < 0.1 || aspect_ratio > 10.0 {
            warn!(
                "Unusual aspect ratio detected: {:.2} ({}x{}). This may produce distorted output.",
                aspect_ratio, new_width, new_height
            );
        }

        // Validate maximum dimensions to prevent memory issues
        const MAX_DIMENSION: u32 = 7680; // 8K width
        if new_width > MAX_DIMENSION || new_height > MAX_DIMENSION {
            return Err(anyhow::anyhow!(
                "Dimensions too large: {}x{} (max: {}x{})",
                new_width,
                new_height,
                MAX_DIMENSION,
                MAX_DIMENSION
            ));
        }

        // Validate minimum dimensions
        const MIN_DIMENSION: u32 = 2;
        if new_width < MIN_DIMENSION || new_height < MIN_DIMENSION {
            return Err(anyhow::anyhow!(
                "Dimensions too small: {}x{} (min: {}x{})",
                new_width,
                new_height,
                MIN_DIMENSION,
                MIN_DIMENSION
            ));
        }

        // Get old dimensions before updating
        let old_dims = {
            let dims = self.output_dimensions.lock().unwrap();
            dims.clone()
        };

        info!(
            "Resizing stream from {}x{} to {}x{} for session {}",
            old_dims.width, old_dims.height, new_width, new_height, self.session_id
        );

        // Early exit if dimensions haven't actually changed
        if old_dims.width == new_width && old_dims.height == new_height {
            debug!("Resize requested but dimensions unchanged, skipping");
            return Ok(());
        }

        // Lock decoding context and update scaler
        let mut ctx_guard = self.decoding_context.lock().unwrap();
        let ctx = ctx_guard
            .as_mut()
            .ok_or(anyhow::anyhow!("Decoding context not initialized"))?;

        // Get the original source format and dimensions
        let src_format = ctx.decoder.format();
        let src_width = ctx.decoder.width();
        let src_height = ctx.decoder.height();

        // Create a new scaler with the requested output dimensions
        let new_scaler = Self::create_scaler_with_fallbacks(
            src_format,
            src_width,
            src_height,
            ffmpeg::format::Pixel::RGBA,
            new_width,
            new_height,
        )
        .inspect_err(|e| error!("Failed to create new scaler: {}", e))?;

        // Replace the scaler in the context
        ctx.scaler = new_scaler;
        info!("Scaler updated to new dimensions");

        // Drop the context lock early to avoid holding it during frame processing
        drop(ctx_guard);

        // Update the output dimensions
        {
            let mut output_dims = self.output_dimensions.lock().unwrap();
            *output_dims = types::VideoDimensions {
                width: new_width,
                height: new_height,
            };
        }

        // Update the payload holder with a frame of new dimensions to ensure the texture can handle it
        if let Some(holder) = self.payload_holder.upgrade() {
            // Rescale the previous frame if available to maintain visual continuity
            let prev_opt = holder.previous_frame.lock().unwrap().clone();
            if let Some(prev) = prev_opt {
                let prev_width = prev.0.width();
                let prev_height = prev.0.height();
                info!(
                    "Rescaling previous frame from {}x{} to {}x{}",
                    prev_width, prev_height, new_width, new_height
                );

                // Verify previous frame dimensions match what we expect
                if prev_width != old_dims.width || prev_height != old_dims.height {
                    warn!(
                        "Previous frame dimensions ({}x{}) don't match expected old dimensions ({}x{})",
                        prev_width, prev_height, old_dims.width, old_dims.height
                    );
                }

                // Create temporary scaler to rescale previous frame from old to new dimensions
                let temp_scaler = Self::create_scaler_with_fallbacks(
                    ffmpeg::format::Pixel::RGBA,
                    prev_width, // Use actual frame dimensions, not old_dims
                    prev_height,
                    ffmpeg::format::Pixel::RGBA,
                    new_width,
                    new_height,
                );

                if let Ok(mut temp_scaler) = temp_scaler {
                    let mut resized_frame = ffmpeg::util::frame::Video::empty();
                    match temp_scaler.run(&prev.0, &mut resized_frame) {
                        Ok(_) => {
                            let result_width = resized_frame.width();
                            let result_height = resized_frame.height();
                            info!(
                                "Successfully rescaled previous frame to {}x{} (expected {}x{})",
                                result_width, result_height, new_width, new_height
                            );

                            // Verify the output dimensions are correct
                            if result_width != new_width || result_height != new_height {
                                error!(
                                    "Scaler produced wrong dimensions! Expected {}x{}, got {}x{}",
                                    new_width, new_height, result_width, result_height
                                );
                                self.create_black_frame(new_width, new_height, &holder);
                            } else {
                                holder.set_payload(Box::new(FFmpegFrameWrapper::from_frame(
                                    resized_frame,
                                )));
                            }
                        }
                        Err(e) => {
                            error!("Failed to rescale previous frame: {}", e);
                            self.create_black_frame(new_width, new_height, &holder);
                        }
                    }
                } else {
                    warn!("Failed to create temp scaler, using black fallback");
                    self.create_black_frame(new_width, new_height, &holder);
                }
            } else {
                info!("No previous frame available, using black fallback");
                self.create_black_frame(new_width, new_height, &holder);
            }
        }

        // Immediately signal the texture that a new frame is available
        {
            let texture_ref = self.sendable_texture.lock().unwrap();
            if let Some(ref weak_texture) = *texture_ref {
                if let Some(texture) = weak_texture.upgrade() {
                    texture.mark_frame_available();
                    info!("Marked frame available after resize");
                }
            }
        }

        Ok(())
    }

    /// Helper function to create and set a black frame with specified dimensions
    fn create_black_frame(&self, width: u32, height: u32, holder: &Arc<PayloadHolder>) {
        info!("Creating black frame with dimensions {}x{}", width, height);
        let mut new_frame =
            ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, width, height);

        // Verify the frame was created with correct dimensions
        let actual_width = new_frame.width();
        let actual_height = new_frame.height();
        if actual_width != width || actual_height != height {
            error!(
                "Black frame created with wrong dimensions! Expected {}x{}, got {}x{}",
                width, height, actual_width, actual_height
            );
        }

        // Fill with black pixels (RGBA: 0, 0, 0, 255) to avoid garbage data
        let data = new_frame.data_mut(0);
        let expected_size = (width * height * 4) as usize;
        if data.len() < expected_size {
            error!(
                "Black frame buffer too small! Expected {} bytes, got {}",
                expected_size,
                data.len()
            );
        }

        for chunk in data.chunks_mut(4) {
            if chunk.len() >= 4 {
                chunk[0] = 0; // R
                chunk[1] = 0; // G
                chunk[2] = 0; // B
                chunk[3] = 255; // A
            }
        }

        info!(
            "Black frame created successfully: {}x{}",
            actual_width, actual_height
        );
        holder.set_payload(Box::new(FFmpegFrameWrapper::from_frame(new_frame)));
    }
}
