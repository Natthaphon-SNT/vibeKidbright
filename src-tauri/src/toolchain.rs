// ── toolchain.rs ─────────────────────────────────────────────────────────────
// "Happy Meal" Toolchain Manager
//
// แทนที่จะให้ Rust ไปสั่งดาวน์โหลด ESP-IDF, Python, Git ทีละตัว
// โมดูลนี้จะดาวน์โหลด pre-packaged ZIP (kb_compiler_v1.zip) จาก Cloud
// แล้วแตกไฟล์ลงใน AppData ทันที — ผู้ใช้แค่รอ Loading bar ครั้งแรกครั้งเดียว
//
// Flow:
//   1. check_toolchain()    → ตรวจสอบว่ามีอยู่แล้วหรือยัง
//   2. download_toolchain() → โหลด ZIP + แตกไฟล์ พร้อม emit progress events
//   3. get_toolchain_dir()  → คืนค่า path ของ toolchain ให้ build_firmware() ใช้
// ─────────────────────────────────────────────────────────────────────────────

use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

// ── Constants ─────────────────────────────────────────────────────────────────

/// ── วิธีที่ 1: Split ZIP หลายพาร์ต (GitHub Releases) ────────────────────────
///
/// ตัดไฟล์ด้วย 7-Zip:
///   7z a -v1800m kb_compiler_v1.zip.001 kb_compiler_v1.zip
///
/// อัปโหลดทุกพาร์ตเข้า GitHub Release เดียวกัน แล้วใส่ URL ด้านล่าง
/// (ถ้ามีแค่พาร์ตเดียวก็ใส่ 1 entry ก็พอ)
///
/// ── วิธีที่ 2: Hugging Face Hub (ไฟล์ใหญ่ถึง 50 GB) ──────────────────────────
///
/// อัปโหลดไฟล์เดียวไปที่ https://huggingface.co แล้วใช้ URL แบบนี้:
///   https://huggingface.co/{username}/{repo}/resolve/main/kb_compiler_v1.zip
///
/// Hugging Face ส่ง direct download ตรงๆ reqwest จัดการได้เลย
/// Google Drive share URL → แปลงเป็น direct download URL อัตโนมัติ
/// รองรับทั้ง: drive.google.com/file/d/{id}/... และ drive.google.com/open?id={id}
const TOOLCHAIN_PARTS: &[&str] = &[
    // Google Drive — ใส่ sharing URL ตรงๆ ได้เลย ระบบจะแปลงให้อัตโนมัติ
    "https://drive.google.com/file/d/1uWHX5w_BD_EmoViaoBbjDqTicJP5CJIT/view?usp=sharing",
];

/// ชื่อ "sentinel file" — ถ้าไฟล์นี้มีอยู่ แปลว่า toolchain สมบูรณ์แล้ว
const SENTINEL_FILE: &str = ".toolchain_ready";

/// เวอร์ชัน toolchain — เปลี่ยนเมื่ออัปเดต ZIP บน Cloud
const TOOLCHAIN_VERSION: &str = "1.0.0";

// ── Global cancel flag ────────────────────────────────────────────────────────

static CANCEL_FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();

fn cancel_flag() -> Arc<AtomicBool> {
    CANCEL_FLAG
        .get_or_init(|| Arc::new(AtomicBool::new(false)))
        .clone()
}

// ── Progress event payload ────────────────────────────────────────────────────

#[derive(Clone, serde::Serialize)]
pub struct ToolchainProgress {
    /// "downloading" | "extracting" | "done" | "error" | "cancelled"
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
/// ตัวอย่าง: C:\Users\name\AppData\Roaming\com.rmutt.KidBrightVibe\toolchain
pub fn get_toolchain_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Cannot resolve AppData: {}", e))?;
    Ok(base.join("toolchain"))
}

/// ตรวจสอบว่า toolchain พร้อมใช้งานหรือยัง
fn is_toolchain_ready(toolchain_dir: &Path) -> bool {
    toolchain_dir.join(SENTINEL_FILE).exists()
}

// ── Tauri Commands ────────────────────────────────────────────────────────────

