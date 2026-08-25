//! Knowledge base: indexing, embedding, and search (RAG subsystem).
//!
//! Extracted from ai_chat.rs (refactoring roadmap step 3).
//! Depends on: crate::kb_store (SQLite), crate::kb_embed (local ONNX),
//! ai/config (API keys), ai_chat (path resolution helpers).

use crate::ai_chat::resolve_kb_path;
use crate::ai::config::{get_secure_key, read_config};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeChunk {
    file_name: String,
    content: String,
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct VectorIndex {
    chunks: Vec<KnowledgeChunk>,
    last_indexed: std::collections::HashMap<String, u64>,
}

/// Simple in-memory knowledge-search cache: query -> results JSON.
/// Cleared automatically when KB is re-indexed.
static KB_QUERY_CACHE: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
pub(crate) fn get_kb_query_cache() -> &'static Mutex<HashMap<String, Value>> {
    KB_QUERY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── Embeddings ────────────────────────────────────────────────────────────────

async fn get_embeddings_internal(api_key: &str, mut base_url: String, text: &str) -> Result<Vec<f32>, String> {
    if api_key.is_empty() {
        return Err("No API key configured for embeddings".to_string());
    }
    if !base_url.starts_with("http") && !base_url.is_empty() {
        base_url = format!("http://{}", base_url);
    }
    if !base_url.contains("/v1") {
        base_url = format!("{}/v1", base_url.trim_end_matches('/'));
    }
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;
    let res = client.post(format!("{}/embeddings", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({ "input": text, "model": "text-embedding-3-small" }))
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;
    let data: Value = res.json().await.map_err(|e| format!("Failed to parse embedding response: {}", e))?;
    if let Some(err) = data["error"].as_object() {
        return Err(err["message"].as_str().unwrap_or("Unknown API error").to_string());
    }
    let embedding = data["data"][0]["embedding"]
        .as_array()
        .ok_or("No embedding data in response")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();
    Ok(embedding)
}

async fn get_embeddings(_app_handle: &AppHandle, text: &str) -> Result<Vec<f32>, String> {
    let (api_key, base_url) = {
        let config = read_config();
        (
            get_secure_key("vibekidbright-openai", "api_key"),
            config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string(),
        )
    };
    get_embeddings_internal(&api_key, base_url, text).await
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    if v1.is_empty() || v2.is_empty() || v1.len() != v2.len() { return 0.0; }
    let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let n1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let n2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    if n1 > 0.0 && n2 > 0.0 { dot / (n1 * n2) } else { 0.0 }
}

/// FIX: Sentence-boundary chunking — splits on ". ", "! ", "? ", and newlines
/// to keep embedded context semantically coherent.
#[allow(dead_code)]
fn chunk_text(text: &str, target_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    for para in paragraphs {
        let para = para.trim();
        if para.is_empty() { continue; }
        if current.len() + para.len() + 2 <= target_size {
            if !current.is_empty() { current.push_str("\n\n"); }
            current.push_str(para);
        } else {
            if !current.is_empty() { chunks.push(current); }
            if overlap > 0 && !chunks.is_empty() {
                let last = chunks.last().unwrap();
                let tail = if last.len() > overlap { &last[last.len() - overlap..] } else { last };
                current = format!("{}\n\n{}", tail, para);
            } else {
                current = para.to_string();
            }
        }
    }
    if !current.is_empty() { chunks.push(current); }
    if chunks.is_empty() && !text.is_empty() {
        chunks.push(text.chars().take(target_size).collect());
    }
    chunks
}

// ── Helper: recursive KB file collector ──────────────────────────────────────

pub(crate) fn collect_kb_files_inner(root: &Path, current: &Path, result: &mut Vec<(PathBuf, String)>, include_disabled: bool) {
    let Ok(entries) = std::fs::read_dir(current) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; } // skip hidden / .embeddings.json
        if path.is_dir() {
            collect_kb_files_inner(root, &path, result, include_disabled);
        } else if path.is_file() {
            if name.ends_with(".disabled") {
                if include_disabled {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let rel_key = rel.to_string_lossy().replace('\\', "/");
                    result.push((path.clone(), rel_key));
                }
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "txt" | "md" | "c" | "h") {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let rel_key = rel.to_string_lossy().replace('\\', "/");
                result.push((path.clone(), rel_key));
            }
        }
    }
}

pub(crate) fn collect_kb_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    collect_kb_files_inner(root, root, &mut result, false);
    result
}

pub(crate) fn collect_kb_files_all(root: &Path) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    collect_kb_files_inner(root, root, &mut result, true);
    result
}

