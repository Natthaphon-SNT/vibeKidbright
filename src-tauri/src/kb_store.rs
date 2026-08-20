/// kb_store.rs — SQLite-backed Knowledge Base vector store
///
/// Replaces the flat `.embeddings.json` approach with a proper SQLite database.
/// Uses `rusqlite` (bundled libsqlite3) for storage and manual cosine similarity
/// for vector search (fast enough for KB sizes under ~50k chunks).
///
/// Schema:
///   kb_chunks(id, file_name, content, embedding BLOB, indexed_at REAL)
///   kb_meta(key TEXT PRIMARY KEY, value TEXT)
///
/// The embedding BLOB is a little-endian f32 array (4 bytes per dimension).
/// Dimension is determined by the model used (all-MiniLM-L6-v2 → 384 dims).
///
/// Usage:
///   let store = KbStore::open(&kb_dir)?;
///   store.upsert_chunks("file.md", chunks_and_embeddings)?;
///   let results = store.search_vector(&query_embedding, 5, 0.3)?;
///   let results = store.search_keyword("gpio", 5)?;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const DB_FILENAME: &str = ".kb_store.sqlite";
#[allow(dead_code)]
const EMBEDDING_DIM: usize = 384; // all-MiniLM-L6-v2 output dimension

// ── KbStore ───────────────────────────────────────────────────────────────────

pub struct KbStore {
    conn: Connection,
    #[allow(dead_code)]
    pub db_path: PathBuf,
}

impl KbStore {
    /// Open (or create) the SQLite database in `kb_dir`.
    pub fn open(kb_dir: &Path) -> Result<Self, String> {
        let db_path = kb_dir.join(DB_FILENAME);
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("KbStore open error: {e}"))?;

        // Enable WAL mode for concurrent reads + writes
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| format!("PRAGMA error: {e}"))?;

        // Create tables if missing
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kb_chunks (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name   TEXT    NOT NULL,
                content     TEXT    NOT NULL,
                embedding   BLOB,
                indexed_at  REAL    NOT NULL DEFAULT (unixepoch('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_kb_chunks_file ON kb_chunks(file_name);

            CREATE TABLE IF NOT EXISTS kb_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Schema creation error: {e}"))?;

        Ok(Self { conn, db_path })
    }

    /// Return the mtime (Unix timestamp) of a file as recorded in the store.
    /// Returns `None` if the file has not been indexed yet.
    #[allow(dead_code)]
    pub fn get_file_mtime(&self, file_name: &str) -> Option<u64> {
        self.conn
            .query_row(
                "SELECT MAX(indexed_at) FROM kb_chunks WHERE file_name = ?1",
                params![file_name],
                |row| row.get::<_, f64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .map(|t| t as u64)
    }

    /// Delete all chunks belonging to a file.
    pub fn delete_file(&self, file_name: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM kb_chunks WHERE file_name = ?1", params![file_name])
            .map_err(|e| format!("delete_file error: {e}"))?;
        Ok(())
    }

    /// Insert chunks + embeddings for a file (deletes existing chunks first).
    /// `chunks`: Vec of (text_content, embedding_vec)
    pub fn upsert_file_chunks(
        &self,
        file_name: &str,
        chunks: &[(String, Vec<f32>)],
        mtime: u64,
    ) -> Result<(), String> {
        // Remove old chunks for this file
        self.delete_file(file_name)?;

        let mut stmt = self
            .conn
            .prepare(
                "INSERT INTO kb_chunks (file_name, content, embedding, indexed_at)
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(|e| format!("prepare error: {e}"))?;

        for (content, embedding) in chunks {
            let emb_blob = f32_slice_to_blob(embedding);
            stmt.execute(params![file_name, content, emb_blob, mtime as f64])
                .map_err(|e| format!("insert error: {e}"))?;
        }
        Ok(())
    }

    /// Vector similarity search — returns top-k chunks with score >= min_score.
    /// Loads all embeddings into memory and computes cosine similarity.
    /// Efficient for KB sizes up to ~100k chunks on modern hardware.
    pub fn search_vector(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        min_score: f32,
    ) -> Result<Vec<KbSearchResult>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT file_name, content, embedding FROM kb_chunks WHERE embedding IS NOT NULL")
            .map_err(|e| format!("search_vector prepare: {e}"))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|e| format!("search_vector query: {e}"))?;

        let mut scored: Vec<(f32, String, String)> = Vec::new();
        for row in rows {
            let (file_name, content, blob) = row.map_err(|e| format!("row error: {e}"))?;
            let emb = blob_to_f32_vec(&blob);
            let score = cosine_similarity(query_embedding, &emb);
            if score >= min_score {
                scored.push((score, file_name, content));
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored
            .into_iter()
            .take(top_k)
            .map(|(score, file_name, content)| KbSearchResult {
                file_name,
                content,
                score,
                method: "vector-sqlite".to_string(),
            })
            .collect())
    }

    /// Keyword (full-text) search — simple LIKE-based scan over content column.
    /// Fast because SQLite handles the scan natively without loading BLOBs.
    pub fn search_keyword(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KbSearchResult>, String> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        // Build a WHERE clause that checks each keyword token
        let tokens: Vec<String> = query
            .split_whitespace()
            .filter(|t| t.len() > 2)
            .take(8) // cap at 8 keywords for performance
            .map(|t| format!("%{}%", t.to_lowercase()))
            .collect();

        if tokens.is_empty() {
            return Ok(vec![]);
        }

        // Score = number of keyword tokens found in content (higher = better)
        let score_expr = tokens
            .iter()
            .enumerate()
            .map(|(i, _)| format!("(CASE WHEN LOWER(content) LIKE ?{} THEN 1 ELSE 0 END)", i + 1))
            .collect::<Vec<_>>()
            .join(" + ");

        let sql = format!(
            "SELECT file_name, content, ({score}) as score
             FROM kb_chunks
             WHERE ({score}) > 0
             ORDER BY score DESC
             LIMIT {top_k}",
            score = score_expr
        );

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| format!("keyword search prepare: {e}"))?;

        let params_vec: Vec<&dyn rusqlite::ToSql> = tokens
            .iter()
            .map(|t| t as &dyn rusqlite::ToSql)
            .collect();

        let rows = stmt
            .query_map(params_vec.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("keyword search query: {e}"))?;

        let mut results = Vec::new();
        for row in rows {
            let (file_name, content, score) = row.map_err(|e| format!("row error: {e}"))?;
            results.push(KbSearchResult {
                file_name,
                content,
                score: score as f32 / tokens.len() as f32, // normalize to [0,1]
                method: "keyword-sqlite".to_string(),
            });
        }
        Ok(results)
    }

    /// Return list of all indexed file names and their mtime.
    pub fn list_indexed_files(&self) -> Result<Vec<(String, u64)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT file_name, MAX(indexed_at) FROM kb_chunks GROUP BY file_name")
            .map_err(|e| format!("list_indexed_files prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)? as u64))
            })
            .map_err(|e| format!("list_indexed_files query: {e}"))?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("row error: {e}"))?);
        }
        Ok(result)
    }

    /// Total number of chunks stored.
    pub fn chunk_count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM kb_chunks", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0) as usize
    }

    /// Delete all chunks belonging to files no longer present on disk.
    pub fn prune_deleted_files(&self, active_files: &[&str]) -> Result<usize, String> {
        if active_files.is_empty() {
            return Ok(0);
        }
        let placeholders: String = active_files
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!("DELETE FROM kb_chunks WHERE file_name NOT IN ({placeholders})");
        let params_vec: Vec<&dyn rusqlite::ToSql> = active_files
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let deleted = self
            .conn
            .execute(&sql, params_vec.as_slice())
            .map_err(|e| format!("prune error: {e}"))?;
        Ok(deleted)
    }
}

