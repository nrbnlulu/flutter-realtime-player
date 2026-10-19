//! Tests Vulkan Video decoding without CPU mapping or a Flutter engine.
//! Run with `--list`, or `h264|h265 PATH_TO_ELEMENTARY_STREAM`.

use std::{ffi::CStr, path::PathBuf, time::Instant};

use anyhow::{bail, ensure, Context, Result};
use ash::{vk, Entry};
use gst::prelude::*;

const CODECS: [Codec; 2] = [Codec::H264, Codec::H265];
const FRAME_LIMIT: usize = 120;
const FRAME_TIMEOUT: gst::ClockTime = gst::ClockTime::from_seconds(10);

#[derive(Clone, Copy, Debug)]
enum Codec {
    H264,
    H265,
}

impl Codec {
    fn name(self) -> &'static str {
        match self {
            Self::H264 => "h264",
            Self::H265 => "h265",
        }
    }

    fn parser(self) -> &'static str {
        match self {
            Self::H264 => "h264parse",
            Self::H265 => "h265parse",
        }
    }

    fn decoder(self) -> &'static str {
        match self {
            Self::H264 => "vulkanh264dec",
            Self::H265 => "vulkanh265dec",
        }
    }

    fn extension(self) -> &'static CStr {
        match self {
            Self::H264 => ash::khr::video_decode_h264::NAME,
            Self::H265 => ash::khr::video_decode_h265::NAME,
        }
    }

    fn operation(self) -> vk::VideoCodecOperationFlagsKHR {
        match self {
            Self::H264 => vk::VideoCodecOperationFlagsKHR::DECODE_H264,
            Self::H265 => vk::VideoCodecOperationFlagsKHR::DECODE_H265,
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let input = match args.as_slice() {
        [] => None,
        [list] if list == "--list" => None,
        [codec, path] => {
            let codec = CODECS
                .into_iter()
                .find(|candidate| codec == candidate.name())
                .context("codec must be h264 or h265")?;
            let path = PathBuf::from(path);
            ensure!(path.is_file(), "input file not found: {}", path.display());
            Some((codec, path))
        }
        _ => bail!("usage: android_vulkan_probe [--list | h264|h265 ELEMENTARY_STREAM]"),
    };

    println!(
        "GStreamer Android SDK Vulkan Video: {}",
        option_env!("GST_ANDROID_VULKAN_VIDEO_COMPILED")
            .unwrap_or("system SDK (inspect decoder factories below)")
    );
    // This only reports extension/queue availability. Actual stream decoding
    // below is the check for profile, format, resolution and driver support.
    inspect_driver()?;
    gst::init()?;
    #[cfg(target_os = "android")]
    flutter_realtime_player::android_gst_plugins::register_for_probe();
    for codec in CODECS {
        println!(
            "{}: parser={}, decoder={}",
            codec.name(),
            gst::ElementFactory::find(codec.parser()).is_some(),
            gst::ElementFactory::find(codec.decoder()).is_some(),
        );
    }

    if let Some((codec, path)) = input {
        decode(codec, path)?;
    }
    Ok(())
}

fn inspect_driver() -> Result<()> {
    // SAFETY: Entry retains the Vulkan loader until all instance calls finish.
    let entry = unsafe { Entry::load() }.context("loading libvulkan")?;
    let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
    // SAFETY: all structures are initialized and their references remain live.
    let instance = unsafe {
        entry.create_instance(
            &vk::InstanceCreateInfo::default().application_info(&app),
            None,
        )
    }
    .context("creating a Vulkan 1.1 instance")?;

    let result = (|| -> Result<()> {
        // SAFETY: instance is live until the final destroy_instance call.
        let devices = unsafe { instance.enumerate_physical_devices() }?;
        ensure!(!devices.is_empty(), "no Vulkan physical devices found");
        for device in devices {
            // SAFETY: device was enumerated from this instance.
            let properties = unsafe { instance.get_physical_device_properties(device) };
            let name = properties.device_name_as_c_str()?;
            let extensions = unsafe { instance.enumerate_device_extension_properties(device) }?;
            let has_extension = |name: &CStr| {
                extensions.iter().any(|extension| {
                    extension
                        .extension_name_as_c_str()
                        .is_ok_and(|value| value == name)
                })
            };
            let video_queue = has_extension(ash::khr::video_queue::NAME)
                && has_extension(ash::khr::video_decode_queue::NAME);
            println!(
                "Vulkan device: {} (video decode extensions={video_queue})",
                name.to_string_lossy()
            );

            // Only chain video properties on devices advertising video_queue.
            let count =
                unsafe { instance.get_physical_device_queue_family_properties2_len(device) };
            let mut video = vec![vk::QueueFamilyVideoPropertiesKHR::default(); count];
            let mut queues: Vec<_> = video
                .iter_mut()
                .map(|properties| {
                    let queue = vk::QueueFamilyProperties2::default();
                    if video_queue {
                        queue.push_next(properties)
                    } else {
                        queue
                    }
                })
                .collect();
            unsafe { instance.get_physical_device_queue_family_properties2(device, &mut queues) };
            let decode_queues: Vec<_> = queues
                .iter()
                .enumerate()
                .filter_map(|(index, queue)| {
                    (queue.queue_family_properties.queue_count > 0
                        && queue
                            .queue_family_properties
                            .queue_flags
                            .contains(vk::QueueFlags::VIDEO_DECODE_KHR))
                    .then_some(index)
                })
                .collect();
            drop(queues);
            for codec in CODECS {
                let families: Vec<_> = decode_queues
                    .iter()
                    .copied()
                    .filter(|&index| {
                        video[index]
                            .video_codec_operations
                            .contains(codec.operation())
                    })
                    .collect();
                println!(
                    "  {}: codec extension={}, decode queue families={families:?}",
                    codec.name(),
                    has_extension(codec.extension())
                );
            }
        }
        Ok(())
    })();
    // SAFETY: no device was created and no object outlives the instance.
    unsafe { instance.destroy_instance(None) };
    result
}

/// Always returns the pipeline to NULL, including on errors and timeouts.
struct RunningPipeline(gst::Pipeline);

impl Drop for RunningPipeline {
    fn drop(&mut self) {
        let _ = self.0.set_state(gst::State::Null);
    }
}

fn decode(codec: Codec, path: PathBuf) -> Result<()> {
    ensure!(
        gst::ElementFactory::find(codec.decoder()).is_some(),
        "{} is unavailable: check SDK Vulkan Video build support and device capabilities",
        codec.decoder()
    );
    let source = gst::ElementFactory::make("filesrc")
        .property(
            "location",
            path.to_str().context("input path must be UTF-8")?,
        )
        .build()?;
    let parser = gst::ElementFactory::make(codec.parser()).build()?;
    let decoder = gst::ElementFactory::make(codec.decoder()).build()?;
    let sink = gst_app::AppSink::builder()
        .caps(
            &gst::Caps::builder("video/x-raw")
                .features(["memory:VulkanImage"])
                .build(),
        )
        .sync(false)
        .max_buffers(2)
        .build();

    let pipeline = RunningPipeline(gst::Pipeline::new());
    pipeline
        .0
        .add_many([&source, &parser, &decoder, sink.upcast_ref()])?;
    gst::Element::link_many([&source, &parser, &decoder, sink.upcast_ref()])?;
    let bus = pipeline.0.bus().context("pipeline has no bus")?;
    pipeline.0.set_state(gst::State::Playing)?;

    let started = Instant::now();
    let mut count = 0;
    while count < FRAME_LIMIT {
        let sample = sink.try_pull_sample(FRAME_TIMEOUT);
        for message in bus.iter() {
            if let gst::MessageView::Error(error) = message.view() {
                bail!("GStreamer: {} ({:?})", error.error(), error.debug());
            }
        }
        let Some(sample) = sample else {
            if sink.is_eos() {
                break;
            }
            bail!("timed out waiting for a decoded Vulkan frame after {count} frames");
        };
        validate_vulkan_sample(&sample)?;
        if count == 0 {
            println!("Negotiated output: {}", sample.caps().unwrap());
        }
        count += 1;
    }
    ensure!(count > 0, "no video frames decoded");
    println!("Decoded {count} frames in {:?}; all retained VulkanImage memory, with no CPU mapping. Flutter image import is not tested by this probe.", started.elapsed());
    Ok(())
}

fn validate_vulkan_sample(sample: &gst::Sample) -> Result<()> {
    let caps = sample.caps().context("sample has no caps")?;
    ensure!(!caps.is_empty(), "sample has empty caps");
    ensure!(
        caps.features(0)
            .is_some_and(|features| features.contains("memory:VulkanImage")),
        "decoder negotiated system memory: {caps}"
    );
    let buffer = sample.buffer().context("sample has no buffer")?;
    ensure!(buffer.n_memory() > 0, "decoded frame has no memory");
    ensure!(
        buffer
            .iter_memories()
            .all(|memory| memory.is_type("VulkanImage")),
        "decoded frame contains non-Vulkan memory despite its caps"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_cpu_memory_even_with_vulkan_caps() {
        gst::init().unwrap();
        let caps = gst::Caps::builder("video/x-raw")
            .features(["memory:VulkanImage"])
            .build();
        let buffer = gst::Buffer::from_slice(vec![0u8; 16]);
        let sample = gst::Sample::builder().caps(&caps).buffer(&buffer).build();
        assert!(validate_vulkan_sample(&sample)
            .unwrap_err()
            .to_string()
            .contains("non-Vulkan memory"));
    }

    #[test]
    fn rejects_system_memory_caps_and_empty_buffers() {
        gst::init().unwrap();
        let caps = gst::Caps::builder("video/x-raw").build();
        let sample = gst::Sample::builder().caps(&caps).build();
        assert!(validate_vulkan_sample(&sample)
            .unwrap_err()
            .to_string()
            .contains("system memory"));

        let caps = gst::Caps::builder("video/x-raw")
            .features(["memory:VulkanImage"])
            .build();
        let buffer = gst::Buffer::new();
        let sample = gst::Sample::builder().caps(&caps).buffer(&buffer).build();
        assert!(validate_vulkan_sample(&sample)
            .unwrap_err()
            .to_string()
            .contains("no memory"));
    }
}
