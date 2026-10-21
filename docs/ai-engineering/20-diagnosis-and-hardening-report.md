# Retrieval diagnosis and desktop hardening — 2026-10-19

This increment preserves the current production ranker, embedding, bundled corpus, frozen 69 cases and historical NSIS 3.7.3. The user confirmed that no reviewer is available and asked to keep ranking unchanged. No candidate rule, decomposition, model replacement, new Block/board, F12 provider or RC is enabled. The starting 216 recorded hashes matched before changes; the large pre-existing dirty tree is preserved.

## Implemented changes

`scripts/diagnose-retrieval.py` analyzes the historical paired expansion runs, verifies query/qrel identity and recomputes the frozen metrics. Its per-query flags are observations, not new ground-truth labels or proven causal diagnoses. Exact-symbol, natural-language, mixed-language, compiler-error and hardware groups are reported separately, with declared overlap. `retrieval-diagnosis-20261019.json` retains every query/rank/metric delta and historical multilingual resource figures.

The twenty-question development review packet remains **DRAFT, zero reviewed cases**. It contains no rankings or suggested qrels and has no normalized text/ID overlap with the frozen/prior development questions. Exact API spellings are present in existing public sources. `review-retrieval.py validate-development` requires independent self-attested review, all five kinds, complete corpus judgments/notes and disjoint cases before export. The independence flag must be boolean true, not a truthy string. The packet is assistant-authored, not an independent human evaluation set. No ranking rule was changed from its contents.

The existing Rust evaluation example and Python measurement runner now reserve new evidence before model work, preserve incomplete/failing runs, keep historical files intact and collect source-hit provenance and process resources. The example embeds its compiled source digest. A real native negative probe verifies that an existing report fails before loading an intentionally unavailable model directory and preserves the old bytes. A Windows blocked-process fault fixture verifies the runner's owned-process timeout; it is not an embedding or LLM result.

The existing desktop probe adds header-reference syntax fallback and actual opened-component definition/reference navigation through React/Tauri/clangd/Monaco. A synthetic existing native app event only bootstraps the additional read-only component tab; it creates no patch. Actual semantic requests and results are not mocked. View/click/frame observations aid diagnosis of unexpected navigation. Build/security helpers also write fresh evidence paths rather than replacing prior results.

## Retrieval findings and comparisons

The historical paired runs show 59/69 queries with duplicate file slots among returned chunks, 37/69 without a declared qrel hit, and 16/20 Thai queries without a fixed alias match. All expected qrel files exist in the frozen corpus. These are not proof that all unlabelled results are irrelevant: labels are assistant-authored and non-exhaustive, code/header counterparts overlap, and duplicates are not refilled under the frozen metric definition. There are **zero compiler-error cases** in this final set; their metrics remain null.

Only `heldout-delay-en` loses reciprocal rank: the first declared relevant document moves from rank 1 to 2 while Recall@5 stays 0.5. This accounts for English MRR 0.2417 to 0.2167. Its expansion uses RTOS-delay terms while the request concerns conversion validation. The promoted broad board document contains those terms, suggesting an intent-mismatch hypothesis; per-channel contribution evidence was not captured in the historical run, so the precise causal explanation remains uncertain. Do not fit a special rule to this final query.

Historical overall Recall@5 remains 0.3623 to 0.3841 and Thai 0.05 to 0.125 with fixed expansion; exact API recall remains 1.0 on nine probes. `19-diagnosis-plan.md` fixes new resource-reproduction modes before execution. All five fresh runs completed successfully and are retained separately in `retrieval-fixed-measurement-20261019.json`, without replacing the historical comparison. Current production rankings match the historical expanded run exactly, and all nine API rankings match between the paired MiniLM modes. None authorizes production changes or proves grounded-answer quality. Process RAM/index/model bytes and per-language latency remain separate from accuracy; host timing is uncontrolled and candidate model bytes are not installer size.


Fresh fixed-policy measurements (MiB = bytes / 1048576):

| Mode | Recall@5 | MRR | NDCG@5 | Median / p95 ms | Peak working set MiB | Cold index seconds | Model artifacts MiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| lexical | 0.3478 | 0.3225 | 0.3027 | 6.4 / 14.9 | 19.7 | 1.6 | 0.0 |
| bundled | 0.3623 | 0.3889 | 0.3488 | 220.2 / 281.7 | 373.1 | 102.8 | 86.9 |
| bundled-expanded | 0.3841 | 0.4010 | 0.3616 | 195.5 / 240.0 | 373.1 | 147.6 | 86.9 |
| multilingual-e5-small | 0.4275 | 0.3877 | 0.3671 | 223.5 / 258.4 | 1384.7 | 202.0 | 129.1 |
| multilingual-minilm | 0.3913 | 0.3635 | 0.3343 | 227.1 / 252.7 | 1352.5 | 306.9 | 129.1 |

| Mode | Thai Recall@5 | English | Mixed | Exact API |
|---|---:|---:|---:|---:|
| lexical | 0.0000 | 0.2250 | 0.5250 | 1.0000 |
| bundled | 0.0500 | 0.2500 | 0.5000 | 1.0000 |
| bundled-expanded | 0.1250 | 0.2500 | 0.5000 | 1.0000 |
| multilingual-e5-small | 0.2000 | 0.3000 | 0.5250 | 1.0000 |
| multilingual-minilm | 0.1000 | 0.2500 | 0.5500 | 1.0000 |

