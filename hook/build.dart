import 'dart:io' show Directory, File, Process, stderr;

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

import 'env_utilizer.dart';

const _prebuiltStreamerRootEnvVar = 'GSTREAMER_ROOT_ANDROID';
const _pkgConfigSysrootx8664EnvVar =
    'PKG_CONFIG_SYSROOT_DIR_x86_64_linux_android';
const _pkgConfigSysrootAarch64EnvVar =
    'PKG_CONFIG_SYSROOT_DIR_aarch64_linux_android';
const _androidNDKHomeEnvVar = 'ANDROID_NDK_HOME';

void main(List<String> args) async {
  // we need to read an standard env file in a known-well path `$HOME/cross_build.env` to get the env vars for building,
  //since the hook is filtering environment variables and there is a known issue about this: https://github.com/dart-lang/native/issues/2623
  final envFile = Env.instance;

  final ndkHome = envFile.getString(_androidNDKHomeEnvVar);
  String? ndkPrebuiltRoot;
  String? pkgConfigSysrootDir;
  // avoid populating empty NDK home env var
  if (ndkHome.isNotEmpty) {
    ndkPrebuiltRoot = '$ndkHome/toolchains/llvm/prebuilt/linux-x86_64';
    pkgConfigSysrootDir = '$ndkPrebuiltRoot/sysroot';
  }

  await build(args, (input, output) async {
    await RustBuilder(
      assetName: 'flutter_realtime_player',
      cratePath: 'rust',
      extraCargoEnvironmentVariables: {
        _prebuiltStreamerRootEnvVar: envFile.getString(
          _prebuiltStreamerRootEnvVar,
        ),
        _pkgConfigSysrootx8664EnvVar: pkgConfigSysrootDir ?? '',
        _pkgConfigSysrootAarch64EnvVar: pkgConfigSysrootDir ?? '',
        _androidNDKHomeEnvVar: ndkHome,
        // Pass Windows GStreamer environment variables if present
        'GSTREAMER_1_0_ROOT_MSVC_X86_64': envFile.getString(
          'GSTREAMER_1_0_ROOT_MSVC_X86_64',
        ),
        'PKG_CONFIG_PATH': envFile.getString(
          'PKG_CONFIG_PATH',
          defaultValue: '',
        ),
      },
    ).run(input: input, output: output);

    if (input.config.buildCodeAssets &&
        input.config.code.targetOS == OS.linux) {
      await _bundleLinuxGStreamer(input, output);
    }
  });
}

// ---------------------------------------------------------------------------
// Linux GStreamer bundling
// ---------------------------------------------------------------------------

/// GLib shared libraries used by the bundled GStreamer build.
const _glibLibs = [
  'libglib-2.0.so.0',
  'libgobject-2.0.so.0',
  'libgmodule-2.0.so.0',
  'libgio-2.0.so.0',
];

/// Core GStreamer shared libraries that the dynamic linker loads when
/// libflutter_realtime_player.so is opened. The $ORIGIN rpath set in build.rs
/// makes the linker look in the same directory as the library itself.
const _gstreamerLibs = [
  'libgstreamer-1.0.so.0',
  'libgstbase-1.0.so.0',
  'libgstapp-1.0.so.0',
  'libgstvideo-1.0.so.0',
  'libgstaudio-1.0.so.0',
  'libgstpbutils-1.0.so.0',
  'libgsttag-1.0.so.0',
  'libgstnet-1.0.so.0',
  'libgstgl-1.0.so.0',
];

/// libav lib name prefixes to bundle from ldd output of libgstlibav.so.
// NOTE: only the FFmpeg libs themselves are bundled here. Their own transitive
// deps (libx264, libvpx, libdav1d, libopus, libass, libdrm, libva, etc.) are
// intentionally left to the system. Bundling them correctly requires skipping
// hardware-specific libs (libOpenCL, libva-*, libvdpau) and display-stack libs
// (libX11, libxcb, libdrm, libvulkan) which must match the running system.
// Target machines are expected to have the relevant codec and display libraries
// installed (e.g. via gst-plugins-ugly, ffmpeg, mesa packages on the distro).
const _libavPrefixes = ['libav', 'libswscale', 'libswresample', 'libpostproc'];

