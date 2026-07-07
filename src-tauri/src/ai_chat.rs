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

/// Static HTTP client for cloud APIs (OpenRouter, OpenAI) — shared connection pool.
/// Timeout: 120s total. Recreating a Client per request causes socket exhaustion
/// and causes each subsequent request to take progressively longer.
static CLOUD_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
fn get_cloud_client() -> &'static Client {
    CLOUD_HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Static HTTP client for local LLM servers (Ollama, LM Studio).
/// Auto-decompression disabled — local SSE is plain text, not gzip/brotli.
/// Timeout: 600s to allow slow models to finish.
static LOCAL_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
fn get_local_client() -> &'static Client {
    LOCAL_HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(600))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .build()
            .unwrap_or_else(|_| Client::new())
    })
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

/// AppData/knowledge_base path — set by seed_knowledge_base() at startup using Tauri API.
/// Used by resolve_kb_path() as the definitive fallback for installed mode.
static APP_DATA_KB: OnceLock<PathBuf> = OnceLock::new();
pub fn get_app_data_kb() -> Option<&'static PathBuf> {
    APP_DATA_KB.get()
}

// ── Config helpers ────────────────────────────────────────────────────────────

fn config_path() -> PathBuf { config_dir().join("config.json") }

fn config_dir() -> PathBuf {
    // Windows: use APPDATA (e.g. C:\Users\<user>\AppData\Roaming)
    // Linux/macOS: use HOME
    // Fallback: use current directory (should never happen in practice)
    let base = std::env::var("APPDATA")
        .or_else(|_| std::env::var("USERPROFILE"))
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join(".vibekidbright")
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
    let proj_root = resolve_project_root(project_dir);
    let proj_kb = proj_root.join("knowledge_base");
    if proj_kb.exists() {
        return proj_kb;
    }
    
    // Fallback 1: global IDE knowledge_base (dev mode — CWD-based)
    let fallback_root = resolve_project_root("");
    let fallback_kb = fallback_root.join("knowledge_base");
    if fallback_kb.exists() {
        return fallback_kb;
    }

    // Fallback 2: AppData/knowledge_base set by Tauri API at startup (installed version)
    if let Some(appdata_kb) = get_app_data_kb() {
        if appdata_kb.exists() {
            return appdata_kb.clone();
        }
    }

    // Fallback 3: Manual APPDATA env var (legacy / safety net)
    if let Ok(app_data) = std::env::var("APPDATA").or_else(|_| std::env::var("HOME")) {
        for name in &["com.cake.tauri-app", "VibeKidbright IDE", "vibekidbright-ide"] {
            let appdata_kb = std::path::PathBuf::from(&app_data)
                .join(name)
                .join("knowledge_base");
            if appdata_kb.exists() {
                return appdata_kb;
            }
        }
    }
    
    proj_kb // Default if neither exists
}

/// Seed bundled knowledge_base files into AppData/knowledge_base (installed version).
/// Called on every startup — overwrites bundled files (installer always has latest KB),
/// but never overwrites user-added files (files not present in bundled KB).
pub fn seed_knowledge_base(app_handle: &AppHandle) {
    let Ok(app_data_dir) = app_handle.path().app_data_dir() else {
        eprintln!("[KB Seed] Cannot resolve app_data_dir");
        return;
    };
    let dst = app_data_dir.join("knowledge_base");

    // ── Build candidate list ───────────────────────────────────────────────────
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 0. Tauri v2 official API: resolve("knowledge_base", BaseDirectory::Resource)
    //    นี่คือ official way ที่ถูกต้องที่สุดสำหรับ NSIS/MSI installer
    if let Ok(resolved) = app_handle.path().resolve("knowledge_base", tauri::path::BaseDirectory::Resource) {
        eprintln!("[KB Seed] resolve(Resource) = {}", resolved.display());
        candidates.push(resolved);
    }

    // 1. Tauri v2: resource_dir() — ที่ Tauri วาง bundled resources ไว้
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        eprintln!("[KB Seed] resource_dir = {}", resource_dir.display());
        candidates.push(resource_dir.join("knowledge_base"));
        // 1b. resource_dir/_up_/knowledge_base (Tauri NSIS บางเวอร์ชัน)
        candidates.push(resource_dir.join("_up_").join("knowledge_base"));
        if let Some(parent) = resource_dir.parent() {
            candidates.push(parent.join("knowledge_base"));
        }
    }

    // 2. exe dir — Windows NSIS/MSI วาง resources ติดกับ .exe
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            eprintln!("[KB Seed] exe_dir = {}", exe_dir.display());
            candidates.push(exe_dir.join("knowledge_base"));
            candidates.push(exe_dir.join("resources").join("knowledge_base"));
            if let Some(parent) = exe_dir.parent() {
                candidates.push(parent.join("knowledge_base"));
                candidates.push(parent.join("resources").join("knowledge_base"));
            }
        }
    }

    // ── Try each candidate ────────────────────────────────────────────────────
    for src in &candidates {
        eprintln!("[KB Seed] trying: {}", src.display());
        if !src.exists() { continue; }

        // ตรวจว่ามีไฟล์ KB อย่างน้อย 1 ไฟล์
        let has_kb = std::fs::read_dir(src)
            .map(|mut d| d.any(|e| {
                e.map(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.ends_with(".md") || n.ends_with(".txt") || n.ends_with(".c") || n.ends_with(".h")
                }).unwrap_or(false)
            }))
            .unwrap_or(false);

        if has_kb {
            eprintln!("[KB Seed] found KB at: {}", src.display());
            let _ = std::fs::create_dir_all(&dst);
            // overwrite_bundled=true: ไฟล์ที่ bundled มาจะถูก overwrite เสมอ (อัพเดท KB ได้)
            copy_dir_overwrite(src, &dst, true);
            eprintln!("[KB Seed] seeded to: {}", dst.display());
            // Set global path เพื่อให้ resolve_kb_path ใช้ได้ทันที
            let _ = APP_DATA_KB.set(dst);
            return;
        }
    }

    eprintln!("[KB Seed] No bundled KB found in any candidate path — skipping (dev mode OK)");

    // ยังคง set global path เพื่อให้ resolve_kb_path รู้ว่า dst อยู่ที่ไหน (อาจเคย seed ไปแล้ว)
    let _ = APP_DATA_KB.set(dst);
}

/// Copy directory recursively.
/// - `overwrite_bundled`: ถ้า true จะ overwrite ไฟล์ที่มีอยู่แล้ว (installer updates)
/// - ข้าม .embeddings.json และ .backup เสมอ
fn copy_dir_overwrite(src: &std::path::Path, dst: &std::path::Path, overwrite_bundled: bool) {
    let Ok(entries) = std::fs::read_dir(src) else { return };
    for entry in entries.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            let _ = std::fs::create_dir_all(&dst_path);
            copy_dir_overwrite(&src_path, &dst_path, overwrite_bundled);
        } else {
            let name = entry.file_name().to_string_lossy().to_string();
            // ข้าม metadata files
            if name.ends_with(".backup") || name == ".embeddings.json" { continue; }
            // overwrite ถ้า flag เปิด หรือถ้าไฟล์ยังไม่มี
            if overwrite_bundled || !dst_path.exists() {
                let _ = std::fs::copy(&src_path, &dst_path);
            }
        }
    }
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
    let kb_path = resolve_kb_path(&project_dir);
    if !kb_path.exists() { return Vec::new(); }
    // Use collect_kb_files_all so .disabled files are included for UI display.
    // (collect_kb_files skips .disabled — that's for search/index only)
    let all_files = collect_kb_files_all(&kb_path);
    let mut files: Vec<String> = all_files
        .into_iter()
        .map(|(_, rel_key)| rel_key)
        .filter(|k| {
            k.ends_with(".txt")
                || k.ends_with(".md")
                || k.ends_with(".c")
                || k.ends_with(".h")
                || k.ends_with(".disabled")
        })
        .collect();
    files.sort();
    files
}

#[tauri::command]
pub fn open_knowledge_base_folder(project_dir: String) {
    let kb_path = resolve_kb_path(&project_dir);
    if !kb_path.exists() { let _ = std::fs::create_dir_all(&kb_path); }
    let _ = tauri_plugin_opener::open_path(kb_path.to_string_lossy().to_string(), None::<String>);
}

#[tauri::command]
pub async fn toggle_knowledge_base_file(project_dir: String, file_name: String) -> Result<bool, String> {
    let kb_path = resolve_kb_path(&project_dir);
    let target_file = kb_path.join(&file_name);
    if !target_file.exists() || !target_file.is_file() {
        return Err("File not found".to_string());
    }
    
    if file_name.ends_with(".disabled") {
        let new_name = file_name.replace(".disabled", "");
        let enabled_file = kb_path.join(new_name);
        std::fs::rename(&target_file, enabled_file).map_err(|e| e.to_string())?;
    } else {
        let disabled_file = kb_path.join(format!("{}.disabled", file_name));
        std::fs::rename(&target_file, disabled_file).map_err(|e| e.to_string())?;
    }
    get_kb_query_cache().lock().unwrap().clear(); // Invalidate cache
    Ok(true)
}

