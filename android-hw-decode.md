# Android GPU texture + hardware decoding (zero-copy) — implementation plan

## Goal

Replace the Android software path (avdec → videoconvert → RGBA appsink → CPU copy →
`ANativeWindow_lock` memcpy) with a zero-copy GPU path:

```
MediaCodec (amcviddec, HW) ─ SurfaceTexture/GLMemory (external-oes, stays on GPU)
        → glimagesink ─ EGL render into ANativeWindow
        → Flutter SurfaceTextureEntry (GPU external texture)
        → Texture widget
```

No pixel data ever touches the CPU. Decoding is done by the device's MediaCodec HW
decoder; glimagesink does one GPU blit into the Flutter-owned Surface.

## Current state (what exists today)

- Both inputs (`rust/src/core/input/wsc_rtp.rs`, `rust/src/core/input/playbin.rs`) build a
  pipeline ending in `videoconvert ! video/x-raw,format=RGBA ! appsink`, copy each frame
  into `RawRgbaFrame`, store it in `PayloadHolder`, and call `mark_frame_available()`.
  On Android irondash then does *another* copy via `ANativeWindow_lock` into the
  SurfaceTexture. Two CPU copies + software decode + software colorconvert per frame.
- `wsc_rtp.rs:916-917` hardcodes `avdec_h264` / `avdec_h265` (software).
- `rust/src/lib.rs` statically registers plugins on Android, **including `androidmedia`**
  (`gst_plugin_androidmedia_register`), but registration almost certainly **fails silently
  today**: its `plugin_init` calls `gst_amc_jni_initialize()` which needs a JavaVM and an
  application-class-loader provider, and neither is available (see JNI findings below). So
  HW decoders are not actually registered at all right now.

## Research findings (all verified against local sources)

### 1. irondash_texture has a zero-copy Android texture type

`~/.cargo/.../irondash_texture-0.5.0/src/platform/android/mod.rs`:

- `Texture::<NativeWindow>::new(engine_handle)` calls
  `TextureRegistry.createSurfaceTexture()` → wraps the `SurfaceTexture` in a
  `android.view.Surface` → `ANativeWindow_fromSurface`. `texture.get()` returns a
  `NativeWindow` (acquire/release refcounted `*mut ANativeWindow`).
- `PlatformTextureWithoutProvider` — no payload provider, no `get_payload` round-trip.
- Flutter's `SurfaceTextureRegistryEntry` registers an `OnFrameAvailableListener`
  internally, so **no `mark_frame_available()` calls are needed** — every
  `eglSwapBuffers` by glimagesink automatically schedules a Flutter frame.
  (`SendableTexture::mark_frame_available` with no provider is a harmless no-op.)
- `NativeWindow` is not `Send`; we need a small `unsafe Send+Sync` wrapper
  (`ANativeWindow` itself is thread-safe / refcounted).
- Dart side is unchanged: same `texture_id` → `Texture(textureId:)`.

### 2. glimagesink works on Android via GstVideoOverlay + ANativeWindow

- `gst-libs/gst/gl/android/gstglwindow_android_egl.c` (checked 1.28.3): the sink's
  window handle **is** an `ANativeWindow*` passed via
  `VideoOverlay::set_window_handle(handle)`. It can be set right after element creation
  (before any state change), so no `prepare-window-handle` bus juggling is needed.
- `draw_cb` calls `eglQuerySurface(WIDTH/HEIGHT)` **on every draw** and resizes —
  so late/changed buffer geometry is picked up automatically.
- `glimagesink` is a bin (glupload ! glcolorconvert ! actual sink): it accepts both
  `video/x-raw(memory:GLMemory)` from amcviddec (zero-copy) *and* plain
  `video/x-raw` from a software-decoder fallback (one GPU upload).

### 3. SurfaceTexture default buffer size is 1×1 — must set geometry

Flutter never calls `setDefaultBufferSize`. The producer-side override
`ANativeWindow_setBuffersGeometry(win, w, h, 0)` fixes this (format 0 = keep).
Call it when video dimensions become known (caps event on the sink pad); combined
with finding 2 (per-draw requery) the surface self-corrects even if the EGL surface
was created earlier at 1×1.

