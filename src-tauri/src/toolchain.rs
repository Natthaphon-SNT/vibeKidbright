//! Managed toolchain installation: trusted manifest -> private archive snapshot ->
//! SHA-256 -> guarded staging extraction -> SHA-256 recheck -> activation.
//! Legacy installation readiness does not certify integrity.

use std::io::{Read, Write};
use crate::toolchain_integrity::{Asset, Manifest, VerifiedArchive};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

// ── Constants ─────────────────────────────────────────────────────────────────

const SENTINEL_FILE: &str = ".toolchain_ready";

/// เวอร์ชัน toolchain
const TOOLCHAIN_VERSION: &str = "1.0.1";

// ── Global cancel flag ────────────────────────────────────────────────────────

static CANCEL_FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static INSTALL_IN_PROGRESS: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn cancel_flag() -> Arc<AtomicBool> {
    CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

#[derive(Debug)]
struct InstallGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for InstallGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

fn try_acquire_install(flag: Arc<AtomicBool>) -> Result<InstallGuard, String> {
    flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map(|_| InstallGuard { flag })
        .map_err(|_| "already_in_progress".to_string())
}

fn install_flag() -> Arc<AtomicBool> {
    INSTALL_IN_PROGRESS
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

fn activate_toolchain(
    staging_dir: &Path,
    final_dir: &Path,
    archives: &mut [VerifiedArchive],
    cancel: &AtomicBool,
) -> Result<(), String> {
    // Re-read every retained snapshot immediately before marking/activation.
    // Never delete a previous run's staging, cached ZIP or recovery directory.
    if archives.len() != crate::toolchain_integrity::PART_IDS.len() {
        return Err("Toolchain integrity: incomplete activation plan.".into());
    }
    for archive in archives { archive.verify(cancel)?; }
    if cancel.load(Ordering::SeqCst) { return Err("Toolchain installation cancelled before activation.".into()); }
    std::fs::write(staging_dir.join(SENTINEL_FILE), TOOLCHAIN_VERSION)
        .map_err(|e| format!("Cannot mark staged toolchain ready ({:?}).", e.kind()))?;
    let backup_dir = staging_dir.with_file_name("previous");
    if backup_dir.exists() { return Err("Toolchain recovery directory already exists; activation stopped.".into()); }

    let had_previous = final_dir.exists();
    if had_previous {
        std::fs::rename(final_dir, &backup_dir)
            .map_err(|e| format!("Cannot preserve previous toolchain ({:?}); activation stopped.", e.kind()))?;
    }

    if let Err(error) = std::fs::rename(staging_dir, final_dir) {
        if had_previous {
            std::fs::rename(&backup_dir, final_dir).map_err(|_| "Toolchain activation and rollback failed; the previous toolchain is preserved in this install session's recovery directory.".to_string())?;
        }
        return Err(format!(
            "Failed to activate downloaded toolchain ({:?}); the previous toolchain was restored.",
            error.kind()
        ));
    }

    if had_previous {
        let _ = std::fs::remove_dir_all(backup_dir);
    }
    // The bundled Python home must reference the activated directory, never
    // the temporary .installing directory that no longer exists after rename.
    if let Err(error) = patch_venv_paths(final_dir) {
        eprintln!("[Toolchain] pyvenv patch warning: {}", error);
    }
    Ok(())
}

// ── Progress event payload ────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
pub struct ToolchainProgress {
    /// "downloading" | "verifying" | "extracting" | "done" | "error" | "cancelled"
    pub stage: String,
    /// 0–100
    pub percent: u8,
    pub message: String,
}

fn emit_progress(app: &AppHandle, stage: &str, percent: u8, message: &str) {
    let _ = app.emit(
        "toolchain-progress",
        ToolchainProgress {
            stage: stage.to_string(),
            percent,
            message: message.to_string(),
        },
    );
}

// ── Helper: resolve toolchain directory ──────────────────────────────────────

/// คืนค่า path ของโฟลเดอร์ toolchain ใน AppData
/// ตำแหน่งจริงมาจาก Tauri `app_data_dir()` ของผู้ใช้แต่ละเครื่อง
pub fn get_toolchain_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Cannot resolve AppData: {}", e))?;
    Ok(base.join("toolchain"))
}

/// ตรวจสอบว่า toolchain พร้อมใช้งานหรือยัง
#[cfg(test)]
fn is_toolchain_ready(toolchain_dir: &Path) -> bool {
    toolchain_dir.join(SENTINEL_FILE).exists()
}

// ── Tauri Commands ────────────────────────────────────────────────────────────

/// ตรวจสอบสถานะ toolchain — frontend เรียกตอนเปิดแอป
/// Returns: "ready" | "not_installed" | { version, path }
#[tauri::command]
pub async fn check_toolchain(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    // A marker records an earlier installation; it cannot prove that Python or
    // the compiler can run on this machine. Use exactly the build resolver.
    let status = match crate::esp_idf::resolve_idf_paths(&app_handle) {
        Ok((idf, tools)) => crate::esp_idf::checked_sdk_status(idf, tools).await,
        Err(error) => Err(error),
    };
    match status {
        Ok(status) => Ok(status),
        Err(error) => Ok(serde_json::json!({
            "status": "not_installed",
            "version": null,
            "path": null,
            "platform": std::env::consts::OS,
            "error": error
        })),
    }
}

/// ดาวน์โหลดและแตกไฟล์ toolchain — เรียกแค่ครั้งเดียวในชีวิต
/// emit events: "toolchain-progress" ระหว่างทำงาน
#[tauri::command]
pub async fn download_toolchain(
    app_handle: AppHandle,
    url: Option<String>,
) -> Result<String, String> {
    let _install_guard = try_acquire_install(install_flag())?;

    // Resolve trust before touching AppData, making requests or accepting a ZIP.
    let parts = Manifest::parse(crate::toolchain_integrity::MANIFEST)?.plan(
        &crate::toolchain_integrity::host_platform(), TOOLCHAIN_VERSION, url.as_deref(),
    )?;
    let toolchain_dir = get_toolchain_dir(&app_handle)?;
    cancel_flag().store(false, Ordering::SeqCst);
    let cancel = cancel_flag();
    let app = app_handle.clone();
    tokio::task::spawn_blocking(move || {
        let _install_guard = _install_guard;
        let client = make_http_client()?;
        install_verified_parts(&toolchain_dir, &parts, &cancel,
            |asset, output| download_archive(&app, &client, asset, output, &cancel),
            |stage, percent, message| emit_progress(&app, stage, percent, message),
        )
    }).await.map_err(|_| "Toolchain installation task failed; the existing toolchain was retained.".to_string())??;
    emit_progress(&app_handle, "done", 100, "Toolchain integrity checked and installation complete.");
    Ok("Toolchain integrity checked and installation complete.".into())
}

/// ยกเลิกการดาวน์โหลดที่กำลังดำเนินอยู่
#[tauri::command]
pub async fn cancel_toolchain_download() -> Result<(), String> {
    cancel_flag().store(true, Ordering::SeqCst);
    Ok(())
}

/// ลบ toolchain ทั้งหมดออก (สำหรับ "Reinstall" หรือแก้ปัญหา)
#[tauri::command]
pub async fn remove_toolchain(app_handle: AppHandle) -> Result<String, String> {
    let _install_guard = try_acquire_install(install_flag())?;
    let toolchain_dir = get_toolchain_dir(&app_handle)?;

    if !toolchain_dir.exists() {
        return Ok("Toolchain directory does not exist, nothing to remove.".to_string());
    }

    std::fs::remove_dir_all(&toolchain_dir)
        .map_err(|e| format!("Failed to remove toolchain: {}", e))?;

    Ok(format!(
        "Toolchain removed from {}",
        toolchain_dir.display()
    ))
}

/// แก้ไข Python venv paths ที่ยังชี้ไปยังเครื่องต้นทางเครื่องอื่น
/// เรียกใช้บนเครื่องที่ติดตั้ง toolchain แล้วแต่ build ไม่ผ่านด้วย "No Python at ..." error
#[tauri::command]
pub async fn repair_toolchain_paths(app_handle: AppHandle) -> Result<String, String> {
    let _install_guard = try_acquire_install(install_flag())?;
    #[cfg(target_os = "macos")]
    {
        let (idf, tools) = crate::esp_idf::resolve_idf_paths(&app_handle)?;
        crate::esp_idf::checked_sdk_status(idf.clone(), tools).await?;
        return Ok(format!("Native macOS SDK is ready at {}. No Windows venv paths were rewritten.", idf.display()));
    }
    #[cfg(not(target_os = "macos"))]
    {
    let toolchain_dir = get_toolchain_dir(&app_handle)?;
    if !toolchain_dir.exists() {
        return Err("Toolchain not installed yet. Please install it first.".to_string());
    }

    // Run the venv path patcher
    let dir = toolchain_dir.clone();
    tokio::task::spawn_blocking(move || patch_venv_paths(&dir))
        .await
        .map_err(|e| format!("Task panicked: {}", e))??;
    let (idf, tools) = crate::esp_idf::resolve_idf_paths(&app_handle)?;
    crate::esp_idf::checked_sdk_status(idf, tools).await?;

    Ok(format!(
        "Python venv paths repaired successfully in {}",
        toolchain_dir.display()
    ))
    }
}

/// คืนค่า path สำคัญต่างๆ ของ toolchain ให้ frontend ตรวจสอบ
#[tauri::command]
pub async fn get_toolchain_paths(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let base = get_toolchain_dir(&app_handle)?;
    let status = match crate::esp_idf::resolve_idf_paths(&app_handle) {
        Ok((idf, tools)) => crate::esp_idf::checked_sdk_status(idf, tools).await,
        Err(error) => Err(error),
    };
    match status {
        Ok(status) => Ok(serde_json::json!({
            "toolchain_dir": base, "idf_path":status["path"], "tools_path":status["tools_path"],
            "idf_exists":true, "tools_exists":true, "python_venv":status["python_venv"],
            "python_bin":status["python_bin"], "python_ready":true, "ready":true,
        })),
        Err(error) => Ok(serde_json::json!({
            "toolchain_dir":base, "idf_path":null, "tools_path":null, "idf_exists":false,
            "tools_exists":false, "python_venv":null, "python_bin":null, "python_ready":false,
            "ready":false, "error":error,
        })),
    }
}

// ── Core Logic: Download + Extract ───────────────────────────────────────────

/// สร้าง HTTP client พร้อม cookie store (จำเป็นสำหรับ Google Drive)
fn make_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30)) // ค้าง connect ได้สูงสุด 30s
        .timeout(std::time::Duration::from_secs(600)) // total timeout 10 นาที
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::limited(15))
        .https_only(true)
        .tcp_keepalive(std::time::Duration::from_secs(30)) // keepalive ป้องกัน connection drop
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// แปลง URL ให้เป็น direct download URL
/// - Google Drive share URL → drive.usercontent.google.com/download?id=...&confirm=t
/// - URL อื่นๆ → คืนค่าเดิม
#[cfg(test)]
fn resolve_download_url(url: &str) -> String {
    // ตรวจว่าเป็น Google Drive URL หรือไม่
    if !url.contains("drive.google.com") && !url.contains("docs.google.com") {
        return url.to_string();
    }

    // แยก file ID จาก URL หลายรูปแบบ
    // รูปแบบ: /file/d/{id}/
    let file_id = if let Some(part) = url.split("/file/d/").nth(1) {
        part.split('/')
            .next()
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("")
    }
    // รูปแบบ: ?id={id}
    else if let Some(part) = url
        .split("?id=")
        .nth(1)
        .or_else(|| url.split("&id=").nth(1))
    {
        part.split('&').next().unwrap_or("")
    }
    // รูปแบบ: /open?id={id}
    else if let Some(part) = url.split("open?id=").nth(1) {
        part.split('&').next().unwrap_or("")
    } else {
        ""
    };

    if file_id.is_empty() {
        return url.to_string();
    }

    // ใช้ drive.usercontent.google.com พร้อม confirm=t เพื่อข้าม virus-scan page
    format!(
        "https://drive.usercontent.google.com/download?id={}&export=download&confirm=t&uuid=1",
        file_id
    )
}

