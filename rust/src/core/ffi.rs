pub mod gst_egl_ext {
    use gst::ffi::GstMiniObject;
    use gst_gl_sys::{GstGLContext, GstGLFormat, GstGLMemory};

    pub type GstEGLImage = *mut std::ffi::c_void;

    extern "C" {

        pub fn gst_egl_image_from_texture(
            //  A pointer to a `GstGLContext` (must be an EGL context).
            context: *mut GstGLContext,
            // A pointer to a `GstGLMemory`.
            gl_mem: *mut GstGLMemory,
            // Additional attributes to add to the `eglCreateImage()` call.
            attribs: *mut u32,
        ) -> GstEGLImage;

    }
}
