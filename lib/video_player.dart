import 'package:flutter/material.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:my_app/src/rust/api/simple.dart' as rlib;
import 'package:my_app/src/rust/core/types.dart';

class VideoPlayer extends StatelessWidget {
  const VideoPlayer({
    super.key,
    required this.url,
  });

  final String url;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder(
      future: () async {
        final handle = await EngineContext.instance.getEngineHandle();
        // play demo video
        try {
          final texture = await rlib.createNewPlayable(
              engineHandle: handle,
              videInfo: VideoInfo(
                  uri: url,
                  dimensions: const VideoDimensions(width: 640, height: 360),
                  mute: true));

          return texture;
        } catch (e) {
          print(e);
        }
      }(),
      builder: (context, snapshot) {
        if (snapshot.connectionState == ConnectionState.done) {
          debugPrint("snapshot.data: ${snapshot.data}");
          return Texture(textureId: snapshot.data!);
        } else {
          return const CircularProgressIndicator();
        }
      },
    );
  }
}
