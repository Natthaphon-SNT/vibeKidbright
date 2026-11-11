"""MP-021: measure Plus ADC0 button bands before suggesting thresholds.

Uses the Plus button generator's ADC(0, RES_BIT_12) / read() contract.
No historical S1/S2 thresholds or external analog connector map is assumed.
Run manually from the IDE; follow the screen's RELEASE/HOLD countdowns.
"""
import json
import math
import statistics
import time as clock
from maix import app, display, image, time

PREPARE_SECONDS = 5.0
SAMPLE_SECONDS = 3.0
SAMPLE_INTERVAL_MS = 20
MIN_SAMPLES = 20
MAX_SAMPLES = 1000
PHASES = (("released", "RELEASE S1 AND S2"), ("s1", "HOLD S1 ONLY"), ("s2", "HOLD S2 ONLY"))


def summarize(phase, samples, elapsed):
    if not samples:
        raise RuntimeError(f"ADC_DATA_ERROR: no samples for {phase}")
    return {
        "phase": phase, "count": len(samples), "min": min(samples),
        "max": max(samples), "median": statistics.median(samples),
        "elapsed_s": round(elapsed, 3),
    }


def propose_thresholds(results):
    """Candidates only. Overlapping/incomplete measured bands are inconclusive."""
    if len(results) != len(PHASES) or {r["phase"] for r in results} != {p[0] for p in PHASES}:
        return {"status": "INCONCLUSIVE", "reason": "incomplete phases", "candidates": []}
    if any(r["count"] < MIN_SAMPLES for r in results):
        return {"status": "INCONCLUSIVE", "reason": "too few samples", "candidates": []}
    ordered = sorted(results, key=lambda result: result["min"])
    pairs = list(zip(ordered, ordered[1:]))
    if any(left["max"] >= right["min"] for left, right in pairs):
        return {"status": "INCONCLUSIVE", "reason": "measured bands overlap", "candidates": []}
    return {
        "status": "WAIT_USER", "reason": "validate candidates with new presses; not applied",
        "candidates": [
            {"between": [left["phase"], right["phase"]],
             "raw_threshold": (left["max"] + right["min"]) / 2,
             "gap_raw": right["min"] - left["max"]}
            for left, right in pairs
        ],
    }


