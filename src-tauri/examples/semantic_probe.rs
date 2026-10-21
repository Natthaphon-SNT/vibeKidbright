//! Real optional clangd adapter probe over previously generated public fixtures.
use std::{path::PathBuf, sync::atomic::AtomicBool};
use tauri_app_lib::intelligence::semantic::{self, Buffer, Operation, Query};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 5 {
        return Err(
            "semantic_probe <public-fixtures-root> <clangd> <configured-tools-root> <report.json>"
                .into(),
        );
    }
    let base = dunce::canonicalize(&args[1])?;
    let permitted = dunce::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/block-c-evaluation-20261018"),
    )?;
    if base != permitted {
        return Err("This probe accepts only the existing generated public fixture root".into());
    }
    let cancel = AtomicBool::new(false);
    let tools = PathBuf::from(&args[3]);
    let mut results = Vec::new();
    for name in ["repeat_delay", "forever_delay"] {
        let root = base.join(name);
        let main = root.join("main/main.c");
        let core = root.join("components/kb_reference/kb_core.c");
        let disk = std::fs::read_to_string(&main)?;
        let core_text = std::fs::read_to_string(&core)?;
        for (scenario,text,symbol,operation) in [
            ("definition",disk.clone(),"kb_delay_ms",Operation::Definition),
            ("references",disk.clone(),"kb_delay_ms",Operation::References),
            ("unsaved_definition",format!("{disk}\nstatic int unsaved_only(void) {{ return 42; }}\nint unsaved_caller(void) {{ return unsaved_only(); }}\n"),"unsaved_only",Operation::Definition),
        ]{
            let offset=if scenario=="unsaved_definition"{text.rfind(symbol)}else{text.find(symbol)}.ok_or("Missing fixture symbol")?;
            let prefix=&text[..offset];let query=Query{file_path:main.to_string_lossy().into(),buffers:vec![Buffer{path:main.to_string_lossy().into(),text:text.clone()},Buffer{path:core.to_string_lossy().into(),text:core_text.clone()}],line:prefix.bytes().filter(|b|*b==b'\n').count()as u32,character:prefix.rsplit('\n').next().unwrap().encode_utf16().count()as u32,operation,clangd_path:args[2].clone()};
            let started=std::time::Instant::now();let answer=semantic::query(&root,&tools,&query,&cancel)?;
            if answer.engine!="clangd"||answer.locations.is_empty(){return Err(format!("{name}/{scenario} did not produce semantic locations: {}",answer.limitation).into());}
            if answer.locations.iter().any(|loc|!PathBuf::from(&loc.path).starts_with(&root)){return Err("Cross-project location leak".into());}
            for location in &answer.locations {
                let source=if PathBuf::from(&location.path)==main{&text}else if PathBuf::from(&location.path)==core{&core_text}else{return Err("Unexpected fixture location".into())};
                let row:Vec<u16>=source.lines().nth(location.line as usize).ok_or("Location line outside snapshot")?.encode_utf16().collect();
                let selected=row.get(location.character as usize..location.end_character as usize).ok_or("Location column outside snapshot")?;
                if location.line!=location.end_line||String::from_utf16(selected)?!=symbol{return Err("Returned semantic range is not the requested identifier".into());}
            }
            if scenario=="definition"&&!answer.locations.iter().all(|loc|PathBuf::from(&loc.path)==core&&core_text.lines().nth(loc.line as usize).is_some_and(|line|line.starts_with("bool kb_delay_ms("))){return Err("Wrong kb_delay_ms definition".into());}
            if scenario=="references"&&answer.locations.len()!=if name=="repeat_delay"{3}else{2}{return Err("Fixture reference count does not match its known call sites and definition".into());}
            if scenario=="unsaved_definition"&&!answer.locations.iter().any(|loc|loc.line>=disk.lines().count()as u32){return Err("Unsaved definition was not resolved".into());}
            assert_eq!(std::fs::read_to_string(&main)?,disk);
            results.push(serde_json::json!({"project":name,"scenario":scenario,"elapsed_ms":started.elapsed().as_secs_f64()*1000.,"answer":answer,"source_unchanged":true}));
        }
    }
    let report = serde_json::json!({"adapter":"actual application Rust semantic::query, native process-tree containment, real unmodified ESP-IDF CDB","results":results,"projects":2,"executed_queries":6,"exact_identifier_ranges_verified":true,"expected_definition_path_and_reference_counts_verified":true,"source_written":false,"external_context_exported":false,"persistent_background_index":false,"all_project_reference_completeness":false});
    std::fs::write(&args[4], serde_json::to_vec_pretty(&report)?)?;
    println!(
        "Passed 6 semantic queries across 2 generated projects; unsaved source unchanged on disk"
    );
    Ok(())
}
