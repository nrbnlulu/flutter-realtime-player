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

/// Combined message type that contains either a state or event message
sealed class CombinedMessage {}

class StateMessage implements CombinedMessage {
  final StreamState state;
  StateMessage(this.state);
}

class EventMessage implements CombinedMessage {
  final StreamEvent event;
  EventMessage(this.event);
}

class VideoController {
  final int sessionId;
  final VideoConfig config;
  final rx.BehaviorSubject<StreamState> stateBroadcast;
  final StreamSubscription _combinedSub;
  final Stream<StreamEvent> eventsStream;
  bool _running = false;

  VideoController(
    StreamSubscription combinedSub, {
    required this.sessionId,
    required this.stateBroadcast,
    required this.eventsStream,
    required this.config,
  }) : _combinedSub = combinedSub;

  Future<void> dispose() async {
    _running = false;
    await rlib.destroyStreamSession(sessionId: sessionId);
    _combinedSub.cancel();
  }

  static Future<(VideoController?, String?)> create({
    required VideoConfig config,
  }) async {
    final handle = await EngineContext.instance.getEngineHandle();
    final sessionId = await rlib.createNewSession();

    try {
      // Create separate subjects for state and events
      final stateSubject = rx.BehaviorSubject<StreamState>.seeded(
        StreamState.loading(),
      );
      final eventsSubject = rx.BehaviorSubject<StreamEvent>();
      final eventsStream = eventsSubject.stream;

      final combinedStream = rlib.createPlayable(
        sessionId: sessionId,
        engineHandle: handle,
        config: config,
      );

      // Listen to the combined stream and split into state and events
      final combinedSub = combinedStream.listen(
        (message) {
          switch (message) {
            case StreamMessage_State(field0: final state):
              if (!stateSubject.isClosed) {
                stateSubject.add(state);
              }
              break;
            case StreamMessage_Event(field0: final event):
              if (!eventsSubject.isClosed) {
                eventsSubject.add(event);
              }
              break;
          }
        },
        onError: (error) {
          if (!stateSubject.isClosed) {
            stateSubject.addError(error);
          }
          if (!eventsSubject.isClosed) {
            eventsSubject.addError(error);
          }
        },
      );

      final ret = VideoController(
        combinedSub,
        sessionId: sessionId,
        stateBroadcast: stateSubject,
        eventsStream: eventsStream,
        config: config,
      );
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
  final Widget Function(BuildContext context)? loadingBuilder;
  /// whether to dispose the stream when the widget disposes?
  final bool autoDispose;

  const VideoPlayer._({
    super.key,
    required this.controller,
    this.child,
    this.loadingBuilder,
    this.autoDispose = true,
  });

  factory VideoPlayer.fromController({
    Key? key,
    required VideoController controller,
    bool autoDispose = true,
    Widget? child,
    Widget Function(BuildContext context)? loadingBuilder,
  }) {
    return VideoPlayer._(
      key: key,
      controller: controller,
      autoDispose: autoDispose,
      loadingBuilder: loadingBuilder,
      child: child,
    );
  }

  static Widget fromConfig({
    Key? key,
    required VideoConfig config,
    bool autoDispose = true,
    Widget? child,
    Widget Function(BuildContext context)? loadingBuilder,
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
        return VideoPlayer._(
          key: key,
          controller: controller!,
          autoDispose: autoDispose,
          loadingBuilder: loadingBuilder,
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

  Widget _defaultLoading(String message) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const CircularProgressIndicator(),
        const SizedBox(width: 10),
        Text(message, style: const TextStyle(fontSize: 14)),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    if (currentState == null) {
      return widget.loadingBuilder?.call(context) ??
          _defaultLoading('Initializing...');
    }
    return switch (currentState!) {
      StreamState_Loading() =>
        widget.loadingBuilder?.call(context) ??
            _defaultLoading('Initializing stream...'),
      StreamState_Error(field0: final message) => Center(
        child: Text(
          'Error: $message',
          style: const TextStyle(color: Colors.red, fontSize: 16),
        ),
      ),
      StreamState_Playing(:final textureId) => Texture(textureId: textureId),
      StreamState_Stopped() => const Center(
        child: Text('Video stopped', style: TextStyle(fontSize: 16)),
      ),
    };
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