/// All archive sources (network or local/cache fixtures) enter this transaction.
/// No archive is extracted until every part has passed verification.
fn install_verified_parts(
    final_dir: &Path,
    parts: &[Asset],
    cancel: &AtomicBool,
    mut acquire: impl FnMut(&Asset, &mut std::fs::File) -> Result<(), String>,
    mut progress: impl FnMut(&str, u8, &str),
) -> Result<(), String> {
    if parts.len() != crate::toolchain_integrity::PART_IDS.len() {
        return Err("Toolchain integrity: incomplete installation plan.".into());
    }
    let parent = final_dir.parent().ok_or("Cannot resolve toolchain parent directory.")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create install workspace ({:?}).", e.kind()))?;
    let workspace = tempfile::Builder::new().prefix(".toolchain-install-").tempdir_in(parent)
        .map_err(|e| format!("Cannot create private staging directory ({:?}).", e.kind()))?;
    let staging = workspace.path().join("staging");
    let mut archives = Vec::new();
    for asset in parts {
        if cancel.load(Ordering::SeqCst) {
            return Err("Toolchain installation cancelled; the existing toolchain was retained.".into());
        }
        progress("verifying", 1, "Acquiring archive and checking SHA-256 against the trusted manifest...");
        archives.push(VerifiedArchive::acquire(asset.clone(), cancel, |file| acquire(asset, file))?);
    }
    for (index, archive) in archives.iter_mut().enumerate() {
        progress("extracting", 50, "Extracting verified archive into staging...");
        archive.extract(&staging, cancel, |i, total, _name| {
            let percent = 50 + ((index as f64 + i as f64 / total.max(1) as f64) / parts.len() as f64 * 48.0) as u8;
            progress("extracting", percent, "Extracting verified archive into staging...");
        })?;
    }
    progress("verifying", 99, "Rechecking archive SHA-256 before activation...");
    let result = activate_toolchain(&staging, final_dir, &mut archives, cancel);
    if result.is_err() && workspace.path().join("previous").exists() {
        // A failed OS rollback must not cause TempDir to delete the old install.
        let _ = workspace.keep();
    }
    result
}

