import 'package:flutter/material.dart';
import 'dart:ffi' as ffi;

import 'package:flutter_gstreamer/src/rust/api/simple.dart' as rlib;
import 'package:flutter_gstreamer/src/rust/frb_generated.dart' as rlib_gen;


Future<void> main() async {
  await RustLib.init();
    WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();
  await rlib_gen.RustLib.init();
  final handle = await EngineContext.instance.getEngineHandle();
  rlib.flutterGstreamerInit(ffiPtr: ffi.NativeApi.initializeApiDLData.address);
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('flutter_rust_bridge quickstart')),
        body: Center(
          child: Text(
            'Action: Call Rust `greet("Tom")`\nResult: `${greet(name: "Tom")}`',
          ),
        ),
      ),
    );
  }
}
