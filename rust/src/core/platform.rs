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
    use gst::{ffi::GstMapInfo, glib::translate::ToGlibPtr, prelude::ElementExt};
    use gst_video::VideoInfo;
    use log::{error, info, trace};
    use std::{
        ffi::{c_char, CString},
        mem::{self, MaybeUninit},
        ptr::null_mut,
        sync::{Arc, Mutex, MutexGuard},
    };
    use windows::Win32::Graphics::{Direct3D11::*, Dxgi::Common::*, Dxgi::*};
    use windows::{core::Interface, Win32::Foundation::CloseHandle};

    use windows::Win32::{
        Foundation::{HANDLE, HMODULE},
        Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE,
    };

    use crate::core::{fluttersink::utils::LogErr, types::VideoDimensions};
    pub mod sys {
        use std::os::windows::raw::HANDLE;

        use gst::glib::ffi::GDestroyNotify;
        use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11Texture2D};
        pub type GstD3dDevice = glib_sys::gpointer;
        pub type GstD3dContext = glib_sys::gpointer;
        pub type GstD3dMemory = glib_sys::gpointer;
        pub type GstMemory = glib_sys::gpointer;
        pub type GstD3D11Allocator = glib_sys::gpointer;
        pub type GstD3D11Device = glib_sys::gpointer;
        pub type GstD3D11Converter = glib_sys::gpointer;

        #[allow(non_camel_case_types)]
        #[repr(C)]
        pub enum GstD3D11ConverterBackend {
            GST_D3D11_CONVERTER_BACKEND_SHADER = 1 << 0,
            GST_D3D11_CONVERTER_BACKEND_VIDEO_PROCESSOR = 1 << 1,
        }

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
            pub fn gst_d3d11_device_get_device_handle(
                device: *mut GstD3dDevice,
            ) -> *mut ID3D11Device;

            pub fn gst_d3d11_allocator_alloc_wrapped(
                allocator: *mut GstD3D11Allocator,
                device: *mut GstD3D11Device,
                texture: *mut ID3D11Texture2D,
                size: usize,
                user_data: *mut std::ffi::c_void,
                notify: GDestroyNotify,
            ) -> *mut GstMemory;

            pub fn gst_d3d11_converter_new(
                device: *mut GstD3D11Device,
                in_info: *const gst_video::ffi::GstVideoInfo,
                out_info: *const gst_video::ffi::GstVideoInfo,
                config: *mut gst::ffi::GstStructure,
            ) -> *mut GstD3D11Converter;

            pub fn gst_d3d11_converter_backend_get_type() -> glib::ffi::GType;

            pub fn gst_d3d11_converter_convert_buffer(
                converter: *mut GstD3D11Converter,
                in_buf: *mut gst::ffi::GstBuffer,
                out_buf: *mut gst::ffi::GstBuffer,
            ) -> glib::ffi::gboolean;
        }
    }

    pub type NativeTextureType = irondash_texture::DxgiSharedHandle;
    pub type D3DTextureProvider = irondash_texture::alternative_api::TextureDescriptionProvider2<
        NativeTextureType,
        TextureProviderCtx,
    >;
    pub static GST_D3D11_DEVICE_HANDLE_CONTEXT_TYPE: &'static str = "gst.d3d11.device.handle";

    const TEXTURE_FORMAT: DXGI_FORMAT = DXGI_FORMAT_B8G8R8A8_UNORM;

    struct SampleWrapper {
        pub sample: gst::Sample,
        pub video_info: gst_video::VideoInfo,
        pub caps: gst::Caps,
    }

    pub struct GstDecodingEngine {
        main_context: glib::MainContext,
        main_loop: glib::MainLoop,
        pipeline: gst::Pipeline,
        pub app_sink: gst_app::AppSink,
        pub video_info: gst_video::VideoInfo,
        pub flutter_texture: ID3D11Texture2D,
        shared_buffer: gst::Buffer,
        gst_d3d_device: sys::GstD3dDevice,
        gst_d3d_converter: Mutex<Option<sys::GstD3D11Converter>>,
        keyed_mutex: IDXGIKeyedMutex,
        last_sample: Mutex<Option<SampleWrapper>>,
    }
    unsafe impl Send for GstDecodingEngine {}
    unsafe impl Sync for GstDecodingEngine {}

    lazy_static::lazy_static! {
            static ref GST_D3D11_CONVERTER_OPT_BACKEND: CString = CString::new("GstD3D11Converter.backend").unwrap();
    }

    impl GstDecodingEngine {
        pub fn new(
            pipeline: gst::Pipeline,
            app_sink: &gst_app::AppSink,
            flutter_device: ID3D11Device,
            dimensions: &VideoDimensions,
        ) -> anyhow::Result<Arc<GstDecodingEngine>> {
            let main_context = glib::MainContext::new();
            let main_loop = glib::MainLoop::new(Some(&main_context), false);

            let video_info = gst_video::VideoInfo::builder(
                gst_video::VideoFormat::Bgra,
                dimensions.width,
                dimensions.height,
            )
            .build()
            .unwrap();
            let dxgi_device = (&flutter_device).cast::<IDXGIDevice>()?;

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
                    | D3D11_RESOURCE_MISC_SHARED_NTHANDLE.0) as u32,
            };
            let mut flutter_texture: Option<ID3D11Texture2D> = None;
            unsafe {
                flutter_device
                    .CreateTexture2D(&texture_desc, None, Some(&mut flutter_texture))
                    .unwrap();
                let flutter_texture =
                    flutter_texture.ok_or_else(|| anyhow::anyhow!("Failed to create texture"))?;
                // Gets keyed mutex interface and acquire sync at render device side.
                // This keyed mutex will be temporarily released
                // when rendering to shared texture by GStreamer D3D11 device,
                // then re-acquired for render engine device
                let keyed_mutex = flutter_texture.cast::<IDXGIKeyedMutex>()?;
                const INFINITE: u32 = 0xffffffff;
                keyed_mutex.AcquireSync(0, INFINITE)?;
                let dxgi_resource: IDXGIResource1 = flutter_texture.cast()?;

                // Create shared NT handle so that GStreamer device can access
                let texture_dxgi_shared_handle = dxgi_resource.CreateSharedHandle(
                    None,
                    DXGI_SHARED_RESOURCE_READ.0 | DXGI_SHARED_RESOURCE_WRITE.0,
                    None,
                )?;
                let decoding_device_gst = sys::gst_d3d11_device_new_for_adapter_luid(0, 0);

                // Open shared texture at GStreamer device side
                let gst_device = sys::gst_d3d11_device_get_device_handle(decoding_device_gst as _);

                let mut gst_device1: MaybeUninit<ID3D11Device1> = MaybeUninit::uninit();
                let res = (*gst_device).query(&ID3D11Device1::IID, gst_device1.as_mut_ptr() as _);
                let gst_device1 = gst_device1.assume_init();

                if res.is_err() {
                    error!("Failed to query ID3D11Device1 interface: {:?}", res);
                    return Err(anyhow::anyhow!("Failed to query ID3D11Device1 interface"));
                }
                // Open shared texture at GStreamer device side
                let mut gst_texture: Option<ID3D11Texture2D> = None;
                gst_device1
                    .OpenSharedResource(texture_dxgi_shared_handle, &mut gst_texture as _)?;
                let gst_texture = gst_texture.ok_or_else(|| {
                    anyhow::anyhow!("Failed to open shared texture at GStreamer device side")
                })?;
                // Can close NT handle now
                // Close the shared NT handle as it is no longer needed
                CloseHandle(texture_dxgi_shared_handle)?;
                // Wrap the shared texture with GstD3D11Memory to enable texture conversion
                // using the GStreamer converter API
                let mem = sys::gst_d3d11_allocator_alloc_wrapped(
                    null_mut(),
                    gst_device as _,
                    flutter_texture.as_raw() as _,
                    0,
                    null_mut(),
                    None,
                );
                assert!(!mem.is_null(), "Failed to allocate memory for texture");
                let mem = gst::Memory::from_glib_full(mem as _);
                let mut shared_buffer_ = gst::Buffer::new();
                if let Some(shared_buffer_mut) = shared_buffer_.get_mut() {
                    shared_buffer_mut.append_memory(mem);
                } else {
                    return Err(anyhow::anyhow!("Failed to get mutable reference to buffer"));
                }

                let ret = Arc::new(GstDecodingEngine {
                    main_context,
                    main_loop,
                    pipeline,
                    keyed_mutex,
                    app_sink: app_sink.clone(),
                    video_info,
                    flutter_texture: flutter_texture,
                    shared_buffer: shared_buffer_,
                    gst_d3d_device: decoding_device_gst as _,
                    gst_d3d_converter: Mutex::new(None),
                    last_sample: Mutex::new(None),
                });
                let self_clone = ret.clone();
                app_sink.set_callbacks(
                    gst_app::AppSinkCallbacks::builder()
                        .new_sample(move |app_sink| Self::new_sample_cb(app_sink, &self_clone))
                        .build(),
                );
                return Ok(ret);
            };
        }

        pub fn new_sample_cb(
            app_sink: &gst_app::AppSink,
            self_: &GstDecodingEngine,
        ) -> Result<gst::FlowSuccess, gst::FlowError> {
            let new_sample = app_sink
                .pull_sample()
                .map_err(|_| gst::FlowError::Flushing)?;
            let new_caps = new_sample.caps_owned().ok_or(gst::FlowError::Flushing)?;
            {
                let mut last_sample = self_.last_sample.lock().unwrap();
                let mut converter_ = self_.gst_d3d_converter.lock().unwrap();
                if let Some(last_sample_ref) = (*last_sample).as_ref() {
                    if last_sample_ref.caps != new_caps {
                        // if the caps are different, we need to create a new converter
                        if let Some(converter) = *converter_ {
                            unsafe {
                                gst_sys::gst_clear_object(converter as _);
                            }
                        }
                        let in_info = VideoInfo::from_caps(&new_caps).map_err(|_| {
                            error!("Failed to get video info from caps: {:?}", new_caps);
                            gst::FlowError::Error
                        })?;
                        trace!("video_info: {:?}", in_info);
                        unsafe {
                            let gtype = sys::gst_d3d11_converter_backend_get_type();
                            let struct_name = CString::new("converter-config").unwrap();

                            let config = gst_sys::gst_structure_new(
                                struct_name.as_ptr(),
                                GST_D3D11_CONVERTER_OPT_BACKEND.as_ptr(),
                                gtype as *const c_char,
                                sys::GstD3D11ConverterBackend::GST_D3D11_CONVERTER_BACKEND_SHADER,
                                null_mut() as *const c_char,
                            );
                            let converter = sys::gst_d3d11_converter_new(
                                self_.gst_d3d_device as _,
                                in_info.to_glib_none().0,
                                self_.video_info.to_glib_none().0,
                                config as _,
                            );
                            if converter.is_null() {
                                error!("Failed to create converter");
                                return Err(gst::FlowError::Error);
                            }
                            *converter_ = Some(converter as _);
                            *last_sample = Some(SampleWrapper {
                                sample: new_sample,
                                video_info: in_info,
                                caps: new_caps,
                            });
                        }
                    } else {
                        // if the caps are the same, we can reuse the converter
                        return Ok(gst::FlowSuccess::Ok);
                    }
                }
            }

            Ok(gst::FlowSuccess::Ok)
        }

        fn bus_sync_handler(&self, bus: &gst::Bus, msg: &gst::Message) {
            if let gst::MessageView::NeedContext(msg) = msg.view() {
                if msg.context_type() == GST_D3D11_DEVICE_HANDLE_CONTEXT_TYPE {
                    unsafe {
                        use gst::prelude::*;

                        let el_raw = msg
                            .src()
                            .unwrap()
                            .clone()
                            .downcast_ref::<gst::Element>()
                            .unwrap()
                            .as_ptr();
                        // Pass our device to the message source element.
                        // Otherwise pipeline will create another device
                        let gst_d3d_ctx_ptr = sys::gst_d3d11_context_new(self.gst_d3d_device as _);
                        gst_sys::gst_element_set_context(el_raw, gst_d3d_ctx_ptr as _);
                        info!("Set context for element: {:?}", msg.src().unwrap().name());
                        gst_sys::gst_mini_object_unref(gst_d3d_ctx_ptr as _);
                    }
                }
            }
        }

        fn bus_handler(&self, bus: &gst::Bus, msg: &gst::Message) {
            match msg.view() {
                gst::MessageView::Error(err) => {
                    error!(
                        "Error from {:?}: {} ({:?})",
                        err.src().map(|s| s.to_string()),
                        err.error(),
                        err.debug()
                    );
                    self.main_loop.quit();
                }
                gst::MessageView::Eos(..) => {
                    info!("End of stream");
                    self.main_loop.quit();
                }
                _ => (),
            }
        }

        fn loop_fn(self: &Arc<Self>) {
            self.main_context
                .with_thread_default(move || {
                    let self_clone = self.clone();
                    let self_clone2 = self.clone();

                    let bus = self.pipeline.bus().unwrap();
                    let bus_clone = bus.clone();
                    let bus_clone2 = bus.clone();
                    bus_clone
                        .add_watch(move |_, msg| {
                            self_clone.bus_sync_handler(&bus, &msg);
                            gst::glib::ControlFlow::Continue
                        })
                        .log_err();

                    bus_clone2.set_sync_handler(move |b, msg| {
                        self_clone2.bus_sync_handler(&b, &msg);
                        gst::BusSyncReply::Pass
                    });
                })
                .log_err();
        }

        pub fn update_texture(&self) {
            let last_sample = self.last_sample.lock().unwrap();
            if let Some(last_sample_ref) = (*last_sample).as_ref() {
                let sample = last_sample_ref.sample.clone();
                // Release sync from render engine device,
                // so that GStreamer device can acquire sync
                if let Some(buffer) = sample.buffer_owned() {
                    unsafe {
                        self.keyed_mutex.ReleaseSync(0).log_err();
                        // Converter will take gst_d3d11_device_lock() and acquire sync
                        let converter = self.gst_d3d_converter.lock().unwrap();
                        if let Some(converter) = *converter {
                            // Convert the buffer to the shared texture
                            // using the GStreamer D3D11 converter API
                            let shared_buf: gst::glib::translate::Stash<
                                '_,
                                *mut gst_sys::GstBuffer,
                                gst::Buffer,
                            > = self.shared_buffer.to_glib_none();
                            let buf: gst::glib::translate::Stash<
                                '_,
                                *mut gst_sys::GstBuffer,
                                gst::Buffer,
                            > = buffer.to_glib_none();

                            let _ = sys::gst_d3d11_converter_convert_buffer(
                                converter as _,
                                buf.0 as _,
                                shared_buf.0 as _,
                            );

                            //  After the above function returned, GStreamer will release sync.
                            //  * Acquire sync again for render engine device */
                            self.keyed_mutex
                                .AcquireSync(0, windows::Win32::System::Threading::INFINITE)
                                .log_err();
                        } else {
                            error!("Failed to get converter");
                        }
                    }
                }
            }

            ()
        }

        pub fn run_in_thread(self: Arc<Self>) -> std::thread::JoinHandle<()> {
            let self_clone = self.clone();
            std::thread::spawn(move || {
                self_clone.loop_fn();
            })
        }

        pub fn get_texture<'a>(&'a self) -> &'a ID3D11Texture2D {
            &self.flutter_texture
        }
    }

    impl Drop for GstDecodingEngine {
        fn drop(&mut self) {
            unsafe {
                self.gst_d3d_converter
                    .lock()
                    .map(|converter| {
                        if let Some(converter) = *converter {
                            gst_sys::gst_clear_object(converter as _);
                        }
                    })
                    .log_err();
                gst_sys::gst_clear_object(self.gst_d3d_device as _);
            }
        }
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
    NativeRegisteredTexture, TextureDescriptionProvider2Ext, GstDecodingEngine
};