/// ตรวจสอบสถานะ toolchain — frontend เรียกตอนเปิดแอป
/// Returns: "ready" | "not_installed" | { version, path }
#[tauri::command]
pub async fn check_toolchain(app_handle: AppHandle) -> Result<serde_json::Value, String> {
    let dir = get_toolchain_dir(&app_handle)?;

    if is_toolchain_ready(&dir) {
        // อ่าน version จาก sentinel file
        let sentinel = dir.join(SENTINEL_FILE);
        let version = std::fs::read_to_string(&sentinel)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        Ok(serde_json::json!({
            "status": "ready",
            "version": version,
            "path": dir.to_string_lossy()
        }))
    } else {
        Ok(serde_json::json!({
            "status": "not_installed",
            "version": null,
            "path": null
        }))
    }
}

/// ดาวน์โหลดและแตกไฟล์ toolchain — เรียกแค่ครั้งเดียวในชีวิต
/// emit events: "toolchain-progress" ระหว่างทำงาน
#[tauri::command]
pub async fn download_toolchain(
    app_handle: AppHandle,
    url: Option<String>,
) -> Result<String, String> {
    let custom_parts: Vec<String> = url
        .filter(|u| !u.trim().is_empty())
        .map(|u| vec![u])
        .unwrap_or_else(|| TOOLCHAIN_PARTS.iter().map(|s| s.to_string()).collect());

    let toolchain_dir = get_toolchain_dir(&app_handle)?;

    // ถ้ามีอยู่แล้ว ข้ามเลย
    if is_toolchain_ready(&toolchain_dir) {
        return Ok(format!(
            "Toolchain already installed at {}",
            toolchain_dir.display()
        ));
    }

    // Reset cancel flag
    cancel_flag().store(false, Ordering::SeqCst);

    // สร้างโฟลเดอร์ปลายทาง
    std::fs::create_dir_all(&toolchain_dir)
        .map_err(|e| format!("Failed to create toolchain directory: {}", e))?;

    let part_count = custom_parts.len();
    emit_progress(
        &app_handle,
        "downloading",
        0,
        &format!("Starting download ({} part(s))...", part_count),
    );

    let app_clone = app_handle.clone();
    let dir_clone = toolchain_dir.clone();
    let cancel = cancel_flag();

    // รันใน blocking thread เพราะ reqwest blocking + zip extraction ใช้ CPU หนัก
    let result = tokio::task::spawn_blocking(move || {
        download_and_extract(&app_clone, &custom_parts, &dir_clone, cancel)
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))??;

    Ok(result)
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

/// คืนค่า path สำคัญต่างๆ ของ toolchain ให้ frontend ตรวจสอบ
#[tauri::command]
pub async fn get_toolchain_paths(
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    let base = get_toolchain_dir(&app_handle)?;
    let idf_path = base.join("esp-idf");
    let tools_path = base.join(".espressif");
    let python_env_dir = tools_path.join("python_env");

    // หา python venv ที่มีอยู่
    let python_venv = find_first_venv(&python_env_dir);
    let python_bin = python_venv.as_deref().map(|v| {
        if cfg!(windows) {
            v.join("Scripts").join("python.exe")
        } else {
            v.join("bin").join("python3")
        }
    });

    Ok(serde_json::json!({
        "toolchain_dir":  base.to_string_lossy(),
        "idf_path":       idf_path.to_string_lossy(),
        "tools_path":     tools_path.to_string_lossy(),
        "idf_exists":     idf_path.join("tools/idf.py").exists(),
        "tools_exists":   tools_path.exists(),
        "python_venv":    python_venv.as_deref().map(|p| p.to_string_lossy().to_string()),
        "python_bin":     python_bin.as_deref().map(|p| p.to_string_lossy().to_string()),
        "python_ready":   python_bin.map(|p| p.exists()).unwrap_or(false),
        "ready":          is_toolchain_ready(&base),
    }))
}

// ── Core Logic: Download + Extract ───────────────────────────────────────────

/// สร้าง HTTP client พร้อม cookie store (จำเป็นสำหรับ Google Drive)
fn make_http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::limited(15))
        .cookie_store(true) // จำเป็นสำหรับ Google Drive confirmation cookie
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// แปลง URL ให้เป็น direct download URL
/// - Google Drive share URL → drive.usercontent.google.com/download?id=...&confirm=t
/// - URL อื่นๆ → คืนค่าเดิม
fn resolve_download_url(url: &str) -> String {
    // ตรวจว่าเป็น Google Drive URL หรือไม่
    if !url.contains("drive.google.com") && !url.contains("docs.google.com") {
        return url.to_string();
    }

    // แยก file ID จาก URL หลายรูปแบบ
    // รูปแบบ: /file/d/{id}/
    let file_id = if let Some(part) = url.split("/file/d/").nth(1) {
        part.split('/').next().unwrap_or("").split('?').next().unwrap_or("")
    }
    // รูปแบบ: ?id={id}
    else if let Some(part) = url.split("?id=").nth(1).or_else(|| url.split("&id=").nth(1)) {
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

/// ดาวน์โหลด 1 URL และ append bytes ลงไปใน `dest_file` ที่เปิดค้างอยู่
/// คืน (bytes_downloaded, content_length) เพื่อใช้คำนวณ progress
fn download_part_into(
    app: &AppHandle,
    client: &reqwest::blocking::Client,
    url: &str,
    part_idx: usize,
    part_total: usize,
    global_downloaded: &mut u64,
    global_total: u64,
    dest_file: &mut std::fs::File,
    cancel: &Arc<AtomicBool>,
) -> Result<(), String> {
    // แปลง Google Drive share URL → direct download URL
    let resolved_url = resolve_download_url(url);
    let url_display = if resolved_url != url {
        format!("[GDrive] {}", &resolved_url[..resolved_url.len().min(60)])
    } else {
        url[..url.len().min(60)].to_string()
    };

    emit_progress(
        app,
        "downloading",
        1,
        &format!("[{}/{}] Connecting: {}...", part_idx + 1, part_total, url_display),
    );

    let mut response = client
        .get(&resolved_url)
        .header("Accept", "application/octet-stream,*/*")
        .send()
        .map_err(|e| {
            format!(
                "Download failed (part {}/{}): {}\nURL: {}",
                part_idx + 1,
                part_total,
                e,
                url
            )
        })?;

    // Google Drive ส่ง HTML หน้า confirm สำหรับไฟล์ใหญ่บางครั้ง
    // ถ้า Content-Type เป็น HTML ให้ลอง fallback URL อีกครั้ง
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if content_type.contains("text/html") && url.contains("drive.google.com") {
        // อ่าน HTML และหา download token
        let html = response.text().map_err(|e| e.to_string())?;
        let token = extract_gdrive_token(&html);

        let file_id = resolved_url
            .split("id=").nth(1)
            .and_then(|s| s.split('&').next())
            .unwrap_or("");

        let retry_url = if let Some(t) = token {
            format!(
                "https://drive.usercontent.google.com/download?id={}&export=download&confirm={}",
                file_id, t
            )
        } else {
            format!(
                "https://drive.google.com/uc?export=download&id={}&confirm=t",
                file_id
            )
        };

        emit_progress(app, "downloading", 2, "Following Google Drive confirmation...");

        response = client
            .get(&retry_url)
            .header("Accept", "application/octet-stream,*/*")
            .send()
            .map_err(|e| format!("GDrive retry failed: {}", e))?;
    }

    if !response.status().is_success() {
        return Err(format!(
            "Server error {} for part {}/{}: {}",
            response.status(),
            part_idx + 1,
            part_total,
            url
        ));
    }

    let part_len = response.content_length().unwrap_or(0);
    let mut buf = vec![0u8; 65536]; // 64 KB chunks

    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err("Download cancelled by user.".to_string());
        }

        let n = response
            .read(&mut buf)
            .map_err(|e| format!("Read error (part {}): {}", part_idx + 1, e))?;

        if n == 0 {
            break;
        }

        dest_file
            .write_all(&buf[..n])
            .map_err(|e| format!("Write error: {}", e))?;

        *global_downloaded += n as u64;

        // Download phase = 0–49%
        let percent = if global_total > 0 {
            ((*global_downloaded as f64 / global_total as f64) * 49.0) as u8
        } else {
            ((*global_downloaded / 1_048_576) % 48 + 1) as u8
        };

        let mb_done = *global_downloaded as f64 / 1_048_576.0;
        let mb_total = global_total as f64 / 1_048_576.0;
        let part_mb = part_len as f64 / 1_048_576.0;

        let msg = if part_total > 1 {
            format!(
                "[{}/{}] Downloading... {:.1} / {:.1} MB (part size {:.1} MB)",
                part_idx + 1,
                part_total,
                mb_done,
                mb_total,
                part_mb
            )
        } else {
            format!("Downloading... {:.1} / {:.1} MB", mb_done, mb_total)
        };

        emit_progress(app, "downloading", percent, &msg);
    }

    Ok(())
}

