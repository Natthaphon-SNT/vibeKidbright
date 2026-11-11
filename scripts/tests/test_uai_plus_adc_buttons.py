"""MP-021 measured-band tests. The fake ADC does not certify physical buttons."""
import contextlib
import io
import json
import runpy
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch

SAMPLE = Path(__file__).resolve().parents[2] / "samples/kidbright-uai-plus/adc_buttons.py"


def runtime(bands=None, *, stop_at=None, missing=None, broken_read=False,
            driver_error=False, optional_close=False, peripheral_alias=False):
    bands = bands or {"released": [3900, 3910, 3920], "s1": [80, 90, 100], "s2": [600, 620, 640]}
    state = {"now": 0.0, "phase": None, "finished": False, "closed": 0, "instances": 0}
    frames, reads = [], []
    instructions = {"RELEASE S1 AND S2": "released", "HOLD S1 ONLY": "s1", "HOLD S2 ONLY": "s2"}

    class Canvas:
        def __init__(self, w, h, fmt):
            assert (w, h, fmt) == (480, 640, "rgb888")
            self.text = []
        def draw_rect(self, *args, **kwargs): self.text = []
        def draw_string(self, x, y, text, **kwargs):
            assert 0 <= x < 480 and 0 <= y < 640
            self.text.append(text)

    class Display:
        def width(self): return 480
        def height(self): return 640
        def show(self, canvas):
            frames.append(canvas.text[:])
            if len(canvas.text) > 1 and canvas.text[1] in instructions:
                state["phase"] = instructions[canvas.text[1]]
            if any("SKIP:" in text for text in canvas.text): state["finished"] = True
            if canvas.text[0] == "MP-021 MEASURED STATS" and len(canvas.text) == 9:
                state["finished"] = True

    class ADC:
        def __init__(self, pin, resolution):
            assert (pin, resolution) == (0, 12)
            state["instances"] += 1
            if driver_error: raise OSError("fixture ADC open failure")
        def read(self):
            if broken_read: raise OSError("fixture ADC read failure")
            phase = state["phase"]
            sequence = bands[phase]
            index = sum(1 for _, p, _ in reads if p == phase)
            value = sequence[index % len(sequence)]
            reads.append((state["now"], phase, value))
            return value

    if optional_close:
        ADC.close = lambda self: state.__setitem__("closed", state["closed"] + 1)
    if missing == "read": ADC.read = None
    adc = types.SimpleNamespace(ADC=ADC, RES_BIT_12=12)
    if missing == "factory": adc.ADC = None
    if missing == "resolution": del adc.RES_BIT_12
    module = types.ModuleType("maix")
    module.display = types.SimpleNamespace(Display=Display)
    module.image = types.SimpleNamespace(Image=Canvas, Format=types.SimpleNamespace(FMT_RGB888="rgb888"),
        Color=types.SimpleNamespace(from_rgb=lambda *rgb: rgb))
    module.app = types.SimpleNamespace(need_exit=lambda: state["finished"] or (stop_at is not None and state["now"] >= stop_at))
    module.time = types.SimpleNamespace(sleep_ms=lambda ms: state.__setitem__("now", state["now"] + ms / 1000))
    modules = {"maix": module}
    if peripheral_alias:
        peripheral = types.ModuleType("maix.peripheral")
        peripheral.adc = adc
        modules["maix.peripheral"] = peripheral
    else:
        module.adc = adc
    return modules, state, frames, reads


def execute(fixture):
    modules, state, frames, reads = fixture
    output = io.StringIO()
    with patch.dict(sys.modules, modules), patch("time.monotonic", lambda: state["now"]), contextlib.redirect_stdout(output):
        namespace = runpy.run_path(str(SAMPLE), run_name="__main__")
    return output.getvalue(), namespace


def payload(output, tag):
    prefix = "[MP-021] " + tag + " "
    return json.loads(next(line[len(prefix):] for line in output.splitlines() if line.startswith(prefix)))


