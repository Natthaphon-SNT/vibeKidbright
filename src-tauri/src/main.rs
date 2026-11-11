// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--sdk-self-check") {
        let result = (|| -> Result<serde_json::Value, String> {
            let output = args.get(2).ok_or("Usage: --sdk-self-check <output.json> <new-scratch-directory> <idf-path> <tools-path>")?;
            let scratch = args.get(3).ok_or("Missing new scratch directory")?;
            let idf = args.get(4).ok_or("Missing SDK directory")?;
            let tools = args.get(5).ok_or("Missing SDK tools directory")?;
            let result = tauri_app_lib::installation::sdk_self_check(std::path::Path::new(idf), std::path::Path::new(tools), std::path::Path::new(scratch));
            let report = match &result { Ok(value) => value.clone(), Err(error) => serde_json::json!({"error":error}) };
            std::fs::write(output, serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
            result
        })();
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if args.get(1).is_some_and(|arg| arg == "--self-check") {
        let result = (|| -> Result<serde_json::Value, String> {
            let output = args.get(2).ok_or("Usage: --self-check <output.json> <scratch-data-directory>")?;
            let data = args.get(3).ok_or("Missing scratch data directory.")?;
            let executable = std::env::current_exe().map_err(|error| error.to_string())?;
            let resources = executable.parent().ok_or("Cannot locate installation directory.")?.to_path_buf();
            // macOS keeps the executable in Contents/MacOS and bundled assets
            // in Contents/Resources. This must also work after copying the app
            // from its DMG, without falling back to the developer's repository.
            #[cfg(target_os = "macos")]
            let resources = resources.parent().ok_or("Cannot locate app bundle Contents directory.")?.join("Resources");
            let result = tauri_app_lib::installation::self_check(&resources, std::path::Path::new(data));
            let json = match &result { Ok(value) => value.clone(), Err(error) => serde_json::json!({ "error": error }) };
            std::fs::write(output, serde_json::to_vec_pretty(&json).map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
            result
        })();
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    tauri_app_lib::run()
}
