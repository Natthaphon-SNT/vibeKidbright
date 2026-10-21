//! Local knowledge engineering. Retrieved content is data, never instructions.
mod chunking;
mod c_parser;
pub mod evaluation;
pub mod diagnostics;
pub mod block_to_c;
pub mod project;
pub mod semantic;
pub mod query_expansion;
mod retrieval;
pub use chunking::{chunk_source, SourceChunk};
pub use retrieval::{
    digest, Engine, IndexReport, Metadata, RetrievalConfig, SearchFilter, SearchHit,
};
pub fn local_embeddings(texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
    crate::kb_embed::embed_texts(texts)
}
pub fn model_identity() -> &'static str {
    retrieval::MODEL_ID
}
