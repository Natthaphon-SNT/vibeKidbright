"""The common preflight must observe symbols without constructing peripherals."""
import contextlib
import io
import json
from pathlib import Path
import runpy
import sys
import types
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
SAMPLE=ROOT/"samples/kidbright-uai-plus/capabilities.py"


def fixture():
    calls=[]
    def forbidden(*args,**kwargs): raise AssertionError("preflight attempted device construction or actuation")
    class Canvas:
        def __init__(self,w,h,fmt): self.lines=[]
        def draw_rect(self,*args,**kwargs): self.lines=[]
        def draw_string(self,x,y,text,**kwargs): self.lines.append(text)
    class Screen:
        def width(self): return 480
        def height(self): return 640
        def show(self,canvas): calls.append("display")
        def close(self): calls.append("display_close")
    def functions(pin):
        calls.append(("pin_functions",pin))
        if pin=="P24": raise ValueError("unknown alias")
        return ["GPIO"+pin]
    maix=types.ModuleType("maix")
    maix.display=types.SimpleNamespace(Display=Screen)
    maix.image=types.SimpleNamespace(Image=Canvas,Format=types.SimpleNamespace(FMT_RGB888="rgb888"),Color=types.SimpleNamespace(from_rgb=lambda *args:args))
    maix.camera=types.SimpleNamespace(Camera=forbidden)
    maix.audio=types.SimpleNamespace(Player=forbidden,Recorder=forbidden)
    maix.gpio=types.SimpleNamespace(GPIO=forbidden)
    maix.pwm=types.SimpleNamespace(PWM=forbidden)
    maix.adc=types.SimpleNamespace(ADC=forbidden,RES_BIT_12=12)
    maix.i2c=types.SimpleNamespace(I2C=forbidden,list_devices=lambda:[5])
    maix.uart=types.SimpleNamespace(UART=forbidden,list_devices=lambda:["/dev/ttyS0"])
    maix.pinmap=types.SimpleNamespace(get_pins=lambda:["A23"],get_pin_functions=functions,set_pin_function=forbidden)
    maix.sys=types.SimpleNamespace(device_name=lambda:"fixture-board",device_key=forbidden,reboot=forbidden,poweroff=forbidden)
    maix.app=types.SimpleNamespace(need_exit=lambda:"display" in calls)
    maix.time=types.SimpleNamespace(sleep_ms=lambda ms:None)
    helper=types.ModuleType("plus_metadata")
    helper.runtime_info=lambda:{"python":"3.11.6","maixpy":"4.12.5","runtime":None}
    return maix,helper,calls


class Capabilities(unittest.TestCase):
    def test_unknown_alias_or_optional_api_does_not_block_other_readonly_results(self):
        maix,helper,calls=fixture(); output=io.StringIO()
        with patch.dict(sys.modules,{"maix":maix,"plus_metadata":helper}), patch("glob.glob",return_value=[]), patch("os.uname",return_value=("Linux","fixture","1","1","riscv64"),create=True),contextlib.redirect_stdout(output):
            runpy.run_path(str(SAMPLE),run_name="__main__")
        report=json.loads(next(line.split("[PLUS PREFLIGHT] ",1)[1] for line in output.getvalue().splitlines() if line.startswith("[PLUS PREFLIGHT] ")))
        self.assertEqual(report["metadata"]["runtime"],"UNKNOWN")
        self.assertEqual(report["device_id"]["status"],"MISSING_API")
        self.assertEqual(report["pins"]["IO7"]["functions"]["status"],"ERROR")
        self.assertEqual(report["pins"]["IO2"]["functions"]["value"],["GPIOA23"])
        self.assertFalse(report["hardware_pass"])
        self.assertEqual(report["symbol_results"]["camera.Camera"]["status"],"AVAILABLE")
        self.assertEqual(report["chip_comparison"]["rtc"]["detected"],"UNKNOWN")
        self.assertEqual(set(report["storage"]),{"/","/mnt/data","/tmp","/mnt/sdcard"})
        self.assertTrue(all(not item.get("write_test_performed",False) for item in report["storage"].values()))
        self.assertNotIn("device_key",report["symbols"])
        self.assertIn("display_close",calls)

    def test_factory_metadata_helper_remains_standard_library_only(self):
        import ast
        tree=ast.parse((ROOT/"src-tauri/src/uai_plus_probe.py").read_text(encoding="utf-8"))
        imports=[node.module for node in ast.walk(tree) if isinstance(node,ast.ImportFrom)]
        self.assertFalse(any(name and name.startswith("maix") for name in imports))
        self.assertIn("importlib.metadata",(ROOT/"src-tauri/src/uai_plus_probe.py").read_text(encoding="utf-8"))
