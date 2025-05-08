



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
        use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
        pub type GstD3dDevice = glib_sys::gpointer;
        pub type D3D11DeviceRaw = glib_sys::gpointer;
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
            pub fn gst_d3d11_device_get_device_handle(device: GstD3dDevice) -> D3D11DeviceRaw;

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

    pub struct SampleWrapper {
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

                let adapter = dxgi_device.GetAdapter()?;
                let adapter_luid = adapter.GetDesc()?.AdapterLuid;
                trace!("adapter_luid: {:?}", adapter_luid);
                let adapter_luid = adapter_luid.LowPart as u64;
                let wrapped_gst_device =
                    sys::gst_d3d11_device_new_for_adapter_luid(adapter_luid, 0);
                trace!(
                    "decoding_device_gst: {:?}",
                    wrapped_gst_device as *const sys::GstD3dDevice
                );
                // Open shared texture at GStreamer device side
                let gst_device_raw =
                    sys::gst_d3d11_device_get_device_handle(wrapped_gst_device as _);
                let gst_device = ID3D11Device::from_raw_borrowed(&gst_device_raw).unwrap();
                trace!("creation flags {:?}", gst_device.GetCreationFlags());
                let gst_device1 = gst_device.cast::<ID3D11Device1>()?;
                trace!("gst_device1: {:?}", gst_device1);
                let dxgi_resource: IDXGIResource1 = flutter_texture.cast()?;

                // Create shared NT handle so that GStreamer device can access
                let texture_dxgi_shared_handle = dxgi_resource.CreateSharedHandle(
                    None,
                    (DXGI_SHARED_RESOURCE_READ | DXGI_SHARED_RESOURCE_WRITE).0,
                    None,
                )?;
                trace!(
                    "texture_dxgi_shared_handle: {:?}",
                    texture_dxgi_shared_handle
                );
                trace!(
                    "creation flags gst_device1 {:?}",
                    gst_device1.GetCreationFlags()
                );

                // Open shared texture at GStreamer device side
                let gst_texture: ID3D11Texture2D =
                    gst_device1.OpenSharedResource1(texture_dxgi_shared_handle)?;

                trace!("gst_texture: {:?}", gst_texture);

                // Can close NT handle now
                // Close the shared NT handle as it is no longer needed
                CloseHandle(texture_dxgi_shared_handle)?;
                trace!("Closed shared NT handle");

                // Wrap the shared texture with GstD3D11Memory to enable texture conversion
                // using the GStreamer converter API
                let mem = sys::gst_d3d11_allocator_alloc_wrapped(
                    null_mut(),
                    wrapped_gst_device as _,
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
                    gst_d3d_device: wrapped_gst_device as _,
                    gst_d3d_converter: Mutex::new(None),
                    last_sample: Mutex::new(None),
                });

                return Ok(ret);
            };
        }

        pub fn set_callbacks<T>(self: Arc<Self>, on_new_sample: T)
        where
            T: Fn() + Send + 'static,
        {
            let app_sink = self.app_sink.clone();

            app_sink.set_callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |app_sink| {
                        Self::new_sample_cb(app_sink, &self, &on_new_sample)
                    })
                    .build(),
            );
        }

        pub fn new_sample_cb<T>(
            app_sink: &gst_app::AppSink,
            self_: &GstDecodingEngine,
            on_new_sample: &T,
        ) -> Result<gst::FlowSuccess, gst::FlowError>
        where
            T:  Fn() + Send + 'static,
        {
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
                    self_.update_texture();
                }
            }
            on_new_sample();
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
            self.pipeline.set_state(gst::State::Playing).log_err();
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

        pub fn create_texture_descriptor(
            &self,
        ) -> irondash_texture::TextureDescriptor<NativeTextureType> {
            let dxgi_resource: IDXGIResource1 = self.flutter_texture.cast().unwrap();
            // TODO: make sure we close the resource?
            let handle = irondash_texture::DxgiSharedHandle(unsafe {
                dxgi_resource.CreateSharedHandle(
                    None,
                    (DXGI_SHARED_RESOURCE_READ ).0,
                    None,
                ).unwrap().0
            } as _);
            trace!("handle: {:?}", handle);
            let width = self.video_info.width() as _;
            let height = self.video_info.height() as _;
            irondash_texture::TextureDescriptor {
                handle,
                width,
                height,
                visible_width: width,
                visible_height: height,
                pixel_format: irondash_texture::PixelFormat::BGRA,
            }
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

    // `gst::ffi::GST_MAP_FLAG_LAST << 1` is because it is defined in a macro so I can't use ffi.
    const MAP_FLAGS: gst::ffi::GstMapFlags =
        gst::ffi::GST_MAP_READ | (gst::ffi::GST_MAP_FLAG_LAST << 1);

    pub trait TextureDescriptionProvider2Ext<T: Clone> {
        fn new(decoding_engine: Arc<GstDecodingEngine>) -> anyhow::Result<Arc<Self>>;
    }

    pub struct TextureProviderCtx {
        pub decoding_engine: Arc<GstDecodingEngine>,
    }

    impl TextureDescriptionProvider2Ext<NativeTextureType>
        for irondash_texture::alternative_api::TextureDescriptionProvider2<
            NativeTextureType,
            TextureProviderCtx,
        >
    {
        // Implement the methods here
        fn new(decoding_engine: Arc<GstDecodingEngine>) -> anyhow::Result<Arc<Self>> {
            let out = Arc::new(Self {
                current_texture: Arc::new(Mutex::new(None)),
                context: TextureProviderCtx { decoding_engine },
            });

            Ok(out)
        }
    }

    pub type NativeRegisteredTexture =
        irondash_texture::alternative_api::RegisteredTexture<NativeTextureType, TextureProviderCtx>;
}

#[cfg(target_os = "windows")]
pub(crate) use windows::{
    D3DTextureProvider as NativeTextureProvider, GstDecodingEngine, NativeRegisteredTexture,
    TextureDescriptionProvider2Ext,
};
