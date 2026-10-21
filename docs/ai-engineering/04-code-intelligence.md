# Code intelligence

Updated 2026-10-18. Documentation describes the current implementation, not certification of every C dialect or board.

## Syntax parser and packaging

C/H ingestion now uses statically linked `tree-sitter = 0.27.1` and `tree-sitter-c = 0.24.2`, exact versions in Cargo.toml/lock. Native Windows MSVC compilation passed. The original lexical scanner remains a marked fallback, and Markdown/text keep paragraph chunking. No parser service, Node addon, Python dependency, runtime grammar download or extra parser DLL is required.

Chunks preserve exact UTF-8 byte start/end (end-exclusive), original CRLF, source/parent line ranges, parent byte ranges, grammar node kind, function signature, includes, symbols, parser identity and syntax-error status. Deserialization defaults permit reading older records; chunk fingerprint v3 forces reingestion. Macro expansion, inactive conditional selection, ESP-IDF extension semantics and C symbol identity are not established by Tree-sitter. Symbols include uses, not only declarations. Includes/symbols are bounded at 64/128; AST traversal is iterative. More than 4096 chunks per source fails indexing while retaining the previous transaction.

Tests cover malformed C, exact Thai/CRLF slicing, strings/comments/macros, functions, large parent ranges, generated C and incremental changes. The final verification record distinguishes native compilation from installer verification.

## Project-scoped incremental search

The existing Engine now supports a separate private scope. `project_search` indexes only saved C/H beneath `main/` and `components/`, with SHA-based create/modify/delete refresh on each request. Indexes live under per-user `private-code-index/<canonical-project-hash>` and cannot mix with the public KB. Hidden/build/target/node_modules and Windows reparse points are excluded. Project records always remain private, even if bytes match bundled reference content. Project search currently uses lexical syntax ranking, not a second embedding stack.

Both provider tool schemas expose this route. The existing private-context gate is checked before reading/indexing/exporting results: remote providers need explicit consent; actual loopback local providers retain local access. Revocation cannot retract prior exports. No automatic background watcher is enabled; refresh happens when the tool is queried. Unsaved editor buffers are not indexed. Deleting a file prunes its active chunks and unused embedding rows; deleting all per-project caches still requires an explicit future cache-management UI.

## Semantic references

An optional **C intelligence** panel is now integrated into the editor in the unreleased working tree. It defaults to Tree-sitter syntax candidates; explicitly enable and select a trusted local clangd executable for semantic definitions/references. A saved executable path does not enable execution on a later mount. No executable is downloaded or bundled. The application uses its existing ESP-IDF resolver for the trusted tools directory.

`intelligence/semantic.rs` runs a contained local clangd process per request. The requested existing C translation unit must have an exact entry in `build/compile_commands.json`; no guessed target/header command is used. Compiler paths must be absolute GCC drivers within the configured SDK tools; query-driver is an exact path, never a glob. Project/user clangd config and background indexing are disabled, PCH storage is memory-only, and returned locations are restricted to project main/components C/H. The existing workspace lease excludes conflicting SDK/writes until cleanup.

Actual ESP-IDF uses `@build/toolchain/cflags`. Only that bounded, checked response file is allowed; nested responses and plugin/config flags are rejected. Both the CDB and response content are checked again after the request. Flags are not rewritten. This prevents silently falling back to desktop compiler assumptions while retaining the genuine SDK configuration. It is not a sandbox against hostile OS filesystem actors or arbitrary behavior in a user-selected executable.

Up to 16 open buffers (512 KiB each) carry the unsaved source snapshot over local LSP with UTF-16 positions. No source is saved or exported to AI by this panel. Switching files/projects, editing the snapshot or Stop cancels/invalidates the old result. Native requests have a 20-second deadline and bounded framing/queues; process cleanup reuses the Windows Job Object tree owner. Early Stop notifications are retained for 30 seconds with a bounded cancellation map. Missing executable/CDB, unsupported drivers, source/configuration errors and timeouts return labelled syntax candidates rather than claiming semantic results.

`semantic-adapter-evaluation.json` records six actual application-adapter queries across two public generated ESP-IDF projects with pinned clangd 23.1.0: definitions, references and unsaved definitions. File hashes remain unchanged. Frontend tests separately cover stale workspace results, Stop, explicit enable and fallback labels. These are not an actual desktop IPC/GUI integration test. References cover opened translation units; header inference, unopened/all-project reference completeness, persistent sessions, rename/completion and performance qualification remain unimplemented or unmeasured. Syntax fallback searches open-buffer candidates only and is not semantic identity.

