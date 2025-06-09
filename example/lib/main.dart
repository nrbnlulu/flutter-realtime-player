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
    text: "rtsp:://your_stream_url_here",
  );

  // FFmpeg options editing
  final List<MapEntry<TextEditingController, TextEditingController>>
  _ffmpegOptionControllers = [
    MapEntry(TextEditingController(), TextEditingController()),
  ];

  @override
  void dispose() {
    _urlController.dispose();
    for (final entry in _ffmpegOptionControllers) {
      entry.key.dispose();
      entry.value.dispose();
    }
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

  Map<String, String> _collectFfmpegOptions() {
    final Map<String, String> options = {};
    for (final entry in _ffmpegOptionControllers) {
      final key = entry.key.text.trim();
      final value = entry.value.text.trim();
      if (key.isNotEmpty) {
        options[key] = value;
      }
    }
    return options;
  }

  void _addFfmpegOptionField() {
    setState(() {
      _ffmpegOptionControllers.add(
        MapEntry(TextEditingController(), TextEditingController()),
      );
    });
  }

  void _removeFfmpegOptionField(int index) {
    setState(() {
      if (_ffmpegOptionControllers.length > 1) {
        _ffmpegOptionControllers[index].key.dispose();
        _ffmpegOptionControllers[index].value.dispose();
        _ffmpegOptionControllers.removeAt(index);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      child: Column(
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

          // FFmpeg options editor
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8.0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'FFmpeg Options:',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                ..._ffmpegOptionControllers.asMap().entries.map((entry) {
                  final idx = entry.key;
                  final controllers = entry.value;
                  return Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: controllers.key,
                          decoration: const InputDecoration(labelText: 'Key'),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: TextField(
                          controller: controllers.value,
                          decoration: const InputDecoration(labelText: 'Value'),
                        ),
                      ),
                      IconButton(
                        icon: const Icon(Icons.remove_circle_outline),
                        onPressed: () => _removeFfmpegOptionField(idx),
                        tooltip: 'Remove option',
                      ),
                    ],
                  );
                }),
                Align(
                  alignment: Alignment.centerLeft,
                  child: TextButton.icon(
                    icon: const Icon(Icons.add),
                    label: const Text('Add Option'),
                    onPressed: _addFfmpegOptionField,
                  ),
                ),
              ],
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
                child: VideoPlayer.fromConfig(
                  url: _urlController.text,
                  ffmpegOptions: _collectFfmpegOptions(),
                ),
              )
              : const Text('Stream is stopped'),
        ],
      ),
    );
  }
}
