import 'package:flutter/material.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:flutter_gstreamer/rust/api/simple.dart' as rlib;

import 'rust/core/types.dart';

// ignore: implementation_imports

class VideoPlayer extends StatefulWidget {
  const VideoPlayer({super.key, required this.url});

  final String url;
  final int? textureId = null;
  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

class _VideoPlayerState extends State<VideoPlayer> {
  int? textureId;
  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: () async {
        int? textureId;
        String? error;

        final handle = await EngineContext.instance.getEngineHandle();
        // play demo video
        try {
          textureId = await rlib.createNewPlayable(
            engineHandle: handle,
            videInfo: VideoInfo(
              uri: widget.url,
              dimensions: const VideoDimensions(width: 640, height: 360),
              mute: true,
            ),
          );
        } catch (e) {
          error = e.toString();
        }
        return (textureId, error);
      }(),
      builder: (context, snapshot) {
        if (snapshot.data != null) {
          final data = snapshot.data!;
          if (data.$2 != null) {
            return Text("Error: ${data.$2}");
          }
          if (data.$1 != null) {
            textureId = data.$1;
            return Texture(textureId: data.$1!);
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
