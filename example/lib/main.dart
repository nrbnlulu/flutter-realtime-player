import 'package:flutter/material.dart';


import 'package:flutter_realtime_player/video_player.dart';

import 'package:window_manager/window_manager.dart';
import 'package:flutter_realtime_player/flutter_realtime_player.dart' as fl_gst;
import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'dart:convert';

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
  final TextEditingController _urlController = TextEditingController(
    text: "enter_real.rtsp",
  );
  @override
  void dispose() {
    _urlController.dispose();
    
    super.dispose();
  }

  void _toggleStream() {
    debugPrint("Toggle stream");
    setState(() {
      _isStreaming = !_isStreaming;
    });
  }

  Future<void> _openInNewWindow() async {
    final url = _urlController.text;
    final window = await DesktopMultiWindow.createWindow(
      jsonEncode({'url': url}),
    );
    window
      ..setFrame(const Offset(100, 100) & const Size(800, 600))
      ..setTitle('Stream Window')
      ..show();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        TextField(
          controller: _urlController,
          decoration: const InputDecoration(
            labelText: 'Stream URL',
            border: OutlineInputBorder(),
          ),
        ),
        const SizedBox(height: 10),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            ElevatedButton(
              onPressed: _toggleStream,
              child: Text(_isStreaming ? 'Stop Stream' : 'Start Stream'),
            ),
            const SizedBox(width: 10),
            ElevatedButton(
              onPressed: _openInNewWindow,
              child: const Text('Open in New Window'),
            ),
          ],
        ),
        const SizedBox(height: 20),
        _isStreaming
            ? SizedBox(
              width: double.infinity,
              height: 300,
              child: VideoPlayer.fromConfig(url: _urlController.text),
            )
            : const Text('Stream is stopped'),
      ],
    );
  }
}
