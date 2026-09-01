use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

// Extracted to ai/config.rs (refactoring step 1)
use crate::ai::config::{get_secure_key, read_config};
// Extracted to ai/kb.rs (refactoring step 3)
use crate::ai::kb::{collect_kb_files, collect_kb_files_all, get_kb_query_cache, reindex_knowledge_base};
// Extracted to ai/providers.rs (refactoring step 5)
use crate::ai::providers::{get_rate_limited_models, run_conversation_loop, run_google_conversation_loop, SYSTEM_PROMPT};

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,
}

use std::sync::{atomic::{AtomicBool, Ordering}, Mutex};
use std::collections::HashMap;
use std::time::Instant;

// ── Global caches ─────────────────────────────────────────────────────────────

/// Cached IDF PATH string — computed once per session, not on every command.
static CACHED_IDF_PATH: OnceLock<Mutex<Option<OsString>>> = OnceLock::new();
fn get_cached_idf_path() -> &'static Mutex<Option<OsString>> {
    CACHED_IDF_PATH.get_or_init(|| Mutex::new(None))
}

/// Max tool-call turns per conversation to prevent infinite loops.

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

pub(crate) fn normalize_project_dir(project_dir: &str) -> String {
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

pub(crate) fn resolve_idf_paths_for_ai(app_handle: &AppHandle) -> Option<(PathBuf, PathBuf)> {
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

pub(crate) fn find_idf_python_bin(tools_path: &Path) -> Option<PathBuf> {
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
pub(crate) fn build_ai_idf_path_cached(tools_path: &Path) -> OsString {
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
                config["openrouter_model"].as_str().unwrap_or("google/gemini-2.5-flash:free").to_string(),
                "https://openrouter.ai/api/v1".to_string(),
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
                config["model"].as_str().unwrap_or("gpt-4.1").to_string(),
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
                "google/gemini-2.5-flash-lite:free",        // Lightest Gemini free
                "meta-llama/llama-3.3-70b-instruct:free",   // Llama 3.3 70B
                "deepseek/deepseek-chat-v3-0324:free",      // DeepSeek V3 (latest)
                "deepseek/deepseek-r1:free",                // DeepSeek R1 reasoning
                "qwen/qwen3-235b-a22b:free",                // Qwen3 flagship free
                "qwen/qwen-2.5-coder-32b-instruct:free",   // Best coder free
                "mistralai/mistral-small-3.2:free",         // Mistral small free
                // ── Tier 2: Mid-size free models ──────────────────────────────
                "meta-llama/llama-3.1-70b-instruct:free",
                "microsoft/phi-4-multimodal-instruct:free",
                "mistralai/mistral-7b-instruct:free",
                "google/gemma-3-12b-it:free",
                "qwen/qwen-2.5-7b-instruct:free",
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
