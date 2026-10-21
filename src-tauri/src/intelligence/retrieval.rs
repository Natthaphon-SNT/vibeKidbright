use super::chunk_source;
use crate::kb_store::{blob_to_f32_vec, cosine_similarity, f32_slice_to_blob, KbStore};
use rusqlite::{named_params, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODEL_ID: &str =
    "Qdrant/all-MiniLM-L6-v2-onnx@d13954661f83248295ba75c1ed411eef3b7b936e:384:mean";
const CHUNK_VERSION: &str = "tree-sitter-c-0.24.2-bytes-v3";
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetrievalConfig {
    pub top_k: usize,
    pub candidate_k: usize,
    pub rrf_k: f64,
    pub lexical_weight: f64,
    pub dense_weight: f64,
    pub max_context_chars: usize,
    pub chunk_chars: usize,
    pub batch_size: usize,
    pub dense_enabled: bool,
    pub query_routing: bool,
    pub technical_expansion: bool,
}
impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            candidate_k: 40,
            rrf_k: 60.,
            lexical_weight: 1.,
            dense_weight: 0.15,
            max_context_chars: 12000,
            chunk_chars: 1600,
            batch_size: 16,
            dense_enabled: true,
            query_routing: false,
            technical_expansion: true,
        }
    }
}
impl RetrievalConfig {
    pub fn needs_dense(&self, query:&str)->bool {
        self.dense_enabled && !(self.query_routing && query.contains('_') && query.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'))
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=20).contains(&self.top_k)
            || !(self.top_k..=200).contains(&self.candidate_k)
            || !(1.0..=1000.0).contains(&self.rrf_k)
            || !(0.0..=10.0).contains(&self.lexical_weight)
            || !(0.0..=10.0).contains(&self.dense_weight)
            || !self.rrf_k.is_finite()
            || !self.lexical_weight.is_finite()
            || !self.dense_weight.is_finite()
            || self.lexical_weight + self.dense_weight == 0.0
            || !(512..=32000).contains(&self.max_context_chars)
            || !(256..=8000).contains(&self.chunk_chars)
            || !(1..=64).contains(&self.batch_size)
        {
            return Err(
                "Invalid retrieval limits/weights. See ai-engineering configuration documentation."
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchFilter {
    pub board: Option<String>,
    pub target: Option<String>,
    /// Exact declared SDK version/series, never inferred from example contents.
    pub sdk_version: Option<String>,
    pub include_unknown: bool,
    #[serde(skip)]
    pub bundled_only: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Metadata {
    pub source_url: Option<String>,
    pub title: String,
    pub category: String,
    pub language: String,
    pub sdk_version: Option<String>,
    pub board: Option<String>,
    pub target: Option<String>,
    pub framework: Option<String>,
    pub component: Option<String>,
    pub verification: String,
    pub license: Option<String>,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            source_url: None,
            title: String::new(),
            category: "documentation".into(),
            language: "unknown".into(),
            sdk_version: None,
            board: None,
            target: None,
            framework: None,
            component: None,
            verification: "unverified-reference".into(),
            license: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub file: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_start_line: usize,
    pub parent_end_line: usize,
    pub symbols: Vec<String>,
    pub includes: Vec<String>,
    pub kind: String,
    pub metadata: Metadata,
    pub content_hash: String,
    pub indexed_at: u64,
    pub score: f64,
    pub method: String,
    pub citation: String,
    pub bundled: bool,
    #[serde(default)] pub start_byte: usize,
    #[serde(default)] pub end_byte: usize,
    #[serde(default)] pub parent_start_byte: usize,
    #[serde(default)] pub parent_end_byte: usize,
    #[serde(default)] pub signature: Option<String>,
    #[serde(default)] pub parser: String,
    #[serde(default)] pub syntax_error: bool,
}
#[derive(Default, Debug, Serialize)]
pub struct IndexReport {
    pub files_seen: usize,
    pub files_updated: usize,
    pub files_unchanged: usize,
    pub files_deleted: usize,
    pub embedded_chunks: usize,
    pub cached_embeddings: usize,
    pub chunks: usize,
    pub warnings: Vec<String>,
}
pub struct Engine {
    store: KbStore,
    pub config: RetrievalConfig,
    model_id: String,
    project_scope: bool,
}
#[derive(Deserialize)]
struct BaselineSource {
    content_sha256: String,
    metadata: Metadata,
}

impl Engine {
    /// Evaluation baseline only: the previous LIKE ranker on the same chunk corpus.
    pub fn legacy_keyword_files(&self, query: &str, k: usize) -> Result<Vec<String>, String> {
        Ok(self
            .store
            .search_keyword(query, k)?
            .into_iter()
            .map(|r| r.file_name)
            .collect())
    }
    pub fn open(index_directory: &Path, config: RetrievalConfig) -> Result<Self, String> {
        Self::open_scoped(index_directory,config,MODEL_ID,false)
    }
    /// Local evaluation only; indexes and vector caches are bound to this identity.
    /// Application callers retain the bundled identity through `open`.
    pub fn open_for_evaluation(index_directory:&Path,config:RetrievalConfig,model_id:&str)->Result<Self,String> {
        if model_id.is_empty() || model_id.len()>512 {return Err("Invalid evaluation model identity.".into());}
        Self::open_scoped(index_directory,config,model_id,false)
    }
    pub fn open_project(index_directory:&Path,config:RetrievalConfig)->Result<Self,String> {
        Self::open_scoped(index_directory,config,MODEL_ID,true)
    }
    fn open_scoped(index_directory:&Path,config:RetrievalConfig,model_id:&str,project_scope:bool)->Result<Self,String> {
        config.validate()?;
        let store = KbStore::open(index_directory)?;
        store
            .connection()
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        let tx = store
            .connection()
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS ki_sources(path TEXT PRIMARY KEY, fingerprint TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS ki_chunks(
                chunk_id INTEGER PRIMARY KEY, stable_id TEXT NOT NULL, info TEXT NOT NULL,
                model_id TEXT, content_hash TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS ki_embeddings(
                hash TEXT NOT NULL, model TEXT NOT NULL, vector BLOB NOT NULL,
                PRIMARY KEY(hash, model));
            CREATE VIRTUAL TABLE IF NOT EXISTS ki_fts USING fts5(
                file_name, content, content='kb_chunks', content_rowid='id',
                tokenize="unicode61 tokenchars '_'");
            CREATE VIRTUAL TABLE IF NOT EXISTS ki_trigram USING fts5(
                file_name, content, content='kb_chunks', content_rowid='id', tokenize='trigram');
            CREATE TRIGGER IF NOT EXISTS ki_insert AFTER INSERT ON kb_chunks BEGIN
                INSERT INTO ki_fts(rowid,file_name,content) VALUES(new.id,new.file_name,new.content);
                INSERT INTO ki_trigram(rowid,file_name,content) VALUES(new.id,new.file_name,new.content);
            END;
            CREATE TRIGGER IF NOT EXISTS ki_delete AFTER DELETE ON kb_chunks BEGIN
                INSERT INTO ki_fts(ki_fts,rowid,file_name,content) VALUES('delete',old.id,old.file_name,old.content);
                INSERT INTO ki_trigram(ki_trigram,rowid,file_name,content) VALUES('delete',old.id,old.file_name,old.content);
                DELETE FROM ki_chunks WHERE chunk_id=old.id;
            END;
            CREATE TRIGGER IF NOT EXISTS ki_update AFTER UPDATE ON kb_chunks BEGIN
                INSERT INTO ki_fts(ki_fts,rowid,file_name,content) VALUES('delete',old.id,old.file_name,old.content);
                INSERT INTO ki_trigram(ki_trigram,rowid,file_name,content) VALUES('delete',old.id,old.file_name,old.content);
                INSERT INTO ki_fts(rowid,file_name,content) VALUES(new.id,new.file_name,new.content);
                INSERT INTO ki_trigram(rowid,file_name,content) VALUES(new.id,new.file_name,new.content);
                DELETE FROM ki_chunks WHERE chunk_id=old.id;
            END;
        "#).map_err(|e| format!("Knowledge schema migration failed: {e}"))?;
        let version: Option<String> = tx
            .query_row(
                "SELECT value FROM kb_meta WHERE key='intelligence_schema'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(ref value) = version {
            if value != "1" {
                return Err("Knowledge index uses an unsupported schema; preserve it and rebuild in a new directory.".into());
            }
        }
        if version.is_none() {
            tx.execute_batch("INSERT INTO ki_fts(ki_fts) VALUES('rebuild'); INSERT INTO ki_trigram(ki_trigram) VALUES('rebuild'); INSERT INTO kb_meta(key,value) VALUES('intelligence_schema','1');").map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Self { store, config, model_id:model_id.into(), project_scope })
    }

    /// Hashes every active small source; only changed files are rechunked/embedded.
    /// The embedding callback is local-only in the application integration.
    pub fn ingest<F>(
        &self,
        root: &Path,
        cancelled: &AtomicBool,
        mut embed: F,
    ) -> Result<IndexReport, String>
    where
        F: FnMut(Vec<String>) -> Result<Vec<Vec<f32>>, String>,
    {
        let mut report = IndexReport::default();
        let mut files = Vec::new();
        if self.project_scope {
            for directory in ["main","components"] {
                let path=root.join(directory);
                if path.exists() {
                    crate::ai::change_journal::checked_path(root,&path)?;
                    discover(root,&path,1,&mut files)?;
                }
            }
            files.retain(|(_,name)|name.ends_with(".c")||name.ends_with(".h"));
        } else {discover(root, root, 0, &mut files)?;}
        files.sort_by(|a, b| a.1.cmp(&b.1));
        // Sidecar data is optional and never executes a URL or instruction.
        let sidecar = root.join(".knowledge-sources.json");
        let baselines: BTreeMap<String, BaselineSource> = serde_json::from_str(include_str!(
            "../../../docs/ai-engineering/knowledge-sources.json"
        ))
        .map_err(|e| e.to_string())?;
        let mut declarations: BTreeMap<String, Metadata> = BTreeMap::new();
        if sidecar.exists() && !self.project_scope {
            if std::fs::symlink_metadata(&sidecar)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Knowledge metadata must not be a symlink.".into());
            }
            let custom: BTreeMap<String, Metadata> = serde_json::from_str(
                &std::fs::read_to_string(&sidecar).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Invalid knowledge metadata: {e}"))?;
            declarations.extend(custom);
        }
        let mut active = HashSet::new();
        for (path, name) in &files {
            check_cancel(cancelled)?;
            report.files_seen += 1;
            active.insert(name.clone());
            if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
                return Err(format!(
                    "Knowledge file exceeds 2 MiB ingestion limit: {name}"
                ));
            }
            let bytes = std::fs::read(path).map_err(|e| format!("Cannot read {name}: {e}"))?;
            let source =
                std::str::from_utf8(&bytes).map_err(|e| format!("Invalid UTF-8 in {name}: {e}"))?;
            let baseline = baselines
                .get(name)
                .filter(|entry| Some(entry.content_sha256.clone()) == crate::ai::privacy::normalized_text_hash(&bytes));
            let mut metadata = declarations
                .get(name)
                .cloned()
                .or_else(|| baseline.map(|entry| entry.metadata.clone()))
                .unwrap_or_default();
            if metadata.title.is_empty() {
                metadata.title = name.clone();
            }
            if !declarations.contains_key(name) && (name.ends_with(".c") || name.ends_with(".h")) {
                metadata.category = "code".into();
            }
            let public_source = !self.project_scope && crate::ai::privacy::bundled_source(name, &bytes);
            let fingerprint = digest(
                format!(
                    "{}|{}|{}|{}|{}|{}|{}",
                    digest(&bytes),
                    serde_json::to_string(&metadata).unwrap(),
                    CHUNK_VERSION,
                    self.config.chunk_chars,
                    self.model_id.as_str(),
                    self.config.dense_enabled,
                    public_source
                )
                .as_bytes(),
            );
            let previous: Option<String> = self
                .store
                .connection()
                .query_row(
                    "SELECT fingerprint FROM ki_sources WHERE path=?1",
                    [name],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if previous.as_ref() == Some(&fingerprint) {
                report.files_unchanged += 1;
                continue;
            }
            let chunks = chunk_source(name, source, self.config.chunk_chars);
            if chunks.len()>4096 {return Err(format!("Source {name} exceeds 4096-chunk limit; prior index was preserved."));}
            let mut vectors: Vec<Option<Vec<f32>>> = vec![None; chunks.len()];
            let mut missing = Vec::new();
            if self.config.dense_enabled {
                for (index, chunk) in chunks.iter().enumerate() {
                    let hash = digest(chunk.content.as_bytes());
                    let cached: Option<Vec<u8>> = self
                        .store
                        .connection()
                        .query_row(
                            "SELECT vector FROM ki_embeddings WHERE hash=?1 AND model=?2",
                            params![hash, self.model_id.as_str()],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(|e| e.to_string())?;
                    if let Some(vector) = cached {
                        let vector = blob_to_f32_vec(&vector);
                        if valid_vector(&vector) {
                            vectors[index] = Some(vector);
                            report.cached_embeddings += 1;
                            continue;
                        }
                    }
                    missing.push(index);
                }
                for batch in missing.chunks(self.config.batch_size) {
                    check_cancel(cancelled)?;
                    let result = embed(
                        batch
                            .iter()
                            .map(|&index| chunks[index].content.clone())
                            .collect(),
                    );
                    match result {
                        Ok(batch_vectors) if batch_vectors.len() == batch.len() && batch_vectors.iter().all(|v| valid_vector(v)) => {
                            for (&index, vector) in batch.iter().zip(batch_vectors) { vectors[index] = Some(vector); report.embedded_chunks += 1; }
                        }
                        Ok(_) => return Err("Embedding output dimensions/count/non-finite values do not match the pinned 384-dimensional model.".into()),
                        Err(error) => { report.warnings.push(format!("Dense unavailable for {name}: {error}; lexical indexing remains available.")); break; }
                    }
                }
            }
            check_cancel(cancelled)?;
            let complete_dense = !self.config.dense_enabled || vectors.iter().all(Option::is_some);
            let tx = self
                .store
                .connection()
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM kb_chunks WHERE file_name=?1", [name])
                .map_err(|e| e.to_string())?;
            for (chunk, vector) in chunks.iter().zip(vectors) {
                let hash = digest(chunk.content.as_bytes());
                let id = digest(
                    format!("{name}:{}:{}:{hash}", chunk.start_line, chunk.end_line).as_bytes(),
                );
                let hit = SearchHit {
                    id: id.clone(),
                    file: name.clone(),
                    content: chunk.content.clone(),
                    start_line: chunk.start_line,
                    end_line: chunk.end_line,
                    parent_start_line: chunk.parent_start_line,
                    parent_end_line: chunk.parent_end_line,
                    symbols: chunk.symbols.clone(),
                    includes: chunk.includes.clone(),
                    kind: chunk.kind.clone(),
                    start_byte: chunk.start_byte, end_byte: chunk.end_byte,
                    parent_start_byte: chunk.parent_start_byte, parent_end_byte: chunk.parent_end_byte,
                    signature: chunk.signature.clone(), parser: chunk.parser.clone(), syntax_error: chunk.syntax_error,
                    metadata: metadata.clone(),
                    content_hash: hash.clone(),
                    indexed_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    score: 0.,
                    method: String::new(),
                    citation: format!("{name}:{}-{}", chunk.start_line, chunk.end_line),
                    bundled: public_source,
                };
                let blob = vector.as_ref().map(|v| f32_slice_to_blob(v));
                tx.execute(
                    "INSERT INTO kb_chunks(file_name,content,embedding) VALUES(?1,?2,?3)",
                    params![name, chunk.content, blob],
                )
                .map_err(|e| e.to_string())?;
                tx.execute("INSERT INTO ki_chunks(chunk_id,stable_id,info,model_id,content_hash) VALUES(?1,?2,?3,?4,?5)", params![tx.last_insert_rowid(), id, serde_json::to_string(&hit).unwrap(), vector.as_ref().map(|_| self.model_id.as_str()), hash]).map_err(|e| e.to_string())?;
                if let Some(blob) = blob {
                    tx.execute(
                        "INSERT OR REPLACE INTO ki_embeddings(hash,model,vector) VALUES(?1,?2,?3)",
                        params![hash, self.model_id.as_str(), blob],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
            // Failed embeddings are retried next ingestion; the lexical copy is usable now.
            tx.execute(
                "INSERT OR REPLACE INTO ki_sources(path,fingerprint) VALUES(?1,?2)",
                params![
                    name,
                    if complete_dense {
                        fingerprint.as_str()
                    } else {
                        "dense-incomplete"
                    }
                ],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            report.files_updated += 1;
        }
        check_cancel(cancelled)?;
        let indexed = self.store.list_indexed_files()?;
        let tx = self
            .store
            .connection()
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        for (name, _) in indexed {
            if !active.contains(&name) {
                tx.execute("DELETE FROM kb_chunks WHERE file_name=?1", [&name])
                    .map_err(|e| e.to_string())?;
                report.files_deleted += 1;
            }
        }
        let recorded: Vec<String> = {
            let mut s = tx
                .prepare("SELECT path FROM ki_sources")
                .map_err(|e| e.to_string())?;
            let names = s
                .query_map([], |r| r.get(0))
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            names
        };
        for name in recorded {
            if !active.contains(&name) {
                tx.execute("DELETE FROM ki_sources WHERE path=?1", [name])
                    .map_err(|e| e.to_string())?;
            }
        }
        // Prevent unlimited retention of private/deleted content embeddings.
        tx.execute(
            "DELETE FROM ki_embeddings WHERE hash NOT IN (SELECT content_hash FROM ki_chunks)",
            [],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        report.chunks = self.store.chunk_count();
        Ok(report)
    }

    pub fn search(
        &self,
        query: &str,
        embedding: Option<&[f32]>,
        filter: &SearchFilter,
    ) -> Result<Vec<SearchHit>, String> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }
        if query.chars().count() > 4000 {
            return Err("Retrieval query exceeds 4000 characters.".into());
        }
        if let Some(vector) = embedding {
            if !valid_vector(vector) {
                return Err("Query embedding must contain 384 finite values.".into());
            }
        }
        let mut channels: Vec<(f64, Vec<(i64, SearchHit)>)> = Vec::new();
        let terms: Vec<String> = query
            .split_whitespace()
            .take(24)
            .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
            .collect();
        let expression = terms.join(" OR ");
        // Exact API identifiers prefer actual code records over incidental prose
        // mentions, while natural-language questions retain BM25 ordering.
        let technical =
            query.contains('_') && query.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        let code_order = if technical {
            "CASE WHEN json_extract(m.info,'$.metadata.category')='code' AND json_extract(m.info,'$.metadata.verification') LIKE 'firmware-build-tested%' THEN 0 WHEN json_extract(m.info,'$.metadata.category')='code' THEN 1 ELSE 2 END,"
        } else {
            ""
        };
        // FTS syntax is constructed from quoted literal terms, never raw user MATCH syntax.
        let expansion=super::query_expansion::plan(query);
        let expanded=expansion.terms.iter().map(|s|format!("\"{s}\"")).collect::<Vec<_>>().join(" OR ");
        let mut lexical_channels=vec![
            ("ki_fts", self.config.lexical_weight, expression.as_str()),
            ("ki_trigram", self.config.lexical_weight * 0.5, expression.as_str()),
        ];
        if self.config.technical_expansion && !expanded.is_empty() {
            lexical_channels.push(("ki_fts",self.config.lexical_weight * 0.35,expanded.as_str()));
        }
        for (table, weight, match_query) in lexical_channels {
            if table == "ki_trigram"
                && !query
                    .chars()
                    .any(|c| ('\u{0e00}'..='\u{0e7f}').contains(&c))
            {
                continue;
            }
            let sql = format!("SELECT c.id,m.info FROM {table} JOIN kb_chunks c ON c.id={table}.rowid JOIN ki_chunks m ON m.chunk_id=c.id WHERE {table} MATCH :query AND {} ORDER BY {code_order}bm25({table}),m.stable_id LIMIT :limit", FILTER_SQL);
            let mut statement = self
                .store
                .connection()
                .prepare(&sql)
                .map_err(|e| e.to_string())?;
            let rows = statement.query_map(named_params! { ":query": match_query, ":limit": self.config.candidate_k, ":board": filter.board, ":target": filter.target, ":sdk": filter.sdk_version, ":unknown": filter.include_unknown, ":public": filter.bundled_only }, |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).map_err(|e| e.to_string())?;
            let mut hits = Vec::new();
            for row in rows {
                let (id, info) = row.map_err(|e| e.to_string())?;
                hits.push((
                    id,
                    serde_json::from_str::<SearchHit>(&info)
                        .map_err(|e| format!("Corrupt chunk metadata; rebuild index: {e}"))?,
                ));
            }
            channels.push((weight, hits));
        }
        if let Some(vector) = embedding.filter(|_| self.config.needs_dense(query)) {
            // Stream rows, retain only bounded candidates; do not load whole corpus into RAM.
            let sql = format!("SELECT c.id,m.info,c.embedding FROM kb_chunks c JOIN ki_chunks m ON m.chunk_id=c.id WHERE m.model_id=:model AND {}", FILTER_SQL);
            let mut statement = self
                .store
                .connection()
                .prepare(&sql)
                .map_err(|e| e.to_string())?;
            let rows = statement.query_map(named_params! { ":model": self.model_id.as_str(), ":board": filter.board, ":target": filter.target, ":sdk": filter.sdk_version, ":unknown": filter.include_unknown, ":public": filter.bundled_only }, |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Vec<u8>>(2)?))).map_err(|e| e.to_string())?;
            let mut best = Vec::new();
            for row in rows {
                let (id, info, blob) = row.map_err(|e| e.to_string())?;
                let candidate = blob_to_f32_vec(&blob);
                if blob.len() != 384 * 4 || !valid_vector(&candidate) {
                    continue;
                }
                let score = cosine_similarity(vector, &candidate);
                if score < 0.15 {
                    continue;
                }
                best.push((score, id, info));
                best.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
                best.truncate(self.config.candidate_k);
            }
            let hits = best
                .into_iter()
                .map(|(_, id, info)| {
                    serde_json::from_str::<SearchHit>(&info)
                        .map(|hit| (id, hit))
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            channels.push((self.config.dense_weight, hits));
        }
        let mut merged: HashMap<i64, SearchHit> = HashMap::new();
        for (weight, hits) in channels {
            if weight == 0.0 {
                continue;
            }
            for (rank, (id, mut hit)) in hits.into_iter().enumerate() {
                hit.score = weight / (self.config.rrf_k + rank as f64 + 1.);
                hit.method = if embedding.is_some()
                    && self.config.dense_enabled
                    && self.config.dense_weight > 0.
                {
                    "hybrid-rrf"
                } else {
                    "lexical-bm25"
                }
                .into();
                merged
                    .entry(id)
                    .and_modify(|current| current.score += hit.score)
                    .or_insert(hit);
            }
        }
        let mut hits: Vec<_> = merged.into_values().collect();
        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        let mut hashes = HashSet::new();
        hits.retain(|hit| hashes.insert(hit.content_hash.clone()));
        hits.truncate(self.config.top_k);
        // Bound every returned payload, including headers/citations/metadata. A very
        // long C line is clipped only in context, never in its stored source record.
        let mut remaining = self
            .config
            .max_context_chars
            .saturating_sub(2 + self.config.top_k);
        let mut bounded = Vec::new();
        for mut hit in hits {
            let overhead = serde_json::to_string(&hit)
                .map_err(|e| e.to_string())?
                .chars()
                .count()
                .saturating_sub(hit.content.chars().count());
            if remaining <= overhead {
                break;
            }
            hit.content = hit.content.chars().take(remaining - overhead).collect();
            let size = serde_json::to_string(&hit).unwrap().chars().count();
            if size > remaining {
                continue;
            }
            remaining -= size;
            bounded.push(hit);
        }
        Ok(bounded)
    }
}

const FILTER_SQL: &str = "(:public=0 OR json_extract(m.info,'$.bundled')=1) AND (:board IS NULL OR json_extract(m.info,'$.metadata.board')=:board OR (:unknown AND json_extract(m.info,'$.metadata.board') IS NULL)) AND (:target IS NULL OR json_extract(m.info,'$.metadata.target')=:target OR (:unknown AND json_extract(m.info,'$.metadata.target') IS NULL)) AND (:sdk IS NULL OR json_extract(m.info,'$.metadata.sdk_version')=:sdk OR (:unknown AND json_extract(m.info,'$.metadata.sdk_version') IS NULL))";
fn valid_vector(vector: &[f32]) -> bool {
    vector.len() == 384 && vector.iter().all(|v| v.is_finite()) && vector.iter().any(|v| *v != 0.)
}
fn check_cancel(flag: &AtomicBool) -> Result<(), String> {
    if flag.load(Ordering::Relaxed) {
        Err("Knowledge operation cancelled; completed file transactions were retained.".into())
    } else {
        Ok(())
    }
}
fn discover(
    root: &Path,
    current: &Path,
    depth: usize,
    out: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    if depth > 12 {
        return Err("Knowledge tree exceeds depth limit 12.".into());
    }
    for entry in
        std::fs::read_dir(current).map_err(|e| format!("Cannot discover knowledge sources: {e}"))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || matches!(name.as_str(), "build" | "target" | "node_modules") {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;
            if std::fs::symlink_metadata(entry.path()).map_err(|e|e.to_string())?.file_attributes() & 0x400 != 0 {continue;}
        }
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() {
            discover(root, &path, depth + 1, out)?;
        } else if kind.is_file()
            && (matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("md" | "txt" | "c" | "h" | "cmake")
            ) || name == "CMakeLists.txt")
        {
            out.push((
                path.clone(),
                path.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .replace('\\', "/"),
            ));
            if out.len() > 2048 {
                return Err("Knowledge tree exceeds 2048-file ingestion limit.".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lexical() -> RetrievalConfig {
        RetrievalConfig {
            dense_enabled: false,
            ..Default::default()
        }
    }
    fn index(engine: &Engine, root: &Path) -> IndexReport {
        engine
            .ingest(root, &AtomicBool::new(false), |_| {
                panic!("lexical index must not call embeddings")
            })
            .unwrap()
    }
    fn search(engine: &Engine, q: &str) -> Vec<SearchHit> {
        engine.search(q, None, &SearchFilter::default()).unwrap()
    }
    #[test]
    fn exact_api_is_not_a_like_wildcard_and_thai_substring_is_searchable() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("good.h"),
            "int gpio_set_level(int pin, int level);\n// อ่านอุณหภูมิจากตัวตรวจจับ",
        )
        .unwrap();
        std::fs::write(
            root.path().join("bad.h"),
            "int gpioXsetXlevel(void); // unrelated",
        )
        .unwrap();
        let engine = Engine::open(root.path(), lexical()).unwrap();
        index(&engine, root.path());
        assert_eq!(search(&engine, "gpio_set_level")[0].file, "good.h");
        assert!(search(&engine, "gpio_set_level")
            .iter()
            .all(|h| h.file != "bad.h"));
        assert_eq!(search(&engine, "อุณหภูมิ")[0].file, "good.h");
        assert!(search(&engine, "zzzz_no_match").is_empty());
        assert!(engine
            .search("\" OR * --", None, &SearchFilter::default())
            .is_ok());
    }
    #[test]
    fn content_hash_detects_same_time_edits_empty_files_disable_and_last_deletion() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.md");
        std::fs::write(&path, "first_symbol").unwrap();
        let engine = Engine::open(root.path(), lexical()).unwrap();
        assert_eq!(index(&engine, root.path()).files_updated, 1);
        assert_eq!(index(&engine, root.path()).files_unchanged, 1);
        std::fs::write(&path, "second_symbol").unwrap();
        assert_eq!(index(&engine, root.path()).files_updated, 1);
        assert!(search(&engine, "first_symbol").is_empty());
        std::fs::write(&path, "").unwrap();
        assert_eq!(index(&engine, root.path()).chunks, 0);
        std::fs::write(&path, "back_symbol").unwrap();
        index(&engine, root.path());
        std::fs::rename(&path, root.path().join("a.md.disabled")).unwrap();
        assert_eq!(index(&engine, root.path()).chunks, 0);
        assert!(search(&engine, "back_symbol").is_empty());
    }
    #[test]
    fn metadata_filters_run_before_candidate_limits_and_update_without_source_edit() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("v5.md"), "gpio_config_t modern").unwrap();
        std::fs::write(root.path().join("v4.md"), "gpio_config_t historical").unwrap();
        let path = root.path().join(".knowledge-sources.json");
        std::fs::write(&path,r#"{"v5.md":{"sdk_version":"5.5","target":"esp32"},"v4.md":{"sdk_version":"4.4","target":"esp32"}}"#).unwrap();
        let engine = Engine::open(root.path(), lexical()).unwrap();
        index(&engine, root.path());
        let filter = SearchFilter {
            sdk_version: Some("5.5".into()),
            ..Default::default()
        };
        let hits = engine.search("gpio_config_t", None, &filter).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file, "v5.md");
        std::fs::write(
            &path,
            r#"{"v5.md":{"sdk_version":"6.0"},"v4.md":{"sdk_version":"4.4"}}"#,
        )
        .unwrap();
        assert!(index(&engine, root.path()).files_updated > 0);
        assert!(engine
            .search("gpio_config_t", None, &filter)
            .unwrap()
            .is_empty());
    }
    #[test]
    fn expansion_combines_original_evidence_and_retains_all_scope_filters() {
        let root=tempfile::tempdir().unwrap();
        for (name,text) in [("original.md","พักการทำงาน source"),("api.c","void vTaskDelay(void);"),("wrong.c","void vTaskDelay(void); // incompatible"),("unknown.c","void vTaskDelay(void); // unknown")] {std::fs::write(root.path().join(name),text).unwrap();}
        std::fs::write(root.path().join(".knowledge-sources.json"),r#"{"original.md":{"board":"b","target":"esp32","sdk_version":"5.5.4"},"api.c":{"board":"b","target":"esp32","sdk_version":"5.5.4"},"wrong.c":{"board":"other","target":"esp32s3","sdk_version":"4.4"}}"#).unwrap();
        let mut engine=Engine::open(root.path(),RetrievalConfig{technical_expansion:true,dense_enabled:false,..Default::default()}).unwrap();
        index(&engine,root.path());
        let filter=SearchFilter{board:Some("b".into()),target:Some("esp32".into()),sdk_version:Some("5.5.4".into()),..Default::default()};
        let hits=engine.search("พักการทำงาน",None,&filter).unwrap();
        assert!(hits.iter().any(|h|h.file=="original.md"));assert!(hits.iter().any(|h|h.file=="api.c"));
        assert!(hits.iter().all(|h|h.file!="wrong.c"&&h.file!="unknown.c"));
        assert!(engine.search("พักการทำงาน",None,&SearchFilter{bundled_only:true,..filter.clone()}).unwrap().is_empty());
        assert!(engine.search("พักการทำงาน",None,&SearchFilter{sdk_version:Some("5.5".into()),..filter}).unwrap().is_empty());
        let before=engine.search("vTaskDelay",None,&SearchFilter::default()).unwrap().iter().map(|h|h.id.clone()).collect::<Vec<_>>();
        engine.config.technical_expansion=false;
        let after=engine.search("vTaskDelay",None,&SearchFilter::default()).unwrap().iter().map(|h|h.id.clone()).collect::<Vec<_>>();
        assert_eq!(before,after);
    }
    #[test]
    fn dense_and_lexical_are_fused_even_when_lexical_finds_a_match() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.md"), "gpio_set_level pin output").unwrap();
        std::fs::write(
            root.path().join("b.md"),
            "A semantically related output voltage guide",
        )
        .unwrap();
        let engine = Engine::open(root.path(), RetrievalConfig::default()).unwrap();
        let mut vector = vec![0.; 384];
        vector[0] = 1.;
        engine
            .ingest(root.path(), &AtomicBool::new(false), |texts| {
                Ok(vec![vector.clone(); texts.len()])
            })
            .unwrap();
        let hits = engine
            .search("gpio_set_level", Some(&vector), &SearchFilter::default())
            .unwrap();
        assert_eq!(hits[0].file, "a.md");
        assert!(hits.iter().any(|h| h.file == "b.md"));
        assert!(hits.iter().all(|h| h.method == "hybrid-rrf"));
    }
    #[test]
    fn schema_migration_preserves_legacy_rows_but_never_trusts_unknown_vectors() {
        let root = tempfile::tempdir().unwrap();
        let store = KbStore::open(root.path()).unwrap();
        store
            .upsert_file_chunks("old.md", &[("legacy_token".into(), vec![1.; 384])], 1)
            .unwrap();
        drop(store);
        let engine = Engine::open(root.path(), lexical()).unwrap();
        assert_eq!(engine.store.chunk_count(), 1);
        assert!(search(&engine, "legacy_token").is_empty());
        std::fs::write(root.path().join("old.md"), "legacy_token").unwrap();
        index(&engine, root.path());
        assert_eq!(search(&engine, "legacy_token").len(), 1);
        Engine::open(root.path(), lexical()).unwrap(); // idempotent migration
    }
    #[test]
    fn failure_cancellation_and_corrupt_database_do_not_destroy_existing_data() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("a.md");
        std::fs::write(&source, "retained_token").unwrap();
        let engine = Engine::open(root.path(), lexical()).unwrap();
        index(&engine, root.path());
        assert!(engine
            .ingest(root.path(), &AtomicBool::new(true), |_| unreachable!())
            .is_err());
        std::fs::write(&source, [0xff]).unwrap();
        assert!(engine
            .ingest(root.path(), &AtomicBool::new(false), |_| unreachable!())
            .is_err());
        assert_eq!(search(&engine, "retained_token").len(), 1);
        let other = tempfile::tempdir().unwrap();
        let db = other.path().join(".kb_store.sqlite");
        std::fs::write(&db, b"corrupt keep me").unwrap();
        assert!(Engine::open(other.path(), lexical()).is_err());
        assert_eq!(std::fs::read(&db).unwrap(), b"corrupt keep me");
    }
    #[test]
    fn invalid_vectors_leave_previous_index_and_embeddings_cache_skips_unchanged_work() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.md");
        std::fs::write(&path, "cached_token").unwrap();
        let engine = Engine::open(root.path(), RetrievalConfig::default()).unwrap();
        engine
            .ingest(root.path(), &AtomicBool::new(false), |texts| {
                Ok(vec![vec![1.; 384]; texts.len()])
            })
            .unwrap();
        let report = engine
            .ingest(root.path(), &AtomicBool::new(false), |_| {
                panic!("unchanged source was embedded again")
            })
            .unwrap();
        assert_eq!(report.files_unchanged, 1);
        std::fs::write(&path, "invalid_replacement").unwrap();
        assert!(engine
            .ingest(root.path(), &AtomicBool::new(false), |_| Ok(vec![
                vec![f32::NAN; 384]
            ]))
            .is_err());
        assert_eq!(search(&engine, "cached_token").len(), 1);
    }
    #[test]
    fn source_replacement_is_transactional_in_legacy_api_too() {
        let root = tempfile::tempdir().unwrap();
        let engine = Engine::open(root.path(), lexical()).unwrap();
        engine
            .store
            .upsert_file_chunks("a.md", &[("keep".into(), vec![1.; 384])], 0)
            .unwrap();
        engine.store.connection().execute_batch("CREATE TRIGGER fail_insert BEFORE INSERT ON kb_chunks WHEN new.content='bad' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(engine
            .store
            .upsert_file_chunks("a.md", &[("bad".into(), vec![])], 0)
            .is_err());
        assert_eq!(engine.store.search_keyword("keep", 1).unwrap().len(), 1);
    }
    #[test]
    fn configuration_rejects_unbounded_or_nonfinite_work() {
        let mut c = RetrievalConfig::default();
        c.top_k = 1000;
        assert!(c.validate().is_err());
        c = RetrievalConfig::default();
        c.dense_weight = f64::NAN;
        assert!(c.validate().is_err());
        assert!(serde_json::from_str::<RetrievalConfig>(r#"{"remote_upload":true}"#).is_err());
    }
    #[test]
    fn private_documents_cannot_override_public_only_filter() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("private.md"), "private_token confidential").unwrap();
        std::fs::write(
            root.path().join("support_scope.md"),
            "private_token changed builtin filename",
        )
        .unwrap();
        let engine = Engine::open(root.path(), lexical()).unwrap();
        index(&engine, root.path());
        let filter = SearchFilter {
            bundled_only: true,
            include_unknown: true,
            ..Default::default()
        };
        assert!(engine
            .search("private_token", None, &filter)
            .unwrap()
            .is_empty());
        assert!(!search(&engine, "private_token").is_empty());
        assert!(serde_json::from_str::<SearchFilter>(
            r#"{"include_unknown":true,"bundled_only":false}"#
        )
        .is_err());
    }
    #[test]
    fn serialized_context_budget_includes_metadata_and_array_delimiters() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("long.md"), "unique_token ".repeat(3000)).unwrap();
        let config = RetrievalConfig {
            dense_enabled: false,
            max_context_chars: 1000,
            ..Default::default()
        };
        let engine = Engine::open(root.path(), config).unwrap();
        index(&engine, root.path());
        let hits = search(&engine, "unique_token");
        assert!(!hits.is_empty());
        assert!(serde_json::to_string(&hits).unwrap().chars().count() <= 1000);
        assert_eq!(hits[0].start_line, 1);
    }
}
