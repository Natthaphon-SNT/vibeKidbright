use futures::StreamExt;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeChunk {
    file_name: String,
    content: String,
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct VectorIndex {
    chunks: Vec<KnowledgeChunk>,
    last_indexed: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
    /// Preserved from Gemini thinking-mode responses — must be echoed back verbatim.
    thought_signature: Option<String>,
}

use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};
use std::collections::HashMap;
use std::time::Instant;

// ── Global caches ─────────────────────────────────────────────────────────────

static RATE_LIMITED_MODELS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
fn get_rate_limited_models() -> &'static Mutex<HashMap<String, Instant>> {
    RATE_LIMITED_MODELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Cached IDF PATH string — computed once per session, not on every command.
static CACHED_IDF_PATH: OnceLock<Mutex<Option<OsString>>> = OnceLock::new();
fn get_cached_idf_path() -> &'static Mutex<Option<OsString>> {
    CACHED_IDF_PATH.get_or_init(|| Mutex::new(None))
}

/// Simple in-memory knowledge-search cache: query -> results JSON.
/// Cleared automatically when KB is re-indexed.
static KB_QUERY_CACHE: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
fn get_kb_query_cache() -> &'static Mutex<HashMap<String, Value>> {
    KB_QUERY_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Max tool-call turns per conversation to prevent infinite loops.
const MAX_TOOL_TURNS: u32 = 20;

// ── State types ───────────────────────────────────────────────────────────────

pub struct AiAbortState(pub Arc<AtomicBool>);

#[derive(Default)]
pub struct AiBackupState {
    pub backups: Mutex<HashMap<String, HashMap<PathBuf, Option<String>>>>,
}

static PENDING_DIFFS: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();
pub fn get_pending_diffs() -> &'static Mutex<HashMap<PathBuf, String>> {
    PENDING_DIFFS.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── Config helpers ────────────────────────────────────────────────────────────

fn config_path() -> PathBuf { config_dir().join("config.json") }

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibekidbright")
}

fn read_config() -> Value {
    let path = config_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(json!({}))
    } else {
        json!({})
    }
}

fn write_config(config: &Value) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(
        config_path(),
        serde_json::to_string_pretty(config).unwrap_or_default(),
    );
}

fn normalize_project_dir(project_dir: &str) -> String {
    project_dir.trim_start_matches("file://").trim().to_string()
}

fn resolve_project_root(project_dir: &str) -> PathBuf {
    let normalized = normalize_project_dir(project_dir);
    if !normalized.is_empty() && normalized != "." {
        return PathBuf::from(normalized);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("knowledge_base").exists() { return cwd; }
    if let Some(parent) = cwd.parent() {
        let p = parent.to_path_buf();
        if p.join("knowledge_base").exists() { return p; }
    }
    if cwd.file_name().is_some_and(|n| n == "src-tauri") {
        if let Some(parent) = cwd.parent() { return parent.to_path_buf(); }
    }
    cwd
}

fn resolve_kb_path(project_dir: &str) -> PathBuf {
    resolve_project_root(project_dir).join("knowledge_base")
}

fn resolve_idf_paths_for_ai(app_handle: &AppHandle) -> Option<(PathBuf, PathBuf)> {
    if let (Some(idf), Some(tools)) = (
        std::env::var_os("VIBEKIDBRIGHT_IDF_PATH"),
        std::env::var_os("VIBEKIDBRIGHT_TOOLS_PATH"),
    ) {
        let idf_path = PathBuf::from(idf);
        let tools_path = PathBuf::from(tools);
        if idf_path.join("tools/idf.py").exists() && tools_path.exists() {
            return Some((idf_path, tools_path));
        }
    }
    {
        let config = read_config();
        if let (Some(idf), Some(tools)) = (
            config["custom_idf_path"].as_str(),
            config["custom_tools_path"].as_str(),
        ) {
            if !idf.is_empty() && !tools.is_empty() {
                let idf_path = PathBuf::from(idf);
                let tools_path = PathBuf::from(tools);
                if idf_path.join("tools/idf.py").exists() && tools_path.exists() {
                    return Some((idf_path, tools_path));
                }
            }
        }
    }
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let runtime_root = app_data_dir.join("esp-idf-runtime");
        let tools_path = runtime_root.join(".espressif");
        if tools_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&runtime_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.file_name().is_some_and(|n| n.to_string_lossy().starts_with("esp-idf-"))
                        && path.join("tools/idf.py").exists()
                    {
                        return Some((path, tools_path.clone()));
                    }
                }
            }
        }
    }
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let idf_path = resource_dir.join("esp-idf");
        let tools_path = resource_dir.join(".espressif");
        if idf_path.join("tools/idf.py").exists() && tools_path.exists() {
            return Some((idf_path, tools_path));
        }
    }
    let dev_idf = PathBuf::from("../resources/esp-idf");
    let dev_tools = PathBuf::from("../resources/.espressif");
    if dev_idf.join("tools/idf.py").exists() && dev_tools.exists() {
        return Some((dev_idf, dev_tools));
    }
    None
}

fn find_idf_python_bin(tools_path: &Path) -> Option<PathBuf> {
    let python_env_dir = tools_path.join("python_env");
    let entries = std::fs::read_dir(&python_env_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with("idf") && name.contains("_py") && name.ends_with("_env")) { continue; }
        let venv = entry.path();
        let candidates = if cfg!(target_os = "windows") {
            vec![venv.join("Scripts/python.exe")]
        } else {
            vec![venv.join("bin/python"), venv.join("bin/python3")]
        };
        for candidate in candidates {
            if candidate.exists() { return Some(candidate); }
        }
    }
    None
}

/// Build the IDF PATH string and cache it — called once, reused on every command.
fn build_ai_idf_path_cached(tools_path: &Path) -> OsString {
    {
        let lock = get_cached_idf_path().lock().unwrap();
        if let Some(cached) = lock.as_ref() {
            return cached.clone();
        }
    }
    let result = build_ai_idf_path_inner(tools_path);
    {
        let mut lock = get_cached_idf_path().lock().unwrap();
        *lock = Some(result.clone());
    }
    result
}

fn build_ai_idf_path_inner(tools_path: &Path) -> OsString {
    let mut paths: Vec<PathBuf> = Vec::new();
    let scan = |tools_dir: &Path, paths: &mut Vec<PathBuf>| {
        if let Ok(entries) = std::fs::read_dir(tools_dir) {
            for entry in entries.flatten() {
                if !entry.path().is_dir() { continue; }
                if let Ok(versions) = std::fs::read_dir(entry.path()) {
                    for ver in versions.flatten() {
                        let bin = ver.path().join("bin");
                        if bin.exists() { paths.push(bin); }
                        if ver.path().is_dir() { paths.push(ver.path()); }
                        let tool_bin = ver.path()
                            .join(entry.file_name().to_string_lossy().to_string())
                            .join("bin");
                        if tool_bin.exists() { paths.push(tool_bin); }
                    }
                }
            }
        }
    };
    scan(&tools_path.join("tools"), &mut paths);
    scan(tools_path, &mut paths);
    let config = read_config();
    if let Some(custom_tools) = config["custom_tools_path"].as_str() {
        if !custom_tools.is_empty() {
            let custom_tools_dir = PathBuf::from(custom_tools);
            if custom_tools_dir.join("tools") != tools_path.join("tools") {
                scan(&custom_tools_dir.join("tools"), &mut paths);
            }
            if custom_tools_dir != tools_path {
                scan(&custom_tools_dir, &mut paths);
            }
        }
    }
    let add_pyenv = |pyenv_dir: &Path, paths: &mut Vec<PathBuf>| {
        if let Ok(entries) = std::fs::read_dir(pyenv_dir) {
            for entry in entries.flatten() {
                let bin = if cfg!(target_os = "windows") {
                    entry.path().join("Scripts")
                } else {
                    entry.path().join("bin")
                };
                if bin.exists() { paths.push(bin); }
            }
        }
    };
    let python_env_dir = tools_path.join("python_env");
    add_pyenv(&python_env_dir, &mut paths);
    if let Some(custom_tools) = config["custom_tools_path"].as_str() {
        if !custom_tools.is_empty() {
            let custom_pyenv = PathBuf::from(custom_tools).join("python_env");
            if custom_pyenv != python_env_dir {
                add_pyenv(&custom_pyenv, &mut paths);
            }
        }
    }
    if let Some(system_path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system_path));
    }
    std::env::join_paths(paths).unwrap_or_else(|_| OsString::from(""))
}

// Invalidate the IDF path cache (call when user changes custom_tools_path in settings).
pub fn invalidate_idf_path_cache() {
    let mut lock = get_cached_idf_path().lock().unwrap();
    *lock = None;
}

// ── Tauri commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_api_key() -> Result<String, String> {
    Ok(read_config()["api_key"].as_str().unwrap_or("").to_string())
}
#[tauri::command]
pub async fn set_api_key(key: String) -> Result<(), String> {
    let mut c = read_config(); c["api_key"] = json!(key); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_model() -> Result<String, String> {
    Ok(read_config()["model"].as_str().unwrap_or("gpt-4o").to_string())
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
    invalidate_idf_path_cache();
    Ok(())
}
#[tauri::command]
pub async fn get_openrouter_api_key() -> Result<String, String> {
    Ok(read_config()["openrouter_api_key"].as_str().unwrap_or("").to_string())
}
#[tauri::command]
pub async fn set_openrouter_api_key(key: String) -> Result<(), String> {
    let mut c = read_config(); c["openrouter_api_key"] = json!(key); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_openrouter_model() -> Result<String, String> {
    Ok(read_config()["openrouter_model"].as_str()
        .unwrap_or("meta-llama/llama-3.3-70b-instruct:free").to_string())
}
#[tauri::command]
pub async fn set_openrouter_model(model: String) -> Result<(), String> {
    let mut c = read_config(); c["openrouter_model"] = json!(model); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_search_api_key() -> Result<String, String> {
    Ok(read_config()["search_api_key"].as_str().unwrap_or("").to_string())
}
#[tauri::command]
pub async fn set_search_api_key(key: String) -> Result<(), String> {
    let mut c = read_config(); c["search_api_key"] = json!(key); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_google_api_key() -> Result<String, String> {
    Ok(read_config()["google_api_key"].as_str().unwrap_or("").to_string())
}
#[tauri::command]
pub async fn set_google_api_key(key: String) -> Result<(), String> {
    let mut c = read_config(); c["google_api_key"] = json!(key); write_config(&c); Ok(())
}
#[tauri::command]
pub async fn get_google_model() -> Result<String, String> {
    Ok(read_config()["google_model"].as_str().unwrap_or("gemini-2.5-flash").to_string())
}
#[tauri::command]
pub async fn set_google_model(model: String) -> Result<(), String> {
    let mut c = read_config(); c["google_model"] = json!(model); write_config(&c); Ok(())
}

#[tauri::command]
pub async fn refresh_knowledge_base(project_dir: String) -> Result<usize, String> {
    let project_path = resolve_project_root(&project_dir);
    // Invalidate query cache so stale results aren't served after re-index.
    get_kb_query_cache().lock().unwrap().clear();
    reindex_knowledge_base(&project_path).await
}

#[tauri::command]
pub fn get_knowledge_base_files(project_dir: String) -> Vec<String> {
    let project_path = resolve_project_root(&project_dir);
    let kb_path = project_path.join("knowledge_base");
    if !kb_path.exists() { return Vec::new(); }
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&kb_path) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if name.ends_with(".txt") || name.ends_with(".md") {
                    files.push(name);
                }
            }
        }
    }
    files
}

