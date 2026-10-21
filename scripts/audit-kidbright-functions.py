"""Static source inventory only. Never import, compile, or execute archive code."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/research/kidbright-2026-10-15-evidence.json"
VALIDATION = ROOT / "docs/research/kidbright-expansion-2026-10-17-validation.json"
LANGUAGE_VALIDATION = ROOT / "docs/research/learner-sidebar-language-2026-10-17-validation.json"
OUTPUT = ROOT / "docs/research/kidbright-function-audit-2026-10-16.json"
REPORT = ROOT / "docs/KIDBRIGHT_FUNCTION_COVERAGE_2026-10-16.md"
SKIP_NAMES = {"if", "for", "while", "switch", "catch", "sizeof", "return", "alignof", "static_assert"}
CLASS = re.compile(r"\b(class|struct)\s+([A-Za-z_]\w*)\s*[^;{}]*\{")
CALLABLE = re.compile(r"(?P<name>~?[A-Za-z_]\w*(?:::[~A-Za-z_]\w*)*)\s*\((?P<args>[^;{}]*?)\)\s*(?:const\s*|override\s*|final\s*|noexcept\s*|=\s*0\s*)*(?P<end>;|\{)")
DEFINITION = re.compile(r"(?m)^[ \t]*(?P<prefix>(?:[\w:*&<>]+[ \t]+)*)(?P<name>~?[A-Za-z_]\w*(?:::[~A-Za-z_]\w*)*)\s*\((?P<args>[^;{}]*?)\)\s*(?:const\s*)?\{")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_evidence(path: Path, validation: dict) -> dict:
    relative = path.relative_to(ROOT).as_posix()
    current_hash = digest(path)

    def matches(sources: list[dict]) -> bool:
        return (any(item["path"] == relative and item["sha256"] == current_hash for item in sources)
                and all((ROOT / item["path"]).is_file() and digest(ROOT / item["path"]) == item["sha256"] for item in sources))

    host = [suite for suite in validation.get("host_c_checks", [])
            if suite.get("status") == "passed" and matches(suite.get("sources", []))]
    firmware = [build for build in validation.get("firmware_builds", [])
                if build.get("exit_code") == 0 and matches(build.get("sources", []))]
    return {"validation_path": VALIDATION.relative_to(ROOT).as_posix(),
            "matching_host_suites": [{"suite": suite["suite"], "checks": suite["checks"]} for suite in host],
            "matching_firmware_builds": [{"example": build["example"], "board_profile": build["board_profile"], "sdk_version": build["sdk_version"]} for build in firmware],
            "status": "recorded_source_and_dependency_hash_matched" if host or firmware else "no_current_hash_matched_build_evidence"}


def source_without_comments(text: str) -> str:
    # Preserve offsets/line numbers; quoted strings remain visible in signatures.
    return re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|//[^\n]*|/\*[\s\S]*?\*/',
                  lambda match: re.sub(r"[^\n]", " ", match[0]) if match[0].startswith(("//", "/*")) else match[0], text)


def braces(text: str) -> tuple[dict[int, int], list[int]]:
    pairs, stack, depths = {}, [], [0] * (len(text) + 1)
    masked = re.sub(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', lambda match: " " * len(match[0]), text)
    for index, character in enumerate(masked):
        depths[index] = len(stack)
        if character == "{":
            stack.append(index)
        elif character == "}" and stack:
            pairs[stack.pop()] = index
    return pairs, depths


def method_inventory(text: str) -> tuple[list[dict], list[dict]]:
    clean = source_without_comments(text)
    pairs, depths = braces(clean)
    classes, ranges = [], []
    for match in CLASS.finditer(clean):
        opening = match.end() - 1
        if opening not in pairs:
            continue
        closing = pairs[opening]
        ranges.append((opening, closing))
        methods = []
        body_start = opening + 1
        access = "private" if match[1] == "class" else "public"
        access_positions = list(re.finditer(r"\b(public|protected|private)\s*:", clean[body_start:closing]))
        for method in CALLABLE.finditer(clean, body_start, closing):
            if method["name"] in SKIP_NAMES or depths[method.start()] != depths[opening] + 1:
                continue
            for label in access_positions:
                if body_start + label.start() < method.start() and depths[body_start + label.start()] == depths[opening] + 1:
                    access = label[1]
            start = max(clean.rfind(";", body_start, method.start()), clean.rfind("}", body_start, method.start()), clean.rfind("{", body_start, method.start()), clean.rfind("\n", body_start, method.start())) + 1
            signature = re.sub(r"\s+", " ", clean[start:method.end()]).strip()
            methods.append({"name": method["name"], "signature": signature, "line": clean.count("\n", 0, method.start()) + 1,
                            "access": access, "kind": "inline_definition_candidate" if method["end"] == "{" else "declaration_candidate",
                            "compatibility_status": "not_reviewed_no_equivalence_proven"})
        classes.append({"name": match[2], "kind": match[1], "line": clean.count("\n", 0, match.start()) + 1, "methods": methods})
    free = []
    for match in CALLABLE.finditer(clean):
        if match["name"] in SKIP_NAMES or any(start < match.start() < end for start, end in ranges):
            continue
        # Candidate only: namespaces, templates and macros are not compiler-resolved.
        if depths[match.start()] > 1:
            continue
        free.append({"name": match["name"], "signature": re.sub(r"\s+", " ", match[0]),
                     "line": clean.count("\n", 0, match.start()) + 1, "kind": "free_callable_candidate",
                     "compatibility_status": "not_reviewed_no_equivalence_proven"})
    return classes, free


def definitions(text: str) -> list[dict]:
    clean = source_without_comments(text)
    output = []
    for match in DEFINITION.finditer(clean):
        if match["name"] in SKIP_NAMES:
            continue
        output.append({"name": match["name"], "line": clean.count("\n", 0, match.start()) + 1,
                       "signature": re.sub(r"\s+", " ", match[0]).strip(),
                       "static": "static" in match["prefix"].split(),
                       "compatibility_status": "not_reviewed_no_equivalence_proven"})
    return output


def mapping(function: str, original: list[str], scope: str, differences: str, kind: str = "portable_core") -> dict:
    return {"c_function": function, "implementation_kind": kind, "original_contract_references": original,
            "review_status": "manually_reviewed_scoped_behavior_only", "scope": scope, "explicit_differences_or_limits": differences,
            "host_behavior_checks": kind == "portable_core", "esp_idf_recipe_build_evidence": kind == "recipe",
            "original_runtime_differential_tested": False, "physical_board_tested": False, "whole_original_method_equivalence_proven": False}


def run(archive: Path) -> None:
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    validation = json.loads(VALIDATION.read_text(encoding="utf-8"))
    language_validation = json.loads(LANGUAGE_VALIDATION.read_text(encoding="utf-8"))
    prior = {entry["path"]: entry for entry in evidence["archive_sources"]}
    all_files = sorted(path for path in archive.rglob("*") if path.is_file())
    mismatches = [path.relative_to(archive).as_posix() for path in all_files
                  if path.relative_to(archive).as_posix() not in prior or digest(path) != prior[path.relative_to(archive).as_posix()]["sha256"]]
    missing = sorted(set(prior) - {path.relative_to(archive).as_posix() for path in all_files})
    original_headers, original_sources = [], []
    for path in all_files:
        if path.suffix.lower() not in {".h", ".hpp", ".c", ".cpp", ".cc"}:
            continue
        relative = path.relative_to(archive).as_posix()
        entry = {"path": relative, "sha256": digest(path), "bytes": path.stat().st_size,
                 "prior_evidence_hash_matches": relative in prior and digest(path) == prior[relative]["sha256"],
                 "scope": "third_party_rapidjson" if "/rapidjson/" in relative else "kidbright_core" if "/lib/release/" in relative else "kidbright_plugin"}
        text = path.read_text(encoding="utf-8-sig", errors="replace")
        if path.suffix.lower() in {".h", ".hpp"}:
            entry["classes"], entry["free_callable_candidates"] = method_inventory(text)
            original_headers.append(entry)
        else:
            entry["definition_candidates"] = definitions(text)
            original_sources.append(entry)

    current = []
    for folder in [ROOT / "knowledge_base/sensor_examples", ROOT / "knowledge_base/compatibility"]:
        for path in sorted(folder.glob("*.c")):
            text = path.read_text(encoding="utf-8-sig", errors="replace")
            clean = source_without_comments(text)
            includes = re.findall(r'^\s*#\s*include\s*[<"]([^>"\n]+)', clean, re.M)
            arduino = any(item in includes for item in ["Arduino.h", "Wire.h", "WiFi.h", "PID_v1.h", "PubSubClient.h"])
            old_i2c, new_i2c = "driver/i2c.h" in includes, any(item in includes for item in ["driver/i2c_master.h", "driver/i2c_slave.h"])
            current.append({"path": path.relative_to(ROOT).as_posix(), "sha256": digest(path), "includes": includes,
                            "source_family": "arduino_cpp_reference_despite_c_extension" if arduino else "portable_c_core" if path.name in {"kb_core.c", "kb_basic.c"} else "esp_idf_c_reference",
                            "i2c_family": "mixed_legacy_and_new" if old_i2c and new_i2c else "legacy_driver_i2c_h_deprecated_in_idf_5" if old_i2c else "new_i2c_driver" if new_i2c else "none_includes_observed",
                            "legacy_adc_header": "driver/adc.h" in includes,
                            "callable_definition_candidates": definitions(text),
                            "recorded_build_evidence": source_evidence(path, validation),
                            "equivalence_status": "manual_scoped_mapping_only" if folder.name == "compatibility" else "reference_only_no_compatibility_proven"})

    core_header = ROOT / "knowledge_base/compatibility/kb_core.h"
    core_clean = source_without_comments(core_header.read_text(encoding="utf-8"))
    exports = [{"name": match[1], "signature": re.sub(r"\s+", " ", match[0]).strip(), "line": core_clean.count("\n", 0, match.start()) + 1}
               for match in re.finditer(r"\b(?:bool|double|size_t)\s+(kb_\w+)\s*\([^;]*?\)\s*;", core_clean)]
    explicit = [
        mapping("kb_delay_ms", ["kbide/app/public/blockly/kidbright/generators.js: basic_delay", "kbide/plugins/pid_control/pid_con/generators.js: ms_delay"], "Nonnegative finite seconds to truncated whole milliseconds", "Rejects invalid/overflow input; RTOS scheduling/ticks are outside this helper"),
        mapping("kb_lm73_celsius", ["kbide/esp32/lib/release/kidbright32/include/lm73.h: LM73::get", "existing LM73 C recipe raw format"], "Signed two-byte LM73 raw /128 conversion", "Original LM73 implementation is not present in captured archive source; no original state machine or full method equivalence claimed"),
        mapping("kb_pid_step", ["kbide/plugins/pid_control/pid_con/pid_con.cpp: PID_CON::run"], "Time-based PID; integral ±60, output ±100", "Double state, proportional-only first sample and rejection of invalid/nonpositive dt are explicit repairs"),
        mapping("kb_sma_step", ["kbide/plugins/pid_control/filter/filter.cpp: FILTER::sma"], "Warm-up and rolling average for windows 1..50", "Window changes reset state, invalid windows are rejected; no EMA implementation"),
        mapping("kb_joystick_init", ["kbide/plugins/pid_control/mjoystick/mjoystick.cpp: MJOYSTICK::calibrate_set_x", "kbide/plugins/pid_control/mjoystick/mjoystick.cpp: MJOYSTICK::calibrate_set_y"], "Per-axis calibration, float slopes, descending endpoints and center ±500", "Caller supplies ADC raw values; validation and state ownership differ from Device object"),
        mapping("kb_joystick_read", ["kbide/plugins/pid_control/mjoystick/mjoystick.cpp: MJOYSTICK::fPosX", "kbide/plugins/pid_control/mjoystick/mjoystick.cpp: MJOYSTICK::fPosY"], "Normalization, strict dead-zone boundaries, mapping and integer truncation", "No ADC acquisition or original object scheduling; only one caller-owned axis state"),
        mapping("kb_ascii_format", ["kbide/plugins/esp_now/esp_now_class/esp_now_class.cpp: ESP_NOW_CLASS::send(float)", "kbide/plugins/esp_now/esp_now_class/esp_now_class.cpp: ESP_NOW_CLASS::send(char*,float)"], "ASCII %f numeric payload, excludes transmitted NUL", "Bounded buffer and finite input validation; no radio setup, peer management or send implementation"),
        mapping("kb_ascii_parse", ["kbide/plugins/esp_now/esp_now_class/esp_now_class.cpp: ESP_NOW_CLASS::readNumber"], "ASCII numeric decode", "Rejects malformed/nonfinite/embedded-NUL packets and trailing junk rather than permissive sscanf; max64 bytes; no receiver state"),
        mapping("button_buzzer.c::app_main", ["kbide/esp32/lib/release/kidbright32/include/button12.h: BUTTON12::wait_sw1_pressed / wait_sw1_released", "kbide/esp32/lib/release/kidbright32/include/sound.h: SOUND::on / off; note/rest behavior subset"], "S1 active low, 440 Hz/500ms, wait release", "GPIO16/GPIO13 only, LEDC rather than original MCPWM; no arbitrary note table, BPM, volume or debounce equivalence", "recipe"),
        mapping("temperature_serial.c::app_main", ["kbide/esp32/lib/release/kidbright32/include/lm73.h: LM73::get"], "LM73 0x4D register0, new I2C master, 500ms serial output", "SDA4/SCL5 only; no original Device async state machine, physical comparison or timing equivalence", "recipe"),
    ]
    # Deliberate, manually reviewed references; never infer a mapping from similar names.
    scoped_methods = {
        ("pid_control/pid_con/pid_con.h", "PID_CON", "run"): ["kb_pid_step"],
        ("pid_control/filter/filter.h", "FILTER", "sma"): ["kb_sma_step"],
        ("pid_control/mjoystick/mjoystick.h", "MJOYSTICK", "calibrate_set_x"): ["kb_joystick_init"],
        ("pid_control/mjoystick/mjoystick.h", "MJOYSTICK", "calibrate_set_y"): ["kb_joystick_init"],
        ("pid_control/mjoystick/mjoystick.h", "MJOYSTICK", "fPosX"): ["kb_joystick_read"],
        ("pid_control/mjoystick/mjoystick.h", "MJOYSTICK", "fPosY"): ["kb_joystick_read"],
        ("esp_now/esp_now_class/esp_now_class.h", "ESP_NOW_CLASS", "send"): ["kb_ascii_format"],
        ("esp_now/esp_now_class/esp_now_class.h", "ESP_NOW_CLASS", "readNumber"): ["kb_ascii_parse"],
        ("kidbright32/include/lm73.h", "LM73", "get"): ["kb_lm73_celsius", "temperature_serial.c::app_main"],
        ("kidbright32/include/button12.h", "BUTTON12", "wait_sw1_pressed"): ["button_buzzer.c::app_main"],
        ("kidbright32/include/button12.h", "BUTTON12", "wait_sw1_released"): ["button_buzzer.c::app_main"],
        ("kidbright32/include/sound.h", "SOUND", "on"): ["button_buzzer.c::app_main"],
        ("kidbright32/include/sound.h", "SOUND", "off"): ["button_buzzer.c::app_main"],
    }
    for entry in original_headers:
        for cls in entry["classes"]:
            for method in cls["methods"]:
                for (suffix, class_name, method_name), references in scoped_methods.items():
                    if entry["path"].endswith(suffix) and cls["name"] == class_name and method["name"] == method_name:
                        if method_name == "send" and "float" not in method["signature"]:
                            continue
                        method["manual_scoped_mapping_references"] = references
                        method["compatibility_status"] = "scoped_behavior_review_only_whole_method_unproven"
    for entry in current:
        if entry["path"].startswith("knowledge_base/compatibility/"):
            for function in entry["callable_definition_candidates"]:
                identifier = function["name"] if entry["path"].endswith("kb_core.c") else entry["path"].split("/")[-1] + "::" + function["name"]
                if any(item["c_function"] == identifier for item in explicit):
                    function["manual_scoped_mapping_reference"] = identifier
                    function["compatibility_status"] = "scoped_behavior_review_only_whole_original_method_unproven"
    primary = [entry for entry in original_headers if entry["scope"] != "third_party_rapidjson"]
    vendor = [entry for entry in original_headers if entry["scope"] == "third_party_rapidjson"]
    methods = [method for entry in primary for cls in entry["classes"] for method in cls["methods"]]
    summary = {"captured_archive_files": len(all_files), "prior_recorded_archive_files": evidence["archive"]["extracted_source_files"],
               "archive_hash_mismatches": len(mismatches), "archive_missing_files": len(missing),
               "primary_headers": len(primary), "primary_class_candidates": sum(len(entry["classes"]) for entry in primary),
               "primary_method_candidates": len(methods), "primary_public_method_candidates": sum(method["access"] == "public" for method in methods),
               "primary_methods_with_scoped_manual_reference": sum("manual_scoped_mapping_references" in method for method in methods),
               "primary_free_callable_candidates": sum(len(entry["free_callable_candidates"]) for entry in primary),
               "vendor_headers": len(vendor), "vendor_method_candidates": sum(len(cls["methods"]) for entry in vendor for cls in entry["classes"]),
               "original_c_cpp_source_files": len(original_sources), "original_definition_candidates": sum(len(entry["definition_candidates"]) for entry in original_sources),
               "current_c_files": len(current), "current_callable_definition_candidates": sum(len(entry["callable_definition_candidates"]) for entry in current),
               "current_c_families": dict(Counter(entry["source_family"] for entry in current)),
               "current_i2c_families": dict(Counter(entry["i2c_family"] for entry in current)),
               "portable_core_exported_functions": len(exports), "manual_scoped_mappings": len(explicit), "proven_full_method_equivalence": 0}
    rust_check = next(item for item in validation["software_checks"] if item["command"].startswith("cargo test "))
    language_paths = ["src-tauri/src/ai/language.rs", "src-tauri/src/ai/providers.rs"]
    current_hashes = {item["path"]: item["sha256"] for item in validation["source_hashes"]}
    language_hashes_match = all(current_hashes.get(path) == digest(ROOT / path) for path in language_paths)
    output = {"schema_version": 1, "audit_date": "2026-10-17", "timezone": "Asia/Bangkok",
              "conclusion": "Function-level compatibility is incomplete; source declarations and names are inventory candidates, never proof of behavioral equivalence.",
              "method": "Read-only SHA256 reconciliation and regex/brace-depth source inventory. No original source execution, compiler semantic analysis, radio, hardware, AI or network invocation.",
              "limits": ["Only the 410 captured files from the prior ZIP extraction are considered, not all archive binaries, dependencies, KidBright releases or extensions.", "C++ macros/templates/operators/conditional compilation and inheritance are not resolved; parser candidates can include false positives or omissions.", "Repeated lifecycle methods and overload declarations are separate candidates; these counts are not a compatibility percentage.", "RapidJSON vendor headers are inventoried separately and excluded from the primary KidBright method count.", "Manual mappings cover only scoped numerical/protocol behavior and two recipes; every original method remains unproven as a whole.", "Reference C files have not been compiled by this inventory; the Arduino C++ file is not an ESP-IDF C implementation.", "Input hashes describe the files at audit generation time; rerun after changing source/evidence files."],
              "inputs": [{"path": path.relative_to(ROOT).as_posix(), "sha256": digest(path)} for path in [EVIDENCE, VALIDATION, LANGUAGE_VALIDATION]],
              "archive_provenance": {"original_zip_sha256_from_prior_evidence": evidence["archive"]["sha256"], "zip_rehashed_in_this_audit": False,
                                     "captured_source_hashes_rechecked": True, "mismatched_paths": mismatches, "missing_paths": missing},
              "summary": summary, "original_headers": original_headers, "original_runtime_sources": original_sources,
              "current_c_sources": current, "core_exported_declarations": {"path": core_header.relative_to(ROOT).as_posix(), "sha256": digest(core_header), "functions": exports},
              "manual_scoped_mappings": explicit,
              "recorded_software_evidence": {"path": VALIDATION.relative_to(ROOT).as_posix(), "host_checks": sum(suite["checks"] for suite in validation["host_c_checks"]), "firmware_builds": len(validation["firmware_builds"]), "scope": "Recorded build/test results, bound to source/dependency hashes per file; no original runtime differential tests or agent hardware verification"},
              "offline_language_helper_evidence": {"command": rust_check["command"], "identified_cases": language_validation["language_regressions"]["offline_tests"], "case_inventory_path": LANGUAGE_VALIDATION.relative_to(ROOT).as_posix(), "suite_tests": rust_check["tests"], "status": rust_check["status"] if language_hashes_match else "stale_source_hashes", "current_source_hashes_match_recorded_full_suite": language_hashes_match, "source_paths": language_paths, "scope": "Identified offline source-selection/provider-payload cases run within the recorded full Rust suite. No multilingual model response was generated.", "live_ai_provider_tested": False}}
    OUTPUT.write_text(json.dumps(output, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    rows = "\n".join(f"| `{entry['path'].split('/')[-1]}` | {entry['source_family']} | {entry['i2c_family']} | {len(entry['callable_definition_candidates'])} |" for entry in current)
    REPORT.write_text(f"""# ตรวจระดับฟังก์ชัน KidBright — ปรับข้อมูล 17 ตุลาคม 2026

