pub mod installation;
mod esp_idf;
mod sdk_process;
mod archive;
mod ai_chat;
mod toolchain;
mod kb_store;   // SQLite vector store for RAG
pub mod intelligence; // Local, versioned ingestion and hybrid retrieval
mod kb_embed;   // Local ONNX embedding (fastembed, offline)
mod ai;         // Modular AI subsystem structure (refactoring roadmap)


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    run_context(tauri::generate_context!());
}

fn run_context(context: tauri::Context<tauri::Wry>) {
    use tauri::Manager;
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(ai_chat::AiAbortState(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))));
            app.manage(ai_chat::AiGenerationState::default());
            // Sanitize config.json on startup (moves legacy plaintext keys to OS Keychain)
            ai::config::initialize(app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            ai::change_journal::initialize(&app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            ai::project_review::initialize(&app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            ai::repair::initialize(&app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            if let Err(error) = ai::config::migrate_plaintext_keys_on_startup() {
                eprintln!("[Settings migration] {error}; legacy credentials were retained for recovery.");
            }
            // Seed bundled knowledge_base files into AppData on first launch
            ai_chat::seed_knowledge_base(app.handle()).map_err(std::io::Error::other)?;
            let model_directory = app.path().resource_dir()?.join("embedding");
            #[cfg(debug_assertions)]
            let model_directory = if model_directory.is_dir() { model_directory } else {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../resources/embedding")
            };
            kb_embed::configure_model_directory(model_directory).map_err(std::io::Error::other)?;
            std::thread::spawn(move || {
                if let Err(error) = kb_embed::init_local_embedder(None) {
                    eprintln!("[Startup] Bundled embedding: {error}");
                }
            });
            // Auto-patch pyvenv.cfg every startup so Python paths match this machine
            // Repairs stale absolute Python paths when the toolchain moves between machines.
            if let Ok(toolchain_dir) = toolchain::get_toolchain_dir(app.handle()) {
                toolchain::auto_repair_on_startup(&toolchain_dir);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            esp_idf::check_esp_idf,
            esp_idf::setup_esp_idf,
            esp_idf::run_idf_command,
            sdk_process::cancel_sdk_command,
            sdk_process::get_sdk_execution_config,
            sdk_process::set_sdk_execution_config,
            intelligence::semantic::query_c_intelligence,
            intelligence::semantic::cancel_c_intelligence,
            ai::repair::get_compiler_repair,
            ai::repair::set_compiler_repair_limit,
            ai::repair::begin_compiler_repair,
            ai::repair::apply_compiler_repair,
            ai::repair::rollback_compiler_repair,
            ai::repair::cancel_compiler_repair,
            esp_idf::run_shell_command,
            esp_idf::create_directory,
            esp_idf::delete_file,
            esp_idf::delete_directory,
            esp_idf::rename_item,
            esp_idf::create_idf_project,
            esp_idf::pick_directory,
            esp_idf::save_project_as,
            esp_idf::list_project_files,
            esp_idf::read_project_file,
            esp_idf::write_project_file,
            esp_idf::create_project_file,
            esp_idf::create_program_file,
            esp_idf::create_program_directory,
            esp_idf::rename_program_item,
            esp_idf::delete_program_item,
            esp_idf::safe_write_project_file,
            esp_idf::validate_idf_project,
            esp_idf::list_serial_ports,
            esp_idf::start_serial_monitor,
            esp_idf::send_serial_input,
            esp_idf::stop_serial_monitor,
            esp_idf::list_adb_devices,
            esp_idf::start_adb_monitor,
            esp_idf::send_adb_input,
            esp_idf::stop_adb_monitor,
            esp_idf::get_idf_custom_paths,
            esp_idf::set_idf_custom_paths,
            esp_idf::clear_idf_custom_paths,
            ai::config::get_api_key,
            ai::wiki::wiki_ai_request,
            ai::config::set_api_key,
            ai::config::get_model,
            ai::config::set_model,
            ai::config::get_base_url,
            ai::config::set_base_url,
            ai::config::get_provider,
            ai::config::set_provider,
            ai::config::get_search_api_key,
            ai::config::set_search_api_key,
            ai::config::get_openrouter_api_key,
            ai::config::set_openrouter_api_key,
            ai::config::get_openrouter_model,
            ai::config::set_openrouter_model,
            ai::config::get_google_api_key,
            ai::config::set_google_api_key,
            ai::config::get_google_model,
            ai::config::set_google_model,
            ai::config::clear_all_api_keys,
            ai_chat::get_knowledge_base_files,
            ai_chat::open_knowledge_base_folder,
            ai_chat::add_knowledge_base_files,
            ai_chat::refresh_knowledge_base,
            ai::kb::get_retrieval_config,
            ai::kb::set_retrieval_config,
            ai::privacy::get_private_context_consent,
            ai::privacy::set_private_context_consent,
            ai_chat::toggle_knowledge_base_file,
            ai_chat::send_ai_message,
            ai_chat::stop_ai_generation,
            ai_chat::undo_ai_changes,
            ai_chat::check_pending_diff,
            ai_chat::accept_diff,
            ai_chat::reject_diff,
            ai::project_review::get_pending_project_reviews,
            ai::project_review::keep_ai_project,
            ai::project_review::undo_ai_project,
            toolchain::check_toolchain,
            toolchain::download_toolchain,
            toolchain::cancel_toolchain_download,
            toolchain::remove_toolchain,
            toolchain::repair_toolchain_paths,
            toolchain::get_toolchain_paths,
            toolchain::build_firmware_with_toolchain,
        ])
        .run(context)
        .expect("error while running tauri application");
}

/// Development harness only. Same React app and command registry, fresh AppData
/// and WebView, no legacy settings/OS credentials or automatic toolchain repair.
/// No corresponding IPC command or release-build entry point exists.
#[cfg(debug_assertions)]
pub fn run_isolated_desktop_evaluation(root: &std::path::Path, idf: &std::path::Path, tools: &std::path::Path, port: u16) -> Result<(),String> {
    use std::path::Path;
    let permitted=Path::new(env!("CARGO_MANIFEST_DIR")).join("target/desktop-evaluation");
    std::fs::create_dir_all(&permitted).map_err(|e|e.to_string())?;
    let root=dunce::canonicalize(root).map_err(|e|e.to_string())?;
    let permitted=dunce::canonicalize(permitted).map_err(|e|e.to_string())?;
    if !root.starts_with(&permitted) || root==permitted || !(1024..=65534).contains(&port) {return Err("Evaluation root/port outside permitted scope".into());}
    if !idf.join("tools/idf.py").is_file() || !tools.is_dir() {return Err("Select an existing installed SDK".into());}
    let data=root.join("app-data");
    std::fs::create_dir(&data).map_err(|e|format!("Evaluation AppData must be new: {e}"))?;
    std::fs::write(data.join("config.json"),serde_json::to_vec(&serde_json::json!({"custom_idf_path":idf,"custom_tools_path":tools,"provider":"local","base_url":"http://localhost:1234/v1","private_context_consent":false})).unwrap()).map_err(|e|e.to_string())?;
    ai::config::isolate_evaluation_credentials();
    let mut context=tauri::generate_context!();
    context.config_mut().identifier="th.nectec.vibekidbright.desktop-evaluation".into();
    // Use the harness's immutable production frontend rather than the user's
    // Vite/HMR server, which can reload mid-cancellation when audits write public assets.
    context.config_mut().build.dev_url=Some(format!("http://127.0.0.1:{}",port+1).parse().map_err(|e|format!("Invalid evaluation frontend URL: {e}"))?);
    context.config_mut().app.app_directories_override=Some(tauri::utils::config::AppDirectoriesOverride::Root(data));
    for window in &mut context.config_mut().app.windows {
        window.title="vibeKidbright — isolated desktop evaluation".into();
        window.data_directory=Some(root.join("webview"));
        window.additional_browser_args=Some(format!("--remote-debugging-address=127.0.0.1 --remote-debugging-port={port} --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"));
    }
    run_context(context);
    Ok(())
}