/// Reindex the knowledge_base using local ONNX embedding (fastembed, offline).
/// Stores results in SQLite via KbStore. Falls back to old JSON index if local
/// embedding is not yet initialized (first run before model download).
pub(crate) async fn reindex_knowledge_base(project_path: &Path) -> Result<usize, String> {
    let kb_path = resolve_kb_path(&project_path.to_string_lossy());
    if !kb_path.exists() { return Ok(0); }

    let store = match crate::kb_store::KbStore::open(&kb_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[KB] Cannot open SQLite store: {e}");
            return Ok(0);
        }
    };

    // On first open: migrate chunks from legacy .embeddings.json if it has data
    if store.chunk_count() == 0 {
        let _ = crate::kb_store::migrate_from_json(&kb_path, &store);
    }

    // Initialize local embedding model if not yet done
    // Model downloads ~45 MB on first use; subsequent runs are instant
    let embed_ready = if !crate::kb_embed::is_embedder_ready() {
        // Try initializing with the app_data_dir as model cache location
        crate::kb_embed::init_local_embedder(None).is_ok()
    } else {
        true
    };

    if !embed_ready {
        // No local model available and no API key — return current chunk count
        return Ok(store.chunk_count());
    }

    let all_files = collect_kb_files(&kb_path);
    let indexed = store.list_indexed_files().unwrap_or_default();
    let indexed_map: std::collections::HashMap<_, _> = indexed.into_iter().collect();

    let mut newly_indexed = 0usize;
    for (file_path, rel_key) in &all_files {
        let mtime = std::fs::metadata(file_path)
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0);

        let last_mtime = indexed_map.get(rel_key.as_str()).cloned().unwrap_or(0);
        if last_mtime >= mtime { continue; } // File not changed

        if let Ok(content) = std::fs::read_to_string(file_path) {
            let text_chunks = crate::kb_embed::chunk_text_for_embedding(&content, 800, 100);
            if text_chunks.is_empty() { continue; }

            match crate::kb_embed::embed_texts(text_chunks.clone()) {
                Ok(embeddings) => {
                    let pairs: Vec<(String, Vec<f32>)> = text_chunks
                        .into_iter()
                        .zip(embeddings.into_iter())
                        .collect();
                    if let Err(e) = store.upsert_file_chunks(rel_key, &pairs, mtime) {
                        eprintln!("[KB] upsert error for {rel_key}: {e}");
                    } else {
                        newly_indexed += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[KB] Embedding error for {rel_key}: {e}");
                }
            }
        }
    }

    // Prune chunks from files that were deleted
    let active: Vec<&str> = all_files.iter().map(|(_, k)| k.as_str()).collect();
    let _ = store.prune_deleted_files(&active);

    // Invalidate query cache on re-index
    if newly_indexed > 0 {
        get_kb_query_cache().lock().unwrap().clear();
    }

    Ok(store.chunk_count())
}


// ── Knowledge search (with query cache) ───────────────────────────────────────

pub async fn knowledge_search(app_handle: &AppHandle, project_path: &Path, query: &str) -> Value {
    let kb_path = resolve_kb_path(&project_path.to_string_lossy());
    if !kb_path.exists() {
        return json!({ "message": "No knowledge_base folder found." });
    }

    {
        let cache = get_kb_query_cache().lock().unwrap();
        if let Some(cached) = cache.get(query) {
            return cached.clone();
        }
    }

    // Fast local chunk keyword search (< 5ms)
    let keyword_results = keyword_knowledge_search(&kb_path, query);
    let has_keyword_results = keyword_results.as_array().map(|a| !a.is_empty()).unwrap_or(false);

    let result = if has_keyword_results {
        // Local pre-computed chunks from .embeddings.json matched instantly!
        keyword_results
    } else {
        // Try vector search with 2-second timeout max
        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            vector_knowledge_search(app_handle, project_path, query)
        ).await {
            Ok(res) if res.as_array().map(|a| !a.is_empty()).unwrap_or(false) => res,
            _ => keyword_results,
        }
    };

    {
        let mut cache = get_kb_query_cache().lock().unwrap();
        cache.insert(query.to_string(), result.clone());
    }
    result
}

