import 'dart:async';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/api/simple.dart' as rlib;
import 'package:flutter_realtime_player/rust/dart_types.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import "package:rxdart/rxdart.dart" as rx;
import 'rust/core/types.dart';

class VideoController {
  final String url;
  final bool mute;
  final bool autoRestart;
  Map<String, String>? ffmpegOptions;

  final int sessionId;
  final rx.BehaviorSubject<StreamState> stateBroadcast;
  final StreamSubscription<StreamState> _originalSub;
  bool _running = false;
  int? _engineHandle;

  VideoController(
    StreamSubscription<StreamState> originalSub, {
    required this.url,
    required this.sessionId,
    required this.stateBroadcast,
    this.mute = true,
    this.autoRestart = false,
    this.ffmpegOptions,
  }) : _originalSub = originalSub;

  Future<void> dispose() async {
    _running = false;
    await rlib.destroyStreamSession(sessionId: sessionId);
    _originalSub.cancel();
  }

  static Future<(VideoController?, String?)> create({
    required String url,
    required VideoDimensions dimensions,
    bool mute = true,
    bool autoRestart = false,
    Map<String, String>? ffmpegOptions,
  }) async {
    final handle = await EngineContext.instance.getEngineHandle();
    final sessionId = await rlib.createNewSession();

    // play demo video
    try {
      final stream = rlib.createNewPlayable(
        sessionId: sessionId,
        engineHandle: handle,
        ffmpegOptions: ffmpegOptions,
        videoInfo: VideoInfo(
          uri: url,
          dimensions: dimensions,
          mute: mute,
          autoRestart: autoRestart,
        ),
      );
      final bs = rx.BehaviorSubject<StreamState>.seeded(StreamState.loading());
      final origSub = stream.listen(
        bs.add,
        onError: bs.addError,
        onDone: () => bs.close(),
      );
      final ret = VideoController(
        origSub,
        sessionId: sessionId,
        stateBroadcast: bs,
        url: url,
        autoRestart: autoRestart,
        ffmpegOptions: ffmpegOptions,
        mute: mute,
      );
      ret._engineHandle = handle;
      ret._running = true;
      // start ping task
      Future.microtask(() async {
        while (ret._running) {
          // ping rust side to announce we still want the stream.
          rlib.markSessionAlive(sessionId: sessionId);
          await Future.delayed(const Duration(seconds: 1));
        }
      });
      return (ret, null);
    } catch (e) {
      return (null, e.toString());
    }
  }

  /// Seek to a specific time in seconds within the video
  Future<void> seekTo(Duration position) async {
    await rlib.seekToTime(
      sessionId: sessionId,
      timeSeconds: position.inMilliseconds / 1000.0,
    );
  }

  /// Get the current playback time of the video
  Future<Duration> getCurrentPosition() async {
    final timeSeconds = await rlib.getCurrentTime(sessionId: sessionId);
    return Duration(milliseconds: (timeSeconds * 1000).round());
  }

  /// Resize the video stream with new dimensions
  Future<void> resizeStream(VideoDimensions newDimensions) async {
    try {
      await rlib.resizeStreamSession(
        sessionId: sessionId,
        width: newDimensions.width,
        height: newDimensions.height,
      );
    } catch (e) {
      debugPrint('Error resizing stream: $e');
    }
  }
}

// ignore: implementation_imports

class VideoPlayer extends StatefulWidget {
  final VideoController? controller; // Changed to nullable
  final Widget? child;
  final String? url; // Added url parameter
  final Map<String, String>? ffmpegOptions;
  final bool mute;
  final bool autoRestart;
  final bool autoDispose;

  const VideoPlayer._({
    super.key,
    this.controller,
    this.url, // This will be null if controller is provided
    this.child,
    this.autoDispose = true,
  }) : assert(
         controller != null || url != null,
         'Either controller or url must be provided',
       );

  factory VideoPlayer.fromController({
    GlobalKey? key,
    required VideoController controller,
    bool autoDispose = true,
    Widget? child,
  }) {
    return VideoPlayer._(
      key: key,
      controller: controller,
      autoDispose: autoDispose,
      child: child,
    );
  }

