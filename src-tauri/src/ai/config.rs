//! Config + secure API key storage.
//!
//! Extracted from ai_chat.rs (refactoring roadmap step 1 — self-contained,
//! no cross-module deps beyond `crate::ai_chat::invalidate_idf_path_cache`).

use serde_json::{json, Value};
use std::path::PathBuf;

// ── Config helpers ────────────────────────────────────────────────────────────

pub fn config_path() -> PathBuf { config_dir().join("config.json") }

pub fn config_dir() -> PathBuf {
    // Windows: use APPDATA (e.g. C:\Users\<user>\AppData\Roaming)
    // Linux/macOS: use HOME
    // Fallback: use current directory (should never happen in practice)
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("USERPROFILE"))
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join(".vibekidbright")
}

pub fn read_config() -> Value {
    let path = config_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(json!({}))
    } else {
        json!({})
    }
}

pub fn write_config(config: &Value) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    // FIX M6: Atomic write — write to temp file then rename.
    // Prevents config corruption if the process is killed mid-write
    // or if two concurrent writes race each other.
    let path = config_path();
    let tmp_path = path.with_extension("json.tmp");
    let data = serde_json::to_string_pretty(config).unwrap_or_default();
    if std::fs::write(&tmp_path, &data).is_ok() {
        let _ = std::fs::rename(&tmp_path, &path);
    } else {
        // Fallback: direct write (e.g. cross-device rename not supported)
        let _ = std::fs::write(&path, &data);
    }
}

// ── Secure API key storage via OS keychain ───────────────────────────────────
// Uses the system credential manager as a *best-effort* secondary store.
// config.json is the PRIMARY reliable fallback — always written on set.
// Windows Credential Manager can silently fail on read even after a successful
// write, so we never rely on it exclusively.
//
// Storage strategy:
//   set_secure_key → write to keyring (best-effort) + always write to config.json
//   get_secure_key → try keyring first; if empty, fall back to config.json

pub fn get_secure_key(service: &str, config_field: &str) -> String {
    // 1. OS keychain (best-effort — may silently fail on Windows)
    if let Ok(entry) = keyring::Entry::new(service, "vibekidbright") {
        if let Ok(key) = entry.get_password() {
            if !key.is_empty() {
                return key;
            }
        }
    }
    // 2. Reliable fallback: config.json (always kept in sync by set_secure_key)
    let fallback = read_config()[config_field].as_str().unwrap_or("").to_string();
    fallback
}

pub fn set_secure_key(service: &str, config_field: &str, key: &str) {
    // Best-effort keyring store (ignore failures — config.json is the real backup)
    if key.is_empty() {
        if let Ok(entry) = keyring::Entry::new(service, "vibekidbright") {
            let _ = entry.delete_credential();
        }
    } else {
        if let Ok(entry) = keyring::Entry::new(service, "vibekidbright") {
            let _ = entry.set_password(key);
        }
    }
    // Always write to config.json so get_secure_key always has a reliable fallback.
    let mut c = read_config();
    c[config_field] = json!(key);
    write_config(&c);
}

/// No-op: previously migrated plaintext keys from config.json into the keyring
/// and wiped them. This is now disabled because config.json IS our reliable
/// fallback — wiping it caused "key not set" errors when keyring read failed.
pub fn migrate_plaintext_keys_on_startup() {
    // Intentionally empty — key retention in config.json is now by design.
}

#[tauri::command]
pub async fn clear_all_api_keys() -> Result<(), String> {
    set_secure_key("vibekidbright-openai", "api_key", "");
    set_secure_key("vibekidbright-openrouter", "openrouter_api_key", "");
    set_secure_key("vibekidbright-google", "google_api_key", "");
    set_secure_key("vibekidbright-search", "search_api_key", "");
    Ok(())
}

// ── Tauri commands ─────────────────────────────────────────────────────────────


#[tauri::command]
pub async fn get_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-openai", "api_key"))
}
#[tauri::command]
pub async fn set_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-openai", "api_key", &key);
    Ok(())
}
#[tauri::command]
pub async fn get_model() -> Result<String, String> {
    Ok(read_config()["model"].as_str().unwrap_or("gpt-4.1").to_string())
}
#[tauri::command]
pub async fn set_model(model: String) -> Result<(), String> {
    let mut c = read_config(); c["model"] = json!(model); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_base_url() -> Result<String, String> {
    Ok(read_config()["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string())
}
#[tauri::command]
pub async fn set_base_url(url: String) -> Result<(), String> {
    let mut c = read_config(); c["base_url"] = json!(url); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_provider() -> Result<String, String> {
    Ok(read_config()["provider"].as_str().unwrap_or("openai").to_string())
}
#[tauri::command]
pub async fn set_provider(provider: String) -> Result<(), String> {
    let mut c = read_config(); c["provider"] = json!(provider); write_config(&c);
    crate::ai_chat::invalidate_idf_path_cache();
    Ok(())
}
#[tauri::command]
pub async fn get_openrouter_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-openrouter", "openrouter_api_key"))
}
#[tauri::command]
pub async fn set_openrouter_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-openrouter", "openrouter_api_key", &key);
    Ok(())
}
#[tauri::command]
pub async fn get_openrouter_model() -> Result<String, String> {
    Ok(read_config()["openrouter_model"].as_str()
        .unwrap_or("google/gemini-2.5-flash:free").to_string())
}
#[tauri::command]
pub async fn set_openrouter_model(model: String) -> Result<(), String> {
    let mut c = read_config(); c["openrouter_model"] = json!(model); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_search_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-search", "search_api_key"))
}
#[tauri::command]
pub async fn set_search_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-search", "search_api_key", &key);
    Ok(())
}
#[tauri::command]
pub async fn get_google_api_key() -> Result<String, String> {
    Ok(get_secure_key("vibekidbright-google", "google_api_key"))
}
#[tauri::command]
pub async fn set_google_api_key(key: String) -> Result<(), String> {
    set_secure_key("vibekidbright-google", "google_api_key", &key);
    Ok(())
}
#[tauri::command]
pub async fn get_google_model() -> Result<String, String> {
    Ok(read_config()["google_model"].as_str().unwrap_or("gemini-2.5-flash").to_string())
}
#[tauri::command]
pub async fn set_google_model(model: String) -> Result<(), String> {
    let mut c = read_config(); c["google_model"] = json!(model); write_config(&c); Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
