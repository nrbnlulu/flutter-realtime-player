use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use flutter_plugin_sdk::{PixelBufferTexture, PluginError, TextureDescriptor, TextureFormat};
use log::{debug, warn};
use parking_lot::Mutex;

/// Owns the Flutter-visible texture backing a single video session.
///
/// The shell only allows texture *creation* on the main thread, so this type
/// always creates (and re-creates, on a resolution change) textures via the
/// `MainThreadDispatcher`. Writing and presenting frames has no such
/// restriction and happens directly on the decoder's callback thread.
pub struct FlutterPixelBufferSink {
    state: Mutex<SinkState>,
    resizing: AtomicBool,
}

struct SinkState {
    texture: PixelBufferTexture,
    width: u32,
    height: u32,
}

/// Must be called on the shell's main thread.
fn create_texture(width: u32, height: u32) -> anyhow::Result<PixelBufferTexture> {
    crate::core::gpu_textures()?
        .create_pixel_buffer_texture(TextureDescriptor {
            width,
            height,
            format: TextureFormat::Rgba8Unorm,
        })
        .map_err(|err| anyhow::anyhow!("failed to create pixel buffer texture: {:?}", err))
}

async fn create_texture_on_main_thread(width: u32, height: u32) -> anyhow::Result<PixelBufferTexture> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::core::main_thread_dispatcher()?
        .dispatch(move || {
            let _ = tx.send(create_texture(width, height));
        })
        .map_err(|err| anyhow::anyhow!("failed to dispatch texture creation: {:?}", err))?;
    rx.await
        .map_err(|_| anyhow::anyhow!("main thread dropped the texture creation request"))?
}

impl FlutterPixelBufferSink {
    /// Creates the sink with a placeholder black frame at `width`x`height`.
    pub async fn new(width: u32, height: u32) -> anyhow::Result<Self> {
        let texture = create_texture_on_main_thread(width, height).await?;
        write_black_frame(&texture, width, height)?;
        Ok(Self {
            state: Mutex::new(SinkState {
                texture,
                width,
                height,
            }),
            resizing: AtomicBool::new(false),
        })
    }

    pub fn texture_id(&self) -> i64 {
        self.state.lock().texture.texture_id()
    }

    /// Writes one decoded RGBA8 frame.
    ///
    /// If `width`/`height` no longer match the current texture, this drops
    /// the frame and (unless one is already in flight) kicks off recreating
    /// the texture on the main thread; `on_resized` is invoked with the new
    /// texture id once that finishes.
    pub fn write_frame(
        self: &Arc<Self>,
        width: u32,
        height: u32,
        data: Vec<u8>,
        on_resized: impl FnOnce(i64) + Send + 'static,
    ) -> anyhow::Result<()> {
        let dims_match = {
            let state = self.state.lock();
            state.width == width && state.height == height
        };
        if !dims_match {
            self.request_resize(width, height, on_resized);
            return Ok(());
        }

        let state = self.state.lock();
        match state.texture.try_next_frame() {
            Ok(mut frame) => {
                let src_stride = width as usize * 4;
                frame
                    .write_pixels(move |pixels, row_bytes| {
                        for (src_row, dst_row) in data
                            .chunks(src_stride)
                            .zip(pixels.chunks_mut(row_bytes))
                            .take(height as usize)
                        {
                            dst_row[..src_stride].copy_from_slice(src_row);
                        }
                    })
                    .map_err(|err| anyhow::anyhow!("failed to write video frame: {:?}", err))?;
                frame
                    .present()
                    .map_err(|err| anyhow::anyhow!("failed to present video frame: {:?}", err))?;
                Ok(())
            }
            Err(PluginError::Busy) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("failed to reserve video frame: {:?}", err)),
        }
    }

    fn request_resize(
        self: &Arc<Self>,
        width: u32,
        height: u32,
        on_resized: impl FnOnce(i64) + Send + 'static,
    ) {
        if self.resizing.swap(true, Ordering::AcqRel) {
            return;
        }
        debug!("scheduling texture resize to {}x{}", width, height);
        let this = Arc::clone(self);
        let dispatch_result = crate::core::main_thread_dispatcher().and_then(|dispatcher| {
            dispatcher
                .dispatch(move || {
                    match create_texture(width, height) {
                        Ok(texture) => {
                            let new_id = texture.texture_id();
                            {
                                let mut state = this.state.lock();
                                state.texture = texture;
                                state.width = width;
                                state.height = height;
                            }
                            on_resized(new_id);
                        }
                        Err(err) => warn!("failed to recreate texture for resize: {}", err),
                    }
                    this.resizing.store(false, Ordering::Release);
                })
                .map_err(|err| anyhow::anyhow!("failed to dispatch texture resize: {:?}", err))
        });
        if let Err(err) = dispatch_result {
            warn!("could not schedule texture resize: {}", err);
            self.resizing.store(false, Ordering::Release);
        }
    }
}

fn write_black_frame(texture: &PixelBufferTexture, width: u32, height: u32) -> anyhow::Result<()> {
    let mut frame = texture
        .try_next_frame()
        .map_err(|err| anyhow::anyhow!("failed to reserve initial frame: {:?}", err))?;
    let row_len = width as usize * 4;
    let rows = height as usize;
    frame
        .write_pixels(move |pixels, row_bytes| {
            for row in pixels.chunks_mut(row_bytes).take(rows) {
                for pixel in row[..row_len].chunks_exact_mut(4) {
                    pixel.copy_from_slice(&[0, 0, 0, 255]);
                }
            }
        })
        .map_err(|err| anyhow::anyhow!("failed to write initial frame: {:?}", err))?;
    frame
        .present()
        .map_err(|err| anyhow::anyhow!("failed to present initial frame: {:?}", err))?;
    Ok(())
}
