import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart' show VideoConfig;
import 'package:flutter_realtime_player/video_player.dart';

class StreamWindowPage extends StatelessWidget {
  final VideoConfig config;
  const StreamWindowPage({super.key, required this.config});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Stream Window')),
      body: Center(
        child: SizedBox(
          width: double.infinity,
          height: 300,
          child: VideoPlayer.fromConfig(config: config),
        ),
      ),
    );
  }
}
