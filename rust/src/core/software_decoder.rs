//! s/w demuxer optimized for live streams with HLS seeking support inspired by Flyleaf's C# demuxer.

use ffmpeg::Rescale;
use std::{
    collections::HashMap,
    fmt,
    sync::{atomic::AtomicBool, Arc, Mutex, Weak},
    thread,
    time::{Duration, Instant},
};

use ffmpeg::ffi;

use irondash_texture::{BoxedPixelData, PayloadProvider, SendableTexture};
use log::{debug, error, info, trace, warn};

use crate::{core::types::DartStateStream, dart_types::StreamState, utils::LogErr};

use super::{ffmpeg_private, types};

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

/// Trait for handling timeline adjustments for different stream types.
///
/// This trait abstracts the timeline management for various streaming protocols,
/// allowing different implementations for HLS (segment-based) and standard streams.
/// The key challenge with HLS is that it uses a segment-based timeline where seeking
/// requires adjusting timestamps to account for the playlist structure.
trait TimelineContext {
    /// Adjust seek timestamp for the specific stream type.
    ///
    /// For HLS streams, this adjusts the requested timestamp to account for:
    /// - The virtual start time calculated from segment information
    /// - The format context's first_timestamp
    ///
    /// For standard streams, this typically returns the timestamp as-is.
    ///
    /// # Arguments
    /// * `timestamp_us` - The requested timestamp in microseconds (relative to stream start)
    ///
    /// # Returns
    /// The adjusted timestamp in microseconds suitable for seeking in the underlying format
    fn adjust_seek_timestamp(&self, timestamp_us: i64) -> i64;

    /// Update timeline information (called periodically during playback).
    ///
    /// For HLS streams, this:
    /// - Monitors playlist changes via segment sequence numbers
    /// - Recalculates duration when segments change
    /// - Updates the virtual start time based on current position
    ///
    /// For standard streams, this is typically a no-op.
    fn update_timeline(&mut self);

    /// Get the current time offset for the stream in microseconds.
    ///
    /// For HLS, this returns the calculated start time of the current viewing window.
    /// For standard streams, this returns the stream's native start_time.
    fn get_time_offset(&self) -> i64;

    /// Check if seeking is supported for this stream.
    ///
    /// For HLS, returns true only after the timeline has been initialized.
    /// For standard streams, typically always returns true.
    #[allow(dead_code)]
    fn is_seekable(&self) -> bool;

    /// Update timeline with packet timestamp information.
    ///
    /// For HLS streams, this is used to calculate hls_start_time from packet timestamps.
    /// For standard streams, this is typically a no-op.
    ///
    /// # Arguments
    /// * `packet_timestamp_us` - Packet timestamp in microseconds
    fn update_from_packet(&mut self, packet_timestamp_us: i64);
}

/// HLS-specific timeline context for handling segment-based seeking.
///
/// HLS (HTTP Live Streaming) presents unique challenges for seeking:
/// 1. Live streams are normally unseekable in FFmpeg
/// 2. The stream is composed of discrete segments with individual timestamps
/// 3. We need to maintain a virtual timeline across segment boundaries
///
/// This implementation follows the approach from Flyleaf's C# demuxer:
/// - Tracks the HLS playlist structure and segment durations
/// - Calculates a virtual start time based on segment information
/// - Adjusts all seek operations to account for the segment timeline
/// - Forces FFmpeg to treat HLS as seekable by manipulating context flags
///
/// The key insight is that we can seek within the "DVR window" (available segments)
/// by calculating the correct timestamp offset for each segment.
struct HLSTimelineContext {
    /// Virtual start time in microseconds, calculated as:
    /// (current packet timestamp - sum of previous segment durations)
    hls_start_time: i64,

    /// Duration in microseconds from playlist start to current segment start.
    /// This is the sum of all segment durations before the current one.
    hls_cur_duration: i64,

    /// Previous sequence number to detect playlist updates (segment wraparound)
    hls_prev_seq_no: i64,

    /// Direct pointer to the HLS context (from format_ctx->priv_data)
    hls_ctx: *const ffmpeg_private::HLSContext,

    /// Direct pointer to the active playlist
    playlist: *const ffmpeg_private::playlist,

