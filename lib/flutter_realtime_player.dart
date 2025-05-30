clibrary;

import 'package:flutter_realtime_player/rust/frb_generated.dart' as rlib_gen;
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'dart:ffi' as ffi;
export './rust/core/types.dart';
export './video_player.dart' show VideoController, VideoPlayer;
import 'package:irondash_engine_context/irondash_engine_context.dart';

Future<void> init() async {
  await rlib_gen.RustLib.init();
  rlib.flutterRealtimePlayerInit(
    ffiPtr: ffi.NativeApi.initializeApiDLData.address,
  );
}

Future<void> dispose() async {
  final engineHandle = await EngineContext.instance.getEngineHandle();
  await rlib.destroyEngineStreams(engineId: engineHandle);
}
