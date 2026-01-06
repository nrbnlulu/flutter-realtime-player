use std::sync::Mutex;

use ffmpeg::format::Pixel;
use irondash_texture::{BoxedPixelData, PayloadProvider};
use log::{debug, error, trace};

#[derive(Clone)]
pub struct FFmpegFrameWrapper(pub ffmpeg::util::frame::Video, pub Option<Vec<u8>>);

impl irondash_texture::PixelDataProvider for FFmpegFrameWrapper {
    fn get(&self) -> irondash_texture::PixelData<'_> {
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
    pub fn from_frame(frame: ffmpeg::util::frame::Video) -> Self {
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
                width,
                expected_stride,
                stride
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

    pub fn previous_frame(&self) -> Option<Box<FFmpegFrameWrapper>> {
        self.previous_frame
            .lock()
            .ok()
            .and_then(|frame| frame.clone())
    }
}

impl PayloadProvider<BoxedPixelData> for PayloadHolder {
    fn get_payload(&self) -> BoxedPixelData {
        // Create a default frame to return on error or if no frame is available.
        let default_frame = || {
            debug!("No frame available, returning a default black frame.");
            Box::new(FFmpegFrameWrapper::from_frame(
                ffmpeg::util::frame::Video::new(Pixel::RGBA, 640, 480),
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
