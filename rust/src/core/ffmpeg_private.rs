use std::os::raw::{c_char, c_int, c_long, c_uchar, c_uint, c_ushort};

// --- Type Aliases and Opaque Structs for FFI ---

/// FFmpeg codec identifier
#[allow(non_camel_case_types)]
pub type AVCodecID = c_int;

/// FFmpeg media type (audio, video, subtitle, etc.)
#[allow(non_camel_case_types)]
pub type AVMediaType = c_int;

/// HLS encryption key type
#[allow(non_camel_case_types)]
pub type KeyType = c_int;

// Opaque forward declarations for structs we only have pointers to
#[allow(non_camel_case_types)]
pub enum AVAES {}

#[allow(non_camel_case_types)]
pub enum AVClass {}

#[allow(non_camel_case_types)]
pub enum AVIOContext {}

#[allow(non_camel_case_types)]
pub enum AVFormatContext {}

#[allow(non_camel_case_types)]
pub enum AVPacket {}

#[allow(non_camel_case_types)]
pub enum AVStream {}

#[allow(non_camel_case_types)]
pub enum AVDictionary {}

/// AVIOInterruptCB callback structure
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct AVIOInterruptCB {
    pub callback: Option<unsafe extern "C" fn(*mut std::ffi::c_void) -> c_int>,
    pub opaque: *mut std::ffi::c_void,
}

/// FFmpeg internal IO context wrapper
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct FFIOContext {
    // This is an embedded struct whose full definition is not provided.
    // In a real binding, this would contain the AVIOContext struct and other fields.
    // For now, it's a placeholder with sufficient size/alignment.
    _opaque: [u8; 512], // Placeholder with reasonable size
}

// --- HLS Related Structs ---

/// HLS Audio Setup Information
///
/// Contains codec-specific audio configuration for HLS streams,
/// particularly for formats like AAC that need initialization data.
#[repr(C)]
pub struct HLSAudioSetupInfo {
    pub codec_id: AVCodecID,
    pub codec_tag: c_uint,
    pub priming: c_ushort,
    pub version: c_uchar,
    pub setup_data_length: c_uchar,
    pub setup_data: [c_uchar; 10],
}

/// HLS Encryption Context
///
/// Manages AES encryption/decryption for encrypted HLS segments.
#[repr(C)]
pub struct HLSCryptoContext {
    pub aes_ctx: *mut AVAES,
    pub key: [c_uchar; 16],
    pub iv: [c_uchar; 16],
}

/// HLS Segment Information
///
/// Represents a single segment (typically a .ts file) in the HLS playlist.
/// This is the key structure for calculating timeline offsets.
#[repr(C)]
pub struct segment {
    /// Duration of this segment in AV_TIME_BASE units (microseconds)
    pub duration: c_long,

    /// Offset of the URL in the playlist file
    pub url_offset: c_long,

    /// Size of the segment in bytes (-1 if unknown)
    pub size: c_long,

    /// URL of the segment (can be relative or absolute)
    pub url: *mut c_char,

    /// Encryption key URL (if segment is encrypted)
    pub key: *mut c_char,

    /// Type of encryption used (NONE, AES-128, SAMPLE-AES)
    pub key_type: KeyType,

    /// Initialization vector for encryption
    pub iv: [c_uchar; 16],

    /// Reference to initialization segment (for fMP4 playlists)
    pub init_section: *mut segment,
}

/// HLS Playlist Information
///
/// Represents a single playlist (variant or media playlist).
/// This can be a master playlist reference or an actual media playlist with segments.
#[repr(C)]
pub struct playlist {
    /// URL of the playlist (up to 4KB)
    pub url: [c_uchar; 4096],

    /// Internal IO context for reading the playlist
    pub pb: FFIOContext,

    /// Buffer for reading playlist data
    pub read_buffer: *mut c_uchar,

    /// Current input IO context
    pub input: *mut AVIOContext,

    /// Whether input reading is complete
    pub input_read_done: c_int,

    /// Next input IO context (for seamless switching)
    pub input_next: *mut AVIOContext,

    /// Whether next input has been requested
    pub input_next_requested: c_int,

    /// Parent format context
    pub parent: *mut AVFormatContext,

    /// Index of this playlist
    pub index: c_int,

    /// Format context for this playlist
    pub ctx: *mut AVFormatContext,

    /// Current packet being processed
    pub pkt: *mut AVPacket,

    /// Whether this playlist has no header
    pub has_noheader_flag: c_int,

    /// Array of streams in this playlist
    pub streams: *mut *mut AVStream,

    /// Number of segments in the playlist
    pub n_segments: c_int,

    /// Array of segments
    pub segments: *mut *mut segment,

    /// Whether this is a live playlist (no EXT-X-ENDLIST)
    pub finished: c_int,

    /// Type of playlist (event, VOD, etc.)
    pub r#type: c_int,

    /// Current sequence number being played
    pub cur_seq_no: c_long,

    /// Starting sequence number in playlist
    pub start_seq_no: c_long,

    /// Target duration from EXT-X-TARGETDURATION
    pub target_duration: c_long,

    /// Age of playlist (for cache management)
    pub age: c_long,
}

/// HLS Rendition Information
///
/// Represents alternative audio, video, or subtitle tracks.
/// Used for alternate audio tracks, subtitles, etc.
#[repr(C)]
pub struct rendition {
    /// Media type (audio, video, subtitles)
    pub r#type: AVMediaType,

    /// Pointer to the playlist for this rendition
    pub playlist: *mut playlist,

