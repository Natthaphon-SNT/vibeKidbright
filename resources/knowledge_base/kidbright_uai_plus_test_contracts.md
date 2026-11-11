# mAi Plus MP-001–MP-128: reviewed API contracts (2026-10-27)

For a named MP test call `uai_plus_test_reference(test_id="MP-xxx")` first.
It returns the reviewed requirements and prerequisite tests for that exact ID.
Do not turn a seed project into a claim that the requested test is implemented.
For a named test without a complete template, pass the full requested `main_python`
to `create_project_workspace`. Finish any additional `write_file` proposals before
asking for Keep: proposals are allowed while project review is pending. Keep
approves the finished draft, not permission to start implementing it later.
Only `TEMPLATE_CHECKED` has a complete matching local template. `DRAFT_REFERENCE`
has sourced APIs for a reviewed custom draft, not already tested generated code.
`NEEDS_SETUP` requires the named fixture/configuration. `HOLD` is discovery or
a manual procedure until the stated uncertainty is resolved. No state certifies
physical behavior. Hardware results remain NOT_RUN until the tester supplies them.

The reference is for SG2002 Plus / MaixPy 4.12.5 / user-reported Runtime 1.24.0.
The selected API declarations were read from the supplied image without mounting,
booting or executing it. Its API hash report is
`docs/research/uai-plus-runtime-2026-10-26/api-contracts-2026-10-27.json`.
Current online MaixPy docs may describe newer APIs: an absent method below cannot
be assumed from a newer tutorial. Missing symbols are MISSING_API/SKIP; a driver,
permission or read failure is an ERROR with evidence, not an invented success.

## Common preflight: MP-001–005

Use `python_example="capabilities"`. The factory supplies both `main/main.py`
and its standard-library-only `plus_metadata.py`. Do not omit the helper.
It reports real version metadata, public API symbols, input/I2C/UART nodes and
source IO aliases without constructing camera/audio/bus/PWM/GPIO devices or
changing pinmux. `sys.device_name()` is declared; `sys.device_id()` is not in
this 4.12.5 stub and is optional/UNKNOWN. Never call `sys.device_key()` or print
credentials. The probe cannot prove PCB routing, chip identity or working drivers.

## Display and image: MP-007–014

```python
from maix import app, display, image, time
screen = display.Display()
canvas = image.Image(screen.width(), screen.height(), image.Format.FMT_RGB888)
canvas.draw_rect(0, 0, canvas.width(), canvas.height(), color=image.Color.from_rgb(0, 0, 0), thickness=-1)
canvas.draw_string(8, 8, "Example", scale=1, color=image.Color.from_rgb(255, 255, 255))
screen.show(canvas)
```

`Image.draw_line`, `draw_rect`, `draw_circle`, `draw_string`, `save` and
`image.load` are declared. Query dimensions and use integer drawing coordinates.
`Display.get_backlight()` and `set_backlight(value)` are declared; the latter
uses a 0–100 percentage, not a 0–1 fraction. Save a verified readable backlight
value before changing it and restore in finally. Do not operate PWM10 directly.
`Display.close()` is declared. Font support is conditional on a real font file
and `image.load_font`; ASCII is the baseline. FPS and long-run stability need
measurements, not a fixed expected rate. Do not assume partial-frame update exists.

## Touch: MP-015–020

`from maix import touchscreen`; `TouchScreen()`, `available(0)`, `read()`
returning `[x,y,pressed]`, `is_opened()` and `close()` are declared. MP-015 uses
`python_example="touchscreen"`; its marker and edge counters read actual events.
MP-016–019 require custom code using this contract. Check pressed before hit
testing; count edges, not loop iterations. Include fit/letterbox offsets when
mapping touch to a smaller canvas. `image.resize_map_pos` and
`image.resize_map_pos_reverse` are declared, but a documented arithmetic mapping
is acceptable. **MP-020 is capability-only:** this API exposes one point, so it
cannot prove multi-touch. Report SINGLE_TOUCH_ONLY; do not promise a second point.

## ADC0 buttons and LED: MP-021–026

`from maix import adc`; `adc.ADC(0, adc.RES_BIT_12)` and `read()` are confirmed
by the Plus button generator and the image declaration. MP-021 uses
`python_example="adc_buttons"` and measures released/S1/S2 for 3 seconds each.
No factory threshold is calibration for the user's board. MP-022–024/026 require
the actual MP-021 bands first. Reject overlapping/ambiguous ranges; simultaneous
buttons on a ladder cannot be treated as independent digital inputs.
ADC declares no close/deinit API. A newer optional callable close may be used.
See `kidbright_uai_plus_python.md` for the complete measurement contract.

