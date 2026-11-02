use std::sync::Arc;

use anyhow::Context;
use gst::prelude::*;
use gst_app::AppSink;
use log::{debug, error, info, warn};
use parking_lot::Mutex;

use crate::{
    core::{
        session::{VideoSession, VideoSessionCommon},
        texture::{attach_appsink, FlutterVideoSink},
        types::PlaybinConfig,
    },
    dart_types::{StreamEvent, StreamState},
};

pub struct PlaybinSession {
    session_common: VideoSessionCommon,
    config: PlaybinConfig,
    shutdown_sender: tokio::sync::mpsc::Sender<()>,
    active_pipeline: Mutex<Option<Arc<gst::Pipeline>>>,
    current_speed: Mutex<f64>,
}

impl PlaybinSession {
    pub fn new(
        config: PlaybinConfig,
        session_common: VideoSessionCommon,
    ) -> (Arc<Self>, tokio::sync::mpsc::Receiver<()>) {
        let (shutdown_sender, shutdown_receiver) = tokio::sync::mpsc::channel(1);

        let session = Arc::new(Self {
            session_common,
            config,
            shutdown_sender,
            active_pipeline: Mutex::new(None),
            current_speed: Mutex::new(1.0),
        });

        (session, shutdown_receiver)
    }

    pub async fn execute(
        self: &Arc<Self>,
        mut shutdown_rx: tokio::sync::mpsc::Receiver<()>,
    ) -> anyhow::Result<()> {
        let video_sink = FlutterVideoSink::new().await?;
        let texture_id = video_sink.texture_id();
        info!("Playbin: texture created, id={}", texture_id);

        let gpu = video_sink.is_gpu();
        let result = self
            .run_pipeline(&mut shutdown_rx, video_sink, texture_id)
            .await;
        // Only decoder/appsink failures say anything about the GPU output; source
        // and other pipeline errors would fail again with the CPU sink.
        let output_failed = result
            .as_ref()
            .err()
            .is_some_and(|e| e.downcast_ref::<super::VideoOutputError>().is_some());
        if gpu && output_failed {
            warn!(
                "Playbin: GPU output failed, retrying CPU output: {:?}",
                result.as_ref().err()
            );
            let sink = FlutterVideoSink::cpu().await?;
            return self
                .run_pipeline(&mut shutdown_rx, sink.clone(), sink.texture_id())
                .await;
        }
        result
    }

    async fn run_pipeline(
        self: &Arc<Self>,
        shutdown_rx: &mut tokio::sync::mpsc::Receiver<()>,
        video_sink: Arc<FlutterVideoSink>,
        texture_id: i64,
    ) -> anyhow::Result<()> {
        self.session_common.send_state_msg(StreamState::Loading);

        let appsink = AppSink::builder().build();

        // Set up appsink callbacks for frame processing
        let session_weak = Arc::downgrade(self);
        let event_session = session_weak.clone();
        let resize_session = session_weak.clone();
        attach_appsink(
            &appsink,
            video_sink.clone(),
            move |event| {
                if let Some(session) = event_session.upgrade() {
                    session.session_common.send_event_msg(event);
                }
            },
            move |texture_id| {
                if let Some(session) = resize_session.upgrade() {
                    session.session_common.send_state_msg(StreamState::Playing {
                        texture_id,
                        seekable: true,
                    });
                }
            },
        );

        // Build playbin3 pipeline
        let playbin = gst::ElementFactory::make("playbin3")
            .build()
            .context("Failed to create playbin3 element")?;
        playbin.set_property("uri", &self.config.uri);
        playbin.set_property("video-sink", &appsink);

        if self.config.mute {
            playbin.set_property("mute", true);
        }

        let pipeline = playbin
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("playbin3 is not a pipeline"))?;
        let pipeline_arc = Arc::new(pipeline);
        let _pipeline_guard = super::PipelineGuard(pipeline_arc.clone());

        *self.active_pipeline.lock() = Some(Arc::clone(&pipeline_arc));

        // Set up GStreamer bus monitoring
        let (gst_event_tx, mut gst_event_rx) = tokio::sync::mpsc::channel::<GstBusEvent>(16);
        let bus = pipeline_arc
            .bus()
            .ok_or(anyhow::anyhow!("Failed to get pipeline bus"))?;
        let bus_session_id = self.session_common.session_id;
        let bus_appsink = appsink.clone();

        bus.set_sync_handler(move |_bus, msg| {
            match msg.view() {
                gst::MessageView::Error(err) => {
                    log::error!("GStreamer failure: {}: {:?}", err.error(), err.debug());
                    let _ = gst_event_tx.try_send(GstBusEvent::Error {
                        message: format!("GStreamer error [{}]: {}", bus_session_id, err.error()),
                        output: is_video_output_error(msg.src(), &bus_appsink),
                    });
                }
                gst::MessageView::Eos(_) => {
                    let _ = gst_event_tx.try_send(GstBusEvent::Eos);
                }
                gst::MessageView::Buffering(buffering) => {
                    let percent = buffering.percent();
                    let _ = gst_event_tx.try_send(GstBusEvent::Buffering(percent));
                }
                gst::MessageView::StateChanged(sc) => {
                    let src_name = msg.src().map(|s| s.name().to_string()).unwrap_or_default();
                    let _ = gst_event_tx.try_send(GstBusEvent::StateChanged {
                        src: src_name,
                        old: sc.old(),
                        new: sc.current(),
                    });
                }
                gst::MessageView::Warning(w) => {
                    let _ = gst_event_tx.try_send(GstBusEvent::Warning(format!(
                        "GStreamer warning [{}]: {}",
                        bus_session_id,
                        w.error()
                    )));
                }
                _ => {}
            }
            gst::BusSyncReply::Drop
        });