#[tauri::command]
pub async fn add_knowledge_base_files(project_dir: String) -> Result<usize, String> {
    use rfd::FileDialog;
    let paths = FileDialog::new()
        .set_title("Add Document to Knowledge Base")
        .add_filter("Documentation", &["txt", "md", "c", "h"])
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

    if !base_url.contains("/v1") {
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
    // "local" provider never requires an API key.
    if api_key.is_empty() && provider == "openai" {
        let is_local = base_url.contains("localhost")
            || base_url.contains("127.0.0.1")
            || base_url.starts_with("http://10.")
            || base_url.starts_with("http://192.168.")
            || base_url.split('/').nth(2)
                .map(|host| host.split(':').next().unwrap_or(""))
                .map(|host| host.split('.').count() == 4 && host.chars().all(|c| c.is_ascii_digit() || c == '.'))
                .unwrap_or(false);
        if !is_local {
            return Err("API key not set. Please configure your OpenAI API key.".to_string());
        }
    }

    // FIX: Convert message_id to Arc<str> before moving into spawn.
    // This is cheap to clone (atomic refcount) and works safely across await points.
    let message_id: Arc<str> = Arc::from(message_id.as_str());

    let mut project_path = resolve_project_root(&project_dir);
    // A workspace is "real" only if the path is non-trivial AND the directory exists
    // with at least one file in it. This prevents the model from calling
    // create_project_workspace when the user already has a project open.
    let mut no_workspace = project_dir == "." || project_dir.is_empty() || {
        // Also treat as no-workspace if the directory doesn't exist or is empty
        !project_path.exists() || std::fs::read_dir(&project_path)
            .map(|mut d| d.next().is_none())  // true if empty dir
            .unwrap_or(true)                   // true if can't read
    };

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

        // A model is free-tier ONLY if it explicitly ends with ":free" suffix
        // (OpenRouter convention). Google and OpenAI/paid models always go through
        // direct call — gemini-2.5-flash-lite and similar models can be charged
        // when the free quota is exceeded, so they must NOT use fallback routing.
        let is_free_tier = model.to_lowercase().ends_with(":free")
            || model == "free"
            || model == "openrouter/free"
            || model == "auto-free";

        if model == "free" || model == "openrouter/free" || model == "auto-free" {
            let best_free_models = vec![
                // ── Tier 1: Best reasoning + coding free models ───────────────
                "qwen/qwen3-coder:free",                    // 480B, best free coder
                "deepseek/deepseek-r1-0528:free",           // Latest DeepSeek R1 reasoning
                "deepseek/deepseek-r1:free",                // Strong reasoning
                "deepseek/deepseek-chat-v3-0324:free",      // V3 fast coding
                "microsoft/phi-4-reasoning:free",           // Phi-4 reasoning
                // ── Tier 2: Large capable free models ────────────────────────
                "openai/gpt-oss-120b:free",                 // GPT-class 120B
                "nvidia/nemotron-3-super-120b-a12b:free",   // top weekly
                "meta-llama/llama-4-maverick:free",         // Llama 4 Maverick 17Bx128E
                "meta-llama/llama-4-scout:free",            // Llama 4 Scout
                "meta-llama/llama-3.3-70b-instruct:free",   // Reliable fallback
                // ── Tier 3: Mid-size free models ──────────────────────────────
                "stepfun/step-3.5-flash:free",
                "google/gemma-4-31b-it:free",
                "google/gemma-3-27b-it:free",
                "arcee-ai/trinity-large-preview:free",
                "minimax/minimax-m2.5:free",
                "qwen/qwen3.6-plus-04-02:free",
                // ── Tier 4: Lightweight fallbacks ─────────────────────────────
                "mistralai/mistral-7b-instruct:free",
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
        } else if is_free_tier && provider != "local" {
            // Free-tier cloud models: try primary model first, then fallback to Google/OpenRouter.
            // NOTE: "local" provider is excluded — local models go straight to direct call below.
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
                    "qwen/qwen3-coder:free",
                    "deepseek/deepseek-r1-0528:free",
                    "deepseek/deepseek-r1:free",
                    "meta-llama/llama-4-maverick:free",
                    "nvidia/nemotron-3-super-120b-a12b:free",
                    "meta-llama/llama-3.3-70b-instruct:free",
                    "google/gemma-4-31b-it:free",
                    "mistralai/mistral-7b-instruct:free",
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
            // Paid models OR local provider: single direct call, no fallback.
            try_queue.push((
                provider.clone(), model.clone(), base_url.clone(), api_key.clone(),
                if provider == "local" {
                    format!("{} [LOCAL]", model)
                } else {
                    format!("{} [PAID]", model)
                },
            ));
        }

        let mut final_error = String::new();

        let mut dynamic_system_prompt = SYSTEM_PROMPT.to_string();
        let kb_path = resolve_kb_path(&project_dir);
        if kb_path.exists() {
            let all_files = collect_kb_files(&kb_path);
            let mut active_files = Vec::new();
            for (_, rel_key) in all_files {
                if rel_key.ends_with(".disabled") { continue; }
                if rel_key.ends_with(".txt") || rel_key.ends_with(".md") || rel_key.ends_with(".c") || rel_key.ends_with(".h") {
                    active_files.push(rel_key);
                }
            }
            if !active_files.is_empty() {
                dynamic_system_prompt.push_str("\n\n[USER KNOWLEDGE BASE INSTRUCTIONS]\nThe user has provided project-specific knowledge base files. You MUST use the `knowledge_search` tool to query information from them. The currently active files are:\n");
                for f in active_files {
                    dynamic_system_prompt.push_str(&format!("- {}\n", f));
                }
            }
        }

        // Only inject hardware rules if the file is NOT disabled by the user.
        let hw_file = kb_path.join("formula_kid_controller.md");
        let hw_file_disabled = kb_path.join("formula_kid_controller.md.disabled");
        let hardware_rules = if hw_file.exists() && !hw_file_disabled.exists() {
            std::fs::read_to_string(&hw_file).unwrap_or_default()
        } else {
            String::new()
        };
        if !hardware_rules.is_empty() {
            dynamic_system_prompt.push_str(&format!(
                "\n\n## MANDATORY HARDWARE RULES (always apply)\n{}\n",
                hardware_rules
            ));
        }

        dynamic_system_prompt.push_str("\n\n[CRITICAL GUARDRAIL]\nห้าม assume หรือ fallback ไปใช้บอร์ดรุ่นอื่น (เช่น KidBright32 รุ่นเก่า) โดยเด็ดขาด หากข้อมูลฮาร์ดแวร์ไม่ชัดเจน หรือไม่ตรงกับที่ระบุใน Knowledge Base ให้ถามผู้ใช้ก่อนเสมอ\n");

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
                    &dynamic_system_prompt,
                ).await
            } else {
                run_conversation_loop(
                    &app_handle, &key, &mod_name, &url, messages.clone(),
                    &mut project_path, current_is_openrouter, Arc::clone(&message_id), &mut no_workspace,
                    &dynamic_system_prompt,
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

### COPY FIDELITY RULE (MANDATORY):
When the user provides an existing file as reference or says "copy this" / "make it match this":
- Output MUST be byte-for-byte identical in logic, variable names, function names, and struct field presence.
- NEVER "fix" bugs, rename variables, add fields, or improve logic unless explicitly asked. The reference file IS the ground truth.
- If you see code that looks wrong (e.g. motor_forward sets all duty=0), copy it exactly and add a comment: `// NOTE: matches reference file`. Do NOT silently correct it.
- Changing `motor_set()` to `motor_set_duty()` = VIOLATION.
- Changing turn range logic = VIOLATION.

### KNOWLEDGE BASE WORKFLOW (CRITICAL):
When asked to write hardware-specific code (GPIO, motors, ESP-NOW, board config, sensors):

**Call `knowledge_search` or `read_knowledge_file` ONLY IF:**
- The exact GPIO pins, I2C address, or hardware spec for this board is NOT already confirmed in this conversation.
- The task involves a complex component (OLED, LED matrix, ESP-NOW protocol, motor driver, accelerometer).
- You are uncertain or the user hasn't specified the board revision yet.

**You MAY skip `knowledge_search` and write directly IF:**
- The board revision AND relevant GPIO pins were already established earlier in this conversation.
- The task is simple (e.g., GPIO button + buzzer where the user just confirmed the board is iA).
- You already called `knowledge_search` or `read_knowledge_file` for this board in this session.

2. DO NOT call `create_project_workspace` before reading the KB if the task involves hardware code.
3. If the search result is empty or truncated → call `read_knowledge_file` with the exact filename.
4. If the KB has no relevant info → say so explicitly, then ask the user before proceeding with general knowledge.

HARD RULE: "KB not loaded" or empty search result is NOT permission to generate from memory. It is a signal to call `read_knowledge_file` instead.

### ⚠️ CRITICAL: CODE MUST GO TO FILES — NEVER IN CHAT (NON-NEGOTIABLE)
- **When creating a project or asked to write firmware**: You MUST call `write_file` tool for EVERY file. **NEVER paste the full code as a chat message.** Showing code in chat instead of writing it to a file is a VIOLATION.
- **When asked to EDIT or MODIFY an existing file** (e.g., "change MAC address", "fix the formula", "update GPIO pin"): You MUST:
  1. Call `read_file` to get the current content.
  2. Apply the change mentally.
  3. Call `write_file` with the **complete modified content**. ← THIS IS MANDATORY.
  4. NEVER just say "ฉันได้อัปเดตไฟล์" without actually calling `write_file`. Saying you changed it without calling the tool = the file is NOT changed.
- **BANNED BEHAVIOR:**
  - Responding with a wall of code in the chat bubble when `write_file` should be used.
  - Calling `read_file` and then ONLY describing the change in chat without calling `write_file`.
  - Saying "โปรดตรวจสอบ diff และคลิก Keep" without having called `write_file` — the file is unchanged.
- **The ONLY exception** is when the user explicitly asks "show me the code" or "explain this snippet" — in that case, use a markdown code fence with `[FILE: main/main.c]` header.
- **After using `write_file`**: Briefly describe WHAT was changed. Do not re-paste the code content.

### CRITICAL RULE: NO ARDUINO CODE
- You MUST write raw ESP-IDF C code (using FreeRTOS, `driver/gpio.h`, `driver/i2c.h`, `driver/ledc.h` etc.).
- ALWAYS include `<stdio.h>`, `"freertos/FreeRTOS.h"`, and `"freertos/task.h"` if you use `vTaskDelay` or other FreeRTOS functions.
- NEVER generate Arduino code (`#include <Wire.h>`, `setup()`, `loop()`, `tone()`, etc.).
- Even if the Knowledge Base shows Arduino examples, you MUST translate them to pure ESP-IDF API before showing the user.
- **BANNED: Multiple `if` on one line** — ESP-IDF GCC 14.2 has `-Werror=misleading-indentation`. NEVER write `if (a < 0) a = 0; if (a > 9) a = 9;` on a single line. Each `if` statement MUST be on its own line:
  ```c
  // ❌ BANNED (causes -Werror=misleading-indentation)
  if (x < 0) x = 0; if (x > 9) x = 9;
  // ✅ CORRECT
  if (x < 0) x = 0;
  if (x > 9) x = 9;
  ```

### ESP-NOW RULES (MANDATORY):
- `espnow_recv_cb()` runs in the WiFi task context, NOT a hardware ISR. You MUST use `xQueueSend()` with `timeout=0`. NEVER use `xQueueSendFromISR()`.
- Absolutely NO blocking calls are allowed inside `espnow_recv_cb()` (never use `portMAX_DELAY`).
- ALWAYS create the FreeRTOS queue BEFORE calling `esp_now_init()` and `esp_now_register_recv_cb()`.
- Send/receive raw `int32_t` directly. Do NOT wrap it in a `struct` to guarantee size compatibility with the controller side.
- Never guard `esp_now_init()` with `#if CONFIG_*` macros unless that config is explicitly defined in the project. Default: call `esp_now_init()` unconditionally.
- NEVER use `esp_wifi_config_espnow_rate()` — deprecated and broken in v5.5+
- NEVER use `esp_wifi_set_storage()` — deprecated in v5.x
- NEVER use `ESP_NOW_WIFI_RATE_1M` — undefined in v5.5+
- ESP-NOW channel MUST match controller (default: channel 1)
- **BOTH sender and receiver MUST call `esp_wifi_disconnect()` after `esp_wifi_start()` to prevent connecting to any AP.**
- Minimal correct wifi_init for ESP-NOW (sender & receiver):
  `esp_netif_init() -> esp_event_loop_create_default() -> esp_wifi_init() -> esp_wifi_set_mode(WIFI_MODE_STA) -> esp_wifi_start() -> esp_wifi_disconnect() -> esp_now_init() -> esp_now_register_*_cb()`
- **Minibike Sender** prints its own MAC on boot: `esp_wifi_get_mac(WIFI_IF_STA, mac)` then `ESP_LOGI`.
- **Minibike Sender send callback signature (ESP-IDF v5.x):**
  ```c
  static void on_sent(const uint8_t *mac_addr, esp_now_send_status_t status) {
      (void)mac_addr;
      s_espnow_ready = (status == ESP_NOW_SEND_SUCCESS);
  }
  ```
  Note: `s_espnow_ready` is `volatile bool` updated from the callback and displayed on OLED.
- **Minibike Receiver receive callback signature (ESP-IDF v5.x):**
  ```c
  static void on_recv(const esp_now_recv_info_t *info, const uint8_t *data, int len) {
      if (len == sizeof(int32_t)) memcpy((void *)&g_cmd, data, sizeof(int32_t));
  }
  ```

### SENSOR RULES (MANDATORY):
- **Temperature Sensor**: The ESP32 chip does NOT have an on-chip temperature sensor usable in this context. You MUST NEVER use `esp_driver_tsens` or `temperature_sensor_install()`. Instead, use the on-board **LM73-compatible** I2C sensor via `I2C_NUM_1` (SDA=GPIO4, SCL=GPIO5, Address=0x4D).
- **LM73 Read Protocol**: Send pointer register `0x00`, then read 2 bytes (MSB first) using `i2c_master_write_read_device()`.
- **MANDATORY FORMULA for ALL KidBright boards (V1.3, 32i, 32iA, 32iP, iA):**
  ```c
  int16_t raw_temp = (int16_t)(((uint16_t)raw[0] << 8) | (uint16_t)raw[1]);
  float temperature = (float)raw_temp / 128.0f;
  // Example: raw_temp=3712 → 3712/128 = 29.0°C ✓
  ```
- **❌ BANNED FORMULA (DO NOT USE — causes ~5°C or ~3.75°C instead of ~29°C):**
  ```c
  // BANNED: (raw_temp >> 5) / 32.0f  ← gives 1/8 of real temperature on all KidBright boards
  // BANNED: (raw_temp >> 2) / 128.0f ← wrong shift for this sensor
  // BANNED: (int16_t)((raw[0] << 8) | raw[1]) ← undefined behavior (uint8_t shift)
  ```
- **Why `/128.0f`:** Hardware-verified on V1.3, KidBright32i, and KidBright32iA. The on-board sensor returns data in right-justified format where 1 LSB = 1/128°C (0.0078125°C). Raw value 3712 = 29.0°C, 3584 = 28.0°C.
- ALWAYS use `i2c_master_write_read_device()` (combined transaction). NEVER split into separate write + read calls.

### ADC RULES (MANDATORY — ESP-IDF v5.x):
#### ✅ Correct Oneshot API (DEFAULT for all new code):
```c
#include "esp_adc/adc_oneshot.h"     // ✅
#include "esp_adc/adc_cali.h"        // ✅ (only if calibration needed)
#include "esp_adc/adc_cali_scheme.h" // ✅ (only if calibration needed)

adc_oneshot_new_unit(...)            // 1. Create unit
adc_oneshot_config_channel(...)      // 2. Config (use ADC_ATTEN_DB_12 ALWAYS)
adc_oneshot_read(...)                // 3. Read raw
adc_cali_raw_to_voltage(...)         // 4. Optional: convert to mV
```
> NEVER include `adc_cali.h` when only reading LDR raw values — calibration is optional.
> **`ADC_ATTEN_DB_11` — DEPRECATED in oneshot API. ALWAYS use `ADC_ATTEN_DB_12` for oneshot.**

#### ⚠️ Legacy ADC API — EXCEPTION for Minibike Sender only:
The file `minibike_sender.c` uses Legacy ADC API intentionally (simple joystick reading, no calibration needed):
```c
// ✅ ALLOWED in minibike_sender.c ONLY
#include "driver/adc.h"
#include "esp_adc_cal.h"
adc1_config_width(ADC_WIDTH_BIT_12);
adc1_config_channel_atten(ADC1_CHANNEL_6, ADC_ATTEN_DB_11);  // GPIO34
adc1_config_channel_atten(ADC1_CHANNEL_7, ADC_ATTEN_DB_11);  // GPIO35
int raw = adc1_get_raw(ADC1_CHANNEL_6);
```
- Channel macros: `ADC1_CHANNEL_6` (GPIO34), `ADC1_CHANNEL_7` (GPIO35)
- For ALL other projects: use `esp_adc/adc_oneshot.h` instead.

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
- **MANDATORY Edit Workflow & Context Persistence:** 
  1) **Existing Project Default:** ถ้าผู้ใช้สั่ง "แก้ไข" หรือเขียนโค้ดเพิ่มเติม โดยไม่ได้ใช้คำว่า "สร้างโปรเจกต์ใหม่" อย่างชัดเจน คุณ **MUST** แก้ไขไฟล์หลักจากโปรเจกต์เดิมที่ทำงานอยู่เสมอ (ใช้ `read_file` ตรวจสอบก่อน) **ห้ามสร้างโปรเจกต์ใหม่หรือย้ายไปทำไฟล์ใหม่แยกต่างหากโดยพลการ**
  2) **อ่านและตรวจสอบ (READ):** เรียกใช้เครื่องมือ `read_file` ทุกครั้งเพื่อประเมินโค้ดเก่าก่อนแก้ไข ห้ามเดาเอาเอง
  3) **เรียกใช้เครื่องมือ (EXECUTE TOOL):** **CRITICAL: ทุกครั้งที่ผู้ใช้สั่งแก้โค้ด คุณ MUST ตอบสนองพร้อมกับเรียกใช้เครื่องมือเพื่อแก้ไขไฟล์ (เช่น `replace_file_content` หรือ `write_file`) ทันที!** ห้ามพิมพ์รับปากลอยๆ ว่า "ได้ครับเดี๋ยวผมจัดการให้" แล้วไม่เรียก Tool เด็ดขาด! (ถ้าไม่ชาร์จ Tool การแก้ไขจะไม่เกิดขึ้นจริง)
  4) **รายงานสรุป (NOTIFY):** พิมพ์ตอบในแชทสั้นๆ ทุกครั้งหลังเรียก Tool สำเร็จว่าเสนอการแปลี่ยนแปลงให้แล้ว
