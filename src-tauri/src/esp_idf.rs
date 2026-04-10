use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager, Emitter};

static CACHED_ESP_IDF_CONFIG: OnceLock<Mutex<Option<serde_json::Value>>> = OnceLock::new();

const DEFAULT_ESP_IDF_VERSION: &str = "v5.5.1";
const ESP_IDF_REPO_URL: &str = "https://github.com/espressif/esp-idf.git";

// ── Persistent Config for custom paths ───────────────────────────────
// Stored in: %APPDATA%/vibeKidbright/config.json

fn esp_idf_config_path() -> std::path::PathBuf {
    let home = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".vibekidbright")
        .join("config.json")
}

fn read_esp_idf_config() -> serde_json::Value {
    let mut guard = get_cached_config().lock().unwrap();
    if let Some(config) = &*guard {
        return config.clone();
    }

    let path = esp_idf_config_path();
    let config = if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    *guard = Some(config.clone());
    config
}

fn write_esp_idf_config(config: &serde_json::Value) {
    let path = esp_idf_config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(config).unwrap_or_default());
    *get_cached_config().lock().unwrap() = None;
}

/// Check if the user has manually configured custom ESP-IDF paths in config
fn resolve_custom_config_paths() -> Option<(PathBuf, PathBuf)> {
    let config = read_esp_idf_config();
    let idf = config["custom_idf_path"].as_str()?;
    let tools = config["custom_tools_path"].as_str()?;
    if idf.is_empty() || tools.is_empty() {
        return None;
    }
    let idf_path = PathBuf::from(idf);
    let tools_path = PathBuf::from(tools);
    if idf_path.join("tools/idf.py").exists() && tools_path.exists() {
        Some((idf_path, tools_path))
    } else {
        None
    }
}

fn runtime_root_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    Ok(app_data_dir.join("esp-idf-runtime"))
}

fn runtime_default_paths(app_handle: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let root = runtime_root_dir(app_handle)?;
    let idf_path = root.join(format!("esp-idf-{}", DEFAULT_ESP_IDF_VERSION));
    let tools_path = root.join(".espressif");
    Ok((idf_path, tools_path))
}

fn maybe_find_runtime_idf(app_handle: &AppHandle) -> Result<Option<PathBuf>, String> {
    let root = runtime_root_dir(app_handle)?;
    if !root.exists() {
        return Ok(None);
    }

    let mut idf_dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .map_err(|e| format!("Failed to read runtime dir {}: {}", root.display(), e))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("esp-idf-"))
                && path.join("tools/idf.py").exists()
        })
        .collect();

    idf_dirs.sort();
    Ok(idf_dirs.pop())
}

fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    dunce::simplified(&path).to_path_buf()
}

