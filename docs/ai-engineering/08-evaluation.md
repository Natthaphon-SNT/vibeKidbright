# Evaluation and reproducibility

## Dataset and method

[evaluation-queries.json](evaluation-queries.json) contains 12 manually labelled repository smoke queries: exact C APIs, English behavior descriptions, Thai, mixed Thai/English and a hardware identifier. Relevance is file-level. Labels are deliberately limited to the named target examples/reference files, not exhaustive judgments for all related documents. This is a development set, not a held-out quality claim.

The runner copies text/code KB sources into a temporary directory, builds a fresh SQLite index and optionally calls the actual bundled local ONNX model. It then reruns unchanged ingestion with an embedding callback that fails if invoked. No user index, private project, provider credential or hardware is used. The reports identify OS/architecture, dataset hash, model, configuration, per-query rankings/latencies and index counts.

```sh
cargo run --manifest-path src-tauri/Cargo.toml --offline --example knowledge_eval -- --output docs/ai-engineering/evaluation-lexical.json
cargo run --manifest-path src-tauri/Cargo.toml --offline --example knowledge_eval -- --dense --output docs/ai-engineering/evaluation-hybrid.json
```

The bundled runtime/model and Rust dependencies must already be prepared for `--offline`; it does not download a new model. First indexing can take minutes. Both reports must be compared using their recorded configuration rather than assuming every prior experiment used the final weights.

## Metrics

The engine returns up to five **chunks**. For file-level scoring the runner removes repeated file names from those results, without filling extra positions. Recall@5 = distinct relevant retrieved files / all labelled relevant files. Precision@5 always divides by 5, including short lists. MRR uses the first relevant distinct file. NDCG@5 uses binary relevance and an ideal ordering with at most five labelled files. Macro averages give each query equal weight.

The legacy LIKE ranker is measured on the same new chunk corpus, isolating ranking; it is not an end-to-end replay of the old index/raw-file/full-dump pipeline. Latency includes local query embedding when enabled but excludes first-time indexing. Cold index and unchanged-index latency are recorded separately. Tests using synthetic vectors verify fusion logic only; they are not counted as real embedding quality.

## Evidence and limits

Historical 2026-10-17 development-set scores (12 queries, lexical chunker; not comparable to a new dataset as a before/after improvement):

| Ranker | Recall@5 | Precision@5 | MRR | NDCG@5 |
|---|---:|---:|---:|---:|
| Legacy LIKE on new chunks | 0.8333 | 0.3000 | 0.6528 | 0.6922 |
| New lexical only | 0.8333 | 0.3000 | 0.6736 | 0.7187 |
| Local hybrid, dense weight 0.15 | 0.7917 | 0.2833 | 0.7111 | 0.7083 |

Hybrid improves first-relevant rank here but reduces recall versus lexical. It remains experimental and does not meet Recall@5 ≥0.90. Median query latency: lexical 1.74 ms, hybrid 143.70 ms. Hybrid cold indexing: 126.30 seconds for 37 files / 1,839 chunks. Unchanged refresh: 98.83 ms, 37 files unchanged and zero embedding calls. These single-run timings are not cross-machine performance guarantees.

Read [lexical results](evaluation-lexical.json) and [hybrid results](evaluation-hybrid.json) for actual measured values. Equal dense/lexical weighting initially harmed these queries; final default dense weight was lowered using this development set. Do not report the ≥90% recall target as achieved on an independent benchmark. The later 2026-10-18 comparison below evaluates two alternatives.

Source/version filter correctness is tested with deliberately incompatible SDK metadata. This is separate from real-corpus relevance, whose documents often have unknown version/board fields. Source-grounded final-answer correctness and citation accuracy from an actual LLM have not been measured.

## Test inventory

