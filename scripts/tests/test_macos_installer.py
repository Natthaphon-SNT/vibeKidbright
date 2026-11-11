"""Portable regression tests for the DMG probe; macOS jobs run the native checks."""
import hashlib
import importlib.util
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest
from unittest.mock import MagicMock, patch

spec = importlib.util.spec_from_file_location("macos_installer", Path(__file__).resolve().parents[1] / "test-macos-installer.py")
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.image = self.root / "VibeKidbright IDE_3.7.4_universal.dmg"
        self.image.write_bytes(b"fixture disk image")
        self.digest = hashlib.sha256(self.image.read_bytes()).hexdigest()
        (self.root / "SHA256SUMS-macos.txt").write_text(self.digest + "  " + self.image.name + "\n", encoding="utf-8")
        self.record = {"version": "3.7.4", "platform": "macos", "architecture": "aarch64"}
        for field in ("settings_write_and_read", "local_vector_search", "native_credential_roundtrip",
                      "lexical_intelligence_search", "durable_review_restart_and_undo"):
            self.record[field] = True
        self.commands = []
        self.native_exit = 0
        self.sdk_success = True
        self.dependency = "/usr/lib/libSystem.B.dylib"
        self.detached = False

    def fake_run(self, command, **kwargs):
        self.commands.append(command)
        output = b""
        if command[0] == "/usr/bin/hdiutil" and command[1] == "attach":
            mount = Path(command[command.index("-mountpoint") + 1])
            app = mount / "VibeKidbright IDE.app"
            (app / "Contents/MacOS").mkdir(parents=True)
            with (app / "Contents/Info.plist").open("wb") as stream:
                plistlib.dump({"CFBundleIdentifier": "th.nectec.vibekidbright", "CFBundleExecutable": "vibe-kidbright",
                               "CFBundleShortVersionString": "3.7.4"}, stream)
            (app / "Contents/MacOS/vibe-kidbright").touch()
            for relative in ("knowledge_base/support_scope.md", "embedding/manifest.json", "embedding/model.onnx"):
                resource = app / "Contents/Resources" / relative
                resource.parent.mkdir(parents=True, exist_ok=True)
                resource.touch()
        elif command[:2] == ["/usr/bin/hdiutil", "detach"]:
            self.detached = True
        elif command[0] == "/usr/bin/ditto":
            shutil.copytree(command[1], command[2])
        elif command[0] == "/usr/bin/lipo":
            output = b"x86_64 arm64\n"
        elif command[0] == "/usr/bin/otool":
            output = (command[-1] + ":\n\t" + self.dependency + " (compatibility version 1.0.0)\n").encode()
        return subprocess.CompletedProcess(command, 0, stdout=output, stderr=b"")

    def fake_self_check(self, command, **kwargs):
        self.assertTrue(self.detached, "Must unmount DMG before executing the copied application")
        self.assertIn("Installed Applications", command[0])
        self.assertIn("Contents/MacOS", Path(command[0]).as_posix())
        self.assertEqual(kwargs["env"]["PATH"], "/usr/bin:/bin:/usr/sbin:/sbin")
        self.assertNotEqual(Path(command[0]).parent, kwargs["cwd"])
        if command[1] == "--sdk-self-check":
            Path(command[2]).write_text(json.dumps({"native_sdk_build":self.sdk_success,"architecture":"aarch64","firmware_bytes":512}), encoding="utf-8")
        else:
            Path(command[2]).write_text(json.dumps(self.record), encoding="utf-8")
        return subprocess.CompletedProcess(command, self.native_exit, stdout="", stderr="")

    def check(self, gui_exits=False, sdk=False):
        child = MagicMock()
        child.poll.return_value = None
        child.wait.side_effect = [0 if gui_exits else subprocess.TimeoutExpired("fixture", 12), 0]
        child.returncode = 0
        with patch.object(installer, "run", side_effect=self.fake_run), \
             patch.object(installer.subprocess, "run", side_effect=self.fake_self_check), \
             patch.object(installer.subprocess, "Popen", return_value=child):
            extra = [self.root / "idf", self.root / "tools"] if sdk else []
            record = installer.probe(self.image, self.root / "report.json", "3.7.4", "aarch64", *extra)
        child.terminate.assert_called_once()
        return record

    def test_checksum_handles_spaces_and_rejects_tampering(self):
        self.assertEqual(installer.verify_checksum(self.root), (self.image, self.digest))
        self.image.write_bytes(b"tampered disk image")
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            installer.verify_checksum(self.root)

    def test_rejects_multiple_installers(self):
        (self.root / "unexpected.dmg").touch()
        with self.assertRaisesRegex(ValueError, "exactly one DMG"):
            installer.verify_checksum(self.root)

    def test_workflow_produces_downloadable_name_and_matching_checksum(self):
        workflow = (Path(__file__).resolve().parents[2] / ".github/workflows/build-macos.yml").read_text(encoding="utf-8")
        source = re.search(r"python3 - <<'PY'\n(.*?)\n\s*PY\n", workflow, re.DOTALL).group(1)
        with patch.dict(installer.os.environ, {"APP_VERSION": "3.7.4"}):
            subprocess.run([installer.sys.executable, "-c", textwrap.dedent(source)], cwd=self.root, check=True)
        image, digest = installer.verify_checksum(self.root)
        self.assertEqual(image.name, "VibeKidbright-IDE_3.7.4_universal.dmg")
        self.assertEqual(digest, self.digest)

    def test_removes_developer_environment_and_proxy_bypasses(self):
        with patch.dict(installer.os.environ, {"VIBEKIDBRIGHT_IDF_PATH": "/developer/sdk", "DYLD_LIBRARY_PATH": "/developer/lib",
                                             "PYTHONPATH": "/developer/python", "NO_PROXY": "*", "no_proxy": "*"}):
            env = installer.clean_environment(self.root)
        self.assertNotIn("VIBEKIDBRIGHT_IDF_PATH", env)
        self.assertNotIn("DYLD_LIBRARY_PATH", env)
        self.assertNotIn("PYTHONPATH", env)
        self.assertEqual(env["NO_PROXY"], "")
        self.assertEqual(env["no_proxy"], "")
        self.assertEqual(env["HOME"], str(self.root))

    def test_installs_from_dmg_and_checks_copied_app_after_unmount(self):
        record = self.check()
        self.assertTrue(record["code_signature_verified"])
        self.assertEqual(set(record["universal_architectures"]), {"arm64", "x86_64"})
        self.assertFalse(record["interactive_gui_tested"])

    def test_self_check_failure_cannot_pass_installation(self):
        self.native_exit = 1
        with self.assertRaisesRegex(ValueError, "self-check failed"):
            self.check()

    def test_installed_binary_must_build_nonempty_esp32_firmware(self):
        record = self.check(sdk=True)
        self.assertTrue(record["sdk_build_check"]["native_sdk_build"])
        self.assertGreater(record["sdk_build_check"]["firmware_bytes"],0)

    def test_native_firmware_build_failure_cannot_pass_installation(self):
        self.sdk_success = False
        with self.assertRaisesRegex(ValueError, "Native ESP32 firmware build failed"):
            self.check(sdk=True)

    def test_wrong_native_slice_cannot_pass_installation(self):
        self.record["architecture"] = "x86_64"
        with self.assertRaisesRegex(ValueError, "native release slice"):
            self.check()

    def test_gui_early_exit_cannot_pass_installation(self):
        with self.assertRaisesRegex(ValueError, "GUI exited during startup"):
            self.check(gui_exits=True)

    def test_rejects_libraries_from_build_machine(self):
        self.dependency = "/opt/homebrew/lib/libonnxruntime.dylib"
        with self.assertRaisesRegex(ValueError, "outside its bundle"):
            self.check()

    def test_unmounts_image_when_copy_fails(self):
        fake_run = self.fake_run

        def fail_copy(command, **kwargs):
            if command[0] == "/usr/bin/ditto":
                raise OSError("disk full")
            return fake_run(command, **kwargs)

        with patch.object(installer, "run", side_effect=fail_copy):
            with self.assertRaisesRegex(OSError, "disk full"):
                installer.probe(self.image, self.root / "report.json", "3.7.4", "aarch64")
        self.assertTrue(self.detached)


if __name__ == "__main__":
    unittest.main()
