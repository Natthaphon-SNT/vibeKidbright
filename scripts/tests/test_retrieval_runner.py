import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('runner',ROOT/'scripts/evaluate-retrieval.py')
runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)

class RetrievalRunnerTests(unittest.TestCase):
    def test_existing_directory_is_preserved_before_any_inference(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'src-tauri/target') as directory:
            path=Path(directory)/'evidence';runner.new_output_directory(path)
            marker=path/'old.json';marker.write_text('preserve',encoding='utf-8')
            with self.assertRaises(FileExistsError):runner.new_output_directory(path)
            self.assertEqual(marker.read_text(encoding='utf-8'),'preserve')
        with self.assertRaises(ValueError):runner.new_output_directory(ROOT.parent/'not-permitted-evaluation')

    @unittest.skipUnless((ROOT/'src-tauri/target/debug/examples/stalling_lsp_fixture.exe').exists(),'Windows native stalling fault fixture not built')
    def test_timeout_kills_the_actual_owned_fault_process(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'src-tauri/target') as directory:
            path=Path(directory);binary=path/'stall.exe'
            shutil.copy2(ROOT/'src-tauri/target/debug/examples/stalling_lsp_fixture.exe',binary)
            with self.assertRaises(TimeoutError):runner.run_mode(binary,path/'unused.json','lexical',path,'2026-10-19',.4)
            self.assertTrue((path/'stall-pid.txt').exists())
            # The test fixture is a blocked process, never an embedding/LLM result.
            self.assertFalse((path/'lexical.json').exists())
