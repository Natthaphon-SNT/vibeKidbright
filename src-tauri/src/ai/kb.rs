//! Local-only incremental ingestion and true lexical+dense fusion.
use crate::ai_chat::resolve_kb_path;
use crate::intelligence::{Engine, RetrievalConfig, SearchFilter};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, Arc};
use tauri::{AppHandle, Emitter, Manager};

pub(crate) fn collect_kb_files_inner(root: &Path, current: &Path, result: &mut Vec<(PathBuf, String)>, include_disabled: bool) {
    if current.components().count().saturating_sub(root.components().count()) > 12 || result.len() >= 2048 { return; }
    let Ok(entries) = std::fs::read_dir(current) else { return; };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let Ok(kind) = entry.file_type() else { continue; };
        if kind.is_symlink() { continue; }
        let path = entry.path();
        if kind.is_dir() { collect_kb_files_inner(root, &path, result, include_disabled); }
        else if kind.is_file() {
            let extension = path.extension().and_then(|s|s.to_str()).unwrap_or("");
            if matches!(extension, "md" | "txt" | "c" | "h" | "cmake") || (include_disabled && name.ends_with(".disabled")) {
                if let Ok(relative) = path.strip_prefix(root) { result.push((path.clone(), relative.to_string_lossy().replace('\\',"/"))); }
            }
        }
    }
}
pub(crate) fn collect_kb_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new(); collect_kb_files_inner(root, root, &mut files, false); files.sort_by(|a,b|a.1.cmp(&b.1)); files
}
pub(crate) fn collect_kb_files_all(root: &Path) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new(); collect_kb_files_inner(root, root, &mut files, true); files.sort_by(|a,b|a.1.cmp(&b.1)); files
}

pub(crate) fn retrieval_config() -> Result<RetrievalConfig, String> {
    let config = super::config::read_config();
    let settings = config.get("knowledge_retrieval").cloned().unwrap_or_else(|| json!({}));
    let settings: RetrievalConfig = serde_json::from_value(settings).map_err(|e|format!("Invalid retrieval settings: {e}"))?;
    settings.validate()?; Ok(settings)
}
#[tauri::command]
pub async fn get_retrieval_config() -> Result<RetrievalConfig,String> { retrieval_config() }
#[tauri::command]
pub async fn set_retrieval_config(settings: RetrievalConfig) -> Result<(),String> {
    settings.validate()?;
    super::config::update_config(|value| value["knowledge_retrieval"] = json!(settings))
}

pub(crate) async fn reindex_knowledge_base(project: &Path) -> Result<usize, String> {
    let root = resolve_kb_path(&project.to_string_lossy());
    let config = retrieval_config()?;
    if !root.is_dir() { return Ok(0); }
    tokio::task::spawn_blocking(move || {
        let engine = Engine::open(&root, config)?;
        let report = engine.ingest(&root, &AtomicBool::new(false), crate::kb_embed::embed_texts)?;
        if !report.warnings.is_empty() { eprintln!("[KB] {} files indexed with dense lookup unavailable.", report.warnings.len()); }
        Ok(report.chunks)
    }).await.map_err(|e|e.to_string())?
}

/// Backward-compatible tool result array, enriched with stable IDs, lines and
/// version/provenance metadata. No full-document dump or cross-project query cache.
pub(crate) async fn knowledge_search_filtered(app: &AppHandle, project: &Path, query: &str, filter: SearchFilter) -> Value {
    let root = resolve_kb_path(&project.to_string_lossy());
    if !root.is_dir() { return json!({"message":"No knowledge_base folder found."}); }
    let config = match retrieval_config() { Ok(c) => c, Err(error) => return json!({"error":error}) };
    let cancelled = app.try_state::<crate::ai_chat::AiAbortState>().map(|s|s.0.clone()).unwrap_or_else(||Arc::new(AtomicBool::new(false)));
    let query = query.to_owned();
    let status_app = app.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<Value,String> {
        if query.trim().is_empty() { return Ok(json!([])); }
        if query.chars().count() > 4000 { return Err("Retrieval query exceeds 4000 characters.".into()); }
        let engine = Engine::open(&root, config)?;
        let report = engine.ingest(&root, &cancelled, crate::kb_embed::embed_texts)?;
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) { return Err("Knowledge search cancelled.".into()); }
        let vector = if engine.config.needs_dense(&query) { crate::kb_embed::embed_query(&query).ok() } else { None };
        let results = engine.search(&query, vector.as_deref(), &filter)?;
        // Status goes to the local UI, not into the bounded model excerpts.
        let _ = status_app.emit("terminal-output",format!("[Knowledge] {} chunks, {} updated files, {} results; dense={}, index warnings={}; public references only={}.",report.chunks,report.files_updated,results.len(),vector.is_some(),report.warnings.len(),filter.bundled_only));
        Ok(json!(results))
    }).await;
    match result { Ok(Ok(value)) => value, Ok(Err(error)) => json!({"error":error}), Err(error) => json!({"error":format!("Knowledge worker failed: {error}")}) }
}