**ยังไม่ครบทุกฟังก์ชัน และยังไม่มีหลักฐานว่าเมธอดต้นฉบับใดเทียบเท่ากันทั้งเมธอดบนบอร์ดจริง** รายงานนี้เพิ่มรายการเมธอด C++ และฟังก์ชัน C แยกจากรายชื่อ Blockly เพื่อให้เห็นส่วนที่ยังไม่ได้ตรวจ โดยไม่ใช้ชื่อคล้ายกันเป็นหลักฐานว่าทำงานเหมือนกัน

## ขอบเขตและวิธีตรวจ

ตรวจไฟล์ที่คัดออกจาก ZIP ไว้ {summary['captured_archive_files']} ไฟล์ เทียบ SHA256 กับหลักฐานเดิม: ไม่ตรง {len(mismatches)} ไฟล์, ขาด {len(missing)} ไฟล์ SHA256 ZIP ที่อ้างจากการตรวจครั้งก่อนคือ `{evidence['archive']['sha256']}` รอบนี้ไม่ได้อ่าน ZIP ทั้งก้อนซ้ำ

รายการเป็น **candidate จาก regex และระดับวงเล็บปีกกา** ไม่ใช่ผล C++ compiler: macro, template, overload, operator, inheritance และเงื่อนไข preprocessor ยังทำให้มีรายการตกหล่นหรือเกินได้ จึงยังเรียกว่า “ครอบคลุมทุกฟังก์ชัน” ไม่ได้

