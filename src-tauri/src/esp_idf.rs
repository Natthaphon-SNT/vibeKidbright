use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Windows: ซ่อน console window ที่โผล่ขึ้นมาระหว่าง idf.py/cmake/ninja
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn apply_no_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
}


#[derive(Clone, serde::Serialize, Debug, PartialEq, Eq)]
pub struct CommandOutcome {
    pub success: bool,
    pub code: Option<i32>,
}

#[cfg(test)]
fn wait_for_child(mut child: std::process::Child) -> Result<CommandOutcome, String> {
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for command: {}", e))?;
    let outcome = CommandOutcome {
        success: status.success(),
        code: status.code(),
    };
    if outcome.success {
        Ok(outcome)
    } else {
        Err(format!(
            "Command failed with exit code {}",
            outcome
                .code
                .map_or_else(|| "unknown".to_string(), |code| code.to_string())
        ))
    }
}

struct StreamedCommandOutput {
    outcome: CommandOutcome,
    stdout: String,
}

type ProcessLineEmitter = Arc<dyn Fn(String, bool) + Send + Sync>;

fn captured_outcome(output:crate::sdk_process::Captured)->Result<StreamedCommandOutput,String> {
    if !output.success {
        return Err(format!("Command failed with exit code {}\n{}{}",output.code.map(|v|v.to_string()).unwrap_or_else(||"unknown".into()),output.stdout,output.stderr));
    }
    Ok(StreamedCommandOutput{outcome:CommandOutcome{success:output.success,code:output.code},stdout:format!("{}{}",output.stdout,output.stderr)})
}
#[cfg(test)]
fn execute_streamed_command(command:Command,emit:ProcessLineEmitter)->Result<StreamedCommandOutput,String> {
    captured_outcome(crate::sdk_process::execute(command,emit,Arc::new(AtomicBool::new(false)),Duration::from_secs(30))?)
}
async fn stream_command_with_lease(command:Command,app_handle:AppHandle,lease:Arc<crate::sdk_process::WorkspaceLease>)->Result<StreamedCommandOutput,String> {
    let timeout=Duration::from_secs(crate::sdk_process::get_sdk_execution_config()?.timeout_seconds);
    let cancel=lease.cancel.clone();
    let _cancel_on_drop=crate::sdk_process::CancelOnDrop(cancel.clone());
    let emit:ProcessLineEmitter=Arc::new(move|line,is_error|{
        let line=if is_error{format!("\x1b[31m{}\x1b[0m",line)}else{line};
        let _=app_handle.emit("terminal-output",line);
    });
    tauri::async_runtime::spawn_blocking(move||{
        let _lease=lease;
        captured_outcome(crate::sdk_process::execute(command,emit,cancel,timeout)?)
    }).await.map_err(|error|format!("Command worker failed: {error}"))?
}

const DEFAULT_ESP_IDF_VERSION: &str = "v5.5.1";
const ESP_IDF_REPO_URL: &str = "https://github.com/espressif/esp-idf.git";

// ── Persistent Config for custom paths ───────────────────────────────
// Stored in: %APPDATA%/vibeKidbright/config.json

fn read_esp_idf_config() -> serde_json::Value { crate::ai::config::read_config() }

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

    // Validate that paths actually exist on THIS machine.
    // If they don't exist (for example, after copying settings from another machine),
    // auto-clear the stale config so the installer toolchain is used instead.
    if !idf_path.exists() || !tools_path.exists() {
        eprintln!(
            "[ESP-IDF] Stale custom paths detected (not found on this machine): idf={} tools={} — clearing.",
            idf, tools
        );
        let mut stale = read_esp_idf_config();
        if let serde_json::Value::Object(ref mut map) = stale {
            map.remove("custom_idf_path");
            map.remove("custom_tools_path");
        }
        let _ = crate::ai::config::update_config(|config| {
            config.as_object_mut().unwrap().remove("custom_idf_path");
            config.as_object_mut().unwrap().remove("custom_tools_path");
        });
        return None;
    }

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
        return Err(format!(
            "ESP-IDF tools path missing at {}",
            tools_path.display()
        ));
    }

    let idf_abs = strip_unc_prefix(
        idf_path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize IDF path: {}", e))?,
    );
    let tools_abs = strip_unc_prefix(
        tools_path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize tools path: {}", e))?,
    );
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
/// Priority: env override > user config > Happy Meal toolchain > runtime > bundled.
pub(crate) fn resolve_idf_paths(app_handle: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
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

    // 3. Happy Meal toolchain (ดาวน์โหลดจาก GitHub Release)
    //    โครงสร้างจริง: toolchain/frameworks/esp-idf-vX.X.X/  +  toolchain/tools/
    if let Ok(app_data) = app_handle.path().app_data_dir() {
        let toolchain_dir = app_data.join("toolchain");
        let frameworks_dir = toolchain_dir.join("frameworks");
        let tools_dir = toolchain_dir.join("tools");

        if frameworks_dir.exists() && tools_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&frameworks_dir) {
                let mut idf_dirs: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_dir()
                            && p.file_name()
                                .map(|n| n.to_string_lossy().starts_with("esp-idf"))
                                .unwrap_or(false)
                            && p.join("tools/idf.py").exists()
                    })
                    .collect();
                idf_dirs.sort();
                if let Some(idf_dir) = idf_dirs.pop() {
                    if let Ok(paths) = canonical_idf_pair(&idf_dir, &tools_dir) {
                        return Ok(paths);
                    }
                }
            }
        }
    }

    // 4. Runtime ESP-IDF (ติดตั้งผ่าน setup_esp_idf command)
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

    let resource_path = app_handle
        .path()
        .resource_dir()
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
        return Err(format!(
            "ESP-IDF python_env not found at {}",
            python_env_dir.display()
        ));
    }

    // Look for any idf*_py*_env directory
    let entries = std::fs::read_dir(&python_env_dir)
        .map_err(|e| format!("Cannot read python_env dir: {}", e))?;

    for entry in entries.flatten() {
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

    Err(format!(
        "No ESP-IDF Python venv found in {}",
        python_env_dir.display()
    ))
}

/// Read the ESP-IDF version from version.txt to avoid git lookups.
fn read_idf_version(idf_path: &Path) -> String {
    let version_file = idf_path.join("version.txt");
    std::fs::read_to_string(&version_file)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Read-only readiness for a configured/external SDK. A managed-install marker
/// is not required when the same resolver used by build found an existing SDK.
pub(crate) fn installed_sdk_status(idf: &Path, tools: &Path) -> Result<serde_json::Value,String> {
    let (idf,tools)=canonical_idf_pair(idf,tools)?;
    find_idf_python(&tools)?;
    let version=read_idf_version(&idf);
    Ok(serde_json::json!({"status":"ready","version":if version.is_empty(){None}else{Some(version.trim_start_matches('v').to_owned())},"path":idf,"tools_path":tools,"source":"configured-sdk"}))
}

/// Scan an ESP-IDF tools directory and add all executable directories to `paths`.
/// Handles two common structures:
///   1. <tool>/<version>/bin/<exe>        → adds bin/
///   2. <tool>/<version>/<exe>            → adds version/ directly  (ninja, idf-python, ...)
///   3. <tool>/<variant>/<version>/bin/   → adds bin/  (xtensa-esp-elf variants)
fn scan_esp_tool_dirs(tools_dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(tools) = std::fs::read_dir(tools_dir) else {
        return;
    };
    for tool_entry in tools.flatten() {
        let tool_path = tool_entry.path();
        if !tool_path.is_dir() {
            continue;
        }
        // Each sub-directory is a "tool name" (e.g. ninja, cmake, xtensa-esp-elf)
        let Ok(versions) = std::fs::read_dir(&tool_path) else {
            continue;
        };
        for ver_entry in versions.flatten() {
            let ver_path = ver_entry.path();
            if !ver_path.is_dir() {
                continue;
            }
            // Does version dir have a bin/ subdir?
            let bin_dir = ver_path.join("bin");
            if bin_dir.is_dir() {
                paths.push(bin_dir);
            }
            // Does version dir itself contain executable files? (ninja, python, ...)
            if dir_has_executables(&ver_path) {
                paths.push(ver_path.clone());
            }
            // Some tools have one more level: <tool>/<variant>/<version>/bin/
            let Ok(sub_entries) = std::fs::read_dir(&ver_path) else {
                continue;
            };
            for sub_entry in sub_entries.flatten() {
                let sub = sub_entry.path();
                if !sub.is_dir() {
                    continue;
                }
                let sub_bin = sub.join("bin");
                if sub_bin.is_dir() {
                    paths.push(sub_bin);
                }
                if dir_has_executables(&sub) {
                    paths.push(sub);
                }
            }
        }
    }
}

/// Returns true if a directory directly contains at least one executable file.
fn dir_has_executables(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if cfg!(windows) {
            if name.ends_with(".exe") {
                return true;
            }
        } else {
            // On Unix check execute bit
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = p.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Build a PATH string that includes ESP-IDF toolchain directories.
fn build_idf_path(tools_path: &Path) -> OsString {
    let mut paths: Vec<PathBuf> = Vec::new();

    // 1. Scan .espressif/tools/ — handles ninja, cmake, compilers, idf-python
    scan_esp_tool_dirs(&tools_path.join("tools"), &mut paths);
    // Also scan the tools_path root in case structure is flat
    scan_esp_tool_dirs(tools_path, &mut paths);

    // 2. Also check config-saved tools dir for extra tools (Ninja/CMake from IDF manager)
    let config = read_esp_idf_config();
    if let Some(custom_tools) = config["custom_tools_path"].as_str() {
        let custom_tools_dir = PathBuf::from(custom_tools);
        if custom_tools_dir.join("tools") != tools_path.join("tools") {
            scan_esp_tool_dirs(&custom_tools_dir.join("tools"), &mut paths);
        }
        if custom_tools_dir != tools_path {
            scan_esp_tool_dirs(&custom_tools_dir, &mut paths);
        }
    }

    // 3. Add venv Scripts/bin dirs
    let python_env_dir = tools_path.join("python_env");
    if let Ok(entries) = std::fs::read_dir(&python_env_dir) {
        for entry in entries.flatten() {
            let bin = venv_bin_dir(&entry.path());
            if bin.exists() {
                paths.push(bin);
            }
        }
    }

    // 4. System PATH (fallback)
    if let Some(system_path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system_path));
    }

    std::env::join_paths(paths).unwrap_or_else(|_| OsString::from(""))
}

pub fn find_esp_rom_elf_dir(tools_path: &Path) -> Option<String> {
    let elfs_dir = tools_path.join("esp-rom-elfs");
    if elfs_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&elfs_dir) {
            let mut subdirs: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            subdirs.sort();
            if let Some(dir) = subdirs.pop() {
                let mut path_str = dir.to_string_lossy().into_owned();
                if !path_str.ends_with('/') && !path_str.ends_with('\\') {
                    path_str.push(std::path::MAIN_SEPARATOR);
                }
                return Some(path_str);
            }
        }
    }
    let elfs_dir_nested = tools_path.join("tools").join("esp-rom-elfs");
    if elfs_dir_nested.exists() {
        if let Ok(entries) = std::fs::read_dir(&elfs_dir_nested) {
            let mut subdirs: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            subdirs.sort();
            if let Some(dir) = subdirs.pop() {
                let mut path_str = dir.to_string_lossy().into_owned();
                if !path_str.ends_with('/') && !path_str.ends_with('\\') {
                    path_str.push(std::path::MAIN_SEPARATOR);
                }
                return Some(path_str);
            }
        }
    }
    None
}

