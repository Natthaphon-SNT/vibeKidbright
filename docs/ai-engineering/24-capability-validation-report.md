# Capability validation framework — 2026-10-19

Started from 253 matched recorded hashes, inspected Git diff/current source and the evidence-integrity reports. Existing dirty changes are preserved and are not attributed to this increment. Production MiniLM/ranking, frozen 69 query/qrel bytes, bundled corpus, nine Block IDs/schema v1, SDK configuration and historical NSIS 3.7.3 are retained. No authorized local endpoint/model was supplied; no endpoint probing, inference, paid-key access or model quality grading occurs.

## Implemented framework

The existing real-Vibe harness embeds a sixteen-criterion capability view from scripts/lib/capability-matrix.mjs. scripts/evaluate-capabilities.mjs provides an offline, additive review/export path. Each criterion distinguishes PASS, FAIL, NOT_EXECUTED and NEEDS_HUMAN_REVIEW, with separate AI-quality and mechanical statuses. CAPTURED never automatically becomes AI-quality PASS. Legacy capability-level fields remain for compatibility; the new matrix is the explicit four-status view.

Review judgments bind exact evaluation bytes, an identified independently self-attested reviewer, case IDs, notes and hashed evidence references. The CLI verifies referenced bytes, UTF-8, workspace/allowed-root containment and symlink boundaries, and refuses stale references/existing outputs. Actual-model review output stays under ignored target. Input/proof sizes and reference counts are bounded; duplicate proofs are hashed once. File identity is not proof of reviewer independence or actual provider execution. A malicious fabricated local report is not authenticated by this tooling.

The draft capability-review form has null judgments. Optional manual compile/diagnostic/repair/rollback packets are hash-bound user observations, require later assessment and authorize no app action. Separate deterministic observations never change model-dependent statuses. No reviewer judgments are synthesized outside explicitly labelled unit fixtures. See 23-capability-validation-protocol.md for reproduction and schema details.

## Capability matrix

The authoritative capability-matrix-20261019.json uses the prepared report and capability-mechanical-complete-20261019.json. All **16 AI-quality criteria and all three levels are NOT_EXECUTED**. Ten separate mechanical observations are PASS, scoped to deterministic source/navigation/repair/parser/build checks; the other six have no mechanical observation. Those ten passes are not ten AI capabilities. The earlier partial mechanical export is retained as a separate intermediate report.

The sixteen criteria are retrieval relevance; citation existence; factual citation support; API/version correctness; hardware metadata consistency; project identification; file/symbol selection; dependency understanding; patch scope; unrelated-code preservation; compiler-error classification; focused repair; per-attempt approval; compilation success; rollback correctness; and bounded attempts. Each matrix row includes required workflow cases and acceptance evidence. Missing required cases prevent a complete criterion/level PASS. Mechanical failure and model-quality rejection are distinguished in per-case fields.

## Executed tests and semantic integration

Frontend: 178 tests in 18 files; Rust: 196 library plus ten example tests (206); Python: 22; Node: 12; host C: 490 assertions. All pass. Node tests exercise preparation/preflight isolation, no CAPTURED promotion, independent-review gates, stale identities, missing cases, failed evidence, manual-stage boundaries and actual CLI no-overwrite/hash rejection. Positive review examples are synthetic test data, not human review results. Debug examples, Rust offline locked release configuration, frontend release build and source policies are verified separately in capability-verification-20261019.json.

Actual debug desktop production React/CSP/Tauri/Monaco/clangd passes 39 observations with no page errors, unchanged saved main/component source and restored CDB. Existing real UTF-16 Thai/emoji navigation, unsaved references, header declaration/reference fallback, isolation, cancellation, native timeout/child death and stale-result rejection remain. Added empty-source/final-line/EOF cases are IPC-only. A debug-only range_lsp_fixture injects invalid EOF/surrogate/reversed ranges through the UI/native path and verifies labelled syntax fallback; these three tests are explicitly protocol faults, not real clangd semantic results. No F12/header inference/unopened-project or installed-release qualification is added.

