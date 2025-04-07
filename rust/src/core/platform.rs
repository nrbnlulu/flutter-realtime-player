
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
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows {
    use glib_sys::gpointer;
    use gst::glib::object::ObjectExt;
    
    use irondash_texture::TextureDescriptor;
    use log::trace;
    use std::{
        mem,
        sync::{Arc, Mutex, RwLock},
    };
    use windows::{
        core::*,
        Win32::{
            Foundation::{HANDLE, HMODULE},
            Graphics::{
                Direct3D::D3D_DRIVER_TYPE_HARDWARE,
                Direct3D11::*,
                Dxgi::{
                    Common::{DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
                    *,
                },
            },
        },
    };

    use crate::core::{fluttersink::utils::LogErr, types::VideoDimensions};
    pub mod sys {
        use std::{ffi::c_void, os::windows::raw::HANDLE};

        type GstD3dDevice = glib_sys::gpointer;
        type GstD3dContext = glib_sys::gpointer;

        #[link(name = "gstd3d11-1.0")]
        extern "C" {
            pub fn gst_d3d11_device_new_wrapped(device: HANDLE) -> GstD3dDevice;
            pub fn gst_d3d11_context_new(device: GstD3dDevice) -> GstD3dContext;
        }
    }

    pub type NativeTextureType = irondash_texture::DxgiSharedHandle;
    pub type D3DTextureProvider = irondash_texture::alternative_api::TextureDescriptionProvider2<
        NativeTextureType,
        TextureProviderCtx,
    >;
    pub struct D3D11Texture {
        texture: ID3D11Texture2D,
        keyed_mutex: IDXGIKeyedMutex,
        handle: HANDLE,
    }

    const TEXTURE_FORMAT: DXGI_FORMAT = DXGI_FORMAT_B8G8R8A8_UNORM;

    pub fn create_d3d11_texture(
        device: &ID3D11Device,
        engine_handle: i64,
        dimensions: &VideoDimensions,
    ) -> anyhow::Result<D3D11Texture> {
        trace!("creating texture desc");
        let texture_desc = D3D11_TEXTURE2D_DESC {
            Width: dimensions.width,
            Height: dimensions.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: TEXTURE_FORMAT,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            // will be used to draw on  + will be used in flutter shader
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            CPUAccessFlags: 0,
            // enable use with other devices
            MiscFlags: D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0 as u32
                | D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX.0 as u32,
        };
        trace!("creating texture");

        let mut texture = None;
        unsafe {
            device.CreateTexture2D(&texture_desc, None, Some(&mut texture))?;
        };
        let texture = texture.ok_or(anyhow::anyhow!("Failed to create d3d11 texture"))?;
        trace!("texture created {:?}", texture);
        let texture_as_resource: IDXGIResource = texture.cast()?;
        let dxgi_keyed_mutex: IDXGIKeyedMutex = texture_as_resource.cast()?;

        let handle = unsafe { texture_as_resource.GetSharedHandle()? };
        if handle.is_invalid() {
            return Err(anyhow::anyhow!("Invalid handle"));
        }

        trace!("Created texture with handle: {:?}", handle);
        Ok(D3D11Texture {
            texture,
            keyed_mutex: dxgi_keyed_mutex,
            handle,
        })
    }

    pub trait TextureDescriptionProvider2Ext<T: Clone> {
        fn new(engine_handle: i64, dimensions: VideoDimensions) -> anyhow::Result<Arc<Self>>;
        fn on_begin_draw(&self, _sink: &gst::Element) -> anyhow::Result<()>;
    }

    pub struct TextureProviderCtx {
        texture: RwLock<Option<D3D11Texture>>,
        engine_handle: i64,
        dimensions: VideoDimensions,
    }

    fn create_d3d_device_and_ctx(
        flags: D3D11_CREATE_DEVICE_FLAG,
        multithread: bool,
    ) -> anyhow::Result<(ID3D11Device, ID3D11DeviceContext)> {
        let mut d3d_device = None;
        unsafe {
            D3D11CreateDevice(
                None, // TODO: take the adapter(GPU) from flutter.
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                flags,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                None,
            )?;
        };
        let d3d_device = d3d_device.ok_or(anyhow::anyhow!("Failed to create d3d11 device"))?;
        let immediate_ctx = unsafe { d3d_device.GetImmediateContext() }?;
        if multithread {
            let mt_device: ID3D11Multithread = d3d_device.cast()?;
            let res = unsafe { mt_device.SetMultithreadProtected(true) };
            if !res.as_bool() {
                return Err(anyhow::anyhow!("Failed to set multithread protected"));
            }
        }

        Ok((d3d_device, immediate_ctx))
    }

    impl TextureDescriptionProvider2Ext<NativeTextureType>
        for irondash_texture::alternative_api::TextureDescriptionProvider2<
            NativeTextureType,
            TextureProviderCtx,
        >
    {
        // Implement the methods here
        fn new(engine_handle: i64, dimensions: VideoDimensions) -> anyhow::Result<Arc<Self>> {
            trace!("Creating new D3D11 texture provider");
            let (device, ctx) = create_d3d_device_and_ctx(D3D11_CREATE_DEVICE_BGRA_SUPPORT, true)?;
            let (gst_device, gst_d3d_ctx) =
                create_d3d_device_and_ctx(D3D11_CREATE_DEVICE_VIDEO_SUPPORT, false)?;
            let gst_device = gst_device.cast::<ID3D11Device1>()?;
            let texture_wrapper = create_d3d11_texture(&device, engine_handle, &dimensions)?;

            let gst_texture: ID3D11Texture2D =
                unsafe { gst_device.OpenSharedResource1(texture_wrapper.handle)? };
            let render_target_view_desc = D3D11_RENDER_TARGET_VIEW_DESC {
                Format: TEXTURE_FORMAT,
                ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                ..unsafe { mem::zeroed() }
            };
            let texture_render_target = None;
            unsafe {
                gst_device.CreateRenderTargetView(
                    &gst_texture,
                    Some(&render_target_view_desc),
                    texture_render_target,
                )
            }?;

            let handle = texture_wrapper.handle;
            let width = dimensions.width;
            let height = dimensions.height;
            let gst_d3d_device_wrapper = unsafe { sys::gst_d3d11_device_new_wrapped(device.as_raw() as _) };
            let gst_d3d_ctx_raw = unsafe { sys::gst_d3d11_context_new(gst_d3d_device_wrapper) };
            let gst_d3d_ctx = unsafe { gst_d3d_ctx_raw.cast::<ID3D11DeviceContext>().as_ref().ok_or(
                anyhow::anyhow!("Failed to cast gst_d3d_ctx_raw to ID3D11DeviceContext")
           )? };



            let out = Arc::new(Self {
                current_texture: Arc::new(Mutex::new(None)),
                context: TextureProviderCtx {
                    texture: RwLock::new(Some(texture_wrapper)),
                    engine_handle,
                    dimensions,
                },
            });

            out.set_current_texture(TextureDescriptor::new(
                irondash_texture::DxgiSharedHandle(handle.0 as *mut _),
                width as _,
                height as _,
                width as _,
                height as _,
                irondash_texture::PixelFormat::BGRA,
            ))?;

            Ok(out)
        }

        fn on_begin_draw(&self, sink: &gst::Element) -> anyhow::Result<()> {
            let mut handle = None;
            // don't let anyone else access the texture while we drawing

            self.current_texture.lock().log_err();
            trace!("on_begin_draw in thread {:?}", std::thread::current().id());

            self.context
                .texture
                .try_read()
                .map(|t| handle = t.as_ref().map(|t| t.handle))
                .map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read texture: {:?} on thread {:?}",
                        e,
                        std::thread::current().id()
                    )
                })?;
            if let Some(handle) = handle {
                if sink.emit_by_name::<bool>(
                    "draw",
                    &[
                        &(handle.0 as gpointer),
                        &(D3D11_RESOURCE_MISC_SHARED.0 as u32),
                        &0u64,
                        &0u64,
                    ],
                ) {
                    return Ok(());
                }
                return Err(anyhow::anyhow!("Failed to emit draw signal"));
            }
            return Err(anyhow::anyhow!("No HANDLE available"));
        }
    }

    pub type NativeRegisteredTexture =
        irondash_texture::alternative_api::RegisteredTexture<NativeTextureType, TextureProviderCtx>;
}

#[cfg(target_os = "windows")]
pub(crate) use windows::{
    D3DTextureProvider as NativeTextureProvider, NativeRegisteredTexture,
    TextureDescriptionProvider2Ext,
};