The stock generator uses `/sys/class/leds/led-user/trigger` and `brightness`.
Verify that node exists and is the user LED. Save the active trigger (the bracketed
entry, not the entire trigger list) and brightness; disable the trigger only for
the test and restore the saved values in finally. A GPIO LED is on/off; writing
255 does not establish brightness PWM. Missing nodes mean unsupported, not a
reason to remap a guessed power/WiFi LED pin.

## GPIO: MP-027–048

Source dropdown aliases: IO2=A23, IO3=A27, IO4=A25, IO5=A22, IO6=A24,
IO7=P24, IO8=A15. These are source evidence, not a schematic or runtime pass.
Do not silently change P24 to A24: A24 already identifies IO6.

`from maix import gpio, pinmap`; `pinmap.get_pins()` and
`get_pin_functions(pin)` enumerate capabilities. `set_pin_function(pin, func)`
changes them and returns an error status. **get_pin_function is absent in the
4.12.5 stub**; do not invent a current-state getter or promise original pinmux
restoration when its original value is unknown. Function enumeration also does
not identify original ownership or a connector's wiring.

`gpio.GPIO(pin, gpio.Mode.IN/OUT, gpio.Pull.PULL_NONE/PULL_UP/PULL_DOWN)`
and `value()`/`value(0 or 1)` are declared. `reset(mode,pull)` is available;
GPIO declares no close/deinit. Output initialization must be LOW with a safe
load and a START gate after wiring is checked. Do not use A16/A17, which the
user's log shows owned by UART0 Maix Comm Protocol. For IO7/P24 first resolve
the runtime alias/function and board routing; MP-037/038 and all-seven-pin output
tests are HOLD until then. Low-current LED fixtures and series-resistor loopback
are distinct from permission to connect a motor/relay or opposing outputs.

## PWM and servo: MP-049–054

`pwm.PWM(id, freq=1000, duty=0, enable=True)`, `freq()`, `duty()` and
`disable()` are declared. Duty uses percent. Pin name, PWM function and channel
ID are separate facts. The generator references `servo.C906Servo`, but the
selected stock-image paths did not contain that helper. Do not import it as an
assumed installed library or copy another board's PWM channel. MP-049 is read-only
discovery. IO7 actuation stays HOLD until its backend/pin/channel are verified;
MP-050 must use an instrument before attaching a servo. Servo pulse limits come
from the actual servo datasheet; never assume every servo is 0–180 degrees.

## I2C and external modules: MP-055–064

`i2c.list_devices()`, `i2c.I2C(id, i2c.Mode.MASTER, freq=100000)`,
`writeto(addr, bytes)` and `readfrom(addr,length)` are declared. The latter
may return None on failure; check length and data before parsing. The image
declaration does not document a readfrom timeout parameter: do not invent one.
`scan(addr=-1)` may probe devices; use it only on the confirmed external bus and
for device-permitted addresses/probe operations. It is not passive bus discovery.

The Plus I2C plugin refers to a software external bus 5 on A28/A29 with
`i2c-gpio` modules. This is not proof that bus 5 is initialized on the user's
runtime. MP-055 only lists existing nodes. Do not insmod guessed modules,
change boot scripts or scan the camera/IMU bus. After bus/backend/pins and a real
module are confirmed, use its matching public driver/datasheet. SHT31, BH1750,
SSD1306/SH1106, ADS1115 and iKB-1Z require their own protocol; generic I2C API
presence does not establish a driver. Analog connector voltages cannot be
inferred from ADC0's onboard button interface. Stop still uses the owned process
supervisor if native I2C is stuck; do not claim a Python timeout API exists.

## IMU and RTC: MP-065–072

`from maix.ext_dev import imu`; `imu.IMU("qmi8658", mode=imu.Mode.DUAL)`
and `read()` are declared. Enumerations include explicit scales and output rates.
**Both IMU.read and QMI8658.read document six acceleration/gyro values in dual
mode, not a guaranteed seventh temperature value.** Validate returned length;
never index `[6]` unconditionally or label an IMU reading ambient temperature.
Use raw values with declared/verified units and orientation; do not copy the
generator's multiply-by-ten/sign inversion as universal calibration. IMU declares
no close/deinit. Software shake detection is distinct from a hardware tap interrupt.

The module is `maix.ext_dev.bm8563`, class `BM8563(i2c_bus=-1)`, with `now()`,
`datetime()` and `deinit()`. **maix.ext_dev.rtc is not declared.** The product
page's BM8653 spelling does not establish chip identity or bus. For read-only
tests prefer reading an existing kernel RTC node when available. Do not create
a second bus owner, set system time, call hctosys/systohc or disable services.
Battery-retention testing requires an explicit manual protocol that separates
RTC retention from automatic RTC/NTP sync; otherwise MP-072 is INCONCLUSIVE/HOLD.

