import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_realtime_player/rust/core/types.dart'
    show VideoDimensions;
import 'package:flutter_realtime_player/video_player.dart';

class SessionMode {
  final bool isLive;
  final int currentTimeMs;
  final double speed;

  SessionMode({
    required this.isLive,
    required this.currentTimeMs,
    required this.speed,
  });

  factory SessionMode.fromJson(Map<String, dynamic> json) {
    return SessionMode(
      isLive: json['is_live'] as bool,
      currentTimeMs: json['current_time_ms'] as int,
      speed: (json['speed'] as num).toDouble(),
    );
  }
}

class TrtpClient {
  TrtpClient({required String baseUrl, required this.sourceId})
    : _baseUri = Uri.parse(baseUrl);

  final Uri _baseUri;
  final String sourceId;
  final HttpClient _httpClient = HttpClient();

  String? token;
  int? serverPort;
  int? clientPort;
  bool _connected = false;
  Timer? _refreshTimer;

  Future<void> connect({int? requestedClientPort}) async {
    if (_connected) {
      return;
    }

    final announcePort = requestedClientPort ?? await _pickEphemeralPort();
    final registerUri = _buildUri('/streams/$sourceId/rtp');
    final registerBody = <String, dynamic>{'client_port': announcePort};
    final registerResp = await _postJson(registerUri, registerBody);
    if (registerResp.statusCode < 200 || registerResp.statusCode >= 300) {
      throw Exception('TRTP register failed: ${registerResp.body}');
    }

    final data = jsonDecode(registerResp.body) as Map<String, dynamic>;
    token = data['token']?.toString();
    serverPort = _readInt(data['server_port']);
    clientPort = announcePort;
    final refreshIntervalSecs = _readInt(
      data['refresh_interval_secs'] ??
          data['refresh_interval_sec'] ??
          data['keepalive_interval_secs'] ??
          data['keepalive_interval_sec'],
    );

    if (token == null || serverPort == null) {
      throw Exception('TRTP register response missing token/server_port');
    }

    await _sendHolepunch(announcePort);
    _connected = true;
    _startRefreshTimer(refreshIntervalSecs ?? 10);
  }

  Future<String> fetchSdp() async {
    if (token == null) {
      throw Exception('TRTP not connected');
    }
    final sdpUri = _buildUri(
      '/streams/$sourceId/rtp/sdp',
      queryParameters: {'token': token!},
    );
    final resp = await _get(sdpUri);
    if (resp.statusCode != 200) {
      throw Exception('Failed to fetch SDP: ${resp.body}');
    }
    return _normalizeSdp(resp.body);
  }

  Future<void> refresh() async {
    if (token == null) {
      return;
    }
    final refreshUri = _buildUri('/streams/$sourceId/rtp/refresh');
    await _postJson(refreshUri, {'token': token});
  }

  Future<void> seekToTimestamp(int timestampMs) async {
    if (token == null) {
      throw Exception('TRTP not connected');
    }
    final seekUri = _buildUri('/streams/$sourceId/rtp/$token/seek');
    final resp = await _postJson(seekUri, {'timestamp': timestampMs});
    if (resp.statusCode != 200) {
      throw Exception('Seek failed: ${resp.body}');
    }
  }

  Future<void> setSpeed(double speed) async {
    if (token == null) throw Exception('TRTP not connected');
    final uri = _buildUri('/streams/$sourceId/rtp/$token/speed');
    final resp = await _postJson(uri, {'speed': speed});
    if (resp.statusCode != 200) {
      throw Exception('Set speed failed: ${resp.body}');
    }
  }

  Future<SessionMode> getStatus() async {
    if (token == null) throw Exception('TRTP not connected');
    final uri = _buildUri('/streams/$sourceId/rtp/$token/status');
    final resp = await _get(uri);
    if (resp.statusCode != 200) {
      throw Exception('Get status failed: ${resp.body}');
    }
    return SessionMode.fromJson(jsonDecode(resp.body));
  }

  void disconnect() {
    _refreshTimer?.cancel();
    _refreshTimer = null;
    _connected = false;
    token = null;
  }

  void dispose() {
    disconnect();
    _httpClient.close(force: true);
  }