fn download_archive(
    app: &AppHandle,
    client: &reqwest::blocking::Client,
    asset: &Asset,
    output: &mut std::fs::File,
    cancel: &AtomicBool,
) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) { return Err("Toolchain download cancelled.".into()); }
    emit_progress(app, "downloading", 1, &format!("Downloading {}...", asset.id));
    let mut response = client.get(&asset.download_url).send()
        .map_err(|_| "Toolchain download connection failed; check the network and proxy.".to_string())?;
    if !response.status().is_success() {
        return Err(format!("Toolchain download rejected by server (HTTP {}).", response.status().as_u16()));
    }
    let mut buffer = [0u8; 128 * 1024];
    let mut last_update = std::time::Instant::now();
    loop {
        if cancel.load(Ordering::SeqCst) { return Err("Toolchain download cancelled.".into()); }
        let size = response.read(&mut buffer).map_err(|_| "Toolchain download was interrupted or unreadable.".to_string())?;
        if size == 0 { break; }
        output.write_all(&buffer[..size]).map_err(|e| format!("Cannot store toolchain archive ({:?}).", e.kind()))?;
        if last_update.elapsed() >= std::time::Duration::from_secs(3) {
            emit_progress(app, "downloading", 1, &format!("Downloading {}...", asset.id));
            last_update = std::time::Instant::now();
        }
    }
    output.flush().map_err(|e| format!("Cannot flush toolchain archive ({:?}).", e.kind()))
}

