import 'dart:async';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:oxidized/oxidized.dart' as oxidized;
import "package:rxdart/rxdart.dart" as rx;
import 'rust/core/types.dart';

class VideoController {
  final int sessionId;
  final VideoConfig config;
  final rx.BehaviorSubject<StreamState> stateBroadcast;
  final StreamSubscription<StreamState> _originalSub;
  bool _running = false;
  int? _engineHandle;

  VideoController(
    StreamSubscription<StreamState> originalSub, {
    required this.sessionId,
    required this.stateBroadcast,
    required this.config,
  }) : _originalSub = originalSub;

  Future<void> dispose() async {
    _running = false;
    await rlib.destroyStreamSession(sessionId: sessionId);
    _originalSub.cancel();
  }

  static Future<(VideoController?, String?)> create({
    required VideoConfig config,
  }) async {
    final handle = await EngineContext.instance.getEngineHandle();
    final sessionId = await rlib.createNewSession();

    try {
      final stream = rlib.createPlayable(
        sessionId: sessionId,
        engineHandle: handle,
        config: config,
      );
      final bs = rx.BehaviorSubject<StreamState>.seeded(StreamState.loading());
      final origSub = stream.listen(
        bs.add,
        onError: bs.addError,
        onDone: () => bs.close(),
      );
      final ret = VideoController(
        origSub,
        sessionId: sessionId,
        stateBroadcast: bs,
        config: config,
      );
      ret._engineHandle = handle;
      ret._running = true;

      Future.microtask(() async {
        while (ret._running) {
          rlib.markSessionAlive(sessionId: sessionId);
          await Future.delayed(const Duration(seconds: 1));
        }
      });
      return (ret, null);
    } catch (e) {
      return (null, e.toString());
    }
  }

  Future<oxidized.Result<void, AnyhowException>> seekToTimestampMs(
    BigInt tsMs,
  ) async {
    try {
      await rlib.seekToTimestamp(sessionId: sessionId, ts: tsMs);
      return oxidized.Result.ok(null);
    } catch (e) {
      return oxidized.Result.err(
        e is AnyhowException ? e : AnyhowException(e.toString()),
      );
    }
  }

  Future<oxidized.Result<void, AnyhowException>> wscRtpGoLive() async {
    try {
      await rlib.wscRtpGoLive(sessionId: sessionId);
      return oxidized.Result.ok(null);
    } catch (e) {
      return oxidized.Result.err(
        e is AnyhowException ? e : AnyhowException(e.toString()),
      );
    }
  }

  Future<oxidized.Result<void, AnyhowException>> setSpeed(double speed) async {
    try {
      await rlib.setSpeed(sessionId: sessionId, speed: speed);
      return oxidized.Result.ok(null);
    } catch (e) {
      return oxidized.Result.err(
        e is AnyhowException ? e : AnyhowException(e.toString()),
      );
    }
  }
}

class VideoPlayer extends StatefulWidget {
  final VideoController controller;
  final Widget? child;

  /// whether to dispose the stream when the widget disposes?
  final bool autoDispose;

  const VideoPlayer._({
    super.key,
    required this.controller,
    this.child,
    this.autoDispose = true,
  });

  factory VideoPlayer.fromController({
    GlobalKey? key,
    required VideoController controller,
    bool autoDispose = true,
    Widget? child,
  }) {
    return VideoPlayer._(
      key: key,
      controller: controller,
      autoDispose: autoDispose,
      child: child,
    );
  }

  static Widget fromConfig({
    GlobalKey? key,
    required VideoConfig config,
    bool autoDispose = true,
    Widget? child,
  }) {
    return FutureBuilder(
      future: VideoController.create(config: config),
      builder: (ctx, res) {
        if (res.hasError) {
          return Text(res.error.toString());
        }
        final (controller, err) = res.data!;
        if (err != null) {
          return Text(err);
        }
        return _VideoPlayerWithSize(
          controller: controller!,
          autoDispose: autoDispose,
          child: child,
        );
      },
    );
  }

  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

// New stateful widget that handles size changes
class _VideoPlayerWithSize extends StatefulWidget {
  final VideoController controller;
  final Widget? child;
  final bool autoDispose;

  const _VideoPlayerWithSize({
    required this.controller,
    this.child,
    this.autoDispose = true,
  });

  @override
  State<_VideoPlayerWithSize> createState() => _VideoPlayerWithSizeState();
}

class _VideoPlayerWithSizeState extends State<_VideoPlayerWithSize> {
  StreamState? currentState;
  StreamSubscription<StreamState>? streamSubscription;

  @override
  void initState() {
    super.initState();

    streamSubscription = widget.controller.stateBroadcast.listen((state) {
      setState(() {
        currentState = state;
      });
    });
  }

  Widget loadingWidget(String message) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const CircularProgressIndicator(),
        const SizedBox(width: 10),
        Text(message, style: const TextStyle(fontSize: 16)),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (currentState == null) {
          return loadingWidget('initializing...');
        }
        switch (currentState!) {
          case StreamState_Loading():
            return loadingWidget('initializing stream...');
          case StreamState_Error(field0: final message):
            return Center(
              child: Text(
                'Error: $message',
                style: const TextStyle(color: Colors.red, fontSize: 16),
              ),
            );
          case StreamState_Playing(textureId: final textureId):
            return Stack(
              children: [
                Texture(textureId: textureId),
                widget.child ?? const SizedBox(),
              ],
            );
          case StreamState_Stopped():
            return Center(
              child: Text(
                'Video stopped',
                style: const TextStyle(fontSize: 16),
              ),
            );
        }
      },
    );
  }

  @override
  void dispose() {
    super.dispose();

    Future.microtask(() async {
      streamSubscription?.cancel();
      if (widget.autoDispose) {
        try {
          if (kDebugMode) {
            debugPrint(
              'disposing stream session(${widget.controller.sessionId})',
            );
          }
          await widget.controller.dispose();
        } catch (e) {
          if (kDebugMode) {
            debugPrint(
              'Error disposing session(${widget.controller.sessionId}): $e',
            );
          }
        }
      }
    });
  }
}

class _VideoPlayerState extends State<VideoPlayer> {
  @override
  Widget build(BuildContext context) {
    return _VideoPlayerWithSize(
      controller: widget.controller,
      autoDispose: widget.autoDispose,
      child: widget.child,
    );
  }
}
