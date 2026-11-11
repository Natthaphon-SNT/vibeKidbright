# KidBright μAI Plus — Python display, camera, touch and ADC0 buttons (2026-10-27)

Current development supports creating and reviewing Plus Python projects, and a
user-operated USB RNDIS / WebSocket Run/Stop adapter under local validation. The
user explicitly authorized continuing Plus on 2026-10-22. Earlier statements
that all Plus implementation must wait for classic completion are historical.
The separate classic-only 3.7.1 hotfix remains unchanged. Original μAI V831 is
a different target and is not enabled by this work.

Runtime compatibility update 2026-10-26: the current baseline is the user's
MaixPy 4.12.5 / Runtime 1.24.0 installation. Connect checks TLS/WebSocket and the
out-of-band stat channel only; it never waits for Python metadata or sends shell
commands. Run reads version metadata from an uploaded private probe file before
native maix import; missing versions stay unknown. Run checks the
required APIs after releasing the exact launcher_daemon before its GUI, and
accepts newer versions when that API/transport contract remains compatible.
Do not claim that every future version is physically certified. The supplied
image's version.py says 4.12.5, while its stale version.pyi says 4.6.0; never use
that stub for version detection. The untouched image lacks the installed runtime
version file, so do not infer Runtime 1.24.0 from the filename. See
docs/KIDBRIGHT_UAI_PLUS_RUNTIME_COMPATIBILITY_2026-10-26.md for evidence/limits.

## Evidence and version boundary

Shared I²C expansion reference added 2026-10-28: [iKB-1Z](ikb_1z.md) can be used
as an add-on while retaining this SG2002 MaixPy 4 Python target. Read its shared
protocol and the [supplied handbook](IKB1Z_BOARD_RULES.md); select Plus-specific
I²C controller/pinmux/transport from actual runtime and wiring evidence. ESP32
GPIO, ESP-IDF C and MicroPython machine.I2C are not substitutes for a verified
Plus adapter. The bundled [Python driver](drivers/README.md) implements a
borrowed-bus MaixI2CTransport and host-tested operations; physical integration
remains unverified. Explicit expansion_boards=["ikb_1z"] adds main/ikb1z.py.

