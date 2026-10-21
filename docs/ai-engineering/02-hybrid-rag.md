# Hybrid retrieval

The old path returned keyword matches before trying dense search. The new `knowledge_search` ingests changes, obtains a local query vector when enabled/available, searches lexical and dense channels, then fuses ranks. No private code is sent to an embedding API.

## Lexical and semantic channels

SQLite FTS5 uses BM25 with `unicode61` and underscores retained in identifiers. MATCH terms are quoted literals; raw query operators are not executed. A separate trigram index supplies Thai substring matches, without claiming Thai word segmentation. Queries are limited to 4,000 characters / 24 lexical terms.

Dense retrieval uses the already bundled MiniLM model, pinned revision, mean pooling and 384 dimensions. Non-finite, zero, wrong-size or wrong-count embeddings are rejected. Legacy vectors with unknown model identity are excluded until rebuilt from source. This implementation has one fixed local embedding provider, not a general provider/plugin registry.

The model is retained for installation compatibility, not declared the best multilingual model. Equal dense/lexical weights degraded the small smoke dataset. The final default lowers dense weight to 0.15; this is a development-set adjustment, not independent validation. Thai retrieval quality remains insufficient. Two multilingual candidates are now measured on a frozen separate dataset; the bundled model is unchanged.

## Fusion and filtering

Each candidate receives `weight / (rrf_k + one_based_rank)` per channel. Default `rrf_k=60`, lexical weight 1, dense weight 0.15, candidate count 40, final count 5. The Thai trigram channel has half the lexical weight. Exact underscore-containing API queries prioritize code records and explicitly declared build-tested recipes, then BM25. No learned reranker is installed.

Board/target/SDK filters run in SQL before candidate limits. Values are exact declarations; SDK `5.5` and `5.5.4` are not silently treated as identical. `include_unknown=false` excludes missing fields for requested constraints. Default unfiltered search includes unknown metadata; it must not be presented as hardware/version-compatible proof.

Public-only policy is internal to the backend and cannot be disabled through tool JSON. Remote use without private-context consent returns only hash-matched bundled sources. Added/changed files are private even when named like bundled files.

Duplicate chunk content is collapsed. Results include stable ID, file, original line/parent range, source metadata, content hash, index timestamp, method, RRF score and citation. RRF score is a rank-fusion score, not cosine confidence. Result JSON is bounded by a character limit; a clipped excerpt must be read at its cited source for complete code.

## Evaluation

See [evaluation](08-evaluation.md), [lexical report](evaluation-lexical.json) and [real local hybrid report](evaluation-hybrid.json). A query with no matching lexical/dense evidence returns an empty result; the old unrelated full-document fallback was removed.

Primary implementation reference: [SQLite FTS5](https://www.sqlite.org/fts5.html), consulted 2026-10-17. It documents BM25 ranking, tokenizers and external-content indexing. No external documentation corpus was automatically downloaded in this increment.

## Frozen comparison and optional routing

The 2026-10-18 comparison uses 69 separate queries and three language groups plus exact-symbol probes, with predeclared unchanged RRF weights. See evaluation-heldout-comparison.json and its freeze manifest. It compares lexical, current hybrid, a query-dependent pure-identifier lexical route, multilingual E5 and multilingual MiniLM. The route changes no metadata filters or context budgets and remains disabled by default (`query_routing=false`): its measured first-hit ranking did not improve. E5 query/passage prefixes and model-specific vector-cache identities are evaluation-only; no candidate path/identity is exposed through application settings. This prevents accidentally searching bundled vectors with an unrelated model.


## Subsequent fixed technical expansion

The earlier model/routing comparison is historical. technical_expansion=true is now the public KB default after the separate 18-case development gate; explicit false is an opt-out through the existing configuration. Private project symbol search disables it. Original query/history and its single dense embedding are preserved. A second lexical channel adds at most three fixed concepts/six source-verified terms with 0.35 * lexical_weight before existing fusion/deduplication/context limits. Thai/English/mixed detection is script-based, not general intent understanding. Pure underscore-containing ASCII APIs bypass expansion.

Eight concepts and immutable source evidence live in intelligence/query_expansion.rs: delay, button, buzzer, temperature, light, matrix, I2C and RTC. No generated API, inferred pinout, board alias or SDK-version normalization is introduced. All channels use the same public/board/target/exact-SDK SQL filters before candidate limits. Prose nicknames do not automatically become constraints, and aliases do not establish hardware compatibility. Strict filters may return nothing because metadata is incomplete.

The plan/development decision and overlap correction are retained in 16-integration-evaluation-plan.md and expansion-development-decision.json. One shared API control was replaced and both development modes rerun before deciding. An early frozen baseline process had launched, but its results were not inspected for that decision. No vocabulary/weights/labels were tuned after reading frozen scores. Recall@5: overall 0.3623 -> 0.3841, Thai 0.0500 -> 0.1250; English/mixed recall and nine API rankings unchanged. English MRR decreases 0.2417 -> 0.2167. No subsequent tuning/model replacement; independent human annotation remains missing.
