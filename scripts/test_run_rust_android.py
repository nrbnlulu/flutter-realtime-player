"""Regression checks for Android native library startup failures."""

import contextlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "run_rust_android", Path(__file__).with_name("run-rust-android.py")
)
android = importlib.util.module_from_spec(spec)
spec.loader.exec_module(android)


class StartupTest(unittest.TestCase):
    def start(self, output, pid="1234", returncode=0):
        with (
            patch.object(android, "run", return_value="Status: ok\n"),
            patch.object(android.subprocess, "Popen") as popen,
            patch.object(
                android.subprocess,
                "run",
                return_value=subprocess.CompletedProcess([], returncode, stdout=pid),
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            logs = popen.return_value.__enter__.return_value
            logs.communicate.side_effect = [
                subprocess.TimeoutExpired("logcat", 5),
                (output, None),
            ]
            logs.poll.return_value = 0
            android.start_app(["adb", "-s", "test-device"])
            logs.terminate.assert_called_once()

    def test_rejects_native_loader_crash_even_while_android_keeps_process_alive(self):
        output = (
            "E/AndroidRuntime: FATAL EXCEPTION: main\n"
            f"E/AndroidRuntime: Process: {android.PACKAGE}, PID: 1234\n"
            'E/AndroidRuntime: UnsatisfiedLinkError: cannot locate symbol "FlutterRustShellLockQueue"\n'
        )
        with self.assertRaisesRegex(RuntimeError, "FlutterRustShellLockQueue"):
            self.start(output)

    def test_rejects_rust_runtime_failure(self):
        with self.assertRaisesRegex(RuntimeError, "run_application failed"):
            self.start(
                "E/flutter_shell_android_runner: run_application failed: incompatible ABI"
            )

    def test_rejects_process_that_died_without_java_exception(self):
        with self.assertRaisesRegex(RuntimeError, "failed during startup"):
            self.start("F/libc: Fatal signal 11", pid="", returncode=1)

    def test_accepts_live_process_and_ignores_other_apps_java_errors(self):
        self.start(
            "E/AndroidRuntime: FATAL EXCEPTION: main\nProcess: another.app, PID: 456\n"
        )


class RunnerTest(unittest.TestCase):
    def test_requires_native_symbols_to_resolve_at_link_time(self):
        with tempfile.TemporaryDirectory() as directory:
            shell = Path(directory) / "sdk"
            template = shell / "crates/flutter-shell-android-runner"
            (template / "src").mkdir(parents=True)
            (shell / "Cargo.toml").write_text(
                '[workspace.package]\nedition = "2024"\nrust-version = "1.93"\n'
            )
            (template / "Cargo.toml").write_text(
                "[target.'cfg(target_os = \"android\")'.dependencies]\n"
                'flutter-shell-winit = { path = "../flutter-shell-winit" }\n'
            )
            (template / "src/lib.rs").write_text(
                "run_application(config, |_registrar| Ok(())); // dev.flutter.rustshell\n"
            )
            (template / "build.rs").write_text("fn main() {}\n")
            with patch.object(android, "BUILD", Path(directory) / "build"):
                runner = android.prepare_runner(shell)
                script = runner / "build.rs"
                self.assertIn(
                    "cargo:rustc-link-arg=-Wl,--no-undefined", script.read_text()
                )
                modified = script.stat().st_mtime_ns
                android.prepare_runner(shell)
                self.assertEqual(modified, script.stat().st_mtime_ns)


if __name__ == "__main__":
    unittest.main()
