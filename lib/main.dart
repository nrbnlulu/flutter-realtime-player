import 'dart:convert';

import 'dart:ui';

import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'package:flutter/material.dart';
import 'dart:ffi' as ffi;

import 'package:my_app/src/rust/api/simple.dart' as rlib;
import 'package:my_app/src/rust/frb_generated.dart' as rlib_gen;
import 'package:my_app/event_widget.dart';
import 'package:my_app/video_player.dart';

Future<void> main(List<String> args) async {
  WidgetsFlutterBinding.ensureInitialized();

  await rlib_gen.RustLib.init();
  rlib.flutterGstreamerInit(ffiPtr: ffi.NativeApi.initializeApiDLData.address);
  if (args.firstOrNull == 'multi_window') {
    final windowId = int.parse(args[1]);
    final argument = args[2].isEmpty
        ? const {}
        : jsonDecode(args[2]) as Map<String, dynamic>;

    runApp(_ExampleSubWindow(
      windowController: WindowController.fromWindowId(windowId),
      args: argument,
    ));
  } else {
    runApp(const _ExampleMainWindow());
  }
}

class _ExampleMainWindow extends StatefulWidget {
  const _ExampleMainWindow({Key? key}) : super(key: key);

  @override
  State<_ExampleMainWindow> createState() => _ExampleMainWindowState();
}

class _ExampleMainWindowState extends State<_ExampleMainWindow> {
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(
          title: const Text('Plugin example app'),
        ),
        body: Column(
          children: [
            TextButton(
              onPressed: () async {
                final window =
                    await DesktopMultiWindow.createWindow(jsonEncode({
                  'args1': 'Sub window',
                  'url':
                      "https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_30mb.mp4",
                }));
                window
                  ..setFrame(const Offset(0, 0) & const Size(1280, 720))
                  ..center()
                  ..setTitle('Another window')
                  ..show();
              },
              child: const Text('Create a new World!'),
            ),
            TextButton(
              child: const Text('Send event to all sub windows'),
              onPressed: () async {
                final subWindowIds =
                    await DesktopMultiWindow.getAllSubWindowIds();
                for (final windowId in subWindowIds) {
                  DesktopMultiWindow.invokeMethod(
                    windowId,
                    'broadcast',
                    'Broadcast from main window',
                  );
                }
              },
            ),
            Expanded(
              child: EventWidget(controller: WindowController.fromWindowId(0)),
            )
          ],
        ),
      ),
    );
  }
}

class _ExampleSubWindow extends StatelessWidget {
  const _ExampleSubWindow({
    Key? key,
    required this.windowController,
    required this.args,
  }) : super(key: key);

  final WindowController windowController;
  final Map? args;

  @override
  Widget build(BuildContext context) {
    final url = args?['url'] as String;
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(
          title: const Text('Plugin example app'),
        ),
        body: Column(
          children: [
            if (args != null)
              Text(
                'Arguments: ${args.toString()}',
                style: const TextStyle(fontSize: 20),
              ),
            TextButton(
              onPressed: () async {
                windowController.close();
              },
              child: const Text('Close this window'),
            ),
            Expanded(
                child: VideoPlayer(
              url: url,
            )),
          ],
        ),
      ),
    );
  }
}
