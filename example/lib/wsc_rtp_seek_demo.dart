import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/core/types.dart'
    show VideoDimensions, WscRtpSessionConfig;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:flutter_realtime_player/video_player.dart';

class SessionMode {
  final bool isLive;
  final int currentTimeMs;
  final double speed;

  SessionMode({
    required this.isLive,
    required this.currentTimeMs,
    required this.speed,
  });
}

class WscRtpSeekDemo extends StatefulWidget {
  const WscRtpSeekDemo({super.key});

  @override
  State<WscRtpSeekDemo> createState() => _WscRtpSeekDemoState();
}

class _WscRtpSeekDemoState extends State<WscRtpSeekDemo> {
  final TextEditingController _baseUrlController = TextEditingController(
    text: 'http://localhost:8009',
  );
  final TextEditingController _sourceIdController = TextEditingController(
    text: '1',
  );
  final TextEditingController _clientPortController = TextEditingController();

  VideoController? _controller;
  int? _sessionId;
  bool _isConnecting = false;
  bool _forceWebsocketTransport = false;
  String? _errorMessage;
  SessionMode? _sessionMode;
  StreamSubscription<StreamEvent>? _eventsSub;

  @override
  void dispose() {
    _eventsSub?.cancel();
    _stopPlayback();
    _baseUrlController.dispose();
    _sourceIdController.dispose();
    _clientPortController.dispose();
    super.dispose();
  }