class AdcButtonsExample(unittest.TestCase):
    def test_samples_three_actual_windows_and_reports_all_statistics_before_candidates(self):
        fixture = runtime()
        output, _ = execute(fixture)
        results = payload(output, "STATS_COMPLETE")
        proposal = payload(output, "THRESHOLD_CANDIDATES")
        self.assertEqual([r["phase"] for r in results], ["released", "s1", "s2"])
        self.assertEqual([(r["min"], r["max"], r["median"]) for r in results], [(3900, 3920, 3910), (80, 100, 90), (600, 640, 620)])
        for result in results:
            self.assertGreaterEqual(result["count"], 20)
            self.assertAlmostEqual(result["elapsed_s"], 3.0, delta=0.021)
            times = [when for when, phase, _ in fixture[3] if phase == result["phase"]]
            self.assertLess(times[-1] - times[0], 3.0)
            self.assertGreaterEqual(times[0], 5.0)
        self.assertLess(output.index("STATS_COMPLETE"), output.index("THRESHOLD_CANDIDATES"))
        self.assertEqual([c["raw_threshold"] for c in proposal["candidates"]], [350.0, 2270.0])
        self.assertEqual(proposal["status"], "WAIT_USER")
        self.assertEqual(fixture[1]["instances"], 1)
        self.assertTrue(any(frame[0] == "MP-021 MEASURED STATS" for frame in fixture[2]))
        self.assertNotIn("[MP-021] PASS", output)

    def test_does_not_assume_numeric_order_of_released_s1_s2(self):
        fixture = runtime({"released": [20], "s1": [3700], "s2": [1700]})
        output, _ = execute(fixture)
        proposal = payload(output, "THRESHOLD_CANDIDATES")
        self.assertEqual([c["between"] for c in proposal["candidates"]], [["released", "s2"], ["s2", "s1"]])
        self.assertEqual([c["raw_threshold"] for c in proposal["candidates"]], [860.0, 2700.0])

    def test_overlapping_or_identical_bands_never_produce_thresholds(self):
        for bands in [
            {"released": [100, 200], "s1": [150, 250], "s2": [600]},
            {"released": [900], "s1": [900], "s2": [900]},
        ]:
            with self.subTest(bands=bands):
                output, _ = execute(runtime(bands))
                proposal = payload(output, "THRESHOLD_CANDIDATES")
                self.assertEqual(proposal["status"], "INCONCLUSIVE")
                self.assertEqual(proposal["candidates"], [])

    def test_stop_during_preparation_or_collection_does_not_use_partial_bands(self):
        for stop_at in [1.0, 6.0, 16.0]:
            with self.subTest(stop_at=stop_at):
                fixture = runtime(stop_at=stop_at, optional_close=True)
                output, _ = execute(fixture)
                self.assertIn("CANCELLED", output)
                self.assertNotIn("THRESHOLD_CANDIDATES", output)
                self.assertEqual(fixture[1]["closed"], 1)

    def test_missing_api_is_skip_without_fake_samples(self):
        for missing in ["factory", "resolution", "read"]:
            with self.subTest(missing=missing):
                fixture = runtime(missing=missing, optional_close=True)
                output, _ = execute(fixture)
                self.assertIn("MISSING_API", output)
                self.assertIn("SKIP", output)
                self.assertEqual(fixture[3], [])
                self.assertNotIn("THRESHOLD_CANDIDATES", output)

    def test_driver_and_read_failures_remain_errors(self):
        for option in ["driver_error", "broken_read"]:
            with self.subTest(option=option):
                fixture = runtime(**{option: True}, optional_close=True)
                with self.assertRaisesRegex(OSError, "fixture ADC"):
                    execute(fixture)
                self.assertEqual(fixture[1]["closed"], 0 if option == "driver_error" else 1)

    def test_invalid_raw_values_are_not_treated_as_button_samples(self):
        for value in [-1, 4096, True, None, "123"]:
            with self.subTest(value=value):
                fixture = runtime({"released": [value], "s1": [80], "s2": [600]}, optional_close=True)
                with self.assertRaisesRegex(RuntimeError, "ADC_DATA_ERROR"):
                    execute(fixture)
                self.assertEqual(fixture[1]["closed"], 1)

    def test_documented_peripheral_import_alias_and_no_close_method_work(self):
        fixture = runtime(peripheral_alias=True)
        output, _ = execute(fixture)
        self.assertEqual(payload(output, "THRESHOLD_CANDIDATES")["status"], "WAIT_USER")
        self.assertEqual(fixture[1]["closed"], 0)

    def test_threshold_proposal_rejects_incomplete_or_too_small_sample_sets(self):
        _, namespace = execute(runtime(stop_at=1.0))
        propose = namespace["propose_thresholds"]
        self.assertEqual(propose([])["candidates"], [])
        small = [{"phase": phase, "count": 1, "min": value, "max": value} for phase, value in [("released", 3900), ("s1", 80), ("s2", 600)]]
        self.assertEqual(propose(small)["status"], "INCONCLUSIVE")


if __name__ == "__main__":
    unittest.main()