| รายการใน snapshot | จำนวน |
|---|---:|
| Header ของ KidBright/plugin ไม่รวม RapidJSON | {summary['primary_headers']} |
| Class/struct candidates | {summary['primary_class_candidates']} |
| Method candidates ทุก access | {summary['primary_method_candidates']} |
| Public method candidates รวม constructor/lifecycle | {summary['primary_public_method_candidates']} |
| Header ของ RapidJSON ที่แยกเป็น dependency | {summary['vendor_headers']} |
| ไฟล์ runtime C/C++ ต้นฉบับ | {summary['original_c_cpp_source_files']} |
| ไฟล์ C ใน KB ปัจจุบัน | {summary['current_c_files']} |
| ฟังก์ชันที่ประกาศ export ใน `kb_core.h` | {summary['portable_core_exported_functions']} |
| การจับคู่ที่ตรวจขอบเขตด้วยตนเอง รวมสอง recipe | {summary['manual_scoped_mappings']} |
| เมธอดต้นฉบับที่พิสูจน์ความเทียบเท่าทั้งเมธอด | 0 |

## โค้ดปัจจุบันมีอะไร

| ไฟล์ | ตระกูลโค้ดจาก include | I2C จาก include | ฟังก์ชันที่พบ |
|---|---|---|---:|
{rows}