Six separate native adapter queries pass across two SDK-configured projects. The production browser smoke uses native/provider fixtures and certifies UI/CSP wiring only. The actual-desktop recorder preflight is a synthetic native marker with zero inference; its new capability matrix also remains NOT_EXECUTED. The read-only component-tab bootstrap remains explicitly synthetic. Historical failures/view-transition uncertainty are retained, not rewritten as fixed.

## Compiler repair and firmware

Existing frontend approval tests verify project/revision binding, consent consumption, fresh consent after failure and prevention of old approval resurfacing. Native tests verify stale/phase rejection, duplicate failed patch rejection, the three-attempt maximum and journal rollback/recovery. The real-file attempt test now reopens serialized session/journal representations between attempts and verifies exact original source plus an untouched unrelated header. This simulates persisted-store reopening, not power loss, OS reboot or an actual model repair workflow. User projects/credentials are not test fixtures.

Four deterministic outputs freshly compile/relink with configured ESP-IDF 5.5.4/esp32, raw exit 0, unchanged source and recorded CDB/log/firmware hashes. They preserve nine IDs/schema v1 and pilot board restrictions. No Flash, actual model-generated firmware or physical correctness is claimed. Native configured SDK/review/rollback implementation is retained.

## Retrieval and real-provider readiness

capability-retrieval-readiness-20261019.json verifies frozen bytes, all 37 frozen source files and qrel path identity. It does not certify relevance. The existing blind development draft contains four questions in each of exact-symbol, natural-language, mixed-language, compiler-error and hardware kinds. All corpus judgments remain null; the strict gate rejects export as expected. The final set still has no compiler-error queries. The new capability review form covers citation support and API/version/hardware assessment without inventing annotations or API/board mappings.

Production ranking, original-query route and MiniLM remain unchanged. No retrieval performance evaluation is executed this increment; performance metrics remain null in the readiness report. Prior Recall@5/MRR/NDCG, latency/RAM/indexing/model costs remain in retrieval-fixed-measurement-20261019.json with their assistant-labelled, single-host limitations. Production prior Recall@5 is 0.3841 (Thai 0.125), and no new accuracy gain or model selection is claimed.

Real-model requests, approved model patches, model-assisted builds/diagnosis/repair/rollback and independent judgments are all zero/unexecuted. Three capability levels remain NOT_EXECUTED in the new matrix (legacy NOT_EVALUATED is unchanged). An endpoint becoming available would still require explicit authorization before inference. No paid credentials are read.

## Release qualification and unresolved work

The release-readiness checklist is capability-release-readiness-20261019.json. It covers actual model-assisted workflows, independent retrieval/citation review, clean Windows VM install/upgrade/uninstall, VM user-data preservation, hardware, advisories and cross-platform behavior. It remains NOT_READY. Source/test success and preserved installer hashes do not authorize a new release candidate.

Fresh npm/OSV audits report zero npm advisories and the same whole-lock Rust ten advisory records/eight packages. Selected Windows normal/build findings are two maintenance notices with no non-maintenance matches at lookup time. Other-platform advisories/maintenance migrations remain; this is not a penetration test or real-model injection-resistance measurement.

Historical NSIS 3.7.3 remains 316,758,944 bytes with SHA256 d09d0f9418d94059e801fc43a38241314757c686e5060e26596a806ba56ae7c1. Model assets match the preserved extraction and manifest pins. Current debug probes are not a new installer. No clean VM, hardware run, version promotion, commit, push, publish or deploy occurs. Current user AppData/settings/credentials and unrelated projects are not targeted; preserving temporary fixture bytes does not certify VM upgrade/user-data preservation.

All eight original requirements remain PARTIAL: system architecture (wider OS/storage fault qualification); hybrid RAG (reviewed relevance/grounding); knowledge-base data (authoritative version/metadata coverage); code-aware chunking (header/unopened/platform completeness); compiler validation/Auto-fix (actual approved model workflow); technology integration (installed/VM/cross-platform qualification); development phases (three reliable capability gates); evaluation quality (independent and real-model/physical/VM evidence). Source hashes and exact executed commands are in capability-changes-20261019.json and capability-verification-20261019.json.
