# Board preparation guide — 2026-10-18

The reported top button only toggled a short checklist in a lower sidebar. It did not move focus or bring that checklist into view, making a successful click appear to do nothing. Both entry points now open the same native browser modal dialog immediately, above the app content.

## Implemented behavior

- A bundled offline Thai/English reader with five preparation steps and troubleshooting for each of KidBright32, original V831 μAI and SG2002 μAI Plus.
- Correct separation of KidBright32 serial/ESP-IDF, original μAI ADB/WebUSB reference and Plus USB-OTG/RNDIS reference. Content is based on the existing hardware knowledge and supplied-manual audit, not new hardware measurements.
- Scrollable content, visible close/back controls, native Escape/backdrop dismissal, modal focus containment and return to the opener. KidBright32's editor action closes the reader and uses the existing actual editor callback.
- No network, provider, connection, driver, provisioning or source-write operation is triggered by reading the guide. Lesson samples, class names and active stage are preserved.
- Source: `src/BoardPreparationGuide.tsx`, `src/LearningWorkspace.tsx`, `src/LearningWorkspace.css`; regressions: `src/test/learningWorkspace.test.tsx`; real browser probe: `scripts/probe-board-guide.mjs`, called by `scripts/check-production-workflows.mjs`.

## Verification and limits

The frontend suite passes **176 tests / 18 files**, including six new guide cases. TypeScript, production frontend, release-policy and browser outcomes are recorded separately in `verification.json` and `board-guide-ui-probe.json`; a command is only recorded as passed after it exits successfully. Browser screenshots are under `docs/previews/board-guide-*.png`. The browser probe loads the actual built React/CSS under production CSP and exercises the native dialog, both entry points, all three families, both languages, keyboard focus, troubleshooting, editor navigation, dark mode and 1280×800 / 800×700 viewports. Native startup/provider replies in that probe are explicit fixtures, not real device or model results.

`npm run tauri -- build --no-bundle --config src-tauri/tauri.windows.conf.json` exits **0**, including `build:release` and native release compilation. The local application is `src-tauri/target/release/vibe-kidbright.exe`; its digest and build-input frontend digest are in `board-guide-build.json`. This is an unbundled local build, not a new installer or portable distribution. The already-running development server at `http://localhost:1420` also serves the updated guide module. No running user process was stopped. There was no automated click against the installed desktop WebView; the successful interaction checks above used the production browser with explicit native fixtures.

μAI/Plus camera capture, model training and deployment remain unavailable; their controls remain disabled. The guide does not detect a board or configure it. Offline guide content is a preparation summary rather than the entire supplied PDF. Existing NSIS 3.7.3 is preserved and does not contain this fix. No installed app is silently replaced, and no commit, publish, deployment or release promotion is performed. Building a local executable is distinct from installer or clean-machine qualification.