The earlier `clangd-evaluation.json` remains historical exploratory evidence. Its two refactoring-tweak failures are not compiler parsing failures.

[clangd compile commands](https://clangd.llvm.org/design/compile-commands) explains the compilation context and explicit query-driver execution requirement. [Tree-sitter parser documentation](https://tree-sitter.github.io/tree-sitter/using-parsers/) describes syntax parsing, not semantic analysis.


## Subsequent actual desktop integration

The earlier adapter-only limitation is superseded for the exercised debug-native Windows path by desktop-semantic-integration.json: actual WebView2, production React/CSP, existing Tauri IPC handlers, installed SDK/clangd and Monaco, without native/provider stubs. Twenty-one observations pass across definition/header-declaration navigation, references, unsaved buffers, malformed CDB, absent clangd, Stop, native 20-second deadline, in-flight project switching/stale-result discard and a separately labelled IPC root rejection. Saved sources stay unchanged and the generated-fixture CDB is restored. A stalled executable injects cancellation/deadline faults, not semantic answers; both children terminate. Six adapter queries are rerun in semantic-integration-adapter.json.

The debug-only desktop_evaluation example reuses the app builder/registry with fresh AppData/WebView under target/desktop-evaluation. It disables legacy import and returns empty credentials before any keyring access; credential writes fail. There is no release entry point or IPC command for it. GUI-linked examples need Common Controls v6 (initially failed with 0xc0000139 before main). The successful probe uses the authorized Windows user session; sandbox CDP was unavailable. An isolated production asset server avoids Vite/HMR reload interference during long faults.

This qualifies the tested debug-native paths, not an installed release/NSIS. Main-source definition navigation reaches kb_core.h's declaration. A header without exact CDB intentionally falls back; references cover open translation units. Project switching exercises force-project-dir via real IPC, not the OS picker. Header inference, unopened/all-project accuracy, persistent LSP and other platforms remain incomplete.


The exercised editor navigation uses the existing C intelligence panel buttons and result list. A Monaco F12/context-menu provider, completion and rename are not implemented or claimed.


## Custom SDK readiness integration fix

The real desktop exposed an existing functional bug: a correctly configured external SDK worked in the resolver/semantic path but check_toolchain checked only the managed AppData installation sentinel, leaving Build disabled. check_toolchain now retains the managed-bundle path and additionally accepts the same existing SDK resolver plus Python-venv layout check. It reports source=configured-sdk without manufacturing an install marker, installing dependencies or running a command during detection. Rust tests reject missing SDK/Python and verify readiness without a sentinel. This is layout discovery, not full dependency/version qualification. The actual desktop now unlocks Build, then a typed idf.py build traverses React terminal -> Tauri -> the configured SDK to completion on the generated forever fixture, without Flash or source-byte changes. This adds two observations to the semantic matrix (21 total), and native test totals become 194 library + 9 example = 203. The four fixture compilation hashes still match after the additional desktop build.

## Opened-component regression qualification

semantic-hardening-20261019.json adds real debug-desktop header-reference fallback and opened-component navigation (27 observations total). Three references span main plus the opened kb_reference implementation; selecting the cross-component definition navigates Monaco to its actual function. The extra read-only component tab is bootstrapped with an explicitly synthetic existing native app event, without creating a patch. Header reference results are labelled syntax candidates; exact header CDB inference and background/unopened-project indexing remain disabled. Cancellation/timeout/project-switch/root-boundary checks pass in the expanded runs, and an observation timeline retains unexpected view changes for diagnosis. The old unexplained transition did not recur but is not declared causally fixed. F12 and installed-release/cross-platform completeness remain unqualified.


## UTF-16 coordinate integrity

The evidence-integrity increment removes a phantom EOF line and validates in-scope clangd ranges against unsaved UTF-16 buffers or bounded saved source. Split surrogates, invalid lines and reversed ranges fail into labelled syntax fallback. evidence-desktop-20261019.json records 32 real debug-desktop observations, including exact Thai/emoji definition navigation (Monaco column 24), unsaved references and two IPC-only negatives. Header inference, unopened-project completeness, F12 and installed-release qualification remain absent.


## Protocol-range resilience

capability-desktop-20261019.json expands the real debug desktop to 39 observations. Three invalid EOF/surrogate/reversed server ranges use an explicitly debug-only protocol fault fixture, not real clangd. Additional IPC-only tests cover empty source, a valid empty final line and a rejected extra EOF line. Existing real Thai/emoji unsaved navigation, header fallback, isolation, cancellation and stale-result checks remain. Header inference/F12/whole-project/platform completeness is not added.