fn canonical_idf_pair(idf_path: &Path, tools_path: &Path) -> Result<(PathBuf, PathBuf), String> {
    if !idf_path.join("tools/idf.py").exists() {
        return Err(format!(
            "ESP-IDF not valid at {} (missing tools/idf.py)",
            idf_path.display()
        ));
    }
    if !tools_path.exists() {
        return Err(format!("ESP-IDF tools path missing at {}", tools_path.display()));
    }

    let idf_abs = strip_unc_prefix(idf_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize IDF path: {}", e))?);
    let tools_abs = strip_unc_prefix(tools_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize tools path: {}", e))?);
    Ok((idf_abs, tools_abs))
}

fn resolve_env_override() -> Option<(PathBuf, PathBuf)> {
    let idf = std::env::var_os("VIBEKIDBRIGHT_IDF_PATH")?;
    let tools = std::env::var_os("VIBEKIDBRIGHT_TOOLS_PATH")?;
    Some((PathBuf::from(idf), PathBuf::from(tools)))
}

fn venv_bin_dir(venv_path: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        venv_path.join("Scripts")
    } else {
        venv_path.join("bin")
    }
}

fn venv_python_candidates(venv_path: &Path) -> Vec<PathBuf> {
    let bin_dir = venv_bin_dir(venv_path);
    if cfg!(target_os = "windows") {
        vec![bin_dir.join("python.exe")]
    } else {
        vec![bin_dir.join("python"), bin_dir.join("python3")]
    }
}

fn detect_host_python() -> Result<String, String> {
    for cmd in ["python3", "python"] {
        let Ok(status) = Command::new(cmd).arg("--version").status() else {
            continue;
        };
        if status.success() {
            return Ok(cmd.to_string());
        }
    }

    Err("Python was not found in PATH. Install Python 3 first.".to_string())
}

/// Resolve the ESP-IDF and tools paths (runtime, bundled, or dev fallback).
/// Priority: env override > user config > runtime > bundled.
fn resolve_idf_paths(app_handle: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    // 1. Environment variable override
    if let Some((idf_path, tools_path)) = resolve_env_override() {
        if let Ok(paths) = canonical_idf_pair(&idf_path, &tools_path) {
            return Ok(paths);
        }
    }

    // 2. User-configured paths from GUI settings (highest user priority)
    if let Some((idf_path, tools_path)) = resolve_custom_config_paths() {
        if let Ok(paths) = canonical_idf_pair(&idf_path, &tools_path) {
            return Ok(paths);
        }
    }

    if let Some(runtime_idf) = maybe_find_runtime_idf(app_handle)? {
        let tools = runtime_root_dir(app_handle)?.join(".espressif");
        if let Ok(paths) = canonical_idf_pair(&runtime_idf, &tools) {
            return Ok(paths);
        }
    }

    if let Ok((default_runtime_idf, default_runtime_tools)) = runtime_default_paths(app_handle) {
        if let Ok(paths) = canonical_idf_pair(&default_runtime_idf, &default_runtime_tools) {
            return Ok(paths);
        }
    }

    let resource_path = app_handle.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let idf_path = resource_path.join("esp-idf");
    let tools_path = resource_path.join(".espressif");

    if let Ok(paths) = canonical_idf_pair(&idf_path, &tools_path) {
        Ok(paths)
    } else {
        let dev_idf = PathBuf::from("../resources/esp-idf");
        let dev_tools = PathBuf::from("../resources/.espressif");
        if let Ok(paths) = canonical_idf_pair(&dev_idf, &dev_tools) {
            Ok(paths)
        } else {
            Err(
                "ESP-IDF not found. Run setup_esp_idf() on first launch to install platform-specific tools."
                    .to_string(),
            )
        }
    }
}

/// Find the Python binary and venv path inside the ESP-IDF virtual environment.
/// Returns (python_binary_path, venv_directory_path).
fn find_idf_python(tools_path: &Path) -> Result<(PathBuf, PathBuf), String> {
    let python_env_dir = tools_path.join("python_env");
    if !python_env_dir.exists() {
        return Err(format!("ESP-IDF python_env not found at {}", python_env_dir.display()));
    }

    // Look for any idf*_py*_env directory
    let entries = std::fs::read_dir(&python_env_dir)
        .map_err(|e| format!("Cannot read python_env dir: {}", e))?;

    for entry in entries {
        if let Ok(entry) = entry {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("idf") && name.contains("_py") && name.ends_with("_env") {
                let venv_path = entry.path();
                for python_bin in venv_python_candidates(&venv_path) {
                    if python_bin.exists() {
                        return Ok((python_bin, venv_path));
                    }
                }
            }
        }
    }

    Err(format!("No ESP-IDF Python venv found in {}", python_env_dir.display()))
}

/// Read the ESP-IDF version from version.txt to avoid git lookups.
fn read_idf_version(idf_path: &Path) -> String {
    let version_file = idf_path.join("version.txt");
    std::fs::read_to_string(&version_file)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn find_tool_bins(path: &Path, current_depth: u32, max_depth: u32, paths: &mut Vec<PathBuf>) {
    if current_depth > max_depth {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().unwrap_or_default().to_string_lossy();
                if name == "bin" {
                    paths.push(p.clone());
                }
                // Recursive search ลงไปใน sub-directory
                find_tool_bins(&p, current_depth + 1, max_depth, paths);
            }
        }
    }
}

