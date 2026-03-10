import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/flutter_realtime_player.dart' as fl_gst;
import 'package:flutter_realtime_player/rust/core/types.dart'
    show
        WscRtpSessionConfig,
        VideoConfig,
        VideoConfig_WscRtp,
        PlaybinConfig,
        VideoConfig_Playbin;
import 'wsc_rtp_player.dart';
import 'wsc_rtp_seek_demo.dart';
import 'playbin_player.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await fl_gst.init();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: DefaultTabController(
        length: 2,
        child: Scaffold(
          appBar: AppBar(
            title: const Text('Flutter Realtime Player'),
            bottom: const TabBar(
              tabs: [Tab(text: 'Streams'), Tab(text: 'WSC-RTP Seek')],
            ),
          ),
          body: const TabBarView(
            children: [StreamControlWidget(), WscRtpSeekDemo()],
          ),
        ),
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
      urlController: TextEditingController(
        text:
            "https://www.freedesktop.org/software/gstreamer-sdk/data/media/sintel_trailer-480p.webm",
      ),
      wscRtpBaseUrlController: TextEditingController(
        text: "https://your_backend_here",
      ),
      wscRtpSourceIdController: TextEditingController(text: "source-id"),
      ffmpegOptionControllers: [
        MapEntry(TextEditingController(), TextEditingController()),
      ],
      isStreaming: false,
      autoRestart: false,
      useWscRtp: false,
      usePlaybin: false,
      forceWebsocketTransport: false,
      mute: false,
    ),
  ];

  @override
  void dispose() {
    for (final stream in _streams) {
      stream.urlController.dispose();
      stream.wscRtpBaseUrlController.dispose();
      stream.wscRtpSourceIdController.dispose();
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
          wscRtpBaseUrlController: TextEditingController(),
          wscRtpSourceIdController: TextEditingController(),
          ffmpegOptionControllers: [
            MapEntry(TextEditingController(), TextEditingController()),
          ],
          isStreaming: false,
          autoRestart: false,
          useWscRtp: false,
          usePlaybin: false,
          forceWebsocketTransport: false,
          mute: false,
        ),
      );
    });
  }

  void _removeStream(int index) {
    setState(() {
      if (_streams.length > 1) {
        final stream = _streams.removeAt(index);
        stream.urlController.dispose();
        stream.wscRtpBaseUrlController.dispose();
        stream.wscRtpSourceIdController.dispose();
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
              childAspectRatio: 1, // Square shape to allow flexible sizing
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
  final TextEditingController wscRtpBaseUrlController;
  final TextEditingController wscRtpSourceIdController;
  final List<MapEntry<TextEditingController, TextEditingController>>
  ffmpegOptionControllers;
  bool isStreaming;
  bool autoRestart;
  bool useWscRtp;
  bool usePlaybin;
  bool forceWebsocketTransport;
  bool mute;

  _StreamConfig({
    required this.urlController,
    required this.wscRtpBaseUrlController,
    required this.wscRtpSourceIdController,
    required this.ffmpegOptionControllers,
    required this.isStreaming,
    required this.autoRestart,
    required this.useWscRtp,
    this.forceWebsocketTransport = false,
    this.usePlaybin = false,
    this.mute = false,
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
  final GlobalKey<_VideoPlayerWithControlsState> _playerKey = GlobalKey();

  @override
  bool get wantKeepAlive => true;

  Future<void> _stopStream() async {
    await _playerKey.currentState?.stop();
    if (mounted) {
      widget.toggleStream(widget.streamIdx);
    }
  }

  Future<void> _removeStream() async {
    await _playerKey.currentState?.stop();
    if (mounted) {
      widget.removeStream(widget.streamIdx);
    }
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final stream = widget.stream;

    if (stream.isStreaming) {
      // When streaming, show only the video covering the whole grid item
      return Card(
        elevation: 4.0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12.0),
        ),
        child: Stack(
          fit: StackFit.expand,
          children: [
            // Full-screen video player
            _VideoPlayerWithControls(
              key: _playerKey,
              config:
                  stream.usePlaybin
                      ? VideoConfig.playbin(
                        PlaybinConfig(
                          uri: stream.urlController.text,
                          mute: stream.mute,
                        ),
                      )
                      : VideoConfig.wscRtp(
                        WscRtpSessionConfig(
                          baseUrl: stream.wscRtpBaseUrlController.text,
                          sourceId: stream.wscRtpSourceIdController.text,
                          forceWebsocketTransport:
                              stream.forceWebsocketTransport,
                          autoRestart: stream.autoRestart,
                        ),
                      ),
            ),
            // Overlay controls
            Positioned(
              top: 8,
              right: 8,
              child: Container(
                decoration: BoxDecoration(
                  color: Colors.black.withValues(alpha: 0.5),
                  borderRadius: BorderRadius.circular(20),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // Stop Stream Button
                    IconButton(
                      icon: const Icon(Icons.stop, color: Colors.white),
                      tooltip: 'Stop Stream',
                      onPressed: _stopStream,
                    ),
                    // Remove Stream Button
                    IconButton(
                      icon: const Icon(Icons.delete, color: Colors.white),
                      tooltip: 'Remove Stream',
                      onPressed: _removeStream,
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      );
    } else {
      // When not streaming, show full configuration interface
      return Card(
        elevation: 4.0,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12.0),
        ),
        child: Padding(
          padding: const EdgeInsets.all(16.0),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  Switch(
                    value: stream.useWscRtp,
                    onChanged: (value) {
                      setState(() {
                        stream.useWscRtp = value;
                        if (value) stream.usePlaybin = false;
                      });
                    },
                  ),
                  const Text('Use WSC-RTP'),
                  const SizedBox(width: 8),
                  Switch(
                    value: stream.usePlaybin,
                    onChanged: (value) {
                      setState(() {
                        stream.usePlaybin = value;
                        if (value) stream.useWscRtp = false;
                      });
                    },
                  ),
                  const Text('Use Playbin'),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close),
                    tooltip: 'Remove this stream',
                    onPressed: () => widget.removeStream(widget.streamIdx),
                  ),
                ],
              ),
              const SizedBox(height: 12.0),
              if (!stream.useWscRtp && !stream.usePlaybin)
                TextField(
                  controller: stream.urlController,
                  decoration: const InputDecoration(
                    labelText: 'Stream URL (e.g., rtsp://...)',
                    border: OutlineInputBorder(),
                  ),
                ),
              if (stream.usePlaybin) ...[
                TextField(
                  controller: stream.urlController,
                  decoration: const InputDecoration(
                    labelText: 'Media URI (file://, http://, rtsp://, etc.)',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 8.0),
                Row(
                  children: [
                    Switch(
                      value: stream.mute,
                      onChanged: (value) {
                        setState(() {
                          stream.mute = value;
                        });
                      },
                    ),
                    const Text('Mute Audio'),
                  ],
                ),
              ],
              if (stream.useWscRtp) ...[
                TextField(
                  controller: stream.wscRtpBaseUrlController,
                  decoration: const InputDecoration(
                    labelText: 'WSC-RTP Base URL (e.g., https://api.example)',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12.0),
                TextField(
                  controller: stream.wscRtpSourceIdController,
                  decoration: const InputDecoration(
                    labelText: 'WSC-RTP Source ID',
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 8.0),
                Row(
                  children: [
                    Switch(
                      value: stream.forceWebsocketTransport,
                      onChanged: (value) {
                        setState(() {
                          stream.forceWebsocketTransport = value;
                        });
                      },
                    ),
                    const Text('Force WebSocket Transport'),
                  ],
                ),
              ],
              const SizedBox(height: 16.0),
              // FFmpeg options expansion
              ExpansionTile(
                title: const Text('FFmpeg Options'),
                childrenPadding: const EdgeInsets.only(
                  left: 16.0,
                  right: 16.0,
                  bottom: 16.0,
                ),
                children: [
                  ...stream.ffmpegOptionControllers.asMap().entries.map((
                    entry,
                  ) {
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
              // Video player area - takes maximum space
              Expanded(
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    // Video player
                    stream.isStreaming
                        ? _VideoPlayerWithControls(
                          config: VideoConfig.wscRtp(
                            WscRtpSessionConfig(
                              baseUrl: stream.wscRtpBaseUrlController.text,
                              sourceId: stream.wscRtpSourceIdController.text,
                              forceWebsocketTransport:
                                  stream.forceWebsocketTransport,
                              autoRestart: stream.autoRestart,
                            ),
                          ),
                        )
                        : const Center(
                          child: Text(
                            'Stream is stopped',
                            style: TextStyle(
                              fontSize: 16.0,
                              color: Colors.grey,
                            ),
                          ),
                        ),
                    // Overlay buttons when video is playing
                    if (stream.isStreaming)
                      Positioned(
                        top: 8,
                        right: 8,
                        child: Container(
                          decoration: BoxDecoration(
                            color: Colors.black.withValues(alpha: 0.5),
                            borderRadius: BorderRadius.circular(20),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              // Stop Stream Button
                              IconButton(
                                icon: const Icon(
                                  Icons.stop,
                                  color: Colors.white,
                                ),
                                tooltip: 'Stop Stream',
                                onPressed: _stopStream,
                              ),
                              // Remove Stream Button
                              IconButton(
                                icon: const Icon(
                                  Icons.delete,
                                  color: Colors.white,
                                ),
                                tooltip: 'Remove Stream',
                                onPressed: _removeStream,
                              ),
                            ],
                          ),
                        ),
                      ),
                  ],
                ),
              ),
              // Controls at bottom when not streaming
              if (!stream.isStreaming) const SizedBox(height: 16.0),
              if (!stream.isStreaming)
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    ElevatedButton(
                      onPressed: () => widget.toggleStream(widget.streamIdx),
                      child: const Text('Start Stream'),
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
}

class _VideoPlayerWithControls extends StatefulWidget {
  final VideoConfig config;

  const _VideoPlayerWithControls({super.key, required this.config});

  @override
  State<_VideoPlayerWithControls> createState() =>
      _VideoPlayerWithControlsState();
}

class _VideoPlayerWithControlsState extends State<_VideoPlayerWithControls> {
  // Used by _StreamGridItemState to stop playback before toggling.
  Future<void> stop() async {
    // Player widgets manage their own lifecycle; nothing to do here.
  }

  @override
  Widget build(BuildContext context) {
    return switch (widget.config) {
      VideoConfig_WscRtp(:final field0) => WscRtpPlayerWidget(config: field0),
      VideoConfig_Playbin(:final field0) => PlaybinPlayerWidget(config: field0),
    };
  }
}
