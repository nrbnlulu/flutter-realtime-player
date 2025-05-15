import 'package:flutter/material.dart';

import 'package:flutter_gstreamer/video_player.dart';

import 'package:window_manager/window_manager.dart';
import 'package:flutter_gstreamer/flutter_gstreamer.dart' as fl_gst;


Future<void> main() async {
    WidgetsFlutterBinding.ensureInitialized();
    await fl_gst.init();
  await windowManager.ensureInitialized();
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
          child: VideoPlayer.fromConfig(url: "rtsp://admin:camteam524@185.183.188.131:554/ch_100")
        ),
      ),
      
    );
  }
}