#[tauri::command]
pub fn open_knowledge_base_folder(project_dir: String) {
    let kb_path = resolve_kb_path(&project_dir);
    if !kb_path.exists() { let _ = std::fs::create_dir_all(&kb_path); }
    let _ = tauri_plugin_opener::open_path(kb_path.to_string_lossy().to_string(), None::<String>);
}

#[tauri::command]
pub async fn add_knowledge_base_files(project_dir: String) -> Result<usize, String> {
    use rfd::FileDialog;
    let paths = FileDialog::new()
        .set_title("Add Document to Knowledge Base")
        .add_filter("Documentation", &["txt", "md"])
        .pick_files();
    if let Some(files) = paths {
        let kb_path = resolve_kb_path(&project_dir);
        if !kb_path.exists() {
            std::fs::create_dir_all(&kb_path).map_err(|e| e.to_string())?;
        }
        let mut copied = 0;
        for file in files {
            if let Some(name) = file.file_name() {
                let dest = kb_path.join(name);
                if std::fs::copy(&file, &dest).is_ok() { copied += 1; }
            }
        }
        // Invalidate KB cache after adding files.
        get_kb_query_cache().lock().unwrap().clear();
        Ok(copied)
    } else {
        Ok(0)
    }
}

#[tauri::command]
pub async fn stop_ai_generation(app_handle: AppHandle) -> Result<(), String> {
    if let Some(state) = app_handle.try_state::<AiAbortState>() {
        state.0.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
pub async fn undo_ai_changes(app_handle: AppHandle, message_id: String) -> Result<(), String> {
    if let Some(state) = app_handle.try_state::<AiBackupState>() {
        let mut backups = state.backups.lock().unwrap();
        if let Some(files) = backups.remove(&message_id) {
            for (path, content) in files {
                if let Some(original) = content {
                    let _ = std::fs::write(&path, original);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
            return Ok(());
        }
    }
    Err("No backups found for this message.".to_string())
}

#[tauri::command]
pub async fn check_pending_diff(path: String) -> Result<Option<String>, String> {
    let target_path = PathBuf::from(path);
    let diffs = get_pending_diffs().lock().unwrap();
    Ok(diffs.get(&target_path).cloned())
}

#[tauri::command]
pub async fn accept_diff(app_handle: AppHandle, path: String) -> Result<String, String> {
    let target_path = PathBuf::from(path);
    let mut diffs = get_pending_diffs().lock().unwrap();
    if let Some(content) = diffs.remove(&target_path) {
        match std::fs::write(&target_path, &content) {
            Ok(_) => {
                let _ = app_handle.emit(
                    "file-modified",
                    json!({ "path": target_path.to_string_lossy() }).to_string(),
                );
                Ok(format!("Changes applied to {}", target_path.display()))
            }
            Err(e) => Err(format!("Failed to write file: {}", e)),
        }
    } else {
        Err("No pending diff found for this file.".to_string())
    }
}

#[tauri::command]
pub async fn reject_diff(path: String) -> Result<String, String> {
    let target_path = PathBuf::from(path);
    let mut diffs = get_pending_diffs().lock().unwrap();
    if diffs.remove(&target_path).is_some() {
        Ok("Changes rejected. File was not modified.".to_string())
    } else {
        Err("No pending diff found for this file.".to_string())
    }
}

#[tauri::command]
pub async fn send_ai_message(
    app_handle: AppHandle,
    messages: Vec<ChatMessage>,
    project_dir: String,
    message_id: String,
) -> Result<(), String> {
    if let Some(state) = app_handle.try_state::<AiAbortState>() {
        state.0.store(false, Ordering::SeqCst);
    }

    let (api_key, raw_model, mut base_url, provider) = {
        let config = read_config();
        let prov = config["provider"].as_str().unwrap_or("openai").to_string();
        let (key, model, url) = if prov == "openrouter" {
            (
                config["openrouter_api_key"].as_str().unwrap_or("").to_string(),
                config["openrouter_model"].as_str().unwrap_or("anthropic/claude-3.5-sonnet").to_string(),
                "https://openrouter.ai/api/v1".to_string(),
            )
        } else if prov == "google" {
            (
                config["google_api_key"].as_str().unwrap_or("").to_string(),
                config["google_model"].as_str().unwrap_or("gemini-2.5-flash").to_string(),
                "https://generativelanguage.googleapis.com/v1beta".to_string(),
            )
        } else {
            (
                config["api_key"].as_str().unwrap_or("").to_string(),
                config["model"].as_str().unwrap_or("gpt-4o").to_string(),
                config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string(),
            )
        };
        (key, model, url, prov)
    };

    if !base_url.starts_with("http") && !base_url.is_empty() {
        base_url = format!("http://{}", base_url);
    }
    if (base_url.contains("localhost") || base_url.contains("127.0.0.1"))
        && !base_url.contains("/v1")
        && !base_url.ends_with("/v1")
    {
        base_url = format!("{}/v1", base_url.trim_end_matches('/'));
    }

    let model = if provider == "openai" {
        raw_model.replace("openai/", "")
    } else {
        raw_model
    };

    if api_key.is_empty() && provider == "openrouter" {
        return Err("OpenRouter API key not set. Please configure it in AI Provider Settings.".to_string());
    }
    if api_key.is_empty() && provider == "google" {
        return Err("Google AI API key not set. Please configure it in AI Provider Settings.".to_string());
    }
    if api_key.is_empty() && provider == "openai"
        && !base_url.contains("localhost")
        && !base_url.contains("127.0.0.1")
    {
        return Err("API key not set. Please configure your OpenAI API key.".to_string());
    }

    // FIX: Convert message_id to Arc<str> before moving into spawn.
    // This is cheap to clone (atomic refcount) and works safely across await points.
    let message_id: Arc<str> = Arc::from(message_id.as_str());

    let mut project_path = resolve_project_root(&project_dir);
    let mut no_workspace = project_dir == "." || project_dir.is_empty();
    let _is_openrouter = provider == "openrouter";

    tokio::spawn(async move {
        let mut try_queue: Vec<(String, String, String, String, String)> = vec![];
        let (config_google_key, config_google_model, config_or_key, config_or_model) = {
            let config = read_config();
            (
                config["google_api_key"].as_str().unwrap_or("").to_string(),
                config["google_model"].as_str().unwrap_or("gemini-1.5-flash").to_string(),
                config["openrouter_api_key"].as_str().unwrap_or("").to_string(),
                config["openrouter_model"]
                    .as_str()
                    .unwrap_or("meta-llama/llama-3.3-70b-instruct:free")
                    .to_string(),
            )
        };

        let is_free_tier = [
            "gemini-1.5", "gemini-2.0", "gemini-2.5",
            "meta-llama", "qwen", "deepseek",
            "nvidia/nemotron", "arcee-ai", "minimax",
            "z-ai/glm", "openai/gpt-oss", "google/gemma",
            ":free",
        ]
        .iter()
        .any(|&m| model.to_lowercase().contains(m));

        if model == "free" || model == "openrouter/free" || model == "auto-free" {
            let best_free_models = vec![
                // ── Top tier (coding + tool support + large context) ──────────
                "qwen/qwen3-coder:free",                    // 480B, best free coder
                "openai/gpt-oss-120b:free",                 // GPT-class 120B
                "stepfun/step-3.5-flash:free",              // fast & free
                "nvidia/nemotron-3-super-120b-a12b:free",   // top weekly
                "deepseek/deepseek-r1:free",                // strong reasoning
                "deepseek/deepseek-chat:free",
                // ── Mid tier ─────────────────────────────────────────────────
                "qwen/qwen3.6-plus-04-02:free",
                "arcee-ai/trinity-large-preview:free",
                "minimax/minimax-m2.5:free",
                "google/gemma-4-31b-it:free",
                "google/gemma-3-27b-it:free",
                "meta-llama/llama-3.3-70b-instruct:free",
                // ── Smaller / lighter fallbacks ───────────────────────────────
                "nvidia/nemotron-3-nano-30b-a3b:free",
                "openai/gpt-oss-20b:free",
                "arcee-ai/trinity-mini:free",
                "z-ai/glm-4.5-air:free",
                "qwen/qwen-max:free",
            ];
            
            let or_url = "https://openrouter.ai/api/v1".to_string();
            let actual_or_key = if provider == "openrouter" && !api_key.is_empty() { api_key.clone() } else { config_or_key.clone() };
            
            for m in best_free_models {
                try_queue.push((
                    "openrouter".to_string(),
                    m.to_string(),
                    or_url.clone(),
                    actual_or_key.clone(),
                    format!("{} [AUTO-FREE]", m),
                ));
            }
            
            if !config_google_key.is_empty() {
                try_queue.push((
                    "google".to_string(), config_google_model.clone(),
                    "https://generativelanguage.googleapis.com/v1beta".to_string(),
                    config_google_key.clone(),
                    format!("{} [AUTO-FREE Google Fallback]", config_google_model),
                ));
            }
        } else if is_free_tier {
            try_queue.push((
                provider.clone(), model.clone(), base_url.clone(), api_key.clone(),
                format!("{} [FREE]", model),
            ));
            if !config_google_key.is_empty()
                && !(provider == "google" && model == config_google_model)
            {
                try_queue.push((
                    "google".to_string(), config_google_model.clone(),
                    "https://generativelanguage.googleapis.com/v1beta".to_string(),
                    config_google_key.clone(),
                    format!("{} [FREE Fallback]", config_google_model),
                ));
            }
            if !config_or_key.is_empty() {
                if !(provider == "openrouter" && model == config_or_model) {
                    try_queue.push((
                        "openrouter".to_string(), config_or_model.clone(),
                        "https://openrouter.ai/api/v1".to_string(),
                        config_or_key.clone(),
                        format!("{} [FREE Fallback]", config_or_model),
                    ));
                }
                
                // Add guaranteed working fallbacks just in case the user's config_or_model is deprecated/removed.
                let guaranteed_fallbacks = vec![
                    "nvidia/nemotron-3-super-120b-a12b:free",
                    "meta-llama/llama-3.3-70b-instruct:free",
                    "google/gemma-4-31b-it:free",
                    "qwen/qwen3-coder:free",
                ];
                for gf in guaranteed_fallbacks {
                    if gf != config_or_model && gf != model {
                        try_queue.push((
                            "openrouter".to_string(), gf.to_string(),
                            "https://openrouter.ai/api/v1".to_string(),
                            config_or_key.clone(),
                            format!("{} [EMERGENCY FALLBACK]", gf),
                        ));
                    }
                }
            }
        } else {
            try_queue.push((
                provider.clone(), model.clone(), base_url.clone(), api_key.clone(),
                format!("{} [PAID]", model),
            ));
        }

        let mut final_error = String::new();

        for (prov, mod_name, url, key, badge) in try_queue {
            {
                let mut lock = get_rate_limited_models().lock().unwrap();
                if let Some(time) = lock.get(&mod_name) {
                    if time.elapsed().as_secs() < 60 {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] Skipping {} (On 60s Hold due to Rate Limit)", mod_name),
                        );
                        continue;
                    } else {
                        lock.remove(&mod_name);
                    }
                }
            }

            let _ = app_handle.emit("ai-active-model", badge.clone());
            let _ = app_handle.emit(
                "terminal-output",
                format!("[AI] Calling {} (Model: {})", url, mod_name),
            );

            let is_g = prov == "google";
            let current_is_openrouter = prov == "openrouter";

            // FIX: Arc::clone is a refcount bump — zero allocation.
            let result = if is_g {
                run_google_conversation_loop(
                    &app_handle, &key, &mod_name, messages.clone(),
                    &mut project_path, Arc::clone(&message_id), &mut no_workspace,
                ).await
            } else {
                run_conversation_loop(
                    &app_handle, &key, &mod_name, &url, messages.clone(),
                    &mut project_path, current_is_openrouter, Arc::clone(&message_id), &mut no_workspace,
                ).await
            };

            match result {
                Ok(_) => return,
                Err(e) => {
                    let err_msg_str = e.to_string();
                    let err_lower = err_msg_str.to_lowercase();
                    if err_lower.contains("429")
                        || err_lower.contains("quota")
                        || err_lower.contains("rate limit")
                        || err_lower.contains("too many requests")
                        || err_lower.contains("overloaded")
                        || err_lower.contains("502")
                        || err_lower.contains("503")
                        || err_lower.contains("unavailable")
                        || err_lower.contains("provider error")
                        || err_lower.contains("404")
                        || err_lower.contains("not found")
                    {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI LIMIT] {} failed ({}), downgrading...", mod_name, err_msg_str),
                        );
                        get_rate_limited_models()
                            .lock()
                            .unwrap()
                            .insert(mod_name.clone(), Instant::now());
                        final_error = err_msg_str;
                        continue;
                    } else {
                        let _ = app_handle.emit("ai-chat-error", err_msg_str.clone());
                        let _ = app_handle.emit("terminal-output", format!("[AI ERROR] {}", err_msg_str));
                        return;
                    }
                }
            }
        }

        let _ = app_handle.emit(
            "ai-chat-error",
            format!(
                "All free models are currently exhausted or on hold due to Rate Limits (429). \
                 Please wait 1 minute, or switch to a Paid provider.\n\n(Last error: {})",
                final_error
            ),
        );
        let _ = app_handle.emit("terminal-output", "[AI FATAL] Exhausted all fallback models.");
    });

    Ok(())
}

