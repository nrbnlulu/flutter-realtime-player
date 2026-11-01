import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/flutter_realtime_player.dart' as player;
import 'package:flutter_realtime_player/rust/dart_types.dart';

// Run with rlutter's --target and --dart-define options. Uses the same public
// controller and Texture widget as the example; the button exercises disposal
// followed by a fresh session without restarting the Android engine.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await player.init();
  runApp(const MaterialApp(home: RenderProbe()));
}

class RenderProbe extends StatefulWidget {
  const RenderProbe({super.key});

  @override
  State<RenderProbe> createState() => _RenderProbeState();
}

class _RenderProbeState extends State<RenderProbe> {
  player.VideoController? _controller;
  StreamSubscription<StreamState>? _states;
  StreamSubscription<StreamEvent>? _events;
  String _status = 'Starting';
  double _aspectRatio = 16 / 9;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_restart());
  }

  Future<void> _restart() async {
    final generation = ++_generation;
    final previous = _controller;
    setState(() {
      _controller = null;
      _status = 'Starting session $generation';
    });
    await _states?.cancel();
    await _events?.cancel();
    await previous?.dispose();
    if (!mounted || generation != _generation) return;

    const sourceId = String.fromEnvironment('PROBE_SOURCE_ID');
    final config =
        sourceId.isEmpty
            ? player.VideoConfig.playbin(
              player.PlaybinConfig(
                uri: const String.fromEnvironment(
                  'PROBE_VIDEO_URL',
                  defaultValue: 'http://127.0.0.1:8765/demo.mp4',
                ),
                mute: true,
              ),
            )
            : player.VideoConfig.wscRtp(
              player.WscRtpSessionConfig(
                baseUrl: const String.fromEnvironment(
                  'PROBE_BASE_URL',
                  defaultValue: 'http://127.0.0.1:8000/media',
                ),
                sourceId: sourceId,
                forceWebsocketTransport: true,
                autoRestart: false,
              ),
            );
    final (controller, error) = await player.VideoController.create(
      config: config,
    );
    if (!mounted || generation != _generation) {
      await controller?.dispose();
      return;
    }
    if (controller == null) {
      debugPrint('RENDER_PROBE error: $error');
      setState(() => _status = 'Error: $error');
      return;
    }
    _states = controller.stateBroadcast.listen((state) {
      debugPrint('RENDER_PROBE session=${controller.sessionId} state=$state');
      if (mounted) setState(() => _status = state.toString());
    });
    _events = controller.eventsStream.listen((event) {
      debugPrint('RENDER_PROBE session=${controller.sessionId} event=$event');
      if (mounted &&
          event is StreamEvent_OriginVideoSize &&
          event.height.toDouble() > 0) {
        setState(
          () => _aspectRatio = event.width.toDouble() / event.height.toDouble(),
        );
      }
    });
    setState(() => _controller = controller);
  }

  @override
  void dispose() {
    ++_generation;
    unawaited(_states?.cancel());
    unawaited(_events?.cancel());
    unawaited(_controller?.dispose());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final controller = _controller;
    return Scaffold(
      appBar: AppBar(title: const Text('Android rendering probe')),
      body: Column(
        children: [
          Padding(padding: const EdgeInsets.all(12), child: Text(_status)),
          Expanded(
            child:
                controller == null
                    ? const Center(child: CircularProgressIndicator())
                    : Center(
                      child: AspectRatio(
                        aspectRatio: _aspectRatio,
                        child: player.VideoPlayer.fromController(
                          key: ValueKey(controller.sessionId),
                          controller: controller,
                          autoDispose: false,
                        ),
                      ),
                    ),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: FilledButton(
              onPressed: _restart,
              child: const Text('Restart session'),
            ),
          ),
        ],
      ),
    );
  }
}
