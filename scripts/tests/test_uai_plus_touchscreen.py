"""Touch event contract tests with a fake runtime; not a physical driver test."""
import contextlib
import io
import runpy
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch

SAMPLE = Path(__file__).resolve().parents[2] / "samples/kidbright-uai-plus/touchscreen.py"


def fake_runtime(events, width=480, height=640, driver_error=False, fail_read=False):
    frames, touch_instances = [], []
    state = {"iteration": 0}

    class Canvas:
        def __init__(self, w, h, fmt):
            assert (w, h, fmt) == (width, height, "rgb888")
            self.text, self.markers = [], []

        def draw_rect(self, *args, **kwargs):
            self.text, self.markers = [], []

        def draw_string(self, x, y, text, **kwargs):
            self.text.append(text)

        def draw_circle(self, x, y, radius, **kwargs):
            assert 0 <= x - radius and x + radius < width
            assert 0 <= y - radius and y + radius < height
            self.markers.append((x, y))

    class Display:
        def width(self): return width
        def height(self): return height
        def show(self, canvas): frames.append((canvas.text[:], canvas.markers[:]))

    class Touch:
        def __init__(self):
            self.closed = False
            self.reads = 0
            touch_instances.append(self)
        def is_opened(self): return not driver_error
        def available(self, timeout):
            assert timeout == 0
            return events[state["iteration"]] is not None
        def read(self):
            self.reads += 1
            if fail_read: raise RuntimeError("test read failure")
            return events[state["iteration"]]
        def close(self): self.closed = True

    module = types.ModuleType("maix")
    module.display = types.SimpleNamespace(Display=Display)
    module.image = types.SimpleNamespace(Image=Canvas, Format=types.SimpleNamespace(FMT_RGB888="rgb888"),
        Color=types.SimpleNamespace(from_rgb=lambda *rgb: rgb))
    module.touchscreen = types.SimpleNamespace(TouchScreen=Touch)
    module.app = types.SimpleNamespace(need_exit=lambda: state["iteration"] >= len(events))
    module.time = types.SimpleNamespace(sleep_ms=lambda ms: state.__setitem__("iteration", state["iteration"] + 1))
    return module, frames, touch_instances


def execute(module):
    output = io.StringIO()
    with patch.dict(sys.modules, {"maix": module}), contextlib.redirect_stdout(output):
        runpy.run_path(str(SAMPLE), run_name="__main__")
    return output.getvalue()


class TouchscreenExample(unittest.TestCase):
    def test_press_move_release_and_second_press_use_actual_events(self):
        module, frames, devices = fake_runtime([(20, 180, 1), (30, 200, 1), (30, 200, 0), None, (40, 220, 1)])
        execute(module)
        self.assertIn("Down=1 Up=0", frames[0][0])
        self.assertIn("Down=1 Up=1", frames[2][0])
        self.assertEqual(frames[2][1], [])
        self.assertIn("Down=2 Up=1", frames[-1][0])
        self.assertEqual(devices[0].reads, 4)
        self.assertTrue(devices[0].closed)

    def test_raw_out_of_range_coordinates_are_visible_but_drawing_is_bounded(self):
        module, frames, _ = fake_runtime([(999, -20, 1)], width=640, height=480)
        execute(module)
        self.assertIn("COORDINATE_RANGE_ERROR", frames[0][0])
        self.assertIn("X=999 Y=-20 pressed=1", frames[0][0])
        self.assertEqual(frames[0][1], [(634, 5)])

    def test_missing_api_is_skip_and_not_hardware_pass(self):
        module, frames, devices = fake_runtime([None, None])
        module.touchscreen = types.SimpleNamespace()
        output = execute(module)
        self.assertIn("MISSING_API", output)
        self.assertIn("SKIP", output)
        self.assertIn("SKIP: MISSING_API", frames[0][0])
        self.assertEqual(devices, [])

    def test_driver_failure_is_an_error_and_closes_created_device(self):
        module, _, devices = fake_runtime([None], driver_error=True)
        with self.assertRaisesRegex(RuntimeError, "TOUCH_DRIVER_ERROR"):
            execute(module)
        self.assertTrue(devices[0].closed)

    def test_read_failure_is_not_swallowed_and_closes_device(self):
        module, _, devices = fake_runtime([(10, 150, 1)], fail_read=True)
        with self.assertRaisesRegex(RuntimeError, "read failure"):
            execute(module)
        self.assertTrue(devices[0].closed)


if __name__ == "__main__":
    unittest.main()