- When calling a tool, do not explain what you are doing first. Just call the tool.

### BOARD DETECTION — MANDATORY FIRST STEP:
Before writing ANY code that involves GPIO, I2C, buttons, or sensors, you MUST know which board revision the user has.
- **If the user has NOT mentioned the board revision in the current conversation**, you MUST ask EXACTLY this question (in Thai) before proceeding:
  > "บอร์ดของคุณเป็นรุ่นไหนครับ? (โปรดระบุ: V1.1 / V1.2 / V1.3 / V1.4 / Rev 3.1 / Rev 3.1G / iA / KidBright32i / KidBright32iA / V1.6 / KidBright32iP / μAI / KidBright Controller V1 / Formula Kid Controller)"
- **NEVER tell the user their board revision does not exist.** All of the above revisions are VALID. If the user says "V1.3", "1.3", "v 1.3", etc., treat it as the V1.3 board (FTDI USB, GPIO layout same as V1.1/V1.2, SW2=GPIO14).
- **If user says "Controller V1" / "KidBright Controller" / "Minibike Sender"** → treat as **KidBright Controller V1** (ESP32 bare module, ADC joystick, OLED SH1106, ESP-NOW sender).
- **If user says "Formula Kid Controller" / "KB1.3 Controller" / "KB1.5G Controller"** → treat as **Formula Kid Controller** (KidBright32 V1.5 Rev3.1/3.1G + Formula Kid rev 1.1, RC timing joystick GPIO26/27/32/33, S1=GPIO36, S2=GPIO39).
- **Do NOT assume `iA` as default** if the user hasn't specified. Wait for the answer before generating hardware-specific code.
- **Once the user confirms the revision**, lock that revision for the entire session. Do not ask again.
- **Exception:** If the code only uses peripherals identical across ALL revisions (e.g., LM73 on I2C_1, buzzer on GPIO13, LED matrix on 0x70), you MAY proceed without asking — but add a comment: `// NOTE: GPIO config below assumes [REVISION]. Verify your board.`

### BOARD HARDWARE REVISIONS (MANDATORY READING):
- **KidBright Controller V1** (ESP32 bare module — Minibike Sender):
  - **ไม่มี HT16K33 Matrix, ไม่มี Buzzer, ไม่มี LM73** — เป็น bare ESP32 module
  - Joystick X: GPIO34 (ADC1_CHANNEL_6, input-only), Joystick Y: GPIO35 (ADC1_CHANNEL_7, input-only)
  - SW1 = GPIO16, SW2 = GPIO14 — Active LOW, GPIO_PULLUP_ENABLE
  - OLED SH1106: I2C_NUM_0 (SDA=GPIO21, SCL=GPIO22), I2C Address 0x3C
  - ADC API: Legacy (`driver/adc.h`) — `adc1_get_raw()`, `ADC_ATTEN_DB_11`
  - ESP-NOW: Sender, channel 1, WiFi STA, `esp_wifi_disconnect()` after start
  - CMakeLists: `PRIV_REQUIRES driver esp_wifi nvs_flash esp_event esp_netif`
  - **ห้ามใช้ `esp_adc/adc_oneshot.h`** บนบอร์ดนี้ — ใช้ legacy API เท่านั้น
  - **ห้ามใช้ SSD1306 init sequence** กับ OLED — ต้องใช้ SH1106 (`0x8D,0x14` charge pump)
  - ดูไฟล์กฎ: `minibike.md`
- **Formula Kid Controller (KB1.3 / KB1.5G)** (KidBright32 V1.5 Rev3.1/3.1G + extension):
  - MCU board: KidBright32 V1.5 Rev 3.1 (KB1.3) หรือ Rev 3.1G (KB1.5G)
  - Joystick: **RC Timing** (ไม่ใช่ ADC!) — JS1 Y: TRIG=GPIO26(OUT1)/CAP=GPIO32(IN1), JS2 X: TRIG=GPIO27(OUT2)/CAP=GPIO33(IN2)
  - S1 button = GPIO36 (input-only, external pull-up, NO pull-up in code, NO interrupt with ESP-NOW)
  - S2 button = GPIO39 (input-only, external pull-up, NO pull-up in code, NO interrupt with ESP-NOW)
  - SW1 ปุ่มบนบอร์ด = GPIO16, SW2 ปุ่มบนบอร์ด = GPIO14 (คนละวงจรกับ S1/S2)
  - LED Matrix: HT16K33 @ I2C_NUM_0 (SDA=21, SCL=22, addr 0x70) — **Matrix หมุน 180°**
  - **ไม่มี KXTJ3 Accelerometer** บนทั้ง KB1.3 และ KB1.5G
  - RC Timing constants: R_SERIE=1000Ω, RC_FACTOR_5V=9.788075945, CAP_TIMEOUT_US=500000
  - ESP-NOW encoding: JS1→ -100…+100, JS2→ +400 offset, 999=stop (Priority JS1>JS2>stop)
  - Dead zone: JS1=±10, JS2=±20
  - CMakeLists: `PRIV_REQUIRES driver esp_timer esp_wifi nvs_flash`
  - **ห้ามใช้ `GPIO_PULLUP_ENABLE` บน GPIO36/39**
  - **ห้ามใช้ interrupt บน GPIO36/39 เมื่อใช้ ESP-NOW** — ใช้ polling เท่านั้น
  - ดูไฟล์กฎ: `formula_kid_controller.md`
- **V1.1 / V1.2 (Cypress USB, ESP32)** (2018):
  - SW1 = GPIO16, SW2 = GPIO14 ← CRITICAL
  - LED WiFi=GPIO2, LED NTP=GPIO5(shared I2C SCL), LED IoT=GPIO12, LED BT=GPIO23
  - I2C_NUM_1: SDA=**GPIO4** (dedicated, NOT shared), SCL=GPIO5
  - Sensors: LDR(GPIO36), LM73(0x4D)+RTC(0x6F) on I2C_1, HT16K33 Matrix(0x70) on I2C_0. **NO Accelerometer.**
  - ADC on IN1–IN4: ❌ NOT supported (digital only)
- **V1.3 (FTDI USB, ESP32)** (2019):
  - SW1 = GPIO16, SW2 = GPIO14 ← CRITICAL (same as V1.1/V1.2)
  - GPIO layout **identical to V1.1/V1.2**. Only USB bridge chip changed (FTDI FT232RL instead of Cypress).
  - I2C_NUM_1: SDA=**GPIO4** (dedicated, NOT shared with LED), SCL=GPIO5. **GPIO4 is safe for I2C on V1.3.**
  - LED BT=GPIO23 (same as V1.1/V1.2 — NOT GPIO4, which is V1.4 only)
  - Sensors: LDR(GPIO36), Temp sensor(0x4D)+RTC(0x6F) on I2C_1, HT16K33 Matrix(0x70) on I2C_0. **NO Accelerometer.**
  - **CRITICAL V1.3 TEMPERATURE FORMULA:** The temperature sensor on V1.3 sends data in **right-justified format** where 1 LSB = 1/128°C. Use: `float temperature = (float)raw_temp / 128.0f;` — NOT the LM73 11-bit formula `(raw>>5)/32.0f`. Debug: raw[0]=0x0E,raw[1]=0x80 → 3712/128 = 29.0°C.
  - ADC on IN1–IN4: ❌ NOT supported (digital only)
- **V1.4 (FTDI USB, LED ลดเหลือ 2 ดวง)** (2019–2020):
  - SW1 = GPIO16, SW2 = GPIO14 ← CRITICAL
  - GPIO4 now shared: LED BT + I2C_NUM_1 SDA (⚠️ shared — pick one)
  - Sensors: LDR(GPIO36), LM73(0x4D)+RTC(0x6F) on I2C_1, HT16K33 Matrix(0x70) on I2C_0. **NO Accelerometer.**
  - ADC on IN1–IN4: ❌ NOT supported (digital only)
- **V1.5 Rev 3.1 (NECTEC Standard)** (Without 'G'):
  - SW1 = GPIO16, **SW2 = GPIO14** ← CRITICAL
  - Sensors: Matrix(0x70) on I2C_0 **(NO KXTJ3)**. LM73(0x4D) + RTC_MCP794xx(0x6F) on I2C_1.
  - ADC on IN1–IN4: ❌ NOT supported (digital only)
- **V1.5 Rev 3.1G (Gravitech OEM)** (With 'G'):
  - SW1 = GPIO16, **SW2 = GPIO14** ← CRITICAL
  - Sensors: Matrix(0x70) on I2C_0 **(NO KXTJ3)**. LM73(0x4D) + RTC_MCP794xx(0x6F) on I2C_1.
  - ADC on IN1–IN4: ❌ NOT supported (digital only)
- **V1.5 iA (INEX)**:
  - SW1 = GPIO16, **SW2 = GPIO14** ← CRITICAL (same as Rev 3.1/3.1G)
  - Sensors: Matrix(0x70) + **KXTJ3 Accelerometer(0x0E)** on I2C_0. LM73(0x4D) + **RTC MCP794xx(0x6F)** on I2C_1.
  - ADC works on IN1(GPIO32) + IN2(GPIO33) + IN3(GPIO34) + IN4(GPIO35) + LDR(GPIO36). **LDR (not Phototransistor).**
- **KidBright32i (INEX บอร์ดสีเขียว)**:
  - SW1 = GPIO16, **SW2 = GPIO14** ← CRITICAL (same as V1.5 Rev 3.1/3.1G)
  - Sensors: Matrix(0x70) on I2C_0 **(NO KXTJ3)**. LM73(0x4D) + RTC(0x6F) on I2C_1.
  - **GPIO36 = Phototransistor** (NOT LDR — different circuit from V1.5 series)
  - ADC works on IN1(GPIO32)+IN2(GPIO33)+IN3(GPIO34)+IN4(GPIO35)+Phototransistor(GPIO36).
  - Extra breakout: GPIO18, GPIO19, GPIO23, VN(GPIO39). 3.3V Regulator from USB.
  - **CRITICAL TEMPERATURE FORMULA (32i):** Same right-justified format as V1.3. Use `(float)raw_temp / 128.0f`. Do NOT use `(raw_temp >> 5) / 32.0f` — gives ~1/8 of real temperature.
- **KidBright32iA (INEX)**:
  - SW1 = GPIO16, **SW2 = GPIO14** ← CRITICAL (same as 32i)
  - Same as KidBright32i but adds **KXTJ3 Accelerometer(0x0E)** on I2C_0.
  - **GPIO36 = Phototransistor** (NOT LDR)
  - **CRITICAL TEMPERATURE FORMULA (32iA):** Same right-justified format. Use `(float)raw_temp / 128.0f`. Do NOT use `(raw_temp >> 5) / 32.0f`.
- **V1.6 (Gravitech)**:
  - SW1 = GPIO16 (shared with SERVO1 — เลือกได้แค่อย่างเดียว), **SW2 = GPIO17 (shared SERVO2)**.
  - Sensors: Matrix(0x70) + **MPU-6050 Accel+Gyro(0x68)** on I2C_0. LM73(0x4D) + **RTC MCP794xx(0x6F)** on I2C_1.
  - ADC works on IN1–IN4 + LDR(GPIO36). Has RGB LED ×6 (WS2812B via RMT).
- **KidBright32iP (INEX บอร์ดสีชมพู)**:
  - SW1 = GPIO16, **SW2 = GPIO14** ← CRITICAL (same as 32i/32iA)
  - Same GPIO as KidBright32i but adds SERVO1(GPIO15)/SERVO2(GPIO17). **NO Accelerometer.**
  - **GPIO36 = Phototransistor** (improved version, more linear than 32i)
- **CRITICAL RULE**:
  - "V1.1", "V1.2", "V1.3", "V1.4" all use SW2=GPIO14.
  - "3.1" and "3.1G" use SW2=GPIO14.
  - **V1.5 iA** uses **SW2=GPIO14** (corrected — same as Rev 3.1/3.1G).
  - **KidBright32i, KidBright32iA, KidBright32iP** all use **SW2=GPIO14**.
  - **V1.6** uses SW2=GPIO17 (shared SERVO2 only — this is the exception).
  - NEVER reject a board revision the user claims. ALWAYS look it up in the knowledge base.