// ── Result type ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct KbSearchResult {
    pub file_name: String,
    pub content: String,
    pub score: f32,
    pub method: String,
}

impl KbSearchResult {
    pub fn to_json(&self) -> Value {
        json!({
            "file": self.file_name,
            "score": self.score,
            "content": self.content,
            "method": self.method,
        })
    }
}

// ── Embedding serialization ───────────────────────────────────────────────────

/// Serialize Vec<f32> to raw little-endian bytes for SQLite BLOB storage.
pub fn f32_slice_to_blob(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// Deserialize SQLite BLOB back to Vec<f32>.
pub fn blob_to_f32_vec(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ── Math ──────────────────────────────────────────────────────────────────────

/// Cosine similarity between two f32 vectors.
/// Returns 0.0 if either vector is zero or lengths differ.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ── Legacy JSON migration ──────────────────────────────────────────────────────

/// Import chunks from the old `.embeddings.json` format into SQLite.
/// This is called once on first launch if the DB doesn't exist yet
/// but the old JSON file does.
pub fn migrate_from_json(kb_dir: &Path, store: &KbStore) -> Result<usize, String> {
    let json_path = kb_dir.join(".embeddings.json");
    if !json_path.exists() {
        return Ok(0);
    }

    let data = std::fs::read_to_string(&json_path)
        .map_err(|e| format!("migration read error: {e}"))?;
    let index: serde_json::Value = serde_json::from_str(&data)
        .map_err(|e| format!("migration JSON parse error: {e}"))?;

    let chunks = match index["chunks"].as_array() {
        Some(c) if !c.is_empty() => c,
        _ => return Ok(0), // JSON index has no chunks — skip migration
    };

    // Group chunks by file_name
    let mut by_file: std::collections::HashMap<String, Vec<(String, Vec<f32>)>> =
        std::collections::HashMap::new();

    for chunk in chunks {
        let file_name = chunk["file_name"].as_str().unwrap_or("").to_string();
        let content = chunk["content"].as_str().unwrap_or("").to_string();
        let embedding: Vec<f32> = chunk["embedding"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
            .unwrap_or_default();

        if !file_name.is_empty() && !content.is_empty() {
            by_file.entry(file_name).or_default().push((content, embedding));
        }
    }

    let total = by_file.values().map(|v| v.len()).sum();
    for (file_name, file_chunks) in &by_file {
        store.upsert_file_chunks(file_name, file_chunks, 0)?;
    }

    eprintln!(
        "[KB Migration] Migrated {} chunks from {} files",
        total,
        by_file.len()
    );
    Ok(total)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_store() -> (KbStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let store = KbStore::open(dir.path()).expect("open store");
        (store, dir)
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0f32, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let v1 = vec![1.0f32, 0.0];
        let v2 = vec![0.0f32, 1.0];
        assert!(cosine_similarity(&v1, &v2).abs() < 1e-6);
    }

    #[test]
    fn test_f32_blob_roundtrip() {
        let original = vec![0.1f32, 0.2, 0.3, -0.4];
        let blob = f32_slice_to_blob(&original);
        let recovered = blob_to_f32_vec(&blob);
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-7, "f32 blob roundtrip failed: {} != {}", a, b);
        }
    }

    #[test]
    fn test_store_open_creates_schema() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KbStore::open(dir.path()).expect("open");
        assert_eq!(store.chunk_count(), 0);
    }

    #[test]
    fn test_upsert_and_count() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KbStore::open(dir.path()).expect("open");
        let emb = vec![0.1f32; 384];
        let chunks = vec![
            ("chunk one content here".to_string(), emb.clone()),
            ("chunk two content here".to_string(), emb.clone()),
        ];
        store.upsert_file_chunks("test.md", &chunks, 12345).expect("upsert");
        assert_eq!(store.chunk_count(), 2);
    }

    #[test]
    fn test_upsert_replaces_old_chunks() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KbStore::open(dir.path()).expect("open");
        let emb = vec![0.1f32; 384];
        let chunks = vec![("old content".to_string(), emb.clone())];
        store.upsert_file_chunks("test.md", &chunks, 1000).expect("first upsert");
        assert_eq!(store.chunk_count(), 1);

        let new_chunks = vec![
            ("new content A".to_string(), emb.clone()),
            ("new content B".to_string(), emb.clone()),
        ];
        store.upsert_file_chunks("test.md", &new_chunks, 2000).expect("second upsert");
        // Should have 2 chunks now (old one deleted)
        assert_eq!(store.chunk_count(), 2);
    }

    #[test]
    fn test_keyword_search() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KbStore::open(dir.path()).expect("open");
        let emb = vec![0.0f32; 384]; // zero embedding, no vector search
        let chunks = vec![
            ("GPIO pin configuration for ESP32".to_string(), emb.clone()),
            ("UART baud rate settings".to_string(), emb.clone()),
            ("SPI bus initialization".to_string(), emb.clone()),
        ];
        store.upsert_file_chunks("driver.md", &chunks, 1000).expect("upsert");

        let results = store.search_keyword("gpio esp32", 5).expect("search");
        assert!(!results.is_empty(), "should find at least one result");
        assert!(
            results[0].content.to_lowercase().contains("gpio"),
            "top result should contain 'gpio'"
        );
    }

    #[test]
    fn test_vector_search() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KbStore::open(dir.path()).expect("open");

        // Create two chunks with distinct embeddings
        let emb_a: Vec<f32> = (0..384).map(|i| if i == 0 { 1.0 } else { 0.0 }).collect();
        let emb_b: Vec<f32> = (0..384).map(|i| if i == 1 { 1.0 } else { 0.0 }).collect();

        store.upsert_file_chunks("a.md", &[("chunk A".to_string(), emb_a.clone())], 1000).unwrap();
        store.upsert_file_chunks("b.md", &[("chunk B".to_string(), emb_b.clone())], 1000).unwrap();

        // Query identical to emb_a — should return "chunk A" first
        let results = store.search_vector(&emb_a, 2, 0.0).expect("search");
        assert!(!results.is_empty());
        assert_eq!(results[0].file_name, "a.md", "a.md should be the top hit");
    }

    #[test]
    fn test_list_indexed_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = KbStore::open(dir.path()).expect("open");
        let emb = vec![0.0f32; 384];
        store.upsert_file_chunks("a.md", &[("a".to_string(), emb.clone())], 1000).unwrap();
        store.upsert_file_chunks("b.md", &[("b".to_string(), emb.clone())], 2000).unwrap();

        let files = store.list_indexed_files().unwrap();
        assert_eq!(files.len(), 2);
    }
}
