use ffmpeg::format::Pixel;
use ffmpeg::Rescale;
use std::{
    collections::HashMap,
    ffi::{c_void, CString},
    fmt, ptr,
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
    thread,
    time::Duration,
};

use log::{debug, error, info, trace, warn};

use crate::{
    core::{
        input::{InputCommand, InputCommandReceiver, InputEvent, InputEventSender},
        texture::payload::PayloadHolder,
        types::{self, VideoDimensions},
    },
    dart_types::StreamState,
};

use crate::core::texture::payload::RawRgbaFrame;

struct DecodingContext {
    input_ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::decoder::Video,
    scaler: Option<ffmpeg::software::scaling::Context>,
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

pub struct FfmpegVideoSession {
    video_info: types::VideoInfo,
    kill_sig: Arc<AtomicBool>,
    payload_holder: Weak<PayloadHolder>,
    #[allow(unused)]
    session_id: i64,
    decoding_context: Mutex<Option<DecodingContext>>,
    ffmpeg_options: Option<HashMap<String, String>>,
    seekable: Arc<AtomicBool>, // Whether the stream is seekable
    output_dimensions: Arc<Mutex<types::VideoDimensions>>, // Track current output dimensions
}

impl FfmpegVideoSession {
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
        payload_holder: Weak<PayloadHolder>,
    ) -> Arc<Self> {
        Arc::new(Self {
            video_info: video_info.clone(),
            kill_sig: Arc::new(AtomicBool::new(false)),
            payload_holder,
            session_id,
            decoding_context: Mutex::new(None),
            ffmpeg_options,
            seekable: Arc::new(AtomicBool::new(false)),
            output_dimensions: Arc::new(Mutex::new(video_info.dimensions.clone())),
        })
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

        let decoder = context_decoder.decoder().video()?;
        let decoder_width = decoder.width();
        let decoder_height = decoder.height();
        if decoder_width == 0 || decoder_height == 0 {
            warn!(
                "Decoder dimensions unknown during init ({}x{}); will init scaler on first frame",
                decoder_width, decoder_height
            );
        } else {
            Self::validate_stream_compatibility(&decoder)?;
        }

        let target_dims = self
            .output_dimensions
            .lock()
            .map(|d| d.clone())
            .unwrap_or_else(|_| self.video_info.dimensions.clone());

        let scaler = if decoder_width == 0 || decoder_height == 0 {
            None
        } else {
            Some(Self::create_scaler_with_fallbacks(
                decoder.format(),
                decoder_width,
                decoder_height,
                ffmpeg::format::Pixel::RGBA,
                target_dims.width,
                target_dims.height,
            )?)
        };

        let avg_frame_rate = input.avg_frame_rate();
        let frame_rate = if avg_frame_rate.denominator() != 0 {
            (avg_frame_rate.numerator() as f64 / avg_frame_rate.denominator() as f64) as u32
        } else {
            0
        };

        let context = DecodingContext {
            input_ctx: ictx,
            video_stream_index,
            decoder,
            scaler,
            framerate: frame_rate,
        };
        debug!(
            "Created context {:?} for url: {}",
            context, &self.video_info.uri
        );

        let is_hls_stream =
            self.video_info.uri.contains(".m3u8") || self.video_info.uri.contains("hls");
        let is_rtsp = self.video_info.uri.starts_with("rtsp");
        let is_seekable = is_hls_stream || is_rtsp;
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

    fn log_stream_exit(&self, reason: &str) {
        debug!(
            "Stream exit ({}) for uri={} session_id={}",
            reason, self.video_info.uri, self.session_id
        );
    }

    fn send_event(event_tx: &InputEventSender, event: InputEvent) {
        if let Err(err) = event_tx.send(event) {
            error!("Failed to send input event: {}", err);
        }
    }

    fn drain_commands(
        &self,
        command_rx: &InputCommandReceiver,
        event_tx: &InputEventSender,
    ) -> bool {
        let mut should_terminate = false;
        loop {
            match command_rx.try_recv() {
                Ok(command) => match command {
                    InputCommand::Resize { width, height } => {
                        if let Err(err) = self.resize_stream(width, height) {
                            error!("Resize failed: {}", err);
                        } else {
                            Self::send_event(event_tx, InputEvent::FrameAvailable);
                        }
                    }
                    InputCommand::Terminate => {
                        self.destroy_stream();
                        should_terminate = true;
                    }
                    InputCommand::Seek { ts } => {
                        if let Err(err) = self.seek(ts) {
                            error!("Seek failed: {}", err);
                        }
                    }
                },
                Err(flume::TryRecvError::Empty) => break,
                Err(flume::TryRecvError::Disconnected) => break,
            }
        }
        should_terminate
    }

    pub fn execute(
        &self,
        event_tx: InputEventSender,
        command_rx: InputCommandReceiver,
        texture_id: i64,
    ) -> anyhow::Result<()> {
        while !self.asked_for_termination() {
            if self.drain_commands(&command_rx, &event_tx) {
                break;
            }
            trace!("FFmpeg session initializing for {}", &self.video_info.uri);
            Self::send_event(&event_tx, InputEvent::State(StreamState::Loading));

            if let Err(e) = self.initialize_stream() {
                let error_msg =
                    format!("Stream initialization failed: {}, reinitializing in 1s", e);
                error!("{} for uri: {}", error_msg, &self.video_info.uri);
                Self::send_event(&event_tx, InputEvent::State(StreamState::Error(error_msg)));
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            info!("Stream initialized: {}", &self.video_info.uri);

            let res = self.stream_impl(&event_tx, &command_rx, texture_id);
            match res {
                StreamExitResult::LegalExit => break,
                StreamExitResult::EOF => {
                    if self.video_info.auto_restart {
                        Self::send_event(&event_tx, InputEvent::State(StreamState::Stopped));
                        info!("Stream EOF, restarting...");
                        thread::sleep(Duration::from_millis(100));
                    } else {
                        break;
                    }
                }
                StreamExitResult::Error => {
                    info!("Stream error, reinitializing...");
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
        if self.asked_for_termination() {
            self.log_stream_exit("terminated");
        } else {
            self.log_stream_exit("loop-exit");
        }
        Self::send_event(&event_tx, InputEvent::State(StreamState::Stopped));
        Ok(())
    }

    fn stream_impl(
        &self,
        event_tx: &InputEventSender,
        command_rx: &InputCommandReceiver,
        texture_id: i64,
    ) -> StreamExitResult {
        let mark_frame_avb = || {
            Self::send_event(event_tx, InputEvent::FrameAvailable);
        };

        let mut is_playing = false;
        let mut stalled = false;

        loop {
            if self.asked_for_termination() {
                self.terminate(None);
                return StreamExitResult::LegalExit;
            }
            if self.drain_commands(command_rx, event_tx) {
                self.terminate(None);
                return StreamExitResult::LegalExit;
            }

            let mut sleep_duration: Option<Duration> = None; // Declare sleep duration outside the guard scope
            let mut retry_sleep: Option<Duration> = None;
            let mut retry_loop = false;

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
                let framerate = ctx.framerate;
                // for rtsp we don't need to sleep
                if framerate > 0 && !self.video_info.uri.starts_with("rtsp") {
                    // 1. Calculate the total number of nanoseconds in one second: 1,000,000,000
                    // 2. Divide this by the framerate to get the duration per frame in nanoseconds.
                    let nanos_per_frame = 1_000_000_000 / framerate;
                    sleep_duration = Some(Duration::from_nanos(nanos_per_frame as u64));
                }

                match ctx.input_ctx.next_packet() {
                    Ok(packet) => {
                        if packet.stream() == ctx.video_stream_index {
                            if let Err(err) = ctx.decoder.send_packet(&packet) {
                                error!("Error sending packet to decoder: {}", err);
                            }
                            // Always try to receive frames, even after send errors.
                            // When the backing stream changes (e.g., camera disconnect -> fallback),
                            // on_new_sample handles flushing the decoder internally.
                            let _ = self
                                .on_new_sample(ctx, &mark_frame_avb)
                                .inspect_err(|e| error!("on new sample err: {e}"));
                            if !is_playing {
                                if stalled {
                                    info!("Stream data resumed: {}", &self.video_info.uri);
                                    stalled = false;
                                }
                                let seekable =
                                    self.seekable.load(std::sync::atomic::Ordering::Relaxed);
                                Self::send_event(
                                    event_tx,
                                    InputEvent::State(StreamState::Playing {
                                        texture_id,
                                        seekable,
                                    }),
                                );
                                is_playing = true;
                            }
                        }
                    }
                    Err(ffmpeg::Error::Eof) => {
                        info!("Stream EOF: {}", &self.video_info.uri);
                        self.terminate(Some(ctx));
                        self.log_stream_exit("ffmpeg-eof");
                        return StreamExitResult::EOF;
                    }
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::error::EAGAIN,
                    }) => {
                        trace!("EAGAIN received, retrying packet read");
                        continue;
                    }
                    Err(ffmpeg::Error::Other {
                        errno: ffmpeg::error::ETIMEDOUT,
                    }) => {
                        if !stalled {
                            warn!(
                                "Stream read timed out, waiting for frames: {}",
                                &self.video_info.uri
                            );
                            Self::send_event(event_tx, InputEvent::State(StreamState::Loading));
                            stalled = true;
                            is_playing = false;
                        }
                        retry_sleep = Some(Duration::from_millis(200));
                        retry_loop = true;
                    }
                    Err(e) => {
                        error!("Failed to read frame: {}", e);
                        Self::send_event(
                            event_tx,
                            InputEvent::State(StreamState::Error("Stream corrupted".to_owned())),
                        );
                        return StreamExitResult::Error;
                    }
                }
            } // ctx_guard is dropped here, releasing the decoding_context lock

            // Perform the sleep outside the lock
            if let Some(duration) = retry_sleep {
                thread::sleep(duration);
            }
            if retry_loop {
                continue;
            }
            if let Some(duration) = sleep_duration {
                thread::sleep(duration);
            }
        }
    }

    pub fn seek(&self, timestamp_us: i64) -> anyhow::Result<()> {
        let mut decoder_ctx = self
            .decoding_context
            .lock()
            .map_err(|e| anyhow::anyhow!("decoding_context mutex poisoned: {e}"))?;
        info!(
            "Performing seek to {} us (relative stream time)",
            timestamp_us
        );
        if let Some(decoder_ctx) = decoder_ctx.as_mut() {
            let position = timestamp_us.rescale((1, 1000000), ffmpeg::rescale::TIME_BASE);
            if let Err(e) = decoder_ctx.input_ctx.seek(position, ..position) {
                log::error!("Failed to seek {:?}", e);
                return Err(anyhow::anyhow!("Failed to seek {:?}", e));
            }
        }
        Ok(())
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

    fn on_new_sample<F>(&self, ctx: &mut DecodingContext, mark_frame_avb: &F) -> anyhow::Result<()>
    where
        F: Fn(),
    {
        let mut decoded = ffmpeg::util::frame::Video::empty();
        log::debug!("got new sample");
        loop {
            match ctx.decoder.receive_frame(&mut decoded) {
                Ok(()) => {}
                Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::error::EAGAIN => {
                    // No more frames available right now, this is normal
                    break;
                }
                Err(ffmpeg::Error::InputChanged) => {
                    // Stream parameters changed (e.g., fallback stream activated)
                    // Flush decoder and reset scaler to adapt to new stream
                    warn!("Input changed detected, flushing decoder");
                    ctx.decoder.flush();
                    ctx.scaler = None;
                    break;
                }
                Err(e) => {
                    // Other errors - log but don't fail the whole function
                    trace!("receive_frame error: {}", e);
                    break;
                }
            }

            let target_dims = self
                .output_dimensions
                .lock()
                .map(|d| d.clone())
                .unwrap_or_else(|_| self.video_info.dimensions.clone());

            // Create or recreate scaler if needed (e.g., after stream switch to fallback)
            if ctx.scaler.is_none() {
                let new_scaler = Self::create_scaler_with_fallbacks(
                    decoded.format(),
                    decoded.width(),
                    decoded.height(),
                    ffmpeg::format::Pixel::RGBA,
                    target_dims.width,
                    target_dims.height,
                )?;
                ctx.scaler = Some(new_scaler);
            }

            let mut rgb_frame = ffmpeg::util::frame::Video::empty();
            if let Some(ref mut scaler) = ctx.scaler {
                if let Err(e) = scaler.run(&decoded, &mut rgb_frame) {
                    // Scaler failed - likely input dimensions changed. Reset and retry next frame.
                    warn!("Scaler error (resetting): {}", e);
                    ctx.scaler = None;
                    continue;
                }
            } else {
                return Err(anyhow::anyhow!("Scaler not initialized"));
            }

            if let Some(payload_holder) = self.payload_holder.upgrade() {
                payload_holder.set_payload(Arc::new(RawRgbaFrame::from_ffmpeg(&rgb_frame)));
                mark_frame_avb();
            } else {
                break;
            }

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

        Ok(())
    }

    /// Updates the scaler in the decoding context to output new dimensions.
    fn update_scaler(&self, new_width: u32, new_height: u32) -> anyhow::Result<()> {
        let mut ctx_guard = self
            .decoding_context
            .lock()
            .map_err(|e| anyhow::anyhow!("decoding_context mutex poisoned: {}", e))?;
        let ctx = match ctx_guard.as_mut() {
            Some(ctx) => ctx,
            None => {
                info!("Decoding context not initialized; deferring scaler update");
                return Ok(());
            }
        };

        let src_width = ctx.decoder.width();
        let src_height = ctx.decoder.height();
        if src_width == 0 || src_height == 0 {
            ctx.scaler = None;
            info!(
                "Scaler deferred until first frame (target {}x{})",
                new_width, new_height
            );
        } else {
            let new_scaler = Self::create_scaler_with_fallbacks(
                ctx.decoder.format(),
                src_width,
                src_height,
                ffmpeg::format::Pixel::RGBA,
                new_width,
                new_height,
            )?;
            ctx.scaler = Some(new_scaler);
            debug!(
                "Scaler updated to new dimensions: {}x{}",
                new_width, new_height
            );
        }
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
            let rescaled = if let Some(prev) = holder.previous_frame() {
                self.rescale_and_set_previous_frame(&holder, &prev, old_dims, new_width, new_height)
            } else {
                false
            };
            if !rescaled {
                info!("No previous frame available or not rescalable, using black fallback.");
                holder.set_payload(Arc::new(RawRgbaFrame::black(new_width, new_height)));
            }
        }
    }

    /// Tries to rescale the last displayed frame to the new dimensions using FFmpeg's scaler.
    fn rescale_and_set_previous_frame(
        &self,
        holder: &Arc<PayloadHolder>,
        prev_frame: &RawRgbaFrame,
        old_dims: &types::VideoDimensions,
        new_width: u32,
        new_height: u32,
    ) -> bool {
        let prev_width = prev_frame.width;
        let prev_height = prev_frame.height;

        if prev_width != old_dims.width || prev_height != old_dims.height {
            warn!(
                "Previous frame dimensions ({}x{}) don't match expected old dimensions ({}x{}).",
                prev_width, prev_height, old_dims.width, old_dims.height
            );
        }

        let temp_scaler = Self::create_scaler_with_fallbacks(
            Pixel::RGBA,
            prev_width,
            prev_height,
            Pixel::RGBA,
            new_width,
            new_height,
        );

        if let Ok(mut temp_scaler) = temp_scaler {
            // Wrap the raw bytes into an FFmpeg frame for the scaler
            let mut src_frame =
                ffmpeg::util::frame::Video::new(Pixel::RGBA, prev_width, prev_height);
            let stride = src_frame.stride(0);
            let row_len = (prev_width * 4) as usize;
            for y in 0..prev_height as usize {
                let src_start = y * row_len;
                let dst_start = y * stride;
                src_frame.data_mut(0)[dst_start..dst_start + row_len]
                    .copy_from_slice(&prev_frame.data[src_start..src_start + row_len]);
            }

            let mut resized_frame = ffmpeg::util::frame::Video::empty();
            if temp_scaler.run(&src_frame, &mut resized_frame).is_ok() {
                if resized_frame.width() == new_width && resized_frame.height() == new_height {
                    info!(
                        "Successfully rescaled previous frame to {}x{}",
                        new_width, new_height
                    );
                    holder.set_payload(Arc::new(RawRgbaFrame::from_ffmpeg(&resized_frame)));
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
