// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
use std::{
    char::DecodeUtf16,
    collections::HashMap,
    fmt, mem,
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
    thread,
    time::{Duration, Instant},
};

use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture};
use log::{debug, error, info, trace, warn};

use crate::{core::types::DartUpdateStream, dart_types::StreamState, utils::LogErr};

use super::types;

#[derive(Clone)]
pub struct FFmpegFrameWrapper(ffmpeg::util::frame::Video, Option<Vec<u8>>);

impl irondash_texture::PixelDataProvider for FFmpegFrameWrapper {
    fn get(&self) -> irondash_texture::PixelData {
        let width = self.0.width() as usize;
        let height = self.0.height() as usize;

        if let Some(ref buffer) = self.1 {
            irondash_texture::PixelData {
                width: width as _,
                height: height as _,
                data: buffer.as_slice(),
            }
        } else {
            irondash_texture::PixelData {
                width: width as _,
                height: height as _,
                data: self.0.data(0),
            }
        }
    }
}

impl FFmpegFrameWrapper {
    /// Create a wrapper from a frame, copying data if stride doesn't match width.
    /// ffmpeg frames may have padding at the end of each row, resulting in a stride
    /// that is larger than `width * bytes_per_pixel`. This function handles such cases
    /// by creating a tightly packed buffer.
    fn from_frame(frame: ffmpeg::util::frame::Video) -> Self {
        let width = frame.width() as usize;
        let height = frame.height() as usize;
        let stride = frame.stride(0);
        let expected_stride = width * 4; // RGBA = 4 bytes per pixel

        if stride == expected_stride {
            // No padding, can use frame data directly.
            Self(frame, None)
        } else {
            // Stride mismatch - copy data row by row to remove padding.
            trace!(
                "Stride mismatch! width: {}, expected stride: {}, actual stride: {}. Copying to contiguous buffer.",
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
        let mut curr_frame = match self.current_frame.lock() {
            Ok(lock) => lock,
            Err(e) => {
                error!("current_frame mutex poisoned in set_payload: {}", e);
                return;
            }
        };
        let mut prev_frame = match self.previous_frame.lock() {
            Ok(lock) => lock,
            Err(e) => {
                error!("previous_frame mutex poisoned in set_payload: {}", e);
                return;
            }
        };
        // Move current to previous before replacing.
        *prev_frame = curr_frame.take();
        *curr_frame = Some(payload);
    }
}

impl PayloadProvider<BoxedPixelData> for PayloadHolder {
    fn get_payload(&self) -> BoxedPixelData {
        // Create a default frame to return on error or if no frame is available.
        let default_frame = || {
            debug!("No frame available, returning a default black frame.");
            Box::new(FFmpegFrameWrapper::from_frame(
                ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, 640, 480),
            ))
        };

        let curr_frame_lock = self.current_frame.lock();
        if let Ok(curr_frame) = curr_frame_lock {
            if let Some(ref frame) = *curr_frame {
                // Clone instead of take to keep frame available for resize operations.
                return frame.clone();
            }
        } else {
            error!("current_frame mutex poisoned in get_payload");
            return default_frame();
        }

        let prev_frame_lock = self.previous_frame.lock();
        if let Ok(prev_frame) = prev_frame_lock {
            if let Some(ref frame) = *prev_frame {
                debug!("Returning previous frame");
                return frame.clone();
            }
        } else {
            error!("previous_frame mutex poisoned in get_payload");
        }

        default_frame()
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

struct FrameSynchronizer {
    start_system_time: Instant,
    start_stream_time: f64,
    frames_processed: u64,
}

impl FrameSynchronizer {
    fn new(current_pts_seconds: f64) -> Self {
        Self {
            start_system_time: Instant::now(),
            start_stream_time: current_pts_seconds,
            frames_processed: 0,
        }
    }

    /// Returns the duration the thread should sleep to synchronize with the video's PTS.
    fn get_sleep_duration(&mut self, pts_seconds: f64, framerate: f64) -> Option<Duration> {
        self.frames_processed += 1;

        // If PTS is unreliable (e.g., stuck at 0), estimate the current time
        // based on the frame rate.
        let mut effective_time = pts_seconds;
        if effective_time <= 0.001 && self.frames_processed > 1 {
            if framerate > 0.0 {
                effective_time = (self.frames_processed as f64) / framerate;
            }
        }

        let stream_elapsed = effective_time - self.start_stream_time;
        let system_elapsed = self.start_system_time.elapsed().as_secs_f64();

        if stream_elapsed > system_elapsed {
            let diff = stream_elapsed - system_elapsed;
            if diff > 0.001 {
                return Some(Duration::from_secs_f64(diff));
            }
        }
        None
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
    seek_request: Arc<Mutex<Option<i64>>>, // Pending seek timestamp in microseconds
    seekable: Arc<AtomicBool>,     // Whether the stream is seekable
    output_dimensions: Arc<Mutex<types::VideoDimensions>>, // Track current output dimensions
    sendable_texture: Arc<Mutex<Option<WeakSendableTexture>>>,
    synchronizer: Mutex<Option<FrameSynchronizer>>, // For frame timing synchronization
}
unsafe impl Send for SoftwareDecoder {}
unsafe impl Sync for SoftwareDecoder {}
pub type SharedSendableTexture = Arc<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
type WeakSendableTexture = Weak<SendableTexture<Box<dyn irondash_texture::PixelDataProvider>>>;
impl SoftwareDecoder {
    /// Validates if a stream is compatible with software scaling.
    fn validate_stream_compatibility(
        decoder: &ffmpeg::decoder::Video,
    ) -> Result<(), ffmpeg::Error> {
        let width = decoder.width();
        let height = decoder.height();

        if width == 0 || height == 0 {
            error!("Invalid frame dimensions: {}x{}", width, height);
            return Err(ffmpeg::Error::InvalidData);
        }
        if width % 2 != 0 || height % 2 != 0 {
            warn!(
                "Stream has odd dimensions: {}x{}, this might cause issues with some codecs.",
                width, height
            );
        }
        if width > 7680 || height > 4320 {
            warn!(
                "Very large frame dimensions detected: {}x{}, may cause performance issues.",
                width, height
            );
        }

        Ok(())
    }

    /// Creates a scaler with fallback options for maximum compatibility.
    fn create_scaler_with_fallbacks(
        src_format: ffmpeg::format::Pixel,
        src_width: u32,
        src_height: u32,
        dst_format: ffmpeg::format::Pixel,
        dst_width: u32,
        dst_height: u32,
    ) -> Result<ffmpeg::software::scaling::Context, ffmpeg::Error> {
        let scaling_flags = [
            ffmpeg::software::scaling::Flags::FAST_BILINEAR,
            ffmpeg::software::scaling::Flags::BILINEAR,
            ffmpeg::software::scaling::Flags::POINT,
        ];
        for flag in scaling_flags.iter() {
            match ffmpeg::software::scaling::Context::get(
                src_format, src_width, src_height, dst_format, dst_width, dst_height, *flag,
            ) {
                Ok(scaler) => {
                    if !flag.contains(ffmpeg::software::scaling::Flags::FAST_BILINEAR) {
                        warn!("Using fallback scaling algorithm: {:?}", flag);
                    }
                    return Ok(scaler);
                }
                Err(e) => {
                    warn!("Scaling with {:?} failed: {}", flag, e);
                }
            }
        }
        error!("All scaling attempts failed.");
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
            seekable: Arc::new(AtomicBool::new(false)),
            output_dimensions: Arc::new(Mutex::new(video_info.dimensions.clone())),
            sendable_texture: Arc::new(Mutex::new(None)),
            synchronizer: Mutex::new(None),
        });
        (self_, payload_holder)
    }

    pub fn set_sendable_texture(&self, texture: WeakSendableTexture) {
        if let Ok(mut texture_ref) = self.sendable_texture.lock() {
            *texture_ref = Some(texture);
        } else {
            error!("sendable_texture mutex poisoned in set_sendable_texture");
        }
    }

    pub fn seek_to(&self, time_seconds: f64) {
        if time_seconds < 0.0 {
            warn!("Seek requested to negative time: {}", time_seconds);
            return;
        }

        if !self.seekable.load(std::sync::atomic::Ordering::Relaxed) {
            warn!("Seek requested but stream is not seekable");
            return;
        }

        let current_time = self.current_time.lock().map(|guard| *guard).unwrap_or(0.0);

        if (current_time - time_seconds).abs() < 0.1 {
            info!(
                "Already at requested time ({}), skipping seek",
                time_seconds
            );
            return;
        }

        // Convert to AV_TIME_BASE and set seek request
        let ts = (time_seconds * 1_000_000.0) as i64;
        if let Ok(mut seek) = self.seek_request.lock() {
            *seek = Some(ts);
            info!(
                "Seek requested to {} seconds ({} us), current time: {}",
                time_seconds, ts, current_time
            );
        } else {
            error!("seek_request mutex poisoned in seek_to");
        }
    }

    pub fn get_current_time(&self) -> anyhow::Result<f64> {
        self.current_time
            .lock()
            .map(|guard| *guard)
            .map_err(|e| anyhow::anyhow!("current_time mutex poisoned: {}", e))
    }

    pub fn initialize_stream(&self) -> Result<(), ffmpeg::Error> {
        trace!("Starting ffmpeg session for {}", &self.video_info.uri);
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
        let mut decoder = context_decoder.decoder().video()?;

        decoder.set_threading(ffmpeg::threading::Config {
            kind: ffmpeg::threading::Type::Frame,
            count: 0, // auto-detect
            ..Default::default()
        });

        Self::validate_stream_compatibility(&decoder)?;

        let src_format = decoder.format();
        let dst_format = ffmpeg::format::Pixel::RGBA;
        let src_width = decoder.width();
        let src_height = decoder.height();
        let target_dims = self
            .output_dimensions
            .lock()
            .map(|d| d.clone())
            .unwrap_or_else(|_| self.video_info.dimensions.clone());

        let scaler = Self::create_scaler_with_fallbacks(
            src_format,
            src_width,
            src_height,
            dst_format,
            target_dims.width,
            target_dims.height,
        )?;

        let avg_frame_rate = input.avg_frame_rate();
        let frame_rate = if avg_frame_rate.denominator() != 0 {
            (avg_frame_rate.numerator() as f64 / avg_frame_rate.denominator() as f64) as u32
        } else {
            0
        };

        let context = DecodingContext {
            ictx,
            video_stream_index,
            decoder,
            scaler,
            framerate: frame_rate,
        };
        debug!(
            "Created context {:?} for url: {}",
            context, &self.video_info.uri
        );
        let duration = context.ictx.duration();
        let is_seekable = duration > 0;
        info!("Stream duration: {}, seekable: {}", duration, is_seekable);

        if let Ok(mut decoding_context) = self.decoding_context.lock() {
            *decoding_context = Some(context);
            self.seekable
                .store(is_seekable, std::sync::atomic::Ordering::Relaxed);
            trace!("FFmpeg session started for {}", &self.video_info.uri);
            Ok(())
        } else {
            error!("decoding_context mutex poisoned in initialize_stream");
            Err(ffmpeg::Error::Unknown)
        }
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
        drop(sendable_texture); // Drop strong reference

        while !self.asked_for_termination() {
            trace!("FFmpeg session initializing for {}", &self.video_info.uri);
            dart_update_stream.add(StreamState::Loading).log_err();

            if let Err(e) = self.initialize_stream() {
                let error_msg =
                    format!("Stream initialization failed: {}, reinitializing in 2s", e);
                error!("{} for uri: {}", error_msg, &self.video_info.uri);
                dart_update_stream
                    .add(StreamState::Error(error_msg))
                    .log_err();
                thread::sleep(Duration::from_millis(2000));
                continue;
            }

            info!("Stream initialized: {}", &self.video_info.uri);
            let res = self.stream_impl(&weak_sendable_texture, &dart_update_stream, texture_id);
            match res {
                StreamExitResult::LegalExit => break,
                StreamExitResult::EOF => {
                    if self.video_info.auto_restart {
                        dart_update_stream.add(StreamState::Stopped).log_err();
                        info!("Stream EOF, restarting...");
                        thread::sleep(Duration::from_millis(800));
                    } else {
                        break;
                    }
                }
                StreamExitResult::Error => {
                    info!("Stream error, reinitializing...");
                    thread::sleep(Duration::from_millis(2000));
                }
            }
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
        let mark_frame_avb = || {
            if let Some(sendable) = sendable_weak.upgrade() {
                sendable.mark_frame_available();
            }
        };

        let mut is_playing = false;

        loop {
            if self.asked_for_termination() {
                self.terminate(None);
                return StreamExitResult::LegalExit;
            }

            let mut sleep_duration: Option<Duration> = None; // Declare sleep duration outside the guard scope

            {
                // Scope to hold the decoding_context lock briefly
                let mut ctx_guard = match self.decoding_context.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        error!("Failed to lock decoding context: {}", e);
                        return StreamExitResult::Error;
                    }
                };

                let ctx = match ctx_guard.as_mut() {
                    Some(ctx) => ctx,
                    None => {
                        error!("Decoding context not available");
                        return StreamExitResult::Error;
                    }
                };
                self.handle_seek_request(ctx, dart_update_stream);
                let mut packet = ffmpeg::Packet::empty();

                // Packet reading and sending to decoder logic
                match packet.read(&mut ctx.ictx) {
                    Ok(_) => {
                        if packet.stream() == ctx.video_stream_index {
                            if let Err(err) = ctx.decoder.send_packet(&packet) {
                                error!("Error sending packet to decoder: {}", err);
                            } else {
                                match self.on_new_sample(
                                    &mut ctx.decoder,
                                    &mut ctx.scaler,
                                    ctx.framerate,
                                    &mark_frame_avb,
                                ) {
                                    Ok(duration) => {
                                        // Capture the returned duration
                                        sleep_duration = duration;
                                        if !is_playing {
                                            is_playing = true;
                                            let seekable = self
                                                .seekable
                                                .load(std::sync::atomic::Ordering::Relaxed);
                                            dart_update_stream
                                                .add(StreamState::Playing {
                                                    texture_id,
                                                    seekable,
                                                })
                                                .log_err();
                                        }
                                    }
                                    Err(e) => error!("Error processing sample: {}", e),
                                }
                            }
                        }
                    }
                    Err(ffmpeg::Error::Eof) => {
                        info!("Stream EOF: {}", &self.video_info.uri);
                        self.terminate(Some(ctx));
                        return StreamExitResult::EOF;
                    }
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::error::EAGAIN,
                    }) => {
                        trace!("EAGAIN received, retrying packet read");
                        thread::sleep(Duration::from_millis(10)); // Avoid busy-looping
                        continue;
                    }
                    Err(e) => {
                        error!("Failed to read frame: {}", e);
                        dart_update_stream
                            .add(StreamState::Error("Stream corrupted".to_owned()))
                            .log_err();
                        return StreamExitResult::Error;
                    }
                }
            } // ctx_guard is dropped here, releasing the decoding_context lock

            // Perform the sleep outside the lock
            if let Some(duration) = sleep_duration {
                thread::sleep(duration);
            }
        }
    }

    fn handle_seek_request(
        &self,
        ctx: &mut DecodingContext,
        dart_update_stream: &DartUpdateStream,
    ) {
        let mut seek_lock = match self.seek_request.lock() {
            Ok(lock) => lock,
            Err(_) => {
                error!("seek_request mutex poisoned");
                return;
            }
        };

        if let Some(ts) = seek_lock.take() {
            info!("Performing seek to {} us", ts);
            if let Err(e) = ctx.ictx.seek(ts, ..ts) {
                error!("Seek failed to timestamp {}: {}", ts, e);
                dart_update_stream
                    .add(StreamState::Error(format!("Seek failed: {}", e)))
                    .log_err();
            } else {
                info!("Seek successful to timestamp: {}", ts);
                ctx.decoder.flush();
                if let Some(holder) = self.payload_holder.upgrade() {
                    holder.current_frame.lock().ok().and_then(|mut g| g.take());
                    holder.previous_frame.lock().ok().and_then(|mut g| g.take());
                }
                if let Ok(mut current_time) = self.current_time.lock() {
                    *current_time = (ts as f64) / 1_000_000.0;
                }
                if let Ok(mut synchronizer) = self.synchronizer.lock() {
                    *synchronizer = None;
                }
            }
        }
    }

    fn terminate(&self, decoding_ctx: Option<&mut DecodingContext>) {
        if let Some(ctx) = decoding_ctx {
            if let Err(e) = ctx.decoder.send_eof() {
                error!("Error sending EOF to decoder: {}", e);
            }
        } else {
            // let mut ctx = self.decoding_context.lock();
            // if let Ok(ctxa) = ctx{
            //     if let Some(dectx) = *ctxa.as_mut(){
                    
            //     }
            // }
        }
    }

    fn on_new_sample<F>(
        &self,
        decoder: &mut ffmpeg::decoder::Video,
        scaler: &mut ffmpeg::software::scaling::Context,
        framerate: u32,
        mark_frame_avb: &F,
    ) -> anyhow::Result<Option<Duration>>
    where
        F: Fn(),
    {
        let mut decoded = ffmpeg::util::frame::Video::empty();
        let mut sleep_duration = None; // Track max sleep duration found

        while decoder.receive_frame(&mut decoded).is_ok() {
            let time_base = decoder.time_base();
            let time_base_seconds = time_base.numerator() as f64 / time_base.denominator() as f64;

            let pts_seconds = decoded
                .pts()
                .map(|pts| pts as f64 * time_base_seconds)
                .unwrap_or(0.0);

            if let Ok(mut current_time) = self.current_time.lock() {
                if self
                    .seek_request
                    .lock()
                    .map(|g| g.is_none())
                    .unwrap_or(true)
                {
                    *current_time = pts_seconds;
                }
            }

            // --- Synchronizer Logic ---
            let mut sync_guard = self.synchronizer.lock().unwrap();
            if sync_guard.is_none() {
                *sync_guard = Some(FrameSynchronizer::new(pts_seconds));
            }
            if let Some(ref mut sync) = *sync_guard {
                let safe_fps = if framerate == 0 {
                    30.0
                } else {
                    framerate as f64
                };
                sleep_duration = sync.get_sleep_duration(pts_seconds, safe_fps);
            }
            drop(sync_guard);

            let mut rgb_frame = ffmpeg::util::frame::Video::empty();
            scaler.run(&decoded, &mut rgb_frame)?;

            if let Some(payload_holder) = self.payload_holder.upgrade() {
                payload_holder.set_payload(Box::new(FFmpegFrameWrapper::from_frame(rgb_frame)));
                mark_frame_avb();
            } else {
                break;
            }

            if self.asked_for_termination() {
                break;
            }
        }

        Ok(sleep_duration) // Return the duration
    }

    pub fn destroy_stream(&self) {
        self.kill_sig
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut seek) = self.seek_request.lock() {
            *seek = None;
        } else {
            error!("seek_request mutex poisoned in destroy_stream");
        }
    }

    pub fn resize_stream(&self, new_width: u32, new_height: u32) -> anyhow::Result<()> {
        let (new_width, new_height) = sanitize_dimensions(new_width, new_height)?;

        let old_dims = self
            .output_dimensions
            .lock()
            .map_err(|e| anyhow::anyhow!("output_dimensions mutex poisoned: {}", e))?
            .clone();

        if old_dims.width == new_width && old_dims.height == new_height {
            warn!("Resize requested but dimensions unchanged, skipping.");
            return Ok(());
        }

        info!(
            "Resizing stream from {}x{} to {}x{}",
            old_dims.width, old_dims.height, new_width, new_height
        );

        self.update_scaler(new_width, new_height)?;

        {
            let mut output_dims = self
                .output_dimensions
                .lock()
                .map_err(|e| anyhow::anyhow!("output_dimensions mutex poisoned on write: {}", e))?;
            *output_dims = types::VideoDimensions {
                width: new_width,
                height: new_height,
            };
        }

        self.update_texture_with_placeholder(&old_dims, new_width, new_height);

        // Immediately signal the texture that a new frame is available
        if let Some(ref weak_texture) = *self.sendable_texture.lock().unwrap() {
            if let Some(texture) = weak_texture.upgrade() {
                texture.mark_frame_available();
                info!("Marked frame available after resize");
            }
        }

        Ok(())
    }

    /// Updates the scaler in the decoding context to output new dimensions.
    fn update_scaler(&self, new_width: u32, new_height: u32) -> anyhow::Result<()> {
        let mut ctx_guard = self
            .decoding_context
            .lock()
            .map_err(|e| anyhow::anyhow!("decoding_context mutex poisoned: {}", e))?;
        let ctx = ctx_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Decoding context not initialized"))?;

        let new_scaler = Self::create_scaler_with_fallbacks(
            ctx.decoder.format(),
            ctx.decoder.width(),
            ctx.decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            new_width,
            new_height,
        )?;

        ctx.scaler = new_scaler;
        info!(
            "Scaler updated to new dimensions: {}x{}",
            new_width, new_height
        );
        Ok(())
    }

    /// Creates a placeholder frame (rescaled previous or black) to prevent visual glitches.
    fn update_texture_with_placeholder(
        &self,
        old_dims: &types::VideoDimensions,
        new_width: u32,
        new_height: u32,
    ) {
        if let Some(holder) = self.payload_holder.upgrade() {
            let prev_frame = holder.previous_frame.lock().ok().and_then(|g| g.clone());
            if let Some(prev) = prev_frame {
                if !self
                    .rescale_and_set_previous_frame(&holder, &prev, old_dims, new_width, new_height)
                {
                    self.create_black_frame(new_width, new_height, &holder);
                }
            } else {
                info!("No previous frame available, using black fallback.");
                self.create_black_frame(new_width, new_height, &holder);
            }
        }
    }

    /// Tries to rescale the last displayed frame to the new dimensions.
    fn rescale_and_set_previous_frame(
        &self,
        holder: &Arc<PayloadHolder>,
        prev_frame: &FFmpegFrameWrapper,
        old_dims: &types::VideoDimensions,
        new_width: u32,
        new_height: u32,
    ) -> bool {
        let prev_width = prev_frame.0.width();
        let prev_height = prev_frame.0.height();

        if prev_width != old_dims.width || prev_height != old_dims.height {
            warn!(
                "Previous frame dimensions ({}x{}) don't match expected old dimensions ({}x{}).",
                prev_width, prev_height, old_dims.width, old_dims.height
            );
        }

        let temp_scaler = Self::create_scaler_with_fallbacks(
            ffmpeg::format::Pixel::RGBA,
            prev_width,
            prev_height,
            ffmpeg::format::Pixel::RGBA,
            new_width,
            new_height,
        );

        if let Ok(mut temp_scaler) = temp_scaler {
            let mut resized_frame = ffmpeg::util::frame::Video::empty();
            if temp_scaler.run(&prev_frame.0, &mut resized_frame).is_ok() {
                if resized_frame.width() == new_width && resized_frame.height() == new_height {
                    info!(
                        "Successfully rescaled previous frame to {}x{}",
                        new_width, new_height
                    );
                    holder.set_payload(Box::new(FFmpegFrameWrapper::from_frame(resized_frame)));
                    return true;
                } else {
                    error!("Rescaled frame has wrong dimensions!");
                }
            } else {
                error!("Failed to rescale previous frame.");
            }
        }
        false
    }

    /// Helper function to create and set a black frame with specified dimensions.
    fn create_black_frame(&self, width: u32, height: u32, holder: &Arc<PayloadHolder>) {
        info!("Creating black frame with dimensions {}x{}", width, height);
        let mut new_frame =
            ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, width, height);

        let data = new_frame.data_mut(0);
        for chunk in data.chunks_mut(4) {
            chunk[0] = 0; // R
            chunk[1] = 0; // G
            chunk[2] = 0; // B
            chunk[3] = 255; // A
        }

        holder.set_payload(Box::new(FFmpegFrameWrapper::from_frame(new_frame)));
    }
}

