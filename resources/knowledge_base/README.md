# KidBright hardware knowledge — 3.7.1 classic hotfix

Updated 22 October 2026. Read [support_scope.md](support_scope.md) before using examples. This package contains hardware references, not new deterministic firmware presets or complete-board certification.

| Reference | Scope |
|---|---|
| [kidbright32_versions.md](kidbright32_versions.md) | Compare V1.3 and supplied V1.5 designs |
| [kidbright32_v13.md](kidbright32_v13.md) | Buffered digital IN; GPIO15 NTP LED |
| [kidbright32_v15.md](kidbright32_v15.md) | Assembly-specific analog paths; Servo15/17; OUT endpoints and connector ambiguities |
| [inex_ia_ip.md](inex_ia_ip.md) | INEX-specific sources and user-reported test scope |
| [hardware_schematics_rules.md](hardware_schematics_rules.md) | Keep classic board, Controller and minibike wiring separate |
| [led_16x8_matrix_mapping.md](led_16x8_matrix_mapping.md) | Source-specific display packing and orientation |
| [compatibility/board_catalog.txt](compatibility/board_catalog.txt) | Sourced comparison data; a listed board is not an executable preset |

For V1.3/V1.5, matrix SDA21/SCL22 and LM73/RTC/Chain SDA4/SCL5 are hardware net assignments, not mandatory ESP-IDF controller numbers. GPIO34–39 have no output/internal pulls. OUT gate/GPIO and banana drain must not be treated as the same endpoint. External power, ADC bounds and Servo timings need matching source/measurements.

The 3.7.1 release does not include µAI/µAI Plus features or an AI-board reference document. Historical snippets must be checked against the selected board and installed SDK before reuse.
