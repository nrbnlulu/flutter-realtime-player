import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

import 'env_utilizer.dart';

const _prebuiltStreamerRootEnvVar = 'GSTREAMER_ROOT_ANDROID';

void main(List<String> args) async {  
  final envVars = Env.instance;

  await build(args, (input, output) async {
    await RustBuilder(
      assetName: 'flutter_realtime_player',
      cratePath: 'rust',
      extraCargoEnvironmentVariables: {
        _prebuiltStreamerRootEnvVar: envVars.getString(
          _prebuiltStreamerRootEnvVar,
        ),
      },
    ).run(
      input: input,
      output: output,
    );
  });
}