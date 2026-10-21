//! User-owned settings and native OS credential storage. No development-machine paths.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::io::Write;

static CONFIG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static CONFIG_LOCK: Mutex<()> = Mutex::new(());
// Only the debug-only isolated desktop evaluation entry point can set this.
// Never access the user's OS credential namespace from an evaluation window.
#[cfg(debug_assertions)]
static EVALUATION_NO_CREDENTIALS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[cfg(debug_assertions)]
pub(crate) fn isolate_evaluation_credentials() {
    EVALUATION_NO_CREDENTIALS.store(true,std::sync::atomic::Ordering::SeqCst);
}
const CREDENTIALS: &[(&str, &str)] = &[
    ("vibekidbright-openai", "api_key"), ("vibekidbright-openrouter", "openrouter_api_key"),
    ("vibekidbright-google", "google_api_key"), ("vibekidbright-search", "search_api_key"),
];
pub fn config_dir() -> PathBuf {
    CONFIG_DIRECTORY.get().cloned().unwrap_or_else(|| directories::BaseDirs::new()
        .map(|base| base.config_dir().join("th.nectec.vibekidbright"))
        .unwrap_or_else(|| std::env::temp_dir().join("th.nectec.vibekidbright")))
}
pub fn config_path() -> PathBuf { config_dir().join("config.json") }

pub(crate) fn read_json(path: &Path) -> Result<Value, String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let value: Value = serde_json::from_slice(&bytes).map_err(|error| format!("Cannot read saved settings: {error}"))?;
            if !value.is_object() { return Err("Saved settings must contain a JSON object.".into()); }
            Ok(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(error) => Err(format!("Cannot read saved settings: {error}")),
    }
}
pub(crate) fn write_json_atomic(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    write_json_atomic_with(path, value, |_| Ok(()))
}
fn write_json_atomic_with(path: &Path, value: &impl serde::Serialize, mut checkpoint: impl FnMut(&str)->Result<(),String>) -> Result<(), String> {
    let parent = path.parent().ok_or("Missing settings directory.")?;
    std::fs::create_dir_all(parent).map_err(|error| format!("Cannot create settings directory: {error}"))?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    checkpoint("serialized")?;
    file.write_all(&bytes).map_err(|error| format!("Cannot save settings: {error}"))?;
    checkpoint("written")?;
    file.as_file().sync_all().map_err(|error| format!("Cannot sync settings: {error}"))?;
    checkpoint("synced")?;
    file.persist(path).map_err(|error| format!("Cannot replace saved settings: {}", error.error))?;
    Ok(())
}
fn update_at(path: &Path, edit: impl FnOnce(&mut Value)) -> Result<(), String> {
    let _lock = CONFIG_LOCK.lock().map_err(|_| "Settings lock is unavailable.")?;
    let mut config = read_json(path)?;
    edit(&mut config);
    write_json_atomic(path, &config)
}
pub(crate) fn update_config(edit: impl FnOnce(&mut Value)) -> Result<(), String> { update_at(&config_path(), edit) }
pub fn read_config() -> Value {
    match read_json(&config_path()) {
        Ok(config) => config,
        Err(error) => { eprintln!("[Settings] {error}"); json!({}) }
    }
}
fn legacy_config_path() -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    if EVALUATION_NO_CREDENTIALS.load(std::sync::atomic::Ordering::SeqCst) { return None; }
    std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))
        .map(|base| PathBuf::from(base).join(".vibekidbright/config.json"))
}
pub fn initialize(directory: PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let target = directory.join("config.json");
    if !target.exists() {
        if let Some(legacy) = legacy_config_path().filter(|path| path.is_file()) {
            write_json_atomic(&target, &read_json(&legacy)?)?;
        }
    }
    CONFIG_DIRECTORY.set(directory).map_err(|_| "Settings directory is already initialized.".to_string())?;
    Ok(())
}
pub fn get_secure_key(service: &str, field: &str) -> String {
    #[cfg(debug_assertions)]
    if EVALUATION_NO_CREDENTIALS.load(std::sync::atomic::Ordering::SeqCst) { return String::new(); }
    if let Ok(entry) = keyring::Entry::new(service, "vibekidbright") {
        if let Ok(key) = entry.get_password() { if !key.is_empty() { return key; } }
    }
    // Read-only compatibility until a successful migration; never create a new plaintext fallback.
    read_config()[field].as_str().unwrap_or("").to_string()
}
pub fn set_secure_key(service: &str, field: &str, key: &str) -> Result<(), String> {
    #[cfg(debug_assertions)]
    if EVALUATION_NO_CREDENTIALS.load(std::sync::atomic::Ordering::SeqCst) { return Err("Credential writes are disabled in isolated desktop evaluation".into()); }
    let entry = keyring::Entry::new(service, "vibekidbright").map_err(|error| format!("Credential store unavailable: {error}"))?;
    if key.is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => (),
            Err(error) => return Err(format!("Cannot remove saved credential: {error}")),
        }
    } else {
        entry.set_password(key).map_err(|error| format!("Cannot save credential: {error}"))?;
        if entry.get_password().map_err(|error| format!("Cannot verify saved credential: {error}"))? != key {
            return Err("Credential store verification failed. The key was not saved to a plaintext file.".into());
        }
    }
    update_config(|config| { config.as_object_mut().unwrap().remove(field); })?;
    if let Some(legacy) = legacy_config_path().filter(|path| path.is_file() && path != &config_path()) {
        update_at(&legacy, |config| { config.as_object_mut().unwrap().remove(field); })?;
    }
    Ok(())
}
pub fn migrate_plaintext_keys_on_startup() -> Result<(), String> {
    for &(service, field) in CREDENTIALS {
        let key = read_config()[field].as_str().unwrap_or("").to_string();
        if !key.is_empty() { set_secure_key(service, field, &key)?; }
    }
    Ok(())
}
#[tauri::command]
pub async fn clear_all_api_keys() -> Result<(), String> {
    for &(service, field) in CREDENTIALS { set_secure_key(service, field, "")?; }
    Ok(())
}