  Future<void> _connect() async {
    if (_isConnecting) return;

    setState(() {
      _isConnecting = true;
      _errorMessage = null;
    });

    await _stopPlayback();

    try {
      final requestedPort = int.tryParse(_clientPortController.text.trim());
      final dimensions = const VideoDimensions(width: 1280, height: 720);
      final result = await VideoController.createWscRtp(
        config: WscRtpSessionConfig(
          baseUrl: _baseUrlController.text.trim(),
          sourceId: _sourceIdController.text.trim(),
          clientPort: requestedPort,
          forceWebsocketTransport: _forceWebsocketTransport,
        ),
        dimensions: dimensions,
        autoRestart: true,
      );

      if (result.$1 == null) {
        throw Exception(result.$2 ?? 'Failed to create player');
      }

      if (mounted) {
        setState(() {
          _controller = result.$1;
          _sessionId = result.$1!.sessionId;
          _isConnecting = false;
          _sessionMode = SessionMode(
            isLive: true,
            currentTimeMs: DateTime.now().millisecondsSinceEpoch,
            speed: 1.0,
          );
        });
        _startEventListener();
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _errorMessage = e.toString();
          _isConnecting = false;
        });
      }
    }
  }

  void _startEventListener() {
    _eventsSub?.cancel();
    final sessionId = _sessionId;
    if (sessionId == null) return;
    _eventsSub = rlib.registerToStreamEventsSink(sessionId: sessionId).listen((
      event,
    ) {
      if (_sessionId != sessionId) return;
      if (event is StreamEvent_Error) {
        debugPrint('WSC-RTP error: ${event.field0}');
        return;
      }
      if (event is StreamEvent_WscRtpStreamState) {
        debugPrint('WSC-RTP state: ${event.field0}');
        return;
      }
      if (event is StreamEvent_WscRtpSessionMode) {
        if (!mounted) return;
        setState(() {
          _sessionMode = SessionMode(
            isLive: event.isLive,
            currentTimeMs: event.currentTimeMs,
            speed: event.speed,
          );
        });
      }
    });
  }

  Future<void> _stopPlayback() async {
    _eventsSub?.cancel();
    _eventsSub = null;
    await _controller?.dispose();
    _controller = null;
    _sessionId = null;
    _sessionMode = null;
    if (mounted) setState(() {});
  }

  Future<void> _seekRelative(int seconds) async {
    final sessionId = _sessionId;
    if (sessionId == null) return;
    final current =
        _sessionMode?.currentTimeMs ?? DateTime.now().millisecondsSinceEpoch;
    final target = current + (seconds * 1000);
    await rlib.seekToTimestamp(sessionId: sessionId, ts: target);
    if (!mounted) return;
    setState(() {
      _sessionMode = SessionMode(
        isLive: false,
        currentTimeMs: target,
        speed: _sessionMode?.speed ?? 1.0,
      );
    });
  }

  Future<void> _seekToNow() async {
    final sessionId = _sessionId;
    if (sessionId == null) return;
    await rlib.wscRtpGoLive(sessionId: sessionId);
    if (!mounted) return;
    setState(() {
      _sessionMode = SessionMode(
        isLive: true,
        currentTimeMs: DateTime.now().millisecondsSinceEpoch,
        speed: _sessionMode?.speed ?? 1.0,
      );
    });
  }

  Future<void> _pickDateTime() async {
    final now = DateTime.now();
    final date = await showDatePicker(
      context: context,
      initialDate: now,
      firstDate: now.subtract(const Duration(days: 30)),
      lastDate: now,
    );
    if (date == null) return;

    if (!mounted) return;
    final time = await showTimePicker(
      context: context,
      initialTime: TimeOfDay.fromDateTime(now),
    );
    if (time == null) return;

    final dt = DateTime(
      date.year,
      date.month,
      date.day,
      time.hour,
      time.minute,
    );
    final sessionId = _sessionId;
    if (sessionId == null) return;
    await rlib.seekToTimestamp(
      sessionId: sessionId,
      ts: dt.millisecondsSinceEpoch,
    );
    if (!mounted) return;
    setState(() {
      _sessionMode = SessionMode(
        isLive: false,
        currentTimeMs: dt.millisecondsSinceEpoch,
        speed: _sessionMode?.speed ?? 1.0,
      );
    });
  }

  Future<void> _setSpeed(double speed) async {
    final sessionId = _sessionId;
    if (sessionId == null) return;
    try {
      await rlib.setSpeed(sessionId: sessionId, speed: speed);
    } catch (e) {
      debugPrint("Set speed failed: $e");
    }
  }

  String _formatTime(int ms) {
    final dt = DateTime.fromMillisecondsSinceEpoch(ms);
    final h = dt.hour.toString().padLeft(2, '0');
    final m = dt.minute.toString().padLeft(2, '0');
    final s = dt.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }

  @override
  Widget build(BuildContext context) {
    if (_controller == null) {
      return _buildConnectionForm();
    }
    return _buildPlayerInterface();
  }

  Widget _buildConnectionForm() {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 400),
        child: Card(
          margin: const EdgeInsets.all(32),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'Connect to Stream',
                  style: Theme.of(context).textTheme.headlineSmall,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 24),
                TextField(
                  controller: _baseUrlController,
                  decoration: const InputDecoration(
                    labelText: 'Base URL',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.link),
                  ),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _sourceIdController,
                  decoration: const InputDecoration(
                    labelText: 'Source ID',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.videocam),
                  ),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _clientPortController,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    labelText: 'Client Port (Optional)',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.router),
                  ),
                ),
                const SizedBox(height: 16),
                SwitchListTile(
                  title: const Text('Force WebSocket Transport'),
                  subtitle: const Text('Skip UDP, use WS for RTP'),
                  value: _forceWebsocketTransport,
                  onChanged: (value) {
                    setState(() {
                      _forceWebsocketTransport = value;
                    });
                  },
                  contentPadding: EdgeInsets.zero,
                ),
                const SizedBox(height: 16),
                if (_errorMessage != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 16),
                    child: Text(
                      _errorMessage!,
                      style: const TextStyle(color: Colors.red),
                      textAlign: TextAlign.center,
                    ),
                  ),
                ElevatedButton(
                  onPressed: _isConnecting ? null : _connect,
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.all(16),
                  ),
                  child:
                      _isConnecting
                          ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                          : const Text('CONNECT'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildPlayerInterface() {
    final isLive = _sessionMode?.isLive ?? true;
    final currentTime =
        _sessionMode != null
            ? DateTime.fromMillisecondsSinceEpoch(_sessionMode!.currentTimeMs)
            : DateTime.now();

    return Column(
      children: [
        // Header
        Container(
          color: Colors.black87,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: isLive ? Colors.red : Colors.blue,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  isLive ? 'LIVE' : 'DVR',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                    fontSize: 12,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Text(
                'Source: ${_sourceIdController.text}',
                style: const TextStyle(color: Colors.white70),
              ),
              const Spacer(),
              IconButton(
                icon: const Icon(Icons.close, color: Colors.white),
                onPressed: _stopPlayback,
                tooltip: 'Disconnect',
              ),
            ],
          ),
        ),

        // Player
        Expanded(
          child: Container(
            color: Colors.black,
            child: Center(
              child: AspectRatio(
                aspectRatio: 16 / 9,
                child:
                    _controller != null
                        ? VideoPlayer.fromController(
                          controller: _controller!,
                          autoDispose: false,
                        )
                        : const Center(child: CircularProgressIndicator()),
              ),
            ),
          ),
        ),

        // Controls
        Container(
          color: Colors.grey[200],
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Time & Speed Status
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'Time: ${_formatTime(currentTime.millisecondsSinceEpoch)}',
                    style: const TextStyle(
                      fontWeight: FontWeight.bold,
                      fontSize: 16,
                    ),
                  ),
                  Text('Speed: ${_sessionMode?.speed.toStringAsFixed(1)}x'),
                ],
              ),
              const SizedBox(height: 12),

              // Seek Controls
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  _buildSeekBtn('-60s', -60),
                  const SizedBox(width: 8),
                  _buildSeekBtn('-10s', -10),
                  const SizedBox(width: 16),
                  _buildSeekBtn('+10s', 10),
                  const SizedBox(width: 8),
                  _buildSeekBtn('+60s', 60),
                ],
              ),
              const SizedBox(height: 12),

              // Playback Controls
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  // Go Live
                  ElevatedButton.icon(
                    onPressed: isLive ? null : _seekToNow,
                    icon: const Icon(Icons.fiber_dvr, color: Colors.red),
                    label: const Text('GO LIVE'),
                  ),
                  const SizedBox(width: 16),

                  // Speed Selector
                  PopupMenuButton<double>(
                    initialValue: _sessionMode?.speed,
                    onSelected: _setSpeed,
                    itemBuilder:
                        (context) => [
                          const PopupMenuItem(value: 0.5, child: Text('0.5x')),
                          const PopupMenuItem(value: 1.0, child: Text('1.0x')),
                          const PopupMenuItem(value: 2.0, child: Text('2.0x')),
                          const PopupMenuItem(value: 5.0, child: Text('5.0x')),
                        ],
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        border: Border.all(color: Colors.grey),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Row(
                        children: [
                          Text('${_sessionMode?.speed ?? 1.0}x'),
                          const Icon(Icons.arrow_drop_down),
                        ],
                      ),
                    ),
                  ),

                  const SizedBox(width: 16),
                  // Custom Seek
                  IconButton(
                    onPressed: _pickDateTime,
                    icon: const Icon(Icons.calendar_today),
                    tooltip: 'Seek to Date/Time',
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildSeekBtn(String label, int seconds) {
    return OutlinedButton(
      onPressed: () => _seekRelative(seconds),
      child: Text(label),
    );
  }
}
