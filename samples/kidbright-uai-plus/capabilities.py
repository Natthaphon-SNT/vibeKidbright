"""Read-only MP-001..005 preflight. The factory includes plus_metadata.py.

Lists symbols and nodes; never creates camera/audio/GPIO/I2C/UART/PWM devices,
changes pinmux, imports optional packages or prints device keys/credentials.
"""
import glob
import importlib.util
import json
import os
from pathlib import Path
import time as clock
from plus_metadata import runtime_info

METADATA = {key: (value if value is not None else "UNKNOWN") for key, value in runtime_info().items()}
import maix
from maix import app, display, image, time

PINS = {"IO2": "A23", "IO3": "A27", "IO4": "A25", "IO5": "A22", "IO6": "A24", "IO7": "P24", "IO8": "A15"}
SYMBOLS = ["display.Display", "image.Image", "touchscreen.TouchScreen", "camera.Camera",
           "audio.Player", "audio.Recorder", "gpio.GPIO", "pinmap.get_pins", "pinmap.get_pin_functions",
           "pwm.PWM", "adc.ADC", "adc.RES_BIT_12", "i2c.I2C", "i2c.list_devices", "uart.UART",
           "uart.list_devices", "ext_dev.imu.IMU", "ext_dev.bm8563.BM8563", "nn.Classifier", "nn.YOLOv5"]


def symbol(path):
    value = maix
    for part in path.split("."):
        value = getattr(value, part, None)
        if value is None: break
    return value


def read_text(path):
    try:
        with open(path, encoding="utf-8") as stream:
            return stream.read(4096)
    except (OSError, UnicodeError):
        return None


def query(path, *args):
    try:
        function = symbol(path)
        if not callable(function): return {"status": "MISSING_API", "value": "UNKNOWN"}
        value = function(*args)
        return {"status": "READ", "value": value[:64] if isinstance(value, list) else value}
    except Exception as error:
        return {"status": "ERROR", "type": type(error).__name__, "detail": str(error)[:180]}


def inspect_symbol(path):
    try:
        return {"status": "AVAILABLE" if symbol(path) is not None else "MISSING_API"}
    except Exception as error:
        return {"status": "ERROR", "type": type(error).__name__, "detail": str(error)[:180]}


def storage_info():
    result = {}
    for path in ("/", "/mnt/data", "/tmp", "/mnt/sdcard"):
        try:
            value = os.statvfs(path)
            result[path] = {"status": "READ", "total_bytes": value.f_blocks * value.f_frsize,
                            "available_bytes": value.f_bavail * value.f_frsize,
                            "write_access_check": os.access(path, os.W_OK), "write_test_performed": False}
        except (OSError, AttributeError) as error:
            result[path] = {"status": "UNKNOWN", "type": type(error).__name__}
    return result


def collect():
    nodes = {pattern: sorted(glob.glob(pattern))[:32] for pattern in
             ("/dev/input/event*", "/dev/i2c-*", "/dev/ttyS*", "/sys/class/leds/*")}
    input_names = {node: read_text("/sys/class/input/" + Path(node).name + "/device/name")
                   for node in nodes["/dev/input/event*"]}
    pins = {}
    for name, pin in PINS.items():
        pins[name] = {"source_alias": pin, "functions": query("pinmap.get_pin_functions", pin),
                      "physical_connector_verified": False}
    packages = {}
    for package in ("paho", "websocket", "flask"):
        try: packages[package] = "FOUND_SPEC" if importlib.util.find_spec(package) else "NOT_FOUND"
        except (ImportError, ValueError): packages[package] = "NOT_FOUND"
    sensor_cfg = read_text("/mnt/data/sensor_cfg.ini")
    sensor_lines = [] if sensor_cfg is None else [line.strip() for line in sensor_cfg.splitlines()
        if line.strip() and not line.lstrip().startswith(("#", ";"))][:12]
    symbols = {path: inspect_symbol(path) for path in SYMBOLS}
    return {
        "metadata": METADATA, "os": list(os.uname()),
        "device_name": query("sys.device_name"), "device_id": query("sys.device_id"),
        "symbols": {path: item["status"] == "AVAILABLE" for path, item in symbols.items()},
        "symbol_results": symbols,
        "pins": pins, "all_pin_names": query("pinmap.get_pins"),
        "i2c_device_ids": query("i2c.list_devices"), "uart_device_paths": query("uart.list_devices"),
        "nodes": nodes, "input_names": input_names, "optional_packages": packages,
        "storage": storage_info(),
        "sensor_cfg_selected_lines": sensor_lines, "imu_chip_identity": "UNKNOWN", "rtc_chip_identity": "UNKNOWN",
        "chip_comparison": {
            "camera": {"documented_catalog": "OV2685", "detected": sensor_lines or "UNKNOWN",
                       "basis": "sensor_cfg only; frame/sensor operation untested"},
            "imu": {"documented_catalog": "QMI8658", "detected": "UNKNOWN"},
            "rtc": {"documented_catalog": "BM8653 (catalog spelling)", "detected": "UNKNOWN"}
        },
        "hardware_pass": False, "scope": "Read-only discovery; no bus opened or connector driven. API presence is not driver, wiring or physical validation."
    }


def run():
    report = collect()
    print("[PLUS PREFLIGHT] " + json.dumps(report, ensure_ascii=True), flush=True)
    screen = display.Display()
    width, height = int(screen.width()), int(screen.height())
    canvas = image.Image(width, height, image.Format.FMT_RGB888)
    white, black = image.Color.from_rgb(255, 255, 255), image.Color.from_rgb(0, 0, 0)
    pages = [
        ["PLUS READ-ONLY PREFLIGHT", "Python: " + str(METADATA.get("python")), "MaixPy: " + str(METADATA.get("maixpy")),
         "Runtime: " + str(METADATA.get("runtime")), "OS: " + report["os"][0] + " " + report["os"][2],
         "Name: " + str(report["device_name"].get("value", "UNKNOWN")),
         "ID: " + str(report["device_id"].get("value", "UNKNOWN")), "Full report: Terminal", "No hardware PASS"],
        ["IO ALIASES (NOT WIRING)"] + [name + "=" + pin + " " + report["pins"][name]["functions"]["status"] for name, pin in PINS.items()],
        ["DISCOVERY ONLY", "No pinmux changes", "No camera/audio opened", "No I2C/UART/PWM opened", "Chip identities UNKNOWN", "Stop returns to menu"]
    ]
    try:
        while not app.need_exit():
            canvas.draw_rect(0, 0, width, height, color=black, thickness=-1)
            page = pages[int(clock.monotonic() // 4) % len(pages)]
            for row, line in enumerate(page):
                canvas.draw_string(8, 8 + row * 26, line[:max(8, (width - 16) // 8)], scale=1, color=white)
            screen.show(canvas)
            time.sleep_ms(50)
    finally:
        screen.close()


if __name__ == "__main__":
    run()