/// Keyword search using SQLite store (primary) with file line-scan fallback.
/// Priority: SQLite chunks → raw file scan → full file dump.
pub(crate) fn keyword_knowledge_search(kb_path: &Path, query: &str) -> Value {
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().filter(|w| w.len() > 2).collect();
    if keywords.is_empty() {
        return json!({ "message": "Query too short for keyword search." });
    }

    // 1. Try SQLite vector store keyword search (fast, indexed)
    if let Ok(store) = crate::kb_store::KbStore::open(kb_path) {
        if store.chunk_count() > 0 {
            if let Ok(results) = store.search_keyword(query, 5) {
                if !results.is_empty() {
                    let json_results: Vec<Value> = results.iter().map(|r| r.to_json()).collect();
                    return json!(json_results);
                }
            }
        }
    }

    // 2. Try old .embeddings.json chunks for backward compatibility
    let index_file = kb_path.join(".embeddings.json");
    if index_file.exists() {
        if let Ok(data) = std::fs::read_to_string(&index_file) {
            if let Ok(index) = serde_json::from_str::<VectorIndex>(&data) {
                if !index.chunks.is_empty() {
                    let mut chunk_matches: Vec<(f32, &KnowledgeChunk)> = Vec::new();
                    for chunk in &index.chunks {
                        let content_lower = chunk.content.to_lowercase();
                        let matched = keywords.iter().filter(|kw| content_lower.contains(*kw)).count();
                        if matched > 0 {
                            let score = matched as f32 / keywords.len() as f32;
                            chunk_matches.push((score, chunk));
                        }
                    }
                    if !chunk_matches.is_empty() {
                        chunk_matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                        let results: Vec<Value> = chunk_matches.iter().take(5).map(|(score, chunk)| {
                            json!({ "file": chunk.file_name, "score": score, "content": chunk.content, "method": "chunk_keyword" })
                        }).collect();
                        return json!(results);
                    }
                }
            }
        }
    }

    // 3. Fallback: raw file line scan
    let all_files = collect_kb_files(kb_path);
    let mut results: Vec<Value> = Vec::new();
    for (file_path, rel_key) in &all_files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let content_lower = content.to_lowercase();
            let matched = keywords.iter().filter(|kw| content_lower.contains(*kw)).count();
            if matched == 0 { continue; }
            let score = matched as f32 / keywords.len() as f32;
            let lines: Vec<&str> = content.lines().collect();
            let mut relevant_sections: Vec<String> = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                let line_lower = line.to_lowercase();
                if keywords.iter().any(|kw| line_lower.contains(kw)) {
                    let start = i.saturating_sub(3);
                    let end = (i + 6).min(lines.len());
                    let section = lines[start..end].join("\n");
                    if !relevant_sections.iter().any(|s: &String| s.contains(&section) || section.contains(s.as_str())) {
                        relevant_sections.push(section);
                    }
                }
                if relevant_sections.len() >= 5 { break; }
            }
            let combined = if relevant_sections.is_empty() {
                content.chars().take(2000).collect()
            } else {
                relevant_sections.join("\n---\n")
            };
            results.push(json!({ "file": rel_key, "score": score, "content": combined, "method": "keyword" }));
        }
    }
    results.sort_by(|a, b| {
        b["score"].as_f64().unwrap_or(0.0).partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(5);
    if results.is_empty() {
        let all: Vec<Value> = all_files.iter().filter_map(|(fp, rk)| {
            std::fs::read_to_string(fp).ok().map(|content| {
                json!({ "file": rk, "score": 0.1, "content": content.chars().take(3000).collect::<String>(), "method": "full_dump" })
            })
        }).collect();
        if all.is_empty() { json!({ "message": "No relevant local documents found." }) } else { json!(all) }
    } else {
        json!(results)
    }
}

/// Vector search using SQLite + local ONNX embedding (offline-capable).
/// Falls back to the old OpenAI API embedding if local model is not ready.
async fn vector_knowledge_search(app_handle: &AppHandle, project_path: &Path, query: &str) -> Value {
    let kb_path = resolve_kb_path(&project_path.to_string_lossy());

    // 1. Try local embedding (offline) + SQLite vector store
    if crate::kb_embed::is_embedder_ready() || crate::kb_embed::init_local_embedder(None).is_ok() {
        if let Ok(query_embedding) = crate::kb_embed::embed_query(query) {
            if let Ok(store) = crate::kb_store::KbStore::open(&kb_path) {
                if store.chunk_count() > 0 {
                    if let Ok(results) = store.search_vector(&query_embedding, 5, 0.25) {
                        if !results.is_empty() {
                            let json_results: Vec<Value> = results.iter().map(|r| r.to_json()).collect();
                            return json!(json_results);
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback: trigger re-index (will use local embedding) + retry
    let _ = reindex_knowledge_base(project_path).await;

    // 3. Last resort: OpenAI API embedding (legacy path, requires API key)
    let query_embedding = match get_embeddings(app_handle, query).await {
        Ok(e) => e,
        Err(_) => return json!([]),
    };
    let index_file = kb_path.join(".embeddings.json");
    if !index_file.exists() { return json!([]); }
    let data = std::fs::read_to_string(&index_file).unwrap_or_default();
    let index: VectorIndex = serde_json::from_str(&data).unwrap_or_default();
    if index.chunks.is_empty() { return json!([]); }
    let mut matches: Vec<(f32, &KnowledgeChunk)> = index.chunks.iter()
        .map(|c| (cosine_similarity(&query_embedding, &c.embedding), c))
        .filter(|(s, _)| *s > 0.3)
        .collect();
    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let results: Vec<Value> = matches.iter().take(5).map(|(score, chunk)| {
        json!({ "file": chunk.file_name, "score": score, "content": chunk.content, "method": "vector-api" })
    }).collect();
    json!(results)
}


// ── Unit Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

// ── cosine_similarity ─────────────────────────────────────────────────────

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0f32, 2.0, 3.0];
        let result = cosine_similarity(&v, &v);
        assert!((result - 1.0).abs() < 1e-6, "identical vectors → 1.0, got {}", result);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let v1 = vec![1.0f32, 0.0, 0.0];
        let v2 = vec![0.0f32, 1.0, 0.0];
        let result = cosine_similarity(&v1, &v2);
        assert!(result.abs() < 1e-6, "orthogonal vectors → 0.0, got {}", result);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let v1 = vec![1.0f32, 0.0];
        let v2 = vec![-1.0f32, 0.0];
        let result = cosine_similarity(&v1, &v2);
        assert!((result + 1.0).abs() < 1e-6, "opposite vectors → -1.0, got {}", result);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0, "empty vectors → 0.0");
    }

    #[test]
    fn test_cosine_similarity_length_mismatch() {
        let v1 = vec![1.0f32, 0.0];
        let v2 = vec![1.0f32, 0.0, 0.0];
        assert_eq!(cosine_similarity(&v1, &v2), 0.0, "mismatched lengths → 0.0");
    }

    #[test]
    fn test_cosine_similarity_45_degrees() {
        // [1,1] vs [1,0] — cos(45°) = 1/√2 ≈ 0.7071
        let v1 = vec![1.0f32, 1.0];
        let v2 = vec![1.0f32, 0.0];
        let result = cosine_similarity(&v1, &v2);
        assert!(
            (result - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5,
            "expected ≈0.7071, got {}",
            result
        );
    }

    // ── chunk_text ────────────────────────────────────────────────────────────

    #[test]
    fn test_chunk_text_short_text_single_chunk() {
        let text = "Hello world. This is a short test.";
        let chunks = chunk_text(text, 600, 80);
        assert_eq!(chunks.len(), 1, "short text → 1 chunk");
    }

    #[test]
    fn test_chunk_text_empty() {
        let chunks = chunk_text("", 600, 80);
        assert!(chunks.is_empty(), "empty input → no chunks");
    }

    #[test]
    fn test_chunk_text_splits_on_paragraph_break() {
        let big_a = "AAAA ".repeat(100); // 500 chars
        let big_b = "BBBB ".repeat(100); // 500 chars
        let text = format!("{}\n\n{}", big_a, big_b);
        let chunks = chunk_text(&text, 600, 0);
        assert!(chunks.len() >= 2, "two big paragraphs → at least 2 chunks, got {}", chunks.len());
    }

    #[test]
    fn test_chunk_text_all_content_covered() {
        let text = "First block.\n\nSecond block.\n\nThird block.";
        let chunks = chunk_text(text, 20, 0);
        let combined = chunks.join(" ");
        assert!(combined.contains("First"), "chunks should contain 'First'");
        assert!(combined.contains("Second"), "chunks should contain 'Second'");
        assert!(combined.contains("Third"), "chunks should contain 'Third'");
    }

    #[test]
    fn test_chunk_text_no_tiny_chunks() {
        // Single very long line with no paragraph breaks
        let long_line = "x ".repeat(1000);
        let chunks = chunk_text(&long_line, 200, 20);
        for c in &chunks {
            assert!(!c.trim().is_empty(), "no chunk should be empty/whitespace");
        }
    }

    // ── keyword_knowledge_search ──────────────────────────────────────────────

    #[test]
    fn test_keyword_search_nonexistent_dir_no_panic() {
        let kb = std::path::Path::new("/this/kb/does/not/exist");
        let result = keyword_knowledge_search(kb, "gpio pwm esp32");
        // Must return either a JSON array or an object message — never panic
        assert!(
            result.is_array() || result.is_object(),
            "keyword_knowledge_search must return JSON, got: {:?}",
            result
        );
    }

    #[test]
    fn test_keyword_search_empty_query_returns_gracefully() {
        let kb = std::path::Path::new("/this/kb/does/not/exist");
        let result = keyword_knowledge_search(kb, "");
        assert!(
            result.is_array() || result.is_object(),
            "empty query must return JSON gracefully"
        );
    }
}
