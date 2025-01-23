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
    debugPrint("Isolate.current.debugName: ${Isolate.current.debugName} ${Service.getIsolateId(Isolate.current)}");
    final texture = await rlib.getOpenglTexture(engineHandle: handle);
    print('Texture: $texture');
  runApp(MyApp(textureId: texture));
}

class MyApp extends StatelessWidget {
  const MyApp({super.key, required this.textureId});
  final int textureId;
  
  @override
  Widget build(BuildContext context) {

    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(title: const Text('flutter_rust_bridge quickstart')),
        body: Center(
          
          child: Texture(
            textureId: textureId,
          ),
        ),
      ),
    );
  }  
}






