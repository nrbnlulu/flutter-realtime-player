import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart'
    show PlaybinConfig, VideoConfig;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:flutter_realtime_player/video_player.dart';

/// A self-contained Playbin player for standard media URIs.
/// Handles file://, http://, https://, rtsp://, and other GStreamer-supported URIs.
class PlaybinPlayerWidget extends StatefulWidget {
  final PlaybinConfig config;

  const PlaybinPlayerWidget({super.key, required this.config});

  @override
  State<PlaybinPlayerWidget> createState() => _PlaybinPlayerWidgetState();
}

class _PlaybinPlayerWidgetState extends State<PlaybinPlayerWidget> {
  VideoController? _controller;
  bool _isLoading = true;
  String? _error;
  double _aspectRatio = 16 / 9;
  bool _isPlaying = false;
  Duration _position = Duration.zero;
  final Duration _duration = Duration.zero;

  Timer? _positionTicker;
  StreamSubscription<StreamEvent>? _eventsSub;

  @override
  void initState() {
    super.initState();
    _connect();
  }

  @override
  void dispose() {
    _positionTicker?.cancel();
    _eventsSub?.cancel();
    _controller?.dispose();
    super.dispose();
  }

  Future<void> _connect() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });

    final result = await VideoController.create(
      config: VideoConfig.playbin(widget.config),
    );

    if (!mounted) return;

    if (result.$1 == null) {
      setState(() {
        _error = result.$2 ?? 'Failed to create session';
        _isLoading = false;
      });
      return;
    }

    _controller = result.$1;
    setState(() {
      _isLoading = false;
      _isPlaying = true;
    });

    _startEventListener();
    _startPositionTicker();
  }

  void _startEventListener() {
    _eventsSub?.cancel();
    _eventsSub = _controller!.eventsStream.listen((event) {
      if (!mounted) return;

      if (event is StreamEvent_OriginVideoSize && event.height > BigInt.zero) {
        setState(() {
          _aspectRatio = event.width.toDouble() / event.height.toDouble();
        });
      } else if (event is StreamEvent_Error) {
        setState(() {
          _error = event.field0;
        });
      }
    });
  }

  void _startPositionTicker() {
    _positionTicker?.cancel();
    // Update position every second
    _positionTicker = Timer.periodic(const Duration(seconds: 1), (_) async {
      if (!mounted || _controller == null) return;

      // Note: Playbin doesn't currently expose position/duration events
      // This is a placeholder for future enhancement
      setState(() {
        _position += const Duration(seconds: 1);
      });
    });
  }

  Future<void> _seek(Duration position) async {
    if (_controller == null) return;

    final result = await _controller!.seekToTimestampMs(
      BigInt.from(position.inMilliseconds),
    );

    if (result.isOk()) {
      setState(() {
        _position = position;
      });
    }
  }

  String _formatDuration(Duration duration) {
    final hours = duration.inHours;
    final minutes = duration.inMinutes.remainder(60);
    final seconds = duration.inSeconds.remainder(60);

    if (hours > 0) {
      return '${hours.toString().padLeft(2, '0')}:${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}';
    }
    return '${minutes.toString().padLeft(2, '0')}:${seconds.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              _error!,
              style: const TextStyle(color: Colors.red),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 12),
            ElevatedButton(onPressed: _connect, child: const Text('Retry')),
          ],
        ),
      );
    }

    if (_controller == null) return const SizedBox();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Video
        Expanded(
          child: Stack(
            fit: StackFit.expand,
            children: [
              AspectRatio(
                aspectRatio: _aspectRatio,
                child: VideoPlayer.fromController(
                  controller: _controller!,
                  autoDispose: false,
                ),
              ),
              if (_error != null)
                Positioned(
                  bottom: 0,
                  left: 0,
                  right: 0,
                  child: GestureDetector(
                    onTap: () => setState(() => _error = null),
                    child: Container(
                      color: Colors.black.withValues(alpha: 0.7),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      child: Row(
                        children: [
                          const Icon(
                            Icons.error_outline,
                            color: Colors.red,
                            size: 16,
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: Text(
                              _error!,
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 12,
                              ),
                            ),
                          ),
                          const Icon(
                            Icons.close,
                            color: Colors.white54,
                            size: 14,
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),

        // Controls
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Status row
              Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: _isPlaying ? Colors.green : Colors.grey,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      _isPlaying ? 'PLAYING' : 'PAUSED',
                      style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        fontSize: 11,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    _formatDuration(_position),
                    style: const TextStyle(fontSize: 12),
                  ),
                ],
              ),

              // Timeline slider
              SliderTheme(
                data: SliderTheme.of(context).copyWith(
                  activeTrackColor: Colors.blue,
                  thumbColor: Colors.blue,
                  overlayColor: Colors.blue.withValues(alpha: 0.2),
                  inactiveTrackColor: Colors.grey.shade300,
                  trackHeight: 3,
                  thumbShape: const RoundSliderThumbShape(
                    enabledThumbRadius: 6,
                  ),
                ),
                child: Slider(
                  min: 0,
                  max: _duration.inMilliseconds.toDouble(),
                  value: _position.inMilliseconds.toDouble().clamp(
                    0,
                    _duration.inMilliseconds.toDouble(),
                  ),
                  label: _formatDuration(_position),
                  divisions:
                      _duration.inMilliseconds > 0
                          ? (_duration.inMilliseconds ~/ 1000).clamp(1, 3600)
                          : null,
                  onChanged: (value) {
                    setState(() {
                      _position = Duration(milliseconds: value.round());
                    });
                  },
                  onChangeEnd: (value) {
                    _seek(Duration(milliseconds: value.round()));
                  },
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
