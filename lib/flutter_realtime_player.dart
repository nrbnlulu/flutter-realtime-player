library;

import 'src/lifecycle_native.dart'
    if (dart.library.js_interop) 'src/lifecycle_web.dart'
    as lifecycle;

export './rust/core/types.dart';
export './video_player.dart' show VideoController, VideoPlayer;

Future<void> init() => lifecycle.init();

Future<void> dispose() => lifecycle.dispose();
