//! GStreamer 1.30's public AHardwareBuffer query. The androidmedia allocator
//! finishes its acquire-fence wait before exposing the buffer; retaining the
//! GstBuffer keeps its AImage leased until the shell releases the GPU readers.
use anyhow::Context;
use flutter_plugin_sdk::{
    android::{HardwareBufferFrame, HardwareBufferTexture},
    PluginError,
};
use std::{ffi::c_void, ptr::NonNull};

unsafe extern "C" {
    fn gst_ahardware_buffer_memory_peek_buffer(
        memory: *mut gst::ffi::GstMemory,
        buffer: *mut *mut c_void,
    ) -> i32;
    fn gst_gl_get_affine_transformation_meta_as_ndc(
        meta: *mut gst_video::ffi::GstVideoAffineTransformationMeta,
        matrix: *mut f32,
    );
}

pub(super) fn present(
    texture: &HardwareBufferTexture,
    caps: &gst::CapsRef,
    buffer: gst::Buffer,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        caps.features(0)
            .is_some_and(|f| f.contains("memory:AHardwareBuffer")),
        "expected hardware-buffer memory"
    );
    anyhow::ensure!(
        buffer.n_memory() == 1,
        "expected one hardware-buffer allocation"
    );
    anyhow::ensure!(
        buffer.peek_memory(0).is_type("GstAmcAImageMemory"),
        "hardware buffer lacks androidmedia's ready-image lease"
    );
    let mut native = std::ptr::null_mut();
    // SAFETY: buffer owns this memory for the query and subsequent frame lease.
    let found = unsafe {
        gst_ahardware_buffer_memory_peek_buffer(
            buffer.peek_memory(0).as_ptr().cast_mut(),
            &mut native,
        )
    };
    anyhow::ensure!(
        found != 0,
        "GStreamer did not expose a ready hardware buffer"
    );
    let native = NonNull::new(native).context("null hardware buffer")?;
    let structure = caps.structure(0).context("missing video caps")?;
    let width = u32::try_from(structure.get::<i32>("width")?)?;
    let height = u32::try_from(structure.get::<i32>("height")?)?;
    let mut crop = [0, 0, width, height];
    if let Some(meta) = buffer.meta::<gst_video::VideoCropMeta>() {
        let (x, y, w, h) = meta.rect();
        crop = [x, y, w, h];
    } else if let Some(meta) = buffer.meta::<gst_video::VideoAffineTransformationMeta>() {
        let mut matrix = [0.0; 16];
        // SAFETY: the meta is borrowed from this live buffer, output has 16 floats.
        unsafe {
            gst_gl_get_affine_transformation_meta_as_ndc(
                meta.as_ptr().cast_mut(),
                matrix.as_mut_ptr(),
            );
        }
        crop = super::video::crop_from_ndc(matrix, width, height)?;
    }
    // SAFETY: androidmedia publishes only after its acquire fence is ready.
    // The GstBuffer owns the AImage lease and prevents its pixels being reused;
    // ownership is transferred through the shell until GPU sampling completes.
    let frame = unsafe {
        HardwareBufferFrame::new(native, crop, buffer)
            .map_err(|e| anyhow::anyhow!("invalid decoder frame: {e:?}"))?
    };
    match texture.present(frame) {
        Ok(()) | Err(PluginError::Busy) => Ok(()),
        Err(error) => anyhow::bail!("hardware-buffer presentation: {error:?}"),
    }
}
