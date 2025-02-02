import 'dart:developer';
import 'dart:ffi';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:my_app/src/rust/api/simple.dart' as rlib;
import 'package:my_app/src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await RustLib.init();
  rlib.flutterGstreamerInit(ffiPtr: NativeApi.initializeApiDLData.address);

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
          child: Container(
            height: 500,
            width: 500,
            child: FutureBuilder(
              future: () async {
                final handle = await EngineContext.instance.getEngineHandle();
                // play demo video
                final texture = await rlib.getOpenglTexture(
                    engineHandle: handle,
                    uri:
                        "https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_30mb.mp4");

                return texture;
              }(),
              builder: (context, snapshot) {
                if (snapshot.connectionState == ConnectionState.done) {
                  return Texture(textureId: snapshot.data as int);
                } else {
                  return const CircularProgressIndicator();
                }
              },
            ),
          ),
        ),
      ),
    );
  }
}
