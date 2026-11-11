# Project opening and complete Plus drafts — 27 October 2026

The user's screenshot showed `mai_test_022` selected while the editor contained
MP-021 calibration code. Read-only inspection confirmed that this was also the
file stored in the new project, rather than proof that the editor had opened the
wrong directory. Its manifest explicitly said `seed_requires_custom_implementation`.
The assistant had asked for Keep before implementing the requested S1 counter.

There was a separate refresh defect: opening the current directory again kept
saved tabs without reading updated files. Delayed reads could also become valid
again after switching away and back to the same directory.

## Changes

- Reopening reloads saved tabs and refreshes the file tree; unsaved buffers stay
  intact. A chosen project opens its own main program, including two Plus projects
  whose entrypoints are both called `main.py`.
- Project generations reject stale file/diff reads across workspace switches.
  Failed main-file reads or refreshes no longer produce a successful Project opened
  message. A late previous-project selection does not overwrite the active log.
- Monaco models use the complete file path, so same-named files in different
  projects have separate models and edit histories.
- `create_project_workspace` accepts complete `main_python` source for a Plus
  draft. A named MP test with no matching complete template requires that source
  at creation, so MP-022 cannot be created with only an MP-021 seed. Both provider
  schemas expose the argument. Missing/invalid source fails before directory creation.
  Passing an unchanged stock example as `main_python` is also rejected for a
  named custom test, including differences only in CRLF/LF line endings.
- The tool response and public guides explain that `write_file` proposals are
  allowed during creation review. Complete the requested code before asking for
  Keep. Keep reviews the finished draft; it is not permission to begin implementing it.
- Existing user projects and pending reviews were not overwritten by this repair.

## MP-022 recovery project

Prepared locally: `src-tauri/target/recovery-projects/mai_test_022_checked`.
It contains the actual S1 counter plus a Python calibration sidecar copied from
the user's MP-021 measurements, rather than a second calibration program.

The controller debounces a stable released state before counting S1, ignores S2,
does not count repeated frames during a hold, and never treats an out-of-band
reading as a confirmed release or a new press. The calibration belongs to this
project/session; there are no shared default ADC thresholds in the sample.
Stop uses the existing IDE supervisor. Physical repeated presses remain unverified.

## Validation

- Frontend: 203 tests passed, including five new project-open regressions for
  saved-file refresh, dirty-file preservation, identical entrypoint names,
  switch-away/back stale reads and failed refreshes without false success logs.
  A separate regression checks distinct Monaco model identities for same-named
  entrypoints in two projects.
- Native library: 239 tests passed, including complete custom Python creation,
  rejection before creating an unfinished named-test directory, and both schemas.
- Plus Python: 54 tests passed, including six new counter fixtures covering
  initial hold, 20 presses with holds, ambiguity, bounce, S2 and invalid calibration.

These are software/fixture results. No device run, firmware change or installation
was performed automatically. Installer details are recorded separately in
`docs/research/uai-plus-runtime-2026-10-26/project-open-validation-2026-10-27.json`.
