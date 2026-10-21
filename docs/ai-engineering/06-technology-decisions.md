# Technology decisions and configuration

## ADR 1: extend SQLite instead of a server

Keep existing bundled SQLite and manual local cosine search; add FTS5/BM25 and metadata tables. This avoids Qdrant/server installation and keeps offline Windows distribution. Dense search streams rows with bounded retained candidates; complexity remains O(number of vectors), so very large corpora need future indexing work. FTS migrations/triggers are verified by tests.

## ADR 2: preserve embedding/provider stack

Keep the bundled MiniLM ONNX resources, existing fastembed, provider abstraction and native credential storage. No paid/provider API was used during evaluation. Actual retrieval results determine limitations; accepting Thai strings or producing 384 finite values is not multilingual relevance certification. See the [upstream model card](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) and pinned local manifest for identity/license.

A multilingual replacement needs a held-out Thai/English/mixed dataset, download size/latency/RAM measurements, model/license review, tokenizer/dimension integration, rebuild behavior and standalone installer checks. It is not installed merely because it is a candidate.

## ADR 3: pinned native syntax parser, lexical fallback

Tree-sitter 0.27.1 and C grammar 0.24.2 are now exact-pinned and statically compiled with Windows MSVC. Preserve the scanner as fallback and reuse the existing Engine. Clangd is evaluated in a separate ignored cache; it is not bundled or enabled as a live editor service. See [code intelligence](04-code-intelligence.md).

## ADR 4: hashes and privacy

Keep direct `sha2 = 0.10` for content/revision identities. This increment also adds the exact-pinned grammar/runtime and existing native Windows bindings needed for Job Objects. SHA256 supplies stable source/chunk identity. Native settings persist consent; no new credentials, cloud embeddings, Python service or infrastructure. Public-only knowledge is selected by a compiled reviewed hash manifest, not by file name or imported verification labels.

## Available configuration

Tauri commands `get_retrieval_config` / `set_retrieval_config` read/write the `knowledge_retrieval` object through existing atomic settings. Unspecified fields default; unknown fields and invalid limits are rejected.

```json
{
  "top_k": 5,
  "candidate_k": 40,
  "rrf_k": 60.0,
  "lexical_weight": 1.0,
  "dense_weight": 0.15,
  "max_context_chars": 12000,
  "chunk_chars": 1600,
  "batch_size": 16,
  "dense_enabled": true,
  "query_routing": false
}
```

Limits: top-k 1..20; candidates top-k..200; RRF k 1..1000; weights 0..10 and finite with at least one positive; context 512..32000 characters; chunks 256..8000 characters; batches 1..64. A whole unusually long source line remains intact in storage but context is clipped to its budget. Batching is at the ingestion callback level.

The advanced fields have backend configuration commands, not a new full settings editor. One local embedding provider/model (384 dimensions), SQLite backend, no learned reranker and the existing app-selected KB/index location are fixed. Unknown provider/backend/dimension/auto-fix settings are rejected rather than displayed as working options. No configurable logging level or unified whole-conversation token budget was implemented.

Private context has a visible checkbox in AI Provider Settings, backed by `get_private_context_consent` / `set_private_context_consent`. Default is false. Loopback endpoint parsing uses the URL hostname/IP, not substring matching; remote LAN endpoints also need consent. Initial remote calls without consent discard prior tool/assistant context and use the latest user message. Revocation signals the existing cancellation flag. Data already sent cannot be retracted.

## ADR 5: keep the installer model after measurement

Two pinned public quantized 384-dimensional mean-pooled candidates were downloaded only to `src-tauri/target/retrieval-candidates`, verified against published ONNX LFS hashes, and loaded locally without remote code. E5 uses query/passage prefixes. The base model licenses are [MIT for multilingual E5](https://huggingface.co/intfloat/multilingual-e5-small) and [Apache-2.0 for multilingual MiniLM](https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2). Conversion repositories do not independently declare the license in their API card; evaluation records identify this inference from the base license. No candidate is redistributed in the installer.

The fixed-weight E5 comparison improves recall on this dataset but remains far below the provisional target and uses substantially more RAM/index time. The held-out set is not a tuning source. Retain bundled MiniLM and leave optional query routing off; obtain human-reviewed external judgments before considering a release model change.

## ADR 6: patched platform/network/archive stack

A current public advisory lookup identified actionable Tauri IPC and network/archive issues. Update within Tauri v2 and compatible TLS/HTTP dependency lines; use patched ZIP 9 with a guarded extractor instead of retaining ZIP 2. Preserve Windows functionality through full tests and installer extraction/probing. Inactive Linux and maintenance findings remain explicit in [dependency security](12-dependency-security.md); no all-platform clean-audit claim is made.
