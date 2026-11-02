library;

import 'package:flutter_realtime_player/rust/frb_generated.dart' as rlib_gen;
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
export './rust/core/types.dart';
export './video_player.dart' show VideoController, VideoPlayer;

/// The Rust shell compiles this plugin's crate directly into the runner
/// executable rather than a standalone dynamic library, so its FRB symbols
/// must be resolved from the current process instead of `dlopen`ed.
Future<void> init() async {
  await rlib_gen.RustLib.init(
    externalLibrary: ExternalLibrary.process(iKnowHowToUseIt: true),
  );
}

Future<void> dispose() async {
  await rlib.destroyAllSessions();
}
