import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart'
    show WscRtpSessionConfig;
import 'wsc_rtp_player.dart';

class WscRtpSeekDemo extends StatefulWidget {
  const WscRtpSeekDemo({super.key});

  @override
  State<WscRtpSeekDemo> createState() => _WscRtpSeekDemoState();
}

class _WscRtpSeekDemoState extends State<WscRtpSeekDemo> {
  final TextEditingController _baseUrlController = TextEditingController(
    text: 'http://localhost:8009',
  );
  final TextEditingController _sourceIdController = TextEditingController(
    text: '1',
  );
  final TextEditingController _clientPortController = TextEditingController();

  bool _forceWebsocketTransport = false;

  // When non-null, the player is shown. Set to null to go back to the form.
  WscRtpSessionConfig? _config;

  @override
  void dispose() {
    _baseUrlController.dispose();
    _sourceIdController.dispose();
    _clientPortController.dispose();
    super.dispose();
  }

  void _connect() {
    final requestedPort = int.tryParse(_clientPortController.text.trim());
    setState(() {
      _config = WscRtpSessionConfig(
        autoRestart: true,
        baseUrl: _baseUrlController.text.trim(),
        sourceId: _sourceIdController.text.trim(),
        clientPort: requestedPort,
        forceWebsocketTransport: _forceWebsocketTransport,
      );
    });
  }

  void _disconnect() {
    setState(() {
      _config = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final config = _config;
    if (config != null) {
      return Stack(
        children: [
          WscRtpPlayerWidget(key: ValueKey(config), config: config),
          Positioned(
            top: 8,
            right: 8,
            child: Material(
              color: Colors.black54,
              borderRadius: BorderRadius.circular(20),
              child: IconButton(
                icon: const Icon(Icons.close, color: Colors.white),
                tooltip: 'Disconnect',
                onPressed: _disconnect,
              ),
            ),
          ),
        ],
      );
    }

    return _buildConnectionForm();
  }

  Widget _buildConnectionForm() {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 400),
        child: Card(
          margin: const EdgeInsets.all(32),
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'Connect to Stream',
                  style: Theme.of(context).textTheme.headlineSmall,
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 24),
                TextField(
                  controller: _baseUrlController,
                  decoration: const InputDecoration(
                    labelText: 'Base URL',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.link),
                  ),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _sourceIdController,
                  decoration: const InputDecoration(
                    labelText: 'Source ID',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.videocam),
                  ),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _clientPortController,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    labelText: 'Client Port (Optional)',
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.router),
                  ),
                ),
                const SizedBox(height: 16),
                SwitchListTile(
                  title: const Text('Force WebSocket Transport'),
                  subtitle: const Text('Skip UDP, use WS for RTP'),
                  value: _forceWebsocketTransport,
                  onChanged: (value) {
                    setState(() {
                      _forceWebsocketTransport = value;
                    });
                  },
                  contentPadding: EdgeInsets.zero,
                ),
                const SizedBox(height: 16),
                ElevatedButton(
                  onPressed: _connect,
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.all(16),
                  ),
                  child: const Text('CONNECT'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