#[tauri::command]
pub async fn check_esp_idf(app_handle: AppHandle) -> Result<String, String> {
    let (actual_idf_path, _) = resolve_idf_paths(&app_handle)?;
    Ok(format!(
        "Ready: ESP-IDF found at {}",
        actual_idf_path.display()
    ))
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

    crate::ai::config::update_config(|config| {
        config["custom_idf_path"] = serde_json::json!(idf_path);
        config["custom_tools_path"] = serde_json::json!(tools_path);
    })?;
    Ok(format!(
        "ESP-IDF paths saved: {} | {}",
        idf_path, tools_path
    ))
}

#[tauri::command]
pub async fn clear_idf_custom_paths() -> Result<(), String> {
    crate::ai::config::update_config(|config| {
        config.as_object_mut().unwrap().remove("custom_idf_path");
        config.as_object_mut().unwrap().remove("custom_tools_path");
    })
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
        })
        .await
        .map_err(|e| format!("Task panicked: {}", e))??;

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
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))??;
    if !install_status.success() {
        return Err(
            "idf_tools.py install failed. Check network/proxy and rerun setup_esp_idf()."
                .to_string(),
        );
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
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))??;
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

/// Resolve every explicit IDF project directory against the process working
/// directory, so a -C argument cannot bypass a pending creation review.
fn idf_project_directories(args: &[String], working_dir: &Path) -> Vec<PathBuf> {
    let mut directories = vec![working_dir.to_path_buf()];
    let mut remaining = args.iter();
    while let Some(argument) = remaining.next() {
        if argument == "--" { break; }
        let value = if argument == "-C" || argument == "--project-dir" {
            remaining.next().map(String::as_str)
        } else {
            argument.strip_prefix("--project-dir=")
                .or_else(|| argument.strip_prefix("-C").filter(|value| !value.is_empty()))
        };
        if let Some(value) = value {
            let path = PathBuf::from(crate::ai_chat::normalize_project_dir(value));
            directories.push(if path.is_absolute() { path } else { working_dir.join(path) });
        }
    }
    directories
}

#[tauri::command]
pub async fn run_idf_command(
    app_handle: AppHandle,
    command: String,
    args: Vec<String>,
    project_dir: Option<String>,
) -> Result<String, String> {
    let working_dir = match project_dir.as_deref().filter(|path| !path.trim().is_empty()) {
        Some(path) => PathBuf::from(crate::ai_chat::normalize_project_dir(path)),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    let idf_args: Vec<_> = std::iter::once(command.clone()).chain(args.iter().cloned()).collect();
    let lease=Arc::new(crate::sdk_process::lease(&working_dir,true)?);
    let _cancel=crate::sdk_process::CancelOnDrop(lease.cancel.clone());
    for project in idf_project_directories(&idf_args, &working_dir) {
        ensure_project_reviewed(&project)?;
    }
    let (actual_idf_path, actual_tools_path) = resolve_idf_paths(&app_handle)?;
    let (python_bin, python_env_path) = find_idf_python(&actual_tools_path)?;
    let path_env = build_idf_path(&actual_tools_path);
    let idf_version = read_idf_version(&actual_idf_path);

    prepare_idf_build(&idf_args, &working_dir, &actual_idf_path, &actual_tools_path, &app_handle,lease.clone()).await?;

    let mut cmd = Command::new(&python_bin);
    cmd.arg(actual_idf_path.join("tools/idf.py"))
        .arg(&command)
        .args(&args)
        .current_dir(&working_dir)
        .env("IDF_PATH", &actual_idf_path)
        .env("IDF_TOOLS_PATH", &actual_tools_path)
        .env("IDF_PYTHON_ENV_PATH", &python_env_path)
        .env("ESP_IDF_VERSION", &idf_version)
        .env("PATH", &path_env)
        // Always set IDF_TARGET so cmake never has to guess from sdkconfig
        .env("IDF_TARGET", "esp32")
        // Force Python to use UTF-8 regardless of Windows system locale (cp874, cp932, etc.)
        // This fixes kconfgen UnicodeDecodeError on non-English Windows machines.
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0");
    if let Some(rom_elf_dir) = find_esp_rom_elf_dir(&actual_tools_path) {
        cmd.env("ESP_ROM_ELF_DIR", rom_elf_dir);
    }
    Ok(stream_command_with_lease(cmd, app_handle,lease).await?.stdout)
}

#[tauri::command]
pub async fn create_idf_project(
    _app_handle: AppHandle,
    path: String,
    name: String,
) -> Result<String, String> {
    let project_path = PathBuf::from(&path).join(&name);
    println!(
        "DEBUG: Creating project at path: {}",
        project_path.display()
    );

    // Create project directory and main subdirectory
    std::fs::create_dir_all(project_path.join("main")).map_err(|e| {
        let err = format!("Failed to create project directory: {}", e);
        println!("DEBUG ERROR: {}", err);
        err
    })?;

    // 1. Root CMakeLists.txt
    let root_cmake = format!(
        "cmake_minimum_required(VERSION 3.16)

include($ENV{{IDF_PATH}}/tools/cmake/project.cmake)
project({})
",
        name
    );
    std::fs::write(project_path.join("CMakeLists.txt"), root_cmake).map_err(|e| {
        let err = format!("Failed to write Root CMakeLists.txt: {}", e);
        println!("DEBUG ERROR: {}", err);
        err
    })?;

    // 2. main/CMakeLists.txt
    let main_cmake = r#"idf_component_register(
    SRCS
    # VIBEKIDBRIGHT_MANAGED_SOURCES_BEGIN
    "main.c"
    # VIBEKIDBRIGHT_MANAGED_SOURCES_END
    INCLUDE_DIRS ".")
"#;
    std::fs::write(project_path.join("main/CMakeLists.txt"), main_cmake).map_err(|e| {
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
    std::fs::write(project_path.join("main/main.c"), main_c).map_err(|e| {
        let err = format!("Failed to write main/main.c: {}", e);
        println!("DEBUG ERROR: {}", err);
        err
    })?;

    println!(
        "DEBUG: Project created successfully at {}",
        project_path.display()
    );
    Ok(format!(
        "Project '{}' created successfully at {}",
        name,
        project_path.display()
    ))
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
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let is_dir = path.is_dir();
                // Older AI-generated projects placed IDE metadata beside source
                // files. Hide those reserved root files while retaining genuine
                // project documents/configuration and identically named user files
                // inside subdirectories. New metadata lives in a hidden directory.
                let is_legacy_project_metadata = depth == 0 && !is_dir
                    && (name.eq_ignore_ascii_case("kidbright-project.json")
                        || name.eq_ignore_ascii_case("KIDBRIGHT_COMPATIBILITY.md"));

                // Skip hidden files and build artifacts
                if name.starts_with('.')
                    || name == "build"
                    || name == "target"
                    || name == "node_modules"
                    || is_legacy_project_metadata
                {
                    continue;
                }

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
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_project_file(path: String, content: String) -> Result<(), String> {
    let parent=Path::new(&path).parent().ok_or("File parent unavailable.")?;
    let _lease=crate::sdk_process::lease(parent,false)?;
    std::fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn create_project_file(path: String) -> Result<(), String> {
    // A learner's New File action must never truncate an existing program.
    std::fs::OpenOptions::new().write(true).create_new(true).open(path)
        .map(drop)
        .map_err(|error| format!("Could not create file (existing files are kept): {}", error))
}

const SOURCES_BEGIN: &str = "# VIBEKIDBRIGHT_MANAGED_SOURCES_BEGIN";
const SOURCES_END: &str = "# VIBEKIDBRIGHT_MANAGED_SOURCES_END";

fn is_program_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|value| value.to_str()), Some("c" | "cpp" | "cc" | "cxx" | "S"))
}

fn same_source_path(left: &str, right: &str) -> bool {
    if cfg!(windows) { left.eq_ignore_ascii_case(right) } else { left == right }
}

fn source_is_within(source: &str, item: &str) -> bool {
    same_source_path(source, item)
        || (source.as_bytes().get(item.len()) == Some(&b'/')
            && source.get(..item.len()).is_some_and(|prefix| same_source_path(prefix, item)))
}

fn safe_relative_program_path(path: &Path) -> Result<String, String> {
    use std::path::Component;
    let mut components = path.components();
    if !components.next().is_some_and(|component| matches!(component, Component::Normal(name)
        if name.to_str().is_some_and(|name| same_source_path(name, "main")))) {
        return Err("Program files must be inside the project's main directory.".into());
    }
    let mut names = Vec::new();
    for component in components {
        let Component::Normal(name) = component else {
            return Err("Program paths cannot contain traversal or absolute components.".into());
        };
        let name = name.to_str().ok_or("Program paths must be valid Unicode.")?;
        if name.is_empty() || name.starts_with('.') || name.ends_with(['.', ' '])
            || matches!(name, "build" | "target" | "node_modules")
            || name.chars().any(|character| character.is_control() || ";$\"\\#()[]:<>|?*".contains(character)) {
            return Err("This path cannot be safely registered as a program file.".into());
        }
        names.push(name);
    }
    if names.is_empty() { return Err("Choose an item below main, not main itself.".into()); }
    Ok(names.join("/"))
}

#[derive(Debug)]
struct CmakeToken { start: usize, end: usize, text: String, quoted: bool }

// Deliberately accept a single static component registration only. Arbitrary
// CMake programs need an explicit human/AI edit instead of an unsafe rewrite.
fn cmake_tokens(text: &str) -> Result<Vec<CmakeToken>, String> {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut tokens = Vec::new();
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() { index += 1; continue; }
        if bytes[index] == b'#' {
            if bytes.get(index + 1) == Some(&b'[') { return Err("Bracket comments require an explicit CMake edit.".into()); }
            while index < bytes.len() && bytes[index] != b'\n' { index += 1; }
            continue;
        }
        let start = index;
        let quoted = bytes[index] == b'"';
        if quoted {
            index += 1;
            let value_start = index;
            while index < bytes.len() && bytes[index] != b'"' {
                if bytes[index] == b'\\' { return Err("Escaped CMake arguments require an explicit CMake edit.".into()); }
                index += 1;
            }
            if index == bytes.len() { return Err("Unterminated CMake argument.".into()); }
            let value = text[value_start..index].to_string();
            index += 1;
            tokens.push(CmakeToken { start, end: index, text: value, quoted });
        } else {
            if matches!(bytes[index], b'(' | b')') { index += 1; }
            else {
                while index < bytes.len() && !bytes[index].is_ascii_whitespace()
                    && !matches!(bytes[index], b'(' | b')' | b'#' | b'"') { index += 1; }
            }
            tokens.push(CmakeToken { start, end: index, text: text[start..index].into(), quoted });
        }
    }
    Ok(tokens)
}

