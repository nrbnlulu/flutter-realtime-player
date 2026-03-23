import 'dart:io';

import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

void main(List<String> args) async {  
  final envVars = Platform.environment;

  await build(args, (input, output) async {
    await RustBuilder(
      assetName: 'rust/frb_generated.io.dart',
      cratePath: 'rust',
      extraCargoEnvironmentVariables: envVars,
    ).run(
      input: input,
      output: output,
    );
  });
}