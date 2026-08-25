use futures::StreamExt;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

// Extracted to ai/config.rs (refactoring step 1)
use crate::ai::config::{get_secure_key, read_config};
// Extracted to ai/kb.rs (refactoring step 3)
use crate::ai::kb::{collect_kb_files, collect_kb_files_all, get_kb_query_cache, knowledge_search, reindex_knowledge_base};

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,
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

pub(crate) fn resolve_kb_path(project_dir: &str) -> PathBuf {
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
                get_secure_key("vibekidbright-openrouter", "openrouter_api_key"),
                config["openrouter_model"].as_str().unwrap_or("anthropic/claude-3.5-sonnet").to_string(),
                "https://openrouter.ai/api/v1".to_string(),
            )
        } else if prov == "zen" {
            (
                get_secure_key("vibekidbright-zen", "zen_api_key"),
                config["zen_model"].as_str().unwrap_or("nemotron-3.5-lightning-free").to_string(),
                "https://opencode.ai/zen/v1".to_string(),
            )
        } else if prov == "google" {
            (
                get_secure_key("vibekidbright-google", "google_api_key"),
                config["google_model"].as_str().unwrap_or("gemini-2.5-flash").to_string(),
                "https://generativelanguage.googleapis.com/v1beta".to_string(),
            )
        } else {
            (
                get_secure_key("vibekidbright-openai", "api_key"),
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
    if api_key.is_empty() && provider == "zen" {
        return Err("OpenCode Zen API key not set. Get a free key at https://opencode.ai/zen".to_string());
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
                get_secure_key("vibekidbright-google", "google_api_key"),
                config["google_model"].as_str().unwrap_or("gemini-2.5-flash").to_string(),
                get_secure_key("vibekidbright-openrouter", "openrouter_api_key"),
                config["openrouter_model"]
                    .as_str()
                    .unwrap_or("google/gemini-2.5-flash:free")
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
                "google/gemini-2.5-flash:free",             // Best overall free
                "meta-llama/llama-3.3-70b-instruct:free",   // 70B standard
                "qwen/qwen-2.5-coder-32b-instruct:free",    // Best coder
                "deepseek/deepseek-chat:free",              // DeepSeek V3
                "nvidia/llama-3.1-nemotron-70b-instruct:free", // Nemotron
                // ── Tier 2: Mid-size free models ──────────────────────────────
                "microsoft/phi-3-medium-128k-instruct:free",
                "mistralai/mistral-7b-instruct:free",
                "google/gemma-2-9b-it:free",
                "huggingfaceh4/zephyr-7b-beta:free",
                "qwen/qwen-2-7b-instruct:free",
                "meta-llama/llama-3-8b-instruct:free",
                "openchat/openchat-7b:free",
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
                    "google/gemini-2.5-flash:free",
                    "meta-llama/llama-3.3-70b-instruct:free",
                    "qwen/qwen-2.5-coder-32b-instruct:free",
                    "deepseek/deepseek-chat:free",
                    "nvidia/llama-3.1-nemotron-70b-instruct:free",
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
// D2: SYSTEM_PROMPT is now stored in ai/system_prompt.txt and loaded at compile
// time via include_str!. This makes the prompt editable independently of the
// Rust source, and reduces this file by ~812 lines.
const SYSTEM_PROMPT: &str = include_str!("ai/system_prompt.txt");



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
            // FIX M2: install_idf_library was missing from Google tools — now parity with OpenAI tools.
            { "name": "install_idf_library", "description": "Install an ESP-IDF component dependency using idf.py add-dependency.", "parameters": { "type": "OBJECT", "properties": { "component": { "type": "STRING", "description": "Component identifier, e.g. espressif/led_strip or espressif/led_strip^2.5.3" } }, "required": ["component"] } },
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
    // FIX H5: Use shared static HTTP client (get_cloud_client) instead of creating
    // a new Client per invocation. Creating a new Client per request causes socket
    // accumulation and progressively slower responses (same issue fixed in OpenAI loop).
    let client = get_cloud_client();
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

// ── Helper: Myers unified diff (via `similar` crate) ─────────────────────────
// Replaces the previous O(n²) naive implementation.
// Produces standard unified diffs with proper hunk headers and context lines.

fn compute_unified_diff(old: &str, new: &str, path: &str) -> String {
    use similar::{Algorithm, ChangeTag, TextDiff};

    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_lines(old, new);

    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    let mut has_diff = false;

    for group in diff.grouped_ops(3) {
        has_diff = true;
        let first = &group[0];
        let _last = &group[group.len() - 1];

        let old_start = first.old_range().start + 1;
        let old_len: usize = group.iter().map(|op| op.old_range().len()).sum();
        let new_start = first.new_range().start + 1;
        let new_len: usize = group.iter().map(|op| op.new_range().len()).sum();

        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            old_start, old_len, new_start, new_len
        ));

        for op in &group {
            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal  => " ",
                };
                out.push_str(prefix);
                out.push_str(change.value());
                if !change.value().ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }

    if !has_diff {
        out.push_str("(no differences)\n");
    }

    out
}
// ── Web search ────────────────────────────────────────────────────────────────

async fn search_the_web(query: &str) -> Result<Value, String> {
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

// ── Unit Tests ────────────────────────────────────────────────────────────────
// Run with: cargo test -- --nocapture
// These tests cover pure-logic functions that don't require a Tauri AppHandle
// or any network connectivity.

#[cfg(test)]
mod tests {
    use super::*;

    // ── normalize_project_dir ─────────────────────────────────────────────────

    #[test]
    fn test_normalize_project_dir_strips_file_scheme() {
        let result = normalize_project_dir("file:///home/user/project");
        assert_eq!(result, "/home/user/project");
    }

    #[test]
    fn test_normalize_project_dir_no_prefix() {
        let result = normalize_project_dir("/home/user/project");
        assert_eq!(result, "/home/user/project");
    }

    #[test]
    fn test_normalize_project_dir_empty() {
        let result = normalize_project_dir("   ");
        assert_eq!(result, "");
    }

    // ── resolve_kb_path ───────────────────────────────────────────────────────

    #[test]
    fn test_resolve_kb_path_nonexistent_project_no_panic() {
        // Must not panic and must return a non-empty path
        let path = resolve_kb_path("/this/path/does/not/exist/in/any/fs");
        assert!(
            !path.to_string_lossy().is_empty(),
            "resolve_kb_path must always return a non-empty fallback path"
        );
    }
}