/// Build a PATH string that includes ESP-IDF toolchain directories.
/// Also scans the user's D:\Espressif\tools for Ninja, CMake, and compilers.
fn build_idf_path(tools_path: &Path) -> OsString {
    let mut paths: Vec<PathBuf> = Vec::new();

    // แก้ไข: เอา mut ออกจากหน้า scan
    let scan = |tools_dir: &Path, paths: &mut Vec<PathBuf>| {
        // เรียกใช้ฟังก์ชัน find_tool_bins ที่เราเพิ่มเข้าไปข้างบน
        find_tool_bins(tools_dir, 0, 4, paths);
    };

    // 1. Scan the resolved tools_path
    // หมายเหตุ: เรียกใช้ find_tool_bins โดยตรงหรือผ่าน scan ก็ได้ 
    // ในที่นี้ผมปรับให้เรียกตามโครงสร้างเดิมที่คุณวางไว้
    scan(&tools_path.join("tools"), &mut paths);
    scan(tools_path, &mut paths);

    // 2. Also check config-saved tools dir for Ninja/CMake
    let config = read_esp_idf_config();
    if let Some(custom_tools) = config["custom_tools_path"].as_str() {
        let custom_tools_dir = PathBuf::from(custom_tools);
        if custom_tools_dir.join("tools") != tools_path.join("tools") {
            scan(&custom_tools_dir.join("tools"), &mut paths);
        }
        if custom_tools_dir != tools_path {
            scan(&custom_tools_dir, &mut paths);
        }
    }

    // 3. Add venv Scripts/bin dirs (โค้ดส่วนเดิมของคุณ...)
    let python_env_dir = tools_path.join("python_env");
    if let Ok(entries) = std::fs::read_dir(&python_env_dir) {
        for entry in entries.flatten() {
            let bin = venv_bin_dir(&entry.path());
            if bin.exists() {
                paths.push(bin);
            }
        }
    }
    
    // ... (ส่วนที่เหลือคงเดิม) ...

    if let Some(system_path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system_path));
    }

    std::env::join_paths(paths).unwrap_or_else(|_| OsString::from(""))
}

#[tauri::command]
pub async fn check_esp_idf(app_handle: AppHandle) -> Result<String, String> {
    let (actual_idf_path, _) = resolve_idf_paths(&app_handle)?;
    Ok(format!("Ready: ESP-IDF found at {}", actual_idf_path.display()))
}

// ── Custom Path Commands (called from Setup/Repair button) ──────────────────

#[tauri::command]
pub async fn get_idf_custom_paths() -> Result<serde_json::Value, String> {
    let config = read_esp_idf_config();
    Ok(serde_json::json!({
        "idf_path": config["custom_idf_path"].as_str().unwrap_or(""),
        "tools_path": config["custom_tools_path"].as_str().unwrap_or("")
    }))
}

#[tauri::command]
pub async fn set_idf_custom_paths(idf_path: String, tools_path: String) -> Result<String, String> {
    // Validate paths before saving
    let idf = PathBuf::from(&idf_path);
    let tools = PathBuf::from(&tools_path);

    if !idf_path.is_empty() && !idf.join("tools/idf.py").exists() {
        return Err(format!(
            "Invalid ESP-IDF path: 'tools/idf.py' not found in '{}'",
            idf_path
        ));
    }
    if !tools_path.is_empty() && !tools.exists() {
        return Err(format!("Tools path does not exist: '{}'", tools_path));
    }

    let mut config = read_esp_idf_config();
    if let serde_json::Value::Object(ref mut map) = config {
        map.insert("custom_idf_path".to_string(), serde_json::json!(idf_path));
        map.insert("custom_tools_path".to_string(), serde_json::json!(tools_path));
    }
    write_esp_idf_config(&config);
    Ok(format!("ESP-IDF paths saved: {} | {}", idf_path, tools_path))
}

#[tauri::command]
pub async fn clear_idf_custom_paths() -> Result<(), String> {
    let mut config = read_esp_idf_config();
    if let serde_json::Value::Object(ref mut map) = config {
        map.remove("custom_idf_path");
        map.remove("custom_tools_path");
    }
    write_esp_idf_config(&config);
    Ok(())
}