- The user confirmed the actual IDE website is
  [kidbright-mai.web.app](https://kidbright-mai.web.app/) on 2026-10-22. Treat
  this as the primary deployed-web reference for this integration. A fresh
  read-only download on that date matched the previously inspected JavaScript
  byte for byte: `assets/index.428a4c98.js`, SHA-256
  `df41393716382457b9fd3d2e459235a7947653b24fcbadce913cdde5f5f41efa`.
  Its `kidbright-mai-plus` configuration selects `websocket-shell` and
  `wss://10.155.55.1:5050`. Website identity is user-confirmed; deployed source
  agreement does not identify or physically test the connected board runtime.
- Latest supplied English `ENTraining-Document-KidBright-µAI-Plus-1.pdf`, 151
  pages: page 16 states a 2.4-inch touchscreen, 640×480, and the workflow image
  buffer → draw → display. Its images show Blockly, not literal Python. The
  illustrated 240×240 canvas is an example buffer size, not the panel size.
- English pages 5–13 describe USB-OTG, the OTG power switch, RNDIS, selecting
  Plus and secure connection. These take precedence over the older supplied
  Thai manual for overlapping tutorial content. They do not prove which OS or
  runtime is actually installed on the user's board.
- Older Thai `วิธีใช้ Kidbright uAi PLUS.pdf`, 89 pages, remains useful for
  OS/MaixPy/device-Runtime distinctions and Upload/run versus Install at boot.
  Its MaixPy 4.12.5 / Runtime 1.24.0 screenshot is an example, not a detected
  device version. Do not automatically re-image a board to match that screenshot.
- Source-pinned Plus generators establish the Python calls:
  [display](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai-plus/blocks/generators_display.js),
  [image](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai-plus/blocks/generators_image.js),
  [camera](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai-plus/blocks/generators_basic.js).
- Checked against primary MaixPy 4 documentation:
  [display](https://wiki.sipeed.com/maixpy/doc/en/vision/display.html),
  [image drawing](https://wiki.sipeed.com/maixpy/doc/en/vision/image_ops.html),
  [camera](https://wiki.sipeed.com/maixpy/doc/en/vision/camera.html).
  Source agreement is not a physical device test.

## Generate an actual project

For any of the 128 MP test prompts, first call `uai_plus_test_reference` with its
exact MP-xxx ID and read `kidbright_uai_plus_test_contracts.md`. That reviewed
reference covers GPIO/PWM/I2C/UART/IMU/RTC/audio/network/model contracts as well as
the examples below. Pass the same `test_id` to project creation. MP-001–005 use
the complete `capabilities` template (including its metadata helper), MP-015 uses
`touchscreen`, and MP-021 uses `adc_buttons`. Other test IDs require the complete
custom `main_python` source in the creation call; do not create a calibration or
circle seed and ask for Keep before implementing the requested test. Additional
`write_file` proposals are permitted during project review, and Keep applies the
finished draft together. HOLD permits unchanged read-only capabilities only.
Reviewed APIs are not a hardware pass, and an absent external fixture/model is
not an unknown API. Ask for the specific missing setup, not another copy of a
bundled public API definition.

When asked to program the Plus screen, create files rather than returning only
conceptual steps or saying the API is unknown. `KidBright uAI Plus`, `mAi Plus`,
`μAI Plus` and the IDE's `miuaiplus` selection mean this SG2002 Python target.
Do not ask the board model again when it is already explicit. If the user asks
about capabilities only, explain without unexpectedly creating files.

Call `create_project_workspace` with `project_type="kidbright_uai_plus"`, a
fresh `project_name`, and optional `python_example`:

| Example | Result |
|---|---|
| `display_text_circle` (default) | Cyan Hello KidBright and a red circle on a black canvas |
| `camera` | Capture 640×480 RGB frames and show each frame |
| `camera_overlay` | Capture, draw text and circle on each frame, show buffer |
| `touchscreen` | MP-015: live touch x/y/pressed, marker, press/release counters and cleanup |
| `adc_buttons` | MP-021: ADC0, released/S1/S2 sampled for 3 seconds each; min/max/median before threshold candidates |
| `capabilities` | MP-001–005: read-only versions, API symbols, source aliases and device-node discovery; no external actuation |

The tool creates `main/main.py` and private target metadata. It returns literal
generated Python for an initial adaptation using `write_file`, with project
Keep/Undo review. No CMake, sdkconfig, FreeRTOS, app_main, idf.py or esptool belongs
in this project. Never substitute a classic ESP32 preset or V831's older API.
For new requests create a new workspace even if a classic project is open.
For explicit edits use the existing Python file with scoped consent and review.

## ADC0, S1 and S2: known Plus API for MP-021

For **KidBright mAi Plus / μAI Plus SG2002**, the source-pinned
[Plus button generator](https://github.com/KidBrightAI/kidbright_MAI_ide/blob/7b549397adf90f75d975091a787d5960001edf01/boards/kidbright-mai-plus/blocks/generators_gpio.js)
uses `from maix import adc`, creates `adc.ADC(0, adc.RES_BIT_12)` and reads
`ADC.read()` to distinguish **S1/S2 on ADC0**. This is board-specific source
evidence for the button input, not an assumption based on the chip or connector
number. Do not confuse ADC0 with external IO2–IO8 or use ESP32 ADC APIs.

The official [MaixPy ADC guide](https://wiki.sipeed.com/maixpy/doc/en/peripheral/adc.html)
also documents `from maix.peripheral import adc`, the same constructor and
`read()`. The user's supplied MaixPy 4.12.5 image contains
`maix/_maix/peripheral/adc.pyi` (862 bytes, SHA-256
`377be516c93e440d3393d9c51f853f43cfa94d3f8d648a38289bd28ca5854327`).
It declares `ADC(pin, resolution, vref=-1)`, `read() -> int`, `read_vol()` and
`RES_BIT_12 = 12`, with raw 12-bit readings from 0 through 4095.
`maix/__init__.py` re-exports peripheral modules, explaining the shorter import.
This establishes an API reference; runtime errors and actual measured bands
must still be checked on the connected board.

```python
from maix import adc

buttons = adc.ADC(0, adc.RES_BIT_12)
raw = buttons.read()
```

For MP-021 create the requested fresh project with
`project_type="kidbright_uai_plus"` and `python_example="adc_buttons"`.
The complete returned `main/main.py` shows a five-second preparation countdown
before each of RELEASE BOTH, HOLD S1 ONLY and HOLD S2 ONLY, samples each state
in a three-second monotonic-time window, and prints/displays its sample count,
min, max and median. It uses a single ADC object and does not require touch or
console input to advance through the phases. Stop cancels incomplete collection.

Display all three measured statistics before calculating candidate boundaries.
The example proposes mid-gap boundaries only if each state has enough samples
and the measured min/max intervals do not overlap. It does not assume their
numerical order or apply a classifier. Overlap, identical bands or insufficient
samples are INCONCLUSIVE; ask the tester to repeat the measurement. Confirm the
physical button states before validating thresholds with a new set of presses.
The generator's historical fixed thresholds are **not calibration for this board**
and must not be copied into MP-021. Reading `ADC0` successfully alone is not a
physical S1/S2 pass. The supplied ADC stub has no `close`/`deinit` method; do not
invent a required cleanup API. A newer runtime's optional callable `close` may
be used, otherwise release references at scope exit.

Do not claim the ADC API or S1/S2 association is unknown without reading this
section. Missing private-context sharing is not a prerequisite to using this
bundled public reference. Check runtime symbols and report MISSING_API/SKIP only
when they are actually absent; constructor/read failures are ADC errors.

## Verified API contract, not guessed function names

```python
from maix import app, display, image, time

screen = display.Display()
width, height = screen.width(), screen.height()
canvas = image.Image(width, height, image.Format.FMT_RGB888)
canvas.draw_rect(0, 0, width, height,
                 color=image.Color.from_rgb(0, 0, 0), thickness=-1)
canvas.draw_string(20, 20, "Hello KidBright", scale=2,
                   color=image.Color.from_rgb(0, 255, 255))
canvas.draw_circle(width // 2, height // 2, min(width, height) // 6,
                   color=image.Color.from_rgb(255, 0, 0), thickness=3)
while not app.need_exit():
    screen.show(canvas)
    time.sleep_ms(50)
```

Use integer coordinates (`//` or explicit `int()`), RGB color objects, and an
exit-aware loop. Query display width/height so orientation/configuration does
not silently clip a hard-coded canvas. The documented panel remains 640×480.
Do not create a new display or camera in every iteration.

```python
from maix import app, camera, display, image

screen = display.Display()
cam = camera.Camera(640, 480, image.Format.FMT_RGB888)
try:
    while not app.need_exit():
        frame = cam.read()
        # Optional overlays belong here, before show().
        screen.show(frame)
finally:
    cam.close()
```

Desktop Python cannot import the board-native `maix` runtime. A local fake or
preview validates syntax, call order and geometry only; never claim it proves
the panel, touch controller, camera or installed runtime works. English ASCII
text is the default; Thai font loading needs separate verified font support.
Touch input is separate from drawing on a touchscreen.

## Touch input: known API for MP-015

The official [MaixPy touch guide](https://wiki.sipeed.com/maixpy/doc/en/vision/touchscreen.html)
documents `from maix import touchscreen`, `touchscreen.TouchScreen()` and
`x, y, pressed = touch.read()`. Read-only inspection of the user's MaixPy 4.12.5
image also found `/usr/lib/python3.11/site-packages/maix/_maix/touchscreen.pyi`
(SHA-256 `df4b18db74f1a8ccdaccf21e3da89fabe98fa00c2701462a20c65dcf800c72da`).
It declares `TouchScreen(device="", open=True)`, `read() -> list[int]` containing
x/y/pressed, `available(timeout=0)`, `is_opened()` and `close()`. The package's
`maix/__init__.py` re-exports these native modules under `maix`. This is an API
reference, not proof that a particular board driver opens or reports correct
coordinates. Check the installed runtime and report its actual error.

For a new touch test use `create_project_workspace` with
`project_type="kidbright_uai_plus"`, `python_example="touchscreen"` and the
chosen fresh project name. Its returned `generated_files` already contains the
complete MP-015 program. A display-only default skeleton does not fulfill a
touch test; never say the touch test was implemented if its code is absent.
Do not request private-context sharing or online-search permission merely to
use this bundled public API reference.

```python
from maix import app, touchscreen, time

touch = touchscreen.TouchScreen()
try:
    while not app.need_exit():
        if touch.available(0):
            x, y, pressed = touch.read()
            print(x, y, bool(pressed))
        time.sleep_ms(20)
finally:
    touch.close()
```

For the visible test, use the complete `touchscreen` example, not just this
minimal input loop. It sizes the canvas to the display, updates raw coordinates
and a marker, counts press/release edges, and calls `close()` in `finally`.
`MISSING_API` means the required symbol is absent. A constructor/read failure is
a driver/runtime error, not evidence that the documented API is unknown. A
successfully opened device with no touch events stays `WAIT_USER`; it is not a
hardware PASS. Do not simulate events or enable multi-touch claims from this
single-point read interface. When scaling a smaller canvas, map touch coordinates
through the same scale and letterbox offset used by display; do not assume that
panel and canvas coordinates always coincide.

## Running without blocks

Python is the code format; ESP-IDF Build & Flash is not the upload path. A
MaixPy-compatible device can also run these scripts through MaixVision's code
editor after a successful device connection. Use the official
[MaixVision guide](https://wiki.sipeed.com/maixpy/doc/en/basic/maixvision.html).
Do not tell the user to install `maix` into Windows Python or replace the board
OS merely to generate a program. Stop other camera owners before testing a
camera program. Running a script is distinct from installing it as a boot app.

Plus's source-pinned IDE transport is WebSocket shell over USB RNDIS at
`wss://10.155.55.1:5050`; a shared USB VID/PID or visible ADB serial number is
not evidence that Android logcat or V831 deployment is correct for Plus.
The development IDE provides Connect Plus, Run Python and Stop Python controls.
Connect is user initiated; first use asks to trust the fixed USB device's TLS
certificate fingerprint. A changed certificate requires verification again.
Run saves files, requires completed Keep/Undo reviews, then uploads main files
to a fresh /tmp directory. Upload acknowledgement and size are checked before
requesting python3 execution. Terminal output and exit code are distinct from
physical display verification. As of 2026-10-26 a temporary standard-library
supervisor releases the exact stock Launcher daemon before its GUI and Python, then
requests its restoration after Python exits. Other active board apps block Run
instead of being stopped automatically. Stop uses a per-run file over the
WebSocket system channel; the supervisor signals only its owned Python process
group, escalating if needed and waiting for actual exit. No boot app, OS or
runtime service script is installed or replaced. This remains locally tested;
the user's unchanged-screen report is not a physical display pass.

Do not use ESP-IDF Build & Flash or ADB logcat for Plus. This adapter does not
install startup apps, modify the board OS or automatically replace system scripts.
A runtime without the documented stat/upload WebSocket contract needs its matching
runtime workflow; use MaixVision as an alternative where supported. Native adapter
fixtures do not prove that the user's installed image or physical board works.
Never invent upload success or start a board program from an AI tool.
