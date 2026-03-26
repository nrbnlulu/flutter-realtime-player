// used Gist from here: https://gist.github.com/nrbnlulu/bd5439dfda1538298167a52eea3f6936

import 'dart:io' show Platform, File;

class Env {
  Map<String, String> _env = {};

  static final Env instance = Env._();

  Env._() {
    // read cwd .env
    _env = Platform.environment.map((key, value) => MapEntry(key, value));
    // find $HOME/cross_build.env file
    final home =
        Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'];
    final file = File('$home/cross_build.env');
    // coverage:ignore-start
    if (file.existsSync()) {
      final lines = file.readAsLinesSync();
      for (final line in lines) {
        int index = line.indexOf("=");
        if (index != -1) {
          _env[line.substring(0, index).trim()] = line.substring(index + 1).trim();
        }
      }
    }
    // coverage:ignore-end
  }
  void set(String key, String value) {
    _env[key] = value;
  }

  void setDefault<T>(String key, T value) {
    _env.putIfAbsent(key, () => value.toString());
  }

  void setBool(String key, bool value) {
    set(key, value.toString());
  }

  String getString(String key, {String defaultValue = ''}) {
    return _env[key] ?? defaultValue;
  }

  bool getBool(String key, {bool defaultValue = false}) {
    final found = _env[key];
    if (found == null) {
      return defaultValue;
    }
    return found.toLowerCase() == 'true';
  }
}
