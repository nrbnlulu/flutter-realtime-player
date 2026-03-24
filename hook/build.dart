import 'dart:io';

import 'package:hooks/hooks.dart';
import 'package:logging/logging.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

const _prebuiltStreamerRootEnvVar = 'GSTREAMER_ROOT_ANDROID';

void main(List<String> args) async {  
  final logger = Logger('html_to_markdown_rust');
  logger.onRecord.listen((record) {
    // ignore: avoid_print
    print('${record.level.name}: ${record.time}: ${record.message}');
  });

  final envVars = Platform.environment;

  await build(args, (input, output) async {
    await RustBuilder(
      assetName: 'rust/frb_generated.dart',
      cratePath: 'rust',
      extraCargoEnvironmentVariables: {
        _prebuiltStreamerRootEnvVar: envVars[_prebuiltStreamerRootEnvVar] ?? '',
      },
    ).run(
      input: input,
      output: output,
      logger: logger,
    );
  });
}