#[tauri::command]
pub async fn setup_esp_idf(
    app_handle: AppHandle,
    version: Option<String>,
    targets: Option<Vec<String>>,
) -> Result<String, String> {
    let mut requested_version = version
        .unwrap_or_else(|| DEFAULT_ESP_IDF_VERSION.to_string())
        .trim()
        .to_string();
    if requested_version.is_empty() {
        requested_version = DEFAULT_ESP_IDF_VERSION.to_string();
    }
    if !requested_version.starts_with('v') {
        requested_version = format!("v{}", requested_version);
    }

    let target_list = targets
        .unwrap_or_else(|| vec!["all".to_string()])
        .into_iter()
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>();
    let targets_arg = if target_list.is_empty() {
        "all".to_string()
    } else {
        target_list.join(",")
    };

    let runtime_root = runtime_root_dir(&app_handle)?;
    std::fs::create_dir_all(&runtime_root)
        .map_err(|e| format!("Failed to create runtime directory: {}", e))?;

    let idf_path = runtime_root.join(format!("esp-idf-{}", requested_version));
    let tools_path = runtime_root.join(".espressif");

    if let Ok((idf_abs, tools_abs)) = canonical_idf_pair(&idf_path, &tools_path) {
        if find_idf_python(&tools_abs).is_ok() {
            return Ok(format!(
                "ESP-IDF already installed at {}",
                idf_abs.display()
            ));
        }
    }

    if !idf_path.exists() {
        let req_ver = requested_version.clone();
        let idf_p = idf_path.clone();
        let clone_output = tokio::task::spawn_blocking(move || {
            Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg("--branch")
                .arg(&req_ver)
                .arg(ESP_IDF_REPO_URL)
                .arg(&idf_p)
                .output()
                .map_err(|e| format!("Failed to run git clone: {}", e))
        }).await.map_err(|e| format!("Task panicked: {}", e))??;

        if !clone_output.status.success() {
            return Err(format!(
                "Failed to clone ESP-IDF {}: {}",
                requested_version,
                String::from_utf8_lossy(&clone_output.stderr)
            ));
        }
    }

    let python_cmd = detect_host_python()?;
    let idf_tools_py = idf_path.join("tools/idf_tools.py");
    if !idf_tools_py.exists() {
        return Err(format!(
            "Invalid ESP-IDF checkout at {} (missing tools/idf_tools.py)",
            idf_path.display()
        ));
    }

    let py_cmd = python_cmd.clone();
    let tools_py = idf_tools_py.clone();
    let targets = targets_arg.clone();
    let idf_p = idf_path.clone();
    let tools_p = tools_path.clone();

    let install_status = tokio::task::spawn_blocking(move || {
        Command::new(&py_cmd)
            .arg(&tools_py)
            .arg("install")
            .arg("--targets")
            .arg(&targets)
            .env("IDF_PATH", &idf_p)
            .env("IDF_TOOLS_PATH", &tools_p)
            .status()
            .map_err(|e| format!("Failed to run idf_tools.py install: {}", e))
    }).await.map_err(|e| format!("Task panicked: {}", e))??;
    if !install_status.success() {
        return Err("idf_tools.py install failed. Check network/proxy and rerun setup_esp_idf().".to_string());
    }

    let py_cmd = python_cmd.clone();
    let tools_py = idf_tools_py.clone();
    let idf_p = idf_path.clone();
    let tools_p = tools_path.clone();

    let pyenv_status = tokio::task::spawn_blocking(move || {
        Command::new(&py_cmd)
            .arg(&tools_py)
            .arg("install-python-env")
            .env("IDF_PATH", &idf_p)
            .env("IDF_TOOLS_PATH", &tools_p)
            .status()
            .map_err(|e| format!("Failed to run idf_tools.py install-python-env: {}", e))
    }).await.map_err(|e| format!("Task panicked: {}", e))??;
    if !pyenv_status.success() {
        return Err("idf_tools.py install-python-env failed. Check Python/pip access and rerun setup_esp_idf().".to_string());
    }

    let (idf_abs, tools_abs) = canonical_idf_pair(&idf_path, &tools_path)?;
    let _ = find_idf_python(&tools_abs)?;

    Ok(format!(
        "ESP-IDF {} installed at {} with targets {}",
        requested_version,
        idf_abs.display(),
        targets_arg
    ))
}