    /// Last packet timestamp we've seen (used to calculate hls_start_time)
    last_packet_timestamp: i64,

    /// Whether the timeline has been initialized with segment information
    is_initialized: bool,
}

unsafe impl Send for HLSTimelineContext {}
unsafe impl Sync for HLSTimelineContext {}

impl HLSTimelineContext {
    /// Create a new HLS timeline context from a format context pointer.
    ///
    /// # Safety
    /// This function accesses FFmpeg internal structures and must verify the format is HLS.
    /// The format context must remain valid for the lifetime of this HLSTimelineContext.
    unsafe fn new(fmt_ctx_ptr: *mut ffi::AVFormatContext) -> Option<Self> {
        if fmt_ctx_ptr.is_null() {
            return None;
        }

        let fmt_ctx = &*fmt_ctx_ptr;

        // Check if this is an HLS format
        if fmt_ctx.iformat.is_null() {
            return None;
        }

        let iformat = &*fmt_ctx.iformat;
        let name = std::ffi::CStr::from_ptr(iformat.name);

        if name.to_string_lossy() != "hls" {
            warn!("Attempted to create HLS context for non-HLS format");
            return None;
        }

        // Cast priv_data to HLSContext
        if fmt_ctx.priv_data.is_null() {
            warn!("HLS format context has null priv_data");
            return None;
        }

        let hls_ctx = fmt_ctx.priv_data as *const ffmpeg_private::HLSContext;
        let hls_ctx_ref = &*hls_ctx;

        // Get the first playlist (most HLS streams have a single playlist)
        // For multi-variant streams, this gets the first variant's playlist
        let playlist = hls_ctx_ref.get_first_playlist().map(|p| p as *const _);

        if playlist.is_none() {
            warn!("HLS context has no available playlists");
            return None;
        }

        info!("HLS timeline context created successfully");

        Some(Self {
            hls_start_time: ffi::AV_NOPTS_VALUE,
            hls_cur_duration: 0,
            hls_prev_seq_no: ffi::AV_NOPTS_VALUE,
            hls_ctx,
            playlist: playlist.unwrap(),
            last_packet_timestamp: ffi::AV_NOPTS_VALUE,
            is_initialized: false,
        })
    }

    /// Calculate duration by accessing HLS playlist segments.
    ///
    /// This implementation:
    /// 1. Accesses the stored playlist pointer
    /// 2. Reads current sequence number
    /// 3. Iterates through segments and sums their durations
    /// 4. Detects sequence number changes (playlist updates)
    /// 5. Updates hls_cur_duration and detects wraparound
    ///
    /// # Safety
    /// Accesses FFmpeg internal structures via raw pointers.
    unsafe fn update_hls_timeline(&mut self) {
        if self.playlist.is_null() {
            warn!("HLS playlist pointer is null, cannot update timeline");
            return;
        }

        let playlist = &*self.playlist;

        // Extract values
        let cur_seq_no = playlist.cur_seq_no;
        let start_seq_no = playlist.start_seq_no;
        let duration_until_seq = playlist.calculate_duration_until_seq(cur_seq_no);
        let prev_seq_no = self.hls_prev_seq_no;

        trace!(
            "Updating HLS timeline: cur_seq={}, prev_seq={}, start_seq={}, duration={}",
            cur_seq_no,
            prev_seq_no,
            start_seq_no,
            duration_until_seq
        );

        // Check if sequence number changed (playlist updated or wraparound)
        if prev_seq_no != ffi::AV_NOPTS_VALUE && cur_seq_no != prev_seq_no {
            debug!(
                "HLS playlist sequence changed: {} -> {}",
                prev_seq_no, cur_seq_no
            );

            self.hls_prev_seq_no = cur_seq_no;
            self.hls_start_time = ffi::AV_NOPTS_VALUE;

            // Convert from AV_TIME_BASE units to microseconds
            self.hls_cur_duration = (duration_until_seq * 1000000) / ffi::AV_TIME_BASE as i64;

            trace!(
                "HLS current duration: {} us ({} segments)",
                self.hls_cur_duration,
                cur_seq_no - start_seq_no
            );
        } else if prev_seq_no == ffi::AV_NOPTS_VALUE {
            // First time initialization
            self.hls_prev_seq_no = cur_seq_no;
            self.hls_cur_duration = (duration_until_seq * 1000000) / ffi::AV_TIME_BASE as i64;

            info!(
                "HLS timeline initialized: start_seq={}, cur_seq={}, duration={} us",
                start_seq_no, cur_seq_no, self.hls_cur_duration
            );
        }

        self.is_initialized = true;
    }

