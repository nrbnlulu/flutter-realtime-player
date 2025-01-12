import 'dart:developer';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:my_app/src/rust/api/simple.dart' as rlib;
import 'package:my_app/src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final handle = await EngineContext.instance.getEngineHandle();

  runApp(MyApp(handle: handle));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key, required this.handle});
  final int handle;
  @override
  Widget build(BuildContext context) {
    debugPrint("Isolate.current.debugName: ${Isolate.current.debugName} ${Service.getIsolateId(Isolate.current)}");
    final texture = rlib.createThatTexturePlease(engineHandle: handle);
    print('Texture: $texture');
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('flutter_rust_bridge quickstart')),
        body: Center(
          child: Text(
              'Action: Call Rust `greet("Tom")`\nResult: `${rlib.greet(name: "Tom")}`'),
        ),
      ),
    );
  }
}