fn source_keyword(token: &CmakeToken) -> bool {
    !token.quoted && matches!(token.text.as_str(), "INCLUDE_DIRS" | "PRIV_INCLUDE_DIRS" | "REQUIRES"
        | "PRIV_REQUIRES" | "LDFRAGMENTS" | "EMBED_FILES" | "EMBED_TXTFILES" | "KCONFIG"
        | "KCONFIG_PROJBUILD" | "WHOLE_ARCHIVE" | "REQUIRED_IDF_TARGETS" | ")")
}

fn planned_cmake_sources(cmake: &str, added: &[String], removed: &[String]) -> Result<String, String> {
    let tokens = cmake_tokens(cmake)?;
    if tokens.len() < 4 || tokens[0].quoted || tokens[0].text != "idf_component_register"
        || tokens[1].quoted || tokens[1].text != "("
        || tokens.last().is_none_or(|token| token.quoted || token.text != ")")
        || tokens[2..tokens.len()-1].iter().any(|token| !token.quoted && matches!(token.text.as_str(), "(" | ")" | "SRC_DIRS" | "EXCLUDE_SRCS")) {
        return Err("Custom/dynamic main/CMakeLists.txt was kept. Register sources explicitly before retrying.".into());
    }
    let positions: Vec<_> = tokens.iter().enumerate().filter(|(_, token)| !token.quoted && token.text == "SRCS").map(|(index, _)| index).collect();
    if positions.len() != 1 { return Err("main/CMakeLists.txt needs one explicit static SRCS list.".into()); }
    let source_index = positions[0];
    let boundary = (source_index+1..tokens.len()).find(|&index| source_keyword(&tokens[index]))
        .ok_or("Could not locate the end of the CMake source list.")?;
    let mut sources = Vec::new();
    for token in &tokens[source_index+1..boundary] {
        let relative = safe_relative_program_path(&Path::new("main").join(&token.text))?;
        if !is_program_source(Path::new(&relative)) { return Err("The SRCS list contains a dynamic or unsupported source; CMake was kept.".into()); }
        if !removed.iter().any(|removed| source_is_within(&relative, removed)) {
            sources.push(relative);
        }
    }
    for source in added {
        if !sources.iter().any(|existing| same_source_path(existing, source)) { sources.push(source.clone()); }
    }
    sources.sort();
    sources.dedup();
    let start = tokens[source_index].end;
    let end = tokens[boundary].start;
    let old_source_area = &cmake[start..end];
    let begin_count = cmake.matches(SOURCES_BEGIN).count();
    let end_count = cmake.matches(SOURCES_END).count();
    if begin_count != end_count || begin_count > 1
        || (begin_count == 1 && (!old_source_area.contains(SOURCES_BEGIN) || !old_source_area.contains(SOURCES_END)
            || old_source_area.find(SOURCES_BEGIN) >= old_source_area.find(SOURCES_END))) {
        return Err("Managed CMake source markers are malformed; the file was kept.".into());
    }
    // Do not discard user comments when adopting an older, unmarked source list.
    if begin_count == 0 && old_source_area.contains('#') {
        return Err("Source-list comments require an explicit CMake edit; the file was kept.".into());
    }
    let newline = if cmake.contains("\r\n") { "\r\n" } else { "\n" };
    let mut area = format!("{newline}    {SOURCES_BEGIN}{newline}");
    for source in sources { area.push_str(&format!("    \"{source}\"{newline}")); }
    area.push_str(&format!("    {SOURCES_END}{newline}    "));
    Ok(format!("{}{}{}", &cmake[..start], area, &cmake[end..]))
}

/// Non-mutating companion for reviewed AI writes. Paths are project-relative;
/// removed directories also remove registered descendants. Other CMake arguments
/// and all content outside the static SRCS area are retained byte-for-byte.
pub(crate) fn source_list_plan(
    project_root: &Path,
    added: &[PathBuf],
    removed: &[PathBuf],
) -> Result<Option<(PathBuf, String)>, String> {
    source_list_plan_with_content(project_root, added, removed, None)
}

pub(crate) fn source_list_plan_with_content(
    project_root: &Path,
    added: &[PathBuf],
    removed: &[PathBuf],
    original_override: Option<&str>,
) -> Result<Option<(PathBuf, String)>, String> {
    let added: Vec<_> = added.iter().filter(|path| is_program_source(path))
        .map(|path| safe_relative_program_path(path)).collect::<Result<_, _>>()?;
    let removed: Vec<_> = removed.iter().map(|path| safe_relative_program_path(path)).collect::<Result<_, _>>()?;
    if added.is_empty() && removed.is_empty() { return Ok(None); }
    let root = checked_program_root(project_root)?;
    let cmake_path = root.join("main/CMakeLists.txt");
    reject_link(&cmake_path)?;
    let original = match original_override {
        Some(content) => content.to_string(),
        None => std::fs::read_to_string(&cmake_path).map_err(|error| format!("Could not read main/CMakeLists.txt: {}", error))?,
    };
    let next = planned_cmake_sources(&original, &added, &removed)?;
    Ok((next != original).then_some((cmake_path, next)))
}

fn reject_link(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| format!("Could not inspect {}: {}", path.display(), error))?;
    let mut linked = metadata.file_type().is_symlink();
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        linked |= metadata.file_attributes() & 0x400 != 0;
    }
    if linked { Err("Program operations cannot follow symbolic links or junctions.".into()) } else { Ok(()) }
}

fn checked_program_root(project_root: &Path) -> Result<PathBuf, String> {
    let root = dunce::canonicalize(project_root).map_err(|error| format!("Invalid project directory: {}", error))?;
    if !root.join("CMakeLists.txt").is_file() || !root.join("main").is_dir() {
        return Err("Choose an ESP-IDF project containing CMakeLists.txt and main.".into());
    }
    reject_link(&root.join("main"))?;
    Ok(root)
}

fn has_project_pending_diff(root: &Path, paths: impl IntoIterator<Item = PathBuf>) -> bool {
    let project = root.to_string_lossy().replace('\\', "/");
    paths.into_iter().any(|path| {
        let path = dunce::canonicalize(&path).unwrap_or(path);
        source_is_within(&path.to_string_lossy().replace('\\', "/"), project.trim_end_matches('/'))
    })
}

fn checked_program_operation_root(project_dir: &str) -> Result<PathBuf, String> {
    let root = checked_program_root(Path::new(&crate::ai_chat::normalize_project_dir(project_dir)))?;
    ensure_project_reviewed(&root)?;
    Ok(root)
}

pub(crate) fn ensure_project_reviewed(project: &Path) -> Result<(), String> {
    crate::ai::project_review::ensure_project_kept(project)?;
    let root = dunce::canonicalize(project).map_err(|error| format!("Could not resolve project directory: {}", error))?;
    let pending = crate::ai::change_journal::with(|journal|Ok(journal.pending_map()))?;
    if has_project_pending_diff(&root, pending.keys().cloned()) {
        return Err("Review and Keep or Undo pending AI file changes before building or changing program items.".into());
    }
    Ok(())
}

fn checked_program_item(root: &Path, supplied: &Path, must_exist: bool) -> Result<(PathBuf, PathBuf), String> {
    use std::path::Component;
    if supplied.components().any(|part| matches!(part, Component::ParentDir | Component::CurDir)) {
        return Err("Program paths cannot contain traversal.".into());
    }
    let raw = if supplied.is_absolute() { supplied.to_path_buf() } else { root.join(supplied) };
    let parent = raw.parent().ok_or("Missing program parent directory.")?;
    let parent = dunce::canonicalize(parent).map_err(|error| format!("Program parent directory does not exist: {}", error))?;
    let path = parent.join(raw.file_name().ok_or("Missing program filename.")?);
    let relative = path.strip_prefix(root).map_err(|_| "Program path is outside the project.")?.to_path_buf();
    let source = safe_relative_program_path(&relative)?;
    if source.eq_ignore_ascii_case("CMakeLists.txt") || source.eq_ignore_ascii_case("kb_board.h") {
        return Err("This file is managed by the IDE. Choose a program file.".into());
    }
    let mut original_chain = root.to_path_buf();
    // Inspect the original path as well as the resolved parent so links cannot
    // make a source operation escape or alias another part of the project.
    let mut raw_components = raw.components();
    for root_component in root.components() {
        let raw_component = raw_components.next().ok_or("Program path is outside the project.")?;
        if !same_source_path(&raw_component.as_os_str().to_string_lossy(), &root_component.as_os_str().to_string_lossy()) {
            return Err("Program path is outside the project.".into());
        }
    }
    let raw_relative = raw_components.as_path();
    for component in raw_relative.components() {
        original_chain.push(component);
        if original_chain.exists() { reject_link(&original_chain)?; }
    }
    if must_exist && !path.exists() { return Err("The selected program item no longer exists.".into()); }
    Ok((path, relative))
}