    /// Update hls_start_time based on packet timestamp
    ///
    /// This calculates the virtual start time as:
    /// hls_start_time = last_packet_timestamp - hls_cur_duration
    ///
    /// This should be called when we receive packets to establish the timeline reference point.
    unsafe fn update_start_time_from_packet(&mut self, packet_timestamp_us: i64) {
        if packet_timestamp_us == ffi::AV_NOPTS_VALUE {
            return;
        }

        self.last_packet_timestamp = packet_timestamp_us;

        // Only calculate hls_start_time if we don't have it yet and we have duration info
        if self.hls_start_time == ffi::AV_NOPTS_VALUE && self.hls_cur_duration > 0 {
            self.hls_start_time = packet_timestamp_us - self.hls_cur_duration;
            info!(
                "HLS start time calculated: {} us (packet: {} us, duration: {} us)",
                self.hls_start_time, packet_timestamp_us, self.hls_cur_duration
            );
        }
    }
}

impl TimelineContext for HLSTimelineContext {
    fn adjust_seek_timestamp(&self, timestamp_us: i64) -> i64 {
        // If timeline not initialized, we can't adjust yet
        if self.hls_start_time == ffi::AV_NOPTS_VALUE {
            warn!(
                "HLS timeline not initialized (hls_start_time = AV_NOPTS_VALUE), cannot adjust timestamp. Returning as-is: {} us",
                timestamp_us
            );
            return timestamp_us;
        }

        // Adjust timestamp to account for HLS segment timeline
        // Formula: adjusted = requested + hls_start_time - first_timestamp
        // This converts from the virtual timeline to actual segment timestamps
        unsafe {
            if !self.hls_ctx.is_null() {
                let hls_ctx = &*self.hls_ctx;
                let first_timestamp = if hls_ctx.first_timestamp != ffi::AV_NOPTS_VALUE {
                    (hls_ctx.first_timestamp * 1000000) / ffi::AV_TIME_BASE as i64
                } else {
                    0
                };

                let adjusted = timestamp_us + self.hls_start_time - first_timestamp;
                trace!(
                    "HLS timestamp adjustment: requested={} us, start={} us, first={} us, adjusted={} us",
                    timestamp_us, self.hls_start_time, first_timestamp, adjusted
                );
                return adjusted;
            }
        }

        timestamp_us
    }

    fn update_timeline(&mut self) {
        unsafe {
            self.update_hls_timeline();

            // Try to calculate hls_start_time from HLS context's first_timestamp
            if self.hls_start_time == ffi::AV_NOPTS_VALUE
                && !self.hls_ctx.is_null()
                && self.hls_cur_duration > 0
            {
                let hls_ctx = &*self.hls_ctx;
                if hls_ctx.first_timestamp != ffi::AV_NOPTS_VALUE {
                    let first_timestamp_us =
                        (hls_ctx.first_timestamp * 1000000) / ffi::AV_TIME_BASE as i64;
                    self.update_start_time_from_packet(first_timestamp_us);
                }
            }
        }
    }

    fn update_from_packet(&mut self, packet_timestamp_us: i64) {
        unsafe {
            self.update_start_time_from_packet(packet_timestamp_us);
        }
    }

    fn get_time_offset(&self) -> i64 {
        if self.hls_start_time != ffi::AV_NOPTS_VALUE {
            self.hls_start_time
        } else {
            0
        }
    }

    fn is_seekable(&self) -> bool {
        // For HLS, we need both initialization and a valid start time
        self.is_initialized && self.hls_start_time != ffi::AV_NOPTS_VALUE
    }
}