#[tauri::command]
pub async fn run_idf_command(
    app_handle: AppHandle,
    command: String,
    args: Vec<String>
) -> Result<String, String> {
    let (actual_idf_path, actual_tools_path) = resolve_idf_paths(&app_handle)?;
    let (python_bin, python_env_path) = find_idf_python(&actual_tools_path)?;
    let path_env = build_idf_path(&actual_tools_path);
    let idf_version = read_idf_version(&actual_idf_path);

    let output = Command::new(&python_bin)
        .arg(actual_idf_path.join("tools/idf.py"))
        .arg(&command)
        .args(&args)
        .env("IDF_PATH", &actual_idf_path)
        .env("IDF_TOOLS_PATH", &actual_tools_path)
        .env("IDF_PYTHON_ENV_PATH", &python_env_path)
        .env("ESP_IDF_VERSION", &idf_version)
        .env("PATH", &path_env)
        .output()
        .map_err(|e| format!("Failed to execute idf.py: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
pub async fn create_idf_project(
    _app_handle: AppHandle,
    path: String,
    name: String,
) -> Result<String, String> {
    let project_path = PathBuf::from(&path).join(&name);
    println!("DEBUG: Creating project at path: {}", project_path.display());
    
    // Create project directory and main subdirectory
    std::fs::create_dir_all(project_path.join("main"))
        .map_err(|e| {
            let err = format!("Failed to create project directory: {}", e);
            println!("DEBUG ERROR: {}", err);
            err
        })?;

    // 1. Root CMakeLists.txt
    let root_cmake = format!(
"cmake_minimum_required(VERSION 3.16)

include($ENV{{IDF_PATH}}/tools/cmake/project.cmake)
project({})
", name);
    std::fs::write(project_path.join("CMakeLists.txt"), root_cmake)
        .map_err(|e| {
            let err = format!("Failed to write Root CMakeLists.txt: {}", e);
            println!("DEBUG ERROR: {}", err);
            err
        })?;

    // 2. main/CMakeLists.txt
    let main_cmake = r#"idf_component_register(SRCS "main.c"
                       INCLUDE_DIRS ".")
"#;
    std::fs::write(project_path.join("main/CMakeLists.txt"), main_cmake)
        .map_err(|e| {
            let err = format!("Failed to write main/CMakeLists.txt: {}", e);
            println!("DEBUG ERROR: {}", err);
            err
        })?;

    // 3. main/main.c
    let main_c = r#"#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

void app_main(void) {
    printf("Hello from vibeKidbright!\n");
    while (1) {
        printf("Heartbeat...\n");
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}
"#;
    std::fs::write(project_path.join("main/main.c"), main_c)
        .map_err(|e| {
            let err = format!("Failed to write main/main.c: {}", e);
            println!("DEBUG ERROR: {}", err);
            err
        })?;

    println!("DEBUG: Project created successfully at {}", project_path.display());
    Ok(format!("Project '{}' created successfully at {}", name, project_path.display()))
}




#[derive(serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[tauri::command]
pub async fn list_project_files(path: String) -> Result<Vec<FileEntry>, String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    
    fn read_dir_recursive(dir: &Path, depth: usize) -> Result<Vec<FileEntry>, String> {
        if depth > 8 {
            return Ok(vec![]);
        }
        let mut entries = Vec::new();
        if let Ok(read_entries) = std::fs::read_dir(dir) {
            for entry in read_entries.flatten() {
                let path = entry.path();
                let name = path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                
                // Skip hidden files and build artifacts
                if name.starts_with('.') || name == "build" || name == "target" || name == "node_modules" {
                    continue;
                }

                let is_dir = path.is_dir();
                let children = if is_dir {
                    Some(read_dir_recursive(&path, depth + 1)?)
                } else {
                    None
                };

                entries.push(FileEntry {
                    name,
                    path: path.to_string_lossy().to_string(),
                    is_dir,
                    children,
                });
            }
        }
        
        // Sort: directories first, then files alphabetically
        entries.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(entries)
    }

    read_dir_recursive(&root, 0)
}

#[tauri::command]
pub async fn read_project_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_project_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn safe_write_project_file(path: String, content: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File does not exist: {}", path));
    }
    if !path_buf.is_file() {
        return Err(format!("Path is not a file: {}", path));
    }
    
    std::fs::write(path, content)
        .map_err(|e| format!("Failed to write file safely: {}", e))
}

#[tauri::command]
pub async fn create_directory(path: String) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Failed to create directory: {}", e))
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if path_buf.is_dir() {
        return Err("Target is a directory, not a file".to_string());
    }
    std::fs::remove_file(path)
        .map_err(|e| format!("Failed to delete file: {}", e))
}

#[tauri::command]
pub async fn delete_directory(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err("Target is not a directory".to_string());
    }
    std::fs::remove_dir_all(path)
        .map_err(|e| format!("Failed to delete directory: {}", e))
}

