"""Check activation of the exact Python snippet copied by Mac users."""
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[2] / 'src/MacosSdkSetup.tsx'
SNIPPET = re.search(r"<<'PY'\n(.*?)\nPY", SOURCE.read_text(encoding='utf-8'), re.DOTALL).group(1)

class CleanMacSetupTests(unittest.TestCase):
    def prepare(self, home):
        root = home / '.vibekidbright/macos-sdk'
        install = root / 'install-fresh123'
        (install / '.espressif/python_env/idf5.5_py3.11_env').mkdir(parents=True)
        (root / 'current.json').write_text('{"directory":"install-previous"}')
        return root, install

    def activate(self, root, install):
        with patch.object(sys, 'argv', ['-', str(root), str(install)]):
            exec(compile(SNIPPET, str(SOURCE), 'exec'), {})

    def test_activation_uses_each_users_new_venv_and_stores_relative_pointer(self):
        with tempfile.TemporaryDirectory() as temporary:
            for username in ['first-user', 'second-user']:
                root, install = self.prepare(Path(temporary) / username)
                with patch('subprocess.run') as run:
                    self.activate(root, install)
                call = run.call_args
                self.assertEqual(call.args[0][0], str(install / '.espressif/python_env/idf5.5_py3.11_env/bin/python'))
                self.assertEqual(call.kwargs['env']['IDF_PATH'], str(install / 'esp-idf'))
                self.assertEqual(json.loads((root / 'current.json').read_text()), {'directory': 'install-fresh123'})

    def test_broken_new_python_keeps_previous_active_install(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, install = self.prepare(Path(temporary))
            with patch('subprocess.run', side_effect=subprocess.CalledProcessError(1, 'python')):
                with self.assertRaises(subprocess.CalledProcessError):
                    self.activate(root, install)
            self.assertEqual(json.loads((root / 'current.json').read_text())['directory'], 'install-previous')

    def test_install_outside_app_sdk_root_cannot_be_activated(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, _ = self.prepare(Path(temporary))
            with self.assertRaises(AssertionError):
                self.activate(root, Path(temporary) / 'outside')
            self.assertEqual(json.loads((root / 'current.json').read_text())['directory'], 'install-previous')

if __name__ == '__main__':
    unittest.main()