`balanced_robot.c` เป็น Arduino/C++ reference แม้นามสกุลเป็น `.c` ส่วนไฟล์ตัวอย่าง ESP-IDF อื่นใน `sensor_examples` ยังไม่ได้คอมไพล์ในรอบ audit นี้ การพบ include legacy I2C หมายถึงต้องเลือก driver family ให้สอดคล้องกับทั้งโปรแกรม ไม่ได้แปลว่าไฟล์ใช้ไม่ได้โดยอัตโนมัติ

## ส่วนที่ตรวจพฤติกรรมแล้วและข้อจำกัด

แกน C มี 8 ฟังก์ชัน: แปลงวินาทีเป็นมิลลิวินาที, ถอดค่า LM73, PID, SMA, ตั้งค่า/อ่าน joystick และเข้า/ถอดรหัส ASCII `%f` มีหลักฐาน host tests 122 checks แต่อ่านเทียบต้นฉบับเพียงขอบเขตที่ระบุ ไม่ได้รัน differential test กับ runtime ต้นฉบับ

PID ใช้ double และแก้ first-sample/invalid dt; SMA แก้การเปลี่ยนขนาดหน้าต่าง; codec ตรวจแพ็กเก็ตเข้มกว่าต้นฉบับ จึงไม่ใช่ bit-exact implementation เมธอด `FILTER::ema`, framework `Device/Driver`, radio/peer management และเมธอดอื่นไม่ถูกนับว่ารองรับจากการมี helper เหล่านี้

