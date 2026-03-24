import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

import 'env_utilizer.dart';

const _prebuiltStreamerRootEnvVar = 'GSTREAMER_ROOT_ANDROID';
const _pkgConfigSysrootEnvVar = 'PKG_CONFIG_SYSROOT_DIR';

void main(List<String> args) async {  
  final envFile = Env.instance;

  await build(args, (input, output) async {
    await RustBuilder(
      assetName: 'flutter_realtime_player',
      cratePath: 'rust',
      extraCargoEnvironmentVariables: {
        _prebuiltStreamerRootEnvVar: envFile.getString(
          _prebuiltStreamerRootEnvVar,
        ),
        _pkgConfigSysrootEnvVar: envFile.getString(_prebuiltStreamerRootEnvVar),
      },
    ).run(
      input: input,
      output: output,
    );
  });
}