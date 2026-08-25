//! REFACTORING ROADMAP — ai_chat.rs (4189 lines → modular structure)
//!
//! This document describes the planned split of ai_chat.rs into focused modules.
//! The actual code movement is deferred to a dedicated refactor PR to minimize risk
//! (the current test coverage confirms behavior before the split).
//!
//! Proposed module boundaries:
//!
//! src-tauri/src/
//! ├── ai_chat.rs              ← current monolith (kept intact for now)
//! └── ai/
//!     ├── mod.rs              ← public re-exports + module docs (this file)
//!     ├── config.rs           ← L120-L560: config_path, read_config, write_config,
//!     │                          config_dir, get_secure_key, set_secure_key,
//!     │                          all get_*/set_* Tauri commands
//!     ├── system_prompt.rs    ← L1-L1910 (const SYSTEM_PROMPT)
//!     ├── providers/
//!     │   ├── mod.rs
//!     │   ├── openai.rs       ← L2111-L2595: run_conversation_loop, get_tools
//!     │   └── google.rs       ← L2596-L2977: run_google_conversation_loop,
//!     │                          get_google_tools, build_google_contents
//!     ├── tools/
//!     │   ├── mod.rs
//!     │   └── executor.rs     ← L3042-L3480: execute_tool, build_file_tree,
//!     │                          search_files_recursive, compute_unified_diff
//!     └── kb/
//!         ├── mod.rs
//!         └── search.rs       ← L3529-L4000: knowledge_search, keyword_search,
//!                                vector_search, reindex, collect_kb_files, chunk_text
//!
//! Status: PLANNED (next sprint)
//! Blocker: All 18 unit tests must pass with 0 regressions after the split.
//!
//! Migration plan:
//! 1. Move config.rs first (self-contained, no cross-module deps)
//! 2. Move system_prompt.rs (just a constant)
//! 3. Move kb/search.rs (depends on config + kb_store + kb_embed)
//! 4. Move tools/executor.rs (depends on kb/search)
//! 5. Move providers/openai.rs and providers/google.rs last (depend on everything)
//! 6. Update ai_chat.rs to import from sub-modules instead of defining inline

// This file is a thin re-export layer until the full split is done.

pub mod config;
