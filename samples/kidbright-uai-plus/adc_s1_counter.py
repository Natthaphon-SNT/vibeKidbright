"""MP-022: count debounced S1 presses using this project's measured bands.

Requires main/button_calibration.py from an actual MP-021 run. No shared ADC
thresholds are built in. UNKNOWN never rearms the counter or generates edges.
"""
import json
import time as clock


def validate_bands(bands):
    if set(bands) != {"released", "s1", "s2"}:
        raise ValueError("Need released, s1 and s2 measured bands")
    for bounds in bands.values():
        if len(bounds) != 2 or any(type(x) is not int for x in bounds) or not 0 <= bounds[0] <= bounds[1] <= 4095:
            raise ValueError("Invalid measured 12-bit ADC band")
    ordered = sorted(bands.values())
    if any(left[1] >= right[0] for left, right in zip(ordered, ordered[1:])):
        raise ValueError("Measured ADC bands overlap; repeat MP-021")


def classify(raw, bands):
    if type(raw) is not int or not 0 <= raw <= 4095:
        raise ValueError("Invalid raw ADC sample")
    return next((name for name, (low, high) in bands.items() if low <= raw <= high), "unknown")


class PressCounter:
    def __init__(self, bands, debounce_s=0.03):
        validate_bands(bands)
        if debounce_s <= 0:
            raise ValueError("debounce_s must be positive")
        self.bands, self.debounce_s = bands, debounce_s
        self.candidate, self.since, self.stable = "unknown", None, "unknown"
        self.count, self.armed = 0, False

    def update(self, raw, now):
        state = classify(raw, self.bands)
        if state == "unknown":
            self.candidate, self.since = "unknown", None
            return state, None
        if state != self.candidate:
            self.candidate, self.since = state, now
        if self.since is None or now - self.since < self.debounce_s or state == self.stable:
            return state, None
        self.stable = state
        event = None
        if state == "released":
            self.armed = True
            event = "RELEASED"
        elif state == "s1":
            if self.armed:
                self.count += 1
                event = "S1_DOWN"
            self.armed = False
        else:
            self.armed = False
            event = "S2_IGNORED"
        return state, event


def run():
    from button_calibration import BANDS, SOURCE
    from maix import app, display, image, time
    try:
        from maix import adc
    except ImportError:
        from maix.peripheral import adc
    counter = PressCounter(BANDS)
    print("[MP-022] CALIBRATION " + json.dumps({"bands": BANDS, "source": SOURCE}), flush=True)
    print("[MP-022] Release both first, then press S1 twenty times. Hold must not add counts. UNKNOWN samples do not create events. No automatic hardware PASS.", flush=True)
    screen = None
    try:
        screen = display.Display()
        width, height = int(screen.width()), int(screen.height())
        canvas = image.Image(width, height, image.Format.FMT_RGB888)
        white, black = image.Color.from_rgb(255, 255, 255), image.Color.from_rgb(0, 0, 0)
        device = adc.ADC(0, adc.RES_BIT_12)
        next_render, next_status = 0.0, 0.0
        while not app.need_exit():
            raw, now = device.read(), clock.monotonic()
            state, event = counter.update(raw, now)
            status = {"raw": raw, "state": state, "stable": counter.stable, "count": counter.count, "armed": counter.armed}
            if event is not None or now >= next_status:
                print("[MP-022] " + json.dumps({**status, "event": event}), flush=True)
                next_status = now + 1.0
            if now >= next_render:
                canvas.draw_rect(0, 0, width, height, color=black, thickness=-1)
                lines = ["MP-022 S1 PRESS COUNTER", f"Raw: {raw}  state: {state}", f"Stable: {counter.stable}",
                         f"S1 count: {counter.count} / 20", f"Armed: {counter.armed}", "Release both between presses",
                         "Hold S1: count stays fixed", "UNKNOWN: no event / no rearm", "Compare count with real presses", "Stop returns to board menu"]
                for row, line in enumerate(lines):
                    canvas.draw_string(8, 8 + row * 26, line, scale=1, color=white)
                screen.show(canvas)
                next_render = now + 0.1
            time.sleep_ms(10)
    finally:
        if screen is not None:
            screen.close()
        print("[MP-022] Counter stopped; tester must confirm actual presses and menu.", flush=True)


if __name__ == "__main__":
    run()
