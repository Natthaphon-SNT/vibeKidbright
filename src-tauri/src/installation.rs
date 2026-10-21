//! Headless installation smoke check, runnable from a copied installation directory.
//! It uses only bundled assets and an explicitly chosen scratch data directory.
use std::path::Path;
use serde_json::{json, Value};

pub fn self_check(resources: &Path, data: &Path) -> Result<Value, String> {
    let knowledge = data.join("knowledge_base");
    let copied = crate::ai_chat::copy_bundled_knowledge(&resources.join("knowledge_base"), &knowledge)?;
    if copied < 10 || !knowledge.join("support_scope.md").is_file() {
        return Err("Bundled knowledge is incomplete.".into());
    }
    for relative in ["compatibility/kb_core.c", "compatibility/button_buzzer.c", "sensor_examples/temp_lm73.c"] {
        if !knowledge.join(relative).is_file() {
            return Err(format!("Bundled knowledge directory structure is incomplete: {relative}"));
        }
    }
    let probe = data.join("installation-check.json");
    crate::ai::config::write_json_atomic(&probe, &json!({ "writable": true }))?;
    if crate::ai::config::read_json(&probe)?["writable"] != true { return Err("Settings read-back failed.".into()); }
    crate::kb_embed::init_local_embedder(Some(&resources.join("embedding")))?;
    let vectors = crate::kb_embed::embed_texts(vec!["KidBright temperature sensor".into(), "อ่านค่าอุณหภูมิจากบอร์ด".into()])?;
    if vectors.len() != 2 || vectors.iter().any(|vector| vector.len() != 384 || vector.iter().any(|value| !value.is_finite())
        || vector.iter().map(|value| value * value).sum::<f32>() < 0.5) {
        return Err("Offline embedding output is invalid.".into());
    }
    let store = crate::kb_store::KbStore::open(&knowledge)?;
    store.upsert_file_chunks("installation-check", &[("KidBright temperature sensor".into(), vectors[0].clone())], 1)?;
    if store.search_vector(&vectors[0], 1, 0.1)?.is_empty() { return Err("Local knowledge search failed.".into()); }
    let index_directory=data.join("intelligence-probe");
    std::fs::create_dir_all(&index_directory).map_err(|error|format!("Cannot create installation index probe: {error}"))?;
    let engine=crate::intelligence::Engine::open(&index_directory,crate::intelligence::RetrievalConfig{dense_enabled:false,..Default::default()})?;
    let index=engine.ingest(&knowledge,&std::sync::atomic::AtomicBool::new(false),|_|Err("No embeddings expected for lexical probe".into()))?;
    let syntax_hits=engine.search("kb_delay_ms",None,&crate::intelligence::SearchFilter{include_unknown:true,..Default::default()})?;
    if !syntax_hits.iter().any(|h|h.parser.starts_with("tree-sitter-c@0.24.2")&&h.end_byte>h.start_byte&&h.kind=="function_definition"){return Err("Packaged Tree-sitter/knowledge search probe failed".into());}
    crate::ai::change_journal::initialize(data)?;
    crate::ai::project_review::initialize(data)?;
    crate::ai::repair::initialize(data)?;
    let workspace=data.join("review-probe");std::fs::create_dir(&workspace).map_err(|e|e.to_string())?;
    let file=workspace.join("main.c");std::fs::write(&file,"original").map_err(|e|e.to_string())?;
    let journal_path=data.join("installation-review-probe.json");
    let mut journal=crate::ai::change_journal::Journal::open(journal_path.clone())?;
    let proposal=journal.propose(&workspace,&file,"installation-probe","reviewed")?;
    if std::fs::read_to_string(&file).map_err(|e|e.to_string())?!="original"{return Err("Proposal unexpectedly changed source".into());}
    drop(journal);
    let mut journal=crate::ai::change_journal::Journal::open(journal_path.clone())?;
    journal.apply(&[(file.clone(),proposal.revision)])?;drop(journal);
    let mut journal=crate::ai::change_journal::Journal::open(journal_path)?;journal.undo_message("installation-probe")?;
    if std::fs::read_to_string(&file).map_err(|e|e.to_string())?!="original"{return Err("Packaged restart/Undo probe failed".into());}
    let credential_service = format!("vibekidbright-installation-check-{}", std::process::id());
    let credential = keyring::Entry::new(&credential_service, "installation-check").map_err(|error| error.to_string())?;
    credential.set_password("temporary-installation-probe").map_err(|error| format!("Credential store write failed: {error}"))?;
    let credential_read = credential.get_password();
    let removed = credential.delete_credential();
    if credential_read.map_err(|error| error.to_string())? != "temporary-installation-probe" { return Err("Credential store read-back failed.".into()); }
    removed.map_err(|error| error.to_string())?;
    Ok(json!({ "version": env!("CARGO_PKG_VERSION"), "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "knowledge_files_copied": copied, "settings_write_and_read": true, "embedding_dimensions": 384,
        "embedding_samples": vectors.len(), "local_vector_search": true, "native_credential_roundtrip": true,
        "tree_sitter_c":"0.24.2", "lexical_intelligence_search":true,"indexed_chunks":index.chunks,
        "durable_review_restart_and_undo":true,"startup_recovery_journals":true,
        "runtime_downloads_required": false, "toolchain_required_for_self_check": false, "hardware_flashed": false }))
}
