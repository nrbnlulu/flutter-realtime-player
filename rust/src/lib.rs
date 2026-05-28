pub mod api;
pub mod core;
pub mod dart_types;
mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
pub mod utils;

#[cfg(target_os = "android")]
pub(crate) mod android_gst_plugins {
    extern "C" {
        pub fn gst_plugin_app_register() -> i32;
        pub fn gst_plugin_coreelements_register() -> i32;
        pub fn gst_plugin_typefindfunctions_register() -> i32;
        pub fn gst_plugin_playback_register() -> i32;
        pub fn gst_plugin_soup_register() -> i32;
        pub fn gst_plugin_libav_register() -> i32;
        pub fn gst_plugin_androidmedia_register() -> i32;
        pub fn gst_plugin_videoconvertscale_register() -> i32;
        pub fn gst_plugin_audioconvert_register() -> i32;
        pub fn gst_plugin_audioresample_register() -> i32;
        pub fn gst_plugin_volume_register() -> i32;
        pub fn gst_plugin_audiomixer_register() -> i32;
        pub fn gst_plugin_autodetect_register() -> i32;
        pub fn gst_plugin_opensles_register() -> i32;
        pub fn gst_plugin_rtsp_register() -> i32;
        pub fn gst_plugin_rtp_register() -> i32;
        pub fn gst_plugin_rtpmanager_register() -> i32;
        pub fn gst_plugin_udp_register() -> i32;
        pub fn gst_plugin_tcp_register() -> i32;
        pub fn gst_plugin_sdpelem_register() -> i32;
        pub fn gst_plugin_videoparsersbad_register() -> i32;
    }

    pub unsafe fn register_all() {
        gst_plugin_app_register();
        gst_plugin_coreelements_register();
        gst_plugin_typefindfunctions_register();
        gst_plugin_playback_register();
        gst_plugin_soup_register();
        gst_plugin_libav_register();
        gst_plugin_androidmedia_register();
        gst_plugin_videoconvertscale_register();
        gst_plugin_audioconvert_register();
        gst_plugin_audioresample_register();
        gst_plugin_volume_register();
        gst_plugin_audiomixer_register();
        gst_plugin_autodetect_register();
        gst_plugin_opensles_register();
        gst_plugin_rtsp_register();
        gst_plugin_rtp_register();
        gst_plugin_rtpmanager_register();
        gst_plugin_udp_register();
        gst_plugin_tcp_register();
        gst_plugin_sdpelem_register();
        gst_plugin_videoparsersbad_register();
    }
}