#[tauri::command]
pub async fn rename_item(old_path: String, new_path: String) -> Result<(), String> {
    std::fs::rename(old_path, new_path)
        .map_err(|e| format!("Failed to rename: {}", e))
}

#[tauri::command]
pub async fn validate_idf_project(path: String) -> Result<bool, String> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Ok(false);
    }
    
    // Simple check: ROOT CMakeLists.txt must exist
    let cmake_exists = root.join("CMakeLists.txt").exists();
    Ok(cmake_exists)
}

#[tauri::command]
pub async fn run_shell_command(
    app_handle: AppHandle,
    cmd: String,
    args: Vec<String>,
    cwd: Option<String>,
) -> Result<(), String> {
    let (actual_idf_path, actual_tools_path) = resolve_idf_paths(&app_handle)?;
    let (python_bin, python_env_path) = find_idf_python(&actual_tools_path)?;
    let path_env = build_idf_path(&actual_tools_path);
    let idf_version = read_idf_version(&actual_idf_path);

    let mut command = if cmd == "idf.py" {
        let mut c = Command::new(&python_bin);
        c.arg(actual_idf_path.join("tools/idf.py"));
        c
    } else {
        Command::new(&cmd)
    };

    if let Some(c_dir) = cwd {
        if !c_dir.is_empty() && c_dir != "." {
            command.current_dir(c_dir);
        }
    }

    command.args(&args)

        .env("IDF_PATH", &actual_idf_path)
        .env("IDF_TOOLS_PATH", &actual_tools_path)
        .env("IDF_PYTHON_ENV_PATH", &python_env_path)
        .env("ESP_IDF_VERSION", &idf_version)
        .env("PATH", &path_env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to open stderr")?;

    let app_handle_clone = app_handle.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = app_handle_clone.emit("terminal-output", l);
            }
        }
    });

    let app_handle_clone_err = app_handle;
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = app_handle_clone_err.emit("terminal-output", format!("\x1b[31m{}\x1b[0m", l));
            }
        }
    });

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

#[tauri::command]
pub async fn pick_directory() -> Result<String, String> {
    use rfd::FileDialog;
    let path = FileDialog::new()
        .pick_folder()
        .ok_or("No directory selected")?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn save_project_as(source_dir: String) -> Result<String, String> {
    use rfd::FileDialog;

    let source = PathBuf::from(&source_dir);
    if !source.exists() || !source.is_dir() {
        return Err(format!("Source project directory does not exist: {}", source_dir));
    }

    // Open folder picker for the destination parent directory
    let dest_parent = FileDialog::new()
        .set_title("Save Project As — Choose Destination Folder")
        .pick_folder()
        .ok_or("No destination directory selected")?;

    // Use the same project folder name at the destination
    let project_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project_copy".to_string());

    let dest = dest_parent.join(&project_name);

    if dest.exists() {
        return Err(format!(
            "A folder named '{}' already exists at '{}'. Please choose a different location or rename it first.",
            project_name,
            dest_parent.display()
        ));
    }

    // Recursively copy the project directory, skipping build artifacts
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<u64, String> {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;

        let mut file_count: u64 = 0;

        let entries = std::fs::read_dir(src)
            .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let src_path = entry.path();
            let name = src_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Skip build artifacts and hidden directories
            if name == "build"
                || name == "target"
                || name == "node_modules"
                || name == ".git"
                || name == "__pycache__"
            {
                continue;
            }

            let dst_path = dst.join(&name);

            if src_path.is_dir() {
                file_count += copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path).map_err(|e| {
                    format!(
                        "Failed to copy {} -> {}: {}",
                        src_path.display(),
                        dst_path.display(),
                        e
                    )
                })?;
                file_count += 1;
            }
        }

        Ok(file_count)
    }

    let src = source.clone();
    let dst = dest.clone();
    let file_count = tokio::task::spawn_blocking(move || {
        copy_dir_recursive(&src, &dst)
    }).await.map_err(|e| format!("Copy task panicked: {}", e))??;

    Ok(format!(
        "{}|{}",
        dest.to_string_lossy(),
        file_count
    ))
}