  static Widget fromConfig({
    GlobalKey? key,
    required String url,
    Map<String, String>? ffmpegOptions,
    bool mute = true,
    bool autoRestart = false,
    bool autoDispose = true,
    Widget? child,
  }) {
    return LayoutBuilder(
      builder: (context, constraints) {
        // Calculate dimensions from layout constraints
        final width =
            constraints.maxWidth.isFinite ? constraints.maxWidth.floor() : 640;
        final height =
            constraints.maxHeight.isFinite
                ? constraints.maxHeight.floor()
                : 360;
        final dimensions = VideoDimensions(width: width, height: height);

        return FutureBuilder(
          future: _createControllerWithSize(
            url,
            dimensions,
            mute,
            autoRestart,
            ffmpegOptions,
          ),
          builder: (ctx, res) {
            if (res.hasError) {
              return Text(res.error.toString());
            }
            final (controller, err) = res.data!;
            if (err != null) {
              return Text(err);
            }
            return _VideoPlayerWithSize(
              controller: controller!,
              autoDispose: autoDispose,
              initialDimensions: dimensions,
              child: child,
            );
          },
        );
      },
    );
  }

  static Future<(VideoController?, String?)> _createControllerWithSize(
    String url,
    VideoDimensions dimensions,
    bool mute,
    bool autoRestart,
    Map<String, String>? ffmpegOptions,
  ) async {
    return VideoController.create(
      url: url,
      dimensions: dimensions,
      mute: mute,
      autoRestart: autoRestart,
      ffmpegOptions: ffmpegOptions,
    );
  }

  @override
  State<VideoPlayer> createState() => _VideoPlayerState();
}

// New stateful widget that handles size changes
class _VideoPlayerWithSize extends StatefulWidget {
  final VideoController controller;
  final Widget? child;
  final bool autoDispose;
  final VideoDimensions initialDimensions;

  const _VideoPlayerWithSize({
    required this.controller,
    required this.initialDimensions,
    this.child,
    this.autoDispose = true,
  });

  @override
  State<_VideoPlayerWithSize> createState() => _VideoPlayerWithSizeState();
}

class _VideoPlayerWithSizeState extends State<_VideoPlayerWithSize> {
  StreamState? currentState;
  StreamSubscription<StreamState>? streamSubscription;
  late VideoDimensions currentDimensions;
  Timer? _resizeDebounceTimer;
  VideoDimensions? _pendingResize;

  @override
  void initState() {
    super.initState();
    currentDimensions = widget.initialDimensions;

    streamSubscription = widget.controller.stateBroadcast.listen((state) {
      setState(() {
        currentState = state;
      });
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
    return LayoutBuilder(
      builder: (context, constraints) {
        final width =
            constraints.maxWidth.isFinite ? constraints.maxWidth.floor() : 640;
        final height =
            constraints.maxHeight.isFinite
                ? constraints.maxHeight.floor()
                : 360;
        final newDimensions = VideoDimensions(width: width, height: height);

        // If the widget resized and it's different from our current dimensions, update the stream
        if (newDimensions.width != currentDimensions.width ||
            newDimensions.height != currentDimensions.height) {
          _handleResize(newDimensions);
        }

        if (currentState == null) {
          return loadingWidget('initializing...');
        }
        switch (currentState!) {
          case StreamState_Loading():
            return loadingWidget('initializing stream...');
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
              child: Text(
                'Video stopped',
                style: const TextStyle(fontSize: 16),
              ),
            );
        }
      },
    );
  }

  void _handleResize(VideoDimensions newDimensions) {
    // Only resize when the decoder is in playing or loading state
    // Don't resize when in error or stopped state
    if (currentState is StreamState_Playing ||
        currentState is StreamState_Loading) {
      // Store the pending resize dimensions
      _pendingResize = newDimensions;

      // Cancel any existing timer
      _resizeDebounceTimer?.cancel();

      // Create a new debounce timer (300ms delay)
      _resizeDebounceTimer = Timer(const Duration(milliseconds: 300), () {
        if (_pendingResize != null && mounted) {
          final dimensionsToApply = _pendingResize!;
          _pendingResize = null;

          debugPrint("resize from $currentDimensions to $dimensionsToApply");

          setState(() {
            currentDimensions = dimensionsToApply;
          });

          widget.controller.resizeStream(dimensionsToApply);
        }
      });
    }
  }

  @override
  void dispose() {
    // Cancel the debounce timer to prevent memory leaks
    _resizeDebounceTimer?.cancel();
    _resizeDebounceTimer = null;
    _pendingResize = null;

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

class _VideoPlayerState extends State<VideoPlayer> {
  @override
  Widget build(BuildContext context) {
    // If a controller was provided, use it directly
    if (widget.controller != null) {
      return _VideoPlayerWithSize(
        controller: widget.controller!,
        initialDimensions: const VideoDimensions(
          width: 640,
          height: 360,
        ), // Fallback dimensions
        autoDispose: widget.autoDispose,
        child: widget.child,
      );
    }
    // This case should not happen due to assertion, but we include it to avoid errors
    return const SizedBox.shrink();
  }
}