/// Standard timeline context for non-HLS streams.
///
/// This is the simpler case where the stream has a continuous timeline
/// without segment boundaries. Seeking is straightforward and requires
/// no timestamp adjustment beyond the stream's native start_time.
struct StandardTimelineContext {
    /// Stream start time in microseconds from the format context
    start_time: i64,
}

impl StandardTimelineContext {
    fn new(start_time: i64) -> Self {
        Self { start_time }
    }
}

impl TimelineContext for StandardTimelineContext {
    fn adjust_seek_timestamp(&self, timestamp_us: i64) -> i64 {
        // For standard streams, add the start_time offset
        let adjusted = timestamp_us + self.start_time;
        trace!(
            "Standard timeline adjustment: requested={} us, start_time={} us, adjusted={} us",
            timestamp_us,
            self.start_time,
            adjusted
        );
        adjusted
    }

    fn update_timeline(&mut self) {
        // No special updates needed for standard streams
    }

    fn get_time_offset(&self) -> i64 {
        self.start_time
    }

    fn is_seekable(&self) -> bool {
        true
    }

    fn update_from_packet(&mut self, _packet_timestamp_us: i64) {
        // No-op for standard streams
    }
}

struct DecodingContext {
    input_ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
    framerate: u32,
    stream_start_time: Option<i64>, // Unix timestamp in seconds when stream started (from #EXT-X-PROGRAM-DATE-TIME)
    timeline_ctx: Box<dyn TimelineContext + Send>,
}

impl fmt::Debug for DecodingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        info!("crash test 2");

        f.debug_struct("DecodingContext")
            .field("video_stream_index", &self.video_stream_index)
            .field("framerate", &self.framerate)
            .field("stream_start_time", &self.stream_start_time)
            .field("decoder", &self.decoder.id())
            .field("timeline_offset", &self.timeline_ctx.get_time_offset())
            .finish()
    }
}

#[allow(dead_code)]
struct FrameSynchronizer {
    start_system_time: Instant,
    start_stream_time: f64,
    frames_processed: u64,
}

impl FrameSynchronizer {
    #[allow(dead_code)]
    fn new(current_pts_seconds: f64) -> Self {
        Self {
            start_system_time: Instant::now(),
            start_stream_time: current_pts_seconds,
            frames_processed: 0,
        }
    }

    /// Returns the duration the thread should sleep to synchronize with the video's PTS.
    #[allow(dead_code)]
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
    kill_sig: Arc<AtomicBool>,
    payload_holder: Weak<PayloadHolder>,
    #[allow(unused)]
    session_id: i64,
    decoding_context: Mutex<Option<DecodingContext>>,
    ffmpeg_options: Option<HashMap<String, String>>,
    seekable: Arc<AtomicBool>, // Whether the stream is seekable
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
            kill_sig: Arc::new(AtomicBool::new(false)),
            payload_holder: Arc::downgrade(&payload_holder),
            session_id,
            decoding_context: Mutex::new(None),
            ffmpeg_options,
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