- Rust: FTS migration, API token preservation, Thai substrings, safe MATCH literals, hash freshness, empty/deleted/disabled sources, metadata updates, dense/lexical fusion, rollback under SQL failure, invalid vectors/UTF-8, corruption preservation, cancellation, bounded configuration, public-only policy and code source ranges.
- AI boundary: latest-user routing without copied source instructions, loopback URL spoof rejection, changed-document hash checks, and command shell/hardware-action rejection.
- Frontend: existing editor/review/build/Wiki tests plus explicit private-context checkbox behavior.
- Required commands: `npm test`, `cargo test --manifest-path src-tauri/Cargo.toml --offline --all-targets`, `npm run build:release` and public/bundle checks.

The execution summary is recorded in [verification.json](verification.json). The previous 3.7.2 installer validation predates this engineering increment and is not proof that these source changes are in that installer.

## Metrics not measured

Real AI-generated C compile success, API/header correctness percentages, repair success/attempts/regressions, generic Block-to-C pass rate and physical behavioral equivalence are **not measured**. A review-gated repair workflow and narrow typed converter are now implemented, but real-model repair quality and generic conversion remain unmeasured. Existing firmware recipe evidence remains separately documented in the earlier project reports.

## Frozen 69-query comparison (2026-10-18)

[evaluation-heldout-v1-freeze.json](evaluation-heldout-v1-freeze.json) freezes 20 English, 20 Thai, 20 mixed and 9 exact-symbol queries, qrels, corpus hashes and ranking parameters before evaluation. Topics/translations overlap and labels were authored by the assistant, not independent human annotators. They are separate from the 12 development queries and were not used to tune parameters. File labels identify representative targets and are non-exhaustive; broad related documents can be useful yet unlabelled. No result justifies a general answer-correctness claim.

A PowerShell native-pipe codepage error initially replaced Thai text with question marks. Those measurements are invalid and preserved in `invalid-encoding-results/`. The original intended Thai/mixed text was restored mechanically; IDs, English, qrels, candidate selection and ranker parameters were unchanged. The corrected dataset hash is `c75de0d838a5f0e6670c48776c67d3b5c0a98ca755f02ba09c2d548c28b55c43`. The runner now checks Thai characters and the freeze digest. This is an encoding correction, not score-driven tuning.

The same 37 public sources produced 2,026 syntax/paragraph chunks in every run. Every mode used a fresh process and index. Candidates are pinned quantized ONNX models, 384-dimensional mean pooling; E5 applies `query: ` / `passage: ` prefixes. All keep the predeclared RRF weights 1.0/0.15, candidate count 40 and top five chunks. The optional routing comparison uses lexical only for a pure ASCII underscore identifier; other queries retain the existing fusion.

| Ranker | Recall@5 | MRR | NDCG@5 | Median / p95 ms | Cold index s | Peak working set MiB | Model assets MiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| lexical | 0.3478 | 0.3225 | 0.3027 | 4.02 / 9.60 | 1.07 | 18.5 | 0.0 |
| bundled | 0.3623 | 0.3889 | 0.3488 | 168.82 / 192.73 | 122.32 | 370.8 | 86.9 |
| bundled-routed | 0.3623 | 0.3551 | 0.3315 | 186.60 / 230.50 | 129.33 | 371.0 | 86.9 |
| multilingual-e5-small | 0.4275 | 0.3877 | 0.3671 | 216.22 / 453.13 | 257.45 | 1353.2 | 129.1 |
| multilingual-minilm | 0.3913 | 0.3635 | 0.3343 | 152.61 / 182.91 | 309.21 | 1356.5 | 129.1 |

Full [comparison](evaluation-heldout-comparison.json) and per-ranker reports retain every query, language macro scores, indexing/unchanged-index timings, private memory sampling and model identity. Windows memory uses GetProcessMemoryInfo at 100 ms intervals; peak working set includes native ONNX. Model assets include model/tokenizer/config/manifest. Timings are single runs on this development host while other development checks could run; no warm-cache repetition, controlled-load or cross-machine performance guarantee. Index time excludes separately recorded model initialization. No embedding request left the machine.

