"""Actual Draft 2020-12 schema checks; native tests enforce semantic bounds."""
import copy
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs/ai-engineering"

class BlockContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads((DOCS / "block-mapping.schema.json").read_text("utf-8"))
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)
        cls.fixtures = [json.loads(p.read_text("utf-8")) for p in sorted((DOCS / "block-fixtures").glob("*.json"))]

    def test_all_four_existing_nested_fixtures_validate(self):
        self.assertEqual(len(self.fixtures), 4)
        for fixture in self.fixtures:
            self.validator.validate(fixture)

    def test_only_two_exact_board_profiles_are_accepted(self):
        for board in ["kidbright32ia", "kidbright32ip", "mai", "mai-plus", "esp32", "kidbright32_v15"]:
            fixture = copy.deepcopy(self.fixtures[0]); fixture["board_profile"] = board
            self.assertFalse(self.validator.is_valid(fixture), board)

    def test_schema_versions_and_extra_properties_are_rejected(self):
        for version in [0, 2, "1"]:
            fixture = copy.deepcopy(self.fixtures[0]); fixture["schema_version"] = version
            self.assertFalse(self.validator.is_valid(fixture))
        fixture = copy.deepcopy(self.fixtures[0]); fixture["execute"] = "shell"
        self.assertFalse(self.validator.is_valid(fixture))

    def test_incorrect_numeric_field_and_unknown_block_are_rejected(self):
        fixture = copy.deepcopy(self.fixtures[0])
        for block in [{"type":"basic_delay","fields":{"NUM":1}}, {"type":"run_shell","command":"bad"}]:
            fixture["blocks"] = [block]
            self.assertFalse(self.validator.is_valid(fixture))

    def test_generic_xml_is_not_a_normalized_program(self):
        self.assertFalse(self.validator.is_valid("<xml><block type='basic_delay'/></xml>"))
        self.assertFalse(self.validator.is_valid({"xml":"<xml/>"}))

    def test_schema_statement_bound_is_enforced(self):
        fixture = copy.deepcopy(self.fixtures[0])
        fixture["blocks"] = [{"type":"basic_delay","fields":{"VALUE":0.02}}] * 1001
        self.assertFalse(self.validator.is_valid(fixture))

if __name__ == "__main__":
    unittest.main()
