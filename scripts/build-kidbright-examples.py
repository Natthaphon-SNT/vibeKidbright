"""Build exported pilot projects using an explicitly selected installed ESP-IDF.

No dependency installation, downloads or flashing. Export the projects with the
Rust example first. All build output stays below --examples-root.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--idf-path", required=True, type=Path)
    parser.add_argument("--tools-path", required=True, type=Path)
    parser.add_argument("--python", required=True, type=Path)
    parser.add_argument("--examples-root", required=True, type=Path)
    parser.add_argument("--evidence-directory", type=Path, help="New directory for logs/results; prior evidence is preserved")
    parser.add_argument("--force-recompile", action="store_true", help="Touch generated main source mtime, preserving bytes")
    parser.add_argument("--examples", nargs="+", default=["button_buzzer", "temperature_serial"],
                        choices=["button_buzzer", "temperature_serial", "music_scale", "matrix_counter", "repeat_delay", "forever_delay", "arithmetic_if", "nested_conditions"])
    args = parser.parse_args()
    import hashlib
    import time
    evidence=(args.evidence_directory or args.examples_root/('build-evidence-'+str(time.time_ns()))).resolve()
    workspace=Path(__file__).resolve().parents[1]
    if not evidence.parent.resolve().is_relative_to(workspace.resolve()):
        parser.error("Evidence must stay within workspace")
    evidence.mkdir()  # Refuse to overwrite previous run logs/results.
    version_cmake = (args.idf_path / "tools/cmake/version.cmake").read_text(encoding="utf-8")
    version_parts = [re.search(r"set\(IDF_VERSION_" + part + r"\s+(\d+)\)", version_cmake)
                     for part in ("MAJOR", "MINOR", "PATCH")]
    if not all(version_parts):
        parser.error("Cannot identify the selected ESP-IDF version from tools/cmake/version.cmake")
    sdk_version = ".".join(match.group(1) for match in version_parts)
    env = os.environ.copy()
    env["IDF_PATH"] = str(args.idf_path.resolve())
    env["IDF_TOOLS_PATH"] = str(args.tools_path.resolve())
    export = subprocess.run(
        [str(args.python), str(args.idf_path / "tools/idf_tools.py"), "export", "--format", "key-value"],
        env=env, text=True, capture_output=True, check=True,
    )
    original_path = env.get("PATH", "")
    for line in export.stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator and key in {"PATH", "IDF_PYTHON_ENV_PATH", "ESP_ROM_ELF_DIR", "OPENOCD_SCRIPTS", "ESP_IDF_VERSION"}:
            env[key] = value.replace("%PATH%", original_path).replace("$PATH", original_path)
    env["IDF_CCACHE_ENABLE"] = "0"
    env["IDF_COMPONENT_MANAGER"] = "0"
    root = args.examples_root.resolve()
    results = []
    for example in args.examples:
        project = root / example
        metadata_path = project / ".vibekidbright/project.json"
        if not metadata_path.exists():
            metadata_path = project / "kidbright-project.json"  # Existing exports remain usable.
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        log = evidence / (example + "-build.log")
        main=project/"main/main.c"
        source_sha256=hashlib.sha256(main.read_bytes()).hexdigest()
        if args.force_recompile: os.utime(main,None)
        command = [str(args.python), str(args.idf_path / "tools/idf.py"), "-C", str(project), "-DCCACHE_ENABLE=0", "build"]
        print("Building " + example, flush=True)
        with log.open("w", encoding="utf-8") as output:
            run = subprocess.run(command, env=env, stdout=output, stderr=subprocess.STDOUT)
        result = {"example": example, "board_profile": metadata["board_profile"], "exit_code": run.returncode, "command":command,"log": log.name, "log_sha256":hashlib.sha256(log.read_bytes()).hexdigest(),"flashed": False, "sdk_version": sdk_version,"source_sha256":source_sha256,"source_unchanged":hashlib.sha256(main.read_bytes()).hexdigest()==source_sha256}
        for label,file in [("firmware_sha256",project/"build/kidbright_block_pilot.bin"),("compile_commands_sha256",project/"build/compile_commands.json")]:
            if file.exists():result[label]=hashlib.sha256(file.read_bytes()).hexdigest()
        config = project / "build/project_description.json"
        if config.exists():
            description = json.loads(config.read_text(encoding="utf-8"))
            result["target"] = description.get("target")
        results.append(result)
        print(json.dumps(result), flush=True)
        if run.returncode:
            print("\n".join(log.read_text(encoding="utf-8", errors="replace").splitlines()[-45:]), flush=True)
    (evidence / "build-results.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    raise SystemExit(0 if all(row["exit_code"] == 0 and row['source_unchanged'] for row in results) else 1)


if __name__ == "__main__":
    main()