// ── Conversation loop ─────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are an expert ESP-IDF coding assistant. You help users build firmware for ESP32 and KidBright boards.

YOU HAVE TWO WAYS TO HELP:
1. AUTONOMOUS CREATION: When asked to create a project, write files, or fix code, you MUST use the `write_file` tool. Do NOT just print the code in the chat.
2. EXPLAIN & GENERATE: When explaining code or offering snippets in chat:
- You MUST format all code using markdown code fences (```c ... ```). NEVER print raw code without fences!
- ALWAYS put a file header like `[FILE: main/main.c]` on the line IMMEDIATELY BEFORE the opening ``` code fence so the IDE can parse it.

### CRITICAL RULE: NO ARDUINO CODE
- You MUST write raw ESP-IDF C code (using FreeRTOS, `driver/gpio.h`, `driver/i2c.h`, `driver/ledc.h` etc.).
- ALWAYS include `<stdio.h>`, `"freertos/FreeRTOS.h"`, and `"freertos/task.h"` if you use `vTaskDelay` or other FreeRTOS functions.
- NEVER generate Arduino code (`#include <Wire.h>`, `setup()`, `loop()`, `tone()`, etc.).
- Even if the Knowledge Base shows Arduino examples, you MUST translate them to pure ESP-IDF API before showing the user.

### SENSOR RULES (MANDATORY):
- **Temperature Sensor**: The ESP32 chip on the KidBright32 iA board does NOT have an internal temperature sensor. You MUST NEVER use `esp_driver_tsens` or `temperature_sensor_install()`. Instead, you MUST use the on-board **LM73** I2C sensor via `I2C_NUM_1` (SDA=GPIO4, SCL=GPIO5, Address=0x4D) to measure temperature.

### ESP-IDF PROJECT STRUCTURE RULES (MANDATORY):
1. **Root Directory Awareness:** The current working directory is ALWAYS the Project Root.
   - **PROHIBITION:** NEVER create a nested project folder inside the Root (e.g., NO `./my_project/main/`). All core files MUST reside at the top level of the workspace.
   - **PROHIBITION:** DO NOT run `idf.py create-project <name>`. It generates a nested folder that breaks our structure. Instead, if asked to create a project, use the `write_file` tool to manually create `CMakeLists.txt` and `main/main.c` DIRECTLY in the current directory.
   - **NEW PROJECT INITIALIZATION:** If the user wants to start a NEW project, you MUST use the `create_project_workspace` tool First.
     This tool prompts the user to select a folder and creates a folder named `project_name` inside it.
   - **IMPORTANT**: The `create_project_workspace` tool will AUTOMATICALLY generate the standard ESP-IDF boilerplate for you (`CMakeLists.txt`, `main/CMakeLists.txt`, and a basic `main/main.c`).
   - AFTER it succeeds, you ONLY need to use `write_file` to overwrite `main/main.c` with the actual logic. Do NOT try to write `CMakeLists.txt` or `sdkconfig` manually unless the user strictly requires custom configurations.
2. **Standard Layout:**
   When asked to create or initialize a new project, you MUST autonomously use the `write_file` tool to create EXACTLY these 4 files with the specified content:
   - `CMakeLists.txt` (Project-level) — Must contain:
     ```
     cmake_minimum_required(VERSION 3.16)
     include($ENV{IDF_PATH}/tools/cmake/project.cmake)
     project(PROJECT_NAME)
     ```
     Replace `PROJECT_NAME` with the actual project name.
   - `main/CMakeLists.txt` (Component-level) — **MUST contain SRCS, never leave it empty:**
     ```
     idf_component_register(SRCS "main.c" INCLUDE_DIRS ".")
     ```
   - `main/main.c` (Your main C code with `void app_main(void)`)
   - `sdkconfig` (Basic configuration, can be minimal or empty)
   Do NOT skip any of these 4 files when initializing a project.
3. **Tool Usage Rules:**
   - When using `write_file` or viewing files, always verify the path is relative to the Root (e.g. `main/main.c` not `project_name/main/main.c`).
   - Do NOT `cd` into new sub-folders during project creation.
4. **Self-Correction:**
   - If you detect a nested structure (e.g., a project folder inside the current project), you MUST proactively suggest moving files to the Root to comply with ESP-IDF requirements.

### TOOL USE & BEHAVIOR RULES (CRITICAL):
- **Path Precision Contract**: Before every `write_file` call, you must explicitly state the full relative path it's about to write (e.g. "Writing to: main/main.c"). This prevents silent mis-placement.
- **Idempotent Write Rule**: Always write the COMPLETE file content in a single `write_file` call. Never truncate, never write partial functions. If token pressure is an issue, warn the user instead of writing incomplete code.
- **Scoped Edit Confirmation**: When the user says 'fix X', only modify the file(s) X lives in. State which file(s) will change before writing. Do not rewrite unrelated files.
- **Session Context Reset**: At the start of every new conversation, silently call `read_file` on `main/main.c` (if it exists) to reload project state before responding. Do not ask the user to re-explain what their project does.
- You MUST use tools to see the project state before making changes.
- **Diff Review Workflow:** When you use `write_file` to modify an *existing* file, the system intercepts it and presents a Diff to the user in the main editor. You MUST NOT say "I have updated the file." You MUST say: "I have proposed changes. Please review the diff in the editor and click Keep or Undo."
- **Tool Execution Priority:** When you need to modify a file, you MUST call the `write_file` tool IMMEDIATELY after your initial thought process. Do NOT write long explanations before calling the tool to avoid hitting token limits.
- **No Code in Chat (Anti-Yapping Rule):** Since we use an Inline Diff Editor, NEVER output the actual C code blocks or diffs in your text response. Your chat response should be a maximum of 1-2 short sentences.
- **MANDATORY Edit Workflow:** เมื่อผู้ใช้สั่งให้ "แก้ไขโค้ด" หรือ "แก้ไฟล์" คุณ **MUST** ปฏิบัติตามลำดับ 3 ขั้นตอนนี้เสมอ:
  1) **อ่านและตรวจสอบ (READ):** เรียกใช้เครื่องมือ `read_file` ทุกครั้งเพื่อตรวจสอบเนื้อหาไฟล์ล่าสุดก่อนแก้ไข ห้ามเดาโค้ดจากหน่วยความจำ
  2) **เขียนทับเพื่อแก้ไข (WRITE):** **CRITICAL: YOU MUST CALL THE `write_file` COMMAND WITH A JSON TOOL BLOCK.** ห้ามพิมพ์ตอบแค่ข้อความว่า "ผมได้เสนอการเปลี่ยนแปลงแล้ว" โดยไม่เรียกใช้ Tool เด็ดขาด! (ถ้าคุณไม่ยอมเรียก Tool เด็ดขาด ไฟล์ก็จะไม่ถูกเขียน)
  3) **รายงานสรุป (NOTIFY):** พิมพ์บอกผู้ใช้ในแชทสั้นๆ ว่าแก้ไขไฟล์เสร็จแล้ว
- When calling a tool, do not explain what you are doing first. Just call the tool.

### MANDATORY LED MATRIX CODE TEMPLATE (KIDBRIGHT32 iA):
- สำหรับบอร์ด KidBright32 iA หน้าจอ LED Matrix 16x8 ใช้ชิป HT16K33 **เพียงตัวเดียวที่ Address `0x70`**
- **Init commands (ส่งไปแค่ 0x70):** `0x21` (Oscillator ON), `0x81` (Display ON), `0xEF` (Brightness MAX)
- **HARDWARE MAPPING PROHIBITION:** บอร์ดมีการวายริ่งแบบ Interleaved และภาพกลับหัวแนวแกน Y คุณ MUST NOT คิดสูตรเอง! ให้ใช้ฟังก์ชัน `rows_to_columns_16x8()` แบบมี `(7 - row)` ตามโค้ดด้านล่างนี้เสมอ!
- **DIGIT ALIGNMENT REQUIREMENT (FONT 4x7):** หน้าจอประกอบด้วยจอ 8x8 สองตัวต่อกัน (ซ้าย 0-7, ขวา 8-15)
  - แสดงเลข 1 ตัวให้อยู่ตรงกลางจอผสม: บังคับใช้ `col_offset = 6` (อยู่ระหว่างสองจอพอดี)
  - แสดงเลข 2 ตัวให้อยู่ตรงกลางแต่ละจอ: ตัวหน้าใช้ `col_offset = 2` (กึ่งกลางจอซ้าย), ตัวหลังใช้ `col_offset = 10` (กึ่งกลางจอขวา)