สอง recipe คือกด S1/เสียง 440 Hz 500 ms และ LM73/Serial 500 ms มีหลักฐาน build ESP-IDF 5.5.4 แต่ยังไม่ได้เทียบ waveform, เวลา, ค่าเซนเซอร์ หรือโปรแกรมจริงกับบอร์ด ไม่มีการ flash ใน audit นี้

รอบ 17 ตุลาคมเพิ่ม `kb_basic.c`, HT16K33 adapter และ recipe เพลง/ตัวเลข รวมเป็น 4 recipe และ 7 คู่ profile/recipe ที่ build ผ่านตาม [หลักฐาน expansion](research/kidbright-expansion-2026-10-17-validation.json) มี host checks 122 เดิมและ 368 ใหม่ แต่ไม่เพิ่มจำนวน original method mappings จาก primitive เหล่านี้โดยอัตโนมัติ สถานะ build รายไฟล์ใน JSON ต้องตรงแฮช source/dependencies ของผลทดสอบที่บันทึกไว้

ผู้ใช้รายงานโปรแกรม Hello/Heartbeat ผ่านบน iA เป็นผล basic runtime แยกจากการตรวจนี้ ดู [หลักฐานผู้ใช้](research/kidbright-inex-user-evidence-2026-10-17.json) ยังไม่ได้ทดสอบอุปกรณ์ต่อพ่วงด้วยไฟล์นั้น