Future<void> _bundleLinuxGStreamer(
  BuildInput input,
  BuildOutputBuilder output,
) async {
  final gstreamerLibDir = await _pkgConfigVar('gstreamer-1.0', 'libdir');
  final glibLibDir = await _pkgConfigVar('glib-2.0', 'libdir');
  final pcre2LibDir = await _pkgConfigVar('libpcre2-8', 'libdir');
  final pluginDir = await _pkgConfigVar('gstreamer-1.0', 'pluginsdir');

  for (final name in _glibLibs) {
    await _stageAndRegister(
      '$glibLibDir/$name',
      name,
      input,
      output,
      required: true,
    );
  }

  await _stageAndRegister(
    '$pcre2LibDir/libpcre2-8.so.0',
    'libpcre2-8.so.0',
    input,
    output,
    required: true,
  );

  for (final name in _gstreamerLibs) {
    await _stageAndRegister(
      '$gstreamerLibDir/$name',
      name,
      input,
      output,
      required: true,
    );
  }

  await _bundlePluginDirectory(pluginDir, input, output);

  // Bundle FFmpeg libs that libgstlibav.so links against (avcodec, avformat…)
  await _bundleLibavTransitiveDeps(
    '$pluginDir/libgstlibav.so',
    gstreamerLibDir,
    input,
    output,
  );
}

/// Runs `pkg-config --variable=<variable> <package>` and returns the value.
Future<String> _pkgConfigVar(String package, String variable) async {
  final pkgConfigPath = Env.instance.getString('PKG_CONFIG_PATH');
  final result = await Process.run(
    'pkg-config',
    ['--variable=$variable', package],
    environment:
        pkgConfigPath.isNotEmpty ? {'PKG_CONFIG_PATH': pkgConfigPath} : null,
  );
  if (result.exitCode != 0) {
    throw Exception(
      'pkg-config --variable=$variable $package failed:\n${result.stderr}',
    );
  }
  return (result.stdout as String).trim();
}

/// Registers every shared object from GStreamer's build-host plugin directory.
/// Flutter places Linux code assets together under the application `lib/`
/// directory, which is used as the isolated plugin directory at runtime.
Future<void> _bundlePluginDirectory(
  String pluginDir,
  BuildInput input,
  BuildOutputBuilder output,
) async {
  final plugins =
      await Directory(pluginDir)
          .list()
          .where((entry) => entry is File && entry.path.endsWith('.so'))
          .cast<File>()
          .toList();
  if (plugins.isEmpty) {
    throw Exception('No GStreamer plugins found in $pluginDir');
  }
  plugins.sort((a, b) => a.path.compareTo(b.path));

  for (final plugin in plugins) {
    final name = plugin.uri.pathSegments.last;
    await _stageAndRegister(plugin.path, name, input, output, required: true);
  }
}

/// Copies [srcPath] into the hook output directory and registers it as a
/// bundled [CodeAsset] so Flutter includes it in the app bundle.
/// Missing optional files are skipped; required bundle components fail the
/// build instead of producing an incomplete application.
Future<void> _stageAndRegister(
  String srcPath,
  String libName,
  BuildInput input,
  BuildOutputBuilder output, {
  bool required = false,
}) async {
  final src = File(srcPath);
  if (!src.existsSync()) {
    if (required) {
      throw Exception('Required Linux bundle file is missing: $srcPath');
    }
    return;
  }

  final destUri = input.outputDirectory.resolve(libName);
  await src.copy(destUri.toFilePath());

  output.assets.code.add(
    CodeAsset(
      package: input.packageName,
      name: 'gstreamer/$libName',
      linkMode: DynamicLoadingBundled(),
      file: destUri,
    ),
  );
}

/// Uses `ldd` on libgstlibav.so to discover FFmpeg .so dependencies and
/// bundles the ones whose names start with a known libav prefix.
Future<void> _bundleLibavTransitiveDeps(
  String libgstlibavPath,
  String libDir,
  BuildInput input,
  BuildOutputBuilder output,
) async {
  if (!File(libgstlibavPath).existsSync()) return;

  final ldd = await Process.run(
    'ldd',
    [libgstlibavPath],
    environment: {'LD_LIBRARY_PATH': libDir},
  );
  if (ldd.exitCode != 0) {
    stderr.writeln('ldd $libgstlibavPath failed; skipping libav bundling');
    return;
  }

  // ldd output lines look like:
  //   libavcodec.so.58 => /usr/lib/x86_64-linux-gnu/libavcodec.so.58 (0x…)
  final linePattern = RegExp(r'(\S+)\s*=>\s*(\S+)');
  for (final line in (ldd.stdout as String).split('\n')) {
    final m = linePattern.firstMatch(line.trim());
    if (m == null) continue;

    final name = m.group(1)!;
    final libPath = m.group(2)!;
    if (libPath == 'not' || libPath.isEmpty) continue;

    if (_libavPrefixes.any((p) => name.startsWith(p))) {
      await _stageAndRegister(libPath, name, input, output);
    }
  }
}