#[tauri::command]
pub async fn get_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-openai", "api_key"))
}
#[tauri::command]
pub async fn set_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-openai", "api_key", &key)
}
#[tauri::command]
pub async fn get_model() -> Result<String, String> {
    Ok(read_config()["model"].as_str().unwrap_or("gpt-4.1").to_string())
}
#[tauri::command]
pub async fn set_model(model: String) -> Result<(), String> {
    update_config(|c| c["model"] = json!(model))
}
#[tauri::command]
pub async fn get_base_url() -> Result<String, String> {
    Ok(read_config()["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string())
}
#[tauri::command]
pub async fn set_base_url(url: String) -> Result<(), String> {
    update_config(|c| c["base_url"] = json!(url))
}
#[tauri::command]
pub async fn get_provider() -> Result<String, String> {
    Ok(read_config()["provider"].as_str().unwrap_or("openai").to_string())
}
#[tauri::command]
pub async fn set_provider(provider: String) -> Result<(), String> {
    update_config(|c| c["provider"] = json!(provider))?;
    crate::ai_chat::invalidate_idf_path_cache();
    Ok(())
}
#[tauri::command]
pub async fn get_openrouter_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-openrouter", "openrouter_api_key"))
}
#[tauri::command]
pub async fn set_openrouter_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-openrouter", "openrouter_api_key", &key)
}
#[tauri::command]
pub async fn get_openrouter_model() -> Result<String, String> {
    Ok(read_config()["openrouter_model"].as_str()
        .unwrap_or("google/gemini-2.5-flash:free").to_string())
}
#[tauri::command]
pub async fn set_openrouter_model(model: String) -> Result<(), String> {
    update_config(|c| c["openrouter_model"] = json!(model))
}
#[tauri::command]
pub async fn get_search_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-search", "search_api_key"))
}
#[tauri::command]
pub async fn set_search_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-search", "search_api_key", &key)
}
#[tauri::command]
pub async fn get_google_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-google", "google_api_key"))
}
#[tauri::command]
pub async fn set_google_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-google", "google_api_key", &key)
}
#[tauri::command]
pub async fn get_google_model() -> Result<String, String> {
    Ok(read_config()["google_model"].as_str().unwrap_or("gemini-2.5-flash").to_string())
}
#[tauri::command]
pub async fn set_google_model(model: String) -> Result<(), String> {
    update_config(|c| c["google_model"] = json!(model))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_atomic_write_stages_preserve_original_and_remove_temporary_file() {
        for failing_stage in ["serialized","written","synced"] {
            let directory=tempfile::tempdir().unwrap();
            let path=directory.path().join("review.json");
            let original=b"{\"schema\":1,\"recovery\":\"retain exact original\"}\r\n";
            std::fs::write(&path,original).unwrap();
            let result=write_json_atomic_with(&path,&json!({"schema":1,"recovery":"new intent"}),|stage|{
                if stage==failing_stage {Err("injected interruption before atomic replacement".into())}else{Ok(())}
            });
            assert!(result.is_err());
            assert_eq!(std::fs::read(&path).unwrap(),original);
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(),1);
            assert_eq!(read_json(&path).unwrap()["recovery"],"retain exact original");
        }
    }

    #[test]
    fn failed_settings_updates_preserve_corrupt_files_and_report_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, b"{broken settings").unwrap();
        assert!(update_at(&path, |config| config["model"] = json!("new")).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{broken settings");
        let folder = dir.path().join("not-a-file");
        std::fs::create_dir(&folder).unwrap();
        assert!(write_json_atomic(&folder, &json!({"saved": true})).is_err());
        assert!(folder.is_dir());
    }

    #[test]
    fn concurrent_settings_updates_preserve_each_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let threads: Vec<_> = (0..12).map(|number| {
            let path = path.clone();
            std::thread::spawn(move || update_at(&path, |config| config[format!("field-{number}")] = json!(number)).unwrap())
        }).collect();
        for thread in threads { thread.join().unwrap(); }
        assert_eq!(read_json(&path).unwrap().as_object().unwrap().len(), 12);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn test_config_dir_not_empty() {
        let dir = config_dir();
        assert!(!dir.to_string_lossy().is_empty(), "config_dir must not be empty");
    }

    #[test]
    fn test_config_path_extension_is_json() {
        let path = config_path();
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("json"),
            "config file must end in .json"
        );
    }

    #[test]
    fn test_read_config_returns_object_when_missing() {
        // read_config should never panic, even if file doesn't exist
        let config = read_config();
        assert!(config.is_object(), "read_config() must return a JSON object");
    }
}
