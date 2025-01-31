mod linux {
    use lazy_static::lazy_static;
    use rayon::ThreadPool;

    use std::{
        collections::HashMap,
        iter::Map,
        sync::{Arc, Mutex},
    };

    use irondash_engine_context::EngineContext;

    use crate::core::{ffi::gst_egl_ext, fluttersink::types::Orientation};

    pub type GlCtx = gst_gl::GLContext;
    pub struct EglImageWrapper {
        pub image_ptr: gst_egl_ext::EGLImage,
        pub width: u32,
        pub height: u32,
        pub format: gst_video::VideoFormat,
        pub orientation: Orientation,
    }
    impl EglImageWrapper {
        pub fn new(
            image_ptr: gst_egl_ext::EGLImage,
            width: u32,
            height: u32,
            format: gst_video::VideoFormat,
            orientation:    Orientation,
        ) -> GstNativeFrameType {
            Arc::new(Self {
                image_ptr,
                width,
                height,
                format,
                orientation,
            })
        }
    }
    
    pub(crate) type GstNativeFrameType = Arc<EglImageWrapper>;

    pub struct GlManager {
        // stores the context for each window
        store: Mutex<HashMap<i64, GlCtx>>,
    }
    impl GlManager {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
        /// This function MUST be called from the platform's main thread
        /// because we want to use gtk's gl context.
        fn create_gl_ctx(&self, engine_handle: i64) -> GlCtx {
            let engine = EngineContext::get().unwrap();
            let fl_view = engine.get_flutter_view(engine_handle).unwrap();
            let fl_view = unsafe { std::mem::transmute(fl_view) };
            let gtk_widget = unsafe {
                std::mem::transmute(gobject_sys::g_type_check_instance_cast(
                    fl_view,
                    gtk_sys::gtk_widget_get_type(),
                ))
            };

            let window = unsafe { gtk_sys::gtk_widget_get_parent_window(gtk_widget) };
            let mut error: *mut glib_sys::GError = std::ptr::null_mut();
            let error_ptr: *mut *mut glib_sys::GError = &mut error;
            unsafe { Arc::from_raw(gdk_sys::gdk_window_create_gl_context(window, error_ptr)) }
        }

        /// Get the context for the given window id
        /// if there is no context for the given window id, we create a new one
        pub fn get_context(&self, engine_handle: i64) -> Option<GlCtx> {
            self.store.lock().unwrap().get(&engine_handle).cloned()
        }
        pub fn set_context(&self, engine_handle: i64, context: GlCtx) {
            self.store.lock().unwrap().insert(engine_handle, context);
        }
    }
}

pub use linux::*;
