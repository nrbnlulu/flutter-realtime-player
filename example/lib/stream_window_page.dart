import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/video_player.dart';

class StreamWindowPage extends StatelessWidget {
  final String url;
  const StreamWindowPage({super.key, required this.url});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Stream Window')),
      body: Center(
        child: SizedBox(
          width: double.infinity,
          height: 300,
          child: VideoPlayer.fromConfig(url: url),
        ),
      ),
    );
  }
}