  Future<int> _pickEphemeralPort() async {
    final socket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, 0);
    final port = socket.port;
    socket.close();
    return port;
  }

  Future<void> _sendHolepunch(int port) async {
    if (token == null || serverPort == null) {
      return;
    }
    final addresses = await InternetAddress.lookup(_baseUri.host);
    final target = addresses.first;
    final socket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, port);
    final payload = 't5rtp $token $port';
    socket.send(utf8.encode(payload), target, serverPort!);
    await Future.delayed(const Duration(milliseconds: 100));
    socket.close();
  }

  void _startRefreshTimer(int intervalSecs) {
    _refreshTimer?.cancel();
    _refreshTimer = Timer.periodic(
      Duration(seconds: intervalSecs.clamp(1, 3600)),
      (_) => refresh(),
    );
  }

  Uri _buildUri(String path, {Map<String, String>? queryParameters}) {
    return _baseUri.replace(path: path, queryParameters: queryParameters);
  }

  Future<_HttpResponse> _get(Uri uri) async {
    final request = await _httpClient.getUrl(uri);
    final response = await request.close();
    final body = await response.transform(utf8.decoder).join();
    return _HttpResponse(response.statusCode, body);
  }

  Future<_HttpResponse> _postJson(Uri uri, Map<String, dynamic> body) async {
    final request = await _httpClient.postUrl(uri);
    request.headers.contentType = ContentType.json;
    request.write(jsonEncode(body));
    final response = await request.close();
    final payload = await response.transform(utf8.decoder).join();
    return _HttpResponse(response.statusCode, payload);
  }

  int? _readInt(dynamic value) {
    if (value is int) {
      return value;
    }
    if (value is String) {
      return int.tryParse(value);
    }
    return null;
  }

  String _normalizeSdp(String sdp) {
    var normalized = sdp.replaceAll('\r\n', '\n').replaceAll('\n', '\r\n');
    if (!normalized.endsWith('\r\n')) {
      normalized = '$normalized\r\n';
    }
    return normalized;
  }
}

class _HttpResponse {
  _HttpResponse(this.statusCode, this.body);

  final int statusCode;
  final String body;
}

class TrtpSeekDemo extends StatefulWidget {
  const TrtpSeekDemo({super.key});

  @override
  State<TrtpSeekDemo> createState() => _TrtpSeekDemoState();
}

class _TrtpSeekDemoState extends State<TrtpSeekDemo> {
  final TextEditingController _baseUrlController = TextEditingController(
    text: 'http://localhost:8009',
  );
  final TextEditingController _sourceIdController = TextEditingController(
    text: '1',
  );
  final TextEditingController _clientPortController = TextEditingController();

  TrtpClient? _client;
  VideoController? _controller;
  File? _sdpFile;
  bool _isConnecting = false;
  String? _errorMessage;
  Timer? _statusTimer;
  SessionMode? _sessionMode;

  @override
  void dispose() {
    _statusTimer?.cancel();
    _stopPlayback();
    _baseUrlController.dispose();
    _sourceIdController.dispose();
    _clientPortController.dispose();
    super.dispose();
  }

  Future<void> _connect() async {
    if (_isConnecting) return;

    setState(() {
      _isConnecting = true;
      _errorMessage = null;
    });

    await _stopPlayback();

    final client = TrtpClient(
      baseUrl: _baseUrlController.text.trim(),
      sourceId: _sourceIdController.text.trim(),
    );

    try {
      final requestedPort = int.tryParse(_clientPortController.text.trim());
      await client.connect(requestedClientPort: requestedPort);
      final sdp = await client.fetchSdp();
      final sdpFile = await _writeSdpFile(sdp, client.token ?? 'unknown');

      final dimensions = const VideoDimensions(width: 1280, height: 720);
      final ffmpegOptions = <String, String>{
        'protocol_whitelist': 'file,udp,rtp',
        'fflags': 'nobuffer',
        'flags': 'low_delay',
        'analyzeduration': '0',
        'probesize': '32',
        if (client.clientPort != null)
          'local_rtpport': client.clientPort.toString(),
        if (client.clientPort != null)
          'local_rtcpport': '${client.clientPort! + 1}',
      };

      final result = await VideoController.create(
        url: sdpFile.path,
        dimensions: dimensions,
        autoRestart: true,
        ffmpegOptions: ffmpegOptions,
      );

      if (result.$1 == null) {
        throw Exception(result.$2 ?? 'Failed to create player');
      }

      if (mounted) {
        setState(() {
          _client = client;
          _controller = result.$1;
          _sdpFile = sdpFile;
          _isConnecting = false;
        });
        _startStatusPolling();
      }
    } catch (e) {
      client.dispose();
      if (mounted) {
        setState(() {
          _errorMessage = e.toString();
          _isConnecting = false;
        });
      }
    }
  }

