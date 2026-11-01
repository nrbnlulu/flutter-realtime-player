#[cfg(all(target_os = "android", gst_android_hardware_buffer))]
mod android;
pub mod flutter;
mod video;

pub(crate) use video::{attach_appsink, FlutterVideoSink, ANDROID_CAPS, CPU_CAPS};
