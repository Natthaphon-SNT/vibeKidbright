# Integration evaluation plan — 2026-10-18

This plan is written before running the new frozen-set comparison. No existing frozen query, label, model or historical report may be overwritten. Previous measurements and labels are known historical evidence, not a blind new benchmark.

## Technical expansion policy

Use only eight fixed concepts taken from existing source/manual terminology: delay, button, buzzer, temperature, light, matrix, I2C and RTC. Each expansion term must occur in its compile-time source evidence. Match English aliases at identifier boundaries and Thai aliases as literal substrings. At most three concepts/six terms are added in one lexical channel. The original query, its lexical/trigram channels and its single original-query embedding remain unchanged. Pure underscore-containing ASCII identifiers bypass expansion. No guessed GPIO, SDK compatibility, board alias or inferred version rewrite is permitted.

Fusion adds `0.35 * lexical_weight / (rrf_k + rank)` for the expansion channel before the existing deduplication/context budget. All channels use the same SQL public/board/target/exact-SDK filters before candidate limits. This fixed small weight and vocabulary will not be adjusted after inspecting frozen-set outcomes. The production embedding model remains unchanged. Existing user chat/history is not rewritten.

`evaluation-expansion-dev-v1.json` contains 18 new development questions (five per prose language plus three API controls), authored from the source contracts without copying frozen queries. Labels are assistant-authored, non-exhaustive and not independently reviewed. Development acceptance: mean Recall@5 improves, no prose language loses recall, and exact-API ranked files remain identical. Otherwise leave production expansion disabled. After the development decision, run the unchanged frozen 69 questions once per baseline/expanded mode and report all outcomes, including regressions; do not tune against them.

Report per-language Recall@5, MRR, NDCG and latency, original query and expansion plan, process RAM where available, corpus/model/source/dataset hashes and exact commands. Current metrics deduplicate the top five chunks without refilling document ranks. Neither retrieval scores nor source matches prove model grounding or hardware compatibility.

## Desktop and model checks

Adapter-only and stubbed-browser checks do not prove desktop IPC. Attempt an isolated application context with its own AppData/WebView profile and real Tauri command handlers, then attach a local browser driver to the actual WebView. Do not touch user provider credentials or existing user settings. Record Go to Definition, references, unsaved buffers, headers, switching, malformed CDB, missing executable, cancellation/deadline, isolation, fallback and Monaco navigation separately. A safe fallback is a tested limitation, not a semantic success.

Probe only local model discovery endpoints. Without a ready authorized endpoint/model, real LLM quality remains NOT_EXECUTED/null. Build an application-workflow harness and public prompts, separate retrieval/citation/model/C-compiler judgments, and stop before every proposed patch until explicit user approval is recorded. No fixture output may count as real LLM quality.

Preserve the nine verified normalized Block IDs and schema v1. Execute contract/rejection tests and actual installed ESP-IDF builds, recording the tested SDK version rather than claiming all-version compatibility. No generic Blockly XML or physical certification. Preserve NSIS 3.7.3; no installer packaging or release promotion is planned until integration evidence warrants a separately identified candidate.
