"""Version/capability detection for Plus; metadata probe never imports maix.

Known baseline: MaixPy 4.12.5 / Runtime 1.24.0. New versions use the same
required API contract, not a hard-coded version allowlist. Unknown is preserved.
"""
import importlib.util
import json
from pathlib import Path
import runpy
import sys

PREFIX = "__VIBE_PLUS_RUNTIME__:"
REQUIRED_APIS = (
    "app.need_exit", "display.Display", "display.Display.width",
    "display.Display.height", "display.Display.show", "image.Image",
    "image.Image.draw_rect", "image.Image.draw_string", "image.Image.draw_circle",
    "image.Color.from_rgb", "image.Format.FMT_RGB888", "time.sleep_ms",
)


def probe_module():
    spec = importlib.util.spec_from_file_location("vibe_plus_probe", Path(__file__).with_name(".vibekidbright-probe.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def missing_apis(module):
    missing = []
    for name in REQUIRED_APIS:
        value = module
        for part in name.split("."):
            value = getattr(value, part, None)
            if value is None:
                missing.append(name)
                break
        else:
            if name != "image.Format.FMT_RGB888" and not callable(value):
                missing.append(name)
    return missing


def emit(info):
    print("\n" + PREFIX + json.dumps(info, separators=(",", ":")), flush=True)


def run_program():
    probe = probe_module()
    info = probe.runtime_info()
    emit(info)
    print("[Plus Run] Loading native MaixPy after releasing Launcher.", flush=True)
    import maix
    info["maixpy"] = probe.clean(getattr(maix, "__version__", None)) or info["maixpy"]
    info["missing_apis"] = missing_apis(maix)
    info["api_checked"] = True
    emit(info)
    if info["missing_apis"]:
        raise RuntimeError("Installed MaixPy is missing required APIs: " + ", ".join(info["missing_apis"]))
    print("[Plus Run] Required MaixPy APIs available; starting main.py.", flush=True)
    directory = Path(__file__).resolve().parent
    before_argv, before_path = sys.argv, list(sys.path)
    sys.argv = [str(directory / "main.py")]
    sys.path.insert(0, str(directory))
    try:
        runpy.run_path(str(directory / "main.py"), run_name="__main__")
    finally:
        sys.argv = before_argv
        sys.path[:] = before_path


if __name__ == "__main__":
    if sys.argv[1:] == ["--run"]:
        run_program()
    elif not sys.argv[1:]:
        emit(probe_module().runtime_info())
    else:
        raise SystemExit("Unsupported Plus runtime operation.")
