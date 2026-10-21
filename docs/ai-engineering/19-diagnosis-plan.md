# Retrieval diagnosis and semantic hardening plan — 2026-10-19

This is a new unreleased increment after the matched 216-file workflow manifest. Historical NSIS 3.7.3, prior reports, frozen 69 query/qrel bytes, bundled corpus, embedding assets and current production ranker are preserved. No ranking change may be accepted from diagnosis of the final labels.

## Diagnosis and review boundary

Analyze the existing before/after expansion runs, recomputing their frozen metrics and reporting every query, duplicate document slots, first-relevant-rank regression and missing labels/source files. Separate exact symbols, natural language, mixed language, compiler errors and previously declared hardware topics; categories overlap. Zero compiler-error coverage is a gap, not a zero-quality metric. Report language and ranking/resource measurements separately. Qrels remain assistant-authored, non-exhaustive and uncertain.

The twenty-question `retrieval-development-review-20261019.json` is a source-contract-based **assistant draft**, disjoint by normalized text/ID from the frozen/prior development sets. It has no rankings or suggested qrels and requires complete 0/1/2 judgments for all frozen corpus files, review notes and an identified independent reviewer. Human review is not currently supplied. It is not an independently authored evaluation set. The existing review tool must reject drafts and leaked ranker results. Do not change expansion vocabulary, decomposition, routing or reranking until this gate is passed.

The user explicitly confirmed that no reviewer is available and requested keeping the existing ranking. No ranking change will be made in this increment; the development packet remains unreviewed and unexecuted.

Conservative candidates to investigate only after review: distinguish API-contract lookup from broad peripheral instructions; decompose multi-part requests into bounded source-verified subqueries while keeping the original channel; rerank using actual declarations/definitions and version metadata. Do not assume those candidates improve results. No API or compatibility term may be generated without source evidence. Proposed scoring rules must be fixed using reviewed development data before a final evaluation.

Future candidate rejection policy: exact-API Recall@5/MRR/NDCG must not decrease; reject English Recall@5/MRR/NDCG regressions exceeding either 5% relative or 0.01 absolute. Check Thai/mixed-language benefit, latency/RAM/index costs and the independent review limitations as well. These gates do not retroactively certify the existing default or select new weights from final results. No candidate/model promotion is authorized by this plan.

## Fixed-policy measurement

With no ranking/model changes, measure these predeclared modes in sequence with fresh per-process indexes: lexical, bundled MiniLM without expansion, current bundled MiniLM with fixed expansion, existing multilingual E5-small, existing multilingual MiniLM. Keep the frozen 69 set and source corpus unchanged. This is reproduction/resource measurement, not training/tuning or independent annotation. Route comparison already has historical evidence; no route candidate is adopted here.

Use the existing Rust evaluation example and Python resource runner, hardened to reserve new outputs before loading models. Record exact commands/exits, source/binary/dataset hashes, per-query returned source metadata, per-language Recall@5/MRR/NDCG and latency, process working set/private bytes, cold/unchanged indexing time and model bytes. Candidate model bytes are not compressed installer sizes. Run one model at a time after CPU-heavy build/test jobs finish. Host load remains uncontrolled; do not claim statistical significance or attribute speedups solely to ranking.

## Desktop, firmware and release

Expand the existing debug-native production-React/Tauri/clangd/Monaco probe with header-reference fallback and opened cross-component definition/reference navigation. Clearly label the synthetic existing native event used only to bootstrap an additional read-only component tab; actual semantic calls/results remain native. Observe click/view/navigation transitions without intercepting application functions. Preserve failed runs. No F12 provider while navigation stability remains uncertain; no header command inference or unopened-project completeness claim.

Retain nine IDs/schema v1 and exact board restrictions. Add negative contract regressions and fresh representative configured ESP-IDF builds without Flash. Keep model patches and each repair attempt behind existing approval gates. No live inference unless an explicitly authorized endpoint/model becomes available; discovery alone is not authorization. Run frontend/Rust/Python/host-C/release/policy/security checks and preserve all historical evidence. No new installer/RC, staging, commit, push, publication or deployment.
