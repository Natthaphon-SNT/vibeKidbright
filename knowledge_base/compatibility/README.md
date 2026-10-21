# KidBright compatibility layer — software expansion 2026-10-17

This is a behavioral compatibility layer for ESP-IDF 5.5 on ESP32, not a claim
that every KidBright block or board is supported.

## Required AI workflow

- Call `kidbright_reference` when asked for behavior matching KidBright or a reference program.
- Confirm the board model and task-specific hardware evidence. Exact revision is
  needed only when an unresolved revision difference affects the task. The pilot
  presets support `kidbright32_v13` and `kidbright32_rev31` only; that does not
  prohibit a custom sourced draft for a named iP. Never infer a board from a
  project's old name or substitute a preset for a different model.
- For a NEW reference project, call
  `create_project_workspace` with both `board_profile` and `reference_example`.
  This installs deterministic C files, a portable core and project metadata.
  Read the generated files before adapting user-specific logic. Do not rewrite
  their pins, driver family, timing or protocol without a stated reason.
- All created files have a project-level Keep / Undo card. Explain the result
  and let the user choose. Do not run commands, install libraries, build or flash
  until the user chooses Keep; do not resolve the review on their behalf.
- Edit requests stay in the current project. Inspect `.vibekidbright/project.json`
  and `main/kb_board.h` when present. Older projects may use root `kidbright-project.json`.
  Cancellation never authorizes writing a
  new program into the previous project.
- Keep reference rules in the application's shared knowledge base; do not copy
  `KIDBRIGHT_COMPATIBILITY.md` into generated learner projects. Private metadata
  is managed by workspace tools and hidden from the project tree and review list.
  Do not add explanatory Markdown or duplicate profile JSON to the visible project
  unless the user explicitly requests a document as part of their project.
- Use `esp_idf_info` to check SDK context. Without private-context permission it
  returns a public reference contract only, not the actual SDK. These examples
  target SDK 5.5; a custom draft may target that documented API pending a later
  configured build. Do not demand private sharing merely to generate new code,
  claim an uninspected SDK version or downgrade the installed SDK.
- Read `compatibility/kb_core.h` and `.c` for numerical or legacy packet helpers.
  No Arduino C++ classes or Device/Driver objects are needed by this component.
- Use the board catalog returned by `kidbright_reference` to compare revisions.
  A comparison entry is not permission to use another executable profile.
  The current phase is classic KidBright. μAI and μAI Plus are future work only.
- The learner sidebar shows main program files. UI file actions maintain a
  managed source list in CMake. AI-created source files propose that CMake
  change for review; preserve existing dependencies and accept the diff before
  build. Custom/dynamic CMake requires an explicit reviewed source-list change.

## Board and protocol scope

| Profile | S1 | Passive buzzer | LM73 | Scope |
|---|---|---|---|---|
| kidbright32_v13 | GPIO16 active low | GPIO13 | SDA4/SCL5, 0x4D | S1/buzzer/LM73, music; matrix on SDA21/SCL22 0x70 with software checks only |
| kidbright32_rev31 | GPIO16 active low | GPIO13 | SDA4/SCL5, 0x4D | Existing narrow S1/buzzer/LM73 profile and music; matrix remains blocked due to revision identity gap |

These constants are based on the attached KidBright library and the existing
LM73 reference. Confirm the physical revision and sensor address. Other
revisions, µAI, motor extensions and external wiring need their own profile.
The v1.3 schematic is https://www.kid-bright.org/kidbright/download/1940/ .
The provided v1.5 PDF title block says Rev3 despite its Rev3_1 filename; keep
that identity conflict visible instead of promoting its pinout to every revision.
ADC joystick (GPIO34/35), Formula Kid RC timing and minibike BLDC are different
hardware contracts; never combine them merely because they all use ESP32.

The legacy ESP-NOW plugin sends a numeric value as ASCII `%f`, with six decimal
places and WITHOUT the terminating NUL. `kb_ascii_format` returns the wire byte
count. `kb_ascii_parse` accepts at most 64 bytes and rejects invalid, embedded-NUL,
NaN/Inf and overflowing numbers. A newer raw `int32_t` protocol is a SEPARATE
contract and may be used only when both peers agree. These pilot examples do
not initialize radio or prove interoperability.

Use one I2C driver family throughout a firmware application. The temperature
recipe uses `driver/i2c_master.h`; an older legacy recipe may use `driver/i2c.h`,
but the two families must not be mixed. Use SDK-specific ESP-NOW callback types:
SDK 5.5 send-info / receive-info pointers differ from old MAC-pointer examples.

## Behaviors and explicit differences

- `button_buzzer`: wait for S1, play A4 (440 Hz) for one quarter note at 120 BPM
  (500 ms), silence the buzzer, then wait for release. Polling is 40 ms.
- `temperature_serial`: LM73 pointer 0x00, MSB-first signed raw /128 degrees C,
  sample every 500 ms. I2C errors are logged as errors, never as 0 degrees.
- `music_scale`: wait S1, play C4 through C5 at 120 BPM, then wait release.
  MIDI frequency conversion rounds to integer Hz; quarter-note beats determine
  durations. This is a new explicit API, not the original SOUND class lifecycle.
- `matrix_counter`: v1.3 only, display 00..99 with a new 5x7 numeric font every
  500 ms. `kb_basic` offers bounded pixel/frame/shift operations and HT16K33
  packet packing. `kb_display_idf` takes a caller-owned new-driver I2C bus,
  reports transfer errors, and releases its device on init failure. Physical
  orientation, original fonts, Unicode text and asynchronous scrolling remain
  unverified or unimplemented; never describe the numeric demo as full display parity.
- Both `basic_delay` and `ms_delay` take SECONDS in the original generators.
  `0.02` is 20 ms; `0.05` is 50 ms. `kb_delay_ms` preserves truncation to whole
  milliseconds for nonnegative finite input and rejects invalid/overflow input.
- PID preserves the time-based equation, integral ±60 and output ±100. Explicit
  repair: the first sample is proportional-only (I/D zero), invalid/nonpositive
  dt is rejected without modifying state. Double state is used, not bit-exact
  float arithmetic. Keep a separate initialized `kb_pid_t` for every PID ID.
- SMA preserves warm-up averaging and a rolling window. Explicit repairs:
  windows outside 1..50 are rejected; changing a window resets only that state.
  Keep one zero-initialized `kb_sma_t` per filter ID.
- Joystick keeps descending raw endpoints, float normalization, integer
  truncation and STRICT dead-zone inequalities inside normalized center ±500.
  Calibration endpoints must differ; raw/release must be within 0..4095.
- Codec formatting assumes the standard C numeric locale, as on ESP-IDF.

## Evidence and limitations

Source contracts: KidBright Desktop IDE 1.7.0 in the attached archive,
`app/public/blockly/kidbright/generators.js`, plugins
`pid_control/{pid_con,filter,mjoystick}` and `esp_now/esp_now_class`, plus the
Simulator's A4/120 BPM note table and existing LM73 recipe.

The portable core has executable host tests. The generated examples are checked
with the installed ESP-IDF; record the actual result rather than claiming a
physical pass from compilation. Button latency, sound waveform and LM73 output
still need a physical board comparison. Full original matrix text/scroll, encoder, MPU,
motors, line following, full Blockly import and µAI deployment are not provided
by this pilot. Do not silently replace those missing parts with dummy functions.