fn write_cmake_atomically(path: &Path, content: &str) -> Result<(), String> {
    static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let temp = path.with_file_name(format!(".vibekidbright-cmake-{}-{}", std::process::id(), NEXT_TEMP.fetch_add(1, Ordering::Relaxed)));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp).map_err(|error| error.to_string())?;
        file.write_all(content.as_bytes()).and_then(|_| file.sync_all()).map_err(|error| error.to_string())?;
        std::fs::set_permissions(&temp, std::fs::metadata(path).map_err(|error| error.to_string())?.permissions()).map_err(|error| error.to_string())?;
        drop(file);
        std::fs::rename(&temp, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        #[cfg(windows)]
        if let Ok(metadata) = std::fs::metadata(&temp) {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            let _ = std::fs::set_permissions(&temp, permissions);
        }
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|error| format!("Could not update CMake; program operation was not completed: {}", error))
}

fn apply_program_operation(
    plan: Option<(PathBuf, String)>,
    operation: impl FnOnce() -> std::io::Result<()>,
) -> Result<(), String> {
    let original = if let Some((path, content)) = &plan {
        let original = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        write_cmake_atomically(path, content)?;
        Some(original)
    } else { None };
    if let Err(error) = operation() {
        if let (Some((path, _)), Some(original)) = (&plan, original) {
            write_cmake_atomically(path, &original).map_err(|restore| format!("Program operation failed: {}. CMake recovery also failed: {}. Inspect main/CMakeLists.txt before building.", error, restore))?;
        }
        return Err(format!("Program operation failed: {}", error));
    }
    Ok(())
}

fn create_program_file_inner(project_root: &Path, supplied: &Path) -> Result<(), String> {
    let root = checked_program_root(project_root)?;
    let (path, relative) = checked_program_item(&root, supplied, false)?;
    if path.exists() { return Err("A file or folder with this name already exists; it was kept.".into()); }
    let plan = source_list_plan(&root, &[relative], &[])?;
    apply_program_operation(plan, || std::fs::OpenOptions::new().write(true).create_new(true).open(path).map(drop))
}

fn collect_program_sources(path: &Path, root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    reject_link(path)?;
    if path.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|error| error.to_string())? {
            collect_program_sources(&entry.map_err(|error| error.to_string())?.path(), root, output)?;
        }
    } else if is_program_source(path) {
        let relative = path.strip_prefix(root).map_err(|_| "Program source escaped the project.")?.to_path_buf();
        safe_relative_program_path(&relative)?;
        output.push(relative);
    }
    Ok(())
}

fn registered_sources_for_rename(root: &Path) -> Result<Vec<PathBuf>, String> {
    let cmake_path = root.join("main/CMakeLists.txt");
    reject_link(&cmake_path)?;
    let cmake = std::fs::read_to_string(cmake_path).map_err(|error| error.to_string())?;
    // Validate the static format before using its tokens. Existing unregistered
    // source files must remain unregistered during a folder rename.
    planned_cmake_sources(&cmake, &[], &[])?;
    let tokens = cmake_tokens(&cmake)?;
    let start = tokens.iter().position(|token| !token.quoted && token.text == "SRCS").unwrap() + 1;
    let end = (start..tokens.len()).find(|&index| source_keyword(&tokens[index])).unwrap();
    Ok(tokens[start..end].iter().map(|token| Path::new("main").join(&token.text)).collect())
}

fn rename_program_item_inner(project_root: &Path, old_path: &Path, new_path: &Path) -> Result<(), String> {
    let root = checked_program_root(project_root)?;
    let (old, old_relative) = checked_program_item(&root, old_path, true)?;
    let (new, new_relative) = checked_program_item(&root, new_path, false)?;
    if new.exists() { return Err("The destination already exists; both items were kept.".into()); }
    let mut sources = Vec::new();
    collect_program_sources(&old, &root, &mut sources)?;
    let plan = if old.is_file() && !is_program_source(&old) && is_program_source(&new) {
        source_list_plan(&root, &[new_relative], &[])?
    } else if !sources.is_empty() {
        let registered = registered_sources_for_rename(&root)?;
        let added: Vec<_> = sources.iter().filter(|source| registered.iter().any(|registered| same_source_path(
                &source.to_string_lossy().replace('\\', "/"), &registered.to_string_lossy().replace('\\', "/"))))
            .map(|source| new_relative.join(source.strip_prefix(&old_relative).unwrap())).collect();
        source_list_plan(&root, &added, &[old_relative])?
    } else { None };
    apply_program_operation(plan, || std::fs::rename(old, new))
}

fn delete_program_item_inner(project_root: &Path, supplied: &Path) -> Result<(), String> {
    let root = checked_program_root(project_root)?;
    let (path, relative) = checked_program_item(&root, supplied, true)?;
    let mut sources = Vec::new();
    collect_program_sources(&path, &root, &mut sources)?;
    let plan = if !sources.is_empty() { source_list_plan(&root, &[], &[relative])? } else { None };
    // The resolved target was verified below canonical root/main above. Rejecting
    // links throughout the tree also protects recursive directory deletion.
    apply_program_operation(plan, || {
        let result = if path.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) };
        result.map_err(|error| std::io::Error::new(error.kind(), format!("Removal may be incomplete; inspect the selected item before retrying: {}", error)))
    })
}

#[tauri::command]
pub async fn create_program_file(app_handle: AppHandle, project_dir: String, path: String) -> Result<(), String> {
    let _guard = crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root = checked_program_operation_root(&project_dir)?;
    let _lease=crate::sdk_process::lease(&root,false)?;
    create_program_file_inner(&root, Path::new(&path))?;
    emit_managed_cmake_changed(&app_handle, &root);
    Ok(())
}

#[tauri::command]
pub async fn create_program_directory(app_handle: AppHandle, project_dir: String, path: String) -> Result<(), String> {
    let _guard = crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root = checked_program_operation_root(&project_dir)?;
    let _lease=crate::sdk_process::lease(&root,false)?;
    let (path, _) = checked_program_item(&root, Path::new(&path), false)?;
    std::fs::create_dir(path).map_err(|error| format!("Could not create program directory (existing items are kept): {}", error))
}

#[tauri::command]
pub async fn rename_program_item(app_handle: AppHandle, project_dir: String, old_path: String, new_path: String) -> Result<(), String> {
    let _guard = crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root = checked_program_operation_root(&project_dir)?;
    let _lease=crate::sdk_process::lease(&root,false)?;
    rename_program_item_inner(&root, Path::new(&old_path), Path::new(&new_path))?;
    emit_managed_cmake_changed(&app_handle, &root);
    Ok(())
}

#[tauri::command]
pub async fn delete_program_item(app_handle: AppHandle, project_dir: String, path: String) -> Result<(), String> {
    let _guard = crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root = checked_program_operation_root(&project_dir)?;
    let _lease=crate::sdk_process::lease(&root,false)?;
    delete_program_item_inner(&root, Path::new(&path))?;
    emit_managed_cmake_changed(&app_handle, &root);
    Ok(())
}

fn emit_managed_cmake_changed(app: &AppHandle, root: &Path) {
    let _ = app.emit("file-modified", serde_json::json!({ "path": root.join("main/CMakeLists.txt").to_string_lossy() }).to_string());
}

#[tauri::command]
pub async fn safe_write_project_file(path: String, content: String) -> Result<(), String> {
    let _lease=crate::sdk_process::lease(Path::new(&path).parent().ok_or("Missing file parent.")?,false)?;
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("File does not exist: {}", path));
    }
    if !path_buf.is_file() {
        return Err(format!("Path is not a file: {}", path));
    }

    std::fs::write(path, content).map_err(|e| format!("Failed to write file safely: {}", e))
}

#[tauri::command]
pub async fn create_directory(path: String) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if path_buf.is_dir() {
        return Err("Target is a directory, not a file".to_string());
    }
    std::fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))
}

#[tauri::command]
pub async fn delete_directory(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err("Target is not a directory".to_string());
    }
    std::fs::remove_dir_all(path).map_err(|e| format!("Failed to delete directory: {}", e))
}

#[tauri::command]
pub async fn rename_item(old_path: String, new_path: String) -> Result<(), String> {
    std::fs::rename(old_path, new_path).map_err(|e| format!("Failed to rename: {}", e))
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

/// Check if a project's `build/` directory has stale CMake cache that will cause
/// cmake to fail (e.g. paths from another machine, or incomplete configuration).
///
/// Returns `true` if the build directory should be deleted and cmake reconfigured.
pub(crate) fn is_build_dir_stale(
    build_dir: &Path,
    current_idf: &Path,
    _current_tools: &Path,
) -> bool {
    let cmake_cache = build_dir.join("CMakeCache.txt");

    // Case 1: build/ exists but CMakeCache.txt is missing → incomplete state, clean it
    if !cmake_cache.exists() {
        // If there are other artifacts but no cache, it's likely a partial/corrupt build
        let has_artifacts = build_dir.join("build.ninja").exists()
            || build_dir.join("compile_commands.json").exists();
        return has_artifacts;
    }

    // Case 2: CMakeCache exists — check if IDF_PATH inside it matches current toolchain
    if let Ok(cache_content) = std::fs::read_to_string(&cmake_cache) {
        // Look for IDF_PATH:PATH= in the cache and compare to current
        let current_idf_canon =
            dunce::canonicalize(current_idf).unwrap_or_else(|_| current_idf.to_path_buf());
        let current_idf_str = current_idf_canon
            .to_string_lossy()
            .to_lowercase()
            .replace('\\', "/");

        for line in cache_content.lines() {
            if line.starts_with("IDF_PATH:") || line.starts_with("IDF_PATH=") {
                if let Some(val) = line.split('=').nth(1) {
                    let cached_path = val.trim().to_lowercase().replace('\\', "/");
                    if !cached_path.is_empty() && cached_path != current_idf_str {
                        eprintln!(
                            "[Build] Stale CMakeCache: cached IDF_PATH={} ≠ current={}",
                            cached_path, current_idf_str
                        );
                        return true;
                    }
                }
            }
            // Also check Python path — most common mismatch between machines
            if line.starts_with("PYTHON:") || line.starts_with("PYTHON=") {
                if let Some(val) = line.split('=').nth(1) {
                    let cached_python = val.trim();
                    if !cached_python.is_empty() && !Path::new(cached_python).exists() {
                        eprintln!(
                            "[Build] Stale CMakeCache: cached Python={} does not exist on this machine",
                            cached_python
                        );
                        return true;
                    }
                }
            }
        }
    }

    // NOTE: Do NOT treat a missing `managed_components_list.temp.cmake` as stale.
    // ESP-IDF itself deletes that file at the end of every successful configure
    // (tools/cmake/build.cmake: `file(REMOVE ${managed_components_list_file})`),
    // so checking for it wiped build/ after every good build and forced a full
    // rebuild (~all of ESP-IDF recompiled) on every Build & Flash.
    // An interrupted configure is handled fine by CMake re-running on its own.

    false
}

fn prepare_idf_build_directory(
    args: &[String],
    working_dir: &Path,
    idf_path: &Path,
    tools_path: &Path,
) -> Result<Option<PathBuf>, String> {
    let options: Vec<_> = args.iter().take_while(|argument| argument.as_str() != "--").collect();
    if !options.iter().any(|argument| matches!(argument.as_str(), "build" | "flash" | "reconfigure")) {
        return Ok(None);
    }
    // Custom build locations may be outside this project and are never removed
    // automatically. In that case the default build directory is unused too.
    if options.iter().any(|argument| argument.as_str() == "--build-dir"
        || argument.starts_with("--build-dir=") || argument.starts_with("-B")) {
        return Ok(None);
    }
    let projects = idf_project_directories(args, working_dir);
    let project = projects.last().ok_or("Missing IDF project directory.")?;
    let project = dunce::canonicalize(project).map_err(|error| format!("Could not resolve IDF project directory: {}", error))?;
    if !project.join("CMakeLists.txt").is_file() {
        return Ok(None);
    }
    let build = project.join("build");
    match std::fs::symlink_metadata(&build) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Could not inspect build directory: {}", error)),
        Ok(metadata) if !metadata.is_dir() => return Err("The project's build path is not a regular directory; it was kept.".into()),
        Ok(_) => (),
    }
    reject_link(&build)?;
    let resolved = dunce::canonicalize(&build).map_err(|error| format!("Could not resolve build directory: {}", error))?;
    if resolved.parent() != Some(project.as_path())
        || !resolved.file_name().is_some_and(|name| same_source_path(&name.to_string_lossy(), "build")) {
        return Err("Build cleanup target escaped the selected project; it was kept.".into());
    }
    if !is_build_dir_stale(&resolved, idf_path, tools_path) { return Ok(None); }
    // Preserve the verified build tree. A cancelled reconfigure must not leave
    // partially deleted build output or silently discard files users added there.
    let recovery=tempfile::Builder::new().prefix(".vibekidbright-stale-build-").tempdir_in(&project).map_err(|e|e.to_string())?.keep();
    std::fs::rename(&resolved,recovery.join("build")).map_err(|error|format!("Could not preserve stale project build directory: {error}"))?;
    Ok(Some(resolved))
}

