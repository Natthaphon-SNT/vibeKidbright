//! Review AI-created projects as a group, including files installed by recipes.
//! Undo only removes an unchanged, explicitly recorded tree; never recurse-delete.
use serde::{Deserialize,Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectReview {
    project_path: String,
    project_name: String,
    previous_project_path: Option<String>,
    files: Vec<String>,
    message_id: String,
    revision: String,
}

#[derive(Clone,Serialize,Deserialize)]
struct CreatedProject {
    root: PathBuf,
    resolved_root: PathBuf,
    previous: Option<PathBuf>,
    message_id: String,
    files: BTreeMap<PathBuf, Vec<u8>>,
    directories: BTreeSet<PathBuf>,
    #[serde(default)]
    undo_destination: Option<PathBuf>,
}

impl CreatedProject {
    fn view(&self) -> ProjectReview {
        ProjectReview {
            project_path: self.root.to_string_lossy().into_owned(),
            project_name: self
                .root
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            previous_project_path: self
                .previous
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            files: self
                .files
                .keys()
                .filter(|relative| !crate::ai::compatibility::is_internal_project_file(relative))
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .collect(),
            message_id: self.message_id.clone(),
            revision: crate::intelligence::digest(&serde_json::to_vec(&(
                &self.root,&self.message_id,&self.files,&self.directories,
                crate::ai::change_journal::with(|journal|Ok(journal.revision_for(&dunce::canonicalize(&self.root).unwrap_or_else(|_|self.root.clone())))).unwrap_or_else(|_|"recovery-unavailable".into()),
            )).unwrap()),
        }
    }

    fn verify_root(&self) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(&self.root).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || std::fs::canonicalize(&self.root).map_err(|e| e.to_string())? != self.resolved_root
        {
            return Err(
                "The created project directory has moved or become a link. No files were removed."
                    .into(),
            );
        }
        Ok(())
    }

    fn relative_file(&self, path: &Path) -> Result<PathBuf, String> {
        self.verify_root()?;
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| "File is outside the created project")?;
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("Invalid project file path".into());
        }
        let mut checked = self.root.clone();
        for component in relative.components() {
            checked.push(component);
            match std::fs::symlink_metadata(&checked) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(
                        "Project contains a link. Review it manually before Keep or Undo.".into(),
                    )
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(relative.to_path_buf())
    }
}

#[derive(Clone,Default,Serialize,Deserialize)]
struct ReviewStore {
    projects: HashMap<PathBuf, CreatedProject>,
    #[serde(default)]
    quarantines: BTreeMap<PathBuf,PathBuf>,
}

fn read_tree(root: &Path) -> Result<(BTreeMap<PathBuf, Vec<u8>>, BTreeSet<PathBuf>), String> {
    read_tree_matching(root, None)
}

fn read_tree_matching(
    root: &Path,
    expected: Option<&CreatedProject>,
) -> Result<(BTreeMap<PathBuf, Vec<u8>>, BTreeSet<PathBuf>), String> {
    let mut files = BTreeMap::new();
    let mut directories = BTreeSet::new();
    let mut remaining = vec![root.to_path_buf()];
    while let Some(directory) = remaining.pop() {
        for entry in std::fs::read_dir(&directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            let relative = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_path_buf();
            if let Some(expected) = expected {
                if (metadata.is_dir() && !expected.directories.contains(&relative))
                    || (metadata.is_file()
                        && expected
                            .files
                            .get(&relative)
                            .map(|bytes| bytes.len() as u64)
                            != Some(metadata.len()))
                {
                    return Err("Undo stopped: files or folders have changed since AI created them. Your files were kept.".into());
                }
            }
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "Project contains a link: {}. No files were removed.",
                    relative.display()
                ));
            } else if metadata.is_dir() {
                directories.insert(relative);
                remaining.push(path);
            } else if metadata.is_file() {
                files.insert(relative, std::fs::read(path).map_err(|e| e.to_string())?);
            } else {
                return Err(
                    "Project contains an unsupported file type. No files were removed.".into(),
                );
            }
        }
    }
    Ok((files, directories))
}