// ── Build Firmware using bundled toolchain ────────────────────────────────────

/// สั่ง build firmware โดยใช้ toolchain ที่โหลดมาไว้ใน AppData
/// ไม่ต้องพึ่ง Python หรือ Git ของระบบ Windows เลย!
#[tauri::command]
pub async fn build_firmware_with_toolchain(
    app_handle: AppHandle, project_dir: String, extra_args: Option<Vec<String>>,
) -> Result<(), String> {
    // Use the same configured SDK, profile/review checks, bounded process tree,
    // cancellation and diagnostics as the main Build button.
    emit_progress(&app_handle,"building",0,"Starting configured SDK build...");
    crate::esp_idf::run_idf_command(app_handle.clone(),"build".into(),extra_args.unwrap_or_default(),Some(project_dir)).await?;
    emit_progress(&app_handle,"done",100,"Build complete; physical behavior not verified.");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_first_venv(python_env_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(python_env_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("idf") && name.contains("_py") && name.ends_with("_env") {
            return Some(entry.path());
        }
    }
    None
}

/// Find Python that will actually work on the current machine.
///
/// Strategy (most reliable first):
///   1. Bundled idf-python inside toolchain/tools/idf-python/<ver>/python.exe
///      — This is the standalone CPython that ESP-IDF ships, no venv stub involved.
///   2. Venv launcher in python_env/<env>/Scripts/python.exe
///      — Only works if pyvenv.cfg has been patched to point to the current machine.
///
/// Returns (python_binary, venv_directory).
fn find_toolchain_python(tools_path: &Path) -> Option<(PathBuf, PathBuf)> {
    let venv_dir = find_first_venv(&tools_path.join("python_env"))?;

    // ── Option 1: bundled idf-python (preferred — machine-independent) ─────
    let idf_python_bin = find_bundled_python(tools_path);
    if let Some(ref bin) = idf_python_bin {
        if bin.exists() {
            eprintln!("[Toolchain] Using bundled idf-python: {}", bin.display());
            return Some((bin.clone(), venv_dir.clone()));
        }
    }

    // ── Option 2: venv launcher (works only after pyvenv.cfg patch) ─────────
    // Read home= from pyvenv.cfg and verify that Python actually exists there.
    let cfg_path = venv_dir.join("pyvenv.cfg");
    if let Ok(cfg) = std::fs::read_to_string(&cfg_path) {
        let home = cfg
            .lines()
            .find(|l| l.trim().to_lowercase().starts_with("home") && l.contains('='))
            .and_then(|l| l.split_once('=').map(|(_, value)| value))
            .map(|s| s.trim().to_string());

        if let Some(home_dir) = home {
            let home_path = PathBuf::from(&home_dir);
            // Verify the home= Python actually exists on THIS machine
            let home_python = if cfg!(windows) {
                home_path.join("python.exe")
            } else {
                home_path.join("python3")
            };
            if home_python.exists() {
                // home= is valid — the venv launcher should work
                let venv_python = if cfg!(windows) {
                    venv_dir.join("Scripts").join("python.exe")
                } else {
                    venv_dir.join("bin").join("python3")
                };
                if venv_python.exists() {
                    eprintln!(
                        "[Toolchain] Using venv python (home= valid): {}",
                        venv_python.display()
                    );
                    return Some((venv_python, venv_dir));
                }
            } else {
                // home= points to a path that is unavailable on this machine — auto-patch now
                eprintln!(
                    "[Toolchain] pyvenv.cfg home= not found ({}), auto-patching...",
                    home_dir
                );
                if let Some(ref fresh_python) = idf_python_bin {
                    // Patch pyvenv.cfg inline
                    let new_home = fresh_python
                        .parent()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let patched = cfg
                        .lines()
                        .map(|line| {
                            if line.trim().to_lowercase().starts_with("home") && line.contains('=')
                            {
                                format!("home = {}", new_home)
                            } else {
                                line.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\r\n");
                    let _ = std::fs::write(&cfg_path, &patched);
                    eprintln!(
                        "[Toolchain] Patched {} home= → {}",
                        cfg_path.display(),
                        new_home
                    );
                    return Some((fresh_python.clone(), venv_dir));
                }
            }
        }
    }

    None
}

/// Find the bundled idf-python binary in the toolchain tools directory.
/// Searches: tools/idf-python/<version>/python.exe and similar layouts.
fn find_bundled_python(tools_path: &Path) -> Option<PathBuf> {
    let idf_python_root = tools_path.join("tools").join("idf-python");
    if idf_python_root.is_dir() {
        let mut versions: Vec<PathBuf> = std::fs::read_dir(&idf_python_root)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        versions.sort(); // highest version last
        versions.reverse();
        for ver_dir in &versions {
            // python.exe directly in version dir (e.g. idf-python/3.11.2/python.exe)
            if cfg!(windows) {
                let candidate = ver_dir.join("python.exe");
                if candidate.exists() {
                    return Some(candidate);
                }
            } else {
                for name in &["python3", "python"] {
                    let candidate = ver_dir.join(name);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    // Fallback: scan tools/tools/ at depth ≤3
    find_python_in_dir(&tools_path.join("tools"), 0, 3)
        .or_else(|| find_python_in_dir(tools_path, 0, 3))
}

/// Walk a directory tree looking for python.exe / python3 (max_depth limit).
fn find_python_in_dir(dir: &Path, depth: u32, max_depth: u32) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if name == "python.exe" || name == "python3" || name == "python" {
                return Some(p);
            }
        } else if p.is_dir() {
            dirs.push(p);
        }
    }
    for d in dirs {
        if let Some(found) = find_python_in_dir(&d, depth + 1, max_depth) {
            return Some(found);
        }
    }
    None
}

/// Called at app startup: silently patch pyvenv.cfg files so Python paths always
/// reflect the current machine's toolchain install location.
/// Safe to call even if toolchain is not installed (no-op).
pub fn auto_repair_on_startup(toolchain_dir: &Path) {
    // Windows venv rewriting cannot create native macOS Python or compilers.
    if cfg!(target_os = "macos") { return; }
    if !toolchain_dir.join(SENTINEL_FILE).exists() {
        return;
    }
    if let Err(e) = patch_venv_paths(toolchain_dir) {
        eprintln!("[Startup] pyvenv auto-repair warning: {}", e);
    } else {
        eprintln!("[Startup] pyvenv paths verified/patched for this machine.");
    }
}

/// Patch all pyvenv.cfg files found under `toolchain_dir` so that the `home =` line
/// points to the **bundled** idf-python bin directory on the current machine.
///
/// Background: Python venvs store an absolute `home =` path in pyvenv.cfg that
/// references the Python interpreter used to create the venv.  When the toolchain
/// ZIP was built on another machine, that absolute path is
/// baked in and will not exist on any other PC, causing:
///   No Python at '<path-from-another-machine>' error.
///
/// We fix this by:
///   1. Finding the bundled idf-python executable under toolchain/tools/idf-python/
///   2. Rewriting `home = <anything>` → `home = <bundled_python_dir>` in every pyvenv.cfg.
fn patch_venv_paths(toolchain_dir: &Path) -> Result<(), String> {
    // ── Step 1: locate bundled idf-python ─────────────────────────────────
    // Expected layout after extraction:
    //   toolchain/tools/idf-python/<version>/python.exe   (Windows)
    //   toolchain/tools/idf-python/<version>/python3      (Unix)
    let tools_dir = toolchain_dir.join("tools");
    let idf_python_dir = tools_dir.join("idf-python");

    let bundled_python_bin_dir: Option<PathBuf> = (|| -> Option<PathBuf> {
        // Walk: idf-python/<version>/python.exe
        if idf_python_dir.is_dir() {
            let versions: Vec<_> = std::fs::read_dir(&idf_python_dir)
                .ok()?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            for ver in &versions {
                // Direct python.exe in version dir
                if cfg!(windows) {
                    if ver.join("python.exe").exists() {
                        return Some(ver.clone());
                    }
                } else {
                    if ver.join("python3").exists() || ver.join("python").exists() {
                        return Some(ver.clone());
                    }
                }
                // python.exe inside bin/
                let bin = ver.join("bin");
                if bin.exists() {
                    return Some(bin);
                }
            }
        }
        // Fallback: scan tools/ for any directory containing python.exe at depth ≤3
        find_python_dir(&tools_dir, 0, 3)
    })();

    // ── Step 2: find all pyvenv.cfg files under toolchain/ ────────────────
    let mut cfg_files: Vec<PathBuf> = Vec::new();
    collect_pyvenv_cfgs(toolchain_dir, &mut cfg_files);

    if cfg_files.is_empty() {
        eprintln!(
            "[Patch] No pyvenv.cfg found under {} — skipping patch",
            toolchain_dir.display()
        );
        return Ok(());
    }

    eprintln!("[Patch] Found {} pyvenv.cfg file(s)", cfg_files.len());

    // ── Step 3: rewrite `home = ...` in each cfg ──────────────────────────
    for cfg_path in &cfg_files {
        let content = match std::fs::read_to_string(cfg_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[Patch] Cannot read {}: {}", cfg_path.display(), e);
                continue;
            }
        };

        // Determine the correct `home` value:
        // Prefer bundled idf-python dir; fall back to the venv's own Scripts/bin dir.
        let new_home: String = if let Some(ref bp) = bundled_python_bin_dir {
            bp.to_string_lossy().into_owned()
        } else {
            // If no bundled python found, keep the venv's own Scripts/bin
            let venv_dir = cfg_path.parent().unwrap_or(cfg_path);
            let bin = if cfg!(windows) {
                venv_dir.join("Scripts")
            } else {
                venv_dir.join("bin")
            };
            bin.to_string_lossy().into_owned()
        };

        let patched = content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.to_lowercase().starts_with("home") && trimmed.contains('=') {
                    format!("home = {}", new_home)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\r\n"); // keep Windows line endings

        if let Err(e) = std::fs::write(cfg_path, &patched) {
            eprintln!("[Patch] Cannot write {}: {}", cfg_path.display(), e);
        } else {
            eprintln!(
                "[Patch] Patched {} → home = {}",
                cfg_path.display(),
                new_home
            );
        }
    }

    Ok(())
}

/// Recursively collect all `pyvenv.cfg` files under `dir` (max depth 8).
fn collect_pyvenv_cfgs(dir: &Path, out: &mut Vec<PathBuf>) {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        if depth > 8 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, depth + 1, out);
            } else if p.file_name().map(|n| n == "pyvenv.cfg").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    walk(dir, 0, out);
}

/// Walk `dir` up to `max_depth` and return the first directory containing python.exe / python3.
fn find_python_dir(dir: &Path, depth: u32, max_depth: u32) -> Option<PathBuf> {
    if depth > max_depth {
        return None;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if name == "python.exe" || name == "python3" || name == "python" {
                return p.parent().map(|d| d.to_path_buf());
            }
        } else if p.is_dir() {
            if let Some(found) = find_python_dir(&p, depth + 1, max_depth) {
                return Some(found);
            }
        }
    }
    None
}

fn build_toolchain_path(tools_path: &Path) -> std::ffi::OsString {
    let mut paths: Vec<PathBuf> = Vec::new();

    // 1. venv Scripts/bin (Python) — must come first so idf.py picks up the right python
    if let Some(venv) = find_first_venv(&tools_path.join("python_env")) {
        let bin = if cfg!(windows) {
            venv.join("Scripts")
        } else {
            venv.join("bin")
        };
        if bin.exists() {
            paths.push(bin);
        }
    }
    // Also add bundled idf-python directory
    if let Some(py) = find_bundled_python(tools_path) {
        if let Some(py_dir) = py.parent() {
            paths.push(py_dir.to_path_buf());
        }
    }

    // 2. Scan .espressif/tools/ — adds ninja, cmake, compiler bin dirs, etc.
    //    Uses scan_esp_tool_dirs which handles BOTH:
    //      <tool>/<ver>/bin/<exe>  (cmake, gcc, ...)
    //      <tool>/<ver>/<exe>      (ninja.exe directly in version dir)
    scan_esp_tool_dirs(&tools_path.join("tools"), &mut paths);
    scan_esp_tool_dirs(tools_path, &mut paths);

    // 3. System PATH (fallback)
    if let Some(system_path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system_path));
    }

    std::env::join_paths(paths).unwrap_or_else(|_| std::ffi::OsString::from(""))
}

