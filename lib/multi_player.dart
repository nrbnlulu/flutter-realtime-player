import 'package:flutter/material.dart';
import 'package:my_app/main.dart';
import 'package:my_app/video_player.dart';

class MultiPlayer extends StatelessWidget {
  const MultiPlayer({
    super.key,
    required this.urls,
  });

  final List<String> urls;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      children: [
        for (final url in urls)
          SizedBox(
            height: 250,
            width: 400,
            child: VideoPlayer(url: url),
          ),
      ],
    );
  }
}
