use super::types::Orientation;

#[cfg(target_os = "linux")]
mod linux {
    use gdk::glib::{translate::FromGlibPtrNone, Cast};
    use glow::HasContext;
    use gst::glib::translate::{FromGlibPtrFull, ToGlibPtr};
    use gst_gl::{ffi::gst_gl_context_activate, prelude::GLContextExt, GLVideoFrameExt};
    use gst_video::VideoFrameExt;
    use irondash_texture::BoxedGLTexture;
    use log::{debug, info, trace};

    use crate::core::fluttersink::utils;
    use std::{
        cell::RefCell,
        collections::HashMap,
        ptr,
        sync::{Arc, Mutex},
    };

    use irondash_engine_context::EngineContext;

    pub type GlCtx = gst_gl::GLContext;
    use irondash_texture::GLTextureProvider;

    use crate::core::gl::{self};

    #[derive(Debug, Clone)]
    pub struct GLTexture {
        pub target: u32,
        pub name_raw: u32,
        pub width: i32,
        pub height: i32,
    }

    impl GLTexture {
        pub fn new(name_raw: u32, width: i32, height: i32) -> Self {
            Self {
                target: gl::TEXTURE_2D,
                name_raw,
                width,
                height,
            }
        }
    }

    impl GLTextureProvider for GLTexture {
        fn get(&self) -> irondash_texture::GLTexture {
            irondash_texture::GLTexture {
                target: self.target,
                name: &self.name_raw,
                width: self.width,
                height: self.height,
            }
        }
    }

    pub struct LinuxNativeTexture {
        pub texture_id: u32,
        pub width: u32,
        pub height: u32,
        pub format: gst_video::VideoFormat,
    }
    impl LinuxNativeTexture {
        pub fn new(
            texture_id: u32,
            width: u32,
            height: u32,
            format: gst_video::VideoFormat,
        ) -> Self {
            Self {
                texture_id,
                width,
                height,
                format,
            }
        }
        pub fn from_gst(
            frame: gst_gl::GLVideoFrame<gst_gl::gl_video_frame::Readable>,
        ) -> anyhow::Result<NativeTextureType> {
            let texture_id = frame.texture_id(0)?;

            Ok(LinuxNativeTexture {
                texture_id,
                width: frame.width(),
                height: frame.height(),
                format: frame.format(),
            })
        }

        pub fn as_texture_provider(&self) -> BoxedGLTexture {
            Box::new(GLTexture::new(
                self.texture_id,
                self.width as _,
                self.height as _,
            ))
        }
    }

    pub(crate) type NativeFrameType = LinuxNativeTexture;

