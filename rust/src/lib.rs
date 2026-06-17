pub mod api;
pub mod core;
pub mod dart_types;
mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */
pub mod utils;

#[cfg(target_os = "android")]
pub(crate) mod android_gst_plugins {
    use crate::utils::{is_gst_result_ok, GstBool};

    extern "C" {
        fn gst_plugin_app_register() -> GstBool;
        fn gst_plugin_coreelements_register() -> GstBool;
        fn gst_plugin_typefindfunctions_register() -> GstBool;
        fn gst_plugin_playback_register() -> GstBool;
        fn gst_plugin_soup_register() -> GstBool;
        fn gst_plugin_libav_register() -> GstBool;
        fn gst_plugin_androidmedia_register() -> GstBool;
        fn gst_plugin_videoconvertscale_register() -> GstBool;
        fn gst_plugin_audioconvert_register() -> GstBool;
        fn gst_plugin_audioresample_register() -> GstBool;
        fn gst_plugin_volume_register() -> GstBool;
        fn gst_plugin_audiomixer_register() -> GstBool;
        fn gst_plugin_autodetect_register() -> GstBool;
        fn gst_plugin_opensles_register() -> GstBool;
        fn gst_plugin_rtsp_register() -> GstBool;
        fn gst_plugin_rtp_register() -> GstBool;
        fn gst_plugin_rtpmanager_register() -> GstBool;
        fn gst_plugin_udp_register() -> GstBool;
        fn gst_plugin_tcp_register() -> GstBool;
        fn gst_plugin_sdpelem_register() -> GstBool;
        fn gst_plugin_videoparsersbad_register() -> GstBool;
    }

    pub unsafe fn register_all() {
        let plugins: &[(&str, unsafe extern "C" fn() -> GstBool)] = &[
            ("app", gst_plugin_app_register),
            ("coreelements", gst_plugin_coreelements_register),
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
            ("videoparsersbad", gst_plugin_videoparsersbad_register),
        ];

        for (name, register_fn) in plugins {
            if !is_gst_result_ok(register_fn()) {
                log::error!("Failed to register GStreamer plugin: {}", name);
            }
        }
    }
}
