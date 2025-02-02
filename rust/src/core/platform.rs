mod linux {
    use gdk::glib::{translate::{FromGlibPtrNone, ToGlibPtr}, Cast};
    use lazy_static::lazy_static;
    use log::{error, info};

    use std::{
        collections::HashMap,
        iter::Map,
        ops::Deref,
        sync::{Arc, Mutex},
    };

    use irondash_engine_context::EngineContext;

    use crate::core::{
        ffi::{self, gst_egl_ext},
        fluttersink::types::Orientation,
        gl,
    };

    pub type GlCtx = gst_gl::GLContext;
    pub struct EglImageWrapper {
        pub image: gst_egl_ext::EGLImage,
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
            orientation: Orientation,
        ) -> GstNativeFrameType {
            Arc::new(Self {
                image: image_ptr,
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

        /// Get the context for the given window id
        /// if there is no context for the given window id, we create a new one
        pub fn get_context(&self, engine_handle: i64) -> Option<GlCtx> {
            let mut store = self.store.lock().unwrap();
            if let Some(context) = store.get(&engine_handle) {
                return Some(context.clone());
            } else {
                let context = self.create_gl_ctx(engine_handle).unwrap();
                store.insert(engine_handle, context.clone());
                Some(context)
            }
        }
        pub fn set_context(&self, engine_handle: i64, context: GlCtx) {
            self.store.lock().unwrap().insert(engine_handle, context);
        }

        /// This function MUST be called from the platform's main thread
        /// because we want to use gtk's gl context.
        fn create_gl_ctx(&self, engine_handle: i64) -> anyhow::Result<gst_gl::GLContext> {
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
            let display = unsafe { gdk_sys::gdk_window_get_display(window) };
            let display = unsafe { gdk::Display::from_glib_none(display) };
            let (_, wrapped_context) = initialize_waylandegl(&display)?;
            Ok(wrapped_context)
        }


    }

    fn initialize_waylandegl(
        display: &gdk::Display,
    ) -> anyhow::Result<(gst_gl::GLDisplay, gst_gl::GLContext)> {
        info!("Initializing GL for Wayland EGL backend and display");

        let platform = gst_gl::GLPlatform::EGL;
        let (gl_api, _, _) = gst_gl::GLContext::current_gl_api(platform);
        let gl_ctx = gst_gl::GLContext::current_gl_context(platform);

        if gl_ctx == 0 {
            return Err(anyhow::anyhow!("Failed to get handle from GdkGLContext"));
        }

        // FIXME: bindings
        unsafe {
            use glib::translate::*;

            // let wayland_display = gdk_wayland::WaylandDisplay::wl_display(display.downcast());
            // get the ptr directly since we are going to use it raw
            let display = display
                .downcast_ref::<gdkwayland::WaylandDisplay>()
                .unwrap();
            let wayland_display =
            gdkwayland::ffi::gdk_wayland_display_get_wl_display(display.to_glib_none().0);
            if wayland_display.is_null() {
                return Err(anyhow::anyhow!("Failed to get Wayland display"));
            }

            let gst_display =
                gst_gl_wayland::ffi::gst_gl_display_wayland_new_with_display(wayland_display);
            let gst_display =
                gst_gl::GLDisplay::from_glib_full(gst_display as *mut gst_gl::ffi::GstGLDisplay);

            let wrapped_context =
                gst_gl::GLContext::new_wrapped(&gst_display, gl_ctx, platform, gl_api);

            let wrapped_context = match wrapped_context {
                None => {
                    return Err(anyhow::anyhow!("Failed to create wrapped GL context"));
                }
                Some(wrapped_context) => wrapped_context,
            };

            Ok((gst_display, wrapped_context))
        }
    }
}

pub use linux::*;