### I2C RULES (MANDATORY):
- **Use legacy API ONLY:** `#include "driver/i2c.h"` and `i2c_master_write_to_device`. NEVER use `driver/i2c_master.h`.
- **`i2c_driver_install()` is called ONCE per port number.** Calling it twice on the same port returns `ESP_ERR_INVALID_STATE`. If the driver is already installed, skip the install step.
- **MANDATORY Init Order** when using multiple I2C devices (always init bus0 before bus1):
  1. `i2c_init_bus0()` → `I2C_NUM_0` (SDA=21, SCL=22):
     - **iA / KidBright32iA:** LED Matrix (0x70) + KXTJ3 accelerometer (0x0E)
     - **V1.6:** LED Matrix (0x70) + MPU-6050 (0x68)
     - **Rev 3.1 / Rev 3.1G / KidBright32i / KidBright32iP:** LED Matrix (0x70) ONLY — **NO KXTJ3, NO MPU-6050**
  2. `i2c_init_bus1()` → `I2C_NUM_1` (SDA=4, SCL=5): **ALL revisions**:
     - LM73 temperature (0x4D) + RTC MCP794xx (0x6F) — applies to Rev 3.1, Rev 3.1G, iA, and V1.6
- **Shared Bus Rule:** External I2C devices (e.g., BME280, LCD) share `I2C_NUM_0` with the LED Matrix. DO NOT reinstall the I2C driver if it's already initialized.
- **DO NOT** use `ESP_ERROR_CHECK()` for `i2c_master_cmd_begin` or any I2C read/write. Handle errors gracefully with `if (ret != ESP_OK)`.
- **I2C Timeout/ESP_FAIL:** Remind the user to check physical pull-up resistors, power supply, and correct pins (SDA=21, SCL=22 for bus0; SDA=4, SCL=5 for bus1).

### MANDATORY LED MATRIX CODE TEMPLATE (KIDBRIGHT32 iA):
- สำหรับบอร์ด KidBright32 iA หน้าจอ LED Matrix 16x8 ใช้ชิป HT16K33 **เพียงตัวเดียวที่ Address `0x70`**
- **Init commands (ส่งไปแค่ 0x70):** `0x21` (Oscillator ON), `0x81` (Display ON), `0xEF` (Brightness MAX)

#### ⚠️ HARDWARE MAPPING — INTERLEAVED FORMAT (CRITICAL):
The HT16K33 on KidBright32 iA uses an **interleaved, counter-clockwise 90° rotated** memory layout.
- **Left Screen (Cols 0-7):** mapped to **Even indexes** of the 16-byte array (index 0,2,4,6,8,10,12,14)
- **Right Screen (Cols 8-15):** mapped to **Odd indexes** (index 1,3,5,7,9,11,13,15)
- **Y-axis:** Bit 0 (0x01) = Top Row, Bit 7 (0x80) = Bottom Row

**PROHIBITION — NEVER create `uint8_t img[16]` arrays manually using linear left-to-right logic.**
Doing so causes the "two arrows pointing outward" visual bug. ALWAYS use one of these two methods:

**Method 1 — Computed (use `rows_to_columns_16x8()`):**
```c
static void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                out_cols[col] |= (1 << (7 - row));
            }
        }
    }
}
// Example: Heart pattern
static const uint16_t PATTERN_HEART[8] = {
    0x0000, 0x0660, 0x0FF0, 0x1FF8, 0x0FF0, 0x07E0, 0x03C0, 0x0180
};
```

**Method 2 — Pre-calculated arrays (use these directly for common shapes):**
```c
// จุดกึ่งกลาง (4x4 square at center seam)
static const uint8_t img_center[16] = {0x00,0x18,0x00,0x18,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x18,0x00,0x18,0x00};
// ลูกศรชี้ขึ้น
static const uint8_t img_up[16]     = {0x00,0xFF,0x00,0xFE,0x00,0x0C,0x00,0x08,0x08,0x00,0x0C,0x00,0xFE,0x00,0xFF,0x00};
// ลูกศรชี้ลง
static const uint8_t img_down[16]   = {0x00,0xFF,0x00,0x7F,0x00,0x30,0x00,0x10,0x10,0x00,0x30,0x00,0x7F,0x00,0xFF,0x00};
// ลูกศรชี้ซ้าย
static const uint8_t img_left[16]   = {0x00,0x18,0x00,0x18,0x18,0x18,0x3C,0x18,0x7E,0x18,0xFF,0x18,0x18,0x00,0x18,0x00};
// ลูกศรชี้ขวา
static const uint8_t img_right[16]  = {0x00,0x18,0x00,0x18,0x18,0xFF,0x18,0x7E,0x18,0x3C,0x18,0x18,0x18,0x00,0x18,0x00};
```

**`matrix_draw()` — ALWAYS use this to send to hardware:**
```c
static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00; // register pointer
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];       // Even index → Left screen col
        buf[2 + (c * 2)] = cols[c + 8];   // Odd index  → Right screen col
    }
    i2c_master_write_to_device(I2C_NUM_0, 0x70, buf, sizeof(buf), pdMS_TO_TICKS(100));
}
```

**TWO-DIGIT DISPLAY REQUIREMENT — MANDATORY (READ CAREFULLY!):**
- **CRITICAL ANTI-PATTERN:** The hardware has an INVERTED Y-axis. If you invent your own 5x7 fonts, the numbers will be **UPSIDE DOWN**. You MUST use EXACTLY these 10 arrays and logic:
```c
static const uint16_t DIGIT_0[8] = {0x0E00,0x1100,0x1100,0x1100,0x1100,0x1100,0x1100,0x0E00};
static const uint16_t DIGIT_1[8] = {0x0200,0x0600,0x0A00,0x0200,0x0200,0x0200,0x0200,0x1F00};
static const uint16_t DIGIT_2[8] = {0x0E00,0x1100,0x0100,0x0200,0x0400,0x0800,0x1000,0x1F00};
static const uint16_t DIGIT_3[8] = {0x0E00,0x1100,0x0100,0x0600,0x0100,0x0100,0x1100,0x0E00};
static const uint16_t DIGIT_4[8] = {0x0200,0x0600,0x0A00,0x1200,0x1F00,0x0200,0x0200,0x0200};
static const uint16_t DIGIT_5[8] = {0x1F00,0x1000,0x1E00,0x0100,0x0100,0x0100,0x1100,0x0E00};
static const uint16_t DIGIT_6[8] = {0x0E00,0x1100,0x1000,0x1E00,0x1100,0x1100,0x1100,0x0E00};
static const uint16_t DIGIT_7[8] = {0x1F00,0x0100,0x0200,0x0400,0x0400,0x0400,0x0400,0x0400};
static const uint16_t DIGIT_8[8] = {0x0E00,0x1100,0x1100,0x0E00,0x1100,0x1100,0x1100,0x0E00};
static const uint16_t DIGIT_9[8] = {0x0E00,0x1100,0x1100,0x0F00,0x0100,0x0100,0x1100,0x0E00};
static const uint16_t *DIGITS[10] = {DIGIT_0, DIGIT_1, DIGIT_2, DIGIT_3, DIGIT_4, DIGIT_5, DIGIT_6, DIGIT_7, DIGIT_8, DIGIT_9};
// REQUIRED function structure for 2 digits:
static void display_two_digits(int tens, int units) {
    uint16_t comb[8];
    for (int i = 0; i < 8; i++) comb[i] = DIGITS[tens][i] | (DIGITS[units][i] >> 8);
    uint8_t cols[16]; rows_to_columns_16x8(comb, cols); matrix_draw(cols);
}
```

**THREE-DIGIT DISPLAY EXCEPTION:**
When displaying values that require **3 digits** (e.g., percentages 0-100, ADC raw values 0-999), `display_two_digits()` is insufficient. In this case, you MAY create a custom **4x7 font** and `draw_char()` function instead. Rules:
- Use `col_offset` positions: `0` (hundreds), `5` (tens), `10` (units) — each digit is 4 cols wide + 1 col gap.
- Format the value first: `char buf[4]; snprintf(buf, sizeof(buf), "%03d", value);`
- Draw each character with `draw_char(buf[i] - '0', col_offset, cols)`.
- The custom font MUST be designed for the **INVERTED Y-axis** of this hardware (bit 0 = top row). Test against known values before use.
- **Do NOT use this exception for 1-digit or 2-digit values** — always use `display_two_digits()` for those.

### ZERO-HALLUCINATION & STRICT DECLARATION RULE (CRITICAL):
1. **Never Invent Variables:** You are FORBIDDEN from inventing variable names, macros, or functions (e.g., guessing musical notes like `NOTE_P4` which do not exist).
2. **Prove It Before Use:** Before using ANY variable, macro, or function, you MUST verify it exists in the current file using `read_file` or standard ESP-IDF documentation.
3. **Exact Matching:** If the user asks to modify a string or array, strictly modify ONLY the values requested. Do not alter the surrounding architecture unless explicitly asked.
4. **C Syntax Restrictions:**
   - NEVER use empty struct initialization like `gpio_config_t conf = {};` (use `{0}` instead).
   - If using `gpio_install_isr_service()`, you MUST define `#define ESP_INTR_FLAG_DEFAULT 0` at the top of your code.
   - **ISR CRITICAL RULE:** NEVER put `ESP_LOGI`, `printf`, or complex blocking logic inside an `IRAM_ATTR` ISR! This causes an immediate panic/crash on ESP32. You MUST use `xQueueSendFromISR` and handle the logic inside a FreeRTOS task.
   - **FreeRTOS Types v5.x:** NEVER use legacy types like `xQueueHandle` or `xTaskHandle` (they are removed). You MUST use `QueueHandle_t` and `TaskHandle_t`.
   - **Mandatory Includes:** Always `#include "freertos/queue.h"` if your code uses queues.
   - **IRAM_ATTR Forward Declaration Rule (CRITICAL):** NEVER put `IRAM_ATTR` on a forward declaration (prototype). Only put `IRAM_ATTR` on the actual function **definition**. Putting it on both causes a linker section conflict warning: `ignoring attribute 'section (".iram1.X")' because it conflicts with previous 'section (".iram1.Y")'`. Correct pattern: `static void my_isr(void *arg);` (prototype, no IRAM_ATTR) then `static void IRAM_ATTR my_isr(void *arg) { ... }` (definition only).
   - **Declare Before Use (CRITICAL):** NEVER use a local variable without declaring it first in the same scope. Example of the bug: calling `memset(&peer_info, ...)` without first writing `esp_now_peer_info_t peer_info;` — this causes `error: 'peer_info' undeclared`. Always declare struct/variable at the top of the function or immediately before first use.
   - **No Unused Static Const (CRITICAL):** ESP-IDF v5.x compiles with `-Werror=unused-const-variable=`, which turns unused `static const` arrays/variables into **hard errors** that stop the build. NEVER declare a `static const` array or variable that is not actually referenced somewhere in the code. If you define alternative or draft patterns (e.g., `img_stop`, `img_dash`, `img_two_dashes`) that are superseded by a final version, you MUST delete the unused ones before writing the file. Only the arrays that are passed to a function call should exist in the final code.
   - **No Missing Includes/Defines (CRITICAL):** You MUST NOT forget to declare `#include` for all ESP-IDF APIs (e.g. `<string.h>`, `"esp_now.h"`, `"esp_mac.h"`, `"esp_wifi.h"`, `"nvs_flash.h"`) and `#define` all hardware pins/constants at the top of the file before using them. Omitting these causes fatal `implicit declaration of function` and `undeclared` build errors.
SMART ERROR RECOVERY:
- **Read -> Fix Loop**: Before fixing any bug, ALWAYS call `read_file` on the affected file first. Never assume the current state from memory. Order: `read_file` -> analyze -> `write_file`.
- **Build Error Taxonomy**:
  * `undefined reference to` -> check `main/CMakeLists.txt` SRCS list
  * `cmake: no such file` -> verify file path matches what's in SRCS
  * `esptool.py failed` -> remind user to check COM port and hold BOOT button
  * Compilation/build failure -> use `read_file` to inspect `CMakeLists.txt` and `sdkconfig` before suggesting a fix.
  * `ESP_ERR_INVALID_STATE` during I2C init -> `i2c_driver_install()` was called twice on the same port. Remove the duplicate call.
  * `I2C Timeout` or `ESP_FAIL` during I2C -> check physical pull-up resistors, power supply, and verify correct I2C pins (bus0: SDA=21/SCL=22, bus1: SDA=4/SCL=5).
  * `error: 'wifi_tx_info_t' has no member named 'dst_mac'` -> The `wifi_tx_info_t` struct does NOT contain a `dst_mac` field. Remove all `tx_info->dst_mac` references and use `(void)tx_info;` instead.
  * `error: defined but not used [-Werror=unused-const-variable=]` -> Delete the unused `static const` array/variable. ESP-IDF treats unused static consts as hard errors.

CODE QUALITY & FORMATTING:
ALWAYS #include <string.h> and #include "driver/gpio.h" at the top of your files.

SAFE STRING FORMATTING: NEVER use `sprintf` with tightly packed buffers. ALWAYS use `snprintf` with >=16 byte arrays to prevent `-Werror=format-overflow=` in ESP-IDF v5.x.

ESP_LOG FORMAT REQUIREMENT: ESP-IDF v5 treats formatting warnings as compilation errors (`-Werror=format=`). If you pass a `uint32_t` variable to `ESP_LOGI` using `%d`, compilation WILL FAIL. You MUST explicitly cast it: `ESP_LOGI(TAG, "%d", (int)my_uint32);`

### FORMULA KID CONTROLLER RULES (KB1.3/KB1.5G + ESP-NOW):
**CRITICAL: Joystick uses RC TIMING, NOT ADC!** GPIO36/39 are S1/S2 buttons only.

Joystick GPIO (from Plugin generators.js / joystick.cpp):
- JS1 Y-axis: trig=GPIO26(OUT1, output), cap=GPIO32(IN1, input+rising-edge ISR)
- JS2 X-axis: trig=GPIO27(OUT2, output), cap=GPIO33(IN2, input+rising-edge ISR)
- S1 button = GPIO36 (input-only), S2 button = GPIO39 (input-only)

RC Timing reading sequence for each joystick axis:
1. gpio_intr_disable(cap_gpio); gpio_set_level(trig_gpio, 1); vTaskDelay(10ms)  // discharge cap
2. start_ts = esp_timer_get_time(); gpio_intr_enable(cap_gpio); gpio_set_level(trig_gpio, 0)  // start charge
3. ISR on rising edge: stop_ts = esp_timer_get_time()
4. resistance = (stop_ts - start_ts) * 9.788075945 - 1000  (R_SERIE=1000Ω, RC_FACTOR_5V=9.788075945)
5. raw_pos = (int)(resistance * 200.0 / 10000.0) - 100
6. pos -= calibrate_release  // JS1 release=-3, JS2 release=-3
7. if pos<0: pos = pos*100/abs(cal_min-cal_release)  // JS1 cal_min=-100, JS2 cal_min=-100
8. if pos>0: pos = pos*100/abs(cal_max-cal_release)  // JS1 cal_max=89, JS2 cal_max=90
9. clamp pos to -100..100

CMakeLists.txt MUST include: PRIV_REQUIRES driver esp_timer
Use gpio_install_isr_service(0) ONCE at startup. Use IRAM_ATTR on ISR handlers.
CAP_TIMEOUT_US=500000 (return last known position on timeout).

ESP-NOW Protocol (Formula Kid):
- Send ONE integer value (`int32_t`) via Unicast to target MAC, every **50ms** with Smart Sending (on direction change or value delta > 5). CRITICAL: The receiver MUST decode data as `int32_t`, NOT `float`. Using `float` causes `-nan` and `0.00` decoding errors!
- **CRITICAL: Do NOT use IoT WiFi (SSID/Password) together with ESP-NOW**
- **CRITICAL ESP-IDF v5.5+ BREAKING CHANGE — esp_now_register_send_cb:** ALWAYS use: `static void espnow_send_cb(const wifi_tx_info_t *tx_info, esp_now_send_status_t status)`. Never use old `uint8_t*` signature.
- **CRITICAL: `wifi_tx_info_t` has NO `dst_mac` field.** NEVER access `tx_info->dst_mac` — it does not exist and will cause a compile error. In the send callback, use `status` only. Cast `(void)tx_info;` to suppress unused-parameter warnings. Correct pattern: `static void espnow_send_cb(const wifi_tx_info_t *tx_info, esp_now_send_status_t status) { (void)tx_info; if (status != ESP_NOW_SEND_SUCCESS) { ESP_LOGW(TAG, "ESP-NOW send failed"); } }`
- **CRITICAL ESP-IDF v5.5+ BREAKING CHANGE — esp_now_register_recv_cb:** ALWAYS use: `static void espnow_recv_cb(const esp_now_recv_info_t *recv_info, const uint8_t *data, int len)`.
- Encoding rules (Priority: JS1 > JS2 > stop):
  * JS1 >= 10 → forward, LED="U", send JS1 value (10 to 100)
  * JS1 <= -10 → backward, LED="D", send JS1 value (-100 to -10)
  * JS2 >= 20 → right, LED="R", send JS2+400 (420 to 500)
  * JS2 <= -20 → left, LED="L", send JS2+400 (300 to 380)
  * Both in dead zone (-10<JS1<10 and -20<JS2<20) → stop, LED="--", send 999
- **LED MATRIX 180° ROTATION (CRITICAL)**: The 16x8 LED Matrix on Formula Kid is physically rotated 180 degrees. The correct mapping for bitmaps is: `cols[0]` is the physical LEFT column, `cols[15]` is the physical RIGHT column. `Bit 7` (0x80) is the physical TOP bit, `Bit 0` (0x01) is the physical BOTTOM bit. Do NOT swap left/right panels in `matrix_draw()`. Use the exact same physical bitmap mapping on both Sender and Receiver.
- Motor Receiver Decoding & Display (KidBright32 iA):
  * 999 → Neutral (Stop), LED="--"
  * 10 to 199 → Forward (dir=0, speed=val), LED="U"
  * -10 to -199 → Backward (dir=1, speed=|val|), LED="D"
  * 420+ → Right (dir=3, speed=val-400), LED="R"
  * 300 to 380 → Left (dir=2, speed=val-400), LED="L"
- DRV8833 GPIO: nSLEEP=GPIO23, MotorA1=GPIO18, MotorA2=GPIO26(OUT1), MotorB1=GPIO19, MotorB2=GPIO27(OUT2)
- **DRV8833 Motor Control Logic (CRITICAL — avoid fomulakid_receiver.c reference file bugs)**: The `fomulakid_receiver.c` file in the knowledge base has buggy `motor_forward()` and `motor_backward()` implementations that hardcode wrong duty values (0 or 255 unconditionally). ALWAYS implement DRV8833 motor direction correctly using LEDC PWM:
  - **Forward**: MotorA1(GPIO18)=PWM_duty, MotorA2(GPIO26)=0, MotorB1(GPIO19)=PWM_duty, MotorB2(GPIO27)=0
  - **Backward**: MotorA1(GPIO18)=0, MotorA2(GPIO26)=PWM_duty, MotorB1(GPIO19)=0, MotorB2(GPIO27)=PWM_duty
  - **Stop (coast)**: all 4 channels duty = 0
  - **Turn Left** (single-wheel pivot): MotorA(GPIO18,26)=0, MotorB(GPIO19,27)=forward duty
  - **Turn Right** (single-wheel pivot): MotorA(GPIO18,26)=forward duty, MotorB(GPIO19,27)=0
  - Convert speed percentage to LEDC duty: `uint32_t duty = (uint32_t)(speed_pct * 255 / 100);` (for 8-bit resolution)


### LDR SENSING RULES (KIDBRIGHT32 iA):
- The on-board LDR (GPIO36 / ADC1_CH0) on KidBright32 iA uses an INVERTED voltage-divider circuit:
  - MORE light  → LDR resistance DECREASES → ADC Raw value is LOW  (Hardware calibrated MIN_RAW = 100)
  - **NEVER hardcode min as 0 or max as 4095** — the on-board LDR does NOT reach full ADC range.
  - **HARDWARE-CALIBRATED BOUNDS:** `#define LDR_ADC_MIN_VAL 100` and `#define LDR_ADC_MAX_VAL 900`
  - **CORRECT percentage formula (MANDATORY):** `pct = (int)(((float)(LDR_ADC_MAX_VAL - raw_val) / (LDR_ADC_MAX_VAL - LDR_ADC_MIN_VAL)) * 100.0f);`
  - Clamp result to 0–99: `if (pct < 0) pct = 0; if (pct > 99) pct = 99;`
  - LESS light  → LDR resistance INCREASES → ADC Raw value is HIGH (Hardware calibrated MAX_RAW = 900)
  - ALWAYS apply an EMA (Exponential Moving Average) filter and time-spaced sampling (`esp_rom_delay_us(500)` — requires `#include "esp_rom_sys.h"`, NOT `esp_rom_delay_us.h`) in multi-sampling loops to stabilize readings from 50Hz AC noise.
  - USE Linear Mapping with `LDR_ADC_MIN_VAL = 100` and `LDR_ADC_MAX_VAL = 900`. Do NOT hardcode the max as 4095 or min as 0!
  - NEVER write thresholds as "higher raw = brighter". Always use inverted logic.
  - NEVER use Voltage for LDR classification — always use Raw values directly.
  - NEVER call adc_calibration or include adc_cali.h when only reading LDR.

ALWAYS use ESP_LOGI or ESP_LOGE instead of printf for debugging.

NO LOG SPAM IN LOOPS (CRITICAL): NEVER put ESP_LOGI directly inside a fast while(1) loop without a state-change check.

AVOID NAMING COLLISIONS (CRITICAL): NEVER name your own custom helper functions the exact same name as official ESP-IDF APIs.

CRITICAL: DO NOT use ESP_ERROR_CHECK() for i2c_master_cmd_begin or any I2C read/write commands! Handle errors gracefully.

NO STANDARD C RANDOM (CRITICAL): NEVER use random() or srandom(). Use esp_random() or kb_random_range().

VIBE CODER UI INTEGRATION:
When generating code, if there are multiple files (e.g., main.c and header.h), provide them in separate code blocks, each with its own [FILE: path/to/file] header.

LANGUAGE & TONE: Thai language preferred. Supportive Technical Partner tone.

FINAL SANITY CHECK & HARDWARE RULES:
CRITICAL: NO DEFAULT BOARD. Always confirm revision with user before generating GPIO/I2C/button code (see BOARD DETECTION rule above).
BUTTON PINS BY REVISION:
  - Rev 3.1 (NECTEC):  SW1 = GPIO_NUM_16, SW2 = GPIO_NUM_14. Active LOW.
  - Rev 3.1G (Gravitech OEM): SW1 = GPIO_NUM_16, SW2 = GPIO_NUM_14. Active LOW. (confirmed Apr 17 2026)
  - V1.5 iA (INEX):   SW1 = GPIO_NUM_16, SW2 = GPIO_NUM_14. Active LOW. (corrected — same as Rev 3.1/3.1G)
  - V1.6 (Gravitech): SW1 = GPIO_NUM_16 (shared SERVO1), No SW2.