def run():
    screen = display.Display()
    width, height = int(screen.width()), int(screen.height())
    canvas = image.Image(width, height, image.Format.FMT_RGB888)
    white, black = image.Color.from_rgb(255, 255, 255), image.Color.from_rgb(0, 0, 0)

    def render(lines):
        canvas.draw_rect(0, 0, width, height, color=black, thickness=-1)
        for row, line in enumerate(lines):
            canvas.draw_string(8, 8 + row * 26, str(line), scale=1, color=white)
        screen.show(canvas)

    def hold(lines, seconds=None):
        deadline = None if seconds is None else clock.monotonic() + seconds
        while not app.need_exit():
            if deadline is not None and clock.monotonic() >= deadline:
                return True
            render(lines)
            time.sleep_ms(50)
        return False

    try:
        from maix import adc
    except ImportError:
        try:
            from maix.peripheral import adc
        except ImportError:
            adc = None
    factory = getattr(adc, "ADC", None)
    resolution = getattr(adc, "RES_BIT_12", None)
    if not callable(factory) or resolution is None:
        print("[MP-021] MISSING_API: ADC / RES_BIT_12; SKIP", flush=True)
        hold(["MP-021 ADC0", "SKIP: MISSING_API", "ADC / RES_BIT_12 unavailable", "No hardware PASS"])
        return

    device = reader = None
    results = []
    try:
        device = factory(0, resolution)
        reader = getattr(device, "read", None)
        if not callable(reader):
            print("[MP-021] MISSING_API: ADC.read; SKIP", flush=True)
            hold(["MP-021 ADC0", "SKIP: MISSING_API", "ADC.read unavailable", "No hardware PASS"])
            return
        print("[MP-021] ADC0 opened at 12 bits. Follow RELEASE / HOLD instructions. No thresholds applied.", flush=True)
        for phase, instruction in PHASES:
            if app.need_exit():
                print("[MP-021] CANCELLED: no thresholds calculated.", flush=True)
                return
            print(f"[MP-021] PREPARE {phase}: {instruction}; sampling starts in {PREPARE_SECONDS:g}s.", flush=True)
            deadline = clock.monotonic() + PREPARE_SECONDS
            while not app.need_exit() and clock.monotonic() < deadline:
                remaining = max(1, math.ceil(deadline - clock.monotonic()))
                render(["MP-021 ADC0", instruction, f"Prepare: {remaining}s", "Keep this state for 3s", "Stop cancels this test"])
                time.sleep_ms(50)
            if app.need_exit():
                print("[MP-021] CANCELLED: no thresholds calculated.", flush=True)
                return
            samples = []
            started = clock.monotonic()
            deadline, next_render = started + SAMPLE_SECONDS, started
            print(f"[MP-021] SAMPLE_START {phase}; window={SAMPLE_SECONDS:g}s.", flush=True)
            while not app.need_exit() and clock.monotonic() < deadline:
                raw = reader()
                if isinstance(raw, bool) or not isinstance(raw, int) or not 0 <= raw <= 4095:
                    raise RuntimeError(f"ADC_DATA_ERROR: expected 12-bit integer, received {raw!r}")
                if len(samples) >= MAX_SAMPLES:
                    raise RuntimeError("ADC_DATA_ERROR: sample buffer limit reached")
                samples.append(raw)
                now = clock.monotonic()
                if now >= next_render:
                    render(["MP-021 ADC0", instruction, "SAMPLING - KEEP STILL",
                            f"Remaining: {max(0.0, deadline - now):.1f}s", f"Raw={raw} n={len(samples)}"])
                    next_render = now + 0.1
                time.sleep_ms(SAMPLE_INTERVAL_MS)
            if app.need_exit():
                print("[MP-021] CANCELLED: partial samples discarded; no thresholds calculated.", flush=True)
                return
            result = summarize(phase, samples, clock.monotonic() - started)
            results.append(result)
            print("[MP-021] STATS " + json.dumps(result), flush=True)
            if not hold(["MP-021 ADC0", f"{phase.upper()} STATS", f"n={result['count']} min={result['min']}",
                         f"max={result['max']} median={result['median']}", "No threshold applied"], 2.0):
                print("[MP-021] CANCELLED: no thresholds calculated.", flush=True)
                return

        lines = ["MP-021 MEASURED STATS"]
        for result in results:
            lines.extend([f"{result['phase'].upper()} n={result['count']} min={result['min']}",
                          f"max={result['max']} med={result['median']}"])
        # Present all measured statistics before even proposing any boundaries.
        render(lines + ["Thresholds not applied"])
        print("[MP-021] STATS_COMPLETE " + json.dumps(results), flush=True)
        proposal = propose_thresholds(results)
        print("[MP-021] THRESHOLD_CANDIDATES " + json.dumps(proposal), flush=True)
        print("[MP-021] WAIT_USER: confirm button states and validate with new presses. No hardware PASS.", flush=True)
        footer = "OVERLAP / RETEST" if proposal["status"] == "INCONCLUSIVE" else "Candidates in Terminal"
        hold(lines + [footer, "No threshold applied"])
    except Exception as error:
        print(f"[MP-021] ADC_ERROR: {type(error).__name__}: {error}", flush=True)
        raise
    finally:
        # The supplied ADC stub has no close/deinit. Use close only if a newer
        # installed runtime actually supplies it, then release Python references.
        close = getattr(device, "close", None)
        try:
            if callable(close):
                close()
        finally:
            reader = device = None


if __name__ == "__main__":
    run()