async fn prepare_idf_build(
    args: &[String], working_dir: &Path, idf_path: &Path, tools_path: &Path, app: &AppHandle,
    lease:Arc<crate::sdk_process::WorkspaceLease>,
) -> Result<(), String> {
    let projects = idf_project_directories(args, working_dir);
    crate::ai::compatibility::validate_build_profile(projects.last().ok_or("Missing IDF project directory.")?)?;
    let (args, working_dir, idf_path, tools_path) = (args.to_vec(), working_dir.to_path_buf(), idf_path.to_path_buf(), tools_path.to_path_buf());
    let cleaned = tauri::async_runtime::spawn_blocking(move || {
        if lease.cancel.load(Ordering::SeqCst){return Err("SDK preparation cancelled.".into());}
        let result=prepare_idf_build_directory(&args, &working_dir, &idf_path, &tools_path);
        drop(lease);result
    })
        .await.map_err(|error| format!("Build preparation task failed: {}", error))??;
    if let Some(build) = cleaned {
        let _ = app.emit("terminal-output", format!("Preserved stale build directory {} in a .vibekidbright-stale-build recovery folder. CMake will reconfigure.", build.display()));
    }
    Ok(())
}

#[tauri::command]
pub async fn run_shell_command(
    app_handle: AppHandle,
    cmd: String,
    args: Vec<String>,
    cwd: Option<String>,
) -> Result<CommandOutcome, String> {
    let working_dir = match cwd.as_deref().filter(|path| !path.trim().is_empty()) {
        Some(path) => PathBuf::from(crate::ai_chat::normalize_project_dir(path)),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    crate::ai::project_review::ensure_project_kept(&working_dir)?;
    let lease=Arc::new(crate::sdk_process::lease(&working_dir,true)?);
    let _cancel=crate::sdk_process::CancelOnDrop(lease.cancel.clone());
    if cmd == "idf.py" {
        for project in idf_project_directories(&args, &working_dir) {
            ensure_project_reviewed(&project)?;
        }
    }
    let (actual_idf_path, actual_tools_path) = resolve_idf_paths(&app_handle)?;
    let (python_bin, python_env_path) = find_idf_python(&actual_tools_path)?;
    let path_env = build_idf_path(&actual_tools_path);
    let idf_version = read_idf_version(&actual_idf_path);

    if cmd == "idf.py" {
        prepare_idf_build(&args, &working_dir, &actual_idf_path, &actual_tools_path, &app_handle,lease.clone()).await?;
    }

    let mut command = if cmd == "idf.py" {
        let mut c = Command::new(&python_bin);
        c.arg(actual_idf_path.join("tools/idf.py"));
        c
    } else if cmd == "adb" || cmd == "adb.exe" {
        // Resolve ADB path (may not be in IDF PATH)
        let adb_path = find_adb().unwrap_or_else(|_| OsString::from(&cmd));
        Command::new(adb_path)
    } else {
        Command::new(&cmd)
    };

    if let Some(c_dir) = cwd {
        if !c_dir.is_empty() && c_dir != "." {
            command.current_dir(c_dir);
        }
    }

    command
        .args(&args)
        .env("IDF_PATH", &actual_idf_path)
        .env("IDF_TOOLS_PATH", &actual_tools_path)
        .env("IDF_PYTHON_ENV_PATH", &python_env_path)
        .env("ESP_IDF_VERSION", &idf_version)
        .env("PATH", &path_env)
        // Always set IDF_TARGET so cmake never has to guess from sdkconfig
        .env("IDF_TARGET", "esp32")
        // Force Python to use UTF-8 regardless of Windows system locale (cp874, cp932, etc.)
        // This fixes kconfgen UnicodeDecodeError on non-English Windows machines.
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0");

    if let Some(rom_elf_dir) = find_esp_rom_elf_dir(&actual_tools_path) {
        command.env("ESP_ROM_ELF_DIR", rom_elf_dir);
    }

    Ok(stream_command_with_lease(command, app_handle,lease).await?.outcome)
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
        return Err(format!(
            "Source project directory does not exist: {}",
            source_dir
        ));
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
    let file_count = tokio::task::spawn_blocking(move || copy_dir_recursive(&src, &dst))
        .await
        .map_err(|e| format!("Copy task panicked: {}", e))??;

    crate::ai::compatibility::hide_internal_metadata(&dest)
        .map_err(|error| format!("Project files were copied, but private metadata could not be hidden: {}", error))?;
    Ok(format!("{}|{}", dest.to_string_lossy(), file_count))
}

struct SerialMonitorState {
    stop: Arc<AtomicBool>,
    tx: Sender<String>,
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
                    let _ = app.emit(
                        "terminal-output",
                        format!("[SERIAL {}] {}", port_name, chunk),
                    );
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

// ── ADB Monitor (MiuAAiPlus board) ───────────────────────────────────────────

struct AdbMonitorState {
    stop: Arc<AtomicBool>,
    tx: Sender<String>,
}

fn adb_monitor_store() -> &'static Mutex<Option<AdbMonitorState>> {
    static ADB_MONITOR: OnceLock<Mutex<Option<AdbMonitorState>>> = OnceLock::new();
    ADB_MONITOR.get_or_init(|| Mutex::new(None))
}

fn stop_adb_monitor_internal() {
    if let Ok(mut guard) = adb_monitor_store().lock() {
        if let Some(state) = guard.take() {
            state.stop.store(true, Ordering::SeqCst);
        }
    }
}

/// Find the ADB executable, searching PATH and common installation locations.
fn find_adb() -> Result<OsString, String> {
    // 1. Try PATH first
    if let Ok(output) = Command::new("adb").arg("version").output() {
        if output.status.success() {
            return Ok(OsString::from("adb"));
        }
    }

    // 2. Search environment-provided and platform-standard SDK locations.
    let candidates: Vec<PathBuf> = {
        let mut paths = Vec::new();
        let executable = if cfg!(windows) { "adb.exe" } else { "adb" };

        for variable in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
            if let Ok(sdk_root) = std::env::var(variable) {
                paths.push(Path::new(&sdk_root).join("platform-tools").join(executable));
            }
        }

        #[cfg(target_os = "windows")]
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            // Winget installation path
            let winget_base = Path::new(&local_app_data)
                .join("Microsoft")
                .join("WinGet")
                .join("Packages");
            if winget_base.exists() {
                if let Ok(entries) = std::fs::read_dir(&winget_base) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with("Google.PlatformTools") {
                            let adb = entry.path().join("platform-tools").join(executable);
                            paths.push(adb);
                        }
                    }
                }
            }
            // Android SDK via Android Studio
            paths.push(
                Path::new(&local_app_data)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join(executable),
            );
        }

        #[cfg(target_os = "windows")]
        if let Ok(home) = std::env::var("USERPROFILE") {
            paths.push(
                Path::new(&home)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join(executable),
            );
        }

        #[cfg(target_os = "macos")]
        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                Path::new(&home)
                    .join("Library")
                    .join("Android")
                    .join("sdk")
                    .join("platform-tools")
                    .join(executable),
            );
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        if let Ok(home) = std::env::var("HOME") {
            paths.push(
                Path::new(&home)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join(executable),
            );
            paths.push(
                Path::new(&home)
                    .join("Android")
                    .join("sdk")
                    .join("platform-tools")
                    .join(executable),
            );
        }

        paths
    };

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.as_os_str().to_os_string());
        }
    }

    Err(
        "ADB not found. Install Android Platform Tools and add its platform-tools directory to PATH, or set ANDROID_HOME/ANDROID_SDK_ROOT."
            .to_string(),
    )
}

