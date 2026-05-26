pub mod api;
pub mod core;
pub mod dart_types;
mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
pub mod utils;

#[cfg(target_os = "android")]
pub(crate) mod android_gst_plugins {
    extern "C" {
        pub fn gst_plugin_coreelements_register();
        pub fn gst_plugin_typefindfunctions_register();
        pub fn gst_plugin_playback_register();
        pub fn gst_plugin_soup_register();
        pub fn gst_plugin_matroska_register();
        pub fn gst_plugin_isomp4_register();
        pub fn gst_plugin_libav_register();
        pub fn gst_plugin_vpx_register();
        pub fn gst_plugin_androidmedia_register();
        pub fn gst_plugin_videoconvertscale_register();
        pub fn gst_plugin_audioconvert_register();
        pub fn gst_plugin_audioresample_register();
        pub fn gst_plugin_volume_register();
        pub fn gst_plugin_audiomixer_register();
        pub fn gst_plugin_autodetect_register();
        pub fn gst_plugin_audioparsers_register();
        pub fn gst_plugin_videoparsersbad_register();
        pub fn gst_plugin_opensles_register();
        pub fn gst_plugin_rtsp_register();
        pub fn gst_plugin_rtp_register();
        pub fn gst_plugin_rtpmanager_register();
        pub fn gst_plugin_udp_register();
        pub fn gst_plugin_tcp_register();
        pub fn gst_plugin_sdpelem_register();
    }

    pub unsafe fn register_all() {
        gst_plugin_coreelements_register();
        gst_plugin_typefindfunctions_register();
        gst_plugin_playback_register();
        gst_plugin_soup_register();
        gst_plugin_matroska_register();
        gst_plugin_isomp4_register();
        gst_plugin_libav_register();
        gst_plugin_vpx_register();
        gst_plugin_androidmedia_register();
        gst_plugin_videoconvertscale_register();
        gst_plugin_audioconvert_register();
        gst_plugin_audioresample_register();
        gst_plugin_volume_register();
        gst_plugin_audiomixer_register();
        gst_plugin_autodetect_register();
        gst_plugin_audioparsers_register();
        gst_plugin_videoparsersbad_register();
        gst_plugin_opensles_register();
        gst_plugin_rtsp_register();
        gst_plugin_rtp_register();
        gst_plugin_rtpmanager_register();
        gst_plugin_udp_register();
        gst_plugin_tcp_register();
        gst_plugin_sdpelem_register();
    }
}