/// Find the directory containing ccache.exe on any machine, regardless of how it was installed.
/// Checks (in priority order):
///   1. System PATH — already in PATH (simplest, covers most cases)
///   2. WinGet packages dir — dynamically scans, any version
///   3. Chocolatey
///   4. Scoop (user or system)
///   5. Common manual install locations
///
/// Returns the *directory* containing ccache.exe, or None if not found anywhere.
fn find_ccache_dir() -> Option<PathBuf> {
    // 1. Already in PATH — check first, zero overhead
    if which_in_path("ccache") {
        return None; // already findable, no need to inject
    }

    // 2. WinGet packages — scan dynamically (no hardcoded version)
    let candidates_winget = [
        std::env::var("LOCALAPPDATA").unwrap_or_default() + "\\Microsoft\\WinGet\\Packages",
        std::env::var("PROGRAMFILES").unwrap_or_default() + "\\WinGet\\Packages",
    ];
    for base in &candidates_winget {
        let base = PathBuf::from(base);
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with("ccache") {
                    // Recurse one level to find the bin dir with ccache.exe
                    if let Some(dir) = find_exe_in_dir(&entry.path(), "ccache.exe", 2) {
                        return Some(dir);
                    }
                }
            }
        }
    }

    // 3. Chocolatey
    let choco = PathBuf::from(
        std::env::var("ChocolateyInstall")
            .unwrap_or_else(|_| "C:\\ProgramData\\chocolatey".to_string()),
    )
    .join("bin");
    if choco.join("ccache.exe").exists() {
        return Some(choco);
    }

    // 4. Scoop (user install, then system-wide)
    let scoop_candidates = [
        std::env::var("USERPROFILE").unwrap_or_default() + "\\scoop\\shims",
        "C:\\ProgramData\\scoop\\shims".to_string(),
        std::env::var("USERPROFILE").unwrap_or_default() + "\\scoop\\apps\\ccache\\current",
    ];
    for s in &scoop_candidates {
        let p = PathBuf::from(s);
        if p.join("ccache.exe").exists() {
            return Some(p);
        }
    }

    // 5. Common manual install locations
    let manual = [
        "C:\\ccache",
        "C:\\tools\\ccache",
        "C:\\Program Files\\ccache",
    ];
    for m in &manual {
        let p = PathBuf::from(m);
        if p.join("ccache.exe").exists() {
            return Some(p);
        }
    }

    None
}