struct SerialMonitorState {
    stop: Arc<AtomicBool>,
    tx: Sender<String>,
}

fn get_cached_config() -> &'static Mutex<Option<serde_json::Value>> {
    CACHED_ESP_IDF_CONFIG.get_or_init(|| Mutex::new(None))
}

fn serial_monitor_store() -> &'static Mutex<Option<SerialMonitorState>> {
    static SERIAL_MONITOR: OnceLock<Mutex<Option<SerialMonitorState>>> = OnceLock::new();
    SERIAL_MONITOR.get_or_init(|| Mutex::new(None))
}

fn stop_serial_monitor_internal() {
    if let Ok(mut guard) = serial_monitor_store().lock() {
        if let Some(state) = guard.take() {
            state.stop.store(true, Ordering::SeqCst);
        }
    }
}

#[tauri::command]
pub async fn list_serial_ports() -> Result<Vec<String>, String> {
    let ports = serialport::available_ports()
        .map_err(|e| format!("Failed to list serial ports: {}", e))?
        .into_iter()
        .map(|p| p.port_name)
        .collect();
    Ok(ports)
}

#[tauri::command]
pub async fn start_serial_monitor(
    app_handle: AppHandle,
    port: String,
    baud_rate: Option<u32>,
) -> Result<String, String> {
    stop_serial_monitor_internal();

    let baud = baud_rate.unwrap_or(115_200);
    let mut serial = serialport::new(&port, baud)
        .timeout(Duration::from_millis(100))
        .open()
        .map_err(|e| format!("Failed to open serial port {}: {}", port, e))?;

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<String>();

    {
        let mut guard = serial_monitor_store()
            .lock()
            .map_err(|_| "Serial monitor lock poisoned".to_string())?;
        *guard = Some(SerialMonitorState {
            stop: Arc::clone(&stop),
            tx: tx.clone(),
        });
    }

    let app = app_handle;
    let port_name = port.clone();
    std::thread::spawn(move || {
        let mut buf = [0_u8; 1024];
        while !stop.load(Ordering::SeqCst) {
            loop {
                match rx.try_recv() {
                    Ok(out) => {
                        if let Err(e) = std::io::Write::write_all(&mut *serial, out.as_bytes()) {
                            let _ = app.emit(
                                "terminal-output",
                                format!("\x1b[31m[SERIAL {} TX ERROR] {}\x1b[0m", port_name, e),
                            );
                            break;
                        }
                        let _ = std::io::Write::flush(&mut *serial);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }

            match serial.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app.emit("terminal-output", format!("[SERIAL {}] {}", port_name, chunk));
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    let _ = app.emit(
                        "terminal-output",
                        format!("\x1b[31m[SERIAL {} ERROR] {}\x1b[0m", port_name, e),
                    );
                    break;
                }
            }
        }
    });

    Ok(format!("Serial monitor connected: {} @ {}", port, baud))
}

#[tauri::command]
pub async fn send_serial_input(input: String) -> Result<(), String> {
    let tx = {
        let guard = serial_monitor_store()
            .lock()
            .map_err(|_| "Serial monitor lock poisoned".to_string())?;
        let Some(state) = guard.as_ref() else {
            return Err("Serial monitor is not connected".to_string());
        };
        state.tx.clone()
    };

    tx.send(input)
        .map_err(|e| format!("Failed to queue serial output: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn stop_serial_monitor() -> Result<String, String> {
    stop_serial_monitor_internal();
    Ok("Serial monitor disconnected".to_string())
}