        let state_change = pipeline_arc
            .set_state(gst::State::Playing)
            .context("setting GStreamer pipeline to Playing")?;
        info!("Playbin: set_state(Playing) -> {:?}", state_change);

        // Send Playing state with texture_id
        self.session_common.send_state_msg(StreamState::Playing {
            texture_id,
            seekable: true,
        });

        // Main event loop
        let mut loop_result: anyhow::Result<()> = Ok(());
        loop {
            tokio::select! {
                cmd = shutdown_rx.recv() => {
                    if cmd.is_some() {
                        info!("Playbin: shutdown command received, stopping");
                        if let Some(pipeline) = self.active_pipeline.lock().take() {
                            let _ = pipeline.set_state(gst::State::Null);
                        }
                        break;
                    }
                }
                event = gst_event_rx.recv() => {
                    match event {
                        Some(GstBusEvent::Error { message: msg, output }) => {
                            error!("Playbin: {}", msg);
                            self.session_common.send_event_msg(StreamEvent::Error(msg.clone()));
                            if let Some(pipeline) = self.active_pipeline.lock().take() {
                                let _ = pipeline.set_state(gst::State::Null);
                            }
                            loop_result = Err(if output {
                                super::VideoOutputError(msg).into()
                            } else {
                                anyhow::anyhow!(msg)
                            });
                            break;
                        }
                        Some(GstBusEvent::Warning(msg)) => {
                            warn!("Playbin: {}", msg);
                        }
                        Some(GstBusEvent::Eos) => {
                            info!("Playbin: EOS received");
                            if let Some(pipeline) = self.active_pipeline.lock().take() {
                                let _ = pipeline.set_state(gst::State::Null);
                            }
                            self.session_common.send_state_msg(StreamState::Stopped);
                            break;
                        }
                        Some(GstBusEvent::Buffering(percent)) => {
                            debug!("Playbin: buffering {}%", percent);
                        }
                        Some(GstBusEvent::StateChanged { src, old, new }) => {
                            debug!("Playbin: [{}] state {:?} -> {:?}", src, old, new);
                        }
                        None => {
                            warn!("Playbin: bus event channel closed unexpectedly");
                            break;
                        }
                    }
                }
            }
        }

        // Send Stopped state
        let _ = self.session_common.send_state_msg(StreamState::Stopped);

        loop_result
    }
}

/// Whether a bus error came from the video output path (the appsink or a decoder).
fn is_video_output_error(src: Option<&gst::Object>, appsink: &AppSink) -> bool {
    let Some(src) = src else {
        return false;
    };
    if src == appsink.upcast_ref::<gst::Object>()
        || src.has_as_ancestor(appsink.upcast_ref::<gst::Object>())
    {
        return true;
    }
    src.downcast_ref::<gst::Element>()
        .and_then(|element| element.factory())
        .is_some_and(|factory| factory.klass().contains("Decoder"))
}

#[derive(Debug, Clone)]
enum GstBusEvent {
    Error {
        message: String,
        /// Raised by the decoder or the appsink rather than the source/demuxer.
        output: bool,
    },
    Warning(String),
    Eos,
    Buffering(i32),
    StateChanged {
        src: String,
        old: gst::State,
        new: gst::State,
    },
}

#[async_trait::async_trait]
impl VideoSession for PlaybinSession {
    fn session_id(&self) -> i64 {
        self.session_common.session_id
    }

    fn last_alive_mark(&self) -> std::time::SystemTime {
        self.session_common.get_last_alive_mark()
    }

    fn make_alive(&self) {
        self.session_common.mark_alive();
    }

    fn terminate(&self) {
        if let Some(pipeline) = self.active_pipeline.lock().take() {
            let _ = pipeline.set_state(gst::State::Null);
        }
        let _ = self.shutdown_sender.blocking_send(());
    }

    async fn seek(&self, ts_ms: u64) -> anyhow::Result<()> {
        let pipeline = self
            .active_pipeline
            .lock()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No active pipeline"))?;

        let pos = gst::ClockTime::from_mseconds(ts_ms);
        pipeline
            .seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, pos)
            .map_err(|_| anyhow::anyhow!("seek failed"))
    }

    async fn go_to_live_stream(&self) -> anyhow::Result<()> {
        // No-op for playbin - not applicable
        Ok(())
    }

    async fn set_speed(&self, speed: f64) -> anyhow::Result<()> {
        let pipeline = self
            .active_pipeline
            .lock()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No active pipeline"))?;

        // Query current position
        let current_pos = pipeline
            .query_position::<gst::ClockTime>()
            .unwrap_or(gst::ClockTime::ZERO);

        pipeline
            .seek(
                speed,
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::SeekType::Set,
                current_pos,
                gst::SeekType::None,
                gst::ClockTime::NONE,
            )
            .map_err(|_| anyhow::anyhow!("set_speed seek failed"))?;

        *self.current_speed.lock() = speed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_appsink_and_decoder_errors_count_as_output_failures() {
        gst::init().unwrap();
        let appsink = AppSink::builder().build();
        let source = gst::ElementFactory::make("fakesrc").build().unwrap();

        assert!(is_video_output_error(
            Some(appsink.upcast_ref::<gst::Object>()),
            &appsink
        ));
        assert!(!is_video_output_error(
            Some(source.upcast_ref::<gst::Object>()),
            &appsink
        ));
        assert!(!is_video_output_error(None, &appsink));
    }
}