    /// Group ID from EXT-X-MEDIA tag
    pub group_id: [c_uchar; 64],

    /// Language code (e.g., "en", "es")
    pub language: [c_uchar; 64],

    /// Human-readable name
    pub name: [c_uchar; 64],

    /// Stream disposition flags
    pub disposition: c_int,
}

/// HLS Variant Information
///
/// Represents a variant stream (different bitrate/quality level).
/// Used for adaptive bitrate streaming.
#[repr(C)]
pub struct variant {
    /// Bandwidth of this variant in bits per second
    pub bandwidth: c_int,

    /// Number of playlists in this variant (usually 1)
    pub n_playlists: c_int,

    /// Array of playlists for this variant
    pub playlists: *mut *mut playlist,
}

/// HLS Context
///
/// Main context structure for the HLS demuxer. This is the private data
/// attached to AVFormatContext->priv_data when demuxing HLS streams.
///
/// Updated to match recent FFmpeg version (7.x / 6.x)
#[repr(C)]
pub struct HLSContext {
    /// AVClass for logging and options
    pub class: *mut AVClass,

    /// Parent AVFormatContext
    pub ctx: *mut AVFormatContext,

    /// Number of variants (different bitrates)
    pub n_variants: c_int,

    /// Array of variant streams
    pub variants: *mut *mut variant,

    /// Number of playlists
    pub n_playlists: c_int,

    /// Array of playlists
    pub playlists: *mut *mut playlist,

    /// Number of renditions (alternate audio/subs)
    pub n_renditions: c_int,

    /// Array of renditions
    pub renditions: *mut *mut rendition,

    /// Current sequence number
    pub cur_seq_no: c_long,

    /// M3U8 hold counters
    pub m3u8_hold_counters: c_int,

    /// Live start index
    pub live_start_index: c_int,

    /// Prefer X-START attribute
    pub prefer_x_start: c_int,

    /// Whether first packet has been read
    pub first_packet: c_int,

    /// First timestamp encountered
    pub first_timestamp: c_long,

    /// Current timestamp
    pub cur_timestamp: c_long,

    /// Interrupt callback for blocking operations
    pub interrupt_callback: *mut AVIOInterruptCB,

    /// AVIO options dictionary
    pub avio_opts: *mut AVDictionary,

    /// Segment format options dictionary
    pub seg_format_opts: *mut AVDictionary,

    /// Allowed file extensions
    pub allowed_extensions: *mut c_char,

    /// Allowed segment extensions
    pub allowed_segment_extensions: *mut c_char,

    /// Whether to be strict about extensions
    pub extension_picky: c_int,

    /// Maximum number of playlist reload attempts
    pub max_reload: c_int,

    /// HTTP persistent connections
    pub http_persistent: c_int,

    /// Whether to allow multiple concurrent connections
    pub http_multiple: c_int,

    /// Whether HTTP seeking is enabled
    pub http_seekable: c_int,

    /// Maximum segment retry attempts
    pub seg_max_retry: c_int,

    /// Playlist AVIOContext
    pub playlist_pb: *mut AVIOContext,

    /// Crypto context for decryption
    pub crypto_ctx: HLSCryptoContext,
}

// Safety markers
unsafe impl Send for HLSContext {}
unsafe impl Sync for HLSContext {}

impl HLSContext {
    /// Get the first playlist (typically used for single-variant streams)
    ///
    /// # Safety
    /// The caller must ensure the context is initialized and has at least one playlist.
    pub unsafe fn get_first_playlist(&self) -> Option<&playlist> {
        if self.n_playlists > 0 && !self.playlists.is_null() {
            let first_playlist_ptr = *self.playlists;
            if !first_playlist_ptr.is_null() {
                return Some(&*first_playlist_ptr);
            }
        }
        None
    }

    /// Get a playlist by index
    ///
    /// # Safety
    /// The caller must ensure the index is valid and within bounds.
    pub unsafe fn get_playlist(&self, index: usize) -> Option<&playlist> {
        if index < self.n_playlists as usize && !self.playlists.is_null() {
            let playlist_ptr = *self.playlists.add(index);
            if !playlist_ptr.is_null() {
                return Some(&*playlist_ptr);
            }
        }
        None
    }
}

impl playlist {
    /// Get a segment by index
    ///
    /// # Safety
    /// The caller must ensure the index is valid and within bounds.
    pub unsafe fn get_segment(&self, index: usize) -> Option<&segment> {
        if index < self.n_segments as usize && !self.segments.is_null() {
            let segment_ptr = *self.segments.add(index);
            if !segment_ptr.is_null() {
                return Some(&*segment_ptr);
            }
        }
        None
    }

    /// Calculate total duration of all segments in the playlist
    ///
    /// # Safety
    /// The caller must ensure the segments array is valid.
    pub unsafe fn calculate_total_duration(&self) -> c_long {
        let mut total: c_long = 0;
        for i in 0..self.n_segments as usize {
            if let Some(seg) = self.get_segment(i) {
                total += seg.duration;
            }
        }
        total
    }

    /// Calculate duration up to a specific sequence number
    ///
    /// # Safety
    /// The caller must ensure the segments array is valid.
    pub unsafe fn calculate_duration_until_seq(&self, seq_no: c_long) -> c_long {
        let mut total: c_long = 0;
        let segments_to_count = (seq_no - self.start_seq_no).min(self.n_segments as c_long);

        for i in 0..segments_to_count as usize {
            if let Some(seg) = self.get_segment(i) {
                total += seg.duration;
            }
        }
        total
    }
}