    pub fn initialize_stream(&self) -> Result<(), ffmpeg::Error> {
        trace!("Starting ffmpeg session for {}", &self.video_info.uri);
        let mut option_dict = ffmpeg::Dictionary::new();
        if let Some(ref options) = self.ffmpeg_options {
            for (key, value) in options {
                option_dict.set(key, value);
            }
        }
        let ictx = ffmpeg::format::input_with_dictionary(&self.video_info.uri, option_dict)?;
        let starttimerealtime = unsafe { (*ictx.as_ptr()).start_time_realtime } / 100_0000;
        let format_ctx_ptr = unsafe { ictx.as_ptr() as *mut ffi::AVFormatContext };

        let input = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        info!("start_time_realtime {starttimerealtime}");
        let video_stream_index = input.index();
        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;

        let decoder = context_decoder.decoder().video()?;
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

        // Determine stream type and create appropriate timeline context
        let is_hls_stream =
            self.video_info.uri.contains(".m3u8") || self.video_info.uri.contains("hls");
        let is_rtsp = self.video_info.uri.starts_with("rtsp");

        let timeline_ctx: Box<dyn TimelineContext + Send> = if is_hls_stream {
            info!(
                "Initializing HLS timeline context for {}",
                &self.video_info.uri
            );
            // Create HLS timeline context with direct access to HLS structures
            unsafe {
                match HLSTimelineContext::new(format_ctx_ptr) {
                    Some(ctx) => Box::new(ctx),
                    None => {
                        error!("Failed to create HLS timeline context, falling back to standard");
                        let start_time = {
                            let fmt_ctx = &*format_ctx_ptr;
                            if fmt_ctx.start_time != ffi::AV_NOPTS_VALUE {
                                (fmt_ctx.start_time * 1000000) / ffi::AV_TIME_BASE as i64
                            } else {
                                0
                            }
                        };
                        Box::new(StandardTimelineContext::new(start_time))
                    }
                }
            }
        } else {
            let start_time = unsafe {
                let fmt_ctx = &*format_ctx_ptr;
                if fmt_ctx.start_time != ffi::AV_NOPTS_VALUE {
                    (fmt_ctx.start_time * 1000000) / ffi::AV_TIME_BASE as i64
                } else {
                    0
                }
            };
            Box::new(StandardTimelineContext::new(start_time))
        };

        let context = DecodingContext {
            input_ctx: ictx,
            video_stream_index,
            decoder,
            scaler,
            framerate: frame_rate,
            stream_start_time: Some(starttimerealtime),
            timeline_ctx,
        };
        debug!(
            "Created context {:?} for url: {}",
            context, &self.video_info.uri
        );

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

    pub fn stream(
        &self,
        sendable_texture: SharedSendableTexture,
        dart_update_stream: DartStateStream,
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
        dart_update_stream: &DartStateStream,
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

                // Update HLS timeline if applicable
                ctx.timeline_ctx.update_timeline();

                let framerate = ctx.framerate;
                if framerate > 0 {
                    // 1. Calculate the total number of nanoseconds in one second: 1,000,000,000
                    // 2. Divide this by the framerate to get the duration per frame in nanoseconds.
                    let nanos_per_frame = 1_000_000_000 / framerate;
                    sleep_duration = Some(Duration::from_nanos(nanos_per_frame as u64));
                }

                let mut packet = ffmpeg::Packet::empty();

                // Packet reading and sending to decoder logic
                match packet.read(&mut ctx.input_ctx) {
                    Ok(_) => {
                        // Update HLS timeline from packet timestamp if this is a video packet
                        if packet.stream() == ctx.video_stream_index {
                            // Get packet timestamp and convert to microseconds
                            let video_stream =
                                ctx.input_ctx.stream(ctx.video_stream_index).unwrap();
                            let time_base = video_stream.time_base();
                            let time_base_us = (time_base.numerator() as i64 * 1000000)
                                / time_base.denominator() as i64;

                            let packet_ts = packet.pts().or(packet.dts());

                            if let Some(ts) = packet_ts {
                                let packet_timestamp_us = ts * time_base_us;
                                ctx.timeline_ctx.update_from_packet(packet_timestamp_us);
                            }

                            if let Err(err) = ctx.decoder.send_packet(&packet) {
                                error!("Error sending packet to decoder: {}", err);
                            } else {
                                let _ = self
                                    .on_new_sample(
                                        &mut ctx.decoder,
                                        &mut ctx.scaler,
                                        &mark_frame_avb,
                                    )
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

    /// Seek to a specific timestamp in the stream.
    ///
    /// This method handles seeking for both HLS and standard streams using the
    /// TimelineContext trait to abstract the differences:
    ///
    /// For HLS streams:
    /// 1. Adjusts the timestamp to account for segment timeline
    /// 2. Forces the stream to be seekable (clears AVFMT_FLAG_UNSEEKABLE)
    /// 3. Performs the seek operation
    /// 4. Falls back to backward seek if forward seek fails
    ///
    /// For standard streams:
    /// 1. Uses the timestamp as-is
    /// 2. Performs the seek operation
    ///
    /// # Arguments
    /// * `timestamp_us` - Target timestamp in microseconds (relative to stream start)
    ///
    /// # Returns
    /// * `Ok(())` if seek succeeded
    /// * `Err` if seek failed after all retry attempts
    ///
    /// # Notes
    /// - Flushes the decoder before seeking to avoid artifacts
    /// - Resets frame synchronizer after seek
    /// - Attempts both forward and backward seek for robustness
    pub fn seek(&self, timestamp_us: i64) -> anyhow::Result<()> {
        let mut decoder_ctx = self
            .decoding_context
            .lock()
            .map_err(|e| anyhow::anyhow!("decoding_context mutex poisoned: {e}"))?;

        if let Some(ctx) = decoder_ctx.as_mut() {
            // Validate and clamp timestamp to valid range
            let clamped_timestamp = timestamp_us.max(0);

            if timestamp_us < 0 {
                warn!(
                    "Seek timestamp {} us is negative, clamping to 0",
                    timestamp_us
                );
            }

            // Check if timeline is initialized for HLS streams
            if !ctx.timeline_ctx.is_seekable() {
                return Err(anyhow::anyhow!(
                    "Timeline not initialized yet. HLS streams need to receive packets before seeking is possible. Try again after playback starts."
                ));
            }

            // Adjust timestamp based on stream type (HLS vs standard)
            let adjusted_timestamp = ctx.timeline_ctx.adjust_seek_timestamp(clamped_timestamp);

            info!(
                "Performing seek to {} us (requested: {} us, clamped: {} us, adjusted: {} us, offset: {} us)",
                adjusted_timestamp,
                timestamp_us,
                clamped_timestamp,
                adjusted_timestamp,
                ctx.timeline_ctx.get_time_offset()
            );

            // For HLS streams, force seekable flag
            unsafe {
                let fmt_ctx_ptr = ctx.input_ctx.as_ptr() as *mut ffi::AVFormatContext;
                let fmt_ctx = &mut *fmt_ctx_ptr;

                // Check if this is HLS
                if !fmt_ctx.iformat.is_null() {
                    let iformat = &*fmt_ctx.iformat;
                    let name = std::ffi::CStr::from_ptr(iformat.name);
                    if name.to_string_lossy() == "hls" {
                        // Clear unseekable flag for HLS (forces seekable)
                        fmt_ctx.ctx_flags &= !ffi::AVFMTCTX_UNSEEKABLE;
                        info!("Forced HLS stream to be seekable");
                    }
                }
            }

            // Flush decoder before seeking
            ctx.decoder.flush();

            // Ensure position is non-negative for FFmpeg
            let position = adjusted_timestamp
                .max(0)
                .rescale((1, 1000000), ffmpeg::rescale::TIME_BASE);

            if position < 0 {
                return Err(anyhow::anyhow!(
                    "Adjusted timestamp {} resulted in negative position {} (AV_TIME_BASE units)",
                    adjusted_timestamp,
                    position
                ));
            }

            debug!("Seeking to position {} (AV_TIME_BASE units)", position);

            // Perform the seek
            if let Err(e) = ctx.input_ctx.seek(position, ..position) {
                warn!("Forward seek to position {} failed: {:?}", position, e);

                // Try backward seek as fallback
                match ctx.input_ctx.seek(position, position..) {
                    Ok(_) => {
                        info!("Fallback backward seek to position {} succeeded", position);
                    }
                    Err(e2) => {
                        error!(
                            "Both seek attempts failed for position {} ({} us): forward={:?}, backward={:?}",
                            position, adjusted_timestamp, e, e2
                        );
                        return Err(anyhow::anyhow!(
                            "Failed to seek to {} us (position {}). Forward seek: {:?}, Backward seek: {:?}. The stream may not support seeking or the position is out of range.",
                            adjusted_timestamp,
                            position,
                            e,
                            e2
                        ));
                    }
                }
            }

            // Reset synchronizer after seek
            if let Ok(mut sync) = self.synchronizer.lock() {
                *sync = None;
            }

            info!("Seek completed successfully to {} us", adjusted_timestamp);
        } else {
            return Err(anyhow::anyhow!("Decoding context not initialized"));
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

    fn on_new_sample<F>(
        &self,
        decoder: &mut ffmpeg::decoder::Video,
        scaler: &mut ffmpeg::software::scaling::Context,
        mark_frame_avb: &F,
    ) -> anyhow::Result<()>
    where
        F: Fn(),
    {
        let mut decoded = ffmpeg::util::frame::Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
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
