# flutter_gstreamer
Make sure to initialize irondash and this plugin
```dart
// main.dart
import 'dart:ffi' as ffi;

import 'package:my_app/src/rust/api/simple.dart' as rlib;
import 'package:my_app/src/rust/frb_generated.dart' as rlib_gen;


Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  await rlib_gen.RustLib.init();
  final handle = await EngineContext.instance.getEngineHandle();
  rlib.flutterGstreamerInit(ffiPtr: ffi.NativeApi.initializeApiDLData.address);
```