### ZERO-HALLUCINATION & STRICT DECLARATION RULE (CRITICAL):
1. **Never Invent Variables:** You are FORBIDDEN from inventing variable names, macros, or functions (e.g., guessing musical notes like `NOTE_P4` which do not exist).
2. **Prove It Before Use:** Before using ANY variable, macro, or function, you MUST verify it exists in the current file using `read_file` or standard ESP-IDF documentation.
3. **Exact Matching:** If the user asks to modify a string or array, strictly modify ONLY the values requested. Do not alter the surrounding architecture unless explicitly asked.

```c
void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                out_cols[col] |= (1 << (7 - row));
            }
        }
    }
}
const uint16_t PATTERN_HEART[8] = {
    0x0000, 0x0660, 0x0FF0, 0x1FF8, 0x0FF0, 0x07E0, 0x03C0, 0x0180
};
void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00;
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];
        buf[2 + (c * 2)] = cols[c + 8];
    }
    i2c_master_write_to_device(I2C_NUM_0, 0x70, buf, sizeof(buf), pdMS_TO_TICKS(100));
}
```

SMART ERROR RECOVERY:
- **Read -> Fix Loop**: Before fixing any bug, ALWAYS call `read_file` on the affected file first. Never assume the current state from memory. Order: `read_file` -> analyze -> `write_file`.
- **Build Error Taxonomy**:
  * `undefined reference to` -> check `main/CMakeLists.txt` SRCS list
  * `cmake: no such file` -> verify file path matches what's in SRCS
  * `esptool.py failed` -> remind user to check COM port and hold BOOT button
  * Compilation/build failure -> use `read_file` to inspect `CMakeLists.txt` and `sdkconfig` before suggesting a fix.
  * 'I2C Timeout' or ESP_FAIL during I2C -> remind the user to check the Physical Pull-up Resistors, Power Supply, and verify correct I2C pins (SDA=21, SCL=22).

CODE QUALITY & FORMATTING:
ALWAYS #include <string.h> and #include "driver/gpio.h" at the top of your files.

SAFE STRING FORMATTING: NEVER use `sprintf` with tightly packed buffers. ALWAYS use `snprintf` with >=16 byte arrays to prevent `-Werror=format-overflow=` in ESP-IDF v5.x.

ALWAYS use ESP_LOGI or ESP_LOGE instead of printf for debugging.

NO LOG SPAM IN LOOPS (CRITICAL): NEVER put ESP_LOGI directly inside a fast while(1) loop without a state-change check.

AVOID NAMING COLLISIONS (CRITICAL): NEVER name your own custom helper functions the exact same name as official ESP-IDF APIs.

CRITICAL: DO NOT use ESP_ERROR_CHECK() for i2c_master_cmd_begin or any I2C read/write commands! Handle errors gracefully.

NO STANDARD C RANDOM (CRITICAL): NEVER use random() or srandom(). Use esp_random() or kb_random_range().

VIBE CODER UI INTEGRATION:
When generating code, if there are multiple files (e.g., main.c and header.h), provide them in separate code blocks, each with its own [FILE: path/to/file] header.

LANGUAGE & TONE: Thai language preferred. Supportive Technical Partner tone.

FINAL SANITY CHECK & HARDWARE RULES:
DEFAULT BOARD = KidBright32 iA. Single HT16K33 at 0x70, Buzzer at GPIO 13.
CRITICAL BUTTON PINS: SW1 = GPIO_NUM_16, SW2 = GPIO_NUM_14. Active LOW.
CRITICAL I2C RULE: Use legacy API (#include "driver/i2c.h") and i2c_master_write_to_device. NEVER use driver/i2c_master.h.
CRITICAL BUZZER (LEDC) RULE: Use #include "driver/ledc.h". Use LEDC_TIMER_10_BIT and LEDC_TIMER_0.
CRITICAL LDR RULE: The on-board LDR (GPIO36 / ADC1_CH0) on KidBright32 iA uses an INVERTED voltage-divider circuit:
  - MORE light  → LDR resistance DECREASES → ADC Raw value is LOW  (~0–100)
  - LESS light  → LDR resistance INCREASES → ADC Raw value is HIGH (~700–900+)
  - ALWAYS apply an EMA (Exponential Moving Average) filter and time-spaced sampling (`esp_rom_delay_us(500)`) in multi-sampling loops to stabilize readings from 50Hz AC noise.
  - USE Linear Mapping with constants like `LDR_ADC_MIN_VAL` (e.g. 0) and `LDR_ADC_MAX_VAL` (e.g. 900) to map percentages. Do NOT hardcode the max as 4095!
  - NEVER write thresholds as "higher raw = brighter". Always use inverted logic.
  NEVER use Voltage for LDR classification — always use Raw values directly.
  NEVER call adc_calibration or include adc_cali.h when only reading LDR.

### EXTERNAL SENSORS & ACTUATORS RULES (V1.3/V1.6):
- **V1.3 vs V1.6:** V1.3 DOES NOT support Analog Input on IN1-IN4. V1.6 supports it (ADC1 CH4-CH7). Always check board version before using Analog sensors (like external LDR).
- **I2C BUS (BME280/LCD):** External I2C screens and BME280 share `I2C_NUM_0` with the LED Matrix. **DO NOT** reinstall the I2C driver if it's already installed.
- **DS18B20:** When using waterproof DS18B20 on 1-Wire, you MUST use a 4.7k pull-up resistor.
- **MOTORS/RELAYS:** **NEVER** drive Fan/Vibration motors directly from GPIO (max 40mA). ALWAYS use a transistor, driver module, or relay.
- **ACTIVE LOW OUTPUTS:** OUT1, OUT2, and USB Port outputs are **ACTIVE LOW** (`gpio_set_level(..., 0)` turns the Output/Relay ON).
- **BUZZERS:** Active Buzzers need Digital HIGH/LOW. Passive Buzzers need PWM (`ledc`).

### COMPONENT MANAGER RULE:
- ถ้าต้องการ library นอก ESP-IDF core (เช่น led_strip, mqtt, cJSON), ให้เรียก tool `install_idf_library` ก่อน write_file EVERY TIME
- ห้าม hardcode component path หรือสร้าง `idf_component.yml` ด้วยมือ — ให้ tool จัดการ
- ตัวอย่างการใช้: `install_idf_library("espressif/led_strip")` หรือ `install_idf_library("espressif/led_strip^2.5.3")`
- หลัง install สำเร็จแล้วค่อย write_file และ run_command `idf.py build`

AUTONOMY & RESEARCH:
DO NOT say "I don't know" without using web_search first.
Check knowledge_search before searching the web.

ENVIRONMENT:
Framework: ESP-IDF. Build Tools: idf.py, cmake, ninja.
Board: KidBright32 — HT16K33 LED Matrix (I2C addr 0x70), Buzzer GPIO_NUM_13, I2C SDA=21/SCL=22, Buttons SW1=16/SW2=14.
When you need ESP-IDF, use run_command with commands like idf.py build, idf.py flash, idf.py set-target esp32.
Do NOT ask the user to install ESP-IDF again unless the tool result explicitly says ESP-IDF is missing."#;

fn get_tools() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read file content. Always use this before editing a file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to project root" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "CRITICAL: You MUST use this tool to write or modify any code. DO NOT put code blocks in your chat response. Overwrites existing content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to project root (e.g. main/main.c)" },
                        "content": { "type": "string", "description": "The FULL complete new file content. DO NOT truncate. DO NOT put comments like '// rest of code here', write the entire file." }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in a directory (shallow, one level).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to list (use '.' for root)" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_file_tree",
                "description": "Recursively list all files in the project as an indented tree. Use this to understand the full project structure at a glance before making changes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum depth to recurse (default 4, max 8)"
                        }
                    },
                    "required": []
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_in_files",
                "description": "Search for a text pattern (regex) across all source files in the project. Returns matching lines with file path and line number. Use this to find function definitions, usages, or error strings.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search for"
                        },
                        "file_extension": {
                            "type": "string",
                            "description": "Optional file extension filter, e.g. 'c', 'h', 'cmake'. Leave empty to search all text files."
                        }
                    },
                    "required": ["pattern"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "diff_file",
                "description": "Compute a unified diff between the current content of a file and proposed new content, without writing anything. Use this to preview changes before calling write_file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to project root" },
                        "new_content": { "type": "string", "description": "The proposed new file content" }
                    },
                    "required": ["path", "new_content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Run a shell command (e.g. 'idf.py build'). Output is returned after the command completes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "The command string" }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "esp_idf_info",
                "description": "Get resolved ESP-IDF paths and execution hints for this runtime.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "install_idf_library",
                "description": "Install an ESP-IDF component dependency using idf.py add-dependency.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "component": { "type": "string", "description": "Component identifier, e.g. espressif/led_strip or espressif/led_strip^2.5.3" }
                    },
                    "required": ["component"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "CRITICAL: Search the internet for latest technical documentation, ESP-IDF API changes, hardware specs, or code examples when your internal knowledge is insufficient or potentially outdated.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The technical search query" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "knowledge_search",
                "description": "Search the local knowledge_base folder for project-specific documentation, rules, or technical notes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query or keywords" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_project_workspace",
                "description": "Create a new project workspace directory. Prompts the user to pick a folder, then makes a subfolder with project_name inside it, and switches the IDE workspace to it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_name": { "type": "string", "description": "The name of the new project (e.g. my_esp_project)." }
                    },
                    "required": ["project_name"]
                }
            }
        }
    ])
}

// ── Conversation loop (OpenAI-compatible) ─────────────────────────────────────

