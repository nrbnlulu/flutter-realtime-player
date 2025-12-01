import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/video_player.dart';
import 'package:window_manager/window_manager.dart';
import 'package:flutter_realtime_player/flutter_realtime_player.dart' as fl_gst;
import 'dart:async';
import 'package:flutter_realtime_player/rust/dart_types.dart';

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
                ? _VideoPlayerWithControls(
                    url: stream.urlController.text,
                    autoRestart: stream.autoRestart,
                    ffmpegOptions: widget.collectFfmpegOptions(
                      stream.ffmpegOptionControllers,
                    ),
                  )
                : const Text('Stream is stopped'),
          ],
        ),
      ),
    );
  }
}



class _VideoPlayerWithControls extends StatefulWidget {
  final String url;
  final bool autoRestart;
  final Map<String, String>? ffmpegOptions;

  const _VideoPlayerWithControls({
    required this.url,
    required this.autoRestart,
    this.ffmpegOptions,
  });

  @override
  State<_VideoPlayerWithControls> createState() => _VideoPlayerWithControlsState();
}

class _VideoPlayerWithControlsState extends State<_VideoPlayerWithControls> {
  VideoController? _controller;
  bool _isLoading = true;
  Duration _duration = Duration.zero; // We don't have duration for this simple player
  Duration _position = Duration.zero;
  Timer? _positionTimer;

  @override
  void initState() {
    super.initState();
    _initializeVideo();
  }

  Future<void> _initializeVideo() async {
    final result = await VideoController.create(
      url: widget.url,
      autoRestart: widget.autoRestart,
      ffmpegOptions: widget.ffmpegOptions,
    );
    
    setState(() {
      _controller = result.$1;
      _isLoading = false;
    });

    if (_controller != null) {
      // Start periodic position updates
      _positionTimer = Timer.periodic(const Duration(seconds: 1), (timer) async {
        try {
          final position = await _controller!.getCurrentPosition();
          final state = _controller!.stateBroadcast.value;
          if (state is StreamState_Playing) {
            setState(() {
              _position = position;
            });
          }
        } catch (e) {
          // Handle potential errors
        }
      });
    }
  }

  Future<void> _seekToPosition(double value) async {
    if (_controller != null) {
      final newPosition = Duration(
        milliseconds: (value * 1000).round(),
      );
      await _controller!.seekTo(newPosition);
      setState(() {
        _position = newPosition;
      });
    }
  }

  @override
  void dispose() {
    _positionTimer?.cancel();
    _controller?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_controller == null) {
      return const Center(child: Text('Failed to load video'));
    }

    return Column(
      children: [
        SizedBox(
          width: double.infinity,
          height: 200, // Reduced to make space for controls
          child: VideoPlayer.fromController(
            controller: _controller!,
            autoDispose: false, // Don't auto dispose since we're managing it
          ),
        ),
        // Video controls
        Padding(
          padding: const EdgeInsets.all(8.0),
          child: Column(
            children: [
              // Position slider
              Slider(
                value: _position.inMilliseconds.toDouble(),
                min: 0.0,
                max: _duration.inMilliseconds.toDouble().isNaN 
                    ? 100.0 
                    : _duration.inMilliseconds.toDouble(),
                onChanged: _seekToPosition,
                label: '${_position.inSeconds}s',
              ),
              // Time display
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(_position.toString().split('.')[0]),
                  Text(_duration.toString().split('.')[0]),
                ],
              ),
              // Seek buttons
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  IconButton(
                    icon: const Icon(Icons.replay_10),
                    onPressed: () async {
                      if (_controller != null) {
                        final newPosition = Duration(
                          milliseconds: _position.inMilliseconds - 10000,
                        );
                        if (newPosition.isNegative) {
                          await _seekToPosition(0.0);
                        } else {
                          await _seekToPosition(newPosition.inMilliseconds.toDouble());
                        }
                      }
                    },
                  ),
                  IconButton(
                    icon: const Icon(Icons.forward_10),
                    onPressed: () async {
                      if (_controller != null) {
                        final newPosition = Duration(
                          milliseconds: _position.inMilliseconds + 10000,
                        );
                        await _seekToPosition(newPosition.inMilliseconds.toDouble());
                      }
                    },
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}