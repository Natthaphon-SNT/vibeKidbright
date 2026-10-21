# User instructions and known limitations

Updated 2026-10-18. Read verification.json for checks actually executed on this source revision.

## Review and recovery

AI writes, including new files and source-list changes, are persisted as proposals. Inspect the editor diff and Keep the current revision, or Undo to reject it. Unsaved editor drafts disable approval. A stale approval is rejected and the UI refreshes the review. New-project Keep/Undo persists across restart; Undo moves an unchanged tree to a recorded same-parent recovery location. It does not recursively delete a possibly edited project.

Message Undo restores exact accepted source only if nobody edited it later. On a conflict, your files and local recovery snapshots remain. Do not remove or overwrite a corrupt journal to make the app start: preserve AppData and recover from its records. Journals contain private source snapshots locally and are bounded, not encrypted or automatically discarded. OS filesystem actors can still race checks; this is not a complete sandbox or transactional filesystem. Unsaved editor drafts are not yet restored across app restarts.

## Knowledge, code search and privacy

Use knowledge_search for bundled/reference documentation, and project_search for saved C/H under main/components. Project results include syntax metadata and exact source byte offsets, not certified semantic references. Refresh is on query; no background watcher. The local project index is separate from the shared KB and retains no credential/docs folders. Modified/deleted C files refresh/prune incrementally. Imported metadata cannot turn private content into a public reference.

For private tool excerpts to remote providers, explicitly select Share private context with remote AI in provider settings, then Save. Default is off; a true loopback endpoint can work locally. The same gate covers project search, file/log tools and web searches. Typed/pasted user chat is still sent when Send is pressed. Revocation cannot retract prior exports. This is not full DLP or a privacy audit of every chat/settings flow.

No new embedding service is required. The bundled MiniLM remains. Candidate models are in an ignored development cache, never installer resources. The 69-query dataset is assistant-authored and non-exhaustive, with translated/topic overlap; it is not an independent human benchmark. Initial codepage-damaged measurements are explicitly invalid and retained separately. No parameters were tuned on the corrected held-out results.

## Compiler Repair

Start from the build error panel. Inspect all changed files in Compiler Repair, check the approval box and approve each attempt separately. The SDK maximum is 1..3 attempts; select the limit before the first application (UI default 3). The build uses the selected installed ESP-IDF, with no flashing. Stop verification cancels its process tree. Failed attempts need explicit Undo/review before another proposal; identical unsuccessful patches are rejected. After interruption, review recovery rather than assuming the build passed.

The workflow does not automatically spend provider calls or continuously apply model edits. No actual LLM repair success percentage is measured. Setup/download/ADB/serial subprocess lifecycle coverage remains separate from the controlled SDK route. Native timeout defaults to 1800 seconds with supported range 1..7200.

## Explicit remaining scope

- No full every-board/every-function/every-plugin KidBright equivalence. The unreleased typed subset covers nine source IDs and normalized JSON, not arbitrary XML/Blockly files. The preserved installer has the earlier four-ID pilot.
- Tree-sitter syntax is not semantic identity. Optional local clangd navigation uses exact SDK CDB/open unsaved snapshots, tested through six native adapter queries. Actual desktop IPC and header/unopened/all-project reference completeness are not certified.
- Source metadata/version coverage remains incomplete and often unknown; the full official ESP-IDF corpus is not bundled.
- No unified whole-conversation tokenizer budget, large-corpus ANN, full index-management settings UI or automatic project watcher.
- Physical Block timing, wiring, matrix orientation, sensors/radio/motors and all-board behavior remain physically unverified for this increment. Prior user-reported iA evidence is not proof of these new fixtures.
- microAI/Plus remain future work; no deployment/training support added.
- All source/package version declarations are aligned at 3.7.3. Earlier 3.7.2 installer evidence is historical; the rebuilt NSIS and extracted-binary checks are recorded in installer-validation-3.7.3.json and installer-probe-3.7.3.json. They do not validate clean-machine GUI installation.
- No fresh virtual machine/macOS GUI, live provider conversation, flash, publish or deploy unless explicitly recorded as executed.

The NSIS 3.7.3 artifact remains byte-identical to its qualification report. New development features in this working tree have not been bundled or promoted. Static inspection finds an explicit Delete app data checkbox in the old uninstaller; keep it unchecked to preserve settings/recovery data. Install/upgrade/rollback/uninstall on a clean VM have not been executed. See installer-preservation-review.json.