  void _startStatusPolling() {
    _statusTimer?.cancel();
    _statusTimer = Timer.periodic(const Duration(seconds: 1), (timer) async {
      if (_client == null || !_client!._connected) {
        timer.cancel();
        return;
      }
      try {
        final status = await _client!.getStatus();
        if (mounted) {
          setState(() {
            _sessionMode = status;
          });
        }
      } catch (e) {
        debugPrint('Status poll failed: $e');
      }
    });
  }

  Future<void> _stopPlayback() async {
    _statusTimer?.cancel();
    _statusTimer = null;
    await _controller?.dispose();
    _controller = null;
    _client?.dispose();
    _client = null;
    if (_sdpFile != null) {
      try {
        await _sdpFile!.delete();
      } catch (_) {}
    }
    _sdpFile = null;
    _sessionMode = null;
    if (mounted) setState(() {});
  }

  Future<void> _seekRelative(int seconds) async {
    if (_sessionMode == null || _client == null) return;
    final current = _sessionMode!.currentTimeMs;
    final target = current + (seconds * 1000);
    await _client!.seekToTimestamp(target);
    // Instant optimistic update
    setState(() {
      _sessionMode = SessionMode(
        isLive: false,
        currentTimeMs: target,
        speed: _sessionMode!.speed,
      );
    });
  }

  Future<void> _seekToNow() async {
    if (_client == null) return;
    final now = DateTime.now().millisecondsSinceEpoch;
    // Adding a small buffer to ensure server treats it as "live" switch
    await _client!.seekToTimestamp(now + 1000);
    setState(() {
      _sessionMode = SessionMode(
        isLive: true,
        currentTimeMs: now,
        speed: 1.0,
      );
    });
  }

  Future<void> _setSpeed(double speed) async {
    if (_client == null) return;
    try {
      await _client!.setSpeed(speed);
      setState(() {
        if (_sessionMode != null) {
          _sessionMode = SessionMode(
            isLive: _sessionMode!.isLive,
            currentTimeMs: _sessionMode!.currentTimeMs,
            speed: speed,
          );
        }
      });
    } catch (e) {
      debugPrint("Set speed failed: $e");
    }
  }

  Future<void> _pickDateTime() async {
    final now = DateTime.now();
    final date = await showDatePicker(
      context: context,
      initialDate: now,
      firstDate: now.subtract(const Duration(days: 30)),
      lastDate: now,
    );
    if (date == null) return;

    if (!mounted) return;
    final time = await showTimePicker(
      context: context,
      initialTime: TimeOfDay.fromDateTime(now),
    );
    if (time == null) return;

    final dt = DateTime(
      date.year,
      date.month,
      date.day,
      time.hour,
      time.minute,
    );
    await _client!.seekToTimestamp(dt.millisecondsSinceEpoch);
  }

  Future<File> _writeSdpFile(String sdp, String token) async {
    final dir = await Directory.systemTemp.createTemp('trtp_sdp_');
    final file = File('${dir.path}/trtp_$token.sdp');
    await file.writeAsString(sdp);
    return file;
  }

  String _formatTime(int ms) {
    final dt = DateTime.fromMillisecondsSinceEpoch(ms);
    final h = dt.hour.toString().padLeft(2, '0');
    final m = dt.minute.toString().padLeft(2, '0');
    final s = dt.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }

  @override
  Widget build(BuildContext context) {
    if (_client == null) {
      return _buildConnectionForm();
    }
    return _buildPlayerInterface();
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
                const SizedBox(height: 24),
                if (_errorMessage != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 16),
                    child: Text(
                      _errorMessage!,
                      style: const TextStyle(color: Colors.red),
                      textAlign: TextAlign.center,
                    ),
                  ),
                ElevatedButton(
                  onPressed: _isConnecting ? null : _connect,
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.all(16),
                  ),
                  child:
                      _isConnecting
                          ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                          : const Text('CONNECT'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildPlayerInterface() {
    final isLive = _sessionMode?.isLive ?? true;
    final currentTime =
        _sessionMode != null
            ? DateTime.fromMillisecondsSinceEpoch(_sessionMode!.currentTimeMs)
            : DateTime.now();

    return Column(
      children: [
        // Header
        Container(
          color: Colors.black87,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          child: Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: isLive ? Colors.red : Colors.blue,
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  isLive ? 'LIVE' : 'DVR',
                  style: const TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                    fontSize: 12,
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Text(
                'Source: ${_sourceIdController.text}',
                style: const TextStyle(color: Colors.white70),
              ),
              const Spacer(),
              if (_client != null)
                Text(
                  'Token: ${_client!.token?.substring(0, 8)}...',
                  style: const TextStyle(color: Colors.white30, fontSize: 10),
                ),
              const SizedBox(width: 12),
              IconButton(
                icon: const Icon(Icons.close, color: Colors.white),
                onPressed: _stopPlayback,
                tooltip: 'Disconnect',
              ),
            ],
          ),
        ),

        // Player
        Expanded(
          child: Container(
            color: Colors.black,
            child: Center(
              child: AspectRatio(
                aspectRatio: 16 / 9,
                child:
                    _controller != null
                        ? VideoPlayer.fromController(
                          controller: _controller!,
                          autoDispose: false,
                        )
                        : const Center(child: CircularProgressIndicator()),
              ),
            ),
          ),
        ),

        // Controls
        Container(
          color: Colors.grey[200],
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              // Time & Speed Status
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'Time: ${_formatTime(currentTime.millisecondsSinceEpoch)}',
                    style: const TextStyle(
                      fontWeight: FontWeight.bold,
                      fontSize: 16,
                    ),
                  ),
                  Text('Speed: ${_sessionMode?.speed.toStringAsFixed(1)}x'),
                ],
              ),
              const SizedBox(height: 12),

              // Seek Controls
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  _buildSeekBtn('-60s', -60),
                  const SizedBox(width: 8),
                  _buildSeekBtn('-10s', -10),
                  const SizedBox(width: 16),
                  _buildSeekBtn('+10s', 10),
                  const SizedBox(width: 8),
                  _buildSeekBtn('+60s', 60),
                ],
              ),
              const SizedBox(height: 12),

              // Playback Controls
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  // Go Live
                  ElevatedButton.icon(
                    onPressed: isLive ? null : _seekToNow,
                    icon: const Icon(Icons.fiber_dvr, color: Colors.red),
                    label: const Text('GO LIVE'),
                  ),
                  const SizedBox(width: 16),

                  // Pause/Play (Speed 0 vs 1)
                  IconButton.filled(
                    onPressed:
                        () => _setSpeed((_sessionMode?.speed ?? 1) == 0 ? 1 : 0),
                    icon: Icon(
                      (_sessionMode?.speed ?? 1) == 0
                          ? Icons.play_arrow
                          : Icons.pause,
                    ),
                    iconSize: 32,
                  ),
                  const SizedBox(width: 16),

                  // Speed Selector
                  PopupMenuButton<double>(
                    initialValue: _sessionMode?.speed,
                    onSelected: _setSpeed,
                    itemBuilder:
                        (context) => [
                          const PopupMenuItem(value: 0.5, child: Text('0.5x')),
                          const PopupMenuItem(value: 1.0, child: Text('1.0x')),
                          const PopupMenuItem(value: 2.0, child: Text('2.0x')),
                          const PopupMenuItem(value: 5.0, child: Text('5.0x')),
                        ],
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        border: Border.all(color: Colors.grey),
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Row(
                        children: [
                          Text('${_sessionMode?.speed ?? 1.0}x'),
                          const Icon(Icons.arrow_drop_down),
                        ],
                      ),
                    ),
                  ),

                  const SizedBox(width: 16),
                  // Custom Seek
                  IconButton(
                    onPressed: _pickDateTime,
                    icon: const Icon(Icons.calendar_today),
                    tooltip: 'Seek to Date/Time',
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildSeekBtn(String label, int seconds) {
    return OutlinedButton(
      onPressed: () => _seekRelative(seconds),
      child: Text(label),
    );
  }
}
