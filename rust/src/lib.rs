pub mod api;
pub mod core;
pub mod dart_types;
mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
pub mod utils;

use flutter_plugin_sdk::{FlutterRustPlugin, PluginError, PluginRegistrar, Result};

struct FlutterRealtimePlayerPlugin;

impl FlutterRustPlugin for FlutterRealtimePlayerPlugin {
    fn register(&self, registrar: &mut PluginRegistrar) -> Result<()> {
        let gpu_textures = registrar.gpu()?.clone();
        let main_thread_dispatcher = registrar.main_thread_dispatcher().clone();
        core::install_plugin_capabilities(gpu_textures, main_thread_dispatcher);

        core::init_logger();
        core::session::registry::init().map_err(|_| PluginError::Unsupported)?;

        #[cfg(target_os = "android")]
        android_gst_plugins::register_all();

        std::thread::spawn(core::session::registry::stream_alive_tester_task);
        Ok(())
    }
}

pub fn register(registrar: &mut PluginRegistrar) -> Result<()> {
    FlutterRealtimePlayerPlugin.register(registrar)
}

#[cfg(target_os = "android")]
#[doc(hidden)]
pub mod android_gst_plugins {
    extern "C" {
        fn gst_plugin_app_register();
        fn gst_plugin_coreelements_register();
        fn gst_plugin_typefindfunctions_register();
        fn gst_plugin_playback_register();
        fn gst_plugin_soup_register();
        fn gst_plugin_libav_register();
        fn gst_plugin_androidmedia_register();
        fn gst_plugin_videoconvertscale_register();
        fn gst_plugin_audioconvert_register();
        fn gst_plugin_audioresample_register();
        fn gst_plugin_volume_register();
        fn gst_plugin_audiomixer_register();
        fn gst_plugin_autodetect_register();
        fn gst_plugin_opensles_register();
        fn gst_plugin_rtsp_register();
        fn gst_plugin_rtp_register();
        fn gst_plugin_rtpmanager_register();
        fn gst_plugin_udp_register();
        fn gst_plugin_tcp_register();
        fn gst_plugin_sdpelem_register();
        fn gst_plugin_videoparsersbad_register();
        #[cfg(feature = "android-vulkan")]
        fn gst_plugin_vulkan_register();
    }

    // These plugins also work in a standalone executable without a Java VM.
    const PROBE_PLUGINS: &[(&str, unsafe extern "C" fn())] = &[
        ("coreelements", gst_plugin_coreelements_register),
        ("videoparsersbad", gst_plugin_videoparsersbad_register),
        #[cfg(feature = "android-vulkan")]
        ("vulkan", gst_plugin_vulkan_register),
    ];

    /// Register only the plugins needed for the standalone Vulkan decoder probe.
    pub fn register_for_probe() {
        register_plugins(PROBE_PLUGINS);
    }

    pub(crate) fn register_all() {
        register_for_probe();
        let plugins: &[(&str, unsafe extern "C" fn())] = &[
            ("app", gst_plugin_app_register),
            ("typefindfunctions", gst_plugin_typefindfunctions_register),
            ("playback", gst_plugin_playback_register),
            ("soup", gst_plugin_soup_register),
            ("libav", gst_plugin_libav_register),
            ("androidmedia", gst_plugin_androidmedia_register),
            ("videoconvertscale", gst_plugin_videoconvertscale_register),
            ("audioconvert", gst_plugin_audioconvert_register),
            ("audioresample", gst_plugin_audioresample_register),
            ("volume", gst_plugin_volume_register),
            ("audiomixer", gst_plugin_audiomixer_register),
            ("autodetect", gst_plugin_autodetect_register),
            ("opensles", gst_plugin_opensles_register),
            ("rtsp", gst_plugin_rtsp_register),
            ("rtp", gst_plugin_rtp_register),
            ("rtpmanager", gst_plugin_rtpmanager_register),
            ("udp", gst_plugin_udp_register),
            ("tcp", gst_plugin_tcp_register),
            ("sdpelem", gst_plugin_sdpelem_register),
        ];

        register_plugins(plugins);
    }

    fn register_plugins(plugins: &[(&str, unsafe extern "C" fn())]) {
        for (name, register_fn) in plugins {
            // GST_PLUGIN_STATIC_DECLARE declares a void function, not gboolean.
            unsafe { register_fn() };
            if gst::Registry::get().find_plugin(name).is_none() {
                log::error!("Failed to register GStreamer plugin: {}", name);
            }
        }
    }
}