async fn run_conversation_loop(
    app_handle: &AppHandle,
    api_key: &str,
    model: &str,
    base_url: &str,
    mut messages: Vec<ChatMessage>,
    project_path: &mut PathBuf,
    is_openrouter: bool,
    // FIX: Arc<str> instead of &str — survives async boundaries, zero-cost to clone.
    message_id: Arc<str>,
    no_workspace: &mut bool,
) -> Result<(), String> {
    let client = Client::new();
    let tools = get_tools();

    let model_supports_tools = if model.ends_with(":free") {
        model.contains("deepseek")
            || model.contains("qwen")
            || model.contains("devstral")
            || model.contains("mimo")
            || model.contains("arcee")
            || model.contains("nemotron")
            || model.contains("hermes")
            || model.contains("llama-3.3")
            || model.contains("gpt-oss")
    } else {
        true
    };

    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 4;
    let mut retry_count: u32 = 0;
    // FIX: Guard against infinite tool-call loops.
    let mut tool_turns: u32 = 0;

    loop {
        let api_messages = build_api_messages(SYSTEM_PROMPT, &messages, model);
        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true
        });
        if model_supports_tools {
            body["tools"] = tools.clone();
        }

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let mut request = client.post(&url).header("Content-Type", "application/json");
        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        if is_openrouter {
            request = request
                .header("HTTP-Referer", "https://github.com/vibeKidbright")
                .header("X-Title", "vibeKidbright IDE");
        }

        let response = request
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("Connection to {} failed: {}", url, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            let provider_name = if base_url.contains("googleapis") {
                "Google AI"
            } else if is_openrouter {
                "OpenRouter"
            } else {
                "Cloud/Local"
            };
            if status.as_u16() == 429 {
                retry_count += 1;
                if retry_count > MAX_RETRIES {
                    return Err(format!(
                        "❌ Model '{}' is rate-limited on {} and all {} retries failed.",
                        model, provider_name, MAX_RETRIES
                    ));
                }
                let _ = app_handle.emit(
                    "terminal-output",
                    format!("[AI] ⚠️ Rate limited (attempt {}/{}). Retrying in {}s...", retry_count, MAX_RETRIES, RETRY_DELAY_SECS),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                continue;
            }
            if status.as_u16() == 402 {
                return Err(format!("💳 {}: No credits remaining. Check your billing dashboard or switch to a free model.", provider_name));
            }
            if status.as_u16() == 401 {
                return Err(format!("🔑 {}: Invalid API key.", provider_name));
            }
            if status.as_u16() == 404 {
                return Err(format!("❌ Model '{}' not found on {}.", model, provider_name));
            }
            return Err(format!(
                "Server error {} from {}: {}",
                status.as_u16(),
                provider_name,
                body_text.chars().take(300).collect::<String>()
            ));
        }

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new();
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            if let Some(state) = app_handle.try_state::<AiAbortState>() {
                if state.0.load(Ordering::SeqCst) {
                    return Err("Generation stopped by user.".to_string());
                }
            }
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                let event: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let choice = &event["choices"][0];
                let delta = &choice["delta"];
                if let Some(content) = delta["content"].as_str() {
                    accumulated_text.push_str(content);
                    let _ = app_handle.emit("ai-chat-delta", content.to_string());
                }
                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                    for tc in tool_calls {
                        let index = tc["index"].as_u64().unwrap_or(0) as usize;
                        while pending_tool_calls.len() <= index {
                            pending_tool_calls.push(PendingToolCall {
                                id: String::new(), name: String::new(), arguments: String::new(),
                                thought_signature: None,
                            });
                        }
                        if let Some(id) = tc["id"].as_str() {
                            pending_tool_calls[index].id = id.to_string();
                        }
                        if let Some(func) = tc["function"].as_object() {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                pending_tool_calls[index].name = name.to_string();
                                let _ = app_handle.emit(
                                    "ai-chat-tool-start",
                                    json!({ "name": name, "id": &pending_tool_calls[index].id }).to_string(),
                                );
                            }
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                pending_tool_calls[index].arguments.push_str(args);
                            }
                        }
                    }
                }
                // Legacy function_call delta (some local servers)
                if let Some(function_call) = delta["function_call"].as_object() {
                    if pending_tool_calls.is_empty() {
                        pending_tool_calls.push(PendingToolCall {
                            id: "call_0".to_string(), name: String::new(), arguments: String::new(),
                            thought_signature: None,
                        });
                    }
                    if let Some(name) = function_call.get("name").and_then(|v| v.as_str()) {
                        pending_tool_calls[0].name = name.to_string();
                        let _ = app_handle.emit(
                            "ai-chat-tool-start",
                            json!({ "name": name, "id": &pending_tool_calls[0].id }).to_string(),
                        );
                    }
                    if let Some(args) = function_call.get("arguments").and_then(|v| v.as_str()) {
                        pending_tool_calls[0].arguments.push_str(args);
                    }
                }
            }
        }

        if !pending_tool_calls.is_empty() {
            // FIX: Max tool-turn guard.
            tool_turns += 1;
            if tool_turns > MAX_TOOL_TURNS {
                let _ = app_handle.emit(
                    "ai-chat-error",
                    format!("⚠️ Stopped after {} tool-call turns to prevent an infinite loop. Please rephrase your request.", MAX_TOOL_TURNS),
                );
                break;
            }

            let tool_calls_json: Vec<Value> = pending_tool_calls.iter().map(|tc| json!({
                "id": tc.id, "type": "function",
                "function": { "name": tc.name, "arguments": tc.arguments }
            })).collect();

            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: json!({
                    "__tool_calls__": tool_calls_json,
                    "__text__": if accumulated_text.is_empty() { Value::Null } else { json!(accumulated_text) }
                }),
            });

            for tc in &pending_tool_calls {
                // FIX: parse errors are now logged, not silently dropped.
                let input: Value = match serde_json::from_str(&tc.arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] ⚠️ Failed to parse args for tool '{}': {} — args: {}", tc.name, e, tc.arguments),
                        );
                        json!({})
                    }
                };
                // FIX: pass &message_id — Arc<str> derefs to &str cleanly.
                let result = execute_tool(app_handle, &tc.name, &input, project_path, &message_id, no_workspace).await;
                let _ = app_handle.emit(
                    "ai-chat-tool-result",
                    json!({ "name": tc.name, "id": tc.id, "result": &result }).to_string(),
                );
                let result_str = if result.is_string() {
                    result.as_str().unwrap().to_string()
                } else {
                    result.to_string()
                };
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: json!({
                        "__tool_response__": true,
                        "tool_call_id": tc.id,
                        "__func_name__": tc.name,
                        "content": result_str
                    }),
                });
            }
            pending_tool_calls.clear();
        } else {
            if !accumulated_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: json!(accumulated_text),
                });
            }
            let _ = app_handle.emit("ai-chat-done", json!({ "text": &accumulated_text }).to_string());
            break;
        }
    }

    Ok(())
}

// ── Google Generative AI conversation loop ────────────────────────────────────

fn build_google_contents(messages: &[ChatMessage]) -> Vec<Value> {
    let mut contents: Vec<Value> = Vec::new();
    for m in messages {
        let role = match m.role.as_str() {
            "assistant" => "model",
            _ => "user",
        };
        if let Some(obj) = m.content.as_object() {
            if obj.contains_key("__tool_calls__") {
                let tool_calls = obj["__tool_calls__"].as_array().cloned().unwrap_or_default();
                let mut parts: Vec<Value> = Vec::new();
                if let Some(text) = obj.get("__text__").and_then(|t| t.as_str()) {
                    if !text.is_empty() { parts.push(json!({ "text": text })); }
                }
                for tc in &tool_calls {
                    let func_name = tc["function"]["name"].as_str().unwrap_or("");
                    // FIX: log parse errors instead of silently using json!({}).
                    let args: Value = tc["function"]["arguments"]
                        .as_str()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_else(|| {
                            eprintln!("[AI] Could not parse Google tool args for '{}'", func_name);
                            json!({})
                        });
                    // Echo back thought_signature required by Gemini thinking models.
                    // Without this, the API returns a 400 error on subsequent turns.
                    let mut fc_obj = json!({ "name": func_name, "args": args });
                    if let Some(sig) = tc["function"]["thought_signature"].as_str() {
                        fc_obj["thoughtSignature"] = json!(sig);
                    }
                    parts.push(json!({ "functionCall": fc_obj }));
                }
                if !parts.is_empty() {
                    contents.push(json!({ "role": "model", "parts": parts }));
                }
                continue;
            }
            if obj.contains_key("__tool_response__") {
                let func_name = obj.get("__func_name__")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| obj["tool_call_id"].as_str().unwrap_or(""));
                let content_str = obj["content"].as_str().unwrap_or("");
                let response_value: Value = serde_json::from_str(content_str)
                    .unwrap_or_else(|_| json!({ "result": content_str }));
                let response_obj = if response_value.is_object() {
                    response_value
                } else {
                    json!({ "result": response_value })
                };
                contents.push(json!({
                    "role": "user",
                    "parts": [{ "functionResponse": { "name": func_name, "response": response_obj } }]
                }));
                continue;
            }
        }
        let text = m.content.as_str().unwrap_or("");
        if !text.is_empty() {
            contents.push(json!({ "role": role, "parts": [{ "text": text }] }));
        }
    }
    contents
}

fn get_google_tools() -> Value {
    json!([{
        "functionDeclarations": [
            { "name": "read_file", "description": "Read file content. Always use this before editing a file.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING", "description": "Path relative to project root" } }, "required": ["path"] } },
            { "name": "write_file", "description": "CRITICAL: You MUST use this tool to write or modify any code. DO NOT put code blocks in your chat response. Overwrites existing content.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING", "description": "Path relative to project root" }, "content": { "type": "STRING", "description": "The FULL complete new file content. DO NOT truncate or omit code." } }, "required": ["path", "content"] } },
            { "name": "list_files", "description": "List files in a directory.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING" } }, "required": ["path"] } },
            { "name": "get_file_tree", "description": "Recursively list all files in the project as an indented tree.", "parameters": { "type": "OBJECT", "properties": { "max_depth": { "type": "INTEGER", "description": "Max depth (default 4)" } } } },
            { "name": "search_in_files", "description": "Search for a regex pattern across all source files. Returns matches with file path and line number.", "parameters": { "type": "OBJECT", "properties": { "pattern": { "type": "STRING" }, "file_extension": { "type": "STRING" } }, "required": ["pattern"] } },
            { "name": "diff_file", "description": "Preview a unified diff between the current file and proposed new content, without writing.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING" }, "new_content": { "type": "STRING" } }, "required": ["path", "new_content"] } },
            { "name": "run_command", "description": "Run a shell command.", "parameters": { "type": "OBJECT", "properties": { "command": { "type": "STRING" } }, "required": ["command"] } },
            { "name": "esp_idf_info", "description": "Get resolved ESP-IDF paths.", "parameters": { "type": "OBJECT", "properties": {} } },
            { "name": "web_search", "description": "Search the internet for technical docs.", "parameters": { "type": "OBJECT", "properties": { "query": { "type": "STRING" } }, "required": ["query"] } },
            { "name": "knowledge_search", "description": "Search the local knowledge_base folder.", "parameters": { "type": "OBJECT", "properties": { "query": { "type": "STRING" } }, "required": ["query"] } },
            { "name": "create_project_workspace", "description": "Create a new project workspace directory. Call FIRST when no workspace is open.", "parameters": { "type": "OBJECT", "properties": { "project_name": { "type": "STRING" } }, "required": ["project_name"] } }
        ]
    }])
}

