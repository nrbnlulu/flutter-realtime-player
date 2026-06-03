import 'dart:async';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart';
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'package:oxidized/oxidized.dart' as oxidized;
import 'package:rxdart/rxdart.dart' as rx;

import 'video_controller_base.dart';

export 'video_controller_base.dart'
    show
        CombinedMessage,
        StateMessage,
        EventMessage,
        LoadingBuilder,
        ContentBuilder,
        defaultLoadingBuilder,
        VideoControllerBase;

/// An inert controller used to keep applications buildable on web.
class VideoController extends VideoControllerBase {
  static int _nextSessionId = 0;

  @override
  final int sessionId;
  @override
  final VideoConfig config;
  @override
  final rx.BehaviorSubject<StreamState> stateBroadcast;
  @override
  final Stream<StreamEvent> eventsStream;

  final StreamSubscription _combinedSub;
  final rx.BehaviorSubject<StreamEvent>? _eventsSubject;

  VideoController(
    StreamSubscription combinedSub, {
    required this.sessionId,
    required this.config,
    required this.stateBroadcast,
    required this.eventsStream,
    rx.BehaviorSubject<StreamEvent>? eventsSubject,
  }) : _combinedSub = combinedSub,
       _eventsSubject = eventsSubject;

  @override
  Future<void> dispose() async {
    await _combinedSub.cancel();
    await stateBroadcast.close();
    await _eventsSubject?.close();
  }

  static Future<(VideoController?, String?)> create({
    required VideoConfig config,
  }) async {
    final stateSubject = rx.BehaviorSubject<StreamState>.seeded(
      StreamState.stopped(),
    );
    final eventsSubject = rx.BehaviorSubject<StreamEvent>();
    return (
      VideoController(
        const Stream<void>.empty().listen(null),
        sessionId: _nextSessionId++,
        config: config,
        stateBroadcast: stateSubject,
        eventsStream: eventsSubject.stream,
        eventsSubject: eventsSubject,
      ),
      null,
    );
  }

  @override
  Future<oxidized.Result<void, AnyhowException>> seekToTimestampMs(
    BigInt tsMs,
  ) async => oxidized.Result.ok(null);

  @override
  Future<oxidized.Result<void, AnyhowException>> wscRtpGoLive() async =>
      oxidized.Result.ok(null);

  @override
  Future<oxidized.Result<void, AnyhowException>> setSpeed(double speed) async =>
      oxidized.Result.ok(null);
}

Widget _defaultContent(BuildContext context, StreamState state) {
  return switch (state) {
    StreamState_Loading() => defaultLoadingBuilder(
      context,
      'Initializing stream...',
    ),
    StreamState_Error(field0: final message) => Center(
      child: Text(
        'Error: $message',
        style: const TextStyle(color: Colors.red, fontSize: 16),
      ),
    ),
    StreamState_Playing() => const SizedBox.shrink(),
    StreamState_Stopped() => const Center(
      child: Text('Video unavailable on web', style: TextStyle(fontSize: 16)),
    ),
  };
}

class VideoPlayer extends StatefulWidget {
  final VideoController controller;
  final LoadingBuilder loadingBuilder;
  final ContentBuilder contentBuilder;
  final bool autoDispose;

  const VideoPlayer._({
    super.key,
    required this.controller,
    this.loadingBuilder = defaultLoadingBuilder,
    this.contentBuilder = _defaultContent,
    this.autoDispose = true,
  });

  factory VideoPlayer.fromController({
    Key? key,
    required VideoController controller,
    bool autoDispose = true,
    LoadingBuilder? loadingBuilder,
    ContentBuilder? contentBuilder,
  }) {
    return VideoPlayer._(
      key: key,
      controller: controller,
      autoDispose: autoDispose,
      loadingBuilder: loadingBuilder ?? defaultLoadingBuilder,
      contentBuilder: contentBuilder ?? _defaultContent,
    );
  }

  static Widget fromConfig({
    Key? key,
    required VideoConfig config,
    bool autoDispose = true,
    LoadingBuilder? loadingBuilder,
    ContentBuilder? contentBuilder,
  }) {
    return FutureBuilder(
      future: VideoController.create(config: config),
      builder: (context, result) {
        if (!result.hasData) {
          return loadingBuilder?.call(context, 'Initializing...') ??
              defaultLoadingBuilder(context, 'Initializing...');
        }
        final (controller, error) = result.data!;
        if (error != null) {
          return Text(error);
        }
        return VideoPlayer._(
          key: key,
          controller: controller!,
          autoDispose: autoDispose,
          loadingBuilder: loadingBuilder ?? defaultLoadingBuilder,
          contentBuilder: contentBuilder ?? _defaultContent,
        );
      },
    );
  }

  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

class _VideoPlayerState extends State<VideoPlayer> {
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

  @override
  Widget build(BuildContext context) {
    if (currentState == null) {
      return widget.loadingBuilder(context, 'Initializing...');
    }
    return widget.contentBuilder(context, currentState!);
  }

  @override
  void dispose() {
    super.dispose();
    Future.microtask(() async {
      await streamSubscription?.cancel();
      if (widget.autoDispose) {
        try {
          await widget.controller.dispose();
        } catch (error) {
          if (kDebugMode) {
            debugPrint(
              'Error disposing session(${widget.controller.sessionId}): $error',
            );
          }
        }
      }
    });
  }
}
