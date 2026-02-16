import 'dart:async';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import "package:rxdart/rxdart.dart" as rx;
import 'rust/core/types.dart';

// Define VideoDimensions for internal compatibility
class VideoDimensions {
  final int width;
  final int height;

  const VideoDimensions({required this.width, required this.height});
}

class VideoController {
  final String url;
  final bool mute;
  final bool autoRestart;
  Map<String, String>? ffmpegOptions;

  final int sessionId;
  final rx.BehaviorSubject<StreamState> stateBroadcast;
  final StreamSubscription<StreamState> _originalSub;
  bool _running = false;
  int? _engineHandle;

  VideoController(
    StreamSubscription<StreamState> originalSub, {
    required this.url,
    required this.sessionId,
    required this.stateBroadcast,
    this.mute = true,
    this.autoRestart = false,
    this.ffmpegOptions,
  }) : _originalSub = originalSub;

  Future<void> dispose() async {
    _running = false;
    await rlib.destroyStreamSession(sessionId: sessionId);
    _originalSub.cancel();
  }

  static Future<(VideoController?, String?)> create({
    required String url,
    bool mute = true,
    bool autoRestart = false,
    Map<String, String>? ffmpegOptions,
  }) async {
    final handle = await EngineContext.instance.getEngineHandle();
    final sessionId = await rlib.createNewSession();

    // play demo video
    try {
      final stream = rlib.createNewPlayable(
        sessionId: sessionId,
        engineHandle: handle,
        ffmpegOptions: ffmpegOptions,
        videoInfo: VideoInfo(uri: url, mute: mute, autoRestart: autoRestart),
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
        url: url,
        autoRestart: autoRestart,
        ffmpegOptions: ffmpegOptions,
        mute: mute,
      );
      ret._engineHandle = handle;
      ret._running = true;

      // start ping task
      Future.microtask(() async {
        while (ret._running) {
          // ping rust side to announce we still want the stream.
          rlib.markSessionAlive(sessionId: sessionId);
          await Future.delayed(const Duration(seconds: 1));
        }
      });
      return (ret, null);
    } catch (e) {
      return (null, e.toString());
    }
  }

  static Future<(VideoController?, String?)> createWscRtp({
    required WscRtpSessionConfig config,
    bool mute = true,
    bool autoRestart = false,
  }) async {
    final handle = await EngineContext.instance.getEngineHandle();
    final sessionId = await rlib.createNewSession();

    try {
      final stream = rlib.createWscRtpPlayable(
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
        url: '${config.baseUrl}/streams/${config.sourceId}',
        autoRestart: autoRestart,
        mute: mute,
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

  /// Seek to a specific time in seconds within the video
  Future<void> seekTo(int ts) async {
    await rlib.seekToTimestamp(sessionId: sessionId, ts: ts);
  }

  Future<void> seekToTimestampMs(int tsMs) async {
    await rlib.seekToTimestamp(sessionId: sessionId, ts: tsMs);
  }

  Future<void> wscRtpGoLive() async {
    await rlib.wscRtpGoLive(sessionId: sessionId);
  }

  Future<void> setSpeed(double speed) async {
    await rlib.setSpeed(sessionId: sessionId, speed: speed);
  }
}
// ignore: implementation_imports

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
    required String url,
    Map<String, String>? ffmpegOptions,
    bool mute = true,
    bool autoRestart = false,
    bool autoDispose = true,
    Widget? child,
  }) {
    return FutureBuilder(
      future: _createControllerWithSize(
        url,
        // Pass dummy dimensions since the API still expects them but ignores them
        const VideoDimensions(width: 1280, height: 720),
        mute,
        autoRestart,
        ffmpegOptions,
      ),
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

  static Future<(VideoController?, String?)> _createControllerWithSize(
    String url,
    VideoDimensions dimensions,
    bool mute,
    bool autoRestart,
    Map<String, String>? ffmpegOptions,
  ) async {
    return VideoController.create(
      url: url,
      mute: mute,
      autoRestart: autoRestart,
      ffmpegOptions: ffmpegOptions,
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
    // If a controller was provided, use it directly
    return _VideoPlayerWithSize(
      controller: widget.controller,
      autoDispose: widget.autoDispose,
      child: widget.child,
    );
    // This case should not happen due to assertion, but we include it to avoid errors
    return const SizedBox.shrink();
  }
}
