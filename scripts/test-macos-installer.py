"""Verify a real DMG on macOS; no Node, Rust, ESP-IDF or Python app dependency.

Python is used only by this developer/CI probe, never by the installed IDE.
Run this separately on Apple Silicon and Intel to exercise both native slices.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import subprocess
import sys
import tempfile


def run(command, **kwargs):
    return subprocess.run(command, check=True, capture_output=True, **kwargs)


def verify_checksum(directory):
    images = list(directory.glob("*.dmg"))
    if len(images) != 1:
        raise ValueError("Expected exactly one DMG in the installer directory.")
    image = images[0]
    entries = (directory / "SHA256SUMS-macos.txt").read_text(encoding="utf-8").splitlines()
    expected = [line.split("  ", 1)[0] for line in entries if line.endswith("  " + image.name)]
    with image.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if expected != [actual]:
        raise ValueError("Installer SHA-256 does not match SHA256SUMS-macos.txt.")
    return image, actual


def clean_environment(data):
    env = os.environ.copy()
    for key in list(env):
        if key.startswith(("DYLD_", "VIBEKIDBRIGHT_")) or key in (
            "IDF_PATH", "IDF_TOOLS_PATH", "IDF_PYTHON_ENV_PATH", "PYTHONHOME", "PYTHONPATH",
            "FASTEMBED_CACHE_DIR", "CARGO_HOME", "RUSTUP_HOME", "ORT_DYLIB_PATH",
            "APPDATA", "LOCALAPPDATA", "USERPROFILE", "XDG_CONFIG_HOME", "XDG_DATA_HOME",
        ):
            env.pop(key, None)
    env.update({"HOME": str(data), "PATH": "/usr/bin:/bin:/usr/sbin:/sbin"})
    for key in ("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
        env[key] = "http://127.0.0.1:9"
    for key in ("NO_PROXY", "no_proxy"):
        env[key] = ""
    return env


def verify_app(app, expected_version):
    with (app / "Contents/Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if info.get("CFBundleIdentifier") != "th.nectec.vibekidbright":
        raise ValueError("Unexpected application identifier.")
    if info.get("CFBundleShortVersionString") != expected_version:
        raise ValueError("App version does not match the requested release.")
    executable = info["CFBundleExecutable"]
    if Path(executable).name != executable:
        raise ValueError("Invalid bundle executable name.")
    binary = app / "Contents/MacOS" / executable
    architectures = run(["/usr/bin/lipo", "-archs", str(binary)]).stdout.decode().split()
    if set(architectures) != {"arm64", "x86_64"}:
        raise ValueError("The DMG must contain both Apple Silicon and Intel slices.")
    run(["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app)])
    dependencies = run(["/usr/bin/otool", "-L", str(binary)]).stdout.decode()
    for line in dependencies.splitlines():
        if line.startswith("\t"):
            dependency = line.strip().split(" (", 1)[0]
            if not dependency.startswith(("/System/Library/", "/usr/lib/", "@rpath/", "@loader_path/", "@executable_path/")):
                raise ValueError("App depends on a library outside its bundle: " + dependency)
    resources = app / "Contents/Resources"
    for relative in ("knowledge_base/support_scope.md", "embedding/manifest.json", "embedding/model.onnx"):
        if not (resources / relative).is_file():
            raise ValueError("Missing bundled resource: " + relative)
    return binary, architectures


def probe(image, report_path, expected_version, expected_arch, sdk_idf=None, sdk_tools=None):
    with tempfile.TemporaryDirectory(prefix="vibekidbright-macos-") as temporary:
        # macOS /var aliases /private/var. Tauri rejects an executable launched
        # through any symlink component; launch the copied bundle canonically.
        scratch = Path(temporary).resolve()
        mount = scratch / "mounted image"
        mount.mkdir()
        mounted = False
        try:
            run(["/usr/bin/hdiutil", "verify", str(image)])
            run(["/usr/bin/hdiutil", "attach", "-readonly", "-nobrowse", "-mountpoint", str(mount), str(image)])
            mounted = True
            apps = list(mount.glob("*.app"))
            if len(apps) != 1 or apps[0].is_symlink():
                raise ValueError("Expected exactly one app bundle in the DMG.")
            # Include spaces in the install path to catch shell/path assumptions.
            installed = scratch / "Installed Applications" / apps[0].name
            installed.parent.mkdir()
            run(["/usr/bin/ditto", str(apps[0]), str(installed)])
        finally:
            if mounted:
                run(["/usr/bin/hdiutil", "detach", str(mount)])
        # Execute only the copied app, after the source image is unmounted.
        binary, architectures = verify_app(installed, expected_version)
        working = scratch / "unrelated working directory"
        data = scratch / "fresh user data"
        working.mkdir()
        data.mkdir()
        env = clean_environment(data)
        # The explicit scratch argument isolates app data. Keychain belongs to
        # the real macOS login session and must not be resolved under a fake HOME.
        native_env = dict(env, HOME=str(Path.home()))
        native_report = scratch / "native-self-check.json"
        result = subprocess.run([str(binary), "--self-check", str(native_report), str(data)],
                                cwd=working, env=native_env, capture_output=True, text=True, timeout=180)
        if not native_report.is_file():
            raise ValueError(f"App self-check produced no report (exit {result.returncode}): {result.stderr[-4000:]}")
        record = json.loads(native_report.read_text(encoding="utf-8"))
        if result.returncode != 0:
            raise ValueError("Installed app self-check failed: " + json.dumps(record))
        if record.get("version") != expected_version or record.get("platform") != "macos" or record.get("architecture") != expected_arch:
            raise ValueError("Self-check did not execute the expected native release slice.")
        for field in ("settings_write_and_read", "local_vector_search", "native_credential_roundtrip",
                      "lexical_intelligence_search", "durable_review_restart_and_undo"):
            if record.get(field) is not True:
                raise ValueError("Installed app check is incomplete: " + field)
        if sdk_idf is not None:
            sdk_report = scratch / "sdk-build-check.json"
            sdk_log = report_path.with_suffix(".sdk.log")
            with sdk_log.open("w", encoding="utf-8") as log:
                build = subprocess.run([str(binary), "--sdk-self-check", str(sdk_report), str(scratch / "esp32-build"),
                                        str(sdk_idf), str(sdk_tools)], cwd=working, env=env, stdout=log, stderr=log, timeout=660)
            if not sdk_report.is_file():
                raise ValueError("The installed app did not write an SDK build report; see " + sdk_log.name)
            sdk = json.loads(sdk_report.read_text(encoding="utf-8"))
            if build.returncode != 0 or sdk.get("native_sdk_build") is not True or sdk.get("architecture") != expected_arch or sdk.get("firmware_bytes", 0) <= 0:
                raise ValueError("Native ESP32 firmware build failed: " + json.dumps(sdk))
            record["sdk_build_check"] = sdk
        # This detects startup crashes; interactive WebView/USB behavior needs a
        # separate manual check and is not certified by a live process alone.
        gui_log = report_path.with_suffix(".gui.log")
        with gui_log.open("w", encoding="utf-8") as log:
            child = subprocess.Popen([str(binary)], cwd=working, env=native_env, stdout=log, stderr=log)
            try:
                try:
                    child.wait(timeout=12)
                except subprocess.TimeoutExpired:
                    pass
                else:
                    raise ValueError(f"Installed GUI exited during startup (exit {child.returncode}); see {gui_log.name}.")
            finally:
                if child.poll() is None:
                    child.terminate()
                    try:
                        child.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        child.kill()
                        child.wait(timeout=10)
        record.update({"universal_architectures": architectures, "code_signature_verified": True,
                       "dmg_verified_and_unmounted": True, "gui_survived_startup_seconds": 12,
                       "interactive_gui_tested": False, "probe_environment": {
                           "native_login_home_for_gui_and_keychain": True,
                           "copied_installation": True, "install_path_contains_spaces": True,
                           "separate_working_directory": True, "fresh_appdata": True,
                           "developer_paths_removed_from_environment": True, "http_proxy_closed": True,
                           "os_firewall_isolation": False, "fresh_virtual_machine": False,
                       }})
        return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--installer-directory", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--expected-version", required=True)
    parser.add_argument("--expected-arch", required=True, choices=("aarch64", "x86_64"))
    parser.add_argument("--sdk-idf", type=Path)
    parser.add_argument("--sdk-tools", type=Path)
    args = parser.parse_args()
    report_path = args.report.resolve()
    report_path.parent.mkdir(parents=True, exist_ok=True)
    record = {"status": "failed"}
    try:
        if sys.platform != "darwin":
            raise ValueError("A real macOS host is required to verify a DMG.")
        if (args.sdk_idf is None) != (args.sdk_tools is None):
            raise ValueError("Supply both --sdk-idf and --sdk-tools for a firmware build check.")
        host = {"arm64": "aarch64", "x86_64": "x86_64"}.get(platform.machine())
        if host != args.expected_arch:
            raise ValueError("Runner architecture does not match --expected-arch.")
        image, digest = verify_checksum(args.installer_directory.resolve())
        record.update(probe(image, report_path, args.expected_version, args.expected_arch, args.sdk_idf, args.sdk_tools))
        record.update({"status": "passed", "installer": image.name, "sha256": digest})
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        record["error"] = str(error)
        if isinstance(error, subprocess.CalledProcessError):
            record["command_stderr"] = (error.stderr or b"").decode(errors="replace")[-4000:]
    report_path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(record, indent=2))
    return 0 if record["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