impl ReviewStore {
    fn contains_project_path(&self, path: &Path) -> bool {
        let resolved = std::fs::canonicalize(path).ok();
        self.projects.values().any(|project| {
            project.root == path
                || resolved
                    .as_ref()
                    .is_some_and(|path| path.starts_with(&project.resolved_root))
        })
    }

    fn register(
        &mut self,
        root: &Path,
        previous: Option<PathBuf>,
        message_id: &str,
    ) -> Result<ProjectReview, String> {
        let resolved_root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
        let (files, directories) = read_tree(root)?;
        let project = CreatedProject {
            root: root.into(),
            resolved_root,
            previous,
            message_id: message_id.into(),
            files,
            directories,
            undo_destination: None,
        };
        project.verify_root()?;
        let view = project.view();
        self.projects.insert(root.into(), project);
        Ok(view)
    }

    fn record_write(&mut self, path: &Path, content: &[u8]) {
        for project in self.projects.values_mut() {
            if let Ok(relative) = project.relative_file(path) {
                let mut parent = relative.parent();
                while let Some(directory) = parent.filter(|p| !p.as_os_str().is_empty()) {
                    project.directories.insert(directory.into());
                    parent = directory.parent();
                }
                project.files.insert(relative, content.into());
                break;
            }
        }
    }

    fn keep(
        &mut self,
        root: &Path,
        diffs: &mut HashMap<PathBuf, String>,
    ) -> Result<Vec<PathBuf>, String> {
        let project = self
            .projects
            .get_mut(root)
            .ok_or("No pending project review found.")?;
        project.verify_root()?;
        let paths: Vec<_> = diffs
            .keys()
            .filter(|p| p.starts_with(root))
            .cloned()
            .collect();
        for path in &paths {
            let relative = project.relative_file(path)?;
            if let Some(expected) = project.files.get(&relative) {
                if std::fs::read(path).map_err(|e| e.to_string())? != *expected {
                    return Err("A file changed outside the AI review. Save your version elsewhere before applying the proposed changes.".into());
                }
            } else {
                return Err("A proposed file is not part of this project review. Review it separately before keeping the project.".into());
            }
        }
        for path in &paths {
            let content = diffs.get(path).unwrap();
            std::fs::write(path, content).map_err(|e| {
                format!(
                    "Could not apply changes: {}. Project review is still available.",
                    e
                )
            })?;
            let relative = project.relative_file(path)?;
            project.files.insert(relative, content.as_bytes().into());
            diffs.remove(path);
        }
        self.projects.remove(root);
        Ok(paths)
    }

    fn undo_with_diffs(
        &mut self,
        root: &Path,
        diffs: &mut HashMap<PathBuf, String>,
    ) -> Result<Option<PathBuf>, String> {
        let previous = self.undo(root)?;
        diffs.retain(|p, _| !p.starts_with(root));
        Ok(previous)
    }

    fn undo(&mut self, root: &Path) -> Result<Option<PathBuf>, String> {
        let _lease=crate::sdk_process::lease(root,false)?;
        let project = self
            .projects
            .get(root)
            .cloned()
            .ok_or("No pending project review found.")?;
        project.verify_root()?;
        let (current_files, current_directories) = read_tree_matching(root, Some(&project))?;
        // Validate the entire tree before deleting anything. User edits/additions,
        // build output and even extra empty folders must be retained.
        if current_files != project.files || current_directories != project.directories {
            return Err("Undo stopped: files or folders have changed since AI created them. Your files were kept. Keep the project or save your changes elsewhere before trying Undo.".into());
        }
        // Keep the whole tree in a same-volume recovery location. Rename is one
        // filesystem operation; a crash cannot leave half the generated tree deleted.
        let parent=root.parent().ok_or("Created project has no parent.")?;
        let container=tempfile::Builder::new().prefix(".vibekidbright-undo-")
            .tempdir_in(parent).map_err(|e|e.to_string())?.keep();
        if !dunce::canonicalize(&container).map_err(|e|e.to_string())?
            .starts_with(dunce::canonicalize(parent).map_err(|e|e.to_string())?) {
            return Err("Undo recovery location escaped the project parent.".into());
        }
        let destination=container.join("project");
        let mut prepared=self.clone();
        prepared.projects.get_mut(root).unwrap().undo_destination=Some(destination.clone());
        prepared.quarantines.insert(root.into(),destination.clone());
        persist(&prepared)?;
        *self=prepared;
        std::fs::rename(root,&destination).map_err(|e|format!("Could not move unchanged project into recovery: {e}. Review retained."))?;
        if !read_tree(&destination).is_ok_and(|tree|tree==(project.files.clone(),project.directories.clone())) {
            let restored=std::fs::rename(&destination,root).is_ok();
            return Err(format!("Project changed during Undo; restore complete={restored}. Recovery location: {}",destination.display()));
        }
        let previous = project.previous.clone();
        self.projects.remove(root);
        Ok(previous)
    }
}

