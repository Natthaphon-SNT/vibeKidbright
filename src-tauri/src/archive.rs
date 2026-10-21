//! Guarded SDK archive extraction. No links, special files, privilege bits or
//! partial replacement of an existing file; cancellation is checked while copying.
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) fn extract_zip(
    source: &Path,
    destination: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize, usize, &str),
) -> Result<(), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("Cancelled before extraction".into());
    }
    std::fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    let root = dunce::canonicalize(destination).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(source).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP: {e}"))?;
    let total = archive.len();
    if total > 250000 {
        return Err("SDK ZIP exceeds entry limit".into());
    }
    let mut copied = 0u64;
    for i in 0..total {
        if cancel.load(Ordering::SeqCst) {
            return Err("Cancelled during extraction".into());
        }
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.into_owned();
        let unix_mode = entry.unix_mode();
        let mode = unix_mode.unwrap_or(0);
        if !matches!(mode & 0o170000, 0 | 0o100000 | 0o040000) {
            return Err(format!("SDK archive link/special file rejected: {name}"));
        }
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| format!("Unsafe SDK archive path: {name}"))?;
        let path = crate::ai::change_journal::checked_path(&root, &root.join(relative))?;
        if i % 500 == 0 || i + 1 == total {
            progress(i, total, &name);
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        if entry.size() > 2 * 1024 * 1024 * 1024 {
            return Err("SDK archive file exceeds 2 GiB limit".into());
        }
        let parent = path.parent().ok_or("Missing SDK output parent")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        crate::ai::change_journal::checked_path(&root, &path)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        let mut buffer = [0u8; 65536];
        let mut size = 0u64;
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err("Cancelled during extraction".into());
            }
            let n = entry.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            size += n as u64;
            copied += n as u64;
            if size > 2 * 1024 * 1024 * 1024 || copied > 16 * 1024 * 1024 * 1024 {
                return Err("SDK ZIP expansion exceeds safety limit".into());
            }
            temporary
                .write_all(&buffer[..n])
                .map_err(|e| e.to_string())?;
        }
        temporary
            .flush()
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = unix_mode {
                temporary
                    .as_file()
                    .set_permissions(std::fs::Permissions::from_mode(mode & 0o777))
                    .map_err(|e| e.to_string())?;
            }
        }
        crate::ai::change_journal::checked_path(&root, &path)?;
        temporary.persist(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn zip(path: &Path, name: &str, content: &str) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content.as_bytes()).unwrap();
        writer.finish().unwrap();
    }
    #[test]
    fn valid_archive_extracts_and_traversal_never_reaches_outside() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sdk.zip");
        let root = temp.path().join("tools");
        zip(&source, "bin/tool.txt", "SDK data");
        extract_zip(&source, &root, &AtomicBool::new(false), |_, _, _| {}).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("bin/tool.txt")).unwrap(),
            "SDK data"
        );
        zip(&source, "../outside.txt", "escape");
        assert!(extract_zip(&source, &root, &AtomicBool::new(false), |_, _, _| {}).is_err());
        assert!(!temp.path().join("outside.txt").exists());
    }
    #[test]
    fn cancelled_extraction_does_not_replace_existing_data() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sdk.zip");
        let root = temp.path().join("tools");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("tool.txt"), "original").unwrap();
        zip(&source, "tool.txt", "new");
        let cancel = AtomicBool::new(false);
        assert!(extract_zip(&source, &root, &cancel, |_, _, _| cancel
            .store(true, Ordering::SeqCst))
        .is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("tool.txt")).unwrap(),
            "original"
        );
    }
    #[test]
    fn archive_links_are_rejected_before_creation() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sdk.zip");
        let root = temp.path().join("tools");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&source).unwrap());
        writer
            .add_symlink(
                "link",
                "../outside",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        assert!(extract_zip(&source, &root, &AtomicBool::new(false), |_, _, _| {}).is_err());
        assert!(!root.join("link").exists());
    }
    #[cfg(unix)]
    #[test]
    fn existing_parent_symlink_is_not_followed() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sdk.zip");
        let root = temp.path().join("tools");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("bin")).unwrap();
        zip(&source, "bin/tool", "bad");
        assert!(extract_zip(&source, &root, &AtomicBool::new(false), |_, _, _| {}).is_err());
        assert!(!outside.join("tool").exists());
    }
}