/// Check if an executable is already visible in the process's PATH.
fn which_in_path(exe: &str) -> bool {
    let exe_name = if cfg!(windows) {
        format!("{exe}.exe")
    } else {
        exe.to_string()
    };
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            if dir.join(&exe_name).exists() {
                return true;
            }
        }
    }
    false
}

/// Recursively search `root` up to `max_depth` levels deep for a file named `exe_name`.
/// Returns the *parent directory* of the found executable.
fn find_exe_in_dir(root: &Path, exe_name: &str, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .as_deref()
                    == Some(&exe_name.to_lowercase())
                {
                    return path.parent().map(|p| p.to_path_buf());
                }
            } else if path.is_dir() {
                if let Some(found) = find_exe_in_dir(&path, exe_name, max_depth - 1) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Scan an ESP-IDF tools directory structure and collect all directories that
/// contain executable files.  Handles the two common ESP-IDF tool layouts:
///   a) <tool>/<version>/bin/<exe>      → adds bin/
///   b) <tool>/<version>/<exe>          → adds version/ directly (ninja, idf-python)
///   c) <tool>/<variant>/<ver>/bin/     → adds bin/ (xtensa-esp-elf sub-variants)
fn scan_esp_tool_dirs(tools_dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(tools) = std::fs::read_dir(tools_dir) else {
        return;
    };
    for tool_entry in tools.flatten() {
        let tool_path = tool_entry.path();
        if !tool_path.is_dir() {
            continue;
        }
        let Ok(versions) = std::fs::read_dir(&tool_path) else {
            continue;
        };
        for ver_entry in versions.flatten() {
            let ver_path = ver_entry.path();
            if !ver_path.is_dir() {
                continue;
            }
            // Layout a: bin/ subdir
            let bin_dir = ver_path.join("bin");
            if bin_dir.is_dir() {
                paths.push(bin_dir);
            }
            // Layout b: executables directly in version dir
            if dir_has_executables(&ver_path) {
                paths.push(ver_path.clone());
            }
            // Layout c: one more level deep
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
        if cfg!(windows) {
            if p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("exe"))
                .unwrap_or(false)
            {
                return true;
            }
        } else {
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

// ── Google Drive Token Extractor ──────────────────────────────────────────────

/// หา download confirmation token จาก HTML ที่ Google Drive ส่งมา
/// Google Drive ใช้ form action หรือ query param หลายรูปแบบ
#[cfg(test)]
fn extract_gdrive_token(html: &str) -> Option<String> {
    for pattern in &["confirm=", "&amp;confirm="] {
        if let Some(pos) = html.find(pattern) {
            let rest = &html[pos + pattern.len()..];
            let token: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !token.is_empty() && token != "t" {
                return Some(token);
            }
        }
    }
    None
}

// ── Unit Tests ────────────────────────────────────────────────────────────────
// Pure-logic tests for URL resolution, token extraction and filesystem scans.

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn install_lock_rejects_concurrent_attempt_and_releases_on_drop() {
        let flag = Arc::new(AtomicBool::new(false));
        let first = try_acquire_install(flag.clone()).unwrap();
        assert_eq!(
            try_acquire_install(flag.clone()).unwrap_err(),
            "already_in_progress"
        );

        drop(first);
        assert!(try_acquire_install(flag).is_ok());
    }

    #[test]
    fn staged_install_is_activated_with_final_python_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let final_dir = tmp.path().join("toolchain");
        let staging_dir = tmp.path().join("toolchain.installing");
        fs::create_dir_all(&staging_dir).unwrap();
        fs::write(staging_dir.join(SENTINEL_FILE), TOOLCHAIN_VERSION).unwrap();
        let python_relative = Path::new("tools/idf-python/test-version");
        fs::create_dir_all(staging_dir.join(python_relative)).unwrap();
        let python_name = if cfg!(windows) { "python.exe" } else { "python3" };
        fs::write(staging_dir.join(python_relative).join(python_name), "").unwrap();
        let venv_relative = Path::new("tools/python_env/test-venv/pyvenv.cfg");
        fs::create_dir_all(staging_dir.join(venv_relative).parent().unwrap()).unwrap();
        fs::write(
            staging_dir.join(venv_relative),
            "home = old-machine\ninclude-system-site-packages = false\n",
        ).unwrap();

        let mut archives = super::install_tests::archives_for_activation();
        activate_toolchain(&staging_dir, &final_dir, &mut archives, &AtomicBool::new(false)).unwrap();

        assert!(!staging_dir.exists());
        assert!(is_toolchain_ready(&final_dir));
        let config = fs::read_to_string(final_dir.join(venv_relative)).unwrap();
        let home = config.lines().find_map(|line| line.strip_prefix("home = ")).unwrap();
        assert_eq!(
            dunce::canonicalize(home).unwrap(),
            dunce::canonicalize(final_dir.join(python_relative)).unwrap()
        );
        assert!(!config.contains(".installing"));
    }

    // ── extract_gdrive_token ──────────────────────────────────────────────────

    #[test]
    fn gdrive_token_extracted_from_confirm_param() {
        let html = "https://drive.usercontent.google.com/download?id=1&confirm=Abc_9-X&x=y";
        assert_eq!(extract_gdrive_token(html), Some("Abc_9-X".to_string()));
    }

    #[test]
    fn gdrive_token_extracted_after_html_entity() {
        // First "confirm=" yields empty token → must fall through to "&amp;confirm="
        let html = "confirm=&amp;confirm=z12_3";
        assert_eq!(extract_gdrive_token(html), Some("z12_3".to_string()));
    }

    #[test]
    fn gdrive_token_bare_t_is_rejected() {
        // "t" is the placeholder sentinel — must not be treated as a real token
        assert_eq!(extract_gdrive_token("?id=x&confirm=t"), None);
    }

    #[test]
    fn gdrive_token_absent_returns_none() {
        assert_eq!(extract_gdrive_token("nothing to see here"), None);
    }

    // ── resolve_download_url ──────────────────────────────────────────────────

    #[test]
    fn resolve_download_url_passthrough_non_gdrive() {
        let url = "https://example.com/toolchain.zip";
        assert_eq!(resolve_download_url(url), url);
    }

    #[test]
    fn resolve_download_url_file_d_form() {
        let out = resolve_download_url("https://drive.google.com/file/d/FILE123/view?usp=sharing");
        assert!(
            out.contains("download?id=FILE123"),
            "expected FILE123 in rewritten URL, got: {}",
            out
        );
        assert!(out.contains("confirm=t"));
    }

    #[test]
    fn resolve_download_url_query_id_form() {
        let out = resolve_download_url("https://docs.google.com/uc?id=ABC456&export=download");
        assert!(
            out.contains("download?id=ABC456"),
            "expected ABC456 in rewritten URL, got: {}",
            out
        );
    }

    #[test]
    fn resolve_download_url_gdrive_without_id_is_passthrough() {
        let url = "https://drive.google.com/folder";
        assert_eq!(resolve_download_url(url), url);
    }

    // ── is_toolchain_ready ────────────────────────────────────────────────────

    #[test]
    fn toolchain_ready_requires_sentinel_file() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_toolchain_ready(tmp.path()));

        fs::write(tmp.path().join(SENTINEL_FILE), "v1").unwrap();
        assert!(is_toolchain_ready(tmp.path()));
    }
}

#[cfg(test)]
#[path = "toolchain_install_tests.rs"]
mod install_tests;
