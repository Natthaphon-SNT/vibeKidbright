# Typed Block-to-C pilot

The converter implements a **normalized JSON subset**, defined in [the schema](block-mapping.schema.json) and Rust tagged types. It does not import arbitrary Blockly XML/JSON, execute JavaScript generators, or accept raw C expressions.

| Verified source ID | Source fields/inputs | Current constraint |
|---|---|---|
| `basic_delay` | numeric field `VALUE` in seconds | 0.02..86400; portable checked milliseconds and modern `pdMS_TO_TICKS` |
| `basic_forever` | statement input `HANDLER` | requires an unconditional blocking delay |
| `controls_repeat_ext` | value input `TIMES`, statement input `DO` | literal integer count 0..10000 |
| `math_number` | numeric field **`VALUE`** in this Desktop override | number expression used for repeat count; generic Blockly `NUM` is rejected |
| `math_arithmetic` | `fields.OP`, `values.A/B` | ADD/MINUS/MULTIPLY/DIVIDE/MODULO only; POWER is commented out in the Desktop override and rejected |
| `logic_compare` | `fields.OP`, `values.A/B` | EQ/NEQ/LT/LTE/GT/GTE over bounded numeric constants; no string comparison |
| `logic_boolean` | `fields.BOOL` | exact strings TRUE/FALSE |
| `logic_negate` | `values.BOOL` | typed Boolean expression |
| `controls_if` | `values.IF0`, `statements.DO0/ELSE` | one conditional branch and optional ELSE; no elseif mutator |

[Source evidence](block-source-evidence.json) records nine IDs and actual Block/generator assignments. The archive digest and four relevant source hashes were rechecked against the supplied ZIP without executing JavaScript. Source precedence matters: Desktop uses `math_number.VALUE`, excludes POWER and adds integer-cast MODULO. Built-in definition contracts and Desktop generator overrides are both recorded.

Only the existing confirmed `kidbright32_v13` and `kidbright32_rev31` classic ESP32 pilot profiles are accepted. The converter reuses the existing reference component/scaffold instead of creating a second board/runtime library. Statement count/depth are bounded. Delays target the configured 1000 Hz FreeRTOS tick; generator repairs and narrowed inputs are explicit. No iA/iP/microAI substitution is inferred.

Calling `kidbright_block_to_c` without program_json returns the exact schema and runtime limits from a compile-time offline asset. Passing normalized JSON returns deterministic C. Creation still uses the selected workspace workflow and edits still require `write_file` review. No converter tool can directly apply a patch, build or Flash.

`block-fixtures/repeat-delay.json` and `forever-delay.json` exercise all four IDs. Rust tests verify strict fields/IDs/boards, invalid counts, non-finite values, blocking requirements, generated syntax and reuse of the portable core. Both exported projects compiled and linked with installed ESP-IDF **5.5.4** for **esp32**. See [build evidence](block-compilation.json). This is two deterministic fixtures, not a measured AI-generated-C success rate.

The additive `arithmetic-if.json` and `nested-conditions.json` fixtures exercise the new five IDs and both branches of a blocking forever loop. Both new projects compile/link with the installed ESP-IDF **5.5.4**. [Expansion evidence](block-expansion-compilation.json) includes source/firmware/CDB hashes and confirms the current generated main C of both older fixtures is byte-identical to their earlier build evidence. Those earlier builds were not counted as new executions.

Arithmetic/conditions are constant-only; no variables or hardware expression is inferred. Numeric constants/results must be finite and within +/-1e9; division by zero fails conversion. MODULO requires integer operands within +/-1,000,000 and nonzero divisor; parentheses and explicit int32_t casts preserve the narrowed Desktop contract. Condition/expression depth and shared generation counters are bounded, alongside the existing 1000-statement limit. Repeat counts stay literal integers 0..10000. A forever loop containing a conditional requires blocking in both branches. Tests reject unsupported fields/operators, non-finite/overflow/zero-divisor cases, unsafe conditional loops and unsupported profiles. These features are unreleased source changes, not present in the preserved NSIS binary.

No timing/behavior was measured on physical boards. Scheduler equivalence, every Blockly construct, Unicode display/font/music, plugins, hardware drivers and all-board compatibility remain unverified or unsupported. Firmware compilation is insufficient to claim original IDE equivalence.


## Subsequent regression run

block-integration-regression.json records all four fixture main sources freshly recompiled and linked again with installed ESP-IDF 5.5.4, after verifying regeneration matches the built source byte-for-byte. Python Draft 2020-12 tests validate the actual schema, four nested fixtures, exact profile/version/field/size rejection and rejection of generic XML. Rust separately checks semantic bounds and mapping-version rejection. Nine source IDs, schema v1 and the two narrow profiles remain unchanged. Host C suites pass 122 + 368 assertions. No other SDK version, all-board, original-IDE equivalence or physical behavior is certified.

The separate firmware-diagnosis-20261019.json records another four actual source-object recompilations/ELF relinks and source/CDB/log/firmware hashes. Native negative regressions also reject boolean/string numeric fields, nested unknown properties and malformed profile names before project export. The helper now writes logs/results to a fresh evidence directory by default; use `--evidence-directory <new-workspace-directory>` explicitly and `--force-recompile` to touch generated main-source mtimes while preserving bytes. Earlier log/result files are retained. No Block IDs, schema/profile compatibility, model-generation claims or physical certification were added.
