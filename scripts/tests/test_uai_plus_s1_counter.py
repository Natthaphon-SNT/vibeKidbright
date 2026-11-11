"""MP-022 edge behavior; these are local fixtures, not physical button passes."""
from pathlib import Path
import runpy
import unittest

API=runpy.run_path(str(Path(__file__).resolve().parents[2]/"samples/kidbright-uai-plus/adc_s1_counter.py"))
BANDS={"released":(2448,2480),"s1":(0,13),"s2":(362,402)}


class S1Counter(unittest.TestCase):
    def state(self,c,raw,t):
        c.update(raw,t)
        return c.update(raw,t+0.04)

    def test_only_measured_bands_are_confident(self):
        self.assertEqual([API["classify"](x,BANDS) for x in [0,13,14,187,362,402,1425,2448,2480,2481]],
                         ["s1","s1","unknown","unknown","s2","s2","unknown","released","released","unknown"])

    def test_held_at_start_does_not_count_without_a_release(self):
        c=API["PressCounter"](BANDS)
        self.state(c,0,0)
        self.assertEqual(c.count,0)

    def test_twenty_presses_and_holds_count_exactly_twenty(self):
        c=API["PressCounter"](BANDS)
        for i in range(20):
            self.state(c,2464,i*2)
            self.state(c,0,i*2+0.2)
            for j in range(10):c.update(0,i*2+0.3+j*0.1)
        self.assertEqual(c.count,20)

    def test_ambiguous_samples_do_not_rearm_or_fake_release(self):
        c=API["PressCounter"](BANDS)
        self.state(c,2464,0);self.state(c,0,1)
        self.state(c,200,2);self.state(c,0,3)
        self.assertEqual(c.count,1)
        self.assertFalse(c.armed)

    def test_bounce_and_s2_do_not_create_s1_events(self):
        c=API["PressCounter"](BANDS)
        self.state(c,2464,0)
        c.update(0,1);c.update(2464,1.01)
        self.assertEqual(c.count,0)
        self.state(c,386,2);self.state(c,0,3)
        self.assertEqual(c.count,0)
        self.state(c,2464,4);self.state(c,0,5)
        self.assertEqual(c.count,1)

    def test_invalid_or_overlapping_calibration_is_rejected(self):
        for bands in [{}, {**BANDS,"s2":(13,402)}, {**BANDS,"s1":(-1,13)}]:
            with self.assertRaises(ValueError):API["PressCounter"](bands)
        with self.assertRaises(ValueError):API["classify"](4096,BANDS)
