#!/usr/bin/env python3
"""Build and launch the example through the local Android Rust shell scaffold."""

import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tarfile
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent
EXAMPLE = ROOT / "example"
BUILD = EXAMPLE / "build" / "rust-android"
PACKAGE = "dev.flutter.realtimeplayer.rustshell"
TARGETS = {
    "arm64-v8a": (
        "aarch64-linux-android",
        "android-arm64",
        "android_debug_unopt_arm64",
        "arm64",
    ),
    "x86_64": (
        "x86_64-linux-android",
        "android-x64",
        "android_debug_unopt_x64",
        "x86_64",
    ),
}


def run(*args, cwd=ROOT, env=None, capture=False, input_file=None):
    print("+", shlex.join(map(str, args)), flush=True)
    return subprocess.run(
        list(map(str, args)),
        cwd=cwd,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stdin=input_file,
    ).stdout


def select_device(requested):
    output = subprocess.check_output(["adb", "devices", "-l"], text=True)
    devices = [line.split() for line in output.splitlines()[1:] if line.strip()]
    ready = [
        fields[0] for fields in devices if len(fields) > 1 and fields[1] == "device"
    ]
    if requested:
        if requested not in ready:
            raise RuntimeError(
                f"Android device {requested!r} is not connected and authorized.\n{output}"
            )
        return requested
    if len(ready) == 1:
        return ready[0]
    if not ready:
        raise RuntimeError(
            "No authorized Android device found. Connect a device with USB debugging enabled.\n"
            + output
        )
    raise RuntimeError(
        "Several Android devices are connected; select one with --device SERIAL:\n"
        + "\n".join(ready)
    )


def cross_environment():
    env = os.environ.copy()
    config = Path.home() / "cross_build.env"
    if config.is_file():
        # Match the existing cross-build configuration, including shell exports.
        output = subprocess.check_output(
            ["bash", "-c", 'set -a; source "$1"; env -0', "cross-build", str(config)],
            env=env,
        )
        env.update(
            item.decode().split("=", 1) for item in output.split(b"\0") if b"=" in item
        )
    for name in ("ANDROID_NDK_HOME", "GSTREAMER_ROOT_ANDROID"):
        if not env.get(name):
            raise RuntimeError(f"Set {name} in ~/cross_build.env.")
    return env


def toml_value(value):
    if isinstance(value, dict):
        return (
            "{ "
            + ", ".join(f"{key} = {toml_value(item)}" for key, item in value.items())
            + " }"
        )
    if isinstance(value, list):
        return "[" + ", ".join(map(toml_value, value)) + "]"
    return json.dumps(value)


def write_if_changed(path, content):
    if not path.is_file() or path.read_text() != content:
        path.write_text(content)


def prepare_runner(shell):
    """Reuse the SDK's Android entry point and register this player's plugin."""
    template = shell / "crates/flutter-shell-android-runner"
    metadata = tomllib.loads((template / "Cargo.toml").read_text())
    workspace = tomllib.loads((shell / "Cargo.toml").read_text())
    dependencies = metadata["target"]['cfg(target_os = "android")']["dependencies"]
    for dependency in dependencies.values():
        if isinstance(dependency, dict) and "path" in dependency:
            dependency["path"] = str((template / dependency["path"]).resolve())
    dependencies["flutter_realtime_player"] = {"path": str(ROOT / "rust")}
    patches = workspace.get("patch", {}).get("crates-io", {})
    for dependency in patches.values():
        if isinstance(dependency, dict) and "path" in dependency:
            dependency["path"] = str((shell / dependency["path"]).resolve())
    patches["flutter-plugin-sdk"] = {"path": str(shell / "crates/flutter-plugin-sdk")}
    package = workspace["workspace"]["package"]
    manifest = (
        '[package]\nname = "flutter-shell-android-runner"\nversion = "0.1.0"\n'
        f"edition = {toml_value(package['edition'])}\n"
        f"rust-version = {toml_value(package['rust-version'])}\n"
        '[lib]\ncrate-type = ["cdylib"]\n\n[workspace]\n\n[dependencies]\n'
        + "\n".join(
            f"{key} = {toml_value(value)}" for key, value in dependencies.items()
        )
        + "\n\n[patch.crates-io]\n"
        + "\n".join(f"{key} = {toml_value(value)}" for key, value in patches.items())
        + "\n"
    )
    source = (template / "src/lib.rs").read_text()
    callback = "|_registrar| Ok(())"
    if source.count(callback) != 1:
        raise RuntimeError(
            "Android runner template changed: cannot locate the application plugin callback."
        )
    source = source.replace(callback, "flutter_realtime_player::register")
    source = source.replace("dev.flutter.rustshell", PACKAGE)
    runner = BUILD / "runner"
    (runner / "src").mkdir(parents=True, exist_ok=True)
    write_if_changed(runner / "Cargo.toml", manifest)
    write_if_changed(runner / "src/lib.rs", source)
    build_script = (
        (template / "build.rs")
        .read_text()
        .replace(
            "fn main() {",
            'fn main() {\n    println!("cargo:rustc-link-arg=-Wl,--no-undefined");',
            1,
        )
    )
    write_if_changed(runner / "build.rs", build_script)
    return runner