static REVIEWS: OnceLock<Mutex<ReviewStore>> = OnceLock::new();
static JOURNAL_FILE: OnceLock<PathBuf> = OnceLock::new();
#[derive(Serialize,Deserialize)]
struct SavedReviews { schema:u32, store:ReviewStore }
fn persist(store:&ReviewStore)->Result<(),String> {
    if serde_json::to_vec(store).map_err(|e|e.to_string())?.len()>64*1024*1024{return Err("Project recovery history exceeds its 64 MiB limit; source was preserved.".into());}
    if let Some(path)=JOURNAL_FILE.get() {
        return crate::ai::config::write_json_atomic(path,&SavedReviews{schema:1,store:store.clone()});
    }
    #[cfg(test)] {Ok(())}
    #[cfg(not(test))] {Err("Project recovery journal is unavailable.".into())}
}
pub(crate) fn initialize(directory:&Path)->Result<(),String> {
    let path=directory.join("created-project-reviews.json");
    let mut store=if path.exists() {
        if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>64*1024*1024 {return Err("Project recovery journal exceeds size limit.".into());}
        let saved:SavedReviews=serde_json::from_slice(&std::fs::read(&path).map_err(|e|e.to_string())?).map_err(|e|format!("Project recovery journal is corrupt; retain it for recovery: {e}"))?;
        if saved.schema!=1{return Err("Unsupported project recovery schema.".into());} saved.store
    } else {ReviewStore::default()};
    let roots:Vec<_>=store.projects.keys().cloned().collect();
    for root in roots {
        let project=store.projects[&root].clone();
        if let Some(destination)=project.undo_destination.as_ref().filter(|_|!root.exists()) {
            if read_tree(destination).is_ok_and(|tree|tree==(project.files.clone(),project.directories.clone())) {
                crate::ai::change_journal::with(|journal|journal.retire_project(&dunce::simplified(&root)))?;
                store.projects.remove(&root);continue;
            }
        }
        // Reconcile an approved write committed just before a process crash.
        for change in crate::ai::change_journal::with(|journal|Ok(journal.applied_in(&root)))? {
            if std::fs::read_to_string(&change.path).ok().as_deref()==Some(&change.proposed) {store.record_write(&change.path,change.proposed.as_bytes());}
        }
    }
    JOURNAL_FILE.set(path).map_err(|_|"Project journal already initialized.")?;
    persist(&store)?;
    REVIEWS.set(Mutex::new(store)).map_err(|_|"Project review state already initialized.".into())
}
fn reviews() -> &'static Mutex<ReviewStore> {
    REVIEWS.get_or_init(|| Mutex::new(ReviewStore::default()))
}

pub(crate) fn register_created_project(
    root: &Path,
    previous: Option<PathBuf>,
    message_id: &str,
) -> Result<ProjectReview, String> {
    let mut store=reviews().lock().map_err(|_|"Project review unavailable.")?;
    let mut next=store.clone();let view=next.register(root,previous,message_id)?;
    persist(&next)?;*store=next;Ok(view)
}

pub(crate) fn record_written_file(app: &AppHandle, path: &Path, content: &str) -> Result<(),String> {
    let mut store = reviews().lock().unwrap();
    let mut next=store.clone();next.record_write(path,content.as_bytes());persist(&next)?;*store=next;
    let view = store
        .projects
        .values()
        .find(|project| path.starts_with(&project.root))
        .map(CreatedProject::view);
    drop(store);
    if let Some(view) = view {
        let _ = app.emit(
            "ai-project-review-pending",
            serde_json::to_string(&view).unwrap(),
        );
    }
    Ok(())
}

