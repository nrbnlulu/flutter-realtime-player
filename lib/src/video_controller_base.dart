import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart';
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';
import 'package:oxidized/oxidized.dart' as oxidized;
import 'package:rxdart/rxdart.dart' as rx;

sealed class CombinedMessage {}

class StateMessage implements CombinedMessage {
  final StreamState state;
  StateMessage(this.state);
}

class EventMessage implements CombinedMessage {
  final StreamEvent event;
  EventMessage(this.event);
}

typedef LoadingBuilder = Widget Function(BuildContext context, String message);
typedef ContentBuilder =
    Widget Function(BuildContext context, StreamState state);

Widget defaultLoadingBuilder(BuildContext context, String message) {
  return Row(
    mainAxisAlignment: MainAxisAlignment.center,
    children: [
      const CircularProgressIndicator(),
      const SizedBox(width: 10),
      Text(message, style: const TextStyle(fontSize: 14)),
    ],
  );
}

abstract class VideoControllerBase {
  int get sessionId;
  VideoConfig get config;
  rx.BehaviorSubject<StreamState> get stateBroadcast;
  Stream<StreamEvent> get eventsStream;

  Future<void> dispose();

  Future<oxidized.Result<void, AnyhowException>> seekToTimestampMs(BigInt tsMs);
  Future<oxidized.Result<void, AnyhowException>> wscRtpGoLive();
  Future<oxidized.Result<void, AnyhowException>> setSpeed(double speed);
}