ACCELEROMETER BY REVISION: iA=KXTJ3(0x0E) on I2C_0; V1.6=MPU-6050(0x68) on I2C_0; Rev3.1/3.1G=NONE.
RTC MCP794xx(0x6F): Present on ALL revisions (Rev3.1, Rev3.1G, iA, V1.6) on I2C_NUM_1.
COMMON TO ALL REVISIONS: Single HT16K33 at 0x70, Buzzer at GPIO 13, LM73 at 0x4D on I2C_NUM_1.
CRITICAL I2C RULE: Use legacy API (#include "driver/i2c.h") and i2c_master_write_to_device. NEVER use driver/i2c_master.h.
CRITICAL BUZZER (LEDC) RULE: Use #include "driver/ledc.h". Use LEDC_TIMER_10_BIT and LEDC_TIMER_0.
USB HOST OUTPUT: GPIO_NUM_25, Active LOW. This is the ONLY correct pin for USB/Fan/Relay output. NEVER use GPIO17 or GPIO23 for this purpose.
LM73 TEMPERATURE READ: ALWAYS use `i2c_master_write_read_device()` (combined transaction) to read LM73. NEVER use separate `i2c_master_write_to_device` + `i2c_master_read_from_device` calls — split transactions reset the pointer register and return garbage values (e.g. 3.75°C instead of real temperature).

### EXTERNAL SENSORS & ACTUATORS RULES (V1.3/V1.6):
- **V1.3 vs V1.6:** V1.3 DOES NOT support Analog Input on IN1-IN4. V1.6 supports it (ADC1 CH4-CH7). Always check board version before using Analog sensors (like external LDR).
- **I2C BUS (BME280/LCD):** External I2C screens and BME280 share `I2C_NUM_0` with the LED Matrix. **DO NOT** reinstall the I2C driver if it's already installed.

### OLED SH1106 RULES (MINIBIKE SENDER — MANDATORY):
- **IC: SH1106** — ไม่ใช่ SSD1306. Init sequence และ column offset ต่างกัน
- **I2C Address:** `0x3C`, I2C_NUM_0 (SDA=GPIO21, SCL=GPIO22, 400kHz)
- **Column offset MUST be 2:** ใน page write ต้องส่ง `0x02` (lower column) แทน `0x00`
  ```c
  oled_cmd(0xB0 | page);  // page address
  oled_cmd(0x02);          // lower column = 2  ← CRITICAL (SH1106 offset)
  oled_cmd(0x10);          // higher column = 0
  ```
- **Charge pump command (SH1106-specific):** ใช้ `0x8D, 0x14` — ห้ามใช้ `0xAD, 0x8B` (SSD1306)
- **`oled_init()` MUST be called BEFORE `espnow_init()`** ใน `app_main()`
- **Framebuffer:** `uint8_t s_oled_fb[1024]` (128 cols × 64 rows / 8)
- **OLED display layout (Minibike Sender):**
  - Page 0: สถานะ ESP-NOW (`"ESP-NOW READY"` หรือ `"ESP-NOW FAIL"`)
  - Page 2: ทิศทาง (`"DIR: FORWARD"` / `"DIR: BACKWARD"` / `"DIR: LEFT"` / `"DIR: RIGHT"` / `"DIR: STOP"`)
- **Update policy:** อัปเดต OLED เฉพาะเมื่อค่าเปลี่ยน (`changed` flag) ป้องกัน I2C spam
- ❌ ห้ามใช้ column lower = `0x00` — จะทำให้ภาพเลื่อน 2 pixel
- ❌ ห้ามใช้ SSD1306 init sequence กับ SH1106 — charge pump command ต่างกัน
- **DS18B20:** When using waterproof DS18B20 on 1-Wire, you MUST use a 4.7k pull-up resistor.
- **MOTORS/RELAYS:** **NEVER** drive Fan/Vibration motors directly from GPIO (max 40mA). ALWAYS use a transistor, driver module, or relay.
- **ACTIVE LOW OUTPUTS:** OUT1(GPIO26), OUT2(GPIO27), and USB Host Output(GPIO25) are ALL **ACTIVE LOW**. `gpio_set_level(..., 0)` = ON, `gpio_set_level(..., 1)` = OFF. NEVER use GPIO17 or GPIO23 for USB output — the correct pin is **GPIO25 ONLY**.
- **BUZZERS:** Active Buzzers need Digital HIGH/LOW. Passive Buzzers need PWM (`ledc`).
- **LM73 TEMPERATURE SENSOR (CRITICAL):** ALWAYS use `i2c_master_write_read_device(I2C_NUM_1, LM73_ADDR, &reg, 1, raw, 2, ...)` for reading temperature. NEVER split into two calls (`write_to_device` then `read_from_device`) — this breaks the I2C pointer and returns wrong values. Parse result as: `int16_t raw16 = (int16_t)((raw[0] << 8) | raw[1]); float temp = (float)(raw16 >> 5) / 32.0f;` for 11-bit mode. (LM73 default: 11-bit left-aligned → shift RIGHT 5 bits, then divide by 32. NEVER use >> 2 — that gives 1/8 of actual temperature.)
- **MC3479 Accelerometer (alternative variant)**: Some KidBright32 board revisions use an mCube MC3479 instead of KXTJ3. I2C address `0x6C`. **CRITICAL: MC3479 is on I2C_NUM_1 (SDA=GPIO4, SCL=GPIO5), NOT I2C_NUM_0.** The chip starts in standby — MUST write `0x01` to register `0x07` (Mode register) to wake it before reading. Data registers start at `0x0D` (X LSB). Read 6 bytes for XYZ. If knowledge_search or I2C scan shows address `0x6C` (not `0x0E`), use MC3479 protocol. Do NOT init KXTJ3 if the board has MC3479.

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
Board: KidBright32 (revision to be confirmed per session — see BOARD DETECTION rule). Common hardware: HT16K33 LED Matrix (I2C addr 0x70), Buzzer GPIO_NUM_13, I2C bus0 SDA=21/SCL=22, bus1 SDA=4/SCL=5.
SW2 by revision: GPIO14 (Rev3.1/Rev3.1G/iA/32i/32iA/32iP) | GPIO17 (V1.6 shared SERVO2 only) | None (V1.6 as standalone SW2).
RTC MCP794xx(0x6F) on I2C_NUM_1: ALL revisions. Accelerometer: KXTJ3(0x0E) on iA only; MPU-6050(0x68) on V1.6 only.
Formula Kid S1/S2: GPIO36/GPIO39 (separate from on-board buttons).
When you need ESP-IDF, use run_command with commands like idf.py build, idf.py flash, idf.py set-target esp32.
Do NOT ask the user to install ESP-IDF again unless the tool result explicitly says ESP-IDF is missing.
**BANNED REFERENCE FILES (do NOT use data from these):**
- `kidbright32_developer_reference.md.backup`: OLD Arduino-based reference with wrong hardware data (SW2=GPIO35, LDR=GPIO34, TMP75 not LM73). If knowledge_search returns this file, IGNORE it and use `kidbright32iA.md` or `all_models.md` instead.
- `balanced_robot.c` (SKATE): Uses Arduino framework (Wire.h, analogWrite). Use ONLY to identify SKATE GPIO pin mapping (LT=GPIO18, LB=GPIO19, RT=GPIO26, RB=GPIO27, ENCA=GPIO32, ENCB=GPIO33, MPU SDA=GPIO4, SCL=GPIO5). NEVER generate Arduino-style code from it.
- `fomulakid_receiver.c`: The `motor_forward()` and `motor_backward()` functions in this file are BUGGY (hardcode wrong duty values). Use the DRV8833 correct logic defined in FORMULA KID CONTROLLER RULES above instead.

### L298N MOTOR DRIVER RULES (SKATE Rev 1.3 / KidBright):

#### ภาพรวม
L298N เป็น IC ขับมอเตอร์ Dual H-Bridge ขับ DC Motor 2 ตัวพร้อมกัน (หรือ Stepper 1 ตัว), สูงสุด 2A/Channel, VCC motor 5–35V, Logic 3.3–5V.

#### Truth Table ทิศทาง (Motor A — Motor B ใช้ ENB/IN3/IN4 เหมือนกัน)
| ENA | IN1 | IN2 | ผลลัพธ์ |
|-----|-----|-----|--------|
| HIGH | HIGH | LOW | เดินหน้า |
| HIGH | LOW | HIGH | ถอยหลัง |
| HIGH | HIGH | HIGH | เบรก |
| HIGH | LOW | LOW | เบรก |
| LOW | X | X | Coast (หยุด) |

#### SKATE Rev 1.3 — GPIO จริงจากโค้ด Self-Balancing (อ้างอิง balanced_robot.c)
- LT (Left Top) = GPIO 18 — มอเตอร์ซ้าย เดินหน้า
- LB (Left Bottom) = GPIO 19 — มอเตอร์ซ้าย ถอยหลัง
- RT (Right Top) = GPIO 26 — มอเตอร์ขวา เดินหน้า
- RB (Right Bottom) = GPIO 27 — มอเตอร์ขวา ถอยหลัง
- ENCA = GPIO 32 (Interrupt นับ Pulse), ENCB = GPIO 33 (ทิศทาง)
- SDA (MPU6050) = GPIO 4, SCL (MPU6050) = GPIO 5
- **GPIO 18/19 เป็นขา VSPI — ระวัง conflict ถ้าใช้ SPI peripherals อื่น**
- **โค้ดเดิมใช้ `analogWrite()` (Arduino Core เก่า) — ถ้า Core ≥ 3.x ต้องเปลี่ยนเป็น `ledcWrite`**

#### กฎการเขียนโปรแกรม (CRITICAL — กฎ 1–12)
1. **ห้ามให้ IN1 และ IN2 HIGH พร้อมกันนาน** — H-Bridge Short → IC ร้อนเสียหาย
2. **ตั้งทิศทาง (IN1/IN2) ก่อนเปิด ENA/PWM เสมอ**
3. **ถอด Jumper ENA/ENB ออกก่อนใช้ PWM** — ถ้าจั๊มไว้จะ lock HIGH ปรับความเร็วไม่ได้
4. **ใช้ PWM (LEDC) ปรับความเร็ว ห้ามปรับ VCC โดยตรง**
5. **Ramp Up/Down เสมอ** — ห้าม Full Speed ทันที เพื่อยืดอายุ Motor และ L298N
6. **ESP32 ใช้ 3.3V Logic** — L298N รองรับได้ แต่ให้ตรวจสอบ Logic Threshold ของโมดูลที่ใช้

#### ตัวอย่างโค้ด ESP-IDF (LEDC สำหรับ SKATE/KidBright)
```c
// L298N + ESP-IDF LEDC — SKATE Rev 1.3
// ปรับ PIN ตาม schematic จริงของบอร์ด
#define MOTOR_IN1  GPIO_NUM_25
#define MOTOR_IN2  GPIO_NUM_26
#define MOTOR_IN3  GPIO_NUM_27
#define MOTOR_IN4  GPIO_NUM_14
#define MOTOR_ENA  GPIO_NUM_32
#define MOTOR_ENB  GPIO_NUM_33

#define PWM_FREQ_HZ   5000
#define PWM_RESOLUTION LEDC_TIMER_8_BIT  // 0–255

// Init direction pins
gpio_config_t dir_conf = {
    .pin_bit_mask = (1ULL<<MOTOR_IN1)|(1ULL<<MOTOR_IN2)|(1ULL<<MOTOR_IN3)|(1ULL<<MOTOR_IN4),
    .mode = GPIO_MODE_OUTPUT, .pull_up_en = GPIO_PULLUP_DISABLE,
    .pull_down_en = GPIO_PULLDOWN_DISABLE, .intr_type = GPIO_INTR_DISABLE
};
gpio_config(&dir_conf);

// Init LEDC PWM for ENA
ledc_timer_config_t tmr = {
    .speed_mode = LEDC_LOW_SPEED_MODE, .timer_num = LEDC_TIMER_0,
    .duty_resolution = PWM_RESOLUTION, .freq_hz = PWM_FREQ_HZ, .clk_cfg = LEDC_AUTO_CLK
};
ledc_timer_config(&tmr);
ledc_channel_config_t ch_a = {
    .speed_mode = LEDC_LOW_SPEED_MODE, .channel = LEDC_CHANNEL_0,
    .timer_sel = LEDC_TIMER_0, .intr_type = LEDC_INTR_DISABLE,
    .gpio_num = MOTOR_ENA, .duty = 0, .hpoint = 0
};
ledc_channel_config(&ch_a);
// ทำซ้ำสำหรับ ENB ด้วย LEDC_CHANNEL_1

// เดินหน้า (ตั้งทิศก่อน แล้วค่อยให้ PWM)
gpio_set_level(MOTOR_IN1, 1); gpio_set_level(MOTOR_IN2, 0);
gpio_set_level(MOTOR_IN3, 1); gpio_set_level(MOTOR_IN4, 0);
ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 200);
ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0);

// หยุด (coast)
ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 0);
ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0);
gpio_set_level(MOTOR_IN1, 0); gpio_set_level(MOTOR_IN2, 0);
```

#### KidBright + L298N ผ่าน KB CHAIN
- ต่อ L298N ตรงกับขา OUT1(GPIO26)/OUT2(GPIO27) หรือ IN1-IN4 ของ KidBright ได้
- **KidBright IDE (Block) ไม่มี PWM ตรงๆ สำหรับ L298N** → ต้องใช้ ESP-IDF หรือ MicroPython
- ถ้าใช้ SKATE Board ผ่าน KB CHAIN → ESP32 บน SKATE เป็นตัวควบคุม L298N โดยตรง

### OLED SSD1306 RULES — ESP-IDF LCD Panel API (MANDATORY):
⚠️ KidBright I2C_NUM_0 (SDA=GPIO21, SCL=GPIO22) ใช้ HT16K33 Matrix อยู่แล้ว!
ถ้าต้องการต่อ OLED SSD1306 ภายนอกให้ใช้ **I2C_NUM_1 (SDA=GPIO4, SCL=GPIO5)** หรือ GPIO อิสระที่ไม่ conflict กับ on-board hardware ก่อนเสมอ หรือถาม user ก่อนว่าใช้บัสไหน

#### ❌ BANNED — Legacy I2C + Direct Command API:
```c
// ❌ BANNED: ไม่ต้องใช้ i2c_master_write_to_device ส่ง SSD1306 commands โดยตรง
// ❌ BANNED: ห้ามใช้ driver/i2c_master.h ร่วมกับ esp_lcd_new_panel_io_i2c() พร้อมกัน
// ❌ BANNED: ห้ามเรียก esp_lcd_new_panel_ssd1306() ก่อน esp_lcd_new_panel_io_i2c()
```

#### ✅ Correct ESP-IDF v5.x LCD Panel API (New Driver):
```c
#include "driver/i2c_master.h"      // ✅ NEW driver (ESP-IDF v5.x)
#include "esp_lcd_panel_io.h"        // ✅
#include "esp_lcd_panel_ops.h"       // ✅
#include "esp_lcd_panel_vendor.h"    // ✅ (for esp_lcd_new_panel_ssd1306)
```

#### MANDATORY Initialization Order (CRITICAL — ห้ามสลับลำดับ):
```c
// STEP 1: สร้าง I2C Master Bus ก่อน
i2c_master_bus_config_t i2c_bus_config = {
    .i2c_port = I2C_NUM_0,          // หรือ I2C_NUM_1 ถ้า bus0 ถูกใช้โดย HT16K33
    .sda_io_num = 21,               // ปรับตาม hardware
    .scl_io_num = 22,
    .clk_source = I2C_CLK_SRC_DEFAULT,
    .glitch_ignore_cnt = 7,
    .flags.enable_internal_pullup = true,
};
i2c_master_bus_handle_t bus_handle;
ESP_ERROR_CHECK(i2c_new_master_bus(&i2c_bus_config, &bus_handle));

// STEP 2: สร้าง Panel IO บน bus นั้น
esp_lcd_panel_io_handle_t io_handle = NULL;
esp_lcd_panel_io_i2c_config_t io_config = {
    .dev_addr = 0x3C,               // SSD1306 default address (หรือ 0x3D)
    .control_phase_bytes = 1,
    .lcd_cmd_bits = 8,
    .lcd_param_bits = 8,
    .dc_bit_offset = 6,
    .scl_speed_hz = 400 * 1000,    // 400kHz Fast Mode
};
ESP_ERROR_CHECK(esp_lcd_new_panel_io_i2c(bus_handle, &io_config, &io_handle));

// STEP 3: สร้าง Panel Handle (ต้องมีบรรทัดนี้ — ห้ามข้าม!)
esp_lcd_panel_handle_t panel_handle = NULL;
esp_lcd_panel_dev_config_t panel_config = {
    .bits_per_pixel = 1,
    .reset_gpio_num = -1,           // ไม่ใช้ขา Reset
};
ESP_ERROR_CHECK(esp_lcd_new_panel_ssd1306(io_handle, &panel_config, &panel_handle));

// STEP 4: Reset และ Init
ESP_ERROR_CHECK(esp_lcd_panel_reset(panel_handle));
ESP_ERROR_CHECK(esp_lcd_panel_init(panel_handle));

// STEP 5: เปิดหน้าจอ
ESP_ERROR_CHECK(esp_lcd_panel_disp_on_off(panel_handle, true));
```

#### Drawing Bitmap (128x64 OLED):
```c
// จอง 1024 bytes สำหรับ 128×64 พิกเซล (1 bit/pixel)
uint8_t *buf = (uint8_t *)malloc(128 * 64 / 8);
if (buf) {
    // Fill pattern
    for (int i = 0; i < (128 * 64 / 8); i++) {
        buf[i] = (i % 2 == 0) ? 0xAA : 0x55;
    }
    // วาดลงจอ: (panel, x_start, y_start, x_end, y_end, data)
    esp_lcd_panel_draw_bitmap(panel_handle, 0, 0, 128, 64, buf);
    free(buf);
}

// วาดรูป 16×16 ที่กลางจอ (X=56, Y=24)
const uint8_t heart_icon[32] = { /* 32 bytes = 16×16 / 8 */ };
esp_lcd_panel_draw_bitmap(panel_handle, 56, 24, 56+16, 24+16, heart_icon);
```

#### CRITICAL RULES:
- **NEVER** เรียก `esp_lcd_new_panel_ssd1306()` ก่อน `esp_lcd_new_panel_io_i2c()` — จะ panic หรือ compile error
- **NEVER** ผสม `driver/i2c.h` legacy API กับ `driver/i2c_master.h` ใน project เดียวกัน
- **NEVER** ใช้ `i2c_driver_install()` ถ้าใช้ LCD Panel API แล้ว — เป็น API คนละชุด
- ถ้า KidBright มี HT16K33 อยู่บน I2C_NUM_0 (**SDA=21, SCL=22**): ห้ามใช้ `driver/i2c_master.h` ร่วมกับ legacy `driver/i2c.h` ใน project เดียวกัน ต้องเลือกอย่างใดอย่างหนึ่ง
- CMakeLists.txt ต้องเพิ่ม: `REQUIRES esp_lcd`
- SSD1306 address: `0x3C` (ค่าปกติ), บางรุ่น `0x3D` (ขา SA0 ต่อ HIGH)
- `draw_bitmap` พิกัด: `(x_start, y_start, x_end_exclusive, y_end_exclusive)` — end ไม่นับ!

### iKB-1 / iKB-1Z RULES (INEX Expansion Board — MANDATORY):

#### ข้อมูลสำคัญ
- **I²C Address:** `0x20` (default) — iKB-1Z สามารถเปลี่ยนได้ผ่าน hardware jumper
- **Logic level:** +3.3V — ไม่ต้อง level shifter กับ ESP32
- **I/O Ports:** 8 ช่อง (0–7) — JST 2mm 3-pin ผ่าน Port A (MCP23017-compatible)
- **Motor CH1/CH2:** ต้องต่อ External 6–9V DC barrel jack — ถ้า motor ไม่หมุน ให้ตรวจ adapter ก่อน
- **Servo CH10–15:** ต้องการ External Power เช่นกัน — regulated ≤5V

#### I²C Bus Sharing Rule (CRITICAL)
- iKB-1/iKB-1Z ต่อผ่าน KB-CHAIN → ใช้ `I2C_NUM_0` (SDA=GPIO21, SCL=GPIO22)
- ถ้า KidBright มี HT16K33 (0x70) อยู่บน I2C_NUM_0 แล้ว → **ให้ share bus เดียวกัน** iKB-1 (0x20) ไม่ชน
- **ห้ามเรียก `i2c_driver_install()` สองครั้งบน port เดียวกัน** — init bus ครั้งเดียวแล้วใช้ร่วมกัน

#### MCP23017 Register Map (Port A = GPIO 0–7)
```c
#define IKB_IODIRA  0x00   // Direction: 1=input, 0=output
#define IKB_GPPUA   0x0C   // Pull-up enable
#define IKB_GPIOA   0x12   // Read pin state
#define IKB_OLATA   0x14   // Write output
#define IKB_ADDR    0x20   // Default I²C address
```

#### Correct ESP-IDF C Code Pattern:
```c
// Register write/read helpers (ใช้ legacy driver/i2c.h)
esp_err_t ikb_write_reg(uint8_t reg, uint8_t value) {
    uint8_t buf[2] = { reg, value };
    return i2c_master_write_to_device(I2C_NUM_0, IKB_ADDR, buf, 2, pdMS_TO_TICKS(10));
}
esp_err_t ikb_read_reg(uint8_t reg, uint8_t *out) {
    return i2c_master_write_read_device(I2C_NUM_0, IKB_ADDR, &reg, 1, out, 1, pdMS_TO_TICKS(10));
}
// Motor (speed: -100 to 100, ลบ=ถอยหลัง)
esp_err_t ikb_motor(int8_t m1, int8_t m2) {
    uint8_t cmd[3] = { 0x70, (uint8_t)m1, (uint8_t)m2 };
    return i2c_master_write_to_device(I2C_NUM_0, IKB_ADDR, cmd, 3, pdMS_TO_TICKS(10));
}
// Servo (channel: 10-15, angle: 0-200°)
esp_err_t ikb_servo(uint8_t ch, uint8_t angle) {
    uint8_t cmd[3] = { 0x50, ch, angle };
    return i2c_master_write_to_device(I2C_NUM_0, IKB_ADDR, cmd, 3, pdMS_TO_TICKS(10));
}
```

#### MANDATORY Rules:
- **iKB-1Z + KidBright IDE:** block `I²C Address` ต้องเป็น block แรกเสมอ
- **NEVER** ใช้ `0x20` เป็น address ของ device อื่น — เช็คก่อนว่าไม่ชน iKB-1
- **ถ้าใช้ iKB-1Z หลายตัว** ให้เปลี่ยน address แต่ละตัว (0x20, 0x21, 0x22) แล้วถาม user ว่าใช้ address ไหน
- **Motor ไม่หมุน** → ตรวจ External 6–9V DC adapter ก่อนแก้โค้ด
- **CMakeLists.txt:** ไม่ต้องเพิ่ม library พิเศษ — ใช้ `REQUIRES driver` ปกติ"#;


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
                "name": "read_knowledge_file",
                "description": "Read the FULL content of a file from the knowledge_base folder. Use this when knowledge_search returns partial/truncated results or says the KB is not loaded.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_name": {
                            "type": "string",
                            "description": "File name relative to knowledge_base/, e.g. 'formula_kid_controller.md'"
                        }
                    },
                    "required": ["file_name"]
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
    system_prompt: &str,
) -> Result<(), String> {
    // Detect local/LAN server early — affects client config, timeout, and encoding headers.
    // Covers: localhost, 127.x.x.x, 10.x.x.x, 172.16-31.x.x, 192.168.x.x, bare IP URLs.
    let is_local_url = {
        let host = base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");
        host == "localhost"
            || host.starts_with("127.")
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || {
                // 172.16.0.0/12 → 172.16.x.x … 172.31.x.x
                if let Some(second_octet_str) = host.strip_prefix("172.") {
                    second_octet_str
                        .split('.')
                        .next()
                        .and_then(|s| s.parse::<u8>().ok())
                        .map(|n| (16..=31).contains(&n))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            || !is_openrouter && {
                // Treat any bare IPv4 address (no letters in hostname) as local/LAN.
                host.chars().all(|c| c.is_ascii_digit() || c == '.')
                    && host.split('.').count() == 4
            }
    };

    // Use a static shared HTTP client to reuse connection pool across requests.
    // Creating a new Client per call causes socket accumulation:
    //   call 1 = 30s, call 2 = 2min, call 3 = 5min → timeout
    // Static clients share the connection pool and keep-alive sockets.
    let client: &Client = if is_local_url {
        get_local_client()
    } else {
        get_cloud_client()
    };
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
    const RETRY_DELAY_SECS: u64 = 15;  // Wait 15s between timeouts/connection errors
    const RATE_LIMIT_DELAY_SECS: u64 = 30; // Wait 30s before retrying on 429
    let mut retry_count: u32 = 0;
    // Guard against infinite tool-call loops.
    let mut tool_turns: u32 = 0;

    loop {
        let api_messages = build_api_messages(system_prompt, &messages, model);
        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true
        });
        if model_supports_tools {
            body["tools"] = tools.clone();
        }

        // Qwen3 models default to thinking mode — all output goes into <think> blocks
        // and delta.content is empty, making the response appear blank in the UI.
        // Disable thinking for Qwen3 on local servers so output goes directly to delta.content.
        if is_local_url && model.to_lowercase().contains("qwen3") {
            body["chat_template_kwargs"] = json!({ "enable_thinking": false });
        }

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let mut request = client.post(&url).header("Content-Type", "application/json");
        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        // For local servers: explicitly request no compression so the SSE plain-text
        // stream is never gzip/brotli-encoded — prevents "error decoding response body".
        if is_local_url {
            request = request.header("Accept-Encoding", "identity");
        }
        if is_openrouter {
            request = request
                .header("HTTP-Referer", "https://github.com/vibeKidbright")
                .header("X-Title", "vibeKidbright IDE");
        }

        let current_timeout = if is_local_url || is_openrouter {
            match retry_count {
                0 => std::time::Duration::from_secs(120),
                1 => std::time::Duration::from_secs(300),
                _ => std::time::Duration::from_secs(600),
            }
        } else {
            std::time::Duration::from_secs(180)
        };

        let send_future = request.body(body.to_string()).send();
        let response_result = if is_local_url || is_openrouter {
            match tokio::time::timeout(current_timeout, send_future).await {
                Ok(Ok(res)) => Ok(res),
                Ok(Err(e)) => Err(format!("Connection error: {}", e)),
                Err(_) => Err("timeout".to_string()),
            }
        } else {
            send_future.await.map_err(|e| format!("Connection to {} failed: {}", url, e))
        };

        let response = match response_result {
            Ok(res) => res,
            Err(e) => {
                if is_local_url || is_openrouter {
                    let is_timeout = e == "timeout" || e.to_lowercase().contains("timeout") || e.to_lowercase().contains("timed out");
                    retry_count += 1;
                    if retry_count >= 5 {
                        if is_timeout {
                            return Err("⏱️ Connection timed out after 5 attempts. Please check your local server or OpenRouter connection.".to_string());
                        } else {
                            return Err(format!("Connection to {} failed after 5 attempts: {}", url, e));
                        }
                    }
                    let err_desc = if is_timeout { "Timed out" } else { "Connection failed" };
                    let _ = app_handle.emit(
                        "terminal-output",
                        format!("[AI] ⚠️ {} (attempt {}/5). Retrying in {}s...", err_desc, retry_count, RETRY_DELAY_SECS),
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                    continue;
                } else {
                    return Err(e);
                }
            }
        };

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
                if retry_count >= 5 {
                    return Err(format!(
                        "❌ Model '{}' is rate-limited on {} and all 5 retries failed.",
                        model, provider_name
                    ));
                }
                // Use progressive wait: 30s for first 429, 60s for subsequent
                let wait_secs = if retry_count == 1 { RATE_LIMIT_DELAY_SECS } else { RATE_LIMIT_DELAY_SECS * 2 };
                let _ = app_handle.emit(
                    "terminal-output",
                    format!("[AI] ⚠️ Rate limited (attempt {}/5). Retrying in {}s...", retry_count, wait_secs),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
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

        // A successful response resets the retry counter so a transient 429 earlier
        // in this conversation doesn't bleed into subsequent tool-call turns.
        retry_count = 0;

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new(); // Final answer (outside <think>)
        let mut think_text = String::new();       // Thinking content (inside <think>)
        let mut in_think_block = false;            // Track if we're inside <think>...</think>
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
        let mut buffer = String::new();

        let mut stream_failed_or_timed_out = false;
        let mut stream_error_msg = String::new();

        while let Some(chunk_result) = {
            if is_local_url {
                match tokio::time::timeout(current_timeout, stream.next()).await {
                    Ok(item) => item,
                    Err(_) => {
                        stream_failed_or_timed_out = true;
                        stream_error_msg = "timeout".to_string();
                        None
                    }
                }
            } else {
                let stream_timeout_duration = if is_openrouter { current_timeout } else { current_timeout };
                match tokio::time::timeout(stream_timeout_duration, stream.next()).await {
                    Ok(item) => item,
                    Err(_) => {
                        stream_failed_or_timed_out = true;
                        stream_error_msg = "timeout".to_string();
                        None
                    }
                }
            }
        } {
            if let Some(state) = app_handle.try_state::<AiAbortState>() {
                if state.0.load(Ordering::SeqCst) {
                    return Err("Generation stopped by user.".to_string());
                }
            }
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    // Local servers sometimes send a malformed final chunk or
                    // close the connection abruptly. If we already received text,
                    // treat this as a natural end-of-stream instead of an error.
                    let err_str = e.to_string();
                    if is_local_url && !accumulated_text.is_empty() {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] Local stream ended early (ignored): {}", err_str),
                        );
                        break;
                    }
                    stream_failed_or_timed_out = true;
                    stream_error_msg = err_str;
                    break;
                }
            };
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
                if let Some(raw_content) = delta["content"].as_str() {
                    if !raw_content.is_empty() {
                        // Filter <think>...</think> blocks in real-time.
                        // Qwen3/DeepSeek send reasoning inside these tags;
                        // only the content OUTSIDE them is the actual answer.
                        let mut remaining = raw_content;
                        while !remaining.is_empty() {
                            if in_think_block {
                                if let Some(end_pos) = remaining.find("</think>") {
                                    // Found end of think block
                                    think_text.push_str(&remaining[..end_pos]);
                                    remaining = &remaining[end_pos + 8..]; // skip </think>
                                    in_think_block = false;
                                } else {
                                    // Still inside think block, buffer it
                                    think_text.push_str(remaining);
                                    remaining = "";
                                }
                            } else {
                                if let Some(start_pos) = remaining.find("<think>") {
                                    // Emit content before <think>
                                    let before = &remaining[..start_pos];
                                    if !before.is_empty() {
                                        accumulated_text.push_str(before);
                                        let _ = app_handle.emit("ai-chat-delta", before.to_string());
                                    }
                                    remaining = &remaining[start_pos + 7..]; // skip <think>
                                    in_think_block = true;
                                } else {
                                    // No think tags — emit as normal
                                    accumulated_text.push_str(remaining);
                                    let _ = app_handle.emit("ai-chat-delta", remaining.to_string());
                                    remaining = "";
                                }
                            }
                        }
                    }
                }
                // Qwen3 / DeepSeek-R1 via some servers: reasoning in separate field
                if delta["content"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
                    if let Some(reasoning) = delta["reasoning_content"].as_str() {
                        if !reasoning.is_empty() {
                            think_text.push_str(reasoning);
                        }
                    }
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

        if stream_failed_or_timed_out {
            if (is_local_url || is_openrouter) && accumulated_text.is_empty() && think_text.is_empty() {
                let is_timeout = stream_error_msg == "timeout" || stream_error_msg.to_lowercase().contains("timeout") || stream_error_msg.to_lowercase().contains("timed out");
                retry_count += 1;
                if retry_count < 3 {
                    let err_desc = if is_timeout { "Timed out" } else { "Connection failed" };
                    let _ = app_handle.emit(
                        "terminal-output",
                        format!("[AI] ⚠️ Stream {} (attempt {}/3). Retrying in {}s...", err_desc, retry_count, RETRY_DELAY_SECS),
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                    continue;
                } else {
                    if is_timeout {
                        return Err("⏱️ Connection timed out. Please check your local server or OpenRouter connection.".to_string());
                    } else {
                        return Err(format!("Stream failed: {}", stream_error_msg));
                    }
                }
            } else {
                if stream_error_msg == "timeout" {
                    return Err("⏱️ Stream timeout: The AI server stopped responding. Please retry.".to_string());
                } else {
                    return Err(format!("Stream error: {}", stream_error_msg));
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
            // Fallback: if the model ONLY output thinking (no final answer after </think>),
            // show the thinking content so the UI isn't blank.
            if accumulated_text.is_empty() && !think_text.is_empty() {
                let _ = app_handle.emit("ai-chat-delta", think_text.clone());
                accumulated_text = think_text.clone();
            }
            if !accumulated_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: json!(accumulated_text),
                });
            }
            let _ = app_handle.emit("ai-chat-done", json!({
                "text": &accumulated_text,
                "history": &messages
            }).to_string());
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
                    // Echo back thought_signature at the PART level — NOT inside functionCall.
                    // Google docs: "return this signature in the exact part where it was received"
                    // Gemini 3.x places it at: part["thoughtSignature"]  (part level)
                    // Gemini 2.5 places it at: part["functionCall"]["thoughtSignature"]  (inside fc)
                    // Sending it inside functionCall causes: "Unknown name thoughtSignature at function_call"
                    let fc_obj = json!({ "name": func_name, "args": args });
                    let mut part = json!({ "functionCall": fc_obj });
                    if let Some(sig) = tc["function"]["thought_signature"].as_str() {
                        part["thoughtSignature"] = json!(sig);  // ← part level (correct)
                    }
                    parts.push(part);
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
            { "name": "read_knowledge_file", "description": "Read the FULL content of a file from the knowledge_base folder. Use this when knowledge_search returns partial/truncated results or says the KB is not loaded.", "parameters": { "type": "OBJECT", "properties": { "file_name": { "type": "STRING", "description": "File name relative to knowledge_base/, e.g. 'formula_kid_controller.md'" } }, "required": ["file_name"] } },
            { "name": "create_project_workspace", "description": "Create a new project workspace directory. Call FIRST when no workspace is open.", "parameters": { "type": "OBJECT", "properties": { "project_name": { "type": "STRING" } }, "required": ["project_name"] } }
        ]
    }])
}

