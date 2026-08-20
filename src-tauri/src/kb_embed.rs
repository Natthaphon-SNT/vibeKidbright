/// kb_embed.rs — Local ONNX-based text embedding (offline, no API key required)
///
/// Uses the `fastembed` crate which bundles ONNX Runtime and downloads
/// the all-MiniLM-L6-v2 model on first use (~45 MB, cached in app data).
///
/// The model is:
///   - sentence-transformers/all-MiniLM-L6-v2
///   - 384-dimensional embeddings
///   - Efficient on CPU (no GPU needed)
///   - Semantically strong for technical English + Thai text
///
/// Model cache location: OS app data dir / fastembed-cache / (managed by fastembed)
///
/// Usage:
///   let model = get_local_embedder()?;
///   let embeddings = model.embed(vec!["text"], None)?;  // returns Vec<Vec<f32>>

use fastembed::{EmbeddingModel, InitOptions, InitOptionsUserDefined, TextEmbedding};
use std::sync::{Mutex, OnceLock};

// ── Lazy global model instance ─────────────────────────────────────────────────
// OnceLock ensures the model is initialized exactly once per process.
// Mutex allows interior mutability since TextEmbedding is not Sync.

static LOCAL_EMBEDDER: OnceLock<Mutex<Option<TextEmbedding>>> = OnceLock::new();

fn embedder_cell() -> &'static Mutex<Option<TextEmbedding>> {
    LOCAL_EMBEDDER.get_or_init(|| Mutex::new(None))
}

/// Initialize the local embedding model.
/// Downloads the model on first call (~45 MB), then uses cached version.
/// Returns Err if the model can't be loaded (network unavailable + no cache).
pub fn init_local_embedder(cache_dir: Option<&std::path::Path>) -> Result<(), String> {
    let mut lock = embedder_cell().lock().unwrap();
    if lock.is_some() {
        return Ok(()); // Already initialized
    }

    eprintln!("[KB Embed] Initializing all-MiniLM-L6-v2 (first run: ~45 MB download)...");

    let mut opts = InitOptions::new(EmbeddingModel::AllMiniLML6V2);

    // Use custom cache directory if provided (e.g. Tauri app_data_dir)
    if let Some(dir) = cache_dir {
        opts = opts.with_cache_dir(dir.to_path_buf());
    }

    let model = TextEmbedding::try_new(opts)
        .map_err(|e| format!("[KB Embed] Failed to load model: {e}"))?;

    *lock = Some(model);
    eprintln!("[KB Embed] Model ready — 384-dim, all-MiniLM-L6-v2");
    Ok(())
}

/// Generate embeddings for a batch of text strings.
/// Calls `init_local_embedder` automatically if not yet initialized.
/// Returns Vec<Vec<f32>> — one embedding per input text.
pub fn embed_texts(texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
    // Auto-init with default cache location if not yet done
    {
        let lock = embedder_cell().lock().unwrap();
        if lock.is_none() {
            drop(lock);
            init_local_embedder(None)?;
        }
    }

    let lock = embedder_cell().lock().unwrap();
    let model = lock.as_ref().ok_or("Embedder not initialized")?;

    model
        .embed(texts, None)
        .map_err(|e| format!("[KB Embed] Embedding error: {e}"))
}

/// Embed a single query string (for search-time use).
/// More efficient than creating a batch for one text.
pub fn embed_query(query: &str) -> Result<Vec<f32>, String> {
    let results = embed_texts(vec![query.to_string()])?;
    results
        .into_iter()
        .next()
        .ok_or_else(|| "embed_query: empty result".to_string())
}

/// Check whether the local model is already loaded (no network needed).
pub fn is_embedder_ready() -> bool {
    embedder_cell()
        .lock()
        .unwrap()
        .is_some()
}

/// Chunk a text string into overlapping segments suitable for embedding.
/// Target chunk size ~600 chars with 80-char overlap (matching Python script).
/// Splits on paragraph/sentence boundaries when possible.
pub fn chunk_text_for_embedding(text: &str, target_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    // Split on double newlines (paragraph boundaries)
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();

    for para in &paragraphs {
        if current.len() + para.len() + 2 <= target_size {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
        } else {
            if !current.is_empty() {
                chunks.push(current.clone());
            }
            // Start next chunk with overlap from previous
            if overlap > 0 && !chunks.is_empty() {
                let prev = &chunks[chunks.len() - 1];
                let tail = &prev[prev.len().saturating_sub(overlap)..];
                current = format!("{}\n\n{}", tail, para);
            } else {
                current = para.to_string();
            }
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }

    // Fallback for text without paragraph breaks
    if chunks.is_empty() && !text.is_empty() {
        let mut i = 0;
        while i < text.len() {
            let end = (i + target_size).min(text.len());
            let chunk = text[i..end].trim().to_string();
            if chunk.len() > 20 {
                chunks.push(chunk);
            }
            if end >= text.len() { break; }
            i += target_size.saturating_sub(overlap);
        }
    }

    chunks.into_iter().filter(|c| c.len() > 5).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_short_single_chunk() {
        let text = "Short text here.";
        let chunks = chunk_text_for_embedding(text, 600, 80);
        // Short text (>5 chars) should produce exactly one chunk
        assert_eq!(chunks.len(), 1, "got chunks: {:?}", chunks);
    }

    #[test]
    fn test_chunk_text_empty() {
        let chunks = chunk_text_for_embedding("", 600, 80);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_text_splits_paragraphs() {
        let big_a = "A ".repeat(200); // 400 chars
        let big_b = "B ".repeat(200); // 400 chars
        let text = format!("{}\n\n{}", big_a, big_b);
        let chunks = chunk_text_for_embedding(&text, 600, 0);
        assert!(chunks.len() >= 2, "should split two large paragraphs");
    }

    #[test]
    fn test_chunk_text_preserves_content() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        // All three paragraphs are > 5 chars, so they should all appear
        let chunks = chunk_text_for_embedding(text, 600, 0);
        // They may be merged into one chunk since total is < 600 chars
        let combined = chunks.join(" ");
        assert!(combined.contains("First"),  "combined: {combined}");
        assert!(combined.contains("Second"), "combined: {combined}");
        assert!(combined.contains("Third"),  "combined: {combined}");
    }

    // Note: embed_texts / embed_query tests are integration tests that require
    // network access (first run) and are not included here.
    // They are covered by manual testing and the CI integration test job.
}
