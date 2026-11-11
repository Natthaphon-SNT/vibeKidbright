# Plus touch test and conversation continuity — 26 October 2026

The user reported that the Vibe Coder repeatedly asked what `v2` meant after
offering `mai_test_015_v2` as a replacement name. It later created that project
with a display-only template and explicitly said MP-015 had not been implemented.
Uploading that template does not exercise touch input.

## Conversation correction

With private-context sharing disabled, `remote_history` previously retained only
the latest user message. That removed the original task and the assistant's name
choice before the remote provider saw `v2`. Separately, reloading a saved chat or
remounting its panel reset API history without reconstructing the visible dialogue.

The remote path now preserves user-authored dialogue and replies/tool exchanges
that the native backend verified were produced without private context. A bounded
in-memory cache stores only message hashes. Assistant/tool content from local or
private-context turns remains blocked without consent; client provenance flags
are not trusted. Tampered and incomplete tool exchanges are dropped. Enabling
private context at any request/tool step prevents the resulting turn from being
recorded as public. After an app restart, unknown assistant/tool provenance stays
blocked while the user's own messages remain available.

The UI reconstructs saved visible dialogue when API history is absent, omitting
display-only tool summaries. Live complete API history remains intact. Error and
Undo recovery retain the discussion while removing stale tool results. Completion
events use the backend's final text even if no streaming delta arrived. The system
prompt resolves short selections against the unfinished task; existing-folder
protection and Keep/Undo still apply.

## Touch implementation

The bundled knowledge document now contains the actual touch contract and a
`python_example="touchscreen"` factory option. The example creates an MP-015
program that reads real touch events, shows raw x/y/pressed and a marker, counts
press/release edges, uses a canvas matching the runtime display, and closes the
touch device in `finally`. It does not simulate touches or declare a physical pass.
Missing API symbols produce SKIP; driver open/read failures remain errors.

The API was verified against the public
[MaixPy touch guide](https://wiki.sipeed.com/maixpy/doc/en/vision/touchscreen.html)
and read-only inspection of the user's supplied image. Its
`maix/_maix/touchscreen.pyi` declares `TouchScreen()`, `read()` returning
x/y/pressed, `available(timeout=0)`, `is_opened()` and `close()`.
[Image API evidence](research/uai-plus-runtime-2026-10-26/touchscreen-api-reference.json)
records the file hash. A symbol declaration establishes an API reference, not a
physical driver pass. Public reference retrieval from the bundled KB does not
require exporting private project context.

A fresh recovery project was prepared at
`src-tauri/target/recovery-projects/mai_test_015_touch` with `main/main.py` and
Plus target metadata. Open that folder in the development IDE and run it through
the user-operated Connect Plus / Run Python controls. The source template is
[touchscreen.py](../samples/kidbright-uai-plus/touchscreen.py). Existing user
projects were not inspected or overwritten to prepare this recovery project.

## Validation

| Check | Result |
|---|---|
| Rust library | 234 passed, including public-history continuity, blocked private/tampered history, complete tool exchanges and bounded provenance |
| Frontend | 197 passed, including saved chat, short `v2` replies, panel remount, backend error and workspace Undo |
| Plus Python | 33 passed, including touch press/move/release, missing API, driver/read errors, cleanup and out-of-range coordinates |
| TypeScript / production frontend build | Passed |
| Bundled knowledge / reviewed source hash | Synchronized and verified |

These are local software checks. The user confirmed the earlier basic
Connect/Run/Stop cycle; the new touch test still requires the user's physical
observation. The prior installer artifacts were not rebuilt for this change.
