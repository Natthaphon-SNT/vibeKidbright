# iKB-1Z drivers — shared protocol, host-specific transport

Implementation added 2026-10-28. These are source implementations with host and
transport-contract tests; see the repository test report for actual SDK build
results. Neither compilation nor fixtures establish physical board behavior.
Read [the protocol reference](../ikb_1z.md) before choosing wiring or power.

## Files and supported operations

| File | Purpose |
|---|---|
| `ikb1z.h`, `ikb1z.c` | Portable C validation, codecs and operations using a caller-supplied exchange callback |
| `ikb1z_esp_idf.h`, `ikb1z_esp_idf.c` | Classic ESP32 adapter; select master or legacy for the entire application |
| `ikb1z.py` | Python codecs, IKB1Z operations and MaixI2CTransport for an existing MaixPy bus |

Both languages implement output, input/pull-up, ADC8, ADC percent, ADC10, onboard
motor M1/M2, scaled servo 0..200, stop M1/M2, and explicit reset. Initialization
does not send commands. Device address is explicitly 0x48 or 0x49; channels,
masks and units are validated before sending. Motor speed is -100..100, not RPM.

M3/M4, servo free (-1), and UART operations are not enabled. Their capability
gates report NOT_SUPPORTED / NotImplementedError and send no command. The C and
Python pure motor encoder describes masks 1..15 for protocol testing; the actual
motor operation permits masks 1..3 only. Do not treat the pure encoder as an
authorization to drive external hardware. No last-command value is feedback.

## New workspaces

For a requested expansion, `create_project_workspace` accepts
`expansion_boards=["ikb_1z"]` while keeping the selected host:

- Plus: `project_type="kidbright_uai_plus"`; installs `main/ikb1z.py`. Omit
  `ikb_i2c_driver`, `board_profile` and `reference_example`.
- Classic: explicitly set `ikb_i2c_driver="master"` or `"legacy"` to match the
  whole application. Adds a `components/ikb1z` component and registers it with
  main. Bundled deterministic recipes use master; mixing them with legacy is
  rejected before the folder picker. For a custom legacy project omit recipe
  parameters. Existing projects require reviewed write_file integration.

The scaffold installs reusable files only. Finish the requested main program
from actual host wiring, power and peripheral requirements before asking for
Keep. No default GPIO, I2C port, Plus device node or actuator command is added.

## Classic ESP-IDF integration

Create/configure the host bus once in the application's existing I2C service.
All users of that bus must share the same mutex. A per-device mutex is not
sufficient to serialize command/read pairs with other devices.

- Master: zero-initialize `ikb1z_esp_idf_t`, then call
  `ikb1z_esp_idf_attach(&device, &context, existing_bus, shared_lock, address,
  confirmed_clock_hz, timeout_ms)`. The adapter owns only its added device
  handle. `ikb1z_esp_idf_detach` removes that handle, never the host bus/mutex.
- Legacy: define `IKB1Z_USE_LEGACY_I2C` for every translation unit that uses the
  adapter and call `ikb1z_esp_idf_init_legacy` with the existing initialized port,
  shared mutex, address and timeout. The component scaffold propagates this
  define PUBLIC. The adapter never installs/deletes the legacy I2C driver.
- Select one family for the complete app; never link both IDF I2C families.
  The project timeout policy is 1..60000 ms, at FreeRTOS tick resolution. Lock
  wait, write and read share one budget. Calls use separate STOP-terminated
  write/read operations, with the mutex held across both.
- Check every returned status before using output values. Failed reads leave
  caller values unchanged. Never reuse an old measurement as a new success.

## MAI Plus integration

Verify the external bus/backend and pinmux from the actual Plus configuration.
Construct a MaixPy I2C object only after that verification, or reuse the existing
object. The driver accepts this object and does not scan buses, load kernel
modules, change pinmux or import MicroPython machine.I2C.

```python
from threading import Lock
from ikb1z import IKB1Z, MaixI2CTransport

# existing_external_bus and jumper_address come from verified app configuration.
# Reuse the application's shared bus lock if it already has other bus users.
shared_bus_lock = Lock()
transport = MaixI2CTransport(existing_external_bus, shared_bus_lock, timeout_ms=100)
extension = IKB1Z(transport, jumper_address)
# No command has been sent. The main program chooses operations explicitly.
```

The native Maix API has no verified timeout parameter. The adapter bounds lock
waiting, detects late results and releases the lock after Python exceptions;
it cannot interrupt stuck native I/O. The Plus process supervisor handles Stop.
Process termination does not prove an iKB motor stopped; no hardware watchdog
or automatic stop-on-I2C-loss is documented. Application shutdown may attempt
stop but cannot guarantee it over a broken bus. Do not use a moving load to
test forced termination.

## Verification and physical bench procedure

Run `npm run test:ikb` for strict C compilation, both IDF adapter contract
fixtures, Python behavior/concurrency tests and exhaustive C/Python codec parity.
`scripts/build-ikb1z-idf.py` separately compiles/links both driver families using
an explicitly selected installed ESP-IDF, with all outputs under target and no
flashing. Contract headers under tests are never shipped as SDK headers.

For each real host record its model/runtime, external bus/pins, jumper address,
iKB revision/firmware, supply and connected channels. Start with motors/servos
disconnected; verify ADC and digital input against known signals, then use one
unloaded motor at low speed and one mechanically safe servo range. Record the
actual write/read bytes, timeout/error behavior and stop behavior separately
for KidBright and Plus. Confirm ambiguous firmware contracts before enabling
UART/free-servo/external motors. No physical bench result is claimed here.

Sources: [INEX protocol](https://github.com/inexglobal/pxt-iKB1z/blob/main/iKB-1z.ts),
[ESP-IDF 5.5.4 I2C](https://docs.espressif.com/projects/esp-idf/en/v5.5.4/esp32/api-reference/peripherals/i2c.html),
[MaixPy I2C](https://wiki.sipeed.com/maixpy/api/maix/peripheral/i2c.html), and the
supplied handbook, with conflicts preserved in `ikb_1z.md`.
