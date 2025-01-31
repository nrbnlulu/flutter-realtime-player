pub mod gst_egl_ext {
    use glib::translate::*;
    use gst_gl_sys::{GstGLContext, GstGLMemory};
    macro_rules! skip_assert_initialized {
        () => {};
    }
    mod ffi {

        use glib::ffi::gpointer;
        use gst::ffi::GstMiniObject;
        use gst_gl_sys::{GstGLContext, GstGLFormat, GstGLMemory};

        pub type GstEGLImageDestroyNotify =
            Option<unsafe extern "C" fn(image: *mut GstEGLImage, data: gpointer)>;

        #[derive(Copy, Clone)]
        #[repr(C)]
        pub struct GstEGLImage {
            pub parent: GstMiniObject,
            pub context: *mut GstGLContext,
            pub image: gpointer,
            pub format: GstGLFormat,
            pub destroy_data: gpointer,
            pub destroy_notify: GstEGLImageDestroyNotify,
            pub _padding: [gpointer; 4],
        }
        extern "C" {

            pub fn gst_egl_image_from_texture(
                //  A pointer to a `GstGLContext` (must be an EGL context).
                context: *mut GstGLContext,
                // A pointer to a `GstGLMemory`.
                gl_mem: *mut GstGLMemory,
                // Additional attributes to add to the `eglCreateImage()` call.
                attribs: *mut usize,
            ) -> *mut GstEGLImage;

            pub fn gst_egl_image_get_type() -> glib::ffi::GType;
        }
    }
    gst::mini_object_wrapper!(EGLImage, EGLImageRef, ffi::GstEGLImage, || {
        ffi::gst_egl_image_get_type()
    });

    pub fn egl_image_from_texture(
        context: *mut GstGLContext,
        gl_mem: *mut GstGLMemory,
        attribs: &mut [usize],
    ) -> EGLImage {
        unsafe {
            EGLImage::from_glib_full(ffi::gst_egl_image_from_texture(
                context,
                gl_mem,
                attribs.as_mut_ptr(),
            ))
        }
    }
}
