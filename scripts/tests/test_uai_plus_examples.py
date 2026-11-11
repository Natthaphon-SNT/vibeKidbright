"""Desktop contract tests only. Fake maix does not certify hardware behavior."""
import runpy
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def runtime(width=640, height=480, fail_show=False):
    events, frames, cameras = [], [], []

    class Image:
        def __init__(self, w, h, fmt):
            self.w, self.h, self.operations = w, h, []
            assert fmt == "rgb888"

        def width(self): return self.w
        def height(self): return self.h

        def draw_rect(self, x, y, w, h, *, color, thickness):
            assert (x, y, w, h) == (0, 0, self.w, self.h)
            assert color == (0, 0, 0) and thickness == -1
            self.operations.append(("clear",))

        def draw_string(self, x, y, text, *, scale, color):
            assert all(isinstance(value, int) for value in (x, y))
            assert 0 <= x < self.w and 0 <= y < self.h
            self.operations.append(("text", x, y, text, scale, color))

        def draw_circle(self, x, y, radius, *, color, thickness):
            assert all(isinstance(value, int) for value in (x, y, radius))
            assert radius <= x < self.w - radius and radius <= y < self.h - radius
            self.operations.append(("circle", x, y, radius, color, thickness))

    class Display:
        def width(self): return width
        def height(self): return height

        def show(self, frame):
            if fail_show: raise RuntimeError("simulated display failure")
            events.append("show")
            frames.append(frame)

    class Camera:
        def __init__(self, w, h, fmt):
            assert (w, h, fmt) == (640, 480, "rgb888")
            self.closed = False
            cameras.append(self)

        def read(self):
            events.append("read")
            return Image(640, 480, "rgb888")

        def close(self): self.closed = True

    iterations = iter([False, False, False, True])
    module = types.ModuleType("maix")
    module.image = types.SimpleNamespace(Image=Image, Format=types.SimpleNamespace(FMT_RGB888="rgb888"),
                                        Color=types.SimpleNamespace(from_rgb=lambda r, g, b: (r, g, b)))
    module.display = types.SimpleNamespace(Display=Display)
    module.camera = types.SimpleNamespace(Camera=Camera)
    module.app = types.SimpleNamespace(need_exit=lambda: next(iterations))
    module.time = types.SimpleNamespace(sleep_ms=lambda ms: events.append(("sleep", ms)))
    return module, events, frames, cameras


def execute(name, **kwargs):
    fake, events, frames, cameras = runtime(**kwargs)
    with patch.dict(sys.modules, {"maix": fake}):
        runpy.run_path(str(ROOT / "samples/kidbright-uai-plus" / f"{name}.py"), run_name="__main__")
    return events, frames, cameras


class PlusExamples(unittest.TestCase):
    def test_canvas_adapts_to_runtime_dimensions_and_draws_before_show(self):
        for w, h in [(640, 480), (480, 640), (240, 240)]:
            with self.subTest(size=(w, h)):
                events, frames, _ = execute("display_text_circle", width=w, height=h)
                self.assertEqual(len(frames), 3)
                self.assertEqual((frames[0].w, frames[0].h), (w, h))
                self.assertEqual([op[0] for op in frames[0].operations], ["clear", "text", "circle"])
                self.assertEqual(frames[0].operations[1][3], "Hello KidBright")
                self.assertTrue(all(frame is frames[0] for frame in frames))
                self.assertEqual(events.count(("sleep", 50)), 3)

    def test_camera_reads_then_shows_new_frames_and_closes(self):
        events, frames, cameras = execute("camera")
        self.assertEqual(events, ["read", "show"] * 3)
        self.assertEqual(len({id(frame) for frame in frames}), 3)
        self.assertTrue(cameras[0].closed)

    def test_overlays_are_drawn_on_each_camera_buffer(self):
        _, frames, cameras = execute("camera_overlay")
        self.assertEqual(len(frames), 3)
        for frame in frames:
            self.assertEqual([op[0] for op in frame.operations], ["text", "circle"])
        self.assertTrue(cameras[0].closed)

    def test_camera_releases_device_after_a_display_error(self):
        fake, _, _, cameras = runtime(fail_show=True)
        with patch.dict(sys.modules, {"maix": fake}), self.assertRaisesRegex(RuntimeError, "display failure"):
            runpy.run_path(str(ROOT / "samples/kidbright-uai-plus/camera_overlay.py"), run_name="__main__")
        self.assertTrue(cameras[0].closed)

    def test_all_examples_have_valid_python_syntax(self):
        for path in (ROOT / "samples/kidbright-uai-plus").glob("*.py"):
            compile(path.read_text(encoding="utf-8"), str(path), "exec")


if __name__ == "__main__":
    unittest.main()
