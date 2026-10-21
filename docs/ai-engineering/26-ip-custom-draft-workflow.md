# KidBright32iP custom draft workflow — 2026-10-19

## Reported failure

The user asked for a new ESP-IDF project on iP to scroll A–Z across the onboard matrix. Vibe Coder demanded a PCB revision and unrelated sensor identification, then required private-context sharing because `esp_idf_info` was blocked. The board had already been identified. A missing deterministic recipe was incorrectly treated as a prohibition on custom firmware.

## Changes

- Replaced the blanket revision question with task-scoped board confirmation in the system prompt, support contract, compatibility workflow and routing hint. An explicit iP / KidBright32iP establishes the model. An unknown revision stays unknown; the published source design is an explicit draft assumption, not a claimed match to the user's PCB.
- Added a dedicated iP display reference section. The [manufacturer schematic](https://doc.inex.co.th/wp-content/uploads/2023/02/020966_0636_KidBrigh3.png), Rev.2021.04.01, was downloaded and visually inspected: HT16K33, 16×8 display, SDA0/SCL0 connected to ESP32 GPIO21/22. This supports a source-grounded custom draft; it does not establish physical orientation, the user's board revision or an executed firmware test. Gyro/accelerometer, RTC, temperature and S2 questions are irrelevant to this LED-only request.
- Kept the two deterministic profiles and nine Block IDs unchanged. For custom creation, omit both preset parameters, create a fresh workspace and propose the firmware through the existing Keep/Undo flow. No fake iP preset or hardware-certification status was added.
- `esp_idf_info` now returns a literal bundled SDK reference contract when private sharing is off. It explicitly reports `installed_sdk_inspected=false`, references ESP32 / SDK 5.5 for drafting, and performs no installation/config/path/header reads or subprocess calls. This is not an installed-version result. The consent-enabled installed-path behavior is preserved.
- Fresh custom creation returns the same in-memory starter text used to initialize its four files. No later filesystem content is read for that result. The initial custom proposal can therefore use known public starter contents without enabling private sharing. Existing-file inspection, SDK builds, dependency installation and other private tools stay blocked without consent. Edits and builds retain all existing review gates.
- Both provider schemas explain the custom workflow, optional preset parameters and public SDK fallback. Bundled copies and the three changed public provenance pins were synchronized deliberately.

## Verification

Commands, outcomes and source hashes are in `ip-workflow-verification-20261019.json` and `ip-workflow-changes-20261019.json`.

Three new Rust regressions cover the actual consent guard used by dispatch, no path leakage from public SDK context, exact public starter contents, exclusion of later private edits from the starter result, fresh custom creation without fake preset substitution and matching optional provider contracts. Existing Keep/Undo, file-safety and repair tests remain in the Rust/frontend suites.

Automated checks cannot establish that a real model will follow these instructions. No authorized inference endpoint is available, so the exact real-provider conversation remains NOT_EXECUTED. No firmware was generated, built, flashed or physically tested in this fix. No new installer, commit, push or publication is authorized/performed; historical NSIS 3.7.3 remains immutable. Any local no-bundle executable build is development output, not installed-release qualification.

The three AI capability levels and eight original requirement statuses remain incomplete. The bundled MiniLM model, production ranking and frozen evaluation set are unchanged; this is a targeted workflow/rule correction, not a measured retrieval-quality improvement.