ตัวช่วยเลือกภาษาตอบมี 15 identified offline cases ตามรายการทดสอบภาษาเดิม อยู่ใน Rust suite {rust_check['tests']} tests ที่บันทึกผลล่าสุด ตรวจเฉพาะการเลือกข้อความและ provider payload ไม่ได้ยืนยันผลตอบจริงครบทุกภาษา

รายละเอียดชื่อ/signature/access/บรรทัด/แฮชของทุก candidate ในขอบเขตอยู่ใน [JSON audit](research/kidbright-function-audit-2026-10-16.json) รายการที่ไม่มี manual mapping มีสถานะ `not_reviewed_no_equivalence_proven` เสมอ ไม่มีการคำนวณเปอร์เซ็นต์จากการจับชื่อ

## ทำซ้ำ

```powershell
python scripts/audit-kidbright-functions.py --archive-root (Join-Path $env:TEMP 'vibekidbright-research-20261015/archive')
```

สคริปต์อ่านไฟล์ต้นฉบับเป็นข้อมูลเท่านั้น ไม่ import, compile หรือ execute โค้ดใน archive แฮช input เป็น snapshot ณ เวลาสร้างรายงาน ควรรันซ้ำเมื่อแก้ source หรือหลักฐานที่อ้าง
""", encoding="utf-8")
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive-root", required=True, type=Path)
    args = parser.parse_args()
    if not args.archive_root.is_dir():
        parser.error("The previously extracted archive directory is required")
    run(args.archive_root)
