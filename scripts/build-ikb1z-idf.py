"""Compile/link both IKB I2C families with a selected installed SDK; never flash.

All projects/logs stay under the repository target directory. No installation,
downloads, SDK edits, bus initialization or physical tests are performed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--idf-path", required=True, type=Path)
    parser.add_argument("--tools-path", required=True, type=Path)
    parser.add_argument("--python", required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    target = root / "src-tauri/target/ikb1z-idf"
    target.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="run-", dir=target))
    version_text = (args.idf_path / "tools/cmake/version.cmake").read_text(encoding="utf-8")
    sdk = ".".join(re.search(r"set\(IDF_VERSION_" + part + r"\s+(\d+)\)", version_text).group(1)
                   for part in ("MAJOR", "MINOR", "PATCH"))
    env = os.environ.copy()
    env.update(IDF_PATH=str(args.idf_path.resolve()), IDF_TOOLS_PATH=str(args.tools_path.resolve()),
               IDF_CCACHE_ENABLE="0", IDF_COMPONENT_MANAGER="0", PYTHONNOUSERSITE="1")
    export = subprocess.run([str(args.python), str(args.idf_path / "tools/idf_tools.py"),
                             "export", "--format", "key-value"], env=env, capture_output=True, text=True, check=True)
    for line in export.stdout.splitlines():
        key, separator, value = line.partition("=")
        if separator and key in {"PATH", "IDF_PYTHON_ENV_PATH", "ESP_ROM_ELF_DIR", "OPENOCD_SCRIPTS", "ESP_IDF_VERSION"}:
            env[key] = value.replace("%PATH%", os.environ.get("PATH", "")).replace("$PATH", os.environ.get("PATH", ""))
    results = []
    for family in ("master", "legacy"):
        project = output / family
        main_dir = project / "main"
        main_dir.mkdir(parents=True)
        for name in ("ikb1z.c", "ikb1z.h", "ikb1z_esp_idf.c", "ikb1z_esp_idf.h"):
            shutil.copyfile(root / "knowledge_base/drivers" / name, main_dir / name)
        (project / "CMakeLists.txt").write_text(
            'cmake_minimum_required(VERSION 3.16)\ninclude($ENV{IDF_PATH}/tools/cmake/project.cmake)\nproject(ikb1z_validation)\n', encoding="utf-8")
        (project / "sdkconfig.defaults").write_text('CONFIG_IDF_TARGET="esp32"\nCONFIG_FREERTOS_HZ=1000\n', encoding="utf-8")
        dependency = "driver" if family == "legacy" else "esp_driver_i2c"
        (main_dir / "CMakeLists.txt").write_text(
            'idf_component_register(SRCS "main.c" "ikb1z.c" "ikb1z_esp_idf.c" INCLUDE_DIRS "." REQUIRES '
            + dependency + ' freertos)\n'
            + ('target_compile_definitions(${COMPONENT_LIB} PUBLIC IKB1Z_USE_LEGACY_I2C)\n' if family == "legacy" else ""), encoding="utf-8")
        # Invalid/null config exercises link-time references without performing
        # hardware I/O even if somebody later runs this compilation fixture.
        call = ('ikb1z_esp_idf_init_legacy(&device,&context,I2C_NUM_0,NULL,0x48,100)'
                if family == "legacy" else 'ikb1z_esp_idf_attach(&device,&context,NULL,NULL,0x48,100000,100)')
        (main_dir / "main.c").write_text('#include "ikb1z_esp_idf.h"\nvoid app_main(void) {\n'
            '    ikb1z_t device = {0};\n    ikb1z_esp_idf_t context = {0};\n    (void)' + call + ';\n}\n', encoding="utf-8")
        log = output / (family + ".log")
        print("Building iKB-1Z " + family + " with ESP-IDF " + sdk, flush=True)
        with log.open("w", encoding="utf-8") as stream:
            run = subprocess.run([str(args.python), str(args.idf_path / "tools/idf.py"),
                                  "-C", str(project), "-DCCACHE_ENABLE=0", "build"],
                                 env=env, stdout=stream, stderr=subprocess.STDOUT)
        binary = project / "build/ikb1z_validation.bin"
        result = {"family": family, "sdk": sdk, "exit_code": run.returncode,
                  "firmware_created": binary.exists(), "physical_tested": False,
                  "flashed": False, "log": log.name,
                  "source_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                    for p in sorted(main_dir.glob("ikb1z*"))}}
        results.append(result)
        print(json.dumps(result), flush=True)
        if run.returncode:
            print("\n".join(log.read_text(encoding="utf-8", errors="replace").splitlines()[-35:]), flush=True)
    (output / "results.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    raise SystemExit(0 if all(r["exit_code"] == 0 and r["firmware_created"] for r in results) else 1)


if __name__ == "__main__":
    main()
