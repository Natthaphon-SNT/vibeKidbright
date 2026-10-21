# Knowledge records and ingestion

Reuse the per-user KB directory and `.kb_store.sqlite`; do not create a second service/database engine. Supported source files: UTF-8 Markdown, text, C, headers and CMake text. PDFs/images remain reference resources and are not silently OCR-indexed.

## Storage

| Table | Purpose |
|---|---|
| Existing `kb_chunks`, `kb_meta` | Original content/vector storage and schema marker |
| `ki_sources` | Relative source path and content/metadata/chunker/model/config fingerprint |
| `ki_chunks` | Stable SHA256 ID, source/line metadata, hash, model identity and public-source provenance |
| `ki_embeddings` | Content-addressed local embedding cache, keyed by model identity |
| `ki_fts`, `ki_trigram` | External-content FTS5 indexes synchronized by triggers |

Schema installation and each source replacement use SQLite transactions. The legacy store replacement method is also transactional now. Migration preserves legacy rows, but the new retrieval engine requires provenance-bearing records and rebuilds those from active sources; it does not assume old JSON/vector caches use the current model. A corrupt database is reported and preserved, not silently deleted. Unsupported schema markers fail with instructions to preserve/rebuild elsewhere.

## Source pipeline

Discover sorted relative paths → read bounded UTF-8 → normalize line representation in chunks → validate → structural/paragraph chunking → metadata extraction → hash/deduplicate/cache → local batch embedding → transactional index → remove records for deleted/disabled sources.

Every refresh reads source bytes to catch edits that preserve mtimes. Only changed files are rechunked and embedded. Empty files remove their previous chunks. Removing the last active file clears indexed content. Embeddings no longer referenced by active records are pruned. No disk source is deleted by ingestion.

Limits: 2 MiB per source, 2,048 files and depth 12. Hidden files/directories, links, build/target/node_modules and disabled extensions are skipped. Hashes are raw-file SHA256; CRLF changes can require reindexing. Canonical workspace access remains the responsibility of existing file tools; this is not an OS sandbox or protection against every filesystem race.

## Metadata

Each hit supports title, category, source URL, language, framework, component, exact SDK declaration, board, target, verification and license. Missing properties remain null/unknown. Source path and hashes always come from ingestion. A label supplied by an imported metadata file is a declaration, not independent certification.

Maintainers' [recipe declarations](knowledge-sources.json) apply only when source hash matches. They cover four existing build-tested recipes. [Bundled source hashes](bundled-source-hashes.json) separately identify public documents; the packaging check verifies those hashes. These two provenance manifests normalize CRLF to LF so Git checkout line-ending conversions on Windows/macOS preserve identity. Raw-file freshness and chunk-content hashes remain separate. New or edited bundled text requires a reviewed manifest update.

Users may place an optional `.knowledge-sources.json` in their KB directory. It maps relative paths to metadata objects, for example:

```json
{
  "my-guide.md": {
    "title": "Team guide",
    "category": "documentation",
    "language": "th",
    "sdk_version": "5.5",
    "target": "esp32",
    "verification": "unverified-reference"
  }
}
```

## Four categories and remaining work

Documentation/code use the ingestion records. Hardware reuses the existing versioned board catalog and executable-profile guard. Block knowledge reuses the historical inventory and explicitly scoped reference recipes; no general block conversion mapping/translator is claimed. A complete official ESP-IDF/FreeRTOS corpus, all peripheral topics, licensing review per external source, and automated fetching remain unfinished. The typed normalized subset expands from four to nine IDs in the unreleased source; its exact schema is available through the converter tool without program_json. See 10-block-to-c.md.

## Project isolation (2026-10-18)

KB discovery now also skips Windows reparse points. The private project syntax index uses a distinct per-project AppData database and only main/components C/H. It never promotes project bytes to public provenance or ingests a project's arbitrary knowledge sidecar as public metadata. Its local retention and on-query refresh limits are documented in 04-code-intelligence.md and 09-known-limitations.md.
