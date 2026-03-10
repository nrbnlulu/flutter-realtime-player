use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use anyhow::{Context, Result};
use gst::prelude::*;
use gst_app::AppSink;
use irondash_texture::Texture;
use log::{debug, error, info, trace, warn};
use parking_lot::Mutex;

use crate::{
    core::{
        session::{VideoSession, VideoSessionCommon},
        texture::{
            payload::{self, RawRgbaFrame, SharedPixelData},
            FlutterTextureSession,
        },
        types::PlaybinConfig,
    },
    dart_types::{StreamEvent, StreamState},
    utils::invoke_on_platform_main_thread,
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
        let payload_holder = Arc::new(payload::PayloadHolder::new());
        let payload_holder_weak = Arc::downgrade(&payload_holder);
        let payload_holder_for_texture = Arc::clone(&payload_holder);
        let engine_handle = self.session_common.engine_handle;

        let (sendable_texture, texture_id) =
            invoke_on_platform_main_thread(move || -> Result<_> {
                let texture =
                    Texture::new_with_provider(engine_handle, payload_holder_for_texture)?;
                let texture_id = texture.id();
                info!("Playbin: texture created, id={}", texture_id);
                Ok((texture.into_sendable_texture(), texture_id))
            })?;

        let texture_session = Arc::new(crate::core::texture::flutter::TextureSession::new(
            texture_id,
            Arc::downgrade(&sendable_texture),
            payload_holder_weak.clone(),
        ));
        let texture_session: Arc<dyn FlutterTextureSession> = texture_session;

        self.session_common.send_state_msg(StreamState::Loading);

        // Build appsink for receiving video frames
        let caps = gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .build();
        let appsink = AppSink::builder().caps(&caps).sync(false).build();

        // Set up appsink callbacks for frame processing
        let session_weak = Arc::downgrade(self);
        let session_weak_for_callbacks = session_weak.clone();
        let size = Arc::new(parking_lot::Mutex::new((0u32, 0u32)));
        let frame_count = Arc::new(AtomicU32::new(0));

        appsink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |sink| {
                    let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                    let caps = sample.caps().ok_or(gst::FlowError::Error)?;
                    let video_info =
                        gst_video::VideoInfo::from_caps(caps).map_err(|_| gst::FlowError::Error)?;
                    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;

                    let width = video_info.width();
                    let height = video_info.height();

                    let mut size_guard = size.lock();
                    let (cached_width, cached_height) = *size_guard;
                    // Emit OriginVideoSize only when dimensions change (lock-free comparison)
                    if cached_width != width || cached_height != height {
                        *size_guard = (width, height);
                        log::debug!("Playbin: video dimensions: {}x{}", width, height);
                        if let Some(session) = session_weak_for_callbacks.upgrade() {
                            session
                                .session_common
                                .send_event_msg(StreamEvent::OriginVideoSize {
                                    width: width as u64,
                                    height: height as u64,
                                });
                        }
                    }
                    drop(size_guard);

                    let video_frame =
                        gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &video_info)
                            .map_err(|_| gst::FlowError::Error)?;

                    let stride = video_info.stride()[0] as usize;
                    let expected_stride = (width as usize) * 4; // RGBA
                    let plane_data = video_frame
                        .plane_data(0)
                        .map_err(|_| gst::FlowError::Error)?;

                    let data = if stride == expected_stride {
                        plane_data.to_vec()
                    } else {
                        // Stride mismatch — copy row by row to strip padding
                        let mut buf = Vec::with_capacity(expected_stride * height as usize);
                        for y in 0..height as usize {
                            let row_start = y * stride;
                            buf.extend_from_slice(
                                &plane_data[row_start..row_start + expected_stride],
                            );
                        }
                        buf
                    };

                    let frame = RawRgbaFrame {
                        width,
                        height,
                        data,
                    };

                    if let Some(holder) = payload_holder_weak.upgrade() {
                        holder.set_payload(Arc::new(frame) as SharedPixelData);
                        texture_session.mark_frame_available();
                    } else {
                        warn!("Playbin: payload_holder dropped, frame discarded");
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
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

        *self.active_pipeline.lock() = Some(Arc::clone(&pipeline_arc));

        // Set up GStreamer bus monitoring
        let (gst_event_tx, mut gst_event_rx) = tokio::sync::mpsc::channel::<GstBusEvent>(16);
        let bus = pipeline_arc
            .bus()
            .ok_or(anyhow::anyhow!("Failed to get pipeline bus"))?;
        let bus_session_id = self.session_common.session_id;

        bus.set_sync_handler(move |_bus, msg| {
            match msg.view() {
                gst::MessageView::Error(err) => {
                    let _ = gst_event_tx.try_send(GstBusEvent::Error(format!(
                        "GStreamer error [{}]: {}",
                        bus_session_id,
                        err.error()
                    )));
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
                        Some(GstBusEvent::Error(msg)) => {
                            error!("Playbin: {}", msg);
                            self.session_common.send_event_msg(StreamEvent::Error(msg.clone()));
                            if let Some(pipeline) = self.active_pipeline.lock().take() {
                                let _ = pipeline.set_state(gst::State::Null);
                            }
                            loop_result = Err(anyhow::anyhow!(msg));
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

        // Texture + payload_holder must always be dropped on the platform main thread,
        // regardless of how the loop exited (including error paths).
        crate::utils::invoke_on_platform_main_thread(move || {
            drop(sendable_texture);
            drop(payload_holder);
        });

        loop_result
    }
}

#[derive(Debug, Clone)]
enum GstBusEvent {
    Error(String),
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

    fn engine_handle(&self) -> i64 {
        self.session_common.engine_handle
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

    fn set_events_sink(&self, sink: crate::core::types::DartEventsStream) {
        self.session_common.set_events_sink(sink);
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
