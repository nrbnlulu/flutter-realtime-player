use super::flutter::FlutterPixelBufferSink;
use crate::dart_types::StreamEvent;
use anyhow::Context;
use gst::prelude::ObjectExt;
use std::sync::Arc;

pub(crate) const ANDROID_CAPS: &str = "video/x-raw(memory:AHardwareBuffer),format=AHARDWARE_BUFFER";
pub(crate) const CPU_CAPS: &str = "video/x-raw,format=RGBA";

/// The output contract shared by URI playback and WSC-RTP. Android's native
/// texture keeps its ID across resolution changes; CPU uploads reuse the
/// existing resize implementation.
pub(crate) enum FlutterVideoSink {
    Pixels(Arc<FlutterPixelBufferSink>),
    #[cfg(all(target_os = "android", gst_android_hardware_buffer))]
    Android(flutter_plugin_sdk::android::HardwareBufferTexture),
}

impl FlutterVideoSink {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        #[cfg(all(target_os = "android", gst_android_hardware_buffer))]
        {
            let (tx, rx) = tokio::sync::oneshot::channel();
            crate::core::main_thread_dispatcher()?
                .dispatch(move || {
                    let result = crate::core::gpu_textures().and_then(|gpu| {
                        gpu.create_android_hardware_buffer_texture()
                            .map_err(|e| anyhow::anyhow!("Android buffer import: {e:?}"))
                    });
                    let _ = tx.send(result);
                })
                .map_err(|e| anyhow::anyhow!("texture dispatch: {e:?}"))?;
            match rx.await? {
                Ok(texture) => {
                    log::info!("Video output: Android hardware buffers (no CPU pixel copy)");
                    return Ok(Arc::new(Self::Android(texture)));
                }
                Err(error) => log::info!("Video output: CPU fallback: {error}"),
            }
        }
        Self::cpu().await
    }

    pub async fn cpu() -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self::Pixels(Arc::new(
            FlutterPixelBufferSink::new(640, 480).await?,
        ))))
    }

    pub fn is_gpu(&self) -> bool {
        match self {
            Self::Pixels(_) => false,
            #[cfg(all(target_os = "android", gst_android_hardware_buffer))]
            Self::Android(_) => true,
        }
    }

    pub fn texture_id(&self) -> i64 {
        match self {
            Self::Pixels(sink) => sink.texture_id(),
            #[cfg(all(target_os = "android", gst_android_hardware_buffer))]
            Self::Android(texture) => texture.texture_id(),
        }
    }

    pub fn caps(&self) -> gst::Caps {
        if self.is_gpu() {
            ANDROID_CAPS
        } else {
            CPU_CAPS
        }
        .parse()
        .unwrap()
    }

    fn write_sample(
        &self,
        sample: &gst::Sample,
        on_resized: impl FnOnce(i64) + Send + 'static,
    ) -> anyhow::Result<()> {
        let caps = sample.caps().context("video sample has no caps")?;
        let buffer = sample
            .buffer_owned()
            .context("video sample has no buffer")?;
        match self {
            Self::Pixels(sink) => {
                let info = gst_video::VideoInfo::from_caps(caps)?;
                sink.write_frame(&info, buffer, on_resized)
            }
            #[cfg(all(target_os = "android", gst_android_hardware_buffer))]
            Self::Android(texture) => super::android::present(texture, caps, buffer),
        }
    }
}

