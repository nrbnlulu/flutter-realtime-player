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
  final List<_StreamConfig> _streams = [
    _StreamConfig(
      urlController: TextEditingController(text: "rtsp://your_stream_url_here"),
      ffmpegOptionControllers: [
        MapEntry(TextEditingController(), TextEditingController()),
      ],
      isStreaming: false,
      autoRestart: false,
    ),
  ];

  @override
  void dispose() {
    for (final stream in _streams) {
      stream.urlController.dispose();
      for (final entry in stream.ffmpegOptionControllers) {
        entry.key.dispose();
        entry.value.dispose();
      }
    }
    super.dispose();
  }

  void _toggleStream(int index) {
    setState(() {
      _streams[index].isStreaming = !_streams[index].isStreaming;
    });
  }

  void _addStream() {
    setState(() {
      _streams.add(
        _StreamConfig(
          urlController: TextEditingController(),
          ffmpegOptionControllers: [
            MapEntry(TextEditingController(), TextEditingController()),
          ],
          isStreaming: false,
          autoRestart: false,
        ),
      );
    });
  }

  void _removeStream(int index) {
    setState(() {
      if (_streams.length > 1) {
        final stream = _streams.removeAt(index);
        stream.urlController.dispose();
        for (final entry in stream.ffmpegOptionControllers) {
          entry.key.dispose();
          entry.value.dispose();
        }
      }
    });
  }

  void _addFfmpegOptionField(int streamIdx) {
    setState(() {
      _streams[streamIdx].ffmpegOptionControllers.add(
        MapEntry(TextEditingController(), TextEditingController()),
      );
    });
  }

  void _removeFfmpegOptionField(int streamIdx, int optionIdx) {
    setState(() {
      final controllers = _streams[streamIdx].ffmpegOptionControllers;
      if (controllers.length > 1) {
        controllers[optionIdx].key.dispose();
        controllers[optionIdx].value.dispose();
        controllers.removeAt(optionIdx);
      }
    });
  }

  Map<String, String> _collectFfmpegOptions(
    List<MapEntry<TextEditingController, TextEditingController>> controllers,
  ) {
    final Map<String, String> options = {};
    for (final entry in controllers) {
      final key = entry.key.text.trim();
      final value = entry.value.text.trim();
      if (key.isNotEmpty) {
        options[key] = value;
      }
    }
    return options;
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: GridView.builder(
            padding: const EdgeInsets.all(8),
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              crossAxisSpacing: 8,
              mainAxisSpacing: 8,
              childAspectRatio: 1.5,
            ),
            itemCount: _streams.length,
            addAutomaticKeepAlives: true,
            itemBuilder: (context, streamIdx) {
              final stream = _streams[streamIdx];
              return _StreamGridItem(
                key: ValueKey(stream),
                stream: stream,
                streamIdx: streamIdx,
                toggleStream: _toggleStream,
                removeStream: _removeStream,
                addFfmpegOptionField: _addFfmpegOptionField,
                removeFfmpegOptionField: _removeFfmpegOptionField,
                collectFfmpegOptions: _collectFfmpegOptions,
              );
            },
          ),
        ),
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: ElevatedButton.icon(
            icon: const Icon(Icons.add),
            label: const Text('Add Stream'),
            onPressed: _addStream,
          ),
        ),
      ],
    );
  }
}

class _StreamConfig {
  final TextEditingController urlController;
  final List<MapEntry<TextEditingController, TextEditingController>>
  ffmpegOptionControllers;
  bool isStreaming;
  bool autoRestart;

  _StreamConfig({
    required this.urlController,
    required this.ffmpegOptionControllers,
    required this.isStreaming,
    required this.autoRestart,
  });
}

class _StreamGridItem extends StatefulWidget {
  final _StreamConfig stream;
  final int streamIdx;
  final void Function(int) toggleStream;
  final void Function(int) removeStream;
  final void Function(int) addFfmpegOptionField;
  final void Function(int, int) removeFfmpegOptionField;
  final Map<String, String> Function(
    List<MapEntry<TextEditingController, TextEditingController>>,
  )
  collectFfmpegOptions;

  const _StreamGridItem({
    super.key,
    required this.stream,
    required this.streamIdx,
    required this.toggleStream,
    required this.removeStream,
    required this.addFfmpegOptionField,
    required this.removeFfmpegOptionField,
    required this.collectFfmpegOptions,
  });

  @override
  State<_StreamGridItem> createState() => _StreamGridItemState();
}

class _StreamGridItemState extends State<_StreamGridItem>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final stream = widget.stream;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Column(
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: stream.urlController,
                    decoration: const InputDecoration(
                      labelText: 'Stream URL',
                      border: OutlineInputBorder(),
                    ),
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close),
                  tooltip: 'Remove this stream',
                  onPressed: () => widget.removeStream(widget.streamIdx),
                ),
              ],
            ),
            const SizedBox(height: 8),
            SizedBox(
              height: 120,
              child: SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'FFmpeg Options:',
                      style: TextStyle(fontWeight: FontWeight.bold),
                    ),
                    ...stream.ffmpegOptionControllers.asMap().entries.map((
                      entry,
                    ) {
                      final idx = entry.key;
                      final controllers = entry.value;
                      return Row(
                        children: [
                          Expanded(
                            child: TextField(
                              controller: controllers.key,
                              decoration: const InputDecoration(
                                labelText: 'Key',
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: TextField(
                              controller: controllers.value,
                              decoration: const InputDecoration(
                                labelText: 'Value',
                              ),
                            ),
                          ),
                          IconButton(
                            icon: const Icon(Icons.remove_circle_outline),
                            onPressed:
                                () => widget.removeFfmpegOptionField(
                                  widget.streamIdx,
                                  idx,
                                ),
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
                        onPressed:
                            () => widget.addFfmpegOptionField(widget.streamIdx),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                ElevatedButton(
                  onPressed: () => widget.toggleStream(widget.streamIdx),
                  child: Text(
                    stream.isStreaming ? 'Stop Stream' : 'Start Stream',
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            SwitchListTile(
              title: const Text('Auto Restart'),
              value: stream.autoRestart,
              onChanged: (value) {
                setState(() {
                  stream.autoRestart = value;
                });
              },
            ),
            const SizedBox(height: 8),
            stream.isStreaming
                ? SizedBox(
                  width: double.infinity,
                  height: 240,
                  child: VideoPlayer.fromConfig(
                    url: stream.urlController.text,
                    autoRestart: stream.autoRestart,
                    ffmpegOptions: widget.collectFfmpegOptions(
                      stream.ffmpegOptionControllers,
                    ),
                  ),
                )
                : const Text('Stream is stopped'),
          ],
        ),
      ),
    );
  }
}
