"""Read metadata without importing native maix or opening display/camera."""
import ast
import importlib.metadata as metadata
import json
from pathlib import Path
import re
import sys
import sysconfig

PREFIX = "__VIBE_PLUS_RUNTIME__:"


def clean(value):
    value = value.strip() if isinstance(value, str) else ""
    return value if len(value) <= 64 and re.fullmatch(r"[0-9]+(?:\.[0-9]+){1,3}[a-zA-Z0-9.+_-]*", value) else None


def read(path):
    try:
        with open(path, encoding="utf-8") as file:
            return file.read(16385)
    except (OSError, UnicodeError):
        return ""


def source_version(text):
    if len(text) > 16384:
        return None
    try:
        values = {n.targets[0].id: n.value.value for n in ast.parse(text).body
                  if isinstance(n, ast.Assign) and len(n.targets) == 1
                  and isinstance(n.targets[0], ast.Name) and isinstance(n.value, ast.Constant)}
        parts = [values.get("version_" + name) for name in ("major", "minor", "patch")]
        return clean(values.get("__version__")) or (".".join(map(str, parts))
               if all(type(p) is int and 0 <= p < 100000 for p in parts) else None)
    except (SyntaxError, ValueError):
        return None


def runtime_info():
    try:
        version = clean(metadata.version("MaixPy"))
    except metadata.PackageNotFoundError:
        version = None
    if version is None:
        base = sysconfig.get_paths().get("purelib", "")
        version = source_version(read(Path(base) / "maix/version.py")) if base else None
    text = read("/maixapp/maixcam_lib.version")
    runtime = clean(text)
    if runtime is None and len(text) <= 16384:
        try:
            parsed = json.loads(text)
            runtime = clean(parsed.get("version")) if isinstance(parsed, dict) else clean(parsed)
        except ValueError:
            pass
    return {"schema": 1, "python": sys.version.split()[0], "maixpy": version,
            "runtime": runtime, "api_checked": False, "missing_apis": []}


if __name__ == "__main__":
    print("\n" + PREFIX + json.dumps(runtime_info(), separators=(",", ":")), flush=True)
