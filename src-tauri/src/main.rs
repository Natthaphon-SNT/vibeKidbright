// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--self-check") {
        let result = (|| -> Result<serde_json::Value, String> {
            let output = args.get(2).ok_or("Usage: --self-check <output.json> <scratch-data-directory>")?;
            let data = args.get(3).ok_or("Missing scratch data directory.")?;
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let resources = executable.parent().ok_or("Cannot locate installation directory.")?;
            let result = tauri_app_lib::installation::self_check(resources, std::path::Path::new(data));
            let json = match &result { Ok(value) => value.clone(), Err(error) => serde_json::json!({ "error": error }) };
            std::fs::write(output, serde_json::to_vec_pretty(&json).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
            result
        })();
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    tauri_app_lib::run()
}