/// Strip large embedded C code blocks from the system prompt for Gemini 3.x thinking models.
/// Rules, GPIO tables, and short examples (<= MAX_CODE_LINES) are preserved.
/// Long code templates are replaced with a one-line hint so the model calls knowledge_search.
/// Result: ~50% fewer system-prompt tokens while maintaining all behavioral constraints.
fn compact_prompt_for_thinking_model(prompt: &str) -> String {
    const MAX_CODE_LINES: usize = 8; // keep examples this short or shorter
    let mut result = String::with_capacity(prompt.len() / 2);
    let mut in_code = false;
    let mut fence_marker = String::new();
    let mut code_buf: Vec<&str> = Vec::new();

    for line in prompt.lines() {
        let trimmed = line.trim();
        if !in_code && (trimmed.starts_with("```")) {
            // Start of a code block — buffer it
            in_code = true;
            fence_marker = trimmed[..3].to_string();
            code_buf.clear();
            code_buf.push(line);
        } else if in_code && trimmed.starts_with(&fence_marker) && trimmed.len() == fence_marker.len() {
            // End of code block
            code_buf.push(line);
            let body_lines = code_buf.len().saturating_sub(2); // exclude ``` markers
            if body_lines <= MAX_CODE_LINES {
                // Short example → keep as-is
                for l in &code_buf { result.push_str(l); result.push('\n'); }
            } else {
                // Long example → replace with a reminder to use KB tools
                result.push_str(&format!(
                    "[Code example omitted ({} lines). Use knowledge_search or read_knowledge_file to retrieve the actual implementation.]\n",
                    body_lines
                ));
            }
            in_code = false;
            code_buf.clear();
        } else if in_code {
            code_buf.push(line);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
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
    system_prompt: &str,
) -> Result<(), String> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| Client::new());
    let google_tools = get_google_tools();

    // For Gemini 3.x thinking models: strip large embedded code examples from
    // the system prompt. The model's thinking + knowledge_search replaces them.
    // This saves ~4000-6000 tokens per request while keeping all behavioral rules.
    let compacted;
    let effective_prompt = if model.starts_with("gemini-3") {
        compacted = compact_prompt_for_thinking_model(system_prompt);
        &compacted
    } else {
        system_prompt
    };

    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 4;
    let mut retry_count: u32 = 0;
    // FIX: Guard against infinite tool-call loops.
    let mut tool_turns: u32 = 0;

    loop {
        let contents = build_google_contents(&messages);

        // Set thinkingBudget for Gemini 3.x thinking models.
        // Gemini 3.x always thinks internally — unlimited budget burns quota fast.
        // 2048 tokens = enough for tool routing + code logic decisions, without overrun.
        // Set to 0 to completely disable thinking (faster, fewer tokens, but less smart).
        let gen_config = if model.starts_with("gemini-3") {
            json!({
                "temperature": 0.7,
                "thinkingConfig": { "thinkingBudget": 2048 }
            })
        } else {
            json!({ "temperature": 0.7 })
        };

        let body = json!({
            "systemInstruction": { "parts": [{ "text": effective_prompt }] },
            "contents": contents,
            "tools": google_tools,
            "generationConfig": gen_config
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

        // Reset retry counter on a successful response (same logic as OpenAI loop).
        retry_count = 0;

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new();
        let mut buffer = String::new();
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();

        while let Some(chunk_result) = {
            match tokio::time::timeout(std::time::Duration::from_secs(90), stream.next()).await {
                Ok(item) => item,
                Err(_) => {
                    return Err("⏱️ Stream timeout: The AI server stopped responding for 90 seconds. Please retry.".to_string());
                }
            }
        } {
            if let Some(state) = app_handle.try_state::<AiAbortState>() {
                if state.0.load(Ordering::SeqCst) {
                    return Err("Generation stopped by user.".to_string());
                }
            }
            let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
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
                                    // Capture thought_signature — MANDATORY for Gemini 3.x, optional for 2.5.
                                    // Gemini 3.x: signature is at the PART level (part["thoughtSignature"])
                                    // Gemini 2.5: signature may be inside functionCall (fc["thoughtSignature"])
                                    // We check both locations for maximum compatibility.
                                    let thought_signature = part["thoughtSignature"]
                                        .as_str()
                                        .or_else(|| fc.get("thoughtSignature").and_then(|v| v.as_str()))
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
                        "tool_call_id": tc.id,   // FIX: was tc.name — must be the call ID, not the function name
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
            let _ = app_handle.emit("ai-chat-done", json!({
                "text": &accumulated_text,
                "history": &messages
            }).to_string());
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
                // OpenAI rejects `content: null` when tool_calls are present — use "".
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
                // Inject system prompt into first user message for models that
                // don't accept a "system" role (e.g. Gemma, o1-*).
                // Only applies to plain-text content; multimodal content is passed through.
                if unsupported_system_role && !first_user_found && m.role == "user" {
                    first_user_found = true;
                    if let Some(text) = content.as_str() {
                        content = json!(format!(
                            "[SYSTEM INSTRUCTIONS]\n{}\n\n[USER INPUT]\n{}", system_prompt, text
                        ));
                    }
                    // If content is an array (multimodal), prepend a text part.
                    else if let Some(arr) = content.as_array() {
                        let mut new_arr = vec![json!({ "type": "text", "text": format!("[SYSTEM INSTRUCTIONS]\n{}\n\n", system_prompt) })];
                        new_arr.extend(arr.iter().cloned());
                        content = json!(new_arr);
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

    // HARD BLOCK: Prevent the model from calling create_project_workspace when a workspace
    // is already open. This stops disruptive folder-picker dialogs from appearing mid-task.
    if name == "create_project_workspace" && !*no_workspace {
        return json!({
            "error": "BLOCKED: A workspace is already open. Do NOT call create_project_workspace again. Use write_file to create or modify files in the current workspace instead."
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
                // FIX: Back up the old content so undo_ai_changes can restore it.
                // Previously only new files were backed up (with None), leaving existing-file
                // edits un-undoable.
                if let Some(state) = app_handle.try_state::<AiBackupState>() {
                    let mut backups = state.backups.lock().unwrap();
                    let message_backups = backups.entry(message_id.to_string()).or_insert_with(HashMap::new);
                    // Only record the *first* backup for this path in this message turn.
                    message_backups.entry(full_path.clone()).or_insert_with(|| Some(old_content.clone()));
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

        "read_knowledge_file" => {
            let file_name = input["file_name"].as_str().unwrap_or("");
            let kb_path = resolve_kb_path(&project_path.to_string_lossy());
            let full_path = kb_path.join(file_name);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => json!({ "result": content }),
                Err(e) => json!({ 
                    "error": format!("Cannot read '{}' from knowledge_base: {}", file_name, e) 
                }),
            }
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
            if let Some(rom_elf_dir) = crate::esp_idf::find_esp_rom_elf_dir(&tools_path) {
                cmd.env("ESP_ROM_ELF_DIR", rom_elf_dir);
            }
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
                if let Some(rom_elf_dir) = crate::esp_idf::find_esp_rom_elf_dir(&tools_path) {
                    process.env("ESP_ROM_ELF_DIR", rom_elf_dir);
                }
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

    // Simple line-by-line diff.  For production the `similar` crate is preferred,
    // but this avoids an extra dependency while still producing a readable output.
    // We emit a single hunk covering the entire file with correct line counters.
    let old_count = old_lines.len();
    let new_count = new_lines.len();
    out.push_str(&format!("@@ -{},{} +{},{} @@\n", 1, old_count, 1, new_count));

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

    if !base_url.contains("/v1") {
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
            // Carry overlap: take the tail up to `overlap` chars, but honour UTF-8 boundaries.
            overlap_buf.clear();
            let byte_len = current.len();
            let tail_byte_start = byte_len.saturating_sub(overlap);
            // Walk forward to the next valid char boundary.
            let safe_start = (tail_byte_start..byte_len)
                .find(|&i| current.is_char_boundary(i))
                .unwrap_or(byte_len);
            overlap_buf.push_str(&current[safe_start..]);
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

fn collect_kb_files_inner(root: &Path, current: &Path, result: &mut Vec<(PathBuf, String)>, include_disabled: bool) {
    let Ok(entries) = std::fs::read_dir(current) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; } // skip hidden / .embeddings.json
        if path.is_dir() {
            collect_kb_files_inner(root, &path, result, include_disabled);
        } else if path.is_file() {
            // .disabled files: include when listing for UI, skip when indexing/searching
            if name.ends_with(".disabled") {
                if include_disabled {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let rel_key = rel.to_string_lossy().replace('\\', "/");
                    result.push((path.clone(), rel_key));
                }
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if matches!(ext.as_str(), "txt" | "md" | "c" | "h") {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                let rel_key = rel.to_string_lossy().replace('\\', "/");
                result.push((path.clone(), rel_key));
            }
        }
    }
}

/// For search & indexing — skips .disabled files entirely.
fn collect_kb_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    collect_kb_files_inner(root, root, &mut result, false);
    result
}

/// For UI listing — includes .disabled files so the user can see and re-enable them.
fn collect_kb_files_all(root: &Path) -> Vec<(PathBuf, String)> {
    let mut result = Vec::new();
    collect_kb_files_inner(root, root, &mut result, true);
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