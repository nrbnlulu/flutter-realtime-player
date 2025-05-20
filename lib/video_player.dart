import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:irondash_engine_context/irondash_engine_context.dart';

import 'rust/core/types.dart';

class VideoController {
  final String url;
  final bool mute;
  int? textureId;

  VideoController({required this.url, this.mute = true});

  Future<void> dispose() async {
    if (textureId != null) {
      await rlib.destroyStreamSession(textureId: textureId!);
    }
  }

  Future<(int?, String?)> init() async {
    int? textureId;
    String? error;

    final handle = await EngineContext.instance.getEngineHandle();
    // play demo video
    try {
      textureId = await rlib.createNewPlayable(
        engineHandle: handle,
        videInfo: VideoInfo(
          uri: url,
          dimensions: const VideoDimensions(width: 640, height: 360),
          mute: mute,
        ),
      );
    } catch (e) {
      error = e.toString();
    }
    return (textureId, error);
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
    bool mute = true,
    Widget? child,
  }) {
    return VideoPlayer._(
      key: key,
      controller: VideoController(url: url, mute: mute),
      child: child,
    );
  }

  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

class _VideoPlayerState extends State<VideoPlayer> {
  int? textureId;
  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: widget.controller.init(),
      builder: (context, snapshot) {
        if (snapshot.data != null) {
          final data = snapshot.data!;
          if (data.$2 != null) {
            return Text("Error: ${data.$2}");
          }
          if (data.$1 != null) {
            textureId = data.$1;
            return Stack(
              children: [
                Texture(textureId: data.$1!),
                widget.child ?? const SizedBox(),
              ],
            );
          }
        }
        return const CircularProgressIndicator();
      },
    );
  }

  @override
  void dispose() {
    super.dispose();
    Future.microtask(() async {
      if (textureId != null) {
        await rlib.destroyStreamSession(textureId: textureId!);
        textureId = null;
      }
    });
  }
}