    pub struct GlManager {
        // stores the context for each window
        store: Mutex<HashMap<i64, (gst_gl::GLDisplay, gst_gl::GLContext)>>,
        fallback_texture: Mutex<HashMap<i64, u32>>,
    }
    impl GlManager {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
                fallback_texture: Mutex::new(HashMap::new()),
            }
        }

        pub fn get_fallback_texture_name(&self, engine_id: i64) -> u32 {
            // check if we have a fallback texture for the given engine id
            let mut store = self.fallback_texture.lock().unwrap();
            if let Some(texture_name) = store.get(&engine_id) {
                return *texture_name;
            } else {
                gl_loader::init_gl();
                let gl_context = unsafe {
                    glow::Context::from_loader_function(|s| {
                        std::mem::transmute(gl_loader::get_proc_address(s))
                    })
                };

                unsafe {
                    let texture = gl_context.create_texture().unwrap();
                    let texture_name = texture.0.get();
                    gl_context.bind_texture(glow::TEXTURE_2D, Some(texture));
                    gl_context.tex_image_2d(
                        glow::TEXTURE_2D,
                        0,
                        glow::RGBA as i32,
                        200,
                        500,
                        0,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelUnpackData::Slice(Some(&vec![0, 255, 0, 255].repeat(200 * 500))),
                    );
                    gl_context.bind_texture(glow::TEXTURE_2D, Some(texture));
                    store.insert(engine_id, texture_name);
                    return texture_name;
                }
            };
        }

        /// Get the context for the given window id
        /// if there is no context for the given window id, we create a new one
        pub fn get_context(
            &self,
            engine_handle: i64,
        ) -> Option<(gst_gl::GLDisplay, gst_gl::GLContext)> {
            trace!(
                "Creating new GL context for engine handle: {}",
                engine_handle
            );

            let mut store = self.store.lock().unwrap();
            if let Some(context) = store.get(&engine_handle) {
                return Some(context.clone());
            } else {
                trace!(
                    "Creating new GL context for engine handle: {}",
                    engine_handle
                );
                let context = utils::invoke_on_gs_main_thread(move || {
                    Self::create_gl_ctx(engine_handle).unwrap()
                });
                store.insert(engine_handle, context.clone());
                Some(context)
            }
        }
        pub fn set_context(
            &self,
            engine_handle: i64,
            context: (gst_gl::GLDisplay, gst_gl::GLContext),
        ) {
            self.store.lock().unwrap().insert(engine_handle, context);
        }

        /// This function MUST be called from the platform's main thread
        /// because we want to use gtk's gl context.
        fn create_gl_ctx(
            engine_handle: i64,
        ) -> anyhow::Result<(gst_gl::GLDisplay, gst_gl::GLContext)> {
            let engine = EngineContext::get().unwrap();
            let fl_view = engine.get_flutter_view(engine_handle).unwrap();
            let fl_view = unsafe { std::mem::transmute(fl_view) };
            let gtk_widget = unsafe {
                std::mem::transmute(gobject_sys::g_type_check_instance_cast(
                    fl_view,
                    gtk_sys::gtk_widget_get_type(),
                ))
            };
            // usually its already realized, in the future we might want to connect to the realize signal.
            let window = unsafe { gtk_sys::gtk_widget_get_parent_window(gtk_widget) };
            let shared_ctx = _glib_err_to_result(gdk_sys::gdk_window_create_gl_context, window)?;
            // realize the context as per https://docs.gtk.org/gdk3/method.Window.create_gl_context.html
            let res = gbool_to_bool(_glib_err_to_result(
                gdk_sys::gdk_gl_context_realize,
                shared_ctx,
            )?);
            debug!("GL context realized: {:?}", res);
            unsafe { gdk_sys::gdk_gl_context_make_current(shared_ctx) };
            // get the display of the context
            let display = unsafe { gdk_sys::gdk_gl_context_get_display(shared_ctx) };
            let display = unsafe { gdk::Display::from_glib_none(display) };
            trace!("Creating GL context for window: {:?}", window);
            trace!("Creating GL context for display: {:?}", display);

            initialize_x11(&display, shared_ctx)
        }
    }

    fn initialize_x11(
        display: &gdk::Display,
        gdk_context: *mut gdk_sys::GdkGLContext,
    ) -> anyhow::Result<(gst_gl::GLDisplay, gst_gl::GLContext)> {
        info!("Initializing GL for X11 backend and display");

        unsafe {
            use glib::translate::*;
            let display: &gdkx11::X11Display =
                display.downcast_ref::<gdkx11::X11Display>().unwrap();
            let x11_display = gdkx11::ffi::gdk_x11_display_get_xdisplay(display.to_glib_none().0);

            let gst_display =
                gst_gl_x11::ffi::gst_gl_display_x11_new_with_display(x11_display as _);
            let gst_display =
                gst_gl::GLDisplay::from_glib_full(gst_display as *mut gst_gl::ffi::GstGLDisplay);
            let wrapped_context = gst_gl::GLContext::new_wrapped(
                &gst_display,
                gdk_context as _,
                gst_gl::GLPlatform::GLX,
                gst_gl::GLAPI::OPENGL,
            )
            .ok_or(anyhow::anyhow!("Failed to create wrapped GL context"))?;
            trace!("Created wrapped GL context gles2: {:?}", wrapped_context);
            wrapped_context.activate(true)?;
            wrapped_context.fill_info().expect("Failed to fill info");
            Ok((gst_display, wrapped_context))
        }
    }

    #[cfg(feature = "wayland")]
    fn initialize_waylandegl(
        display: &gdk::Display,
        _gdk_window: *mut gdk_sys::GdkWindow,
    ) -> anyhow::Result<(gst_gl::GLDisplay, gst_gl::GLContext)> {
        info!("Initializing GL for Wayland EGL backend and display");

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
            let current_gdk_gl_ctx = gdk_sys::gdk_gl_context_get_current();

            let wrapped_context = gst_gl::GLContext::new_wrapped(
                &gst_display,
                current_gdk_gl_ctx as _,
                gst_gl::GLPlatform::EGL,
                gst_gl::GLAPI::OPENGL,
            );
            let wrapped_context =
                wrapped_context.ok_or(anyhow::anyhow!("Failed to create wrapped GL context"))?;

            Ok((gst_display, wrapped_context))
        }
    }

    fn _glib_err_to_result<T, TArg>(
        callback: unsafe extern "C" fn(TArg, *mut *mut glib_sys::GError) -> T,
        arg: TArg,
    ) -> anyhow::Result<T> {
        let mut error: *mut glib_sys::GError = ptr::null_mut();
        let result = unsafe { callback(arg, &mut error) };
        if !error.is_null() {
            let error: glib::Error = unsafe { glib::translate::from_glib_full(error) };
            return Err(anyhow::anyhow!("Failed to create GL context: {:?}", error));
        }
        Ok(result)
    }
    fn gbool_to_bool(gbool: glib::ffi::gboolean) -> bool {
        gbool != glib::ffi::GFALSE
    }
    thread_local! {
        pub static GL_MANAGER: RefCell<GlManager> = RefCell::new(GlManager::new());
    }
}
use gst_video::VideoInfo;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows {
    use std::cell::RefCell;

    use gst_video::VideoInfo;

    use crate::core::types::Orientation;

   pub struct WindowsTextureProvider {
        current_texture: RefCell<Option<NativeTextureType>>,
    }
    impl WindowsTextureProvider {
        pub fn set_current_texture(&self, texture: irondash_texture::ID3D11Texture2D) {
            self.current_texture.replace(Some(Box::new(texture)));
        }

        fn on_d3d11_present(&self, rtv_raw: glib::Pointer) {
            self.set_current_texture(irondash_texture::ID3D11Texture2D(rtv_raw as *mut _));
        }
    }
    unsafe impl Send for WindowsTextureProvider {}
    unsafe impl Sync for WindowsTextureProvider {}
    pub type GFBD = irondash_texture::BoxedTextureDescriptor<irondash_texture::ID3D11Texture2D>;

    impl irondash_texture::PayloadProvider<GFBD> for WindowsTextureProvider {
        fn get_payload(&self) -> Box<NativeTextureType> {
            unimplemented!("get_payload")
        }
    }
    impl irondash_texture::PlatformTextureWithProvider<GFBD> for WindowsTextureProvider {
        fn create_texture(
            engine_handle: i64,
            payload_provider: Arc<dyn irondash_texture::PayloadProvider<GFBD>>,
        ) -> anyhow::Result<irondash_texture::PlatformTexture<GFBD>>{
            unimplemented!("create_texture")
        }
    }

    impl super::PlatformNativeTextureProviderTrait for WindowsTextureProvider {
        fn on_frame_received(
            _buffer: &gst::Buffer,
            _info: &VideoInfo,
            _orientation: Orientation,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    pub(crate) type NativeTextureType = Box<irondash_texture::ID3D11Texture2D>;

}

#[cfg(target_os = "windows")]
pub(crate) use windows::{NativeTextureType, WindowsTextureProvider as PlatformNativeTextureProvider};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TextureCacheId {
    Memory(usize),
    GL(usize),
}

struct WithFrameInfo<T> {
    frame: T,
    info: VideoInfo,
    orientation: Orientation,
}

trait PlatformNativeTextureProviderTrait: irondash_texture::PayloadProvider<GFBD> {
    fn on_frame_received(
        buffer: &gst::Buffer,
        info: &VideoInfo,
        orientation: Orientation,
    ) -> anyhow::Result<()>;
}
