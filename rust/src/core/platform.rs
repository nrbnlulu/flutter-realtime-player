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
    use glib::translate::Uninitialized;
    use glib_sys::gpointer;
    use gst::{
        ffi::GstMapInfo,
        glib::{object::ObjectExt, translate::FromGlibPtrFull},
    };

    use gst_app::AppSink;
    use gst_video::ffi::{
        GstVideoColorRange, GstVideoColorimetry, GstVideoInfo, GstVideoInterlaceMode,
    };
    use irondash_texture::TextureDescriptor;
    use log::{error, trace};
    use std::{
        mem::{self, MaybeUninit},
        ptr::{addr_of, null_mut},
        sync::{Arc, Mutex, RwLock},
    };
    use windows::{core::Interface, Win32::Graphics::Dxgi};
    use windows::Win32::{
        Foundation::{HANDLE, HMODULE},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::*,
            Dxgi::{
                Common::{DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
                *,
            },
        },
    };

    use crate::core::{fluttersink::utils::LogErr, types::VideoDimensions};
    pub mod sys {
        use std::os::windows::raw::HANDLE;
        pub type GstD3dDevice = glib_sys::gpointer;
        pub type GstD3dContext = glib_sys::gpointer;
        pub type GstD3dMemory = glib_sys::gpointer;
        pub type GstMemory = glib_sys::gpointer;

        #[link(name = "gstd3d11-1.0")]
        extern "C" {
            pub fn gst_d3d11_device_new_wrapped(device: HANDLE) -> GstD3dDevice;
            pub fn gst_d3d11_context_new(device: GstD3dDevice) -> GstD3dContext;
            pub fn gst_d3d11_memory_get_subresource_index(mem: *mut GstD3dMemory) -> u32;
            pub fn gst_is_d3d11_memory(mem: *mut GstMemory) -> glib::ffi::gboolean;
            pub fn gst_d3d11_device_new_for_adapter_luid(
                adapter_luid: u64,
                flags: u32,
            ) -> *mut GstD3dDevice;
        }
    }

    pub type NativeTextureType = irondash_texture::DxgiSharedHandle;
    pub type D3DTextureProvider = irondash_texture::alternative_api::TextureDescriptionProvider2<
        NativeTextureType,
        TextureProviderCtx,
    >;

    const TEXTURE_FORMAT: DXGI_FORMAT = DXGI_FORMAT_B8G8R8A8_UNORM;

    pub fn initialize_pipeline(
        app_sink: &gst_app::AppSink,
        flutter_device: ID3D11Device,
        dimensions: &VideoDimensions,
    ) -> anyhow::Result<()> {
        let video_info = gst_video::VideoInfo::builder(
            gst_video::VideoFormat::Bgra,
            dimensions.width,
            dimensions.height,
        )
        .build()
        .unwrap();
        let dxgi_device = flutter_device.cast::<IDXGIDevice>()?;

        app_sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(new_sample_cb)
                .build(),
        );
        unsafe {
            let decoding_device_gst = sys::gst_d3d11_device_new_for_adapter_luid(0, 0);
            fn create_texture_for_flutter(
                flutter_device: &ID3D11Device,
                dimensions: &VideoDimensions,
            ) -> anyhow::Result<ID3D11Texture2D> {
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
                    BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                    CPUAccessFlags: 0,
                    MiscFlags: (D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX.0
                        | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0)
                        as u32,
                };
                let mut texture: Option<ID3D11Texture2D> = None;
                unsafe { flutter_device.CreateTexture2D(&texture_desc, None, Some(&mut texture)) }
                    .unwrap();
                let texture = texture.ok_or_else(|| anyhow::anyhow!("Failed to create texture"))?;
                // Gets keyed mutex interface and acquire sync at render device side.
                // This keyed mutex will be temporarily released
                // when rendering to shared texture by GStreamer D3D11 device,
                // then re-acquired for render engine device
                let keyed_mutex = texture.cast::<IDXGIKeyedMutex>()?;  
                const INFINITE: u32 = 0xffffffff;
;
                unsafe { keyed_mutex.AcquireSync(0, INFINITE) }?;
                let dxgi_resource: IDXGIResource1 = texture.cast()?;
                let mut shared_handle = None;

                // Create shared NT handle so that GStreamer device can access
                unsafe {
                    dxgi_resource.CreateSharedHandle(
                        None,
                        DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE,
                        None,
                        &mut shared_handle,
                    )?;
                }

                let shared_handle = shared_handle.ok_or(anyhow::anyhow!("Failed to create shared handle"))?;

                // Open shared texture at GStreamer device side
                let gst_device = sys::gst_d3d11_device_new_wrapped(flutter_device.as_raw() as _);
                let gst_device: ID3D11Device1 = gst_device.cast()?;

                let mut gst_texture: Option<ID3D11Texture2D> = None;
                unsafe {
                    gst_device.OpenSharedResource1(shared_handle, &mut gst_texture)?;
                }

                // Close NT handle as it's no longer needed
                unsafe {
                    CloseHandle(shared_handle);
                }

                let gst_texture = gst_texture.ok_or(anyhow::anyhow!("Failed to open shared texture"))?;

                // Wrap shared texture with GstD3D11Memory
                let mem = unsafe {
                    sys::gst_d3d11_allocator_alloc_wrapped(
                        std::ptr::null_mut(),
                        gst_device.as_raw() as _,
                        gst_texture.as_raw() as _,
                        0, // CPU accessible memory size is unknown, pass zero
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };

                if mem.is_null() {
                    return Err(anyhow::anyhow!("Failed to wrap shared texture with GstD3D11Memory"));
                }

                // Create a new GstBuffer and append the memory
                let shared_buffer = unsafe { gst::Buffer::from_glib_full(gst::ffi::gst_buffer_new()) };
                unsafe {
                    gst::ffi::gst_buffer_append_memory(shared_buffer.as_mut_ptr(), mem);
                }
            };
        }

        Ok(())
    }

    pub fn new_sample_cb(app_sink: &gst_app::AppSink) -> Result<gst::FlowSuccess, gst::FlowError> {
        todo!()
    }

    pub fn create_d3d11_device(
        engine_handle: i64,
        dimensions: &VideoDimensions,
    ) -> anyhow::Result<ID3D11Device> {
        let mut d3d_device = None;
        unsafe {
            D3D11CreateDevice(
                None, // TODO: adapter needed?
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                None,
            )?;
        };
        let d3d_device = d3d_device.ok_or(anyhow::anyhow!("Failed to create d3d11 device"))?;
        let mt_device: ID3D11Multithread = d3d_device.cast()?;

        unsafe {
            let _ = mt_device.SetMultithreadProtected(true);
        };

        Ok(d3d_device)
    }

    pub fn create_gst_d3d_ctx(device: &ID3D11Device) -> sys::GstD3dContext {
        trace!("created gst device");
        unsafe {
            let gst_d3d_device_wrapper = sys::gst_d3d11_device_new_wrapped(device.as_raw() as _);
            trace!("created gst d3d device wrapper");
            let gst_d3d_ctx_raw = sys::gst_d3d11_context_new(gst_d3d_device_wrapper);

            trace!("created gst d3d ctx raw");
            gst_d3d_ctx_raw
        }
    }

    pub fn get_texture_from_sample(
        sample: gst::Sample,
        device: &ID3D11Device,
    ) -> Result<(HANDLE, gst_video::VideoInfo), gst::FlowError> {
        if let Some(buffer) = sample.buffer() {
            if let Some(caps) = sample.caps() {
                unsafe {
                    let video_info = gst_video::VideoInfo::from_caps(caps).map_err(|_| {
                        error!("Failed to get video info from caps: {:?}", caps);
                        gst::FlowError::Error
                    })?;
                    trace!("video_info: {:?}", video_info);

                    let mem = buffer.peek_memory(0);
                    trace!("peek_memory: {:?}", mem);
                    let mem_raw = mem.as_mut_ptr();
                    trace!("peek_memory raw: {:?}", mem_raw);

                    if sys::gst_is_d3d11_memory(mem_raw as _) != 0 {
                        // TODO: decoder output texture may be texture array. Application should check
                        // subresource index
                        let _subresource_index =
                            sys::gst_d3d11_memory_get_subresource_index(mem_raw as _);
                        trace!("subresource index: {:?}", _subresource_index);
                        // Use GST_MAP_D3D11 flag to indicate that direct Direct3D11 resource
                        // * is required instead of system memory access.
                        // *
                        // * CAUTION: Application must not try to write/modify texture rendered by
                        // * video decoder since it's likely a reference frame. If it's modified by
                        // * application, then the other decoded frames would be broken.
                        // * Only read access is allowed in this case
                        let mut info: MaybeUninit<GstMapInfo> = mem::MaybeUninit::uninit();
                        if gst::ffi::gst_memory_map(mem_raw as _, info.as_mut_ptr(), MAP_FLAGS) != 0
                        {
                            let data = info.assume_init().data;
                            trace!("texture raw ptr: {:?}", data);
                            let texture = data as *mut ID3D11Texture2D;
                            let texture_as_resource = texture.cast::<IDXGIResource>();
                            trace!("texture as resource: {:?}", texture_as_resource);
                            let handle = (*texture_as_resource).GetSharedHandle().unwrap();
                            trace!("texture handle: {:?}", handle);

                            if handle.is_invalid() {
                                error!("Invalid handle: {:?}", handle);
                                return Err(gst::FlowError::Error);
                            }
                            device
                                .GetImmediateContext()
                                .map(|ctx| {
                                    ctx.Flush();
                                })
                                .map_err(|_| {
                                    error!("Failed to flush context");
                                    gst::FlowError::Error
                                })?;
                            return Ok((handle, video_info));
                        }
                    };
                }
            }
        }
        error!("Failed to get texture from sample: {:?}", sample);
        Err(gst::FlowError::Error)
    }
    // `gst::ffi::GST_MAP_FLAG_LAST << 1` is because it is defined in a macro so I can't use ffi.
    const MAP_FLAGS: gst::ffi::GstMapFlags =
        gst::ffi::GST_MAP_READ | (gst::ffi::GST_MAP_FLAG_LAST << 1);

    pub trait TextureDescriptionProvider2Ext<T: Clone> {
        fn new(engine_handle: i64, dimensions: VideoDimensions) -> anyhow::Result<Arc<Self>>;
    }

    pub struct TextureProviderCtx {
        pub device: ID3D11Device,
        engine_handle: i64,
        dimensions: VideoDimensions,
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

            let device = create_d3d11_device(engine_handle, &dimensions)?;

            let render_target_view_desc = D3D11_RENDER_TARGET_VIEW_DESC {
                Format: TEXTURE_FORMAT,
                ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
                ..unsafe { mem::zeroed() }
            };

            let width = dimensions.width;
            let height = dimensions.height;

            trace!("casted gst d3d ctx raw to ID3D11DeviceContext");

            let out = Arc::new(Self {
                current_texture: Arc::new(Mutex::new(None)),
                context: TextureProviderCtx {
                    device,
                    engine_handle,
                    dimensions,
                },
            });

            Ok(out)
        }
    }

    pub type NativeRegisteredTexture =
        irondash_texture::alternative_api::RegisteredTexture<NativeTextureType, TextureProviderCtx>;
}

#[cfg(target_os = "windows")]
pub(crate) use windows::{
    create_gst_d3d_ctx, get_texture_from_sample, D3DTextureProvider as NativeTextureProvider,
    NativeRegisteredTexture, TextureDescriptionProvider2Ext,
};