pub(crate) fn emit_pending(app:&AppHandle,path:&Path)->Result<(),String> {
    let store=reviews().lock().map_err(|_|"Project review unavailable.")?;
    if let Some(project)=store.projects.values().find(|project|path.starts_with(&project.root)) {
        app.emit("ai-project-review-pending",serde_json::to_string(&project.view()).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    }
    Ok(())
}

pub(crate) fn has_pending_project(path: &Path) -> bool {
    reviews().lock().unwrap().contains_project_path(path)
}

pub(crate) fn ensure_project_kept(path: &Path) -> Result<(), String> {
    if has_pending_project(path) {
        Err("Choose Keep or Undo in the new project review before running commands or building this project.".into())
    } else {
        Ok(())
    }
}

pub(crate) fn has_pending_message(message_id: &str) -> bool {
    reviews()
        .lock()
        .unwrap()
        .projects
        .values()
        .any(|p| p.message_id == message_id)
}

#[tauri::command]
pub(crate) fn get_pending_project_reviews() -> Vec<ProjectReview> {
    let mut pending:Vec<_>=reviews().lock().unwrap().projects.values().map(CreatedProject::view).collect();
    pending.sort_by(|a,b|a.project_path.cmp(&b.project_path));pending
}

#[tauri::command]
pub(crate) fn keep_ai_project(app_handle:AppHandle,project_path:String,expected_revision:String)->Result<String,String> {
    let _operation=crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root=PathBuf::from(&project_path);
    let mut store=reviews().lock().map_err(|_|"Project review unavailable.")?;
    let project=store.projects.get(&root).ok_or("No pending project review.")?;
    project.verify_root()?;
    if project.view().revision!=expected_revision {return Err("Project review changed; reload before choosing Keep.".into());}
    let resolved=dunce::canonicalize(&root).map_err(|e|e.to_string())?;
    let applied=crate::ai::change_journal::with(|journal|{
        let authorizations=journal.pending_map().keys().filter(|path|path.starts_with(&resolved))
            .filter_map(|path|journal.pending(path).map(|change|(path.clone(),change.revision))).collect::<Vec<_>>();
        journal.apply(&authorizations)
    })?;
    let mut next=store.clone();
    for change in &applied {next.record_write(&change.path,change.proposed.as_bytes());}
    next.keep(&root,&mut HashMap::new())?;
    persist(&next)?;*store=next;drop(store);
    for change in applied {let _=app_handle.emit("file-modified",json!({"path":change.path}).to_string());}
    let _=app_handle.emit("ai-project-review-resolved",json!({"projectPath":project_path,"action":"keep"}).to_string());
    Ok("Project kept.".into())
}

#[tauri::command]
pub(crate) fn undo_ai_project(app_handle:AppHandle,project_path:String,expected_revision:String)->Result<String,String> {
    let _operation=crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root=PathBuf::from(&project_path);
    let mut store=reviews().lock().map_err(|_|"Project review unavailable.")?;
    let project=store.projects.get(&root).ok_or("No pending project review.")?;
    if project.view().revision!=expected_revision {return Err("Project review changed; reload before choosing Undo.".into());}
    let resolved=dunce::canonicalize(&root).map_err(|e|e.to_string())?;
    let mut next=store.clone();
    let previous=next.undo(&root)?;
    crate::ai::change_journal::with(|journal|journal.retire_project(&resolved))?;
    persist(&next)?;*store=next;drop(store);
    let _=app_handle.emit("force-project-dir",previous.map(|p|p.to_string_lossy().into_owned()).unwrap_or_else(||".".into()));
    let _=app_handle.emit("ai-project-review-resolved",json!({"projectPath":project_path,"action":"undo"}).to_string());
    Ok("Project moved to a recorded recovery directory; previous workspace restored.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, ReviewStore) {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("new_project");
        std::fs::create_dir_all(root.join("main")).unwrap();
        std::fs::write(root.join("main/main.c"), "generated firmware").unwrap();
        std::fs::write(root.join("CMakeLists.txt"), "generated config").unwrap();
        let mut store = ReviewStore::default();
        store
            .register(&root, Some(base.path().join("previous")), "message")
            .unwrap();
        (base, root, store)
    }

    #[test]
    fn creation_is_queryable_without_a_write_file_call() {
        let (_base, root, store) = fixture();
        let view = store.projects[&root].view();
        assert_eq!(view.files, vec!["CMakeLists.txt", "main/main.c"]);
        assert_eq!(view.message_id, "message");
        assert!(root.join("main/main.c").exists());
    }

    #[test]
    fn command_guard_covers_project_aliases_and_subdirectories_only() {
        let (base, root, store) = fixture();
        assert!(store.contains_project_path(&root));
        assert!(store.contains_project_path(&root.join("main")));
        assert!(store.contains_project_path(&root.join("main/..")));
        assert!(!store.contains_project_path(base.path()));
        #[cfg(windows)]
        assert!(store.contains_project_path(&PathBuf::from(root.to_string_lossy().to_lowercase())));
    }

    #[test]
    fn keep_applies_proposals_and_preserves_other_projects_pending_changes() {
        let (base, root, mut store) = fixture();
        let outside = base.path().join("previous/main.c");
        let mut diffs = HashMap::from([
            (root.join("main/main.c"), "proposed firmware".into()),
            (outside.clone(), "other project proposal".into()),
        ]);
        let applied = store.keep(&root, &mut diffs).unwrap();
        assert_eq!(applied, vec![root.join("main/main.c")]);
        assert_eq!(
            std::fs::read_to_string(root.join("main/main.c")).unwrap(),
            "proposed firmware"
        );
        assert!(root.join("CMakeLists.txt").exists());
        assert!(diffs.contains_key(&outside));
        assert!(!diffs.contains_key(&root.join("main/main.c")));
        assert!(!store.projects.contains_key(&root));
    }

    #[test]
    fn keeping_a_reference_project_preserves_code_and_private_metadata() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join("reference");
        std::fs::create_dir_all(root.join("main")).unwrap();
        for path in [
            "CMakeLists.txt",
            "main/CMakeLists.txt",
            "main/main.c",
            "sdkconfig",
        ] {
            std::fs::write(root.join(path), "boilerplate").unwrap();
        }
        for (relative, content) in
            crate::ai::compatibility::files("kidbright32_v13", "button_buzzer").unwrap()
        {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let before = read_tree(&root).unwrap();
        assert_eq!(before.0.len(), 9);
        let mut store = ReviewStore::default();
        let review = store.register(&root, None, "create only").unwrap();
        assert_eq!(review.files.len(), 8);
        assert!(!review
            .files
            .iter()
            .any(|path| path.contains(".vibekidbright") || path.ends_with(".md")));
        store.keep(&root, &mut HashMap::new()).unwrap();
        assert_eq!(read_tree(&root).unwrap(), before);
        assert!(store.projects.is_empty());
    }

    #[test]
    fn undo_includes_private_metadata_even_when_it_is_hidden_from_review() {
        let (_base, root, mut store) = fixture();
        let private = root.join(crate::ai::compatibility::PROJECT_METADATA_PATH);
        std::fs::create_dir_all(private.parent().unwrap()).unwrap();
        std::fs::write(&private, "internal metadata").unwrap();
        store.record_write(&private, b"internal metadata");
        crate::ai::compatibility::hide_internal_metadata(&root).unwrap();
        assert_eq!(store.projects[&root].view().files.len(), 2);
        assert!(store.projects[&root]
            .files
            .contains_key(Path::new(crate::ai::compatibility::PROJECT_METADATA_PATH)));
        store.undo(&root).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn keep_conflict_preserves_files_proposals_and_review_for_retry() {
        let (_base, root, mut store) = fixture();
        let file = root.join("main/main.c");
        let mut diffs = HashMap::from([(file.clone(), "AI proposal".into())]);
        std::fs::write(&file, "user changes").unwrap();
        assert!(store.keep(&root, &mut diffs).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "user changes");
        assert!(store.projects.contains_key(&root));
        assert!(diffs.contains_key(&file));
    }

    #[test]
    fn project_undo_clears_only_its_pending_diffs_after_success() {
        let (base, root, mut store) = fixture();
        let outside = base.path().join("previous/main.c");
        let file = root.join("main/main.c");
        let mut diffs = HashMap::from([
            (file.clone(), "AI proposal".into()),
            (outside.clone(), "other proposal".into()),
        ]);
        std::fs::write(&file, "user changes").unwrap();
        assert!(store.undo_with_diffs(&root, &mut diffs).is_err());
        assert!(diffs.contains_key(&file));
        std::fs::write(&file, "generated firmware").unwrap();
        store.undo_with_diffs(&root, &mut diffs).unwrap();
        assert!(!diffs.contains_key(&file));
        assert!(diffs.contains_key(&outside));
    }

    #[test]
    fn undo_removes_only_the_recorded_tree_and_returns_previous_workspace() {
        let (base, root, mut store) = fixture();
        let previous = base.path().join("previous");
        std::fs::create_dir(&previous).unwrap();
        std::fs::write(previous.join("important.c"), "existing user firmware").unwrap();
        assert_eq!(store.undo(&root).unwrap(), Some(previous.clone()));
        assert!(!root.exists());
        assert_eq!(
            std::fs::read_to_string(previous.join("important.c")).unwrap(),
            "existing user firmware"
        );
        assert!(store.projects.is_empty());
    }

    #[test]
    fn changed_or_additional_files_block_undo_before_any_deletion() {
        for change in ["edit", "add", "directory"] {
            let (_base, root, mut store) = fixture();
            match change {
                "edit" => std::fs::write(root.join("main/main.c"), "user changes").unwrap(),
                "add" => std::fs::write(root.join("notes.txt"), "user notes").unwrap(),
                _ => std::fs::create_dir(root.join("user_folder")).unwrap(),
            }
            assert!(store.undo(&root).is_err());
            assert!(root.join("CMakeLists.txt").exists());
            assert!(root.join("main/main.c").exists());
            assert!(store.projects.contains_key(&root));
        }
    }

    #[test]
    fn ai_created_and_written_files_remain_undoable() {
        let (_base, root, mut store) = fixture();
        std::fs::create_dir(root.join("extra")).unwrap();
        let added = root.join("extra/driver.c");
        std::fs::write(&added, "AI driver").unwrap();
        store.record_write(&added, b"AI driver");
        std::fs::write(root.join("main/main.c"), "AI revision").unwrap();
        store.record_write(&root.join("main/main.c"), b"AI revision");
        assert!(store.undo(&root).is_ok());
        assert!(!root.exists());
    }

    #[test]
    fn traversal_paths_cannot_be_recorded_as_project_files() {
        let (base, root, mut store) = fixture();
        let outside = base.path().join("user.txt");
        std::fs::write(&outside, "keep me").unwrap();
        store.record_write(&root.join("../user.txt"), b"not owned");
        assert_eq!(store.projects[&root].files.len(), 2);
        assert!(store.undo(&root).is_ok());
        assert_eq!(std::fs::read_to_string(outside).unwrap(), "keep me");
    }

    #[test]
    fn undo_without_a_previous_project_returns_no_workspace() {
        let (_base, root, mut store) = fixture();
        store.projects.get_mut(&root).unwrap().previous = None;
        assert_eq!(store.undo(&root).unwrap(), None);
    }

    #[test]
    fn restart_roundtrip_preserves_snapshot_and_project_undo_recovery_tree() {
        let (_base,root,store)=fixture();
        let bytes=serde_json::to_vec(&SavedReviews{schema:1,store}).unwrap();
        let mut recovered:SavedReviews=serde_json::from_slice(&bytes).unwrap();
        let original=recovered.store.projects[&root].files.clone();
        recovered.store.undo(&root).unwrap();
        let destination=&recovered.store.quarantines[&root];
        assert!(!root.exists()); assert!(destination.is_dir());
        assert_eq!(read_tree(destination).unwrap().0,original);
        let saved=serde_json::to_vec(&recovered).unwrap();
        let reopened:SavedReviews=serde_json::from_slice(&saved).unwrap();
        assert_eq!(&reopened.store.quarantines[&root],destination);
    }

    #[cfg(unix)]
    #[test]
    fn undo_never_follows_a_link_to_user_files() {
        let (base, root, mut store) = fixture();
        let outside = base.path().join("user.txt");
        std::fs::write(&outside, "keep me").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        assert!(store.undo(&root).is_err());
        assert!(root.join("CMakeLists.txt").exists());
        assert_eq!(std::fs::read_to_string(outside).unwrap(), "keep me");
    }
}
