// inspired by
// - https://github.com/zmwangx/rust-ffmpeg/blob/master/examples/dump-frames.rs
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
        input::VideoInput,
        texture::{payload::PayloadHolder, FlutterTextureSession},
        types::{self, DartStateStream, VideoDimensions},
    },
    dart_types::StreamState,
    utils::LogErr,
};

use crate::core::texture::payload::FFmpegFrameWrapper;

struct DecodingContext {
    input_ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::decoder::Video,
    scaler: Option<ffmpeg::software::scaling::Context>,
    framerate: u32,
    stream_start_time: Option<i64>, // Unix timestamp in seconds when stream started (from #EXT-X-PROGRAM-DATE-TIME)
    custom_io: Option<CustomIoHandle>,
}

struct SdpIoContext {
    data: Arc<Vec<u8>>,
    position: usize,
}

struct CustomIoHandle {
    avio_ctx: *mut ffmpeg::ffi::AVIOContext,
    opaque: *mut SdpIoContext,
}

impl Drop for CustomIoHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.avio_ctx.is_null() {
                ffmpeg::ffi::avio_context_free(&mut self.avio_ctx);
            }
            if !self.opaque.is_null() {
                drop(Box::from_raw(self.opaque));
                self.opaque = ptr::null_mut();
            }
        }
    }
}

unsafe extern "C" fn read_packet(opaque: *mut c_void, buf: *mut u8, buf_size: i32) -> i32 {
    if opaque.is_null() || buf.is_null() || buf_size <= 0 {
        return ffmpeg::ffi::AVERROR_EOF;
    }
    let ctx = &mut *(opaque as *mut SdpIoContext);
    let remaining = ctx.data.len().saturating_sub(ctx.position);
    if remaining == 0 {
        return ffmpeg::ffi::AVERROR_EOF;
    }
    let to_copy = remaining.min(buf_size as usize);
    unsafe {
        ptr::copy_nonoverlapping(ctx.data.as_ptr().add(ctx.position), buf, to_copy);
    }
    ctx.position += to_copy;
    to_copy as i32
}

impl fmt::Debug for DecodingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodingContext")
            .field("video_stream_index", &self.video_stream_index)
            .field("framerate", &self.framerate)
            .field("stream_start_time", &self.stream_start_time)
            .field("custom_io", &self.custom_io.is_some())
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