/// Validates and sanitizes dimensions for resizing.
fn sanitize_dimensions(width: u32, height: u32) -> anyhow::Result<(u32, u32)> {
    if width == 0 || height == 0 {
        return Err(anyhow::anyhow!(
            "Invalid resize dimensions: {}x{}",
            width,
            height
        ));
    }

    let mut new_width = width;
    let mut new_height = height;

    if new_width % 2 != 0 {
        new_width = new_width.saturating_sub(1).max(2);
        debug!(
            "Adjusted width from {} to {} (must be even)",
            width, new_width
        );
    }
    if new_height % 2 != 0 {
        new_height = new_height.saturating_sub(1).max(2);
        debug!(
            "Adjusted height from {} to {} (must be even)",
            height, new_height
        );
    }

    const MAX_DIMENSION: u32 = 7680; // 8K
    if new_width > MAX_DIMENSION || new_height > MAX_DIMENSION {
        return Err(anyhow::anyhow!(
            "Dimensions {}x{} too large",
            new_width,
            new_height
        ));
    }
    const MIN_DIMENSION: u32 = 2;
    if new_width < MIN_DIMENSION || new_height < MIN_DIMENSION {
        return Err(anyhow::anyhow!(
            "Dimensions {}x{} too small",
            new_width,
            new_height
        ));
    }

    Ok((new_width, new_height))
}
