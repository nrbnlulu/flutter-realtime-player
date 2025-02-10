import 'dart:developer';
import 'dart:ffi';
import 'dart:isolate';
import 'package:video_player/video_player.dart';

import 'package:flutter/material.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:my_app/src/rust/api/simple.dart' as rlib;
import 'package:my_app/src/rust/frb_generated.dart';
import 'package:fvp/fvp.dart' as fvp;

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  await RustLib.init();
  rlib.flutterGstreamerInit(ffiPtr: NativeApi.initializeApiDLData.address);
  fvp.registerWith(options: {
    "lowLatency": 1,
  });
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    const baseUrl = "rtsp://admin:tzv12345@192.168.3.3:554/ch_10";
    final List<String> urls = [];
    for (int i = 0; i < 16; i++) {
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
          child: RTSPStream(url: url),
        ),
        ],
      )
    ));
  }
}

class RTSPStream extends StatefulWidget {
  final String url;
  const RTSPStream({
    required this.url,
    super.key,
  });

  @override
  State<RTSPStream> createState() => _RTSPStreamState();
}

class _RTSPStreamState extends State<RTSPStream> {
  late VideoPlayerController _controller;

  @override
  void initState() {
    super.initState();
    _controller = VideoPlayerController.networkUrl(Uri.parse(widget.url),
        formatHint: VideoFormat.hls, videoPlayerOptions: VideoPlayerOptions());
    _controller.addListener(() {
      setState(() {});
    });
    _controller.
    _controller.setLooping(true);
    _controller.initialize().then((_) => setState(() {}));
    _controller.play();
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      alignment: Alignment.bottomCenter,
      children: <Widget>[
        VideoPlayer(_controller),
        VideoProgressIndicator(_controller, allowScrubbing: true),
      ],
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }
}

class NewWidget extends StatelessWidget {
  const NewWidget({
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
    );
  }
}