pub struct FfmpegVideoInput {
    video_info: types::VideoInfo,
    kill_sig: Arc<AtomicBool>,
    payload_holder: Weak<PayloadHolder>,
    #[allow(unused)]
    session_id: i64,
    decoding_context: Mutex<Option<DecodingContext>>,
    ffmpeg_options: Option<HashMap<String, String>>,
    seekable: Arc<AtomicBool>, // Whether the stream is seekable
    output_dimensions: Arc<Mutex<types::VideoDimensions>>, // Track current output dimensions
    texture_session: Weak<dyn FlutterTextureSession>,
    sdp_data: Option<Arc<Vec<u8>>>,
}
unsafe impl Send for FfmpegVideoInput {}
unsafe impl Sync for FfmpegVideoInput {}
impl FfmpegVideoInput {
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
        sdp_data: Option<Arc<Vec<u8>>>,
        payload_holder: Weak<PayloadHolder>,
        texture_session: Weak<dyn FlutterTextureSession>,
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
            texture_session,
            sdp_data,
        })
    }

    fn open_input_with_sdp_data(
        sdp_data: Arc<Vec<u8>>,
        options: ffmpeg::Dictionary,
    ) -> Result<(ffmpeg::format::context::Input, Option<CustomIoHandle>), ffmpeg::Error> {
        unsafe {
            info!(
                "Initializing SDP input with custom IO ({} bytes)",
                sdp_data.len()
            );
            let buffer_size = 4096;
            let buffer = ffmpeg::ffi::av_malloc(buffer_size) as *mut u8;
            if buffer.is_null() {
                return Err(ffmpeg::Error::Unknown);
            }

            let opaque = Box::into_raw(Box::new(SdpIoContext {
                data: sdp_data,
                position: 0,
            }));
            let mut avio_ctx = ffmpeg::ffi::avio_alloc_context(
                buffer,
                buffer_size as i32,
                0,
                opaque as *mut c_void,
                Some(read_packet),
                None,
                None,
            );
            if avio_ctx.is_null() {
                ffmpeg::ffi::av_free(buffer as *mut c_void);
                drop(Box::from_raw(opaque));
                return Err(ffmpeg::Error::Unknown);
            }

            let mut fmt_ctx = ffmpeg::ffi::avformat_alloc_context();
            if fmt_ctx.is_null() {
                ffmpeg::ffi::avio_context_free(&mut avio_ctx);
                drop(Box::from_raw(opaque));
                return Err(ffmpeg::Error::Unknown);
            }

            (*fmt_ctx).pb = avio_ctx;
            (*fmt_ctx).flags |= ffmpeg::ffi::AVFMT_FLAG_CUSTOM_IO;

            let fmt_name = CString::new("sdp").unwrap();
            let iformat = ffmpeg::ffi::av_find_input_format(fmt_name.as_ptr());
            if iformat.is_null() {
                error!("Failed to find SDP demuxer");
                ffmpeg::ffi::avformat_free_context(fmt_ctx);
                ffmpeg::ffi::avio_context_free(&mut avio_ctx);
                drop(Box::from_raw(opaque));
                return Err(ffmpeg::Error::InvalidData);
            }

            let mut opts = options.disown();
            let res =
                ffmpeg::ffi::avformat_open_input(&mut fmt_ctx, ptr::null(), iformat, &mut opts);
            ffmpeg::Dictionary::own(opts);
            if res < 0 {
                error!(
                    "avformat_open_input failed for SDP: {}",
                    ffmpeg::Error::from(res)
                );
                ffmpeg::ffi::avformat_free_context(fmt_ctx);
                ffmpeg::ffi::avio_context_free(&mut avio_ctx);
                drop(Box::from_raw(opaque));
                return Err(ffmpeg::Error::from(res));
            }

            let res_info = ffmpeg::ffi::avformat_find_stream_info(fmt_ctx, ptr::null_mut());
            if res_info < 0 {
                error!(
                    "avformat_find_stream_info failed for SDP: {}",
                    ffmpeg::Error::from(res_info)
                );
                ffmpeg::ffi::avformat_close_input(&mut fmt_ctx);
                ffmpeg::ffi::avio_context_free(&mut avio_ctx);
                drop(Box::from_raw(opaque));
                return Err(ffmpeg::Error::from(res_info));
            }

            let handle = CustomIoHandle { avio_ctx, opaque };
            Ok((ffmpeg::format::context::Input::wrap(fmt_ctx), Some(handle)))
        }
    }

    pub fn initialize_stream(&self) -> Result<(), ffmpeg::Error> {
        trace!("Starting ffmpeg session for {}", &self.video_info.uri);
        let mut option_dict = ffmpeg::Dictionary::new();
        if let Some(ref options) = self.ffmpeg_options {
            for (key, value) in options {
                option_dict.set(key, value);
            }
        }
        if self.sdp_data.is_some() {
            info!("Using SDP custom IO for {}", &self.video_info.uri);
        }
        let (ictx, custom_io) = if let Some(sdp_data) = self.sdp_data.as_ref() {
            Self::open_input_with_sdp_data(Arc::clone(sdp_data), option_dict)?
        } else {
            (
                ffmpeg::format::input_with_dictionary(&self.video_info.uri, option_dict)?,
                None,
            )
        };
        let starttimerealtime = unsafe { (*ictx.as_ptr()).start_time_realtime } / 100_0000;
        let input = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        info!("start_time_realtime {starttimerealtime}");
        let video_stream_index = input.index();
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;

        let decoder = context_decoder.decoder().video()?;
        let decoder_width = decoder.width();
        let decoder_height = decoder.height();
        if decoder_width == 0 || decoder_height == 0 {
            if self.sdp_data.is_none() {
                error!(
                    "Invalid frame dimensions during init: {}x{}",
                    decoder_width, decoder_height
                );
                return Err(ffmpeg::Error::InvalidData);
            }
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
            stream_start_time: Some(starttimerealtime),
            custom_io,
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

    fn mark_frame_available(&self) {
        if let Some(session) = self.texture_session.upgrade() {
            session.mark_frame_available();
        }
    }

    fn log_stream_exit(&self, reason: &str) {
        debug!(
            "Stream exit ({}) for uri={} session_id={}",
            reason, self.video_info.uri, self.session_id
        );
    }

    pub fn execute(
        &self,
        dart_update_stream: DartStateStream,
        texture_id: i64,
    ) -> anyhow::Result<()> {
        while !self.asked_for_termination() {
            trace!("FFmpeg session initializing for {}", &self.video_info.uri);
            dart_update_stream.add(StreamState::Loading).log_err();

            if let Err(e) = self.initialize_stream() {
                let error_msg =
                    format!("Stream initialization failed: {}, reinitializing in 1s", e);
                error!("{} for uri: {}", error_msg, &self.video_info.uri);
                dart_update_stream
                    .add(StreamState::Error(error_msg))
                    .log_err();
                thread::sleep(Duration::from_millis(1000));
                continue;
            }

            info!("Stream initialized: {}", &self.video_info.uri);

            let res = self.stream_impl(&dart_update_stream, texture_id);
            match res {
                StreamExitResult::LegalExit => break,
                StreamExitResult::EOF => {
                    if self.video_info.auto_restart {
                        dart_update_stream.add(StreamState::Stopped).log_err();
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
        dart_update_stream.add(StreamState::Stopped).log_err();
        Ok(())
    }

    fn stream_impl(
        &self,
        dart_update_stream: &DartStateStream,
        texture_id: i64,
    ) -> StreamExitResult {
        let mark_frame_avb = || {
            self.mark_frame_available();
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
                let framerate = ctx.framerate;
                // for rtsp we don't need to sleep
                if framerate > 0 && !self.video_info.uri.starts_with("rtsp") {
                    // 1. Calculate the total number of nanoseconds in one second: 1,000,000,000
                    // 2. Divide this by the framerate to get the duration per frame in nanoseconds.
                    let nanos_per_frame = 1_000_000_000 / framerate;
                    sleep_duration = Some(Duration::from_nanos(nanos_per_frame as u64));
                }

                let mut packet = ffmpeg::Packet::empty();

                // Packet reading and sending to decoder logic
                match packet.read(&mut ctx.input_ctx) {
                    Ok(_) => {
                        if packet.stream() == ctx.video_stream_index {
                            if let Err(err) = ctx.decoder.send_packet(&packet) {
                                error!("Error sending packet to decoder: {}", err);
                            } else {
                                let _ = self
                                    .on_new_sample_rtsp(ctx, &mark_frame_avb)
                                    .inspect_err(|e| error!("on new sample err: {e}"));
                                if !is_playing {
                                    dart_update_stream
                                        .add(StreamState::Playing {
                                            texture_id,
                                            seekable: true,
                                        })
                                        .log_err();
                                    is_playing = true;
                                }
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

    fn on_new_sample_rtsp<F>(
        &self,
        ctx: &mut DecodingContext,
        mark_frame_avb: &F,
    ) -> anyhow::Result<()>
    where
        F: Fn(),
    {
        let mut decoded = ffmpeg::util::frame::Video::empty();
        while ctx.decoder.receive_frame(&mut decoded).is_ok() {
            if ctx.scaler.is_none() {
                let target_dims = self
                    .output_dimensions
                    .lock()
                    .map(|d| d.clone())
                    .unwrap_or_else(|_| self.video_info.dimensions.clone());
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
                scaler.run(&decoded, &mut rgb_frame)?;
            } else {
                return Err(anyhow::anyhow!("Scaler not initialized"));
            }

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

        // Immediately signal the texture that a new frame is available
        self.mark_frame_available();
        info!("Marked frame available after resize");

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
            info!(
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
            let prev_frame = holder.previous_frame();
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

impl VideoInput for FfmpegVideoInput {
    fn execute(&self, update_stream: DartStateStream, texture_id: i64) -> anyhow::Result<()> {
        FfmpegVideoInput::execute(self, update_stream, texture_id)
    }

    fn resize(&self, width: u32, height: u32) -> anyhow::Result<()> {
        self.resize_stream(width, height)
    }

    fn terminate(&self) {
        self.destroy_stream();
    }

    fn seek(&self, ts: i64) -> anyhow::Result<()> {
        FfmpegVideoInput::seek(self, ts)
    }

    fn output_dimensions(&self) -> VideoDimensions {
        self.output_dimensions
            .lock()
            .map(|dims| dims.clone())
            .unwrap_or_else(|_| self.video_info.dimensions.clone())
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