/// Keeps dimension reporting, sample ownership, backpressure, and callback
/// errors identical for both input pipelines. Returning FlowError makes output
/// failures visible on the bus, where GPU negotiation/import can fall back.
pub(crate) fn attach_appsink(
    appsink: &gst_app::AppSink,
    sink: Arc<FlutterVideoSink>,
    on_event: impl Fn(StreamEvent) + Send + Sync + 'static,
    on_texture: impl Fn(i64) + Send + Sync + 'static,
) {
    let mut last_size = None;
    let on_texture = Arc::new(on_texture);
    // Use the pipeline clock for both URI and RTP playback. Without this,
    // file playback can decode to EOS before Flutter has painted its first frame.
    appsink.set_property("sync", true);
    appsink.set_property("max-lateness", 100_000_000_i64);
    appsink.set_property("qos", true);
    appsink.set_caps(Some(&sink.caps()));
    appsink.set_max_buffers(1);
    appsink.set_drop(true);
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |appsink| {
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let process = || -> anyhow::Result<(u32, u32)> {
                    let caps = sample.caps().context("video sample has no caps")?;
                    let structure = caps.structure(0).context("video caps are empty")?;
                    let width = u32::try_from(structure.get::<i32>("width")?)?;
                    let height = u32::try_from(structure.get::<i32>("height")?)?;
                    anyhow::ensure!(width > 0 && height > 0, "invalid video dimensions");
                    let on_texture = on_texture.clone();
                    sink.write_sample(&sample, move |id| on_texture(id))?;
                    Ok((width, height))
                };
                match process() {
                    Ok(size) => {
                        if last_size != Some(size) {
                            last_size = Some(size);
                            log::info!(
                                "Video sample: {}x{}, output={}",
                                size.0,
                                size.1,
                                if sink.is_gpu() {
                                    "Android hardware buffer"
                                } else {
                                    "CPU RGBA"
                                }
                            );
                            on_event(StreamEvent::OriginVideoSize {
                                width: size.0 as u64,
                                height: size.1 as u64,
                            });
                        }
                        Ok(gst::FlowSuccess::Ok)
                    }
                    Err(error) => {
                        log::warn!("Video output failed: {error:#}");
                        Err(gst::FlowError::Error)
                    }
                }
            })
            .build(),
    );
}

#[cfg(any(test, all(target_os = "android", gst_android_hardware_buffer)))]
pub(super) fn crop_from_ndc(
    matrix: [f32; 16],
    width: u32,
    height: u32,
) -> anyhow::Result<[u32; 4]> {
    // androidmedia describes padding/cropping as a scale and translation in
    // NDC. Arbitrary rotations/shears need a transform-aware presentation API.
    for (index, value) in matrix.iter().enumerate() {
        anyhow::ensure!(value.is_finite(), "non-finite video transform");
        if ![0, 5, 10, 12, 13, 15].contains(&index) {
            anyhow::ensure!(value.abs() < 0.0001, "unsupported video transform");
        }
    }
    anyhow::ensure!(
        matrix[0] >= 1.0 && matrix[5] >= 1.0 && (matrix[15] - 1.0).abs() < 0.0001,
        "unsupported video scale"
    );
    let x = (matrix[0] - 1.0 - matrix[12]) * width as f32 / 2.0;
    let y = (matrix[13] - 1.0 + matrix[5]) * height as f32 / 2.0;
    anyhow::ensure!(
        x >= -0.01 && y >= -0.01 && x < u32::MAX as f32 && y < u32::MAX as f32,
        "invalid video crop"
    );
    Ok([
        x.max(0.0).round() as u32,
        y.max(0.0).round() as u32,
        width,
        height,
    ])
}

#[cfg(test)]
mod tests {
    use super::crop_from_ndc;
    fn identity() -> [f32; 16] {
        [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ]
    }
    #[test]
    fn converts_decoder_padding_and_crop_to_source_pixels() {
        assert_eq!(
            crop_from_ndc(identity(), 100, 100).unwrap(),
            [0, 0, 100, 100]
        );
        let mut m = identity();
        m[0] = 2.;
        m[5] = 2.;
        m[12] = 0.;
        m[13] = -0.6;
        assert_eq!(crop_from_ndc(m, 100, 100).unwrap(), [50, 20, 100, 100]);
    }
    #[test]
    fn rejects_transforms_that_cannot_be_presented_as_a_source_rectangle() {
        let mut m = identity();
        m[1] = 1.;
        assert!(crop_from_ndc(m, 100, 100).is_err());
        m = identity();
        m[12] = f32::NAN;
        assert!(crop_from_ndc(m, 100, 100).is_err());
        m = identity();
        m[0] = -1.;
        assert!(crop_from_ndc(m, 100, 100).is_err());
    }
}
