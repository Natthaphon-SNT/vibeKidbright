//! Durable, review-gated file changes. Filesystem writes use a persisted intent
//! and exact-content preconditions; recovery never overwrites a conflicting file.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Status {
    Pending,
    Applying,
    Applied,
    RollingBack,
    Rejected,
    Undone,
    Conflict,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Change {
    pub revision: String,
    pub root: PathBuf,
    pub path: PathBuf,
    pub message: String,
    pub before: Option<String>,
    pub proposed: String,
    pub status: Status,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Data {
    schema: u32,
    changes: Vec<Change>,
    rejected: BTreeSet<String>,
}
pub(crate) struct Journal {
    file: Option<PathBuf>,
    data: Data,
}

pub(crate) fn checked_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let root = dunce::canonicalize(root).map_err(|e| format!("Workspace unavailable: {e}"))?;
    let relative = path
        .strip_prefix(&root)
        .map_err(|_| "Change is outside its recorded workspace.")?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Invalid reviewed file path.".into());
    }
    let mut cursor = root.clone();
    for part in relative.components() {
        let name = part.as_os_str().to_string_lossy();
        #[cfg(windows)]
        {
            let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            if name.contains(':')
                || name.ends_with([' ', '.'])
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
            {
                return Err("Invalid Windows file alias.".into());
            }
        }
        cursor.push(part);
        match std::fs::symlink_metadata(&cursor) {
            Ok(meta) => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err("Reviewed paths must not contain reparse points.".into());
                    }
                }
                if meta.file_type().is_symlink() {
                    return Err("Reviewed paths must not contain links.".into());
                }
                let canonical = dunce::canonicalize(&cursor).map_err(|e| e.to_string())?;
                if !canonical.starts_with(&root) {
                    return Err("Reviewed path escaped the workspace.".into());
                }
                cursor = canonical;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(cursor)
}
fn read(path: &Path) -> Result<Option<String>, String> {
    if std::fs::metadata(path).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
        return Err("Reviewed source exceeds the 2 MiB safety limit; file was preserved.".into());
    }
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot read reviewed file: {e}")),
    }
}
fn same_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        dunce::simplified(left)
            .to_string_lossy()
            .replace('/', "\\")
            .eq_ignore_ascii_case(
                &dunce::simplified(right)
                    .to_string_lossy()
                    .replace('/', "\\"),
            )
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}
fn fingerprint(root: &Path, path: &Path, before: &Option<String>, proposed: &str) -> String {
    crate::intelligence::digest(
        serde_json::to_vec(&(root, path, before, proposed))
            .unwrap()
            .as_slice(),
    )
}
fn write_checked(
    change: &Change,
    expected: &Option<String>,
    next: &Option<String>,
) -> Result<(), String> {
    let path = checked_path(&change.root, &change.path)?;
    if read(&path)? != *expected {
        return Err(
            "File changed after review. Your version was preserved; reopen the diff.".into(),
        );
    }
    if let Some(content) = next {
        use std::io::Write;
        let parent = path.parent().ok_or("Missing reviewed parent.")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        checked_path(&change.root, &path)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temp.write_all(content.as_bytes())
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        if read(&path)? != *expected {
            return Err("File changed while preparing reviewed write; nothing was applied.".into());
        }
        if expected.is_none() {
            temp.persist_noclobber(&path).map_err(|e| e.to_string())?;
        } else {
            temp.persist(&path).map_err(|e| e.to_string())?;
        }
    } else if expected.is_some() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
impl Journal {
    pub fn open(file: PathBuf) -> Result<Self, String> {
        if file.exists()
            && std::fs::metadata(&file).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024
        {
            return Err(
                "Review journal exceeds recovery size limit; preserve it for manual recovery."
                    .into(),
            );
        }
        let mut journal = if file.exists() {
            let data: Data = serde_json::from_slice(
                &std::fs::read(&file).map_err(|e| e.to_string())?,
            )
            .map_err(|e| format!("Review journal is corrupt; preserve it for recovery: {e}"))?;
            if data.schema != 1 {
                return Err("Unsupported review journal schema.".into());
            }
            Self {
                file: Some(file),
                data,
            }
        } else {
            Self {
                file: Some(file),
                data: Data {
                    schema: 1,
                    ..Default::default()
                },
            }
        };
        let mut next = journal.data.clone();
        let interrupted_undo: BTreeSet<_> = next
            .changes
            .iter()
            .filter(|c| c.status == Status::RollingBack)
            .map(|c| c.message.clone())
            .collect();
        for change in &mut next.changes {
            if matches!(change.status, Status::Applying | Status::RollingBack) {
                let disk = checked_path(&change.root, &change.path).and_then(|p| read(&p));
                change.status = match disk {
                    Ok(value) if value == Some(change.proposed.clone()) => Status::Applied,
                    Ok(value) if value == change.before => {
                        if change.status == Status::RollingBack {
                            Status::Undone
                        } else {
                            Status::Pending
                        }
                    }
                    _ => Status::Conflict,
                };
            }
        }
        // An interrupted multi-file Undo remains a build-blocking review until
        // the user retries it. Do not silently accept a half-restored program.
        for change in &mut next.changes {
            if interrupted_undo.contains(&change.message) && change.status == Status::Applied {
                change.status = Status::Conflict;
            }
        }
        journal.save(next)?;
        Ok(journal)
    }
    fn save(&mut self, next: Data) -> Result<(), String> {
        let bytes = serde_json::to_vec(&next).map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("Review history is full; no files were changed.".into());
        }
        if let Some(file) = &self.file {
            super::config::write_json_atomic(file, &next)?;
        }
        self.data = next;
        Ok(())
    }
    pub fn pending(&self, path: &Path) -> Option<Change> {
        self.data
            .changes
            .iter()
            .rev()
            .find(|c| {
                same_path(&c.path, path)
                    && matches!(
                        c.status,
                        Status::Pending | Status::Applying | Status::RollingBack | Status::Conflict
                    )
            })
            .cloned()
    }
    pub fn pending_map(&self) -> BTreeMap<PathBuf, String> {
        self.data
            .changes
            .iter()
            .filter(|c| {
                matches!(
                    c.status,
                    Status::Pending | Status::Applying | Status::RollingBack | Status::Conflict
                )
            })
            .map(|c| (c.path.clone(), c.proposed.clone()))
            .collect()
    }
    pub fn revision_for(&self, root: &Path) -> String {
        let states: Vec<_> = self
            .data
            .changes
            .iter()
            .filter(|c| c.root == root && !matches!(c.status, Status::Rejected | Status::Undone))
            .map(|c| (&c.revision, &c.status))
            .collect();
        crate::intelligence::digest(&serde_json::to_vec(&states).unwrap())
    }
    pub fn propose(
        &mut self,
        root: &Path,
        path: &Path,
        message: &str,
        proposed: &str,
    ) -> Result<Change, String> {
        if proposed.len() > 2 * 1024 * 1024 || self.data.changes.len() >= 2048 {
            return Err("Reviewed change exceeds size/history limits.".into());
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "Proposal path is outside its workspace.")?
            .to_path_buf();
        let root = dunce::canonicalize(root).map_err(|e| e.to_string())?;
        let path = checked_path(&root, &root.join(relative))?;
        let before = read(&path)?;
        let revision = fingerprint(&root, &path, &before, proposed);
        if self.data.rejected.contains(&revision) {
            return Err("Identical rejected/unsuccessful patch was blocked. Propose a different correction.".into());
        }
        if let Some(existing) = self.pending(&path) {
            if existing.status != Status::Pending {
                return Err(
                    "Unresolved recovery conflict; Keep/Undo must be reviewed manually.".into(),
                );
            }
            if existing.revision == revision {
                return Ok(existing);
            }
            return Err("A different change is already pending for this file. Review it before proposing another.".into());
        }
        let change = Change {
            revision,
            root,
            path,
            message: message.into(),
            before,
            proposed: proposed.into(),
            status: Status::Pending,
        };
        let mut next = self.data.clone();
        next.changes.push(change.clone());
        self.save(next)?;
        Ok(change)
    }
    pub fn propose_batch(
        &mut self,
        root: &Path,
        message: &str,
        files: &[(PathBuf, String)],
    ) -> Result<Vec<Change>, String> {
        let mut staged = Self {
            file: None,
            data: self.data.clone(),
        };
        let mut changes = Vec::new();
        for (path, content) in files {
            changes.push(staged.propose(root, path, message, content)?);
        }
        self.save(staged.data)?;
        Ok(changes)
    }
    pub fn reject(&mut self, path: &Path, revision: &str) -> Result<(), String> {
        let change = self.pending(path).ok_or("No pending change found.")?;
        if change.revision != revision || change.status != Status::Pending {
            return Err("Review revision changed or requires recovery. Reload the diff.".into());
        }
        let mut next = self.data.clone();
        let change = next
            .changes
            .iter_mut()
            .rev()
            .find(|c| same_path(&c.path, path) && c.revision == revision)
            .unwrap();
        change.status = Status::Rejected;
        next.rejected.insert(revision.into());
        self.save(next)
    }
    pub fn apply(&mut self, authorizations: &[(PathBuf, String)]) -> Result<Vec<Change>, String> {
        self.apply_with(authorizations, write_checked)
    }
    fn apply_with(
        &mut self,
        authorizations: &[(PathBuf, String)],
        mut write: impl FnMut(&Change, &Option<String>, &Option<String>) -> Result<(), String>,
    ) -> Result<Vec<Change>, String> {
        let mut changes = Vec::new();
        for (path, revision) in authorizations {
            let change = self
                .pending(path)
                .ok_or("Pending review is no longer available.")?;
            if change.revision != *revision || change.status != Status::Pending {
                return Err("Stale review authorization; reopen the current diff.".into());
            }
            checked_path(&change.root, &change.path)?;
            if changes.iter().any(|c: &Change| c.path == change.path) {
                return Err("Duplicate authorization for one file is not permitted.".into());
            }
            if read(&change.path)? != change.before {
                return Err(
                    "Reviewed file was edited after the proposal. No files changed.".into(),
                );
            }
            changes.push(change);
        }
        let roots: BTreeSet<_> = changes.iter().map(|c| c.root.clone()).collect();
        let _leases = roots
            .iter()
            .map(|root| crate::sdk_process::lease(root, false))
            .collect::<Result<Vec<_>, _>>()?;
        let mut next = self.data.clone();
        for c in &changes {
            next.changes
                .iter_mut()
                .rev()
                .find(|x| x.path == c.path && x.revision == c.revision)
                .unwrap()
                .status = Status::Applying;
        }
        self.save(next)?; // durable intent BEFORE touching source
        for (index, change) in changes.iter().enumerate() {
            if let Err(error) = write(change, &change.before, &Some(change.proposed.clone())) {
                let mut restored = true;
                for written in changes[..index].iter().rev() {
                    if write_checked(written, &Some(written.proposed.clone()), &written.before)
                        .is_err()
                    {
                        restored = false;
                    }
                }
                let mut next = self.data.clone();
                for c in &changes {
                    next.changes
                        .iter_mut()
                        .rev()
                        .find(|x| x.path == c.path && x.revision == c.revision)
                        .unwrap()
                        .status = if restored {
                        Status::Pending
                    } else {
                        Status::Conflict
                    };
                }
                self.save(next)?;
                return Err(format!("Reviewed batch failed: {error}. Rollback complete={restored}; recovery records retained."));
            }
        }
        let mut next = self.data.clone();
        for c in &changes {
            next.changes
                .iter_mut()
                .rev()
                .find(|x| x.path == c.path && x.revision == c.revision)
                .unwrap()
                .status = Status::Applied;
        }
        self.save(next)?;
        Ok(changes)
    }
    pub fn undo_message(&mut self, message: &str) -> Result<Vec<PathBuf>, String> {
        let mut per_file: BTreeMap<PathBuf, Change> = BTreeMap::new();
        for change in self.data.changes.iter().filter(|c| {
            c.message == message && matches!(c.status, Status::Applied | Status::Conflict)
        }) {
            per_file
                .entry(change.path.clone())
                .and_modify(|first| first.proposed = change.proposed.clone())
                .or_insert_with(|| change.clone());
        }
        let pending: Vec<_> = self
            .data
            .changes
            .iter()
            .filter(|c| c.message == message && c.status == Status::Pending)
            .cloned()
            .collect();
        if per_file.is_empty() && pending.is_empty() {
            return Err(
                "No reversible changes for this message. Recovery conflicts require manual review."
                    .into(),
            );
        }
        let roots: BTreeSet<_> = per_file.values().map(|c| c.root.clone()).collect();
        let _leases = roots
            .iter()
            .map(|root| crate::sdk_process::lease(root, false))
            .collect::<Result<Vec<_>, _>>()?;
        for change in per_file.values() {
            checked_path(&change.root, &change.path)?;
            if read(&change.path)? != Some(change.proposed.clone()) {
                return Err("Undo stopped: a file was edited later. Your work and recovery records were preserved.".into());
            }
        }
        let mut next = self.data.clone();
        for c in next.changes.iter_mut().filter(|c| {
            c.message == message && matches!(c.status, Status::Applied | Status::Conflict)
        }) {
            let merged = &per_file[&c.path];
            c.before = merged.before.clone();
            c.proposed = merged.proposed.clone();
            c.status = Status::RollingBack;
        }
        self.save(next)?;
        for change in per_file.values().rev() {
            if let Err(error) =
                write_checked(change, &Some(change.proposed.clone()), &change.before)
            {
                return Err(format!("Undo incomplete: {error}. Recovery journal retained; restart to reconcile exact file states."));
            }
        }
        let mut next = self.data.clone();
        for c in next.changes.iter_mut().filter(|c| c.message == message) {
            if matches!(c.status, Status::RollingBack | Status::Pending) {
                next.rejected.insert(c.revision.clone());
                c.status = Status::Undone;
            }
        }
        self.save(next)?;
        Ok(per_file
            .into_keys()
            .chain(pending.into_iter().map(|c| c.path))
            .collect())
    }
    pub fn applied_in(&self, root: &Path) -> Vec<Change> {
        let root = dunce::canonicalize(root).unwrap_or_else(|_| root.into());
        self.data
            .changes
            .iter()
            .filter(|c| c.root == root && c.status == Status::Applied)
            .cloned()
            .collect()
    }
    pub fn retire_project(&mut self, root: &Path) -> Result<(), String> {
        let mut next = self.data.clone();
        for c in next.changes.iter_mut().filter(|c| c.root == root) {
            if !matches!(c.status, Status::Rejected | Status::Undone) {
                next.rejected.insert(c.revision.clone());
                c.status = Status::Undone;
            }
        }
        self.save(next)
    }
}
static JOURNAL: OnceLock<Mutex<Journal>> = OnceLock::new();
pub(crate) fn initialize(directory: &Path) -> Result<(), String> {
    let journal = Journal::open(directory.join("review-journal.json"))?;
    JOURNAL
        .set(Mutex::new(journal))
        .map_err(|_| "Review journal already initialized.".into())
}
pub(crate) fn with<T>(action: impl FnOnce(&mut Journal) -> Result<T, String>) -> Result<T, String> {
    #[cfg(test)]
    let journal = JOURNAL.get_or_init(|| {
        Mutex::new(Journal {
            file: None,
            data: Data {
                schema: 1,
                ..Default::default()
            },
        })
    });
    #[cfg(not(test))]
    let journal = JOURNAL
        .get()
        .ok_or("Review recovery journal is unavailable. No changes are permitted.")?;
    let mut guard = journal
        .lock()
        .map_err(|_| "Review journal is unavailable.")?;
    action(&mut guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Journal) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("workspace");
        std::fs::create_dir(&root).unwrap();
        let file = root.join("main.c");
        std::fs::write(&file, "original").unwrap();
        let journal = Journal::open(dir.path().join("journal.json")).unwrap();
        (dir, root, file, journal)
    }
    #[test]
    fn proposal_and_rejection_never_modify_disk_and_require_current_revision() {
        let (_d, root, file, mut j) = fixture();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
        assert!(j.apply(&[(file.clone(), "stale".into())]).is_err());
        j.reject(&file, &p.revision).unwrap();
        assert!(j.propose(&root, &file, "other", "changed").is_err());
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
    }
    #[test]
    fn pending_apply_and_undo_survive_reopen_and_preserve_later_user_edits() {
        let (_d, root, file, mut j) = fixture();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        let saved = j.file.clone().unwrap();
        drop(j);
        let mut j = Journal::open(saved.clone()).unwrap();
        j.apply(&[(file.clone(), p.revision)]).unwrap();
        drop(j);
        let mut j = Journal::open(saved.clone()).unwrap();
        std::fs::write(&file, "user work").unwrap();
        assert!(j.undo_message("m").is_err());
        assert_eq!(read(&file).unwrap().as_deref(), Some("user work"));
        std::fs::write(&file, "changed").unwrap();
        j.undo_message("m").unwrap();
        drop(j);
        let j = Journal::open(saved).unwrap();
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
        assert!(j.pending(&file).is_none());
    }
    #[test]
    fn new_file_is_reviewed_and_undo_preserves_a_concurrent_new_file() {
        let (_d, root, _file, mut j) = fixture();
        let path = root.join("new.c");
        let p = j.propose(&root, &path, "m", "new").unwrap();
        assert!(!path.exists());
        std::fs::write(&path, "user-created").unwrap();
        assert!(j.apply(&[(path.clone(), p.revision.clone())]).is_err());
        assert_eq!(read(&path).unwrap().as_deref(), Some("user-created"));
        std::fs::remove_file(&path).unwrap();
        j.apply(&[(path.clone(), p.revision)]).unwrap();
        j.undo_message("m").unwrap();
        assert!(!path.exists());
    }
    #[test]
    fn multiple_applied_revisions_in_one_message_restore_the_first_original() {
        let (_d, root, file, mut j) = fixture();
        for text in ["first", "second"] {
            let p = j.propose(&root, &file, "m", text).unwrap();
            j.apply(&[(file.clone(), p.revision)]).unwrap();
        }
        j.undo_message("m").unwrap();
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
    }
    #[test]
    fn recovery_reconciles_a_crash_between_filesystem_and_journal_commit() {
        let (_d, root, file, mut j) = fixture();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        let mut data = j.data.clone();
        data.changes[0].status = Status::Applying;
        j.save(data).unwrap();
        write_checked(&p, &p.before, &Some(p.proposed.clone())).unwrap();
        let saved = j.file.clone().unwrap();
        drop(j);
        let mut j = Journal::open(saved).unwrap();
        assert_eq!(j.data.changes[0].status, Status::Applied);
        j.undo_message("m").unwrap();
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
    }
    #[test]
    fn recovery_does_not_guess_when_user_content_conflicts() {
        let (_d, root, file, mut j) = fixture();
        j.propose(&root, &file, "m", "changed").unwrap();
        let mut data = j.data.clone();
        data.changes[0].status = Status::Applying;
        j.save(data).unwrap();
        std::fs::write(&file, "user work").unwrap();
        let saved = j.file.clone().unwrap();
        drop(j);
        let mut j = Journal::open(saved).unwrap();
        assert_eq!(j.pending(&file).unwrap().status, Status::Conflict);
        assert!(j.undo_message("m").is_err());
        assert_eq!(read(&file).unwrap().as_deref(), Some("user work"));
    }
    #[test]
    fn journal_write_failure_and_invalid_paths_preserve_originals() {
        let (dir, root, file, mut j) = fixture();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        let bad = dir.path().join("not-a-file");
        std::fs::create_dir(&bad).unwrap();
        j.file = Some(bad);
        assert!(j.apply(&[(file.clone(), p.revision)]).is_err());
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
        assert!(j
            .propose(&root, &root.join("../outside.c"), "m", "bad")
            .is_err());
    }
    #[test]
    fn stale_file_preflight_prevents_partial_batch_application() {
        let (_d, root, file, mut j) = fixture();
        let other = root.join("other.c");
        std::fs::write(&other, "other").unwrap();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        let q = j.propose(&root, &other, "m", "new other").unwrap();
        std::fs::write(&other, "user edit").unwrap();
        assert!(j
            .apply(&[(file.clone(), p.revision), (other, q.revision)])
            .is_err());
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
    }
    #[test]
    fn corrupt_journal_is_preserved_and_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("journal.json");
        std::fs::write(&file, "{broken").unwrap();
        assert!(Journal::open(file.clone()).is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "{broken");
    }
    #[test]
    fn failure_on_second_write_rolls_back_first_and_retains_pending_review() {
        let (_d, root, file, mut j) = fixture();
        let other = root.join("other.c");
        std::fs::write(&other, "other").unwrap();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        let q = j.propose(&root, &other, "m", "changed other").unwrap();
        let mut writes = 0;
        let result = j.apply_with(
            &[(file.clone(), p.revision), (other.clone(), q.revision)],
            |c, b, a| {
                writes += 1;
                if writes == 2 {
                    Err("injected disk failure".into())
                } else {
                    write_checked(c, b, a)
                }
            },
        );
        assert!(result.unwrap_err().contains("Rollback complete=true"));
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
        assert_eq!(read(&other).unwrap().as_deref(), Some("other"));
        assert!(j.data.changes.iter().all(|c| c.status == Status::Pending));
    }
    #[test]
    fn interrupted_partial_undo_blocks_build_and_can_be_retried() {
        let (_d, root, file, mut j) = fixture();
        let other = root.join("other.c");
        std::fs::write(&other, "other").unwrap();
        let changes = j
            .propose_batch(
                &root,
                "m",
                &[
                    (file.clone(), "changed".into()),
                    (other.clone(), "changed other".into()),
                ],
            )
            .unwrap();
        j.apply(
            &changes
                .iter()
                .map(|c| (c.path.clone(), c.revision.clone()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let mut data = j.data.clone();
        for c in &mut data.changes {
            c.status = Status::RollingBack;
        }
        j.save(data).unwrap();
        write_checked(
            &changes[0],
            &Some(changes[0].proposed.clone()),
            &changes[0].before,
        )
        .unwrap();
        let saved = j.file.clone().unwrap();
        drop(j);
        let mut j = Journal::open(saved).unwrap();
        assert_eq!(j.data.changes[0].status, Status::Undone);
        assert_eq!(j.pending(&other).unwrap().status, Status::Conflict);
        j.undo_message("m").unwrap();
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
        assert_eq!(read(&other).unwrap().as_deref(), Some("other"));
    }
    #[test]
    fn duplicate_authorization_is_rejected_before_mutation() {
        let (_d, root, file, mut j) = fixture();
        let p = j.propose(&root, &file, "m", "changed").unwrap();
        assert!(j
            .apply(&[
                (file.clone(), p.revision.clone()),
                (file.clone(), p.revision)
            ])
            .is_err());
        assert_eq!(read(&file).unwrap().as_deref(), Some("original"));
    }
}
