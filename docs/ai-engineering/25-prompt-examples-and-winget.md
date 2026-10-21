# Prompt examples and WinGet workflow — 2026-10-19

Unreleased UI/CI increment. The existing working tree contained substantial earlier changes; those are preserved and are not attributed to this increment. No commit, push, publication, provider invocation or new installer was performed.

## Implemented behavior

- Replaced the three static KidBright32 lesson tiles with a labeled native dropdown: button/sound, temperature and LED display. Thai/English examples include an explicit board-revision placeholder and ask for board verification and reviewed changes. These prompts do not establish hardware compatibility.
- Added an editable preview, per-example drafts, reset, copy and an explicit **Use in Vibe Coder** action. Untouched templates follow the selected language; user edits remain as typed. Empty prompts cannot be transferred.
- Connected LearningWorkspace → App → AiChat. The selected prompt is appended to any existing unsent text exactly once, including under React StrictMode. It opens the editor/chat without sending a message, building, flashing or applying a patch. The existing μAI/Plus idea action now also transfers its displayed text; training/deployment support is unchanged.
- Prevented Monaco's asynchronous mount from stealing focus from an already focused input/textarea. The production browser probe initially caught this failure; the final probe verifies the composer retains focus.
- Added a WinGet classic-PAT scope preflight, a manual retry entry with an existing release tag, and an installer regex limited to x64 NSIS setup assets. Tokens are supplied through an environment variable, not interpolated into shell source. The preflight captures scope headers without printing the token/user response.

## Executed validation

Exact commands, exit codes, log hashes and file hashes are recorded in `prompt-ui-verification-20261019.json` and `prompt-ui-changes-20261019.json`.

- `npm test -- --reporter=dot`: 183 tests in 19 files passed. Five new cases cover selection/editing/localization and one-time composer handoff. Earlier new-test runs failed due to an incomplete native mock and unsupported test matcher; these were corrected and the failure logs retained locally.
- `npx tsc --noEmit`: exit 0. The final `npm run build:release` also reruns TypeScript and passes the existing runtime, standalone, knowledge, board, coverage, rule and public/bundle checks before Vite.
- `powershell -NoProfile -File scripts/tests/winget-token.tests.ps1`: seven offline scope cases passed, without credential access or network requests. Windows PowerShell was used locally; an actual hosted Actions/pwsh run was not executed.
- `node scripts/check-production-workflows.mjs --report docs/ai-engineering/prompt-ui-production-20261019.json`: passed using the compiled frontend and production CSP. Native startup/provider responses are explicit fixtures. Covers Thai/English, compact dark layout, selection/editing, existing-draft preservation, composer focus, unchanged source/review state and no inference/mutation caused by prompt selection. Existing guide and review browser paths also pass. No external requests or page errors occurred.
- The initial browser probe failed the composer-focus assertion. That failure is retained in `src-tauri/target/prompt-ui-production-20261019.log`; the final run is separate. Dark screenshots wait for theme transitions to finish.
- No Rust runtime, retrieval-ranking, frozen evaluation dataset, production model or Block mapping changes were made. Rust, firmware, real-model, physical-board and installed-native-desktop tests were not rerun for this UI/CI increment; earlier evidence remains historical.

## Remaining external action for WinGet

The screenshot reports HTTP 422 during fork synchronization because WINGET_TOKEN lacks `workflow` scope. The action requires a classic PAT with `public_repo` and `workflow`; `repo` is accepted as the broader repository scope. GITHUB_TOKEN permissions cannot grant scopes to this separate PAT. An authorized owner must update that token/secret, and an authorized maintainer must land the workflow before retrying the original release tag. The package must already have an initial accepted upstream version. See [release setup](../WINGET_RELEASE_SETUP.md) and the [action prerequisites](https://github.com/vedantmgoyal9/winget-releaser#readme).

No repository secret was read/changed locally, no hosted workflow was executed and no WinGet PR was submitted. Scope tests prove preflight behavior, not a successful submission.

## Evidence and release limits

The historical NSIS 3.7.3 SHA256 remains `d09d0f9418d94059e801fc43a38241314757c686e5060e26596a806ba56ae7c1` (316758944 bytes). This frontend build does not update an installed application or that installer. Final integrity records compare model resources to the preserved extraction and current source hashes to the prior reported manifest.

The prior manifest was copied after the initial UI changes, before updating verification records. It is a reported baseline, not a claim that all files were hash-verified at increment start. The change record distinguishes previously recorded hashes from newly recorded paths and preserves prior reports.

Knowledge-aware, Project-aware and Build-aware model quality has not been reevaluated. All eight original requirements remain PARTIAL; this increment does not promote mechanical browser fixtures into model-quality, hardware, independent-review or clean-VM PASS evidence.

Current UI previews: [Thai](../previews/prompt-examples-th.png), [English compact dark](../previews/prompt-examples-en-dark.png).