def start_app(adb):
    # `am start -W` can report success before Java loads the native libraries.
    # Capture fresh startup errors and check that the app survives startup.
    with subprocess.Popen(
        [
            *adb,
            "logcat",
            "-T1",
            "-v",
            "brief",
            "AndroidRuntime:E",
            "DEBUG:F",
            "libc:F",
            "flutter_shell_android_runner:V",
            "RustStdoutStderr:V",
            "flutter:E",
            "*:S",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    ) as logs:
        try:
            launch = run(
                *adb,
                "shell",
                "am",
                "start",
                "-W",
                "-n",
                f"{PACKAGE}/com.google.androidgamesdk.GameActivity",
                capture=True,
            )
            print(launch, end="", flush=True)
            if "Error:" in launch or "Error type" in launch:
                raise RuntimeError(f"Android activity launch failed:\n{launch}")
            try:
                output, _ = logs.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                logs.terminate()
                output, _ = logs.communicate(timeout=5)
        finally:
            if logs.poll() is None:
                logs.kill()
                logs.communicate()

    crashed = (
        (f"Process: {PACKAGE}," in output and "FATAL EXCEPTION" in output)
        or "flutter-shell-android-runner panicked:" in output
        or "run_application failed:" in output
    )
    process = subprocess.run(
        [*adb, "shell", "pidof", PACKAGE],
        capture_output=True,
        text=True,
    )
    if crashed or process.returncode != 0 or not process.stdout.strip():
        raise RuntimeError(f"Rust shell failed during startup:\n{output[-8000:]}")
    if output.strip():
        print(output, end="", flush=True)


def prepare_gradle(shell, sdk):
    project = BUILD / "gradle"
    shutil.copytree(
        shell / "android_shell_app",
        project,
        dirs_exist_ok=True,
        ignore=shutil.ignore_patterns(
            ".gradle", "build", "local.properties", "jniLibs"
        ),
    )
    build_file = project / "app/build.gradle"
    build_file.write_text(
        build_file.read_text().replace(
            'applicationId "dev.flutter.rustshell"', f'applicationId "{PACKAGE}"'
        )
    )
    (project / "local.properties").write_text(f"sdk.dir={sdk}\n")
    android = "http://schemas.android.com/apk/res/android"
    ET.register_namespace("android", android)
    manifest = project / "app/src/main/AndroidManifest.xml"
    tree = ET.parse(manifest)
    ET.SubElement(
        tree.getroot(),
        "uses-permission",
        {f"{{{android}}}name": "android.permission.INTERNET"},
    )
    tree.getroot().find("application").set(
        f"{{{android}}}label", "Flutter Realtime Player (Rust)"
    )
    tree.write(manifest, encoding="utf-8", xml_declaration=True)
    return project


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--flutter", default="fvm exec flutter", help="Flutter fork command"
    )
    parser.add_argument("--engine-src-path", type=Path, required=True)
    parser.add_argument("--host-engine", default="host_debug")
    parser.add_argument(
        "--engine", help="Override the Android debug engine output directory name"
    )
    parser.add_argument("--device", default=os.environ.get("ANDROID_SERIAL"))
    parser.add_argument(
        "--build-only",
        action="store_true",
        help="Build the APK without a connected device",
    )
    parser.add_argument(
        "--abi", choices=TARGETS, help="Required only to override the device ABI"
    )
    args = parser.parse_args()

    device = None if args.build_only else select_device(args.device)
    adb = ["adb", "-s", device] if device else []
    abi = args.abi
    if not abi and device:
        supported = (
            run(*adb, "shell", "getprop", "ro.product.cpu.abilist", capture=True)
            .strip()
            .split(",")
        )
        abi = next((candidate for candidate in supported if candidate in TARGETS), None)
        if not abi:
            raise RuntimeError(
                f"Rust shell supports arm64-v8a and x86_64; device reports {supported}."
            )
    abi = abi or "arm64-v8a"
    target, platform, default_engine, gst_arch = TARGETS[abi]
    engine_src = args.engine_src_path.resolve()
    engine = engine_src / "out" / (args.engine or default_engine)
    shell = engine_src / "flutter/shell/platform/rust"
    if not (engine / "args.gn").is_file():
        raise RuntimeError(f"Local Android engine is not configured: {engine}.")

    env = cross_environment()
    ndk = Path(env["ANDROID_NDK_HOME"])
    sdk = Path(env.get("ANDROID_HOME") or env.get("ANDROID_SDK_ROOT") or ndk.parents[1])
    toolchain = ndk / "toolchains/llvm/prebuilt/linux-x86_64/bin"
    compiler = toolchain / f"{target}24-clang"
    gst = Path(env["GSTREAMER_ROOT_ANDROID"]) / gst_arch / "lib/pkgconfig"
    if not compiler.is_file() or not gst.is_dir():
        raise RuntimeError(f"Check Android cross-build paths: {compiler} and {gst}.")
    env.update(
        {
            "ANDROID_HOME": str(sdk),
            "CC": str(compiler),
            "CXX": str(toolchain / f"{target}24-clang++"),
            "AR": str(toolchain / "llvm-ar"),
            f"CARGO_TARGET_{target.upper().replace('-', '_')}_LINKER": str(compiler),
            "PKG_CONFIG_ALLOW_CROSS": "1",
            "PKG_CONFIG_SYSROOT_DIR": "/",
            "PKG_CONFIG_PATH": str(gst),
            "PKG_CONFIG_LIBDIR": str(gst),
            "FLUTTER_RUST_ENGINE_DIR": str(engine),
            "CARGO_TARGET_DIR": str(BUILD / "cargo-target"),
        }
    )
    if "JAVA_HOME" not in env and Path("/opt/android-studio/jbr").is_dir():
        env["JAVA_HOME"] = "/opt/android-studio/jbr"

    depot_tools = env.get(
        "DEPOT_TOOLS", str(engine_src / "flutter/third_party/depot_tools")
    )
    env["PATH"] = depot_tools + os.pathsep + env.get("PATH", "")
    env.setdefault(
        "VPYTHON_BYPASS", "manually managed python not supported by chrome operations"
    )
    # The Rust crates and private C++ ABI must come from the same revision.
    ninja = shutil.which("ninja")
    if not ninja:
        raise RuntimeError("Install ninja to build the local Android engine.")
    run(ninja, "-C", engine, "libflutter_rust_engine.so", "icudtl.dat", env=env)

    pub = ["--no-pub"] if (EXAMPLE / ".dart_tool/package_config.json").is_file() else []
    run(
        *shlex.split(args.flutter),
        "build",
        "bundle",
        "--debug",
        *pub,
        f"--target-platform={platform}",
        f"--local-engine={engine.name}",
        f"--local-engine-host={args.host_engine}",
        f"--local-engine-src-path={engine_src}",
        cwd=EXAMPLE,
        env=env,
    )
    runner = prepare_runner(shell)
    run(
        "cargo",
        "build",
        "--manifest-path",
        runner / "Cargo.toml",
        "--target",
        target,
        env=env,
    )
    jni = BUILD / "jniLibs" / abi
    jni.mkdir(parents=True, exist_ok=True)
    for artifact in (
        engine / "libflutter_rust_engine.so",
        BUILD / "cargo-target" / target / "debug/libflutter_shell_android_runner.so",
    ):
        destination = jni / artifact.name
        shutil.copy2(artifact, destination)
        run(toolchain / "llvm-strip", "--strip-debug", destination)
    project = prepare_gradle(shell, sdk)
    run(
        "bash",
        project / "gradlew",
        "--no-daemon",
        f"-PflutterShellJniLibsDir={jni.parent}",
        ":app:assembleDebug",
        cwd=project,
        env=env,
    )
    apk = project / "app/build/outputs/apk/debug/app-debug.apk"
    if args.build_only:
        print(f"Built {apk}")
        return

    run(*adb, "install", "-r", apk)
    run(*adb, "shell", "am", "force-stop", PACKAGE)
    archive = BUILD / "assets.tar"
    with tarfile.open(archive, "w") as bundle:
        bundle.add(EXAMPLE / "build/flutter_assets", arcname="flutter_assets")
        bundle.add(engine / "icudtl.dat", arcname="icudtl.dat")
    run(*adb, "shell", "run-as", PACKAGE, "mkdir", "-p", "files")
    with archive.open("rb") as assets:
        run(
            *adb,
            "exec-in",
            "run-as",
            PACKAGE,
            "tar",
            "-xf",
            "-",
            "-C",
            "files",
            input_file=assets,
        )
    start_app(adb)
    print(
        f"Rust shell started on {device}. Logs: adb -s {shlex.quote(device)} logcat -s flutter_shell_android_runner RustStdoutStderr flutter AndroidRuntime"
    )


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