### 4. androidmedia JNI bootstrap — why it fails today and how to fix it

`gst-plugins-bad/sys/androidmedia/gstjniutils.c` (1.28.3) resolves two **exported C
symbols** via `g_module_open(NULL)` + `g_module_symbol`:

- `JavaVM *gst_android_get_java_vm(void)`
- `jobject gst_android_get_application_class_loader(void)` — **mandatory**; even with a
  VM set, `initialize_classes()` fails with "Could not find application class loader
  provider" if this symbol is absent. This is exactly why registration fails today.

Verified in GLib `gmodule/gmodule-dl.c`: on `__ANDROID__`, `g_module_open(NULL)`
returns `RTLD_DEFAULT`, and bionic's `dlsym(RTLD_DEFAULT)` searches *the caller's local
group* — the caller is gstjniutils code linked **inside our own cdylib**, so
`#[no_mangle] pub extern "C"` exports from `libflutter_realtime_player.so` **will be
found** even though Dart dlopen()s us `RTLD_LOCAL`. (This mirrors how the official
`libgstreamer_android.so` glue works.)

Both values are trivially available from `irondash_engine_context` (already a dep, has
an Android gradle part that loads before Dart runs):

- `EngineContext::get_java_vm() -> &'static jni::JavaVM` (works on any thread, uses
  `dlopen(libirondash_engine_context_native.so, RTLD_NOLOAD)` fallback globals)
