"""Probe an extracted/copied installation without access to the development environment.

Development verification only; users do not need Python to run the installed IDE.
Native credential checks require a real Windows user logon, not a service sandbox.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument("--installed-directory", required=True, type=Path)
parser.add_argument("--report", required=True, type=Path)
args = parser.parse_args()
installation = args.installed_directory.resolve()
binary = installation / "vibe-kidbright.exe"
for name in ("knowledge_base/support_scope.md", "embedding/model.onnx", "vcruntime140.dll", "msvcp140.dll"):
    if not (installation / name).is_file():
        raise SystemExit("Missing installed runtime file: " + name)
scratch = Path(tempfile.mkdtemp(prefix="standalone-user-", dir="src-tauri/target"))
working = scratch / "unrelated working directory"
working.mkdir()
data = scratch / "fresh-user-data"
data.mkdir()
env = os.environ.copy()
for variable in ("IDF_PATH", "IDF_TOOLS_PATH", "VIBEKIDBRIGHT_IDF_PATH", "VIBEKIDBRIGHT_TOOLS_PATH", "FASTEMBED_CACHE_DIR", "CARGO_HOME", "RUSTUP_HOME"):
    env.pop(variable, None)
env.update({"APPDATA": str(data / "Roaming"), "LOCALAPPDATA": str(data / "Local"), "USERPROFILE": str(data), "HOME": str(data),
            "PATH": str(Path(env.get("SYSTEMROOT", "C:/Windows")) / "System32"),
            "HTTP_PROXY": "http://127.0.0.1:9", "HTTPS_PROXY": "http://127.0.0.1:9"})
report = args.report.resolve()
report.parent.mkdir(parents=True, exist_ok=True)
result = subprocess.run([str(binary), "--self-check", str(report), str(data)], cwd=working, env=env,
                        timeout=120, creationflags=subprocess.CREATE_NO_WINDOW)
if not report.is_file():
    raise SystemExit(f"Application did not write a self-check report (exit {result.returncode}).")
record = json.loads(report.read_text(encoding="utf-8"))
record["probe_environment"] = {"separate_install_directory": True, "separate_working_directory": True, "fresh_appdata": True,
                               "developer_paths_removed_from_environment": True, "http_proxy_closed": True,
                               "os_firewall_isolation": False, "fresh_virtual_machine": False}
report.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
print(json.dumps(record, indent=2))
raise SystemExit(result.returncode)
