package dev.onetboss.realtimeplayer;

import io.flutter.embedding.engine.plugins.FlutterPlugin;

/** Supplies GStreamer's Java runtime before the Rust plugin registers codecs. */
public final class FlutterRealtimePlayerPlugin implements FlutterPlugin {
    @Override
    public void onAttachedToEngine(FlutterPluginBinding binding) {
        // The Rust shell has already loaded the application library. Its native
        // entry points share this class loader; there is no second library load.
        if (!nativeInitialize(FlutterRealtimePlayerPlugin.class.getClassLoader())) {
            throw new IllegalStateException("Could not initialize GStreamer's Java runtime");
        }
    }

    @Override
    public void onDetachedFromEngine(FlutterPluginBinding binding) {
        // The VM/class loader is process-scoped and shared by native sessions.
    }

    private static native boolean nativeInitialize(ClassLoader loader);
}
