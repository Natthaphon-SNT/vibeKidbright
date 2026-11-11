"""Read selected API declarations from the supplied image; never run image code."""
import argparse
import ast
import hashlib
import json
import posixpath
import stat
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src-tauri/target/mai-image-tools"))
from dissect.extfs import ExtFS
from dissect.util.stream import RangeStream

MODULES = ["display", "image", "app", "time", "sys", "touchscreen", "camera", "audio", "http",
           "peripheral/gpio", "peripheral/pinmap", "peripheral/pwm", "peripheral/i2c", "peripheral/uart", "peripheral/adc",
           "ext_dev/imu", "ext_dev/qmi8658", "ext_dev/bm8563", "network/wifi", "nn/__init__"]
PREFIX = "/usr/lib/python3.11/site-packages/maix/_maix/"


def declarations(data):
    symbols = {}
    def walk(body, prefix=""):
        for node in body:
            if isinstance(node, ast.ClassDef):
                symbols[prefix + node.name] = {"kind": "class"}
                walk(node.body, prefix + node.name + ".")
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                if node.name.startswith("__") and node.name != "__init__": continue
                symbols[prefix + node.name] = {"kind": "function", "signature": ast.unparse(node.args),
                    "returns": ast.unparse(node.returns) if node.returns else None}
            elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
                if not node.target.id.startswith("__"):
                    symbols[prefix + node.target.id] = {"kind": "constant", "annotation": ast.unparse(node.annotation)}
    walk(ast.parse(data.decode("utf-8")).body)
    return symbols


def collect(image_path):
    result = {"schema_version": 1, "date": "2026-10-27", "maixpy": "4.12.5",
              "method": "Read-only extfs parsing and AST declarations; no mount, device connection or foreign-code execution.",
              "physical_runtime_tested": False, "modules": {}, "absent_paths": []}
    digest = hashlib.sha256()
    with image_path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(4 * 1024 * 1024), b""): digest.update(chunk)
        stream.seek(0)
        header = stream.read(512)
        partition = next(header[446+i*16:462+i*16] for i in range(4) if header[450+i*16] == 0x83)
        start, count = struct.unpack_from("<II", partition, 8)
        fs = ExtFS(RangeStream(stream, start * 512, count * 512))

        def resolve(path, depth=0):
            if depth > 16: raise ValueError("symlink depth")
            parts = [p for p in posixpath.normpath("/" + path.lstrip("/")).split("/") if p]
            node, done = fs.root, []
            for index, part in enumerate(parts):
                node = node.listdir()[part]
                if node.filetype == stat.S_IFLNK:
                    target = node.link if node.link.startswith("/") else "/".join(done + [node.link])
                    return resolve(posixpath.join(target, *parts[index+1:]), depth+1)
                done.append(part)
            return node

        for name in MODULES:
            path = PREFIX + name + ".pyi"
            node = resolve(path)
            if node.filetype != stat.S_IFREG or node.size > 512 * 1024: raise ValueError(path)
            with node.open() as source: data = source.read(node.size)
            module = name.replace("/", ".").removesuffix(".__init__")
            result["modules"][module] = {"path": path, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
                                           "symbols": declarations(data)}
        for path in [PREFIX + "ext_dev/rtc.pyi", "/root/servo.py", "/root/scripts/servo.py",
                     "/usr/lib/python3.11/site-packages/servo.py"]:
            try: resolve(path)
            except KeyError: result["absent_paths"].append(path)
        result["image"] = {"filename": image_path.name, "bytes": image_path.stat().st_size, "sha256": digest.hexdigest()}
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/research/uai-plus-runtime-2026-10-26/api-contracts-2026-10-27.json")
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to(ROOT): parser.error("output must stay inside this workspace")
    report = collect(args.image)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"modules": len(report["modules"]), "symbols": sum(len(m["symbols"]) for m in report["modules"].values()),
                      "image_sha256": report["image"]["sha256"], "output": output.relative_to(ROOT).as_posix()}))
