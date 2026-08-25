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

use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

// ── Constants ─────────────────────────────────────────────────────────────────

/// GitHub Releases framework & toolchain — มี 2 ไฟล์แยกกัน:
///   frameworks.zip  (~300 MB)  → แตกลง toolchain/
///   tools.zip       (~1.98 GB) → แตกลง toolchain/
///
/// โครงสร้างหลังแตกไฟล์:
///   toolchain/
///     esp-idf/          ← มาจาก frameworks.zip
///     .espressif/       ← มาจาก tools.zip
///     .toolchain_ready  ← sentinel file
///
/// รองรับ Google Drive URL ด้วย (แปลงอัตโนมัติ)
const TOOLCHAIN_PARTS: &[(&str, &str)] = &[
    // (URL, label)
    (
        "https://github.com/Natthaphon-SNT/vibeKidbright/releases/download/framework/frameworks.zip",
        "frameworks (~300 MB)",
    ),
    (
        "https://github.com/Natthaphon-SNT/vibeKidbright/releases/download/toolchain/tools.zip",
        "tools (~1.98 GB)",
    ),
];

/// ชื่อ "sentinel file" — ถ้าไฟล์นี้มีอยู่ แปลว่า toolchain สมบูรณ์แล้ว
const SENTINEL_FILE: &str = ".toolchain_ready";

