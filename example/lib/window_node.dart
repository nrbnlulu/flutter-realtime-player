import 'dart:async';
import 'dart:io';

class WindowNode {
  final String name;
  final (String, int) addr;

  const WindowNode({required this.name, required this.addr});
}

class WindowRootNode extends WindowNode {
  final Map<String, (WindowNode, Socket)> children;
  final ServerSocket server;
  StreamSubscription<Socket>? _subscription;

  WindowRootNode({
    required super.name,
    required super.addr,
    required this.server,
    required this.children,
  });

  static Future<WindowRootNode> init(String name) async {
    final server = await ServerSocket.bind("localhost", 0, shared: true);
    return WindowRootNode(
      name: name,
      addr: ("localhost", server.port),
      children: {},
      server: server,
    );
  }

  Future<void> connectionHandler(Socket socket) async {
    await for (var msg in socket) {
      var raw = String.fromCharCodes(msg);
    }
  }

  Future<void> listen() async {
    _subscription = server.listen(connectionHandler);
  }

  Future<void> dispose() async {
    _subscription?.cancel();
  }
}
