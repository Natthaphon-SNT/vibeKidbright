# Plus ADC0 button measurement (MP-021) — 27 October 2026

The user reported that the Vibe Coder refused to create `mai_test_021` because
the retrieved generic variant document did not supply an ADC0/S1/S2 API contract.
The specific API and board mapping were available in primary references but
missing from the bundled Plus programming guide. The guide and its generic
entrypoint now route this request to the sourced ADC0 section.

## Source evidence

The captured Plus IDE source at commit
`7b549397adf90f75d975091a787d5960001edf01` implements
`maixpy3_gpio_switch` with `from maix import adc`,
`adc.ADC(0, adc.RES_BIT_12)` and `read()` to distinguish S1 and S2.
Its historical numeric thresholds are not calibration for the user's board.

Read-only inspection of the user's MaixPy 4.12.5 image found the corresponding
`maix/_maix/peripheral/adc.pyi`, including the constructor, `read() -> int`,
`RES_BIT_12` and a 0–4095 raw range. The official
[MaixPy ADC guide](https://wiki.sipeed.com/maixpy/doc/en/peripheral/adc.html)
documents the equivalent `from maix.peripheral import adc` path. The shorter
alias is re-exported by the package initializer and is used by the Plus source.
[ADC evidence](research/uai-plus-runtime-2026-10-26/adc-buttons-api-reference.json)
records hashes and the distinction between API evidence and physical validation.

## Implementation

The new factory option `python_example="adc_buttons"` creates a complete Plus
Python project with [adc_buttons.py](../samples/kidbright-uai-plus/adc_buttons.py)
as `main/main.py`. The provider schema exposes the option, and the system prompt
selects it for MP-021. The bundled guide supplies both API and board mapping so
private-context sharing is not a prerequisite to generating this public-reference
test. Existing-folder and Keep/Undo protections remain in place.

The program uses one ADC0 instance, without opening a camera, touch input or
external GPIO. It guides the tester through these three states:

1. RELEASE S1 AND S2.
2. HOLD S1 ONLY.
3. HOLD S2 ONLY.

Each state has a five-second preparation countdown followed by a three-second
monotonic-time sampling window. It displays and logs count/min/max/median before
any threshold calculation. Sample buffers are bounded and invalid raw values
are errors. A cancelled or partial test never calculates thresholds.

Candidate boundaries are midpoints of gaps between the measured min/max bands,
sorted by actual values rather than an assumed S1/S2 order. Overlapping bands,
identical readings and insufficient samples produce INCONCLUSIVE. Candidates
are not applied as a classifier; new presses and confirmation of the sampled
physical states remain necessary. Missing symbols are SKIP/MISSING_API, while
constructor/read failures remain errors with their traceback. The supplied ADC
stub declares no close/deinit API; optional close is used only if present in the
installed runtime, then Python references leave scope.

A ready-to-open project was prepared at
`src-tauri/target/recovery-projects/mai_test_021`. Its metadata identifies the
SG2002 MaixPy target and `main/main.py`. It does not overwrite a project in the
user's other folders or start hardware execution automatically.

## Validation

| Local check | Result |
|---|---|
| Rust library | 234 passed, including creation of every exposed Plus example and both provider schemas |
| Plus Python | 42 passed, including nine MP-021 tests |
| MP-021 behavior | Three measured windows, actual statistics, order-independent candidate boundaries, overlap rejection, cancellation, missing APIs, driver/read failures, invalid data and peripheral import alias tested with fixtures |
| Bundled guide/provenance | Source and resource copies synchronized; only the two reviewed knowledge hashes refreshed |

Fixtures verify software behavior. ADC0/S1/S2 values on the physical board have
not yet been measured by this change. The earlier user-confirmed Run/Stop pass
does not establish button thresholds. Existing installer artifacts were not
rebuilt as part of this change.

## User measurement follow-up — 27 October 2026

The user subsequently supplied [actual MP-021 output](research/uai-plus-runtime-2026-10-26/adc-buttons-session-2026-10-27.json).
This supersedes the earlier absence of ADC measurements, within the scope below.

| Phase label | Samples | Min | Max | Median | Seconds |
|---|---:|---:|---:|---:|---:|
| Released | 138 | 2448 | 2480 | 2464 | 3.002 |
| S1 | 139 | 0 | 13 | 0 | 3.011 |
| S2 | 139 | 362 | 402 | 386 | 3.016 |

The bands do not overlap. Their gap midpoints are 187.5 (S1/S2) and 1425
(S2/released), matching the program's candidates. They have not been applied.
WAIT_USER is the expected end of this measurement program, not an ADC error.
The phase labels still need physical confirmation, followed by fresh presses
in MP-022/023 to validate classification and debounce. Values outside measured
bands must not silently become confident button events. Keep these measurements
specific to this board/session rather than replacing shared defaults.

The hardware-results CSV records MP-021 as WAIT_USER with this evidence.
No repeated-press, simultaneous-press or Stop/menu result is inferred from this
excerpt. Version context comes from the earlier session, not this ADC log.
