import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/core/types.dart'
    show VideoConfig, WscRtpSessionConfig;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:flutter_realtime_player/video_player.dart';

/// A self-contained WSC-RTP player with seek timeline controls.
/// Handles connection, session events, and live/DVR timeline UI.
class WscRtpPlayerWidget extends StatefulWidget {
  final WscRtpSessionConfig config;

  const WscRtpPlayerWidget({super.key, required this.config});

  @override
  State<WscRtpPlayerWidget> createState() => _WscRtpPlayerWidgetState();
}

class _WscRtpPlayerWidgetState extends State<WscRtpPlayerWidget> {
  VideoController? _controller;
  bool _isLoading = true;
  String? _error;

  // Session mode from server events
  bool _isLive = true;

  // Current position as Unix epoch milliseconds.
  // Updated from WscRtpSessionMode events and advanced by timer.
  int _currentTimeMs = 0;

  // Most recent live edge known from the server.
  int _liveEdgeMs = 0;

  double _speed = 1.0;

  // While the user is dragging the slider we freeze the display value.
  bool _isDragging = false;
  int _dragTimeMs = 0;

  // Last error from a control operation (seek/live/speed), shown inline.
  // Cleared when a new successful session mode event arrives.
  String? _controlError;

  // How far back the timeline shows (60 minutes).
  static const int _timelineWindowMs = 60 * 60 * 1000;

  Timer? _ticker;
  StreamSubscription<StreamEvent>? _eventsSub;

  @override
  void initState() {
    super.initState();
    _connect();
  }

  @override
  void dispose() {
    _ticker?.cancel();
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
      config: VideoConfig.wscRtp(widget.config),
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
    _isLoading = false;

    _startEventListener();
    _startTicker();

    setState(() {});
  }

  void _startEventListener() {
    final sessionId = _controller!.sessionId;
    _eventsSub?.cancel();
    _eventsSub = rlib.registerToStreamEventsSink(sessionId: sessionId).listen(
      (event) {
        if (!mounted) return;
        if (event is StreamEvent_WscRtpSessionMode) {
          final mode = event.field0;
          setState(() {
            _controlError = null; // clear on any successful mode update
            if (mode is WscRtpMode_Live) {
              _isLive = true;
              // Advance live edge to now so the slider has a valid range.
              _liveEdgeMs = DateTime.now().millisecondsSinceEpoch;
              _currentTimeMs = _liveEdgeMs;
            } else if (mode is WscRtpMode_Dvr) {
              _isLive = false;
              _currentTimeMs = mode.currentTimeMs;
              _speed = mode.speed;
            }
          });
        } else if (event is StreamEvent_Error) {
          setState(() {
            _controlError = event.field0;
          });
        }
      },
    );
  }