E5 improves fixed-weight recall from 0.3623 to 0.4275 (+0.0652 absolute) on these qrels, but first-hit MRR is nearly unchanged and peak RAM/index time increase markedly. Thai-only recall remains 0.20 for E5 versus 0.05 bundled, far below target. Pure-identifier routing keeps recall unchanged and lowers MRR. Retain the bundled model and default routing=false. No candidate model/parameter was selected or retuned against this test set. More independent human-reviewed data and development-only experiments are required.

Reproduction, with dependencies already cached:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --offline --example knowledge_eval
python scripts/evaluate-retrieval.py
```

The separate `prepare-retrieval-candidates.py` downloads pinned public artifacts into ignored target caches only; it is development tooling and is not needed by the installer.

These performance runs preceded the final dependency-security refresh. The ranker, frozen data and bundled weights were unchanged by that refresh; the timings are evidence for the recorded evaluation executable, not a performance certification of the rebuilt installer.

## Other new verification evidence

- [Block compilation](block-compilation.json): both deterministic typed fixtures compile/link on actual installed ESP-IDF 5.5.4 for esp32, no Flash. This is not LLM generation accuracy.
- [clangd evaluation](clangd-evaluation.json): pinned official clangd 23.1.0 reads the real 917-entry SDK CDB. Check mode reports two SwapBinaryOperands refactoring-tweak failures, not compiler errors. Actual local LSP over two opened fixture translation units returns the correct kb_delay_ms definition and three locations, with no diagnostic errors. Header/all-project reference completeness and live editor integration are unmeasured. No false claim that the target failed to parse.
- Native file/process/repair and frontend approval/cancellation regressions are recorded in verification.json. No power-cut, every OS race, actual provider repair quality, clean-VM GUI or physical behavior is inferred.

## Continuation after the verified NSIS baseline

The frozen set, weights, routing default and production embedding model are unchanged. `review-retrieval.py summarize` post-processes the existing recorded query results; this is not a new model run or an accuracy improvement.

| Ranker | Thai Recall@5 (20) | English (20) | Mixed (20) | Exact API (9) | Hardware (33) |
|---|---:|---:|---:|---:|---:|
| lexical | 0.0000 | 0.2250 | 0.5250 | 1.0000 | 0.2727 |
| bundled MiniLM | 0.0500 | 0.2500 | 0.5000 | 1.0000 | 0.3030 |
| bundled routed | 0.0500 | 0.2500 | 0.5000 | 1.0000 | 0.3030 |
| multilingual E5 | 0.2000 | 0.3000 | 0.5250 | 1.0000 | 0.3939 |
| multilingual MiniLM | 0.1000 | 0.2500 | 0.5500 | 1.0000 | 0.3030 |

Hardware overlaps language groups and uses the 11 declared board/peripheral topics; it is not an all-board test. [The stratified report](retrieval-stratified-review.json) includes MRR/NDCG/precision, category latency and process-level RAM/index/model bytes. Only the preserved actual NSIS size (316,758,944 bytes) is known; no candidate installer size or compression savings is inferred from ONNX bytes. Annotation uncertainty and the blank independent human review packet are documented in [the review protocol](13-independent-evaluation.md).

The new [semantic adapter report](semantic-adapter-evaluation.json) contains six real native queries across two SDK-generated projects, with unsaved definitions and no source writes. [Block expansion](block-expansion-compilation.json) records two new actual SDK builds and byte-identical regenerated old fixture C. [Real LLM evaluation](real-llm-evaluation.json) remains NOT_EXECUTED: the user confirms no local model; no paid authorization was given. [The development production-UI probe](production-workflow-development.json) remains fixture-backed. The preserved installer probe is distinct from these new source changes.


## Subsequent paired expansion evaluation

query-expansion-comparison.json records new runs of the unchanged 69-query dataset with the same bundled MiniLM. Eighteen separate development cases determined the default; the corrected control overlap/protocol deviation is disclosed in expansion-development-decision.json. No vocabulary/weight/label was adjusted from final scores. These are agent-authored, non-exhaustive file labels, not independent human validation.

| Group | Recall@5 before -> after | Median ms before -> after | p95 ms before -> after |
|---|---|---|---|
| Thai (20) | 0.0500 -> 0.1250 | 289.1 -> 132.7 | 343.4 -> 164.0 |
| English (20) | 0.2500 -> 0.2500 | 329.5 -> 151.7 | 387.9 -> 166.4 |
| Mixed (20) | 0.5000 -> 0.5000 | 330.2 -> 162.1 | 425.7 -> 192.3 |
| Exact API (9) | 1.0000 -> 1.0000 | 293.0 -> 136.0 | 355.4 -> 143.3 |
| Overall (69) | 0.3623 -> 0.3841 | Per-group above | Per-group above |

Overall MRR 0.3889 -> 0.4010, NDCG@5 0.3488 -> 0.3616. Two query recalls improve and none regresses. English MRR 0.2417 -> 0.2167 and NDCG 0.2101 -> 0.1988 regress. Uncontrolled single-host timing with concurrent tasks is diagnostic, NOT a speedup attributable to expansion; no new RAM sampling was captured. Every query, expansion plan and source/dataset evidence is retained. Original embedding query and all nine exact API rankings remain unchanged.

Desktop evidence is now real WebView2/production React/CSP/Tauri/clangd/Monaco with fresh data and zero native stubs (desktop-semantic-integration.json), with explicit header/open-TU limitations. The application LLM harness preflight passes actual IPC but sends zero model requests and leaves grades null. block-integration-regression.json records four fresh ESP-IDF 5.5.4 recompilations/relinks, source regeneration equality and 490 portable C assertions; no Flash/physical claim.

## Diagnosis and fresh fixed-policy reproduction

See retrieval-diagnosis-20261019.json for all paired query/rank deltas and query-kind coverage. The twenty-question development review packet is unreviewed; the user requests no ranking changes. No diagnosis flag is used as a new training label. The final set contains zero compiler-error queries, and their metrics are null.

The existing evaluator now requires a fresh output directory and explicit client date, reserves individual Rust reports before loading models, and records richer returned-source provenance. Reproduce fixed modes with `python scripts/evaluate-retrieval.py --output-directory src-tauri/target/new-retrieval-run --modes lexical bundled bundled-expanded multilingual-e5-small multilingual-minilm --date YYYY-MM-DD` after building the existing knowledge_eval example. Paths must be fresh; historical reports are preserved. The production MiniLM with fixed expansion remains the default. Fresh resource/stratified results are in retrieval-fixed-measurement-20261019.json; they are not independent human evaluation or model-promotion evidence. Single-host process RAM/index/query latency must not be interpreted as a controlled speedup or compressed installer size.


## Mechanical evidence acceptance

The real-Vibe harness now rejects malformed retrieval payloads, invalid source hashes/ranges, unmatched literal citations, empty answers and missing case-specific requested tools/cited hits. CAPTURED remains ungraded mechanical evidence. Factual support, alternate citation formats and real model quality still require review. The prepared report has zero inference; synthetic unit traces and native recorder preflight are not model results. Prior five-mode retrieval metrics are referenced with unchanged-source/model hashes, not rerun or claimed as new gains. See 22-evidence-integrity-report.md.


## Hash-bound capability review

The existing real-Vibe harness embeds a capability matrix; the offline evaluate-capabilities CLI binds exact file bytes and validates reviewer/manual/mechanical evidence references. CAPTURED is never automatic AI-quality PASS. Independent review is self-attested, not authenticated, and raw actual-model review outputs remain under ignored target. The current matrix has no actual model execution or human judgments. Prior retrieval metrics are not rerun or relabelled. See 23-capability-validation-protocol.md.
