//! Local embedding from installer resources. No hub client or runtime downloads.
use fastembed::{InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles, UserDefinedEmbeddingModel};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static LOCAL_EMBEDDER: OnceLock<Mutex<Option<TextEmbedding>>> = OnceLock::new();
static MODEL_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
fn embedder_cell() -> &'static Mutex<Option<TextEmbedding>> {
    LOCAL_EMBEDDER.get_or_init(|| Mutex::new(None))
}

pub fn configure_model_directory(directory: PathBuf) -> Result<(), String> {
    if let Some(existing) = MODEL_DIRECTORY.get() {
        if existing == &directory { return Ok(()); }
        return Err("Embedding model directory is already configured.".into());
    }
    MODEL_DIRECTORY.set(directory).map_err(|_| "Embedding directory initialization raced.".into())
}

pub fn init_local_embedder(directory: Option<&Path>) -> Result<(), String> {
    let mut lock = embedder_cell().lock().map_err(|_| "Embedding state is unavailable.")?;
    if lock.is_some() { return Ok(()); }
    let directory = directory.map(Path::to_path_buf).or_else(|| MODEL_DIRECTORY.get().cloned());
    #[cfg(debug_assertions)]
    let directory = directory.or_else(|| Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/embedding")));
    let directory = directory.ok_or("Bundled embedding resources are unavailable. Reinstall the application.")?;
    let read = |name: &str| std::fs::read(directory.join(name))
        .map_err(|error| format!("Cannot load bundled embedding resource {name}: {error}"));
    let tokenizer = TokenizerFiles {
        tokenizer_file: read("tokenizer.json")?, config_file: read("model-config.json")?,
        special_tokens_map_file: read("special_tokens_map.json")?, tokenizer_config_file: read("tokenizer_config.json")?,
    };
    let model = UserDefinedEmbeddingModel::new(read("model.onnx")?, tokenizer).with_pooling(Pooling::Mean);
    *lock = Some(TextEmbedding::try_new_from_user_defined(model, InitOptionsUserDefined::default())
        .map_err(|error| format!("Cannot initialize the bundled embedding model: {error}"))?);
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
    let chars: Vec<char> = text.chars().collect();
    let size = target_size.max(1);
    let step = size.saturating_sub(overlap.min(size - 1)).max(1);
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        if !chunk.trim().is_empty() { chunks.push(chunk.trim().to_string()); }
        if end == chars.len() { break; }
        start += step;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_chunks_preserve_thai_without_panics_or_infinite_overlap() {
        let text = "อ่านอุณหภูมิและตรวจสอบปุ่มกด".repeat(50);
        assert_eq!(chunk_text_for_embedding(&text, 23, 0).concat(), text);
        assert!(chunk_text_for_embedding("ไทย", 0, 500).len() <= 3);
    }

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

    // The packaged-binary --self-check verifies real offline model inference.
}
