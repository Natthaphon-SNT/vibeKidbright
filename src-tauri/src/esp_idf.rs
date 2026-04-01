use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager, Emitter};

const DEFAULT_ESP_IDF_VERSION: &str = "v5.2.2";
const ESP_IDF_REPO_URL: &str = "https://github.com/espressif/esp-idf.git";

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

    let idf_abs = idf_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize IDF path: {}", e))?;
    let tools_abs = tools_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize tools path: {}", e))?;
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
/// All paths are canonicalized to absolute paths for consistency.
fn resolve_idf_paths(app_handle: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    if let Some((idf_path, tools_path)) = resolve_env_override() {
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

/// Build a PATH string that includes ESP-IDF toolchain directories.
fn build_idf_path(tools_path: &Path) -> OsString {
    let mut paths: Vec<PathBuf> = Vec::new();

    // Add all tool bin directories (xtensa-esp-elf, openocd, etc.)
    let tools_dir = tools_path.join("tools");
    if let Ok(entries) = std::fs::read_dir(&tools_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                // Each tool may have version subdirs containing a bin/
                if let Ok(versions) = std::fs::read_dir(entry.path()) {
                    for ver in versions.flatten() {
                        let bin = ver.path().join("bin");
                        if bin.exists() {
                            paths.push(bin);
                        }
                        // Some tools put binaries directly in the version dir
                        let elf_bin = ver.path().join(entry.file_name().to_string_lossy().to_string());
                        let tool_bin = elf_bin.join("bin");
                        if tool_bin.exists() {
                            paths.push(tool_bin);
                        }
                    }
                }
            }
        }
    }

    // Add the venv bin dir
    let python_env_dir = tools_path.join("python_env");
    if let Ok(entries) = std::fs::read_dir(&python_env_dir) {
        for entry in entries.flatten() {
            let bin = venv_bin_dir(&entry.path());
            if bin.exists() {
                paths.push(bin);
            }
        }
    }

    // Append the current system PATH
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
        let clone_output = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--branch")
            .arg(&requested_version)
            .arg(ESP_IDF_REPO_URL)
            .arg(&idf_path)
            .output()
            .map_err(|e| format!("Failed to run git clone: {}", e))?;

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

    let install_status = Command::new(&python_cmd)
        .arg(&idf_tools_py)
        .arg("install")
        .arg("--targets")
        .arg(&targets_arg)
        .env("IDF_PATH", &idf_path)
        .env("IDF_TOOLS_PATH", &tools_path)
        .status()
        .map_err(|e| format!("Failed to run idf_tools.py install: {}", e))?;
    if !install_status.success() {
        return Err("idf_tools.py install failed. Check network/proxy and rerun setup_esp_idf().".to_string());
    }

    let pyenv_status = Command::new(&python_cmd)
        .arg(&idf_tools_py)
        .arg("install-python-env")
        .env("IDF_PATH", &idf_path)
        .env("IDF_TOOLS_PATH", &tools_path)
        .status()
        .map_err(|e| format!("Failed to run idf_tools.py install-python-env: {}", e))?;
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
    
    fn read_dir_recursive(dir: &Path) -> Result<Vec<FileEntry>, String> {
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
                    Some(read_dir_recursive(&path)?)
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

    read_dir_recursive(&root)
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
