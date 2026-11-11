"""MP-015: real MaixPy touch coordinates; physical results need user confirmation.

API: https://wiki.sipeed.com/maixpy/doc/en/vision/touchscreen.html
The supplied MaixPy 4.12.5 image declares TouchScreen.read/available/close.
"""
import time as clock
from maix import app, display, image, time


def run():
    screen = display.Display()
    width, height = int(screen.width()), int(screen.height())
    canvas = image.Image(width, height, image.Format.FMT_RGB888)
    white = image.Color.from_rgb(255, 255, 255)
    cyan = image.Color.from_rgb(0, 255, 255)
    black = image.Color.from_rgb(0, 0, 0)

    def render(status, detail, counts, point=None):
        canvas.draw_rect(0, 0, width, height, color=black, thickness=-1)
        for row, text in enumerate(("MP-015 TOUCH", status, detail, counts)):
            canvas.draw_string(8, 8 + row * 32, text, scale=1, color=white)
        if point is not None:
            # Keep raw coordinates in the text. Clamp only the drawn marker so
            # invalid driver coordinates cannot draw outside the image.
            x = max(5, min(width - 6, point[0]))
            y = max(5, min(height - 6, point[1]))
            canvas.draw_circle(x, y, 4, color=cyan, thickness=2)
        screen.show(canvas)

    try:
        from maix import touchscreen
    except ImportError:
        touchscreen = None
    factory = getattr(touchscreen, "TouchScreen", None)
    if not callable(factory):
        print("[MP-015] MISSING_API: maix.touchscreen.TouchScreen; SKIP", flush=True)
        while not app.need_exit():
            render("SKIP: MISSING_API", "TouchScreen unavailable", "No hardware PASS")
            time.sleep_ms(50)
        return

    touch = None
    try:
        touch = factory()
        if not callable(getattr(touch, "read", None)):
            raise RuntimeError("MISSING_API: TouchScreen.read")
        opened = getattr(touch, "is_opened", None)
        if callable(opened) and not opened():
            raise RuntimeError("TOUCH_DRIVER_ERROR: TouchScreen device is not open")
        available = getattr(touch, "available", None)
        presses = releases = 0
        previous_pressed = False
        state = None
        next_report = 0.0
        last_report = None
        print(f"[MP-015] Touch device opened; display={width}x{height}. WAIT_USER: touch the screen.", flush=True)
        while not app.need_exit():
            if not callable(available) or available(0):
                raw = touch.read()
                if len(raw) != 3:
                    raise RuntimeError("TOUCH_DATA_ERROR: expected [x, y, pressed]")
                x, y, pressed = int(raw[0]), int(raw[1]), bool(raw[2])
                presses += int(pressed and not previous_pressed)
                releases += int(previous_pressed and not pressed)
                previous_pressed = pressed
                state = (x, y, pressed)
            detail, point = "Waiting for touch input", None
            status = "WAIT_USER: tap and release"
            if state is not None:
                x, y, pressed = state
                detail = f"X={x} Y={y} pressed={int(pressed)}"
                if pressed:
                    point = (x, y)
                    if not (0 <= x < width and 0 <= y < height):
                        status = "COORDINATE_RANGE_ERROR"
                now = clock.monotonic()
                if state != last_report and now >= next_report:
                    print(f"[MP-015] {detail}; presses={presses} releases={releases}", flush=True)
                    last_report, next_report = state, now + 0.1
            render(status, detail, f"Down={presses} Up={releases}", point)
            time.sleep_ms(20)
    except Exception as error:
        print(f"[MP-015] TOUCH_ERROR: {type(error).__name__}: {error}", flush=True)
        raise
    finally:
        close = getattr(touch, "close", None)
        if callable(close):
            close()
            print("[MP-015] Touch device closed.", flush=True)


if __name__ == "__main__":
    run()
