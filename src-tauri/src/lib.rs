mod esp_idf;
mod ai_chat;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            esp_idf::check_esp_idf,
            esp_idf::setup_esp_idf,
            esp_idf::run_idf_command,
            esp_idf::run_shell_command,
            esp_idf::create_idf_project,
            esp_idf::pick_directory,
            esp_idf::list_project_files,
            esp_idf::read_project_file,
            esp_idf::write_project_file,
            esp_idf::validate_idf_project,
            esp_idf::list_serial_ports,
            esp_idf::start_serial_monitor,
            esp_idf::send_serial_input,
            esp_idf::stop_serial_monitor,
            ai_chat::get_api_key,
            ai_chat::set_api_key,
            ai_chat::get_model,
            ai_chat::set_model,
            ai_chat::get_base_url,
            ai_chat::set_base_url,
            ai_chat::get_provider,
            ai_chat::set_provider,
            ai_chat::get_search_api_key,
            ai_chat::set_search_api_key,
            ai_chat::get_knowledge_base_files,
            ai_chat::open_knowledge_base_folder,
            ai_chat::refresh_knowledge_base,
            ai_chat::send_ai_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
