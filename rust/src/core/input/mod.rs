pub mod playbin;
pub mod wsc_rtp;

/// Ensures every early error/cancellation stops decoder callbacks before their
/// texture owners are dropped. Both input paths retain this for their run.
struct PipelineGuard(std::sync::Arc<gst::Pipeline>);
impl Drop for PipelineGuard {
    fn drop(&mut self) {
        use gst::prelude::*;
        let _ = self.0.set_state(gst::State::Null);
        if let Some(bus) = self.0.bus() {
            bus.unset_sync_handler();
        }
    }
}

#[derive(Debug)]
struct VideoOutputError(String);
impl std::fmt::Display for VideoOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for VideoOutputError {}
