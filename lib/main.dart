import 'dart:ffi';

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
    const baseUrl = "rtsp://admin:tzv12345@192.168.3.3:554/ch_10";
    final List<String> urls = [];
    for (int i = 0; i < 1; i++) {
      final url = "$baseUrl$i";
      urls.add(url);
    }

    return MaterialApp(
        home: Scaffold(
      appBar: AppBar(title: const Text('flutter_rust_bridge quickstart')),
      body: Wrap(
        children: [
          for (final url in urls)
        SizedBox(
          height: 250,
          width: 400,
          child: NewWidget(url: url),
        ),
        ],
      )
    ));
  }
}

class NewWidget extends StatelessWidget {
  final String url;
  const NewWidget({
    required this.url,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 500,
      width: 500,
      child: FutureBuilder(
        future: () async {
          final handle = await EngineContext.instance.getEngineHandle();
          // play demo video
          try{
  final texture = await rlib.getTexture(
                engineHandle: handle,
                uri:
                    "https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_30mb.mp4");

          return texture;
          } catch (e) {
            print(e);

          }
          
        }(),
        builder: (context, snapshot) {
          if (snapshot.connectionState == ConnectionState.done) {
            debugPrint("snapshot.data: ${snapshot.data}");
            return Texture(textureId: snapshot.data!);
          } else {
            return const CircularProgressIndicator();
          }
        },
      ),
    );
  }
}