- `EngineContext::get_class_loader() -> GlobalRef` (FlutterJNI's class loader — the app
  class loader, which can also load our plugin's Java classes)

The class-loader GlobalRef must be cached in a `static OnceLock<GlobalRef>` and
returned via `.as_raw()` so the jobject stays valid forever.

### 5. amcviddec's GL path needs one Java class compiled into the app

`libgstandroidmedia.a` loads `org.freedesktop.gstreamer.androidmedia.GstAmcOnFrameAvailableListener`
through the class loader above (its `native_onFrameAvailable` is registered via
`RegisterNatives` by the plugin itself — works regardless of how our .so was loaded).
The `.java` source ships in the GStreamer Android SDK
(`share/gst-android/ndk-build/androidmedia/`). The camera/sensor classes
(`GstAhcCallback`, `GstAhsCallback`) are **not** required: in `gstamc.c` `plugin_init`,
`ahc_init`/`ahs_init` failures are non-fatal (verified 1.28.3, lines 2009-2030).

If the Java class were missing, amcviddec degrades to ByteBuffer output (still HW
decode, but with CPU copies) — graceful, but we ship the class so the GL path works.

### 6. Static linking: the `opengl` plugin and its deps are in the SDK

`$GSTREAMER_ROOT_ANDROID/<arch>/lib/gstreamer-1.0/libgstopengl.a` exists.
Its `.la` lists deps beyond what `build.rs` already links:
`graphene-1.0`, `png16`, `jpeg`, `-lEGL -lGLESv2` (gstgl-1.0, gstcontroller-1.0,
gstallocators-1.0, orc, z, etc. — gstgl/allocators already linked; **gstcontroller-1.0
is not yet linked** and is required).
Registration entry point: `gst_plugin_opengl_register()`.

### 7. Decoder selection

Element names for HW decoders are device-specific (`amcviddec-omx...`,
`amcviddec-c2...`), so they cannot be hardcoded. Use **`decodebin3`** after
depay/parse — amcviddec registers at rank PRIMARY+ so decodebin3 auto-picks HW and
falls back to avdec if absent. For playbin3 (already used) it's automatic once
androidmedia actually registers.

### 8. Versions to match irondash (avoid type mismatches across crate boundary)

- `jni = "0.21.1"` (both irondash crates use 0.21.1; `GlobalRef` crosses our boundary)
- `ndk-sys = "0.4.1"` (what irondash_texture uses; provides
  `ANativeWindow_setBuffersGeometry` + links libandroid)

## Implementation steps

### A. `rust/build.rs` — link the GL bits (Android section)

```rust
// OpenGL plugin (glimagesink/glupload/glcolorconvert) + deps
println!("cargo:rustc-link-lib=static=gstopengl");
println!("cargo:rustc-link-lib=static=gstcontroller-1.0");
println!("cargo:rustc-link-lib=static=graphene-1.0");
println!("cargo:rustc-link-lib=static=png16");
println!("cargo:rustc-link-lib=static=jpeg");
println!("cargo:rustc-link-lib=EGL");
println!("cargo:rustc-link-lib=GLESv2");
```

### B. `rust/Cargo.toml` — Android deps

```toml
[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21.1"
ndk-sys = "0.4.1"
```

### C. `rust/src/lib.rs` — JNI glue exports + opengl registration

1. Add `gst_plugin_opengl_register` to the extern list + `register_all()`.
2. New `#[cfg(target_os = "android")]` module exporting the two glue symbols:

```rust
static CLASS_LOADER: OnceLock<jni::objects::GlobalRef> = OnceLock::new();

#[no_mangle]
pub extern "C" fn gst_android_get_java_vm() -> *mut jni::sys::JavaVM {
    irondash_engine_context::EngineContext::get_java_vm()
        .map(|vm| vm.get_java_vm_pointer())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn gst_android_get_application_class_loader() -> jni::sys::jobject {
    // cache GlobalRef so the jobject stays valid for the process lifetime
    ... EngineContext::get_class_loader() → CLASS_LOADER.get_or_init(...).as_raw()
}
```

(Keep registration order: these are plain exports, resolvable before `register_all()`
runs, so `gst_plugin_androidmedia_register()` in `flutter_realtime_player_init` now
succeeds. `gst_amc_jni_set_java_vm` is unnecessary — the VM is discovered through the
exported getter.)

Note: androidmedia registration scans MediaCodec codecs via JNI on first init
(~100-500 ms, once per process). Acceptable at init; if it bothers startup we can move
just the androidmedia registration onto a background thread later.

### D. New module `rust/src/core/output/android_surface.rs` (`#[cfg(target_os = "android")]`)

```rust
pub struct AndroidVideoOutput {
    pub texture_id: i64,
    sendable_texture: Arc<SendableTexture<NativeWindow>>, // keeps registry entry alive
    native_window: SendableNativeWindow,                  // unsafe Send+Sync wrapper
}
```

- `new(engine_handle) -> Result<Self>`: on platform main thread
  (`invoke_on_platform_main_thread`) create `Texture::<NativeWindow>::new`, grab
  `texture.get()` + `texture.id()`, convert to sendable.
- `window_handle(&self) -> usize` — for `VideoOverlay::set_window_handle`.
- `set_video_size(&self, w, h)` — `ANativeWindow_setBuffersGeometry(win, w, h, 0)`.
- `Drop`/explicit `destroy()`: pipeline must be NULL first, then drop the sendable
  texture on the platform main thread (same pattern as today's `finalize_texture`).
- Impl `FlutterTextureSession` (mark_frame_available = no-op, SurfaceTexture handles it).

Shared helper (used by both inputs):

```rust
/// glimagesink caps → OriginVideoSize event + buffer geometry update
fn install_video_size_watch(sink: &gst::Element, output: Arc<AndroidVideoOutput>, common: ...)
```
implemented as a sink-pad probe on CAPS events (`gst_video::VideoInfo::from_caps` works
for GLMemory caps too — width/height are regular fields).

### E. `rust/src/core/input/wsc_rtp.rs` — Android pipeline

- `build_pipeline_str` (cfg android):
  `appsrc name=src caps=... format=time is-live=true ! rtpjitterbuffer !
   {rtpXdepay ! Xparse} ! decodebin3 ! glimagesink name=sink sync=false`
  (non-Android string unchanged; keep explicit avdec there for now.)
- `execute()`: cfg-split texture creation — Android uses `AndroidVideoOutput`
  instead of `PayloadHolder`+`Texture::new_with_provider`.
- `run_session_loop()`: on Android skip the appsink callback block; instead
  `pipeline.by_name("sink")` → `set_window_handle(output.window_handle())` (unsafe,
  via `gst_video::prelude::VideoOverlayExtManual`) **before** `set_state(Playing)`,
  and install the video-size watch (replaces the appsink-based OriginVideoSize emit).
- Everything else (WS/UDP feed, DISCONT handling, bus watch, reconnect loop) is
  untouched. Note: the reconnect loop creates a **new pipeline per connection** but
  reuses the texture — the new glimagesink gets the same window handle; EGL surface
  is recreated per pipeline, which is fine.

### F. `rust/src/core/input/playbin.rs` — Android sink

- cfg android: create `glimagesink` via `ElementFactory`, `set_window_handle`, set as
  `video-sink` on playbin3 (instead of the RGBA appsink), install video-size watch.
- Audio path unchanged (autoaudiosink → openslessink, already registered).

### G. Ship the Java class — add an Android plugin platform

1. `android/build.gradle` — minimal library module, `namespace
   "com.nrbnlulu.flutter_realtime_player"`, no dependencies.
2. `android/src/main/AndroidManifest.xml` — empty manifest.
3. `android/src/main/java/org/freedesktop/gstreamer/androidmedia/GstAmcOnFrameAvailableListener.java`
   — verbatim copy from
   `$GSTREAMER_ROOT_ANDROID/arm64/share/gst-android/ndk-build/androidmedia/` (LGPL
   header retained).
4. `android/src/main/java/com/nrbnlulu/flutter_realtime_player/FlutterRealtimePlayerPlugin.java`
   — no-op `FlutterPlugin` (required for a `pluginClass` declaration).
5. `pubspec.yaml`:

```yaml
flutter:
    plugin:
        platforms:
            android:
                package: com.nrbnlulu.flutter_realtime_player
                pluginClass: FlutterRealtimePlayerPlugin
```

(Android-only plugin declaration doesn't break other platforms — they just skip it.)

## Failure/fallback behavior (by design, no extra code)

| Failure | Result |
|---|---|
| androidmedia JNI init fails | plugin not registered → decodebin3 picks avdec (SW) → glimagesink uploads raw frames. Still GPU texture, one upload, no appsink copies. |
| Java listener class missing | amcviddec outputs ByteBuffer (HW decode, CPU copy) → glupload. |
| No HW codec for format (VP8/9 on some devices) | decodebin3 falls back by rank. |

## Verification

1. `source ~/cross_build.env && cd rust && cargo ndk -t arm64-v8a -P 35 build` — links.
2. `cargo check` for host (Linux) — non-Android paths unchanged.
3. Run example on a device: `flutter run` in `example/` (Android).
   - logcat: confirm no "Failed to register GStreamer plugin: androidmedia";
     `GST_DEBUG=amc*:4,glimagesink:4` via `GST_DEBUG` env or `gst::debug_set_threshold`.
   - Confirm caps on glimagesink sink pad contain `memory:GLMemory` +
     `texture-target=external-oes` (true zero-copy) rather than `video/x-raw`.
   - Visual: video renders, OriginVideoSize event still reaches Dart, aspect correct,
     resolution not stuck at 1×1/640×480-blur.
   - Lifecycle: dispose player → texture released without crash (pipeline → NULL before
     texture drop on main thread); reconnect path (kill server, let it retry).
4. Performance sanity: CPU usage while playing 1080p H.265 should drop dramatically vs
   the avdec path (was: decode + convert + 2 memcpys per frame).

## Open questions / later work

- The DISCONT+IDR decoder-reinit trick (`wsc_rtp.rs:441-445`) is avdec-specific;
  verify amcviddec handles mid-stream SDP/resolution changes (it does its own
  format-change handling via MediaCodec; if not, we may need to force decodebin3
  reconfiguration on SDP update).
- Multiple concurrent sessions: each pipeline creates its own EGL context/display —
  works, but if we ever want context sharing, set a common `GstGLDisplay` via
  `gst.gl.GLDisplay` context on the pipelines.
- iOS/macOS zero-copy (`BoxedIOSurface`) and Windows (`BoxedTextureDescriptor` D3D11)
  can follow the same output-abstraction seam introduced in step D.
- Consider `glsinkbin sink=fakesink`-style latency tuning later; `sync=false` is kept
  to match current realtime behavior.