/// เวอร์ชัน toolchain
const TOOLCHAIN_VERSION: &str = "1.0.1";


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
    // ถ้ามี custom URL ให้ใช้เป็น single-entry มิฉะนั้นใช้ค่าเริ่มต้น (GitHub Release)
    let parts: Vec<(String, String)> = url
        .filter(|u| !u.trim().is_empty())
        .map(|u| vec![(u, "custom".to_string())])
        .unwrap_or_else(|| {
            TOOLCHAIN_PARTS
                .iter()
                .map(|(u, l)| (u.to_string(), l.to_string()))
                .collect()
        });

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

    emit_progress(
        &app_handle,
        "downloading",
        0,
        &format!("Starting download ({} file(s)): {}",
            parts.len(),
            parts.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>().join(" + ")
        ),
    );

    let app_clone = app_handle.clone();
    let dir_clone = toolchain_dir.clone();
    let cancel = cancel_flag();

    // รันใน blocking thread
    let result = tokio::task::spawn_blocking(move || {
        download_and_extract(&app_clone, &parts, &dir_clone, cancel)
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

/// แก้ไข Python venv paths ที่ยังชี้ไปยังเครื่อง dev เดิม (C:\Users\Acer\...)
/// เรียกใช้บนเครื่องที่ติดตั้ง toolchain แล้วแต่ build ไม่ผ่านด้วย "No Python at ..." error
#[tauri::command]
pub async fn repair_toolchain_paths(app_handle: AppHandle) -> Result<String, String> {
    let toolchain_dir = get_toolchain_dir(&app_handle)?;
    if !toolchain_dir.exists() {
        return Err("Toolchain not installed yet. Please install it first.".to_string());
    }

    // Run the venv path patcher
    let dir = toolchain_dir.clone();
    tokio::task::spawn_blocking(move || patch_venv_paths(&dir))
        .await
        .map_err(|e| format!("Task panicked: {}", e))??;

    Ok(format!(
        "Python venv paths repaired successfully in {}",
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
        .connect_timeout(std::time::Duration::from_secs(30))  // ค้าง connect ได้สูงสุด 30s
        .timeout(std::time::Duration::from_secs(600))         // total timeout 10 นาที
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .redirect(reqwest::redirect::Policy::limited(15))
        .cookie_store(true) // จำเป็นสำหรับ Google Drive confirmation cookie
        .tcp_keepalive(std::time::Duration::from_secs(30))    // keepalive ป้องกัน connection drop
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


fn download_and_extract(
    app: &AppHandle,
    parts: &[(String, String)], // (url, label)
    dest_dir: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let client = make_http_client()?;
    let total_parts = parts.len();

    // ─── Phase 1: HEAD เพื่อรวม total size (best-effort) ───────────────────
    let global_total: u64 = {
        let mut total = 0u64;
        for (url, _) in parts {
            let resolved = resolve_download_url(url);
            if let Ok(resp) = client.head(&resolved)
                .header("Accept", "application/octet-stream,*/*")
                .send() {
                total += resp.content_length().unwrap_or(0);
            }
        }
        total
    };

    // ─── Phase 2: ดาวน์โหลด ทุกไฟล์พร้อมกัน (parallel) ──────────────────
    // Shared atomic counter สำหรับ progress รวมจากทุก thread
    let global_downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let zip_paths: Vec<PathBuf> = (0..total_parts)
        .map(|i| dest_dir.join(format!("_download_{}.zip", i)))
        .collect();

    // ใช้ thread::scope ดาวน์โหลดทุกไฟล์พร้อมกัน
    emit_progress(app, "downloading", 1,
        &format!("เริ่มดาวน์โหลด {} ไฟล์พร้อมกัน...", total_parts)
    );

    let download_errors: Vec<Result<(), String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = parts
            .iter()
            .zip(zip_paths.iter())
            .enumerate()
            .map(|(idx, ((url, label), zip_path))| {
                let client = client.clone();
                let app = app.clone();
                let cancel = cancel.clone();
                let gd = global_downloaded.clone();
                let url = url.clone();
                let label = label.clone();
                let zip_path = zip_path.clone();

                scope.spawn(move || -> Result<(), String> {
                    let resolved = resolve_download_url(&url);
                    emit_progress(&app, "downloading", 1,
                        &format!("[{}/{}] Connecting {}...", idx + 1, total_parts, label));

                    // Connect
                    let mut resp = client.get(&resolved)
                        .header("Accept", "application/octet-stream,*/*")
                        .send()
                        .map_err(|e| format!("Connect failed ({}): {}", label, e))?;

                    // จัดการ GDrive HTML confirm page
                    let ct = resp.headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("").to_string();
                    if ct.contains("text/html") && url.contains("drive.google.com") {
                        let html = resp.text().map_err(|e| e.to_string())?;
                        let token = extract_gdrive_token(&html);
                        let file_id = resolved.split("id=").nth(1)
                            .and_then(|s| s.split('&').next()).unwrap_or("");
                        let retry = if let Some(t) = token {
                            format!("https://drive.usercontent.google.com/download?id={}&export=download&confirm={}", file_id, t)
                        } else {
                            format!("https://drive.google.com/uc?export=download&id={}&confirm=t", file_id)
                        };
                        resp = client.get(&retry)
                            .header("Accept", "application/octet-stream,*/*")
                            .send().map_err(|e| format!("GDrive retry failed: {}", e))?;
                    }

                    if !resp.status().is_success() {
                        return Err(format!("Server {} for {}", resp.status(), label));
                    }

                    let part_size = resp.content_length().unwrap_or(0);

                    // Stream ลงไฟล์ด้วย BufWriter (8 MB read buffer + 4 MB write buffer)
                    let raw_file = std::fs::File::create(&zip_path)
                        .map_err(|e| format!("Cannot create temp ({}): {}", label, e))?;
                    let mut file = BufWriter::with_capacity(4 * 1024 * 1024, raw_file);
                    let mut buf = vec![0u8; 8 * 1024 * 1024]; // 8 MB read buffer
                    let mut local_done: u64 = 0;

                    // Throttle: emit progress ทุก 3 วินาที ลด UI lag
                    let mut last_emit = std::time::Instant::now();
                    let emit_interval = std::time::Duration::from_secs(3);
                    // Speed tracking
                    let download_start = std::time::Instant::now();
                    let mut bytes_since_last_emit: u64 = 0;

                    loop {
                        if cancel.load(Ordering::SeqCst) {
                            let _ = std::fs::remove_file(&zip_path);
                            return Err("Cancelled.".to_string());
                        }

                        let n = resp.read(&mut buf)
                            .map_err(|e| format!("Read error ({}): {}", label, e))?;
                        if n == 0 { break; }

                        file.write_all(&buf[..n])
                            .map_err(|e| format!("Write error ({}): {}", label, e))?;

                        local_done += n as u64;
                        bytes_since_last_emit += n as u64;
                        let combined = gd.fetch_add(n as u64, Ordering::Relaxed) + n as u64;

                        // Emit ทุก 3 วินาที (ไม่ใช่ทุก chunk) เพื่อลด UI lag
                        if last_emit.elapsed() >= emit_interval {
                            let elapsed_secs = last_emit.elapsed().as_secs_f64();
                            last_emit = std::time::Instant::now();

                            // คำนวณ speed จาก bytes ที่โหลดในช่วง interval นี้
                            let speed_mbps = (bytes_since_last_emit as f64 / 1_048_576.0) / elapsed_secs;
                            bytes_since_last_emit = 0;

                            // คำนวณ ETA จาก average speed ตลอดการโหลด
                            let total_elapsed = download_start.elapsed().as_secs_f64();
                            let avg_speed = if total_elapsed > 0.0 { local_done as f64 / total_elapsed } else { 1.0 };
                            let remaining_bytes = if part_size > local_done { part_size - local_done } else { 0 };
                            let eta_secs = if avg_speed > 0.0 { remaining_bytes as f64 / avg_speed } else { 0.0 };

                            let eta_str = if eta_secs > 3600.0 {
                                format!("{:.0}h {:.0}m", eta_secs / 3600.0, (eta_secs % 3600.0) / 60.0)
                            } else if eta_secs > 60.0 {
                                format!("{:.0}m {:.0}s", eta_secs / 60.0, eta_secs % 60.0)
                            } else {
                                format!("{:.0}s", eta_secs)
                            };

                            let percent = if global_total > 0 {
                                ((combined as f64 / global_total as f64) * 49.0) as u8
                            } else {
                                (combined / 1_048_576 % 48 + 1) as u8
                            };

                            let mb_done_total = combined as f64 / 1_048_576.0;
                            let mb_total = global_total as f64 / 1_048_576.0;
                            let mb_local = local_done as f64 / 1_048_576.0;
                            let mb_part  = part_size as f64  / 1_048_576.0;

                            emit_progress(&app, "downloading", percent,
                                &format!("[{}/{}] {} — {:.1}/{:.1} MB  ⚡ {:.1} MB/s  ⏱ ETA {}",
                                    idx + 1, total_parts, label,
                                    mb_local, mb_part,
                                    speed_mbps,
                                    if remaining_bytes == 0 { "done".to_string() } else { eta_str }));

                            let _ = mb_done_total + mb_total; // suppress unused warning
                        }
                    }

                    // Flush BufWriter ก่อนปิด
                    file.flush()
                        .map_err(|e| format!("Flush error ({}): {}", label, e))?;

                    Ok(())
                })
            })
            .collect();

        handles.into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("Thread panicked".to_string())))
            .collect()
    });

    // ตรวจสอบ error จาก download threads
    for (i, res) in download_errors.into_iter().enumerate() {
        if let Err(e) = res {
            // ลบ temp ที่สำเร็จแล้ว
            for zip_path in &zip_paths {
                let _ = std::fs::remove_file(zip_path);
            }
            return Err(format!("ดาวน์โหลดไฟล์ที่ {} ล้มเหลว: {}", i + 1, e));
        }
    }

    // ─── Phase 3: Extract ทีละไฟล์ (sequential) ───────────────────────────────
    for (file_idx, ((_, label), zip_path)) in parts.iter().zip(zip_paths.iter()).enumerate() {
        if cancel.load(Ordering::SeqCst) {
            for zp in &zip_paths { let _ = std::fs::remove_file(zp); }
            return Err("Cancelled before extraction.".to_string());
        }

        let extract_base = 50u8 + (file_idx as u8 * (49 / total_parts as u8));
        let extract_range = 49u8 / total_parts as u8;

        emit_progress(app, "extracting", extract_base,
            &format!("[{}/{}] Extracting {}...", file_idx + 1, total_parts, label));

        let res = extract_zip_ranged(app, zip_path, dest_dir, &cancel, extract_base, extract_range);
        let _ = std::fs::remove_file(zip_path);
        res?;
    }

    // ─── Phase 4: Patch venv paths (fix machine-specific Python paths) ─────
    // The bundled venv was created on the developer's machine (e.g. C:\Users\Acer\...).
    // We rewrite pyvenv.cfg `home =` lines to point to the bundled idf-python in
    // the actual install directory so Python can be found on any machine.
    emit_progress(app, "extracting", 99, "Patching Python venv paths for this machine...");
    if let Err(e) = patch_venv_paths(dest_dir) {
        // Non-fatal — log but continue.  Build will fail with a clearer message if needed.
        eprintln!("[Toolchain] pyvenv patch warning: {}", e);
    }

    // ─── Phase 5: Sentinel file ────────────────────────────────────────────
    std::fs::write(dest_dir.join(SENTINEL_FILE), TOOLCHAIN_VERSION)
        .map_err(|e| format!("Failed to write sentinel: {}", e))?;

    emit_progress(app, "done", 100,
        &format!("Toolchain v{} ready!", TOOLCHAIN_VERSION));

    Ok(format!("Toolchain installed at {}", dest_dir.display()))
}

/// `base_percent` = \u0e40\u0e23\u0e34\u0e48\u0e21\u0e15\u0e49\u0e19\u0e17\u0e35\u0e48 (0–99), `range` = \u0e08\u0e33\u0e19\u0e27\u0e19 percent \u0e17\u0e35\u0e48\u0e43\u0e0a\u0e49\u0e2a\u0e33\u0e2b\u0e23\u0e31\u0e1a\u0e44\u0e1f\u0e25\u0e4c\u0e19\u0e35\u0e49
fn extract_zip_ranged(
    app: &AppHandle,
    zip_path: &Path,
    dest_dir: &Path,
    cancel: &Arc<AtomicBool>,
    base_percent: u8,
    range: u8,
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

        let out_path = match zip_file.enclosed_name() {
            Some(p) => dest_dir.join(p),
            None => continue,
        };

        let percent = base_percent + ((i as f64 / total as f64) * range as f64) as u8;

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
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Cannot create parent dir: {}", e))?;
            }

            let raw_out = std::fs::File::create(&out_path)
                .map_err(|e| format!("Cannot create file {}: {}", out_path.display(), e))?;
            let mut out_file = BufWriter::with_capacity(512 * 1024, raw_out); // 512 KB write buffer

            std::io::copy(&mut zip_file, &mut out_file)
                .map_err(|e| format!("Cannot extract {}: {}", zip_file.name(), e))?;

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

    // หา Python จาก toolchain โดยตรง (ไม่พึ่ง venv launcher stub ที่มี hardcoded path)
    // Priority: 1) idf-python bundled  2) venv python (ถ้า pyvenv.cfg ถูก patch แล้ว)
    let (python_bin, python_venv) = find_toolchain_python(&tools_path)
        .ok_or_else(|| {
            format!(
                "Cannot find Python in toolchain at {}. \
                Try clicking 'Repair Python Paths' or reinstall the toolchain.",
                tools_path.display()
            )
        })?;

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

        let mut cmd = Command::new(&python_bin_clone);
        cmd.arg(&idf_py)
            .args(&args)
            .current_dir(&project_path)
            .env("IDF_PATH", &idf_path_clone)
            .env("IDF_TOOLS_PATH", &tools_path_clone)
            .env("IDF_PYTHON_ENV_PATH", &python_venv_clone)
            .env("PATH", &custom_path_clone)
            // Force Python to use UTF-8 regardless of Windows system locale (cp874, cp932, etc.)
            // This fixes kconfgen UnicodeDecodeError on non-English Windows machines.
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONLEGACYWINDOWSSTDIO", "0");
        if let Some(rom_elf_dir) = crate::esp_idf::find_esp_rom_elf_dir(&tools_path_clone) {
            cmd.env("ESP_ROM_ELF_DIR", rom_elf_dir);
        }
        let mut child = cmd.stdin(Stdio::null())
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
        let home = cfg.lines()
            .find(|l| l.trim().to_lowercase().starts_with("home") && l.contains('='))
            .and_then(|l| l.splitn(2, '=').nth(1))
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
                    eprintln!("[Toolchain] Using venv python (home= valid): {}", venv_python.display());
                    return Some((venv_python, venv_dir));
                }
            } else {
                // home= is stale (C:\Users\Acer\...) — auto-patch now
                eprintln!("[Toolchain] pyvenv.cfg home= not found ({}), auto-patching...", home_dir);
                if let Some(ref fresh_python) = idf_python_bin {
                    // Patch pyvenv.cfg inline
                    let new_home = fresh_python.parent()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let patched = cfg.lines().map(|line| {
                        if line.trim().to_lowercase().starts_with("home") && line.contains('=') {
                            format!("home = {}", new_home)
                        } else {
                            line.to_string()
                        }
                    }).collect::<Vec<_>>().join("\r\n");
                    let _ = std::fs::write(&cfg_path, &patched);
                    eprintln!("[Toolchain] Patched {} home= → {}", cfg_path.display(), new_home);
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
            .ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        versions.sort(); // highest version last
        versions.reverse();
        for ver_dir in &versions {
            // python.exe directly in version dir (e.g. idf-python/3.11.2/python.exe)
            if cfg!(windows) {
                let candidate = ver_dir.join("python.exe");
                if candidate.exists() { return Some(candidate); }
            } else {
                for name in &["python3", "python"] {
                    let candidate = ver_dir.join(name);
                    if candidate.exists() { return Some(candidate); }
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
    if depth > max_depth { return None; }
    let Ok(entries) = std::fs::read_dir(dir) else { return None; };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
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
    if !toolchain_dir.join(SENTINEL_FILE).exists() { return; }
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
/// ZIP was built on the developer's machine (e.g. C:\Users\Acer\...) that path is
/// baked in and will not exist on any other PC, causing:
///   No Python at 'C:\Users\Acer\...' error.
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
            let versions: Vec<_> = std::fs::read_dir(&idf_python_dir).ok()?.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            for ver in &versions {
                // Direct python.exe in version dir
                if cfg!(windows) {
                    if ver.join("python.exe").exists() { return Some(ver.clone()); }
                } else {
                    if ver.join("python3").exists() || ver.join("python").exists() { return Some(ver.clone()); }
                }
                // python.exe inside bin/
                let bin = ver.join("bin");
                if bin.exists() { return Some(bin); }
            }
        }
        // Fallback: scan tools/ for any directory containing python.exe at depth ≤3
        find_python_dir(&tools_dir, 0, 3)
    })();

    // ── Step 2: find all pyvenv.cfg files under toolchain/ ────────────────
    let mut cfg_files: Vec<PathBuf> = Vec::new();
    collect_pyvenv_cfgs(toolchain_dir, &mut cfg_files);

    if cfg_files.is_empty() {
        eprintln!("[Patch] No pyvenv.cfg found under {} — skipping patch", toolchain_dir.display());
        return Ok(());
    }

    eprintln!("[Patch] Found {} pyvenv.cfg file(s)", cfg_files.len());

    // ── Step 3: rewrite `home = ...` in each cfg ──────────────────────────
    for cfg_path in &cfg_files {
        let content = match std::fs::read_to_string(cfg_path) {
            Ok(c) => c,
            Err(e) => { eprintln!("[Patch] Cannot read {}: {}", cfg_path.display(), e); continue; }
        };

        // Determine the correct `home` value:
        // Prefer bundled idf-python dir; fall back to the venv's own Scripts/bin dir.
        let new_home: String = if let Some(ref bp) = bundled_python_bin_dir {
            bp.to_string_lossy().into_owned()
        } else {
            // If no bundled python found, keep the venv's own Scripts/bin
            let venv_dir = cfg_path.parent().unwrap_or(cfg_path);
            let bin = if cfg!(windows) { venv_dir.join("Scripts") } else { venv_dir.join("bin") };
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
            eprintln!("[Patch] Patched {} → home = {}", cfg_path.display(), new_home);
        }
    }

    Ok(())
}

/// Recursively collect all `pyvenv.cfg` files under `dir` (max depth 8).
fn collect_pyvenv_cfgs(dir: &Path, out: &mut Vec<PathBuf>) {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        if depth > 8 { return; }
        let Ok(entries) = std::fs::read_dir(dir) else { return; };
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
    if depth > max_depth { return None; }
    let Ok(entries) = std::fs::read_dir(dir) else { return None; };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
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
        let bin = if cfg!(windows) { venv.join("Scripts") } else { venv.join("bin") };
        if bin.exists() { paths.push(bin); }
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

/// Scan an ESP-IDF tools directory structure and collect all directories that
/// contain executable files.  Handles the two common ESP-IDF tool layouts:
///   a) <tool>/<version>/bin/<exe>      → adds bin/
///   b) <tool>/<version>/<exe>          → adds version/ directly (ninja, idf-python)
///   c) <tool>/<variant>/<ver>/bin/     → adds bin/ (xtensa-esp-elf sub-variants)
fn scan_esp_tool_dirs(tools_dir: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(tools) = std::fs::read_dir(tools_dir) else { return; };
    for tool_entry in tools.flatten() {
        let tool_path = tool_entry.path();
        if !tool_path.is_dir() { continue; }
        let Ok(versions) = std::fs::read_dir(&tool_path) else { continue; };
        for ver_entry in versions.flatten() {
            let ver_path = ver_entry.path();
            if !ver_path.is_dir() { continue; }
            // Layout a: bin/ subdir
            let bin_dir = ver_path.join("bin");
            if bin_dir.is_dir() { paths.push(bin_dir); }
            // Layout b: executables directly in version dir
            if dir_has_executables(&ver_path) { paths.push(ver_path.clone()); }
            // Layout c: one more level deep
            let Ok(sub_entries) = std::fs::read_dir(&ver_path) else { continue; };
            for sub_entry in sub_entries.flatten() {
                let sub = sub_entry.path();
                if !sub.is_dir() { continue; }
                let sub_bin = sub.join("bin");
                if sub_bin.is_dir() { paths.push(sub_bin); }
                if dir_has_executables(&sub) { paths.push(sub); }
            }
        }
    }
}

/// Returns true if a directory directly contains at least one executable file.
fn dir_has_executables(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false; };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        if cfg!(windows) {
            if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("exe")).unwrap_or(false) {
                return true;
            }
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = p.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 { return true; }
                }
            }
        }
    }
    false
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

// ── Unit Tests ────────────────────────────────────────────────────────────────
// Pure-logic tests for URL resolution, token extraction and filesystem scans.

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