fn download_and_extract(
    app: &AppHandle,
    parts: &[String],
    dest_dir: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let client = make_http_client()?;
    let zip_path = dest_dir.join("_toolchain_download.zip");

    // ─── Phase 1: HEAD requests เพื่อรวม total size (best-effort) ────────────
    let mut global_total: u64 = 0;
    for url in parts {
        if let Ok(resp) = client.head(url.as_str()).send() {
            global_total += resp.content_length().unwrap_or(0);
        }
    }

    // ─── Phase 2: Download ทุกพาร์ต → เขียนลงไฟล์เดียว ──────────────────────
    {
        let mut combined_file = std::fs::File::create(&zip_path)
            .map_err(|e| format!("Cannot create temp file: {}", e))?;

        let mut global_downloaded: u64 = 0;

        for (i, url) in parts.iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                drop(combined_file);
                let _ = std::fs::remove_file(&zip_path);
                return Err("Download cancelled by user.".to_string());
            }

            let result = download_part_into(
                app,
                &client,
                url.as_str(),
                i,
                parts.len(),
                &mut global_downloaded,
                global_total,
                &mut combined_file,
                &cancel,
            );

            if let Err(e) = result {
                drop(combined_file);
                let _ = std::fs::remove_file(&zip_path);
                return Err(e);
            }
        }
    } // combined_file ถูก flush+close ที่นี่

    emit_progress(app, "extracting", 50, "All parts downloaded. Extracting...");

    // ─── Phase 3: Extract ZIP ──────────────────────────────────────────────
    let extract_result = extract_zip_with_progress(app, &zip_path, dest_dir, &cancel);
    let _ = std::fs::remove_file(&zip_path); // ลบ temp เสมอ
    extract_result?;

    if cancel.load(Ordering::SeqCst) {
        return Err("Extraction cancelled.".to_string());
    }

    // ─── Phase 4: Sentinel file ────────────────────────────────────────────
    std::fs::write(dest_dir.join(SENTINEL_FILE), TOOLCHAIN_VERSION)
        .map_err(|e| format!("Failed to write sentinel: {}", e))?;

    emit_progress(
        app,
        "done",
        100,
        &format!("Toolchain ready at {}", dest_dir.display()),
    );

    Ok(format!(
        "Toolchain installed successfully at {}",
        dest_dir.display()
    ))
}