## Dependency security follow-up

See 12-dependency-security.md and the current advisory JSON. Fixed Tauri IPC/TLS/HTTP/ZIP releases are integrated, but the entire multi-platform Cargo.lock is not advisory-free. Windows selected graph findings are classified separately from GTK/Linux portal dependencies and maintenance notices. No universal security certification is claimed. Automatic absolute project-path injection into chat was removed; native private project metadata and SDK-path context also require private-context permission. Explicit typed/pasted user content and every possible OS-error string are not a complete DLP boundary.


## Subsequent integration scope

The earlier lack of any desktop IPC evidence is superseded only for the debug-native Windows paths in desktop-semantic-integration.json. Installed-release IPC, header inference, unopened/all-project reference completeness and other platforms remain unqualified.

Fixed expansion is enabled after the separate development gate, but Thai frozen Recall@5 remains only 0.1250 and English MRR regresses slightly. A small alias vocabulary is not general translation/intent or compatibility proof. Board/version prose does not create metadata constraints. No tuning/model change followed final scores. No local model was available at discovery ports 11434/1234/8080. Prepared prompts/harness and actual-desktop no-inference preflight are not real LLM quality, repair or rollback results. Original installer/independent annotation/VM/physical limitations still apply.

## Three-level workflow evidence (2026-10-19)

The strengthened real-Vibe harness can collect native traces, verify bundled source excerpts and detect differences in its scoped source manifest. No actual model run has exercised that new collection branch; the actual desktop preflight validates listener subscription/cleanup and an explicitly synthetic native marker, not model/tool outputs. Citation support, dependency understanding, generated-C quality and human approvals remain ungraded. The complete official SDK corpus is still absent. Model-generated compilation/diagnosis/repair/Undo stages remain unexecuted; existing builds do not fill that gap. See 18-capability-milestone.md. No capability level or original requirement is marked COMPLETE.

## Diagnosis increment

The user confirms no relevance reviewer and requests keeping ranking unchanged. The new twenty-question draft is not reviewed or used for tuning. Diagnostic flags from the frozen set are not new training labels. The final set contains no compiler-error probes. Opened cross-component navigation and labelled header-reference fallback now have real debug-desktop evidence; semantic header inference/unopened-project completeness/F12 do not. The earlier unexplained view transition did not recur in the expanded runs, but no cause/fix is claimed. Fresh npm/OSV audits do not clear inactive platforms or maintenance risks. See 20-diagnosis-and-hardening-report.md for all remaining boundaries.


## Evidence-integrity follow-up

The harness now fails known invalid source/citation evidence and missing requested mechanical evidence; it still cannot grade relevance, factual support, alternate citation syntax or generated code quality. Actual debug-desktop Thai/emoji coordinates have 32-observation evidence; full semantic/header/platform qualification and F12 remain open. Repair consent is project/revision-bound and consumed on Apply, with frontend/native safety checks, not an actual model repair session. Frozen data/ranking/model remain unchanged and no reviewer/model/VM/physical-board blocker is resolved. See 22-evidence-integrity-report.md.


## Capability framework limitations

The explicit sixteen-criterion matrix and hash-bound review forms are evaluation infrastructure, not AI quality results. File identity cannot authenticate an independent reviewer, a provider invocation or recorded manual-stage claims. All model-dependent criteria remain NOT_EXECUTED; actual model-assisted repair, independent review, VM and hardware evidence are absent. Debug protocol fault tests demonstrate invalid-range fallback, not additional real clangd correctness. Clean-machine user-data preservation and cross-platform behavior remain unqualified. See 24-capability-validation-report.md.
# iP custom draft qualification boundary (2026-10-19)

Task-scoped clarification and SDK/starter fallback remove the reported mechanical barriers. No authorized real provider has replayed the user's conversation, and no iP scrolling firmware/physical orientation test was executed. The iP model is identified by the user; the unknown PCB revision is not silently equated with the manufacturer's Rev.2021.04.01 schematic. A custom draft is permitted without adding a deterministic profile. Private sharing remains necessary for exporting existing private content and installed SDK/build information. Local no-bundle output is not a newly qualified installer. See [26-ip-custom-draft-workflow.md](26-ip-custom-draft-workflow.md).

