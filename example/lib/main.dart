import 'package:flutter/material.dart';

import 'package:flutter_realtime_player/video_player.dart';

import 'package:window_manager/window_manager.dart';
import 'package:flutter_realtime_player/flutter_realtime_player.dart' as fl_gst;


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
        body: StreamControlWidget(),
      ),
    );   
  }
  
  
  
}

class StreamControlWidget extends StatefulWidget {
  const StreamControlWidget({super.key});

  @override
  StreamControlWidgetState createState() => StreamControlWidgetState();
  
}

class StreamControlWidgetState extends State<StreamControlWidget> {
  bool _isStreaming = true;

  
  void _toggleStream() {
      debugPrint("Toggle stream");
    setState(() {
      _isStreaming = !_isStreaming;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        ElevatedButton(
          onPressed: _toggleStream,
          child: Text(_isStreaming ? 'Stop Stream' : 'Start Stream'),
        ),
        const SizedBox(height: 20),
        _isStreaming
            ? SizedBox(
                width: double.infinity,
                height: 300,
                child: VideoPlayer.fromConfig(
                  url: "rtsp://admin:camteam524@185.183.188.131:554/ch_100",
                ),
              )
            : const Text('Stream is stopped'),
      ],
      
      
    );
  }
}

