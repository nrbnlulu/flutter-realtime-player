import 'dart:async';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import "package:rxdart/rxdart.dart" as rx;
import 'rust/core/types.dart';

class VideoController {
  final String url;
  final bool mute;
  Map<String, String>? ffmpegOptions;

  final int sessionId;
  final rx.BehaviorSubject<StreamState> stateBroadcast;
  final StreamSubscription<StreamState> _originalSub;
  bool _running = false;

  VideoController(
    StreamSubscription<StreamState> originalSub, {
    required this.url,
    required this.sessionId,
    required this.stateBroadcast,
    this.mute = true,
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
        videoInfo: VideoInfo(
          uri: url,
          dimensions: const VideoDimensions(width: 640, height: 360),
          mute: mute,
        ),
      );
      final bs = rx.BehaviorSubject<StreamState>();
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
        ffmpegOptions: ffmpegOptions,
        mute: mute,
      );
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
}

// ignore: implementation_imports

class VideoPlayer extends StatefulWidget {
  final VideoController controller;
  final Widget? child;

  const VideoPlayer._({super.key, required this.controller, this.child});

  factory VideoPlayer.fromController({
    GlobalKey? key,
    required VideoController controller,
    Widget? child,
  }) {
    return VideoPlayer._(key: key, controller: controller, child: child);
  }

  static Widget fromConfig({
    GlobalKey? key,
    required String url,
    Map<String, String>? ffmpegOptions,
    bool mute = true,
    Widget? child,
  }) {
    return FutureBuilder(
      future: VideoController.create(
        url: url,
        mute: mute,
        ffmpegOptions: ffmpegOptions,
      ),
      builder: (ctx, res) {
        if (res.hasError) {
          return Text(res.error.toString());
        }
        final (controller, err) = res.data!;
        if (err != null) {
          return Text(err);
        }
        return VideoPlayer.fromController(
          key: key,
          controller: controller!,
          child: child,
        );
      },
    );
  }

  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

class _VideoPlayerState extends State<VideoPlayer> {
  StreamState? currentState;
  late Stream<StreamState> rustStateStream;
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
          child: Text('Video stopped', style: const TextStyle(fontSize: 16)),
        );
    }
  }

  @override
  void dispose() {
    super.dispose();

    Future.microtask(() async {
      streamSubscription?.cancel();
      try {
        if (kDebugMode) {
          debugPrint(
            'disposing stream session(${widget.controller.sessionId})',
          );
        }
        await rlib.destroyStreamSession(sessionId: widget.controller.sessionId);
      } catch (e) {
        if (kDebugMode) {
          debugPrint(
            'Error disposing session(${widget.controller.sessionId}): $e',
          );
        }
      }
    });
  }
}