async fn run_google_conversation_loop(
    app_handle: &AppHandle,
    api_key: &str,
    model: &str,
    mut messages: Vec<ChatMessage>,
    project_path: &mut PathBuf,
    // FIX: Arc<str> instead of &str.
    message_id: Arc<str>,
    no_workspace: &mut bool,
) -> Result<(), String> {
    let client = Client::new();
    let google_tools = get_google_tools();

    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 4;
    let mut retry_count: u32 = 0;
    // FIX: Guard against infinite tool-call loops.
    let mut tool_turns: u32 = 0;

    loop {
        let contents = build_google_contents(&messages);
        let body = json!({
            "systemInstruction": { "parts": [{ "text": SYSTEM_PROMPT }] },
            "contents": contents,
            "tools": google_tools,
            "generationConfig": { "temperature": 0.7 }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, api_key
        );

        let response = client.post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("Connection to Google AI failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                retry_count += 1;
                if retry_count > MAX_RETRIES {
                    return Err(format!("❌ Google AI model '{}' rate-limited. All {} retries failed.", model, MAX_RETRIES));
                }
                let _ = app_handle.emit(
                    "terminal-output",
                    format!("[AI] ⚠️ Rate limited (attempt {}/{}). Retrying in {}s...", retry_count, MAX_RETRIES, RETRY_DELAY_SECS),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                continue;
            }
            if status.as_u16() == 400 {
                return Err(format!("❌ Google AI 400 Bad Request: {}", body_text.chars().take(400).collect::<String>()));
            }
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err("🔑 Google AI: Invalid or unauthorized API key.".to_string());
            }
            if status.as_u16() == 404 {
                return Err(format!("❌ Google AI model '{}' not found.", model));
            }
            return Err(format!("Google AI error {}: {}", status.as_u16(), body_text.chars().take(300).collect::<String>()));
        }

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new();
        let mut buffer = String::new();
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();

        while let Some(chunk) = stream.next().await {
            if let Some(state) = app_handle.try_state::<AiAbortState>() {
                if state.0.load(Ordering::SeqCst) {
                    return Err("Generation stopped by user.".to_string());
                }
            }
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                let event: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(candidates) = event["candidates"].as_array() {
                    for candidate in candidates {
                        if let Some(parts) = candidate["content"]["parts"].as_array() {
                            for part in parts {
                                if let Some(text) = part["text"].as_str() {
                                    accumulated_text.push_str(text);
                                    let _ = app_handle.emit("ai-chat-delta", text.to_string());
                                }
                                if let Some(fc) = part["functionCall"].as_object() {
                                    let name = fc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let args = fc.get("args").cloned().unwrap_or(json!({}));
                                    // Capture thought_signature from Gemini thinking models.
                                    // Must be echoed back verbatim in the next turn's history.
                                    let thought_signature = fc.get("thoughtSignature")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    let id = format!("call_{}", pending_tool_calls.len());
                                    let _ = app_handle.emit(
                                        "ai-chat-tool-start",
                                        json!({ "name": &name, "id": &id }).to_string(),
                                    );
                                    pending_tool_calls.push(PendingToolCall {
                                        id, name, arguments: args.to_string(), thought_signature,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !pending_tool_calls.is_empty() {
            // FIX: Max tool-turn guard.
            tool_turns += 1;
            if tool_turns > MAX_TOOL_TURNS {
                let _ = app_handle.emit(
                    "ai-chat-error",
                    format!("⚠️ Stopped after {} tool-call turns to prevent an infinite loop.", MAX_TOOL_TURNS),
                );
                break;
            }

            let tool_calls_json: Vec<Value> = pending_tool_calls.iter().map(|tc| {
                let mut func = json!({ "name": tc.name, "arguments": tc.arguments });
                // Preserve thought_signature so build_google_contents can echo it back.
                if let Some(sig) = &tc.thought_signature {
                    func["thought_signature"] = json!(sig);
                }
                json!({ "id": tc.id, "type": "function", "function": func })
            }).collect();

            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: json!({
                    "__tool_calls__": tool_calls_json,
                    "__text__": if accumulated_text.is_empty() { Value::Null } else { json!(accumulated_text) }
                }),
            });

            for tc in &pending_tool_calls {
                // FIX: log parse errors.
                let input: Value = match serde_json::from_str(&tc.arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] ⚠️ Failed to parse Google tool args for '{}': {}", tc.name, e),
                        );
                        json!({})
                    }
                };
                // FIX: &message_id derefs Arc<str> to &str cleanly.
                let result = execute_tool(app_handle, &tc.name, &input, project_path, &message_id, no_workspace).await;
                let _ = app_handle.emit(
                    "ai-chat-tool-result",
                    json!({ "name": tc.name, "id": tc.id, "result": &result }).to_string(),
                );
                let result_str = if result.is_string() {
                    result.as_str().unwrap().to_string()
                } else {
                    result.to_string()
                };
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: json!({
                        "__tool_response__": true,
                        "tool_call_id": tc.name,
                        "__func_name__": tc.name,
                        "content": result_str
                    }),
                });
            }
            pending_tool_calls.clear();
        } else {
            if !accumulated_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: json!(accumulated_text),
                });
            }
            let _ = app_handle.emit("ai-chat-done", json!({ "text": &accumulated_text }).to_string());
            break;
        }
    }

    Ok(())
}

// ── API message builder ────────────────────────────────────────────────────────

fn build_api_messages(system_prompt: &str, messages: &[ChatMessage], model: &str) -> Vec<Value> {
    let mut api_msgs: Vec<Value> = Vec::new();
    let unsupported_system_role =
        model.to_lowercase().contains("gemma") || model.to_lowercase().contains("o1-");
    if !unsupported_system_role {
        api_msgs.push(json!({ "role": "system", "content": system_prompt }));
    }
    let mut first_user_found = false;
    for m in messages {
        if let Some(obj) = m.content.as_object() {
            if obj.contains_key("__tool_calls__") {
                let mut msg = json!({ "role": "assistant" });
                msg["content"] = obj.get("__text__").and_then(|t| t.as_str())
                    .map(|t| json!(t))
                    .unwrap_or(json!(""));
                msg["tool_calls"] = obj["__tool_calls__"].clone();
                api_msgs.push(msg);
            } else if obj.contains_key("__tool_response__") {
                api_msgs.push(json!({
                    "role": "tool",
                    "tool_call_id": obj["tool_call_id"],
                    "content": obj["content"]
                }));
            } else {
                let mut content = m.content.clone();
                if unsupported_system_role && !first_user_found && m.role == "user" {
                    first_user_found = true;
                    if let Some(text) = content.as_str() {
                        content = json!(format!(
                            "[SYSTEM INSTRUCTIONS]\n{}\n\n[USER INPUT]\n{}", system_prompt, text
                        ));
                    }
                }
                api_msgs.push(json!({ "role": m.role.clone(), "content": content }));
            }
        } else {
            let mut content = m.content.clone();
            if unsupported_system_role && !first_user_found && m.role == "user" {
                first_user_found = true;
                if let Some(text) = content.as_str() {
                    content = json!(format!(
                        "[SYSTEM INSTRUCTIONS]\n{}\n\n[USER INPUT]\n{}", system_prompt, text
                    ));
                }
            }
            api_msgs.push(json!({ "role": m.role.clone(), "content": content }));
        }
    }
    api_msgs
}

// ── Tool execution ─────────────────────────────────────────────────────────────