The JSON retains per-language MRR/NDCG/latency and process private bytes, unchanged-index costs and source-hit metadata. Candidate promotion remains ineligible: no reviewed development set and the user requires current ranking/model. These are embedding/retrieval measurements, not LLM-answer metrics.

## Desktop integration

`semantic-hardening-20261019.json` records **27 passing observations** with the current debug-native example and production frontend/CSP. It covers header-reference labelled syntax fallback, three references across two opened C translation units, cross-component definition navigation to Monaco, main/header navigation, unsaved text, invalid CDB, missing clangd, native Stop/20-second timeout and process termination, pending-request project switch, stale-result rejection, private root rejection and an actual React-terminal configured SDK build. Main/component source bytes stay unchanged and CDB bytes are restored. No JavaScript page errors were observed.

The first new run failed because the test selected a non-button file tab as a button; the app had opened the component correctly as read-only. The locator was corrected to the existing close-tab control and the failed report is retained. Two subsequent full expanded runs passed. The older unexpected learning-view transition did not recur; its original cause is not established or declared fixed. Instrumentation is retained. Header CDB inference, unopened-project semantic completeness, installed-release/cross-platform coverage and F12 remain unqualified.

## Tests and firmware

Exact commands, exits, logs and hashes belong to `diagnosis-verification-20261019.json`, with changed files in `diagnosis-changes-20261019.json`. Frontend, Rust, Python, host-C, source-evidence, semantic, release and policy checks are separately scoped; an increase in test counts does not certify the three AI capability levels. Initial PowerShell redirected-stderr invocations returned shell exit 1 despite all Rust/Python cases passing; explicit-exit confirmation runs are retained rather than erasing the first observations.

Confirmed executions passed: 176 frontend tests in 18 files, 195 Rust library tests plus 10 example tests, 22 Python tests, five Node source-evidence tests and 490 host-C assertions. The release frontend/policy build and offline locked Rust release check returned exit 0. The first release build log records successful Vite completion but its tool session exit was unavailable after continuation; a separate confirmation build returned exit 0. A fresh production-browser probe also passed citation rendering, review-bound Keep and the board guide with no page errors or external requests. Its native/provider responses are explicit fixtures, so it is UI evidence, not actual provider/native quality evidence.

Nine Block IDs/schema v1 and the two exact pilot profiles remain unchanged. New native negative cases reject incorrect scalar types, nested unknown fields and malformed/aliased profile names. `firmware-diagnosis-20261019.json` records four fresh actual ESP-IDF 5.5.4/esp32 builds, with main object compilation and ELF relinking, source/CDB/log/firmware hashes and raw exit 0. These are deterministic fixtures, not model-generated C or physical tests. No Flash occurred. Approval-gated bounded repair remains implemented but is not reclassified as successful real-model repair.

## Real Vibe Coder status

Read-only loopback discovery in a network-permitted session found no ready service at the three checked endpoints; discovery is not exhaustive. No explicit endpoint/model inference authorization was received. No LLM inference, paid-provider credential access, approved model patch, model-generated build, real-model repair/citation grading or hardware test was executed. Existing evaluation harness/preflight/source checks are infrastructure evidence. All Knowledge-aware, Project-aware and Build-aware quality gates remain NOT_EVALUATED with a real LLM.

## Security and release readiness

`dependency-security-diagnosis-20261019.json` records a fresh successful OSV lookup of 753 public crate/version entries and a successful public npm audit. Whole-lock Rust findings remain ten advisory records across eight packages; the selected Windows normal/build graph has only the two maintenance notices (`fxhash`, `paste`) and no non-maintenance matches at lookup time. npm reports zero advisories. These are audit observations, not a penetration test or universal reachability proof. Other-platform findings and maintenance migrations remain. Initial sandbox network denials and successful permitted retries are preserved. No dependency version or lockfile changed.

NSIS 3.7.3 must remain 316,758,944 bytes with SHA256 `d09d0f9418d94059e801fc43a38241314757c686e5060e26596a806ba56ae7c1`. No new installer, clean-VM lifecycle test, version promotion, RC, staging, commit, push, publication or deployment is performed. Meaningful real-model quality and independent judgments remain release blockers despite working deterministic desktop/firmware paths.

`release-preservation-diagnosis-20261019.json` verifies the actual installer hash and all eight production embedding files against the preserved 3.7.3 extraction. Source manifests, per-run compiled-source digests and separate firmware/resource hashes distinguish the current unreleased tests from the historical installer.

## Original requirements and remaining blockers

| Original requirement | Status | Remaining completion evidence |
|---|---|---|
| System architecture | PARTIAL | Wider OS/storage/recovery fault matrix and hostile race qualification |
| Hybrid RAG | PARTIAL | Reviewed development/independent evaluation, acceptable Thai/English grounding and scale |
| Knowledge-base data | PARTIAL | Versioned authoritative SDK coverage and independently reviewed relevance |
| Code-aware chunking | PARTIAL | Broader semantic/header/unopened-project correctness and platform evidence |
| Compiler validation/Auto-fix | PARTIAL | Actual approved model generation, diagnostics, bounded repair and recovery sessions |
| Technology integration | PARTIAL | Real-provider end-to-end quality, installed-release/VM/cross-platform qualification |
| Development phases | PARTIAL | All three capability gates before board/Block expansion or RC |
| Evaluation quality | PARTIAL | Human-reviewed labels/claim support and actual LLM/physical/clean-VM results |

Without a reviewer, ranking changes stay blocked by the user's instruction. Without an authorized model, AI quality metrics stay null. The draft dataset, deterministic tests, successful compiler invocations and synthetic native bootstrap events do not fill those gaps.