fn extract_zip_with_progress(
    app: &AppHandle,
    zip_path: &Path,
    dest_dir: &Path,
    cancel: &Arc<AtomicBool>,
) -> Result<(), String> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| format!("Cannot open ZIP file: {}", e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Invalid ZIP file: {}", e))?;

    let total = archive.len();

    for i in 0..total {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled during extraction.".to_string());
        }

        let mut zip_file = archive
            .by_index(i)
            .map_err(|e| format!("ZIP read error at index {}: {}", i, e))?;

        // แปลง path ใน ZIP ให้ปลอดภัย (ป้องกัน path traversal)
        let out_path = match zip_file.enclosed_name() {
            Some(p) => dest_dir.join(p),
            None => continue,
        };

        // คำนวณ percent (50–99 = extract phase)
        let percent = 50 + ((i as f64 / total as f64) * 49.0) as u8;

        if i % 500 == 0 || i == total - 1 {
            emit_progress(
                app,
                "extracting",
                percent,
                &format!("Extracting... ({}/{}) {}", i + 1, total, zip_file.name()),
            );
        }

        if zip_file.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Cannot create dir {}: {}", out_path.display(), e))?;
        } else {
            // สร้าง parent directory ถ้ายังไม่มี
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent dir: {}", e))?;
            }

            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create file {}: {}", out_path.display(), e))?;

            std::io::copy(&mut zip_file, &mut out_file)
                .map_err(|e| format!("Cannot extract {}: {}", zip_file.name(), e))?;

            // บน Unix ต้องตั้ง permission bits (executable)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = zip_file.unix_mode() {
                    let _ = std::fs::set_permissions(
                        &out_path,
                        std::fs::Permissions::from_mode(mode),
                    );
                }
            }
        }
    }

    Ok(())
}

// ── Build Firmware using bundled toolchain ────────────────────────────────────

