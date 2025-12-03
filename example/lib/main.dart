import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart'
    show VideoDimensions;
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
        appBar: AppBar(title: const Text('Flutter Realtime Player')),
        body: const StreamControlWidget(),
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
            padding: const EdgeInsets.all(16.0),
            gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
              crossAxisCount: 2,
              crossAxisSpacing: 16.0,
              mainAxisSpacing: 16.0,
              childAspectRatio:
                  16 / 11, // Adjusted to account for controls below video
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
          padding: const EdgeInsets.all(16.0),
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
      elevation: 4.0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12.0)),
      child: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: stream.urlController,
                    decoration: const InputDecoration(
                      labelText: 'Stream URL (e.g., rtsp://...)',
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
            const SizedBox(height: 16.0),
            ExpansionTile(
              title: const Text('FFmpeg Options'),
              childrenPadding: const EdgeInsets.only(
                left: 16.0,
                right: 16.0,
                bottom: 16.0,
              ),
              children: [
                ...stream.ffmpegOptionControllers.asMap().entries.map((entry) {
                  final idx = entry.key;
                  final controllers = entry.value;
                  return Padding(
                    padding: const EdgeInsets.only(bottom: 8.0),
                    child: Row(
                      children: [
                        Expanded(
                          child: TextField(
                            controller: controllers.key,
                            decoration: const InputDecoration(
                              labelText: 'Option Key',
                              border: OutlineInputBorder(),
                            ),
                          ),
                        ),
                        const SizedBox(width: 8.0),
                        Expanded(
                          child: TextField(
                            controller: controllers.value,
                            decoration: const InputDecoration(
                              labelText: 'Option Value',
                              border: OutlineInputBorder(),
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
                    ),
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
            const SizedBox(height: 16.0),
            Expanded(
              child:
                  stream.isStreaming
                      ? _VideoPlayerWithControls(
                        url: stream.urlController.text,
                        autoRestart: stream.autoRestart,
                        ffmpegOptions: widget.collectFfmpegOptions(
                          stream.ffmpegOptionControllers,
                        ),
                      )
                      : const Center(
                        child: Text(
                          'Stream is stopped',
                          style: TextStyle(fontSize: 16.0, color: Colors.grey),
                        ),
                      ),
            ),
            const SizedBox(height: 16.0),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                ElevatedButton(
                  onPressed: () => widget.toggleStream(widget.streamIdx),
                  child: Text(
                    stream.isStreaming ? 'Stop Stream' : 'Start Stream',
                  ),
                ),
                Row(
                  children: [
                    Switch(
                      value: stream.autoRestart,
                      onChanged: (value) {
                        setState(() {
                          stream.autoRestart = value;
                        });
                      },
                    ),
                    const Text('Auto Restart'),
                  ],
                ),
              ],
            ),
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
  State<_VideoPlayerWithControls> createState() =>
      _VideoPlayerWithControlsState();
}

class _VideoPlayerWithControlsState extends State<_VideoPlayerWithControls> {
  VideoController? _controller;
  bool _isLoading = true;
  Duration _position = Duration.zero;
  Duration _duration = Duration.zero;
  Timer? _positionTimer;
  bool _isSeeking = false;
  bool _isSeekable = false;
  double _currentStreamTime = 0.0; // Stream time in seconds
  final TextEditingController _iso8601Controller = TextEditingController();

  @override
  void initState() {
    super.initState();
    _iso8601Controller.text = DateTime.now().toIso8601String();
    _initializeVideo();
  }

  Future<void> _initializeVideo() async {
    // Use a standard HD resolution that maintains 16:9 aspect ratio
    final dimensions = const VideoDimensions(
      width: 1280,
      height: 720,
    ); // 16:9 aspect ratio for HD resolution

    final result = await VideoController.create(
      url: widget.url,
      dimensions: dimensions,
      autoRestart: widget.autoRestart,
      ffmpegOptions: widget.ffmpegOptions,
    );

    setState(() {
      _controller = result.$1;
      _isLoading = false;
    });

    if (_controller != null) {
      _controller!.stateBroadcast.listen((state) {
        if (state is StreamState_Playing) {
          setState(() {
            _isSeekable = state.seekable;
          });
        }
      });

      // Subscribe to stream time updates
      try {
        _controller!.timeBroadcast
            ?.listen((time) {
              setState(() {
                _currentStreamTime = time;
              });
            })
            .onError((error) {
              debugPrint('Time broadcast error: $error');
            });
      } catch (e) {
        debugPrint('Time broadcast not available: $e');
      }

      // Start periodic position updates
      _positionTimer = Timer.periodic(const Duration(seconds: 1), (
        timer,
      ) async {
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
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Expanded(
          child: AspectRatio(
            aspectRatio: 16 / 9, // Standard video aspect ratio
            child: VideoPlayer.fromController(
              controller: _controller!,
              autoDispose: false, // Don't auto dispose since we're managing it
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(top: 8.0),
          child: Column(
            children: [
              // Seek bar - only show if the stream is seekable
              if (_isSeekable)
                Slider(
                  value:
                      _isSeeking
                          ? _position.inMilliseconds.toDouble()
                          : _position.inMilliseconds.toDouble(),
                  onChanged: (value) {
                    setState(() {
                      _isSeeking = true;
                      _position = Duration(milliseconds: value.round());
                    });
                  },
                  onChangeEnd: (value) async {
                    if (_controller != null) {
                      await _controller!.seekTo(
                        Duration(milliseconds: value.round()),
                      );
                    }
                    setState(() {
                      _isSeeking = false;
                      _position = Duration(milliseconds: value.round());
                    });
                  },
                  min: 0.0,
                  max:
                      _isSeekable && _duration.inMilliseconds.toDouble() > 0
                          ? _duration.inMilliseconds.toDouble()
                          : _position.inMilliseconds.toDouble() > 0
                          ? _position.inMilliseconds.toDouble() *
                              2 // Use double the current position as max if duration is unknown but we've started playing
                          : 60000.0, // 60 seconds fallback for unknown duration
                  label: _formatDuration(_position),
                ),
              // Time display with stream time
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  // Show current position and stream time
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _formatDuration(_position),
                        style: const TextStyle(color: Colors.grey),
                      ),
                      if (_currentStreamTime >= 0)
                        Text(
                          'Stream Time: ${_formatDuration(Duration(seconds: _currentStreamTime.floor()))}',
                          style: const TextStyle(
                            color: Colors.blue,
                            fontSize: 12,
                          ),
                        ),
                    ],
                  ),
                  Text(
                    _isSeekable && _duration.inMilliseconds > 0
                        ? _formatDuration(_duration)
                        : '--:--:--',
                    style: const TextStyle(color: Colors.grey),
                  ),
                ],
              ),
              if (!_isSeekable)
                const Text(
                  'Stream is not seekable',
                  style: TextStyle(color: Colors.grey, fontSize: 12),
                ),
              // HLS/X-PROGRAM-DATE-TIME seeking controls for streams with temporal metadata
              if (_isSeekable)
                Column(
                  children: [
                    const SizedBox(height: 8),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                      children: [
                        // Seek backward button
                        ElevatedButton.icon(
                          icon: const Icon(Icons.replay_5),
                          label: const Text('-5s'),
                          onPressed: () async {
                            if (_controller != null) {
                              try {
                                final currentPos =
                                    await _controller!.getCurrentPosition();
                                final newPos = Duration(
                                  seconds:
                                      (currentPos.inSeconds - 5).compareTo(0) >
                                              0
                                          ? currentPos.inSeconds - 5
                                          : 0,
                                );
                                await _controller!.seekTo(newPos);
                              } catch (e) {
                                debugPrint('Error seeking backward: $e');
                              }
                            }
                          },
                        ),
                        // ISO 8601 time picker for absolute seeking
                        Expanded(
                          flex: 2,
                          child: TextField(
                            controller: _iso8601Controller,
                            decoration: const InputDecoration(
                              hintText:
                                  'ISO 8601 time (e.g., 2025-12-03T07:00:00Z)',
                              border: OutlineInputBorder(),
                              contentPadding: EdgeInsets.symmetric(
                                horizontal: 8,
                              ),
                            ),
                          ),
                        ),
                        // Seek to ISO 8601 time button
                        ElevatedButton(
                          onPressed: () async {
                            if (_controller != null &&
                                _iso8601Controller.text.isNotEmpty) {
                              try {
                                await _controller!.seekToISO8601(
                                  _iso8601Controller.text,
                                );
                              } catch (e) {
                                debugPrint('Error seeking to ISO time: $e');
                              }
                            }
                          },
                          child: const Text('Seek'),
                        ),
                        // Seek forward button
                        ElevatedButton.icon(
                          icon: const Icon(Icons.forward_5),
                          label: const Text('+5s'),
                          onPressed: () async {
                            debugPrint("trying to seek forward");
                            if (_controller != null) {
                              try {
                                final currentPos =
                                    await _controller!.getCurrentPosition();
                                final newPos = Duration(
                                  seconds: currentPos.inSeconds + 5,
                                );
                                await _controller!.seekTo(newPos);
                              } catch (e) {
                                debugPrint('Error seeking forward: $e');
                              }
                            }
                          },
                        ),
                      ],
                    ),
                  ],
                ),
            ],
          ),
        ),
      ],
    );
  }

  String _formatDuration(Duration duration) {
    String twoDigits(int n) => n.toString().padLeft(2, "0");
    String twoDigitMinutes = twoDigits(duration.inMinutes.remainder(60));
    String twoDigitSeconds = twoDigits(duration.inSeconds.remainder(60));
    return "${twoDigits(duration.inHours)}:$twoDigitMinutes:$twoDigitSeconds";
  }
}
