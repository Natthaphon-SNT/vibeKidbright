# Repository audit — 2026-10-17

This document preserves the pre-implementation audit and decisions from that date. Its gap list is historical; read 07-implementation-progress.md and verification.json for the 2026-10-18 implementation and executed checks.

Scope: inspected the existing README, package/Cargo manifests and lockfiles, Tauri configuration/command registration, AI orchestration/providers/tools/configuration, KB embedding/store/search, compatibility profiles, project review, editor/build UI, diagnostics, tests, and release workflows before implementation. This is a source audit, not a hardware certification.

## Existing implementation to reuse

| Area | Existing implementation | Integration decision |
|---|---|---|
| Desktop/UI | Tauri 2, React 19, TypeScript, Monaco; `lib.rs`, `App.tsx` | Preserve framework/layout and current providers |
| AI orchestration | `ai_chat.rs`, `ai/providers.rs`; OpenAI-compatible/Google streaming, abort state, 20 tool turns | Extend context/tool integration, preserve streaming |
| Workspace access/review | `ai/tools.rs`, `ai/project_review.rs`; canonical path checks, pending diffs, project Keep/Undo | Reuse gates; no new automatic writes |
| Firmware build | `esp_idf.rs`, `buildFlash.ts`; configured Python/idf.py with separate arguments; learner edits saved before build | Preserve trusted build route |
| Diagnostics | `errorHints.ts`, `BuildErrorList.tsx`; source/CMake errors and user-triggered AI assistance | Reuse UI; controlled build/fix automation remains separate work |
| Knowledge | `ai/kb.rs`, `kb_store.rs`, `kb_embed.rs`; local ONNX, SQLite, legacy JSON migration | Extend existing store, avoid external server |
| Hardware | `ai/compatibility.rs`, board catalog, deterministic reference recipes | Keep explicit profiles and unsupported statuses |
| Settings/privacy | Native keyring, serialized atomic config, per-user AppData | Reuse persistence; no remote embeddings |
| Release | Windows/macOS workflows, offline runtime preparation, public/bundle checks | Preserve standalone installation |

## Gaps observed before changes

- Search is keyword-first fallback, not simultaneous hybrid fusion. Keyword SQL uses LIKE, not BM25. Unmatched queries can return full unrelated documents.
- Index freshness uses second-resolution mtimes. Empty/deleted files and disable operations can leave stale records/cache. Replacement deletes before insertion without a transaction.
- No per-chunk provenance/version/hash/line metadata; model identity is not tied to stored vectors. Legacy embedding identity is unknown.
- C files use arbitrary character windows. There is no installed C AST parser or semantic project index.
- RAG work includes synchronous filesystem/model work in async paths. Current vector search retains every matching content row before truncation.
- No reproducible multilingual retrieval benchmark, no measured multilingual model comparison, no review-gated bounded compiler auto-fix state machine.
- General AI `run_command` invokes a shell. Source path validation does not sandbox that shell. Existing process/review protections do not constitute a full command policy.
- Per-message Undo and pending review recovery across restarts need further work. Do not expand automatic patch application on these assumptions.

## Integration and constraints

Add versioned source/chunk records and FTS5 migration to the existing SQLite store. Add incremental, local-only ingestion and RRF retrieval behind `knowledge_search`. Preserve its result-array contract and add citations/metadata. Move indexing/search off the async worker. Classify requests without treating retrieved text as instructions. Supply bounded reference context through the existing provider abstraction.

Code structural indexing will begin with a documented conservative C lexical scanner using the existing regex dependency. Tree-sitter C was evaluated against the official parser documentation; no Tree-sitter crate is cached in this workspace. A lexical scanner is not an AST or semantic reference resolver. Full parser/project indexing and compiler auto-fix require separately verified integration; track these explicitly rather than adding placeholders.

Reuse bundled SQLite/ONNX. The only planned direct dependency addition is SHA-256 (`sha2` 0.10, already in Cargo.lock transitively). No Qdrant service, cloud embedding, new credentials, production infrastructure or hardware writes are required.

The bundled MiniLM model must remain identifiable as the existing model; Thai input acceptance alone is not evidence of multilingual retrieval quality. Evaluate real local inference separately from deterministic tests. Historical block catalogs remain reference material, not executable mappings.