/// สั่ง build firmware โดยใช้ toolchain ที่โหลดมาไว้ใน AppData
/// ไม่ต้องพึ่ง Python หรือ Git ของระบบ Windows เลย!
#[tauri::command]
pub async fn build_firmware_with_toolchain(
    app_handle: AppHandle,
    project_dir: String,
    extra_args: Option<Vec<String>>,
) -> Result<(), String> {
    let toolchain_dir = get_toolchain_dir(&app_handle)?;

    if !is_toolchain_ready(&toolchain_dir) {
        return Err(
            "Toolchain is not installed yet. Please run download_toolchain() first.".to_string(),
        );
    }

    let project_path = std::path::PathBuf::from(&project_dir);
    if !project_path.exists() {
        return Err(format!("Project directory not found: {}", project_dir));
    }

    let idf_path = toolchain_dir.join("esp-idf");
    let tools_path = toolchain_dir.join(".espressif");

    if !idf_path.join("tools/idf.py").exists() {
        return Err(format!(
            "ESP-IDF not found in toolchain at {}",
            idf_path.display()
        ));
    }

    // หา Python venv ใน toolchain
    let python_venv = find_first_venv(&tools_path.join("python_env"))
        .ok_or_else(|| "Python venv not found in toolchain".to_string())?;

    let python_bin = if cfg!(windows) {
        python_venv.join("Scripts").join("python.exe")
    } else {
        python_venv.join("bin").join("python3")
    };

    if !python_bin.exists() {
        return Err(format!(
            "Python binary not found at {}",
            python_bin.display()
        ));
    }

    // สร้าง PATH ที่ชี้ไปยัง toolchain ของเรา
    let custom_path = build_toolchain_path(&tools_path);

    let mut args = vec!["build".to_string()];
    if let Some(extra) = extra_args {
        args.extend(extra);
    }

    emit_progress(&app_handle, "building", 0, "Starting build...");

    let app_clone = app_handle.clone();
    let python_bin_clone = python_bin.clone();
    let idf_py = idf_path.join("tools/idf.py");
    let idf_path_clone = idf_path.clone();
    let tools_path_clone = tools_path.clone();
    let python_venv_clone = python_venv.clone();
    let custom_path_clone = custom_path.clone();

    tokio::task::spawn_blocking(move || {
        use std::process::{Command, Stdio};
        use std::io::BufRead;

        let mut child = Command::new(&python_bin_clone)
            .arg(&idf_py)
            .args(&args)
            .current_dir(&project_path)
            .env("IDF_PATH", &idf_path_clone)
            .env("IDF_TOOLS_PATH", &tools_path_clone)
            .env("IDF_PYTHON_ENV_PATH", &python_venv_clone)
            .env("PATH", &custom_path_clone)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start build: {}", e))?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

        let app_out = app_clone.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().flatten() {
                let _ = app_out.emit("terminal-output", &line);
            }
        });

        let app_err = app_clone.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().flatten() {
                let _ = app_err.emit("terminal-output", format!("\x1b[31m{}\x1b[0m", line));
            }
        });

        let status = child.wait().map_err(|e| format!("Build process error: {}", e))?;

        if status.success() {
            let _ = app_clone.emit(
                "toolchain-progress",
                ToolchainProgress {
                    stage: "done".to_string(),
                    percent: 100,
                    message: "Build complete! Firmware is ready.".to_string(),
                },
            );
            Ok(())
        } else {
            Err("Build failed. Check terminal output for details.".to_string())
        }
    })
    .await
    .map_err(|e| format!("Task panicked: {}", e))?
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

fn build_toolchain_path(tools_path: &Path) -> std::ffi::OsString {
    let mut paths: Vec<PathBuf> = Vec::new();

    // เพิ่ม venv Scripts/bin
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

    // สแกน bin directories ใน tools/
    scan_bin_dirs(&tools_path.join("tools"), 0, 4, &mut paths);
    scan_bin_dirs(tools_path, 0, 4, &mut paths);

    // ต่อท้ายด้วย PATH ของระบบ
    if let Some(system_path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system_path));
    }

    std::env::join_paths(paths).unwrap_or_else(|_| std::ffi::OsString::from(""))
}

fn scan_bin_dirs(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if p.file_name().map(|n| n == "bin").unwrap_or(false) {
                    out.push(p.clone());
                }
                scan_bin_dirs(&p, depth + 1, max_depth, out);
            }
        }
    }
}

// ── Google Drive Token Extractor ──────────────────────────────────────────────

/// หา download confirmation token จาก HTML ที่ Google Drive ส่งมา
/// Google Drive ใช้ form action หรือ query param หลายรูปแบบ
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
