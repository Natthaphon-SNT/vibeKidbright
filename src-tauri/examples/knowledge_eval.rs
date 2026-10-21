//! Run against a COPY of the source KB in a temporary directory. No user index writes.
use serde_json::json;
use std::{io::{Seek, SeekFrom, Write}, path::Path, sync::atomic::AtomicBool, time::Instant};
use tauri_app_lib::intelligence::evaluation::{metrics, QueryCase};
use tauri_app_lib::intelligence::{Engine, RetrievalConfig, SearchFilter};
use fastembed::{InitOptionsUserDefined,Pooling,TextEmbedding,TokenizerFiles,UserDefinedEmbeddingModel};

fn reserve_report(path: &Path) -> std::io::Result<std::fs::File> {
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(b"{\"status\":\"INCOMPLETE\",\"reason\":\"Reserved before evaluation; success not established\"}\n")?;
    Ok(file)
}

fn copy_sources(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name();
        if kind.is_symlink() || name.to_string_lossy().starts_with('.') {
            continue;
        }
        if kind.is_dir() {
            copy_sources(&entry.path(), &target.join(name))?;
        } else if matches!(
            entry.path().extension().and_then(|s| s.to_str()),
            Some("md" | "txt" | "c" | "h" | "cmake")
        ) {
            std::fs::copy(entry.path(), target.join(name))?;
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let args: Vec<_> = std::env::args().collect();
    let dense = args.iter().any(|a| a == "--dense");
    let option = |name:&str| args.windows(2).find(|a|a[0]==name).map(|a|a[1].clone());
    let output=option("--output").ok_or("Use --output <new-report.json> [--dense]")?;
    // Fail before loading a model or doing work if evidence already exists.
    let mut output_file=reserve_report(Path::new(&output))?;
    let dataset=option("--dataset").unwrap_or_else(||root.join("docs/ai-engineering/evaluation-queries.json").to_string_lossy().into_owned());
    let dataset_bytes=std::fs::read(&dataset)?;
    let cases:Vec<QueryCase>=serde_json::from_slice(&dataset_bytes)?;
    let model_directory=option("--model-directory");
    if model_directory.is_some() && !dense {return Err("Candidate model requires --dense".into());}
    let mut model_identity=tauri_app_lib::intelligence::model_identity().to_owned();
    let (mut query_prefix,mut passage_prefix)=(String::new(),String::new());
    let load=Instant::now();
    let mut candidate=None;
    let directory=model_directory.as_ref().map(std::path::PathBuf::from).unwrap_or_else(||root.join("resources/embedding"));
    if model_directory.is_some() {
        let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
        model_identity=manifest["model_identity"].as_str().ok_or("Missing candidate identity")?.into();
        query_prefix=manifest["query_prefix"].as_str().unwrap_or("").into();
        passage_prefix=manifest["passage_prefix"].as_str().unwrap_or("").into();
        let read=|name:&str|std::fs::read(directory.join(name));
        for item in manifest["artifacts"].as_array().ok_or("Missing artifact manifest")? {
            let name=item["file"].as_str().ok_or("Missing artifact file")?;
            if !matches!(name,"model.onnx"|"model-config.json"|"tokenizer.json"|"tokenizer_config.json"|"special_tokens_map.json") {return Err("Invalid candidate artifact name".into());}
            if tauri_app_lib::intelligence::digest(&read(name)?)!=item["sha256"].as_str().ok_or("Missing artifact digest")? {return Err("Candidate artifact changed after preparation".into());}
        }
        let tokenizer=TokenizerFiles{tokenizer_file:read("tokenizer.json")?,config_file:read("model-config.json")?,special_tokens_map_file:read("special_tokens_map.json")?,tokenizer_config_file:read("tokenizer_config.json")?};
        candidate=Some(TextEmbedding::try_new_from_user_defined(UserDefinedEmbeddingModel::new(read("model.onnx")?,tokenizer).with_pooling(Pooling::Mean),InitOptionsUserDefined::default())?);
    }
    if dense && candidate.is_none() {tauri_app_lib::intelligence::local_embeddings(vec!["initialization".into()])?;}
    let model_load_ms=load.elapsed().as_secs_f64()*1000.;
    let embed = |texts:Vec<String>,prefix:&str|->Result<Vec<Vec<f32>>,String>{
        let texts:Vec<_>=texts.into_iter().map(|s|format!("{prefix}{s}")).collect();
        if let Some(model)=&candidate {model.embed(texts,None).map_err(|e|e.to_string())}else{tauri_app_lib::intelligence::local_embeddings(texts)}
    };
    let scratch = tempfile::tempdir()?;
    copy_sources(&root.join("knowledge_base"), scratch.path())?;
    let config = RetrievalConfig {
        dense_enabled: dense,
        query_routing: args.iter().any(|a|a=="--routing"),
        technical_expansion: args.iter().any(|a|a=="--expand"),
        ..Default::default()
    };
    let engine = Engine::open_for_evaluation(scratch.path(), config.clone(), &model_identity)?;
    let started = Instant::now();
    let report = engine.ingest(
        scratch.path(),
        &AtomicBool::new(false),
        |texts|embed(texts,&passage_prefix),
    )?;
    let index_ms = started.elapsed().as_secs_f64() * 1000.;
    let started = Instant::now();
    let unchanged = engine.ingest(scratch.path(), &AtomicBool::new(false), |_| {
        panic!("unchanged corpus must not be re-embedded")
    })?;
    let unchanged_ms = started.elapsed().as_secs_f64() * 1000.;
    let mut rows = Vec::new();
    let mut total = [0.; 4];
    let mut baseline = [0.; 4];
    for case in &cases {
        let started = Instant::now();
        let vector = if config.needs_dense(&case.query) {
            Some(embed(vec![case.query.clone()],&query_prefix)?.remove(0))
        } else {
            None
        };
        let hits = engine.search(
            &case.query,
            vector.as_deref(),
            &SearchFilter {
                include_unknown: true,
                ..Default::default()
            },
        )?;
        let elapsed = started.elapsed().as_secs_f64() * 1000.;
        let files: Vec<_> = hits.iter().map(|h| h.file.clone()).collect();
        let measured = metrics(&files, &case.relevant_files);
        let old = engine.legacy_keyword_files(&case.query, 5)?;
        let old_score = metrics(&old, &case.relevant_files);
        for (sum, score) in total.iter_mut().zip([
            measured.recall_at_5,
            measured.precision_at_5,
            measured.mrr,
            measured.ndcg_at_5,
        ]) {
            *sum += score;
        }
        for (sum, score) in baseline.iter_mut().zip([
            old_score.recall_at_5,
            old_score.precision_at_5,
            old_score.mrr,
            old_score.ndcg_at_5,
        ]) {
            *sum += score;
        }
        let evidence:Vec<_>=hits.iter().map(|hit|json!({"id":hit.id,"file":hit.file,"citation":hit.citation,"score":hit.score,"method":hit.method,"content_hash":hit.content_hash,"start_byte":hit.start_byte,"end_byte":hit.end_byte,"parser":hit.parser,"kind":hit.kind,"metadata":hit.metadata,"returned_characters":hit.content.chars().count()})).collect();
        rows.push(json!({"id":case.id,"query":case.query,"query_plan":tauri_app_lib::intelligence::query_expansion::plan(&case.query),"language":case.language,"relevant_files":case.relevant_files,"ranked_files":files,"hits":evidence,"metrics":measured,"legacy_like_files":old,"legacy_like_metrics":old_score,"latency_ms":elapsed}));
    }
    let count = cases.len() as f64;
    let mean = |v: [f64; 4]| json!({"recall_at_5":v[0]/count,"precision_at_5":v[1]/count,"mrr":v[2]/count,"ndcg_at_5":v[3]/count});
    let result = json!({"schema_version":1,"date":"2026-10-17","mode":if dense{"real-local-dense-plus-lexical"}else{"lexical-only"},"model":if dense{Some(tauri_app_lib::intelligence::model_identity())}else{None},"query_count":cases.len(),"config":config,"index":report,"index_latency_ms":index_ms,"macro_average":mean(total),"legacy_like_macro_average":mean(baseline),"queries":rows,"limits":["Small manually labelled repository smoke dataset, not an independent or exhaustive benchmark","Relevance is file-level; top 5 chunk results are deduplicated for document metrics; Precision@5 always divides by 5","Legacy LIKE and new search share the new chunk corpus, isolating ranking rather than reproducing the whole old pipeline","No LLM, remote embedding, firmware build, flash or hardware test performed by this runner","Model comparison, grounded-answer quality and measured API correctness are not evaluated"]});
    let mut result = result;
    result["unchanged_index"] = json!({"report":unchanged,"latency_ms":unchanged_ms});
    result["date"]=json!(option("--date").unwrap_or_else(||"not-supplied".into()));
    result["evaluation_source_sha256"]=json!(tauri_app_lib::intelligence::digest(include_bytes!("knowledge_eval.rs")));
    result["model"]=if dense {json!(model_identity)}else{json!(null)};
    result["model_load_ms"]=json!(model_load_ms);
    result["model_artifact_bytes"]=json!(if dense {std::fs::read_dir(&directory)?.filter_map(|e|e.ok()).filter_map(|e|e.metadata().ok()).filter(|m|m.is_file()).map(|m|m.len()).sum::<u64>()}else{0});
    result["dataset_sha256"] = json!(tauri_app_lib::intelligence::digest(&dataset_bytes));
    result["dataset"]=json!(Path::new(&dataset).file_name().unwrap().to_string_lossy());
    result["limits"]=json!(["Agent-authored non-exhaustive file labels; not a human independent benchmark or grounded-answer evaluation","No held-out tuning; fixed weights and routing recorded before evaluation","Relevance deduplicates only the top five chunks; does not refill document ranks","No LLM/provider, firmware build, flash or hardware test by this runner"]);
    result["scope"] = json!({"private_tool_export":false,"source_directory":"knowledge_base (temporary copy)","operating_system":std::env::consts::OS,"architecture":std::env::consts::ARCH});
    output_file.set_len(0)?;output_file.seek(SeekFrom::Start(0))?;
    output_file.write_all(serde_json::to_string_pretty(&result)?.as_bytes())?;
    output_file.sync_all()?;
    println!(
        "Evaluation saved: {} ({} queries; {})",
        output,
        cases.len(),
        if dense {
            "real local model"
        } else {
            "lexical only"
        }
    );
    Ok(())
}

#[cfg(test)]
mod report_safety {
    use super::*;
    #[test]
    fn existing_evidence_is_never_truncated_and_interrupted_work_is_not_success() {
        let dir=tempfile::tempdir().unwrap();let path=dir.path().join("report.json");
        let file=reserve_report(&path).unwrap();drop(file);
        let before=std::fs::read(&path).unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&before).unwrap()["status"],"INCOMPLETE");
        assert!(reserve_report(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(),before);
    }
}
