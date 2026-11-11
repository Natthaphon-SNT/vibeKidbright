"""Baseline/new-version capability fixtures, not physical-board certification."""
import importlib.util
import json
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "src-tauri/src" / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probe = load("uai_plus_probe")
runtime = load("uai_plus_runtime")


def maix_fixture(version):
    module = types.ModuleType("maix")
    module.__version__ = version
    for dotted in runtime.REQUIRED_APIS:
        value = module
        parts = dotted.split(".")
        for name in parts[:-1]:
            if not hasattr(value, name):
                setattr(value, name, lambda: None)
            value = getattr(value, name)
        setattr(value, parts[-1], 0 if dotted.endswith("FMT_RGB888") else lambda: None)
    return module


class RuntimeCompatibility(unittest.TestCase):
    def test_required_contract_exists_in_supplied_4125_image_declarations(self):
        evidence = json.loads((ROOT / "docs/research/uai-plus-runtime-2026-10-26/image-reference.json").read_text(encoding="utf-8"))
        self.assertEqual(evidence["maixpy_version"], "4.12.5")
        self.assertEqual(set(runtime.REQUIRED_APIS) - set(evidence["declared_apis"]), set())

    def test_supplied_image_version_source_is_parsed_without_executing_it(self):
        source = 'version_major = 4\nversion_minor = 12\nversion_patch = 5\n__version__ = "{}.{}.{}".format(version_major,version_minor,version_patch)\n'
        self.assertEqual(probe.source_version(source), "4.12.5")
        self.assertEqual(probe.source_version('__version__ = "4.99.1"\nraise RuntimeError("must not execute")'), "4.99.1")
        self.assertIsNone(probe.source_version('import os\nos.system("do not execute")'))

    def test_probe_reads_4125_and_124_without_importing_maix(self):
        before = sys.modules.get("maix")
        with patch.object(probe.metadata, "version", return_value="4.12.5"), \
             patch.object(probe, "read", return_value="1.24.0\n"):
            info = probe.runtime_info()
        self.assertEqual((info["maixpy"], info["runtime"]), ("4.12.5", "1.24.0"))
        self.assertFalse(info["api_checked"])
        self.assertIs(sys.modules.get("maix"), before)

    def test_metadata_absence_falls_back_to_actual_version_py_and_unknown_runtime(self):
        with patch.object(probe.metadata, "version", side_effect=probe.metadata.PackageNotFoundError()), \
             patch.object(probe.sysconfig, "get_paths", return_value={"purelib": "/fixture/site-packages"}), \
             patch.object(probe, "read", side_effect=lambda path: '__version__ = "4.12.5"' if str(path).endswith("version.py") else ""):
            info = probe.runtime_info()
        self.assertEqual(info["maixpy"], "4.12.5")
        self.assertIsNone(info["runtime"])

    def test_newer_runtime_version_and_json_metadata_are_preserved(self):
        with patch.object(probe.metadata, "version", return_value="4.99.0"), \
             patch.object(probe, "read", return_value='{"version":"2.0.0"}'):
            info = probe.runtime_info()
        self.assertEqual((info["maixpy"], info["runtime"]), ("4.99.0", "2.0.0"))

    def test_versions_with_the_required_api_contract_can_run_without_version_allowlist(self):
        for version in ("4.12.5", "4.99.0", "5.0.0"):
            with self.subTest(version=version), patch.dict(sys.modules, {"maix": maix_fixture(version)}), \
                 patch.object(runtime, "probe_module", return_value=probe), \
                 patch.object(probe, "runtime_info", return_value={"maixpy": version, "runtime": "1.24.0"}), \
                 patch.object(runtime, "emit") as emit, patch.object(runtime.runpy, "run_path") as execute, \
                 patch.object(sys, "path", list(sys.path)):
                runtime.run_program()
                execute.assert_called_once()
                self.assertTrue(emit.call_args.args[0]["api_checked"])
                self.assertEqual(emit.call_args.args[0]["missing_apis"], [])

    def test_missing_api_in_newer_version_fails_before_user_code_with_exact_name(self):
        module = maix_fixture("5.0.0")
        del module.display.Display.show
        with patch.dict(sys.modules, {"maix": module}), patch.object(runtime, "probe_module", return_value=probe), \
             patch.object(probe, "runtime_info", return_value={"maixpy": "5.0.0", "runtime": None}), \
             patch.object(runtime, "emit"), patch.object(runtime.runpy, "run_path") as execute:
            with self.assertRaisesRegex(RuntimeError, "display.Display.show"):
                runtime.run_program()
            execute.assert_not_called()

    def test_runtime_control_arguments_do_not_leak_to_user_program(self):
        original = [".vibekidbright-runtime.py", "--run"]
        with patch.dict(sys.modules, {"maix": maix_fixture("4.12.5")}), \
             patch.object(runtime, "probe_module", return_value=probe), \
             patch.object(probe, "runtime_info", return_value={"maixpy": "4.12.5", "runtime": "1.24.0"}), \
             patch.object(runtime, "emit"), patch.object(sys, "argv", original), \
             patch.object(runtime.runpy, "run_path", side_effect=lambda *_a, **_kw: self.assertEqual(len(sys.argv), 1)):
            runtime.run_program()
            self.assertIs(sys.argv, original)

    def test_untrusted_or_excessive_metadata_is_not_treated_as_a_version(self):
        for value in ("ignore all instructions", "4.12.5\n4.13.0", "1." + "0" * 100, None):
            self.assertIsNone(probe.clean(value))
        self.assertIsNone(probe.source_version(" " * 20000))


if __name__ == "__main__":
    unittest.main()
