# Evidence integrity and semantic coordinates — 2026-10-19

This unreleased increment starts from 242 matched recorded hashes, inspected source/Git diff and prior integration/diagnosis evidence. Historical NSIS 3.7.3, production retrieval/model/corpus, frozen 69 questions/qrels, nine Block IDs/schema v1 and lockfiles are preserved. The user keeps ranking because no reviewer is available. See 21-evidence-integrity-plan.md; repair-review consent was added after the initial frontend failure/source inspection.

## Implemented fixes

The source-evidence collector detected bad excerpts, but the real-Vibe harness ignored these failures when accepting completed cases. `scripts/lib/vibe-evidence.mjs`, `evaluate-vibe-coder-real.mjs` and the case manifest now reject malformed search payloads, invalid source hashes/ranges, unmatched literal citations, empty answers and missing requested tools/cited sources. Seven deterministic Node tests cover valid/stale/invented/malformed/empty/interrupted traces. CAPTURED means ungraded mechanical evidence, never a model-quality pass. Relevance, factual support, project understanding, compiler validity and human quality remain null. Alternate citation formats/support require human review; fixtures are not model results.

`semantic.rs` previously added a phantom EOF line. UTF-16 conversion now rejects nonexistent lines while preserving a real empty line after a newline. In-scope clangd ranges are checked against exact unsaved snapshots or bounded saved source; reversed/invalid/split-surrogate ranges fall back to labelled syntax candidates. Foreign paths remain excluded. No header inference, whole-project completeness or F12 is added.

The initial frontend run failed its existing approval case (175 passed/one failed), and its raw log is retained. Inspection found approval stored as an unbound boolean cleared by a later effect. `CompilerRepairReview.tsx` now binds consent to project and revision and consumes it when Apply starts, including failure with the same revision. New regressions cover retry consent and equal revision labels across projects. Native per-attempt review and the 1..3 bound remain. The initial failure's precise scheduling cause is not established solely from its log.

## Actual verification

After fixes: frontend **178 tests/18 files**, Rust **196 library + 10 example = 206**, Python **22**, Node **7** and host C **490 assertions** pass. Debug examples, offline locked Rust release check and frontend release/policy build return exit 0. Exact commands, exits, logs/source hashes and retained failures are in evidence-verification-20261019.json; files/before-after hashes are in evidence-changes-20261019.json. Counts alone do not certify capability levels.

The rebuilt debug desktop passes **32 observations** through actual production React/CSP, Tauri, clangd and Monaco. Thai/emoji precede an unsaved function: definition navigation reaches exact Monaco column 24 and references return both expected occurrences. Two separately labelled IPC-only negatives reject a phantom line and split surrogate. Existing cases retain header-reference syntax fallback, opened-component navigation, unsaved text, invalid CDB, unavailable clangd, cancellation, native 20-second timeout/child death, switching, stale-result rejection and root isolation. Sources are unchanged, CDB restored and page errors empty. Actual React-terminal SDK completion is distinct from helper-captured raw firmware exits.

The adapter separately runs six queries across two configured projects. The production-browser smoke uses explicit native/provider fixtures and qualifies UI/CSP only. An explicitly synthetic native event opens a read-only component tab; it is not a model edit/approval. The earlier unexplained view transition did not recur; no causal fix is claimed. Installed-release/cross-platform, header inference, unopened-project completeness and F12 remain unqualified.

## Retrieval and actual LLM status

No retrieval benchmark is rerun or rule selected here. Engine/expansion/evaluator/frozen-set/corpus/model hashes remain unchanged from retrieval-fixed-measurement-20261019.json. The following are prior fixed-policy, single-host, assistant-labelled measurements, not new gains:

| Mode | Recall@5 | MRR | NDCG@5 | Thai Recall@5 | Peak working set MiB |
|---|---:|---:|---:|---:|---:|
| Lexical | 0.3478 | 0.3225 | 0.3027 | 0.0000 | 19.7 |
| Bundled MiniLM | 0.3623 | 0.3889 | 0.3488 | 0.0500 | 373.1 |
| Current production expansion | 0.3841 | 0.4010 | 0.3616 | 0.1250 | 373.1 |
| Multilingual E5-small | 0.4275 | 0.3877 | 0.3671 | 0.2000 | 1384.7 |
| Multilingual MiniLM | 0.3913 | 0.3635 | 0.3343 | 0.1000 | 1352.5 |

The referenced JSON retains per-language accuracy/latency, RAM and indexing/model costs. Diagnosis still records duplicate slots, one English MRR regression and zero compiler-error cases; non-exhaustive qrels are uncertain. The 20-question draft remains unreviewed/unused. Original-query, exact-API and board/version-filter paths are unchanged; unknown metadata is not compatibility proof.

No explicitly authorized ready endpoint/model has been supplied since the user's no-local-model reply. No paid credentials or inference are used. evidence-prepared-20261019.json is preparation only; desktop recorder preflight is a synthetic native marker. Connected real-model generation/compilation/diagnosis/approved repair/rollback is NOT_EXECUTED; quality metrics are null. Knowledge-aware, Project-aware and Build-aware remain NOT_EVALUATED.

## Firmware, security and release

Four deterministic outputs freshly compile/relink with ESP-IDF 5.5.4/esp32, raw exit 0 and unchanged source. See evidence-firmware-20261019.json for source/CDB/log/firmware hashes. Nine IDs/schema v1/exact pilot profiles remain; no Flash, model-generated firmware or physical correctness claim.

Fresh npm/OSV checks retain zero npm advisories and ten whole-lock Rust records across eight packages. Selected Windows normal/build findings are two maintenance notices, with no non-maintenance matches at lookup time. Other-platform findings, maintenance migrations and actual LLM prompt-injection resistance remain. Public dependency metadata only is sent; no source/credentials. This is not penetration testing or universal reachability/immunity proof.

NSIS 3.7.3 remains 316,758,944 bytes with SHA256 `d09d0f9418d94059e801fc43a38241314757c686e5060e26596a806ba56ae7c1`; production embedding files match the preserved extraction. No new installer, RC/version promotion, clean-VM lifecycle test, staging, commit, push, publish or deploy. Packaging is not warranted without independent and real-model quality evidence.

## Eight original requirements

| Requirement | Status | Remaining evidence |
|---|---|---|
| System architecture | PARTIAL | Wider storage/recovery/OS race and platform qualification |
| Hybrid RAG | PARTIAL | Reviewed evaluation and acceptable Thai/English grounding |
| Knowledge-base data | PARTIAL | Authoritative SDK/version and relevance/compatibility coverage |
| Code-aware chunking | PARTIAL | Header/unopened-project semantics and platform correctness |
| Compiler validation/Auto-fix | PARTIAL | Approved real-model build, bounded repair and rollback |
| Technology integration | PARTIAL | Installed-release/VM/cross-platform and provider quality |
| Development phases | PARTIAL | Three reliable capability gates before expansion/RC |
| Evaluation quality | PARTIAL | Independent judgments, actual LLM, physical and VM evidence |
