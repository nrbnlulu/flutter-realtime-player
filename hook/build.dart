import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

void main(List<String> args) async {
  //todo: get envs from host system environments 
  final envVars = <String, String>{
    "PKG_CONFIG_SYSROOT_DIR_x86_64_linux_android":
        "PKG_CONFIG_SYSROOT_DIR_x86_64_linux_android",
    "GSTREAMER_ROOT_ANDROID": "GSTREAMER_ROOT_ANDROID",
  };
  
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