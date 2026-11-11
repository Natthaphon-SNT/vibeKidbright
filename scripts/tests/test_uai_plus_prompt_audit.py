"""Cross-file coverage and source declarations for all 128 reviewed prompts."""
import csv
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT=Path(__file__).resolve().parents[2]
def read_json(path): return json.loads((ROOT/path).read_text(encoding="utf-8"))


class PromptAudit(unittest.TestCase):
    def setUp(self):
        self.registry=read_json("knowledge_base/compatibility/uai_plus_test_contracts.json")
        self.evidence=read_json(self.registry["api_evidence"])

    def test_every_prompt_has_one_audit_row_and_hardware_results_are_separate(self):
        catalog=read_json("docs/research/uai-plus-runtime-2026-10-26/test-prompts-th.json")
        ids=[f"MP-{i:03}" for i in range(1,129)]
        self.assertEqual([t["id"] for t in self.registry["tests"]],ids)
        self.assertEqual([t["audit"]["id"] for t in catalog["tests"]],ids)
        self.assertEqual(sum(self.registry["summary"].values()),128)
        with (ROOT/"docs/KIDBRIGHT_MAI_PLUS_PROMPT_AUDIT_2026-10-27.csv").open(encoding="utf-8-sig",newline="") as stream:
            rows=list(csv.DictReader(stream))
        self.assertEqual([row["test_id"] for row in rows],ids)
        self.assertTrue(all(row["physical_result"]=="NOT_RUN" for row in rows))
        self.assertFalse(self.registry["live_model_generation_tested"])
        self.assertFalse(self.registry["all_cases_have_generated_programs"])

    def test_required_apis_exist_in_the_actual_image_reference(self):
        self.assertEqual(hashlib.sha256((ROOT/self.registry["api_evidence"]).read_bytes()).hexdigest(),self.registry["api_evidence_sha256"])
        for contract in self.registry["contracts"].values():
            for module,symbols in contract["symbols"].items():
                for symbol in symbols:
                    self.assertIn(symbol,self.evidence["modules"][module]["symbols"])
        self.assertNotIn("get_pin_function",self.evidence["modules"]["peripheral.pinmap"]["symbols"])
        self.assertNotIn("Player.stop",self.evidence["modules"]["audio"]["symbols"])
        self.assertNotIn("Player.close",self.evidence["modules"]["audio"]["symbols"])

    def test_templates_exist_and_their_hashes_match_the_audit(self):
        factory=(ROOT/"src-tauri/src/ai/uai_plus.rs").read_text(encoding="utf-8")
        for name,template in self.registry["templates"].items():
            self.assertIn('"'+name+'"',factory)
            self.assertEqual(hashlib.sha256((ROOT/template["main"]).read_bytes()).hexdigest(),template["sha256"])
            compile((ROOT/template["main"]).read_text(encoding="utf-8"),template["main"],"exec")
        ready=[t for t in self.registry["tests"] if t["state"]=="TEMPLATE_CHECKED"]
        self.assertEqual(len(ready),7)
        self.assertTrue(all(t["template"] in self.registry["templates"] for t in ready))

    def test_known_blockers_are_not_disguised_as_ready_hardware_tests(self):
        tests={t["id"]:t for t in self.registry["tests"]}
        for id in ["MP-020","MP-037","MP-038","MP-050","MP-052","MP-072","MP-121","MP-124","MP-127"]:
            self.assertEqual(tests[id]["state"],"HOLD")
            self.assertTrue(tests[id]["prerequisites"])
        self.assertIn("หก",tests["MP-065"]["correction"])
        self.assertTrue(tests["MP-116"]["expected_failure"])
        self.assertTrue(tests["MP-120"]["desktop_only"])
        self.assertEqual(tests["MP-128"]["action"],"analyze_results")