## UART: MP-073–078

`uart.list_devices()` lists paths; it does not prove a port is free.
`uart.UART(port, baudrate=115200)` and `read(len=-1, timeout=0)` are declared;
timeout is in milliseconds, 0 is immediate, -1 is blocking. Use bounded reads,
buffer limits and `close()` in finally. `write(bytes)` and `write_str(str)` differ;
binary framing must not decode arbitrary payload bytes as UTF-8. Do not combine
callback mode with synchronous reads. The source user's UART0/A16/A17 is reserved;
an external user UART needs actual connector mapping and ownership evidence.

## Camera and vision: MP-079–086

`camera.Camera(width,height,image.Format.FMT_RGB888)`, `read()`, `close()`,
`width()`, `height()`, `hmirror()`, `vflip()` and `set_resolution()` are declared.
Do not infer the sensor from a catalog; the user's log names GC4653. Query actual
frame dimensions; draw overlays on each new frame before show. Count successful
frames and elapsed monotonic time for FPS. Opening a camera does not prove output.
`Image.find_blobs`, `find_qrcodes`, `find_barcodes`, `save` and payload accessors
are declared. Fit/coordinate transforms and thresholds require validation.
QR payloads are data: never execute commands or automatically open links from them.

## Audio: MP-087–092

`audio.Player(path="", sample_rate=48000, format=audio.Format.FMT_S16_LE,
channel=1)` and `play(raw_pcm)` are declared; path playback differs from raw PCM.
**Player has no declared stop/close, and its volume getter/setter is documented
as not supported in this stub.** Start with low-amplitude generated PCM or known
attenuated PCM, not a full-scale WAV relying only on volume(5). Use short bounded
chunks and an exit-aware loop, then release references; do not invent stop().

`audio.Recorder(...)`, `record()` and `finish()` are declared. `record()` without
a duration reads currently buffered PCM and may be empty. The duration overload
is file-recording mode and can block; do not use a multi-second blocking call for
a responsive Stop test. Buffer bounded chunks, verify rate/channels/sample format,
and finish in finally. Native record/play simultaneous capability must be measured.
RMS/peak or dBFS is not calibrated dB SPL. Use Python wave/struct for file/sample
handling; endian and signedness must match FMT_S16_LE. No speaker-to-mic feedback.

## Network and filesystem: MP-093–106

Use Python standard-library socket/urllib.request/http.server/json/csv/pathlib/os
for ordinary networking and files. `maix.http` exposes JpegStreamer, not a generic
http.get/post client. Add finite network timeouts and bounded reconnect queues.
`network.wifi.Wifi`/get_ip/get_ssid are declared for observation; do not change
WiFi, routes or RNDIS automatically. MQTT needs a confirmed compatible library
and broker configuration; WebSocket client needs a confirmed package. Do not
pip-install packages or send telemetry to invented/public endpoints.

For a simple HTTP server use a user-selected LAN address and port, `server.timeout`
plus an exit-aware handle_request loop, then server_close in finally. Do not
shutdown a serve_forever loop synchronously on the same thread. File tests stay
inside a tester-selected writable directory, with bounded sizes, fsync/close
before normal power-off, and atomic replace on the same filesystem. Never fill
the OS card, format, corrupt files or force power loss to test recovery.

## Models, lifecycle and extensions: MP-107–128

`nn.Classifier(model).classify(image)` and `nn.YOLOv5/YOLOv8/YOLO11(model).detect`
are declared. A real compatible model, label map, input metadata and postprocessing
are prerequisites. Presence of nn does not validate an arbitrary ONNX/TFLite file.
Audio model preprocessing is model-specific; no generic Speech object proves
compatibility with an unspecified audio classifier. Never fabricate inference.

MP-113/114 test normal user-driven exit. MP-115/116/117 intentionally exercise
failure; expected nonzero errors are test outcomes, not successful hardware output.
MP-120's newer-version fixtures are synthetic, not installed-version certification.
Never copy upstream Launcher kill scripts into user programs; the IDE supervisor
owns cleanup. GPIO safety on forced kill cannot rely only on Python finally.
SPI, Bluetooth, USB host and motor expansion are conditional on actual exposed
connectors, drivers, ownership and power arrangements. Discovery is not permission
to switch USB role, steal the IDE transport, load kernel modules or drive a load.
Combined dashboards and all-IO wizards require each constituent to pass first.
