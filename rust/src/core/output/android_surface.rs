#![cfg(target_os = "android")]

use std::sync::Arc;

use anyhow::{Context, Result};
use gst::prelude::*;
use irondash_texture::{NativeWindow, SendableTexture, Texture};
use ndk_sys::{ANativeWindow, ANativeWindow_setBuffersGeometry};
use parking_lot::Mutex;

use crate::{core::texture::FlutterTextureSession, utils::invoke_on_platform_main_thread};

pub struct AndroidVideoOutput {
    texture_id: i64,
    sendable_texture: Mutex<Option<Arc<SendableTexture<NativeWindow>>>>,
    native_window: SendableNativeWindow,
}

impl AndroidVideoOutput {
    pub fn new(engine_handle: i64) -> Result<Arc<Self>> {
        invoke_on_platform_main_thread(move || -> Result<_> {
            let texture = Texture::<NativeWindow>::new(engine_handle)
                .map_err(|err| anyhow::anyhow!("create Android native-window texture: {err:?}"))?;
            let texture_id = texture.id();
            let native_window = SendableNativeWindow::new(texture.get());
            let sendable_texture = texture.into_sendable_texture();

            log::info!("Android video output texture created, id={texture_id}");

            Ok(Arc::new(Self {
                texture_id,
                sendable_texture: Mutex::new(Some(sendable_texture)),
                native_window,
            }))
        })
    }

    pub fn texture_id(&self) -> i64 {
        self.texture_id
    }

    pub fn window_handle(&self) -> usize {
        self.native_window.as_ptr() as usize
    }

    pub fn set_video_size(&self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        let result = unsafe {
            ANativeWindow_setBuffersGeometry(
                self.native_window.as_ptr(),
                width as i32,
                height as i32,
                0,
            )
        };

        if result != 0 {
            log::warn!(
                "Android video output: ANativeWindow_setBuffersGeometry({width}x{height}) failed: {result}"
            );
        }
    }

    pub fn destroy_texture(&self) {
        let sendable_texture = self.sendable_texture.lock().take();
        if let Some(sendable_texture) = sendable_texture {
            invoke_on_platform_main_thread(move || {
                drop(sendable_texture);
            });
        }
    }
}

impl Drop for AndroidVideoOutput {
    fn drop(&mut self) {
        let sendable_texture = self.sendable_texture.lock().take();
        if let Some(sendable_texture) = sendable_texture {
            invoke_on_platform_main_thread(move || {
                drop(sendable_texture);
            });
        }
    }
}

impl FlutterTextureSession for AndroidVideoOutput {
    fn mark_frame_available(&self) {
        // SurfaceTexture's OnFrameAvailableListener is driven by eglSwapBuffers.
    }

    fn terminate(&self) {
        self.destroy_texture();
    }
}

struct SendableNativeWindow {
    native_window: NativeWindow,
}

impl SendableNativeWindow {
    fn new(native_window: NativeWindow) -> Self {
        Self { native_window }
    }

    fn as_ptr(&self) -> *mut ANativeWindow {
        self.native_window.get_native_window()
    }
}

unsafe impl Send for SendableNativeWindow {}
unsafe impl Sync for SendableNativeWindow {}

pub fn set_window_handle(sink: &gst::Element, output: &AndroidVideoOutput) -> Result<()> {
    let overlay = sink
        .dynamic_cast_ref::<gst_video::VideoOverlay>()
        .with_context(|| format!("{} does not implement GstVideoOverlay", sink.name()))?;

    unsafe {
        gst_video::prelude::VideoOverlayExtManual::set_window_handle(
            overlay,
            output.window_handle(),
        );
    }

    Ok(())
}

pub fn install_video_size_watch(
    sink: &gst::Element,
    output: Arc<AndroidVideoOutput>,
    on_size_changed: Arc<dyn Fn(u32, u32) + Send + Sync + 'static>,
    label: &'static str,
) -> Result<()> {
    let sink_pad = sink
        .static_pad("sink")
        .with_context(|| format!("{label}: glimagesink sink pad not found"))?;
    let last_size = Arc::new(Mutex::new(None::<(u32, u32)>));

    sink_pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_pad, info| {
        let Some(event) = info.event() else {
            return gst::PadProbeReturn::Ok;
        };

        let gst::EventView::Caps(caps_event) = event.view() else {
            return gst::PadProbeReturn::Ok;
        };

        match gst_video::VideoInfo::from_caps(caps_event.caps()) {
            Ok(video_info) => {
                let width = video_info.width();
                let height = video_info.height();

                let mut guard = last_size.lock();
                if *guard != Some((width, height)) {
                    *guard = Some((width, height));
                    drop(guard);

                    log::debug!("{label}: video dimensions changed to {width}x{height}");
                    output.set_video_size(width, height);
                    on_size_changed(width, height);
                }
            }
            Err(err) => {
                log::warn!("{label}: failed to parse video caps: {err}");
            }
        }

        gst::PadProbeReturn::Ok
    });

    Ok(())
}