/// execute_tool takes message_id as &str — it doesn't store or spawn, so no Arc needed here.
async fn execute_tool(
    app_handle: &AppHandle,
    name: &str,
    input: &Value,
    project_path: &mut PathBuf,
    message_id: &str,
    no_workspace: &mut bool,
) -> Value {
    if *no_workspace && matches!(name, "write_file" | "run_command" | "read_file" | "list_files" | "get_file_tree" | "search_in_files" | "diff_file") {
        return json!({
            "error": "BLOCKED: No project workspace is open. You MUST call 'create_project_workspace' FIRST."
        });
    }

    match name {
        // ── read_file ──────────────────────────────────────────────────────────
        "read_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            match std::fs::read_to_string(&full_path) {
                Ok(c) => json!({ "result": c }),
                Err(e) => json!({ "error": format!("Error reading file: {}", e) }),
            }
        }

        // ── write_file ─────────────────────────────────────────────────────────
        "write_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let content = input["content"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return json!({ "error": "write_file rejected: empty content." });
            }
            let looks_like_placeholder = (trimmed.starts_with('<') && trimmed.ends_with('>'))
                || trimmed.contains("updated_content")
                || trimmed.contains("<your_code_here>")
                || trimmed.contains("TODO_REPLACE");
            if looks_like_placeholder {
                return json!({ "error": "write_file rejected: placeholder content detected. Send the full, real file contents." });
            }
            if let Some(parent) = full_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let file_exists = full_path.exists();
            if file_exists {
                let old_content = std::fs::read_to_string(&full_path).unwrap_or_default();
                if old_content == content {
                    return json!({ "result": format!("File {} is already up to date.", rel_path) });
                }
                {
                    let mut diffs = get_pending_diffs().lock().unwrap();
                    diffs.insert(full_path.clone(), content.to_string());
                }
                let _ = app_handle.emit("ai-diff-pending", json!({
                    "fullPath": full_path.to_string_lossy(),
                    "relPath": rel_path
                }).to_string());
                json!({ "result": format!("I have proposed changes to '{}'. Please review the diff in the editor and click Keep or Undo.", rel_path) })
            } else {
                if let Some(state) = app_handle.try_state::<AiBackupState>() {
                    let mut backups = state.backups.lock().unwrap();
                    let message_backups = backups.entry(message_id.to_string()).or_insert_with(HashMap::new);
                    if !message_backups.contains_key(&full_path) {
                        message_backups.insert(full_path.clone(), None);
                    }
                }
                match std::fs::write(&full_path, content) {
                    Ok(_) => {
                        let _ = app_handle.emit("file-modified", json!({ "path": full_path.to_string_lossy() }).to_string());
                        json!({ "result": format!("Created new file: {}", rel_path) })
                    }
                    Err(e) => json!({ "error": format!("Error writing file: {}", e) }),
                }
            }
        }

        // ── list_files ─────────────────────────────────────────────────────────
        "list_files" => {
            let rel_path = input["path"].as_str().unwrap_or(".");
            let full_path = project_path.join(rel_path);
            match std::fs::read_dir(&full_path) {
                Ok(entries) => {
                    let mut items: Vec<String> = entries.flatten().filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') || name == "node_modules" || name == "target" { return None; }
                        Some(if e.path().is_dir() { format!("{}/", name) } else { name })
                    }).collect();
                    items.sort();
                    json!({ "result": items.join("\n") })
                }
                Err(e) => json!({ "error": format!("Error listing directory: {}", e) }),
            }
        }

        // ── get_file_tree (NEW) ────────────────────────────────────────────────
        "get_file_tree" => {
            let max_depth = input["max_depth"].as_u64().unwrap_or(4).min(8) as usize;
            let mut output = String::new();
            build_file_tree(project_path, project_path, 0, max_depth, &mut output);
            if output.is_empty() {
                json!({ "result": "(empty directory)" })
            } else {
                json!({ "result": output })
            }
        }

        // ── search_in_files (NEW) ──────────────────────────────────────────────
        "search_in_files" => {
            let pattern_str = input["pattern"].as_str().unwrap_or("");
            let ext_filter = input["file_extension"].as_str().unwrap_or("").to_lowercase();
            let regex = match regex::Regex::new(pattern_str) {
                Ok(r) => r,
                Err(e) => return json!({ "error": format!("Invalid regex '{}': {}", pattern_str, e) }),
            };
            let mut matches: Vec<String> = Vec::new();
            search_files_recursive(project_path, project_path, &regex, &ext_filter, &mut matches, 0);
            if matches.is_empty() {
                json!({ "result": format!("No matches found for pattern '{}'", pattern_str) })
            } else {
                // Cap output to avoid overwhelming the context window.
                matches.truncate(100);
                json!({ "result": matches.join("\n"), "note": "Results capped at 100 matches." })
            }
        }

        // ── diff_file (NEW) ────────────────────────────────────────────────────
        "diff_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let new_content = input["new_content"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            let old_content = if full_path.exists() {
                std::fs::read_to_string(&full_path).unwrap_or_default()
            } else {
                String::new()
            };
            if old_content == new_content {
                return json!({ "result": "No differences — files are identical." });
            }
            let diff = compute_unified_diff(&old_content, new_content, rel_path);
            json!({ "result": diff })
        }

        // ── web_search ─────────────────────────────────────────────────────────
        "web_search" => {
            let query = input["query"].as_str().unwrap_or_default();
            match search_the_web(query).await {
                Ok(results) => json!({ "results": results }),
                Err(e) => json!({ "error": format!("Search failed: {}", e) }),
            }
        }

        // ── knowledge_search ───────────────────────────────────────────────────
        "knowledge_search" => {
            let query = input["query"].as_str().unwrap_or_default();
            knowledge_search(app_handle, project_path, query).await
        }

        // ── create_project_workspace ───────────────────────────────────────────
        "create_project_workspace" => {
            let project_name = input["project_name"].as_str().unwrap_or("my_esp_project");
            if let Some(picked_dir) = rfd::FileDialog::new()
                .set_title("Select a Base Directory for Your New Project")
                .pick_folder()
            {
                let new_proj_path = picked_dir.join(project_name);
                if let Err(e) = std::fs::create_dir_all(&new_proj_path) {
                    return json!({ "error": format!("Failed to create directory: {}", e) });
                }
                *project_path = new_proj_path.clone();
                *no_workspace = false;
                let root_cmake = format!(
                    "cmake_minimum_required(VERSION 3.16)\ninclude($ENV{{IDF_PATH}}/tools/cmake/project.cmake)\nproject({})\n",
                    project_name
                );
                let _ = std::fs::write(new_proj_path.join("CMakeLists.txt"), root_cmake);
                let _ = std::fs::create_dir_all(new_proj_path.join("main"));
                let _ = std::fs::write(
                    new_proj_path.join("main/CMakeLists.txt"),
                    "idf_component_register(SRCS \"main.c\"\n                    INCLUDE_DIRS \".\")\n",
                );
                let _ = std::fs::write(
                    new_proj_path.join("main/main.c"),
                    "#include <stdio.h>\nvoid app_main(void) {\n    printf(\"Hello\\n\");\n}\n",
                );
                let _ = std::fs::write(
                    new_proj_path.join("sdkconfig"),
                    "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n",
                );
                let _ = app_handle.emit("force-project-dir", new_proj_path.to_string_lossy().to_string());
                json!({
                    "result": format!(
                        "Successfully created project workspace: {}. Boilerplate files were automatically created. Use write_file to populate main/main.c.",
                        new_proj_path.display()
                    ),
                    "workspace_path": new_proj_path.to_string_lossy().to_string()
                })
            } else {
                json!({ "error": "User cancelled the folder selection dialog." })
            }
        }

        // ── esp_idf_info ───────────────────────────────────────────────────────
        "esp_idf_info" => {
            if let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) {
                let python = find_idf_python_bin(&tools_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "not found".to_string());
                json!({
                    "result": {
                        "idf_path": idf_path.to_string_lossy(),
                        "tools_path": tools_path.to_string_lossy(),
                        "python": python,
                        "hint": "Use run_command with `idf.py ...`; runtime injects IDF env vars."
                    }
                })
            } else {
                json!({ "error": "ESP-IDF paths are not resolved in this runtime." })
            }
        }

        // ── install_idf_library ────────────────────────────────────────────────
        "install_idf_library" => {
            let component = input["component"].as_str().unwrap_or("").trim();
            if component.is_empty() {
                return json!({ "error": "component is required, e.g. espressif/led_strip" });
            }
            let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) else {
                return json!({ "error": "ESP-IDF paths are not resolved in this runtime." });
            };
            let Some(python_bin) = find_idf_python_bin(&tools_path) else {
                return json!({ "error": "ESP-IDF python environment not found." });
            };
            let idf_py = idf_path.join("tools/idf.py");
            let idf_version = std::fs::read_to_string(idf_path.join("version.txt"))
                .unwrap_or_default().trim().to_string();
            let mut cmd = tokio::process::Command::new(&python_bin);
            cmd.arg(&idf_py).arg("add-dependency").arg(component)
                .current_dir(project_path)
                .env("IDF_PATH", &idf_path)
                .env("IDF_TOOLS_PATH", &tools_path)
                .env("ESP_IDF_VERSION", &idf_version)
                .env("PATH", build_ai_idf_path_cached(&tools_path));
            match cmd.output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    if output.status.success() {
                        let result = [stdout.trim(), stderr.trim()]
                            .iter().filter(|s| !s.is_empty())
                            .cloned().collect::<Vec<_>>().join("\n");
                        let result = if result.is_empty() {
                            format!("Installed dependency: {}", component)
                        } else {
                            result
                        };
                        json!({ "result": result })
                    } else {
                        let err = if !stderr.trim().is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
                        json!({ "error": format!("Failed to install {}: {}", component, err) })
                    }
                }
                Err(e) => json!({ "error": format!("Failed to run idf.py add-dependency: {}", e) }),
            }
        }

        // ── run_command ────────────────────────────────────────────────────────
        "run_command" => {
            let command = input["command"].as_str().unwrap_or("");
            if command.contains("esptool.py") {
                return json!({ "error": "Direct esptool.py usage is disabled. Use `idf.py build flash` instead." });
            }
            let mut exec_command = command.to_string();
            let mut process = if cfg!(target_os = "windows") {
                let mut c = tokio::process::Command::new("cmd");
                c.arg("/C"); c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.arg("-c"); c
            };

            if let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) {
                let idf_version = std::fs::read_to_string(idf_path.join("version.txt"))
                    .unwrap_or_default().trim().to_string();
                // FIX: Use cached IDF path — no directory scan on every command.
                process
                    .env("IDF_PATH", &idf_path)
                    .env("IDF_TOOLS_PATH", &tools_path)
                    .env("ESP_IDF_VERSION", idf_version)
                    .env("PATH", build_ai_idf_path_cached(&tools_path));
                if let Some(python_bin) = find_idf_python_bin(&tools_path) {
                    if let Some(rel) = command.trim_start().strip_prefix("idf.py") {
                        let tail = rel.trim();
                        let idf_py = idf_path.join("tools/idf.py");
                        exec_command = if tail.is_empty() {
                            format!("\"{}\" \"{}\"", python_bin.to_string_lossy(), idf_py.to_string_lossy())
                        } else {
                            format!("\"{}\" \"{}\" {}", python_bin.to_string_lossy(), idf_py.to_string_lossy(), tail)
                        };
                    }
                }
            }

            match process.arg(&exec_command).current_dir(project_path).output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() { result.push_str(&stdout); }
                    if !stderr.is_empty() {
                        if !result.is_empty() { result.push_str("\n--- stderr ---\n"); }
                        result.push_str(&stderr);
                    }
                    if result.is_empty() {
                        json!({ "result": "Command completed successfully (no output)" })
                    } else {
                        if result.len() > 10000 {
                            result.truncate(10000);
                            result.push_str("\n... (output truncated)");
                        }
                        json!({ "result": result })
                    }
                }
                Err(e) => json!({ "error": format!("Error running command: {}", e) }),
            }
        }

        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

// ── Helper: recursive file tree ───────────────────────────────────────────────

fn build_file_tree(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    output: &mut String,
) {
    if depth > max_depth { return; }
    let skip = ["node_modules", "target", ".git", "build", ".embeddings.json"];
    let mut entries: Vec<_> = match std::fs::read_dir(current) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || skip.iter().any(|&s| s == name) { continue; }
        let indent = "  ".repeat(depth);
        let is_dir = entry.path().is_dir();
        output.push_str(&format!("{}{}{}\n", indent, name, if is_dir { "/" } else { "" }));
        if is_dir {
            build_file_tree(root, &entry.path(), depth + 1, max_depth, output);
        }
    }
}

// ── Helper: recursive file search ─────────────────────────────────────────────

fn search_files_recursive(
    root: &Path,
    current: &Path,
    regex: &regex::Regex,
    ext_filter: &str,
    matches: &mut Vec<String>,
    depth: usize,
) {
    if depth > 8 { return; }
    let skip = ["node_modules", "target", ".git", "build"];
    let entries: Vec<_> = match std::fs::read_dir(current) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || skip.iter().any(|&s| s == name) { continue; }
        if path.is_dir() {
            search_files_recursive(root, &path, regex, ext_filter, matches, depth + 1);
        } else {
            if !ext_filter.is_empty() {
                let file_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if file_ext != ext_filter { continue; }
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                for (i, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        matches.push(format!("{}:{}: {}", rel.display(), i + 1, line.trim()));
                        if matches.len() >= 100 { return; }
                    }
                }
            }
        }
    }
}

// ── Helper: simple unified diff ────────────────────────────────────────────────

fn compute_unified_diff(old: &str, new: &str, path: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let mut out = format!("--- {}\n+++ {}\n", path, path);

    // Simple line-by-line diff using longest common subsequence approach.
    // For production you'd use the `similar` crate, but this keeps zero extra deps.
    let mut i = 0;
    let mut j = 0;
    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            out.push_str(&format!(" {}\n", old_lines[i]));
            i += 1; j += 1;
        } else if j < new_lines.len() && (i >= old_lines.len() || old_lines[i] != new_lines[j]) {
            out.push_str(&format!("+{}\n", new_lines[j]));
            j += 1;
        } else {
            out.push_str(&format!("-{}\n", old_lines[i]));
            i += 1;
        }
    }
    out
}

// ── Web search ────────────────────────────────────────────────────────────────

/// FIX: DuckDuckGo scraper with fallback user-agents and a secondary fallback to Bing.
async fn search_the_web(query: &str) -> Result<Value, String> {
    // Try DDG with two different user-agents before falling back to Bing.
    let user_agents = [
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0",
    ];
    for ua in &user_agents {
        if let Ok(results) = try_ddg_search(query, ua).await {
            if !results.as_array().map(|a| a.is_empty()).unwrap_or(true) {
                return Ok(results);
            }
        }
    }
    // Fallback to Bing
    try_bing_search(query).await
}

