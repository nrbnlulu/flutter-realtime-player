import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';

import 'rust/core/types.dart';

class VideoController {
  final String url;
  final bool mute;
  int? sessionId;
  Map<String, String>? ffmpegOptions;
  Stream<StreamState>? _stream;

  Stream<StreamState>? get stream => _stream;

  VideoController({required this.url, this.mute = true, this.ffmpegOptions});

  Future<void> dispose() async {
    if (sessionId != null) {
      await rlib.destroyStreamSession(sessionId: sessionId!);
    }
  }

  Future<(Stream<StreamState>?, String?)> init() async {
    Stream<StreamState>? stream;
    String? error;

    final handle = await EngineContext.instance.getEngineHandle();
    // play demo video
    try {
      stream = rlib.createNewPlayable(
        engineHandle: handle,
        ffmpegOptions: ffmpegOptions,
        videoInfo: VideoInfo(
          uri: url,
          dimensions: const VideoDimensions(width: 640, height: 360),
          mute: mute,
        ),
      );
      _stream = stream;
    } catch (e) {
      error = e.toString();
    }
    return (stream, error);
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

  factory VideoPlayer.fromConfig({
    GlobalKey? key,
    required String url,
    Map<String, String>? ffmpegOptions,
    bool mute = true,
    Widget? child,
  }) {
    return VideoPlayer._(
      key: key,
      controller: VideoController(
        url: url,
        mute: mute,
        ffmpegOptions: ffmpegOptions,
      ),
      child: child,
    );
  }

  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

class _VideoPlayerState extends State<VideoPlayer> {
  StreamState? currentState;
  late Stream<StreamState> streamState;
  StreamSubscription<StreamState>? streamSubscription;

  @override
  void initState() {
    super.initState();
    Future.microtask(() async {
      if (widget.controller.stream case final initiatedStream?) {
        streamState = initiatedStream;
      } else {
        streamState = rlib.createNewPlayable(
          engineHandle: await EngineContext.instance.getEngineHandle(),
          videoInfo: VideoInfo(
            uri: widget.controller.url,
            dimensions: const VideoDimensions(width: 640, height: 360),
            mute: widget.controller.mute,
          ),
        );
      }

      streamSubscription = streamState.listen(
        (state) => setState(() {
          currentState = state;
        }),
      );
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
      case StreamState_Init(sessionId: final sessionId):
        widget.controller.sessionId = sessionId;
        return loadingWidget('initializing stream...');
      case StreamState_Loading():
        return loadingWidget('loading video...');
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
      if (widget.controller.sessionId != null) {
        try {
          await rlib.destroyStreamSession(
            sessionId: widget.controller.sessionId!,
          );
        } catch (e) {
          // Optionally handle the error, e.g., log it
        }
        widget.controller.sessionId = null;
      }
    });
  }
}