  void _startTicker() {
    _ticker?.cancel();
    // Advance currentTimeMs by speed * 1s every second.
    _ticker = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      setState(() {
        _currentTimeMs += (_speed * 1000).round();
        if (_isLive) {
          _liveEdgeMs = _currentTimeMs;
        }
      });
    });
  }

  // Effective displayed time (frozen while dragging).
  int get _displayedTimeMs => _isDragging ? _dragTimeMs : _currentTimeMs;

  // Slider range: [liveEdge - window, liveEdge]
  int get _sliderMinMs =>
      (_liveEdgeMs > 0 ? _liveEdgeMs : _currentTimeMs) - _timelineWindowMs;
  int get _sliderMaxMs =>
      _liveEdgeMs > 0 ? _liveEdgeMs : _currentTimeMs;

  double get _sliderValue {
    if (_sliderMaxMs <= _sliderMinMs) return _sliderMaxMs.toDouble();
    return _displayedTimeMs.clamp(_sliderMinMs, _sliderMaxMs).toDouble();
  }

  String _formatTimestamp(int epochMs) {
    final dt = DateTime.fromMillisecondsSinceEpoch(epochMs, isUtc: false);
    final h = dt.hour.toString().padLeft(2, '0');
    final m = dt.minute.toString().padLeft(2, '0');
    final s = dt.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }

  Future<void> _seekTo(int targetMs) async {
    if (targetMs <= 0) return; // guard against uninitialized slider values
    // Errors are delivered via StreamEvent::Error and shown inline.
    await _controller?.seekToTimestampMs(BigInt.from(targetMs));
  }

  Future<void> _goLive() async {
    await _controller?.wscRtpGoLive();
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

    // Only show the timeline once we have a real live-edge timestamp from the server.
    final timelineAvailable = _liveEdgeMs > 0;
    final liveColor = Colors.green;
    final dvrColor = Colors.red;
    final activeColor = _isLive ? liveColor : dvrColor;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Video
        Expanded(
          child: Stack(
            fit: StackFit.expand,
            children: [
              StreamBuilder<StreamEvent>(
                stream: rlib.registerToStreamEventsSink(
                  sessionId: _controller!.sessionId,
                ),
                builder: (context, snapshot) {
                  double aspectRatio = 16 / 9;
                  if (snapshot.hasData) {
                    final event = snapshot.data!;
                    if (event is StreamEvent_OriginVideoSize &&
                        event.height > BigInt.zero) {
                      aspectRatio =
                          event.width.toDouble() / event.height.toDouble();
                    }
                  }
                  return AspectRatio(
                    aspectRatio: aspectRatio,
                    child: VideoPlayer.fromController(
                      controller: _controller!,
                      autoDispose: false,
                    ),
                  );
                },
              ),
              if (_controlError != null)
                Positioned(
                  bottom: 0,
                  left: 0,
                  right: 0,
                  child: GestureDetector(
                    onTap: () => setState(() => _controlError = null),
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
                              _controlError!,
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
                  // LIVE / DVR badge
                  Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 2,
                    ),
                    decoration: BoxDecoration(
                      color: activeColor,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      _isLive ? 'LIVE' : 'DVR',
                      style: const TextStyle(
                        color: Colors.white,
                        fontWeight: FontWeight.bold,
                        fontSize: 11,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    _currentTimeMs > 0 ? _formatTimestamp(_currentTimeMs) : '--:--:--',
                    style: const TextStyle(fontSize: 12),
                  ),
                  const Spacer(),
                  if (!_isLive)
                    TextButton.icon(
                      onPressed: _goLive,
                      icon: Icon(
                        Icons.fiber_dvr,
                        size: 16,
                        color: liveColor,
                      ),
                      label: Text(
                        'GO LIVE',
                        style: TextStyle(color: liveColor, fontSize: 12),
                      ),
                      style: TextButton.styleFrom(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 4,
                        ),
                        minimumSize: Size.zero,
                        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      ),
                    ),
                ],
              ),

              // Timeline slider
              if (timelineAvailable) ...[
                SliderTheme(
                  data: SliderTheme.of(context).copyWith(
                    activeTrackColor: activeColor,
                    thumbColor: activeColor,
                    overlayColor: activeColor.withValues(alpha: 0.2),
                    inactiveTrackColor: Colors.grey.shade300,
                    trackHeight: 3,
                    thumbShape: const RoundSliderThumbShape(
                      enabledThumbRadius: 6,
                    ),
                  ),
                  child: Slider(
                    min: _sliderMinMs.toDouble(),
                    max: _sliderMaxMs.toDouble(),
                    value: _isLive ? _sliderMaxMs.toDouble() : _sliderValue,
                    // Tooltip shown while dragging.
                    label: _formatTimestamp(
                      _isDragging ? _dragTimeMs : _displayedTimeMs,
                    ),
                    // 1-second granularity so labels appear; fine-grained enough.
                    divisions: _sliderMaxMs > _sliderMinMs
                        ? ((_sliderMaxMs - _sliderMinMs) ~/ 1000).clamp(1, 3600)
                        : null,
                    // Always interactive — dragging from live transitions to DVR.
                    onChangeStart: (value) {
                      setState(() {
                        _isDragging = true;
                        _dragTimeMs = value.round();
                      });
                    },
                    onChanged: (value) {
                      setState(() {
                        _dragTimeMs = value.round();
                      });
                    },
                    onChangeEnd: (value) {
                      final target = value.round();
                      setState(() {
                        _isDragging = false;
                        _currentTimeMs = target;
                      });
                      _seekTo(target);
                    },
                  ),
                ),
                // Timeline labels
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(
                        _sliderMinMs > 0
                            ? _formatTimestamp(_sliderMinMs)
                            : '-60min',
                        style: const TextStyle(
                          color: Colors.grey,
                          fontSize: 10,
                        ),
                      ),
                      Text(
                        _liveEdgeMs > 0
                            ? _formatTimestamp(_liveEdgeMs)
                            : 'LIVE',
                        style: TextStyle(
                          color: liveColor,
                          fontSize: 10,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }
}