async fn try_ddg_search(query: &str, user_agent: &str) -> Result<Value, String> {
    let client = Client::builder()
        .user_agent(user_agent)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let response = client.get(&url).send().await
        .map_err(|e| format!("DDG request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("DDG error: {}", response.status()));
    }
    let html_content = response.text().await
        .map_err(|e| format!("Failed to read DDG body: {}", e))?;
    let document = Html::parse_document(&html_content);
    let result_selector = Selector::parse(".result").map_err(|_| "Bad selector")?;
    let title_selector = Selector::parse(".result__a").map_err(|_| "Bad selector")?;
    let snippet_selector = Selector::parse(".result__snippet").map_err(|_| "Bad selector")?;
    let mut results = Vec::new();
    for element in document.select(&result_selector).take(5) {
        let title = element.select(&title_selector).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let link = element.select(&title_selector).next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("").to_string();
        let snippet = element.select(&snippet_selector).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if !title.is_empty() {
            results.push(json!({ "title": title, "link": link, "snippet": snippet }));
        }
    }
    Ok(json!(results))
}

async fn try_bing_search(query: &str) -> Result<Value, String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; Googlebot/2.1)")
        .build()
        .map_err(|e| format!("Failed to build Bing client: {}", e))?;
    let url = format!("https://www.bing.com/search?q={}", urlencoding::encode(query));
    let response = client.get(&url).send().await
        .map_err(|e| format!("Bing request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Bing error: {}", response.status()));
    }
    let html = response.text().await
        .map_err(|e| format!("Failed to read Bing body: {}", e))?;
    let document = Html::parse_document(&html);
    // Bing results are in <li class="b_algo"> elements
    let result_sel = Selector::parse("li.b_algo").map_err(|_| "Bad selector")?;
    let title_sel = Selector::parse("h2 a").map_err(|_| "Bad selector")?;
    let snippet_sel = Selector::parse(".b_caption p").map_err(|_| "Bad selector")?;
    let mut results = Vec::new();
    for element in document.select(&result_sel).take(5) {
        let title = element.select(&title_sel).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let link = element.select(&title_sel).next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("").to_string();
        let snippet = element.select(&snippet_sel).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if !title.is_empty() {
            results.push(json!({ "title": title, "link": link, "snippet": snippet, "source": "bing" }));
        }
    }
    Ok(json!(results))
}

// ── Embeddings ────────────────────────────────────────────────────────────────

async fn get_embeddings_internal(api_key: &str, mut base_url: String, text: &str) -> Result<Vec<f32>, String> {
    if !base_url.starts_with("http") && !base_url.is_empty() {
        base_url = format!("http://{}", base_url);
    }
    if (base_url.contains("localhost") || base_url.contains("127.0.0.1")) && !base_url.contains("/v1") {
        base_url = format!("{}/v1", base_url.trim_end_matches('/'));
    }
    let client = Client::new();
    let res = client.post(format!("{}/embeddings", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({ "input": text, "model": "text-embedding-3-small" }))
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;
    let data: Value = res.json().await.map_err(|e| format!("Failed to parse embedding response: {}", e))?;
    if let Some(err) = data["error"].as_object() {
        return Err(err["message"].as_str().unwrap_or("Unknown API error").to_string());
    }
    let embedding = data["data"][0]["embedding"]
        .as_array()
        .ok_or("No embedding data in response")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();
    Ok(embedding)
}

async fn get_embeddings(_app_handle: &AppHandle, text: &str) -> Result<Vec<f32>, String> {
    let (api_key, base_url) = {
        let config = read_config();
        (
            config["api_key"].as_str().unwrap_or("").to_string(),
            config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string(),
        )
    };
    get_embeddings_internal(&api_key, base_url, text).await
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let dot: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let n1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let n2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    if n1 > 0.0 && n2 > 0.0 { dot / (n1 * n2) } else { 0.0 }
}

/// FIX: Sentence-boundary chunking — splits on ". ", "! ", "? ", and newlines
/// to keep embedded context semantically coherent.
fn chunk_text(text: &str, target_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    // Split on sentence terminators first, then fall back to word boundaries.
    let sentence_ends: Vec<usize> = text.char_indices()
        .filter(|(i, c)| {
            (*c == '.' || *c == '!' || *c == '?')
                && text.get(*i + 1..).map(|s| s.starts_with(' ') || s.starts_with('\n')).unwrap_or(true)
        })
        .map(|(i, _)| i + 1)
        .collect();

    let mut last = 0;
    let mut sentences: Vec<&str> = Vec::new();
    for &end in &sentence_ends {
        sentences.push(&text[last..end]);
        last = end;
    }
    if last < text.len() {
        sentences.push(&text[last..]);
    }

    let mut overlap_buf = String::new();

    for sentence in &sentences {
        if current.len() + sentence.len() > target_size && !current.is_empty() {
            chunks.push(current.clone());
            // Carry overlap: take the tail of current up to `overlap` chars.
            overlap_buf.clear();
            let tail_start = current.len().saturating_sub(overlap);
            overlap_buf.push_str(&current[tail_start..]);
            current = overlap_buf.clone();
            current.push(' ');
        }
        current.push_str(sentence);
    }
    if !current.trim().is_empty() {
        chunks.push(current);
    }
    if chunks.is_empty() && !text.is_empty() {
        // Absolute fallback for text with no sentence terminators.
        chunks.push(text.chars().take(target_size).collect());
    }
    chunks
}

// ── Helper: recursive KB file collector ──────────────────────────────────────
// Walks knowledge_base/ recursively and collects text/doc files (md, txt, c, h).
// Returns Vec of (absolute_path, relative_key) pairs.
// The relative_key uses forward slashes so it is OS-independent (e.g. "sensor_examples/accel_kxtj3.c").

fn collect_kb_files_inner(root: &Path, current: &Path, result: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir(current) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; } // skip hidden / .embeddings.json
        if path.is_dir() {
            collect_kb_files_inner(root, &path, result);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "txt" | "md" | "c" | "h") {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let rel_key = rel.to_string_lossy().replace('\\', "/");
                result.push((path.clone(), rel_key));
            }
        }
    }
}

fn collect_kb_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    collect_kb_files_inner(root, root, &mut result);
    result
}

async fn reindex_knowledge_base(project_path: &Path) -> Result<usize, String> {
    let kb_path = project_path.join("knowledge_base");
    if !kb_path.exists() { return Ok(0); }
    let index_file = kb_path.join(".embeddings.json");
    let mut index: VectorIndex = if index_file.exists() {
        let data = std::fs::read_to_string(&index_file).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        VectorIndex::default()
    };
    let (api_key, base_url) = {
        let config = read_config();
        (
            config["api_key"].as_str().unwrap_or("").to_string(),
            config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string(),
        )
    };
    // FIX: Use recursive collector so sensor_examples/ and other subfolders are indexed.
    let all_files = collect_kb_files(&kb_path);
    let mut changed = false;
    for (file_path, rel_key) in &all_files {
        let mtime = std::fs::metadata(file_path)
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0);
        if index.last_indexed.get(rel_key).cloned().unwrap_or(0) < mtime {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                index.chunks.retain(|c| &c.file_name != rel_key);
                // Use sentence-boundary chunking.
                let chunks = chunk_text(&content, 800, 100);
                for chunk_content in chunks {
                    if let Ok(embedding) = get_embeddings_internal(&api_key, base_url.clone(), &chunk_content).await {
                        index.chunks.push(KnowledgeChunk {
                            file_name: rel_key.clone(), content: chunk_content, embedding,
                        });
                    }
                }
                index.last_indexed.insert(rel_key.clone(), mtime);
                changed = true;
            }
        }
    }
    if changed {
        let data = serde_json::to_string_pretty(&index).unwrap_or_default();
        let _ = std::fs::write(&index_file, data);
    }
    Ok(index.chunks.len())
}

// ── Knowledge search (with query cache) ───────────────────────────────────────

pub async fn knowledge_search(app_handle: &AppHandle, project_path: &Path, query: &str) -> Value {
    let kb_path = project_path.join("knowledge_base");
    if !kb_path.exists() {
        return json!({ "message": "No knowledge_base folder found." });
    }

    // FIX: Check in-memory query cache before doing any I/O or embeddings.
    {
        let cache = get_kb_query_cache().lock().unwrap();
        if let Some(cached) = cache.get(query) {
            return cached.clone();
        }
    }

    let result = {
        let vector_results = vector_knowledge_search(app_handle, project_path, query).await;
        if vector_results.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
            vector_results
        } else {
            keyword_knowledge_search(&kb_path, query)
        }
    };

    // Store in cache.
    {
        let mut cache = get_kb_query_cache().lock().unwrap();
        cache.insert(query.to_string(), result.clone());
    }
    result
}

fn keyword_knowledge_search(kb_path: &Path, query: &str) -> Value {
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().filter(|w| w.len() > 2).collect();
    if keywords.is_empty() {
        return json!({ "message": "Query too short for keyword search." });
    }
    // FIX: Use recursive collector so sensor_examples/ subfolders and .c files are searched.
    let all_files = collect_kb_files(kb_path);
    let mut results: Vec<Value> = Vec::new();
    for (file_path, rel_key) in &all_files {
        if let Ok(content) = std::fs::read_to_string(file_path) {
            let content_lower = content.to_lowercase();
            let matched = keywords.iter().filter(|kw| content_lower.contains(*kw)).count();
            if matched == 0 { continue; }
            let score = matched as f32 / keywords.len() as f32;
            let lines: Vec<&str> = content.lines().collect();
            let mut relevant_sections: Vec<String> = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                let line_lower = line.to_lowercase();
                if keywords.iter().any(|kw| line_lower.contains(kw)) {
                    let start = i.saturating_sub(3);
                    let end = (i + 6).min(lines.len());
                    let section = lines[start..end].join("\n");
                    if !relevant_sections.iter().any(|s: &String| s.contains(&section) || section.contains(s.as_str())) {
                        relevant_sections.push(section);
                    }
                }
                if relevant_sections.len() >= 5 { break; }
            }
            let combined = if relevant_sections.is_empty() {
                content.chars().take(2000).collect()
            } else {
                relevant_sections.join("\n---\n")
            };
            results.push(json!({ "file": rel_key, "score": score, "content": combined, "method": "keyword" }));
        }
    }
    results.sort_by(|a, b| {
        b["score"].as_f64().unwrap_or(0.0).partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(5);
    if results.is_empty() {
        // Full-dump fallback: return first 3000 chars of every file in KB.
        let all: Vec<Value> = all_files.iter().filter_map(|(fp, rk)| {
            std::fs::read_to_string(fp).ok().map(|content| {
                json!({ "file": rk, "score": 0.1, "content": content.chars().take(3000).collect::<String>(), "method": "full_dump" })
            })
        }).collect();
        if all.is_empty() { json!({ "message": "No relevant local documents found." }) } else { json!(all) }
    } else {
        json!(results)
    }
}

async fn vector_knowledge_search(app_handle: &AppHandle, project_path: &Path, query: &str) -> Value {
    let _ = reindex_knowledge_base(project_path).await;
    let query_embedding = match get_embeddings(app_handle, query).await {
        Ok(e) => e,
        Err(_) => return json!([]),
    };
    let kb_path = project_path.join("knowledge_base");
    let index_file = kb_path.join(".embeddings.json");
    if !index_file.exists() { return json!([]); }
    let data = std::fs::read_to_string(&index_file).unwrap_or_default();
    let index: VectorIndex = serde_json::from_str(&data).unwrap_or_default();
    if index.chunks.is_empty() { return json!([]); }
    let mut matches: Vec<(f32, &KnowledgeChunk)> = index.chunks.iter()
        .map(|c| (cosine_similarity(&query_embedding, &c.embedding), c))
        .filter(|(s, _)| *s > 0.3)
        .collect();
    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let results: Vec<Value> = matches.iter().take(5).map(|(score, chunk)| {
        json!({ "file": chunk.file_name, "score": score, "content": chunk.content, "method": "vector" })
    }).collect();
    json!(results)
}