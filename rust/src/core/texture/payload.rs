use std::sync::{Arc, Mutex};

use irondash_texture::{BoxedPixelData, PayloadProvider, PixelData, PixelDataProvider};
use log::{debug, error};

/// Unified RGBA frame used by all decoder backends (FFmpeg, GStreamer, etc.).
#[derive(Clone)]
pub struct RawRgbaFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RawRgbaFrame {
    /// Create a black (zeroed) frame with the given dimensions.
    pub fn black(width: u32, height: u32) -> Self {
        let mut data = vec![0u8; (width * height * 4) as usize];
        // Set alpha to 255
        for chunk in data.chunks_mut(4) {
            chunk[3] = 255;
        }
        Self {
            width,
            height,
            data,
        }
    }
}

impl PixelDataProvider for RawRgbaFrame {
    fn get(&self) -> PixelData<'_> {
        PixelData {
            width: self.width as _,
            height: self.height as _,
            data: &self.data,
        }
    }
}

/// Shared pixel data — cheaply cloneable via Arc.
pub type SharedPixelData = Arc<RawRgbaFrame>;

pub struct PayloadHolder {
    current_frame: Mutex<Option<SharedPixelData>>,
    previous_frame: Mutex<Option<SharedPixelData>>,
}

impl PayloadHolder {
    pub fn new() -> Self {
        Self {
            current_frame: Mutex::new(None),
            previous_frame: Mutex::new(None),
        }
    }

    pub fn set_payload(&self, payload: SharedPixelData) {
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
        *prev_frame = curr_frame.take();
        *curr_frame = Some(payload);
    }

    pub fn previous_frame(&self) -> Option<SharedPixelData> {
        self.previous_frame.lock().ok().and_then(|f| f.clone())
    }
}

impl PayloadProvider<BoxedPixelData> for PayloadHolder {
    fn get_payload(&self) -> BoxedPixelData {
        let default_frame = || -> BoxedPixelData {
            debug!("No frame available, returning a default black frame.");
            Box::new(RawRgbaFrame::black(640, 480))
        };

        let curr_frame_lock = self.current_frame.lock();
        if let Ok(curr_frame) = curr_frame_lock {
            if let Some(ref frame) = *curr_frame {
                return Box::new((**frame).clone());
            }
        } else {
            error!("current_frame mutex poisoned in get_payload");
            return default_frame();
        }

        let prev_frame_lock = self.previous_frame.lock();
        if let Ok(prev_frame) = prev_frame_lock {
            if let Some(ref frame) = *prev_frame {
                debug!("Returning previous frame");
                return Box::new((**frame).clone());
            }
        } else {
            error!("previous_frame mutex poisoned in get_payload");
        }

        default_frame()
    }
}