/// List ADB devices connected to the system.
/// Returns device serial IDs (e.g. "R5CT200XXXX", "emulator-5554").
#[tauri::command]
pub async fn list_adb_devices() -> Result<Vec<String>, String> {
    let adb = find_adb()?;
    let mut cmd = Command::new(&adb);
    cmd.args(["devices"]);
    apply_no_window(&mut cmd);

    let output = cmd.output().map_err(|e| {
        format!(
            "Failed to run 'adb devices': {}. Make sure ADB is installed and in PATH.",
            e
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("adb devices failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let devices: Vec<String> = stdout
        .lines()
        .skip(1) // skip "List of devices attached" header
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Format: "<serial>\t<state>" — only show "device" state (not offline/unauthorized)
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 && parts[1] == "device" {
                Some(parts[0].to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(devices)
}

/// Start ADB logcat monitor in a background thread.
/// Streams output via the "terminal-output" Tauri event.
#[tauri::command]
pub async fn start_adb_monitor(
    app_handle: AppHandle,
    device: Option<String>,
) -> Result<String, String> {
    // Stop any existing ADB monitor first
    stop_adb_monitor_internal();

    let adb = find_adb()?;
    let mut cmd = Command::new(&adb);
    if let Some(ref dev) = device {
        cmd.args(["-s", dev]);
    }
    cmd.args(["logcat"]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    apply_no_window(&mut cmd);

    let mut child = cmd.spawn().map_err(|e| {
        format!(
            "Failed to start adb logcat: {}. Make sure ADB is installed and in PATH.",
            e
        )
    })?;

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<String>();

    // Store monitor state
    {
        let mut guard = adb_monitor_store()
            .lock()
            .map_err(|_| "ADB monitor lock poisoned".to_string())?;
        *guard = Some(AdbMonitorState {
            stop: Arc::clone(&stop),
            tx: tx.clone(),
        });
    }

    let device_label = device.clone().unwrap_or_else(|| "default".to_string());
    let app = app_handle;

    // Read stdout from adb logcat in a background thread
    let stdout = child.stdout.take();
    let stop_clone = Arc::clone(&stop);
    let app_clone = app.clone();
    let device_label_clone = device_label.clone();

    std::thread::spawn(move || {
        if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if stop_clone.load(Ordering::SeqCst) {
                    break;
                }
                match line {
                    Ok(text) => {
                        let _ = app_clone.emit(
                            "terminal-output",
                            format!("[ADB {}] {}", device_label_clone, text),
                        );
                    }
                    Err(e) => {
                        let _ = app_clone.emit(
                            "terminal-output",
                            format!("\x1b[31m[ADB {} ERROR] {}\x1b[0m", device_label_clone, e),
                        );
                        break;
                    }
                }
            }
        }
        // Kill the child process when done
        let _ = child.kill();
        let _ = child.wait();
    });

    // Separate thread to handle input sent to ADB shell
    let stop_input = Arc::clone(&stop);
    let app_input = app.clone();
    std::thread::spawn(move || {
        while !stop_input.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(input) => {
                    // Send input via `adb shell <command>`
                    let adb_path = find_adb().unwrap_or_else(|_| OsString::from("adb"));
                    let mut shell_cmd = Command::new(&adb_path);
                    if let Some(ref dev) = device {
                        shell_cmd.args(["-s", dev]);
                    }
                    shell_cmd.args(["shell", &input]);
                    apply_no_window(&mut shell_cmd);

                    match shell_cmd.output() {
                        Ok(output) => {
                            let out = String::from_utf8_lossy(&output.stdout);
                            if !out.is_empty() {
                                let _ = app_input
                                    .emit("terminal-output", format!("[ADB SHELL] {}", out));
                            }
                            let err = String::from_utf8_lossy(&output.stderr);
                            if !err.is_empty() {
                                let _ = app_input.emit(
                                    "terminal-output",
                                    format!("\x1b[31m[ADB SHELL ERR] {}\x1b[0m", err),
                                );
                            }
                        }
                        Err(e) => {
                            let _ = app_input.emit(
                                "terminal-output",
                                format!("\x1b[31m[ADB SHELL ERROR] {}\x1b[0m", e),
                            );
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(format!("ADB monitor connected: {}", device_label))
}

/// Send input to the ADB monitor (executed as `adb shell <input>`)
#[tauri::command]
pub async fn send_adb_input(input: String) -> Result<(), String> {
    let tx = {
        let guard = adb_monitor_store()
            .lock()
            .map_err(|_| "ADB monitor lock poisoned".to_string())?;
        let Some(state) = guard.as_ref() else {
            return Err("ADB monitor is not connected".to_string());
        };
        state.tx.clone()
    };

    tx.send(input)
        .map_err(|e| format!("Failed to queue ADB input: {}", e))?;
    Ok(())
}

/// Stop the ADB monitor
#[tauri::command]
pub async fn stop_adb_monitor() -> Result<String, String> {
    stop_adb_monitor_internal();
    Ok("ADB monitor disconnected".to_string())
}

// ── Unit Tests ────────────────────────────────────────────────────────────────
// Pure-logic / filesystem tests — no Tauri AppHandle or network required.

#[cfg(test)]
mod tests {
    #[test]
    fn configured_sdk_readiness_requires_sdk_and_python_without_install_marker() {
        let scratch=tempfile::tempdir().unwrap();let idf=scratch.path().join("idf");let tools=scratch.path().join("tools");
        assert!(super::installed_sdk_status(&idf,&tools).is_err());
        std::fs::create_dir_all(idf.join("tools")).unwrap();std::fs::write(idf.join("tools/idf.py"),"# fixture").unwrap();std::fs::write(idf.join("version.txt"),"v5.5.4\n").unwrap();
        let venv=tools.join("python_env/idf5.5_py3.11_env");let python=super::venv_python_candidates(&venv)[0].clone();
        std::fs::create_dir_all(python.parent().unwrap()).unwrap();std::fs::write(&python,"fixture, never executed").unwrap();
        let status=super::installed_sdk_status(&idf,&tools).unwrap();assert_eq!(status["status"],"ready");assert_eq!(status["version"],"5.5.4");assert_eq!(status["source"],"configured-sdk");
        assert!(!scratch.path().join(".toolchain_ready").exists());
        std::fs::remove_file(python).unwrap();assert!(super::installed_sdk_status(&idf,&tools).is_err());
    }
    use super::*;
    use std::fs;
    use std::process::Command;

    fn managed_test_project(cmake: &str) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("main")).unwrap();
        fs::write(directory.path().join("CMakeLists.txt"), "project(test)").unwrap();
        fs::write(directory.path().join("main/CMakeLists.txt"), cmake).unwrap();
        fs::write(directory.path().join("main/main.c"), "void app_main(void) {}\n").unwrap();
        directory
    }

    #[test]
    fn managed_new_source_preserves_dependencies_and_never_overwrites() {
        let original = "# lesson configuration\nidf_component_register(SRCS \"main.c\" INCLUDE_DIRS \".\" \"include\" PRIV_REQUIRES driver esp_driver_i2c kb_reference)\n";
        let project = managed_test_project(original);
        create_program_file_inner(project.path(), Path::new("main/helper.cpp")).unwrap();
        let cmake = fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap();
        assert!(cmake.starts_with("# lesson configuration\nidf_component_register(SRCS"));
        assert!(cmake.ends_with("INCLUDE_DIRS \".\" \"include\" PRIV_REQUIRES driver esp_driver_i2c kb_reference)\n"));
        assert!(cmake.contains("\"helper.cpp\""));
        assert!(cmake.contains(SOURCES_BEGIN));
        fs::write(project.path().join("main/helper.cpp"), "learner work").unwrap();
        assert!(create_program_file_inner(project.path(), Path::new("main/helper.cpp")).is_err());
        assert_eq!(fs::read_to_string(project.path().join("main/helper.cpp")).unwrap(), "learner work");
        assert_eq!(fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap(), cmake);
    }

    #[test]
    fn source_plan_is_non_mutating_and_handles_each_compiler_source_extension() {
        let original = "idf_component_register(SRCS \"main.c\" INCLUDE_DIRS \".\" PRIV_REQUIRES driver)\r\n";
        let project = managed_test_project(original);
        fs::create_dir(project.path().join("main/tasks")).unwrap();
        let additions: Vec<_> = ["main/tasks/a.c", "main/tasks/b.cpp", "main/tasks/c.cc", "main/tasks/d.cxx", "main/tasks/e.S", "main/tasks/api.h"]
            .iter().map(PathBuf::from).collect();
        let (path, next) = source_list_plan(project.path(), &additions, &[]).unwrap().unwrap();
        assert_eq!(path, dunce::canonicalize(project.path()).unwrap().join("main/CMakeLists.txt"));
        for name in ["a.c", "b.cpp", "c.cc", "d.cxx", "e.S"] { assert!(next.contains(&format!("\"tasks/{}\"", name))); }
        assert!(!next.contains("api.h"));
        assert!(next.contains("\r\n"));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
        assert_eq!(planned_cmake_sources(&next, &[], &[]).unwrap(), next);
    }

    #[test]
    fn source_plan_preserves_pending_dependency_edits_without_touching_disk() {
        let original = "idf_component_register(SRCS main.c INCLUDE_DIRS . PRIV_REQUIRES driver)\n";
        let pending = "idf_component_register(SRCS main.c INCLUDE_DIRS . include PRIV_REQUIRES driver esp_wifi nvs_flash)\n";
        let project = managed_test_project(original);
        let (path, next) = source_list_plan_with_content(project.path(), &[PathBuf::from("main/helper.c")], &[], Some(pending)).unwrap().unwrap();
        assert!(next.contains("\"helper.c\""));
        assert!(next.contains("INCLUDE_DIRS . include PRIV_REQUIRES driver esp_wifi nvs_flash)"));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }

    #[test]
    fn managed_rename_and_delete_sync_nested_sources_without_adding_unregistered_files() {
        let project = managed_test_project("idf_component_register(SRCS \"main.c\" \"tasks/a.c\" INCLUDE_DIRS \".\" PRIV_REQUIRES driver)\n");
        fs::create_dir(project.path().join("main/tasks")).unwrap();
        fs::write(project.path().join("main/tasks/a.c"), "registered").unwrap();
        fs::write(project.path().join("main/tasks/draft.c"), "unregistered").unwrap();
        fs::write(project.path().join("main/tasks/helper.h"), "header").unwrap();
        rename_program_item_inner(project.path(), Path::new("main/tasks"), Path::new("main/lessons")).unwrap();
        let cmake = fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap();
        assert!(cmake.contains("\"lessons/a.c\""));
        assert!(!cmake.contains("\"tasks/a.c\""));
        assert!(!cmake.contains("draft.c"));
        assert!(project.path().join("main/lessons/draft.c").is_file());
        rename_program_item_inner(project.path(), Path::new("main/lessons/a.c"), Path::new("main/lessons/a.cc")).unwrap();
        assert!(fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap().contains("\"lessons/a.cc\""));
        delete_program_item_inner(project.path(), Path::new("main/lessons")).unwrap();
        assert!(!project.path().join("main/lessons").exists());
        let cmake = fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap();
        assert!(!cmake.contains("lessons/"));
        assert!(cmake.contains("\"main.c\""));
        assert!(cmake.contains("PRIV_REQUIRES driver"));
    }

    #[test]
    fn custom_cmake_is_retained_and_no_program_mutation_is_reported_as_success() {
        for custom in [
            "file(GLOB sources *.c)\nidf_component_register(SRCS ${sources} INCLUDE_DIRS .)\n",
            "idf_component_register(SRC_DIRS . INCLUDE_DIRS .)\n",
            "idf_component_register(SRCS main.c INCLUDE_DIRS .)\nset_property(TARGET x PROPERTY C_STANDARD 17)\n",
            "idf_component_register(SRCS ${sources} INCLUDE_DIRS .)\n",
        ] {
            let project = managed_test_project(custom);
            assert!(create_program_file_inner(project.path(), Path::new("main/helper.c")).is_err());
            assert!(!project.path().join("main/helper.c").exists());
            assert!(rename_program_item_inner(project.path(), Path::new("main/main.c"), Path::new("main/lesson.c")).is_err());
            assert!(delete_program_item_inner(project.path(), Path::new("main/main.c")).is_err());
            assert!(project.path().join("main/main.c").is_file());
            assert_eq!(fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap(), custom);
            // Headers have no source-list effect and need no CMake rewrite.
            create_program_file_inner(project.path(), Path::new("main/lesson.h")).unwrap();
        }
    }

    #[test]
    fn managed_operations_reject_outside_reserved_traversal_and_existing_destinations() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        for path in ["outside.c", "main/../outside.c", "main/CMakeLists.txt", "main/kb_board.h", "main/kb_board.h.", "main/kb_board.h ", "main/subdir./helper.c", "main", "main/name.c:stream", "main/missing/helper.c"] {
            assert!(create_program_file_inner(project.path(), Path::new(path)).is_err(), "accepted {}", path);
        }
        fs::write(project.path().join("main/keep.c"), "kept").unwrap();
        assert!(rename_program_item_inner(project.path(), Path::new("main/main.c"), Path::new("main/keep.c")).is_err());
        assert_eq!(fs::read_to_string(project.path().join("main/keep.c")).unwrap(), "kept");
        assert!(delete_program_item_inner(project.path(), Path::new("main")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn windows_protected_aliases_are_rejected_and_project_case_variants_work() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let header = project.path().join("main/kb_board.h");
        fs::write(&header, "protected board pin definitions").unwrap();
        for alias in ["main/kb_board.h.", "main/kb_board.h "] {
            assert!(delete_program_item_inner(project.path(), Path::new(alias)).is_err());
            assert!(rename_program_item_inner(project.path(), Path::new(alias), Path::new("main/renamed.h")).is_err());
            assert_eq!(fs::read_to_string(&header).unwrap(), "protected board pin definitions");
        }
        let root = dunce::canonicalize(project.path()).unwrap();
        let differently_cased = PathBuf::from(root.to_string_lossy().to_ascii_uppercase()).join("main/helper.c");
        create_program_file_inner(project.path(), &differently_cased).unwrap();
        assert!(root.join("main/helper.c").is_file());
        fs::rename(root.join("main"), root.join("Main")).unwrap();
        create_program_file_inner(project.path(), &root.join("Main/lesson.c")).unwrap();
        assert!(root.join("Main/lesson.c").is_file());
    }

    #[test]
    fn pending_diff_guard_matches_only_the_selected_project_and_windows_path_case() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let root = dunce::canonicalize(project.path()).unwrap();
        assert!(has_project_pending_diff(&root, [root.join("main/CMakeLists.txt")]));
        assert!(!has_project_pending_diff(&root, [root.with_file_name(format!("{}-other", root.file_name().unwrap().to_string_lossy())).join("main/main.c")]));
        #[cfg(windows)]
        assert!(has_project_pending_diff(&root, [PathBuf::from(root.join("main/new.c").to_string_lossy().to_ascii_uppercase())]));
    }

    #[test]
    fn pending_source_registration_must_be_reviewed_before_building_or_mutating() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let root = dunce::canonicalize(project.path()).unwrap();
        let cmake = root.join("main/CMakeLists.txt");
        let proposed=crate::ai::change_journal::with(|journal|journal.propose(&root,&cmake,"source-registration-test","pending SRCS/dependency proposal")).unwrap();
        let build_error = ensure_project_reviewed(&root).unwrap_err();
        let mutation_error = checked_program_operation_root(root.to_str().unwrap()).unwrap_err();
        crate::ai::change_journal::with(|journal|journal.reject(&cmake,&proposed.revision)).unwrap();
        assert!(build_error.contains("Keep or Undo"));
        assert!(mutation_error.contains("Keep or Undo"));
        assert_eq!(proposed.proposed, "pending SRCS/dependency proposal");
        ensure_project_reviewed(&root).unwrap();
    }

    #[test]
    fn failed_filesystem_operation_restores_original_cmake() {
        let original = "idf_component_register(SRCS main.c INCLUDE_DIRS . PRIV_REQUIRES driver)\n";
        let project = managed_test_project(original);
        let plan = source_list_plan(project.path(), &[PathBuf::from("main/helper.c")], &[]).unwrap();
        let error = apply_program_operation(plan, || Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "simulated operation failure"))).unwrap_err();
        assert!(error.contains("simulated operation failure"));
        assert_eq!(fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap(), original);
    }

    #[cfg(windows)]
    #[test]
    fn failed_cmake_replace_does_not_create_a_source_or_truncate_configuration() {
        let original = "idf_component_register(SRCS main.c INCLUDE_DIRS . PRIV_REQUIRES driver)\n";
        let project = managed_test_project(original);
        let cmake = project.path().join("main/CMakeLists.txt");
        let mut permissions = fs::metadata(&cmake).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&cmake, permissions).unwrap();
        assert!(create_program_file_inner(project.path(), Path::new("main/helper.c")).is_err());
        assert!(!project.path().join("main/helper.c").exists());
        assert_eq!(fs::read_to_string(&cmake).unwrap(), original);
        assert!(!fs::read_dir(project.path().join("main")).unwrap().any(|entry|
            entry.unwrap().file_name().to_string_lossy().starts_with(".vibekidbright-cmake-")));
        let mut permissions = fs::metadata(&cmake).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(cmake, permissions).unwrap();
    }

    #[test]
    fn renaming_header_to_source_adds_it_and_source_to_header_removes_it() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        fs::write(project.path().join("main/helper.h"), "learner work").unwrap();
        rename_program_item_inner(project.path(), Path::new("main/helper.h"), Path::new("main/helper.c")).unwrap();
        assert!(fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap().contains("\"helper.c\""));
        rename_program_item_inner(project.path(), Path::new("main/helper.c"), Path::new("main/helper.h")).unwrap();
        assert!(!fs::read_to_string(project.path().join("main/CMakeLists.txt")).unwrap().contains("helper.c"));
        assert_eq!(fs::read_to_string(project.path().join("main/helper.h")).unwrap(), "learner work");
    }

    fn diagnostic_command(code: i32) -> Command {
        let mut command = if cfg!(windows) { Command::new("cmd") } else { Command::new("sh") };
        if cfg!(windows) {
            command.args(["/C", &format!("echo compiler-out& echo compiler-error 1>&2& exit {}", code)]);
        } else {
            command.args(["-c", &format!("printf 'compiler-out\\n'; printf 'compiler-error\\n' >&2; exit {}", code)]);
        }
        command
    }

    #[test]
    fn streamed_process_joins_both_readers_before_success_and_reports_stdout_on_failure() {
        let lines = Arc::new(Mutex::new(Vec::new()));
        let captured = lines.clone();
        let emit: ProcessLineEmitter = Arc::new(move |line, error| captured.lock().unwrap().push((line, error)));
        let output = execute_streamed_command(diagnostic_command(0), emit.clone()).unwrap();
        assert_eq!(output.outcome.code, Some(0));
        assert!(output.stdout.contains("compiler-out"));
        let lines = lines.lock().unwrap().clone();
        assert!(lines.iter().any(|(line, error)| line.contains("compiler-out") && !error));
        assert!(lines.iter().any(|(line, error)| line.contains("compiler-error") && *error));
        let error = execute_streamed_command(diagnostic_command(2), emit).err().unwrap();
        assert!(error.contains("exit code 2"));
        assert!(error.contains("compiler-out"));
        assert!(error.contains("compiler-error"));
    }

    #[tokio::test]
    async fn new_project_file_does_not_overwrite_an_existing_program() {
        let directory = tempfile::tempdir().unwrap();
        let existing = directory.path().join("main.c");
        fs::write(&existing, "original learner program").unwrap();
        assert!(create_project_file(existing.to_string_lossy().into_owned()).await.is_err());
        assert_eq!(fs::read_to_string(&existing).unwrap(), "original learner program");
        let new = directory.path().join("lesson.c");
        create_project_file(new.to_string_lossy().into_owned()).await.unwrap();
        assert_eq!(fs::read(&new).unwrap(), Vec::<u8>::new());
    }

    fn project_tree_relative_paths(entries: &[FileEntry], root: &Path) -> Vec<String> {
        let mut paths = Vec::new();
        for entry in entries {
            paths.push(Path::new(&entry.path).strip_prefix(root).unwrap()
                .to_string_lossy().replace('\\', "/"));
            if let Some(children) = &entry.children {
                paths.extend(project_tree_relative_paths(children, root));
            }
        }
        paths
    }

    #[tokio::test]
    async fn project_tree_hides_only_reserved_root_metadata_and_retains_user_documents() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("main")).unwrap();
        fs::create_dir(root.path().join("docs")).unwrap();
        for relative in [
            "CMakeLists.txt", "sdkconfig", "sdkconfig.defaults", "main/main.c",
            "main/CMakeLists.txt", "main/idf_component.yml", "config.json",
            "README.md", "assets.yml", "docs/kidbright-project.json",
            "docs/KIDBRIGHT_COMPATIBILITY.md", "docs/lesson.md",
            "kidbright-project.json", "KIDBRIGHT_COMPATIBILITY.md",
        ] {
            fs::write(root.path().join(relative), "project content").unwrap();
        }
        let entries = list_project_files(root.path().to_string_lossy().into_owned()).await.unwrap();
        let paths = project_tree_relative_paths(&entries, root.path());
        for relative in [
            "CMakeLists.txt", "sdkconfig", "sdkconfig.defaults", "main/main.c",
            "main/CMakeLists.txt", "main/idf_component.yml", "config.json",
            "README.md", "assets.yml", "docs/kidbright-project.json",
            "docs/KIDBRIGHT_COMPATIBILITY.md", "docs/lesson.md",
        ] {
            assert!(paths.iter().any(|path| path == relative), "missing user file: {}", relative);
        }
        assert!(!paths.iter().any(|path| path == "kidbright-project.json"));
        assert!(!paths.iter().any(|path| path == "KIDBRIGHT_COMPATIBILITY.md"));
    }

    #[tokio::test]
    async fn project_tree_excludes_hidden_internal_state_and_build_outputs() {
        let root = tempfile::tempdir().unwrap();
        for relative in [
            ".vibekidbright", ".git", "build", "target", "node_modules",
            "main/.cache", "main/build", "components/driver/include",
            "KIDBRIGHT_COMPATIBILITY.md",
        ] {
            fs::create_dir_all(root.path().join(relative)).unwrap();
        }
        for relative in [
            ".vibekidbright/project.json", ".git/config", "build/project.bin",
            "target/output.bin", "node_modules/package.json", "main/.cache/state.json",
            "main/build/ninja.log", ".env", "components/driver/include/driver.h",
            "components/driver/driver.c", "components/driver/CMakeLists.txt",
            "KIDBRIGHT_COMPATIBILITY.md/lesson.c",
        ] {
            fs::write(root.path().join(relative), "project content").unwrap();
        }
        let entries = list_project_files(root.path().to_string_lossy().into_owned()).await.unwrap();
        let paths = project_tree_relative_paths(&entries, root.path());
        assert_eq!(paths.len(), 9, "unexpected visible tree: {:?}", paths);
        for relative in [
            "components", "components/driver", "components/driver/include",
            "components/driver/include/driver.h", "components/driver/driver.c",
            "components/driver/CMakeLists.txt", "main",
            "KIDBRIGHT_COMPATIBILITY.md", "KIDBRIGHT_COMPATIBILITY.md/lesson.c",
        ] {
            assert!(paths.iter().any(|path| path == relative), "missing user path: {}", relative);
        }
    }

    #[test]
    fn idf_project_flags_resolve_the_actual_target_before_process_launch() {
        let base = tempfile::tempdir().unwrap();
        let absolute = base.path().join("absolute project");
        let args = vec![
            "-C".into(), "relative project".into(),
            format!("--project-dir={}", absolute.display()),
            "-Cjoined".into(), "--project-dir".into(), "separate".into(),
        ];
        assert_eq!(idf_project_directories(&args, base.path()), vec![
            base.path().to_path_buf(), base.path().join("relative project"),
            absolute, base.path().join("joined"), base.path().join("separate"),
        ]);
    }

    #[test]
    fn idf_project_flags_stop_after_the_option_terminator() {
        let base = tempfile::tempdir().unwrap();
        let args = vec!["build".into(), "--".into(), "-C".into(), "not-an-option".into()];
        assert_eq!(idf_project_directories(&args, base.path()), vec![base.path().to_path_buf()]);
    }

    fn cached_test_build(project: &Path, cached_idf: &Path) -> PathBuf {
        let build = project.join("build");
        fs::create_dir(&build).unwrap();
        fs::write(build.join("CMakeCache.txt"), format!("IDF_PATH:PATH={}\n", cached_idf.display())).unwrap();
        fs::write(build.join("build.ninja"), "test build output").unwrap();
        build
    }

    #[test]
    fn idf_build_preparation_repairs_stale_cache_and_preserves_matching_cache() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let idf = project.path().join("installed-sdk");
        fs::create_dir(&idf).unwrap();
        let build = cached_test_build(project.path(), &project.path().join("old-sdk"));
        let args = vec!["-p".into(), "COM999".into(), "build".into(), "flash".into()];
        assert_eq!(prepare_idf_build_directory(&args, project.path(), &idf, project.path()).unwrap(), Some(dunce::canonicalize(project.path()).unwrap().join("build")));
        assert!(!build.exists());
        assert!(project.path().join("main/main.c").is_file());
        let build = cached_test_build(project.path(), &dunce::canonicalize(&idf).unwrap());
        assert!(prepare_idf_build_directory(&args, project.path(), &idf, project.path()).unwrap().is_none());
        assert!(build.join("build.ninja").is_file());
        assert!(project.path().join("main/main.c").is_file());
    }

    #[test]
    fn idf_build_preparation_uses_selected_project_and_keeps_other_directories() {
        let working = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let selected = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let unrelated = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let idf = working.path().join("installed-sdk");
        fs::create_dir(&idf).unwrap();
        let working_build = cached_test_build(working.path(), &working.path().join("old-sdk"));
        let selected_build = cached_test_build(selected.path(), &selected.path().join("old-sdk"));
        let unrelated_build = cached_test_build(unrelated.path(), &unrelated.path().join("old-sdk"));
        let args = vec!["-p".into(), "COM999".into(), "--project-dir".into(), selected.path().to_string_lossy().into_owned(), "build".into(), "flash".into()];
        assert!(prepare_idf_build_directory(&args, working.path(), &idf, working.path()).unwrap().is_some());
        assert!(!selected_build.exists());
        assert!(working_build.join("build.ninja").is_file());
        assert!(unrelated_build.join("build.ninja").is_file());
        assert!(selected.path().join("main/main.c").is_file());
        let args = vec!["build".into(), "-B".into(), unrelated_build.to_string_lossy().into_owned()];
        assert!(prepare_idf_build_directory(&args, working.path(), &idf, working.path()).unwrap().is_none());
        assert!(working_build.join("build.ninja").is_file());
        assert!(unrelated_build.join("build.ninja").is_file());
        let args = vec!["monitor".into(), "--".into(), "build".into()];
        assert!(prepare_idf_build_directory(&args, working.path(), &idf, working.path()).unwrap().is_none());
        assert!(working_build.join("build.ninja").is_file());
    }

    #[test]
    fn idf_build_preparation_repairs_incomplete_build_but_keeps_non_directory_target() {
        let project = managed_test_project("idf_component_register(SRCS main.c INCLUDE_DIRS .)\n");
        let build = project.path().join("build");
        fs::create_dir(&build).unwrap();
        fs::write(build.join("build.ninja"), "interrupted configuration").unwrap();
        let args = vec!["reconfigure".into()];
        assert!(prepare_idf_build_directory(&args, project.path(), project.path(), project.path()).unwrap().is_some());
        assert!(!build.exists());
        fs::write(&build, "user file").unwrap();
        assert!(prepare_idf_build_directory(&args, project.path(), project.path(), project.path()).is_err());
        assert_eq!(fs::read_to_string(build).unwrap(), "user file");
    }

    fn shell_exit(code: i32) -> Command {
        if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.args(["/C", &format!("exit {}", code)]);
            command
        } else {
            let mut command = Command::new("sh");
            command.args(["-c", &format!("exit {}", code)]);
            command
        }
    }

    #[test]
    fn command_exit_zero_returns_success_outcome() {
        let child = shell_exit(0).spawn().unwrap();
        assert_eq!(
            wait_for_child(child).unwrap(),
            CommandOutcome {
                success: true,
                code: Some(0)
            }
        );
    }

    #[test]
    fn command_nonzero_exit_reports_code() {
        let child = shell_exit(2).spawn().unwrap();
        let error = wait_for_child(child).unwrap_err();
        assert!(error.contains("exit code 2"), "unexpected error: {}", error);
    }

    #[test]
    fn missing_command_returns_spawn_error_immediately() {
        let error = Command::new("vibekidbright-command-that-does-not-exist")
            .spawn()
            .unwrap_err();
        assert_ne!(error.kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn venv_bin_dir_platform_layout() {
        let venv = Path::new("/venv");
        let bin = venv_bin_dir(venv);
        if cfg!(target_os = "windows") {
            assert_eq!(bin, PathBuf::from("/venv/Scripts"));
        } else {
            assert_eq!(bin, PathBuf::from("/venv/bin"));
        }
    }

    #[test]
    fn venv_python_candidates_are_python_named() {
        let cands = venv_python_candidates(Path::new("/venv"));
        assert!(!cands.is_empty(), "must produce at least one candidate");
        for c in &cands {
            assert!(
                c.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("python"),
                "unexpected candidate: {}",
                c.display()
            );
        }
    }

    #[test]
    fn read_idf_version_trims_and_missing_ok() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            read_idf_version(tmp.path()),
            "",
            "missing version.txt → empty string"
        );
        fs::write(tmp.path().join("version.txt"), "v5.2.1\n").unwrap();
        assert_eq!(read_idf_version(tmp.path()), "v5.2.1");
    }

    #[test]
    fn dir_has_executables_empty_dir_is_false() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!dir_has_executables(tmp.path()));
    }

    #[test]
    fn scan_esp_tool_dirs_finds_bin_dirs() {
        // Structure: <root>/ninja/1.11/bin/ninja(.exe)
        let tmp = tempfile::tempdir().unwrap();
        let bin_dir = tmp.path().join("ninja").join("1.11").join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let exe_name = if cfg!(windows) { "ninja.exe" } else { "ninja" };
        fs::write(bin_dir.join(exe_name), b"x").unwrap();

        // Non-existent directory → nothing added, no panic
        let mut paths = Vec::new();
        scan_esp_tool_dirs(&tmp.path().join("does-not-exist"), &mut paths);
        assert!(paths.is_empty());

        // Existing structure → bin dir discovered
        scan_esp_tool_dirs(tmp.path(), &mut paths);
        assert!(
            paths.iter().any(|p| p.ends_with("bin")),
            "bin dir must be discovered, got: {:?}",
            paths
        );
    }

    #[test]
    fn canonical_idf_pair_validates_idf_py_presence() {
        let tmp = tempfile::tempdir().unwrap();
        let idf = tmp.path().join("esp-idf");
        fs::create_dir_all(idf.join("tools")).unwrap();
        let tools = tmp.path().join(".espressif");
        fs::create_dir_all(&tools).unwrap();

        // Missing tools/idf.py → descriptive error
        let err = canonical_idf_pair(&idf, &tools).unwrap_err();
        assert!(err.contains("idf.py"), "unexpected error: {}", err);

        // With idf.py present → canonicalized pair returned
        fs::write(idf.join("tools").join("idf.py"), "# python entry point").unwrap();
        let (got_idf, got_tools) = canonical_idf_pair(&idf, &tools).unwrap();
        assert_eq!(got_idf.file_name(), Some(std::ffi::OsStr::new("esp-idf")));
        assert!(got_tools.exists());
    }
}
