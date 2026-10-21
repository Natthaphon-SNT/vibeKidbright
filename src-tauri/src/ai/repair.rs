//! Bounded compiler repair. The model can propose; only a revision-bound native
//! UI command can apply an attempt. Verification always uses configured idf.py.
use super::change_journal::{self, Change};
use crate::intelligence::{
    diagnostics::{self, Diagnostic},
    digest,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    AwaitingProposal,
    AwaitingApproval,
    Applying,
    Building,
    Failed,
    Succeeded,
    Cancelled,
    RecoveryRequired,
    Exhausted,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Session {
    root: PathBuf,
    id: String,
    phase: Phase,
    max_attempts: u8,
    attempts: u8,
    changes: Vec<Change>,
    message: Option<String>,
    diagnostics: Vec<Diagnostic>,
    failed: BTreeSet<String>,
    last_error: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Store {
    schema: u32,
    sessions: BTreeMap<PathBuf, Session>,
}
static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static FILE: OnceLock<PathBuf> = OnceLock::new();
#[derive(Clone, Serialize)]
pub(crate) struct Review {
    session: Session,
    revision: String,
}
impl Session {
    fn review(&self) -> Review {
        Review {
            session: self.clone(),
            revision: digest(&serde_json::to_vec(self).unwrap()),
        }
    }
    fn check(&self, revision: &str) -> Result<(), String> {
        if self.review().revision == revision {
            Ok(())
        } else {
            Err("Repair revision changed. Review the current patch before authorizing it.".into())
        }
    }
    fn authorize(&mut self, revision: &str) -> Result<Vec<(PathBuf, String)>, String> {
        self.check(revision)?;
        if !(1..=3).contains(&self.max_attempts)
            || self.phase != Phase::AwaitingApproval
            || self.changes.is_empty()
            || self.attempts >= self.max_attempts
        {
            return Err("This repair cannot be applied in its current state".into());
        }
        self.attempts += 1;
        self.phase = Phase::Applying;
        Ok(self
            .changes
            .iter()
            .map(|c| (c.path.clone(), c.revision.clone()))
            .collect())
    }
    fn recover(&mut self) {
        if matches!(self.phase, Phase::Applying | Phase::Building) {
            self.phase = Phase::RecoveryRequired;
            self.last_error=Some("Application stopped during an attempt. Inspect source and explicitly roll back; build success is unknown.".into());
        }
    }
}
pub(crate) fn initialize(directory: &Path) -> Result<(), String> {
    let file = directory.join("compiler-repairs.json");
    let mut store = if file.exists() {
        if std::fs::metadata(&file).map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
            return Err("Compiler repair journal exceeds limit; preserve it for recovery.".into());
        }
        let store: Store = serde_json::from_slice(
            &std::fs::read(&file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("Compiler repair journal corrupt; preserve it for recovery: {e}"))?;
        if store.schema != 1 {
            return Err("Unsupported compiler repair schema".into());
        }
        store
    } else {
        Store {
            schema: 1,
            ..Default::default()
        }
    };
    for (key, session) in &mut store.sessions {
        if key != &session.root || !(1..=3).contains(&session.max_attempts) || session.attempts > 3
        {
            return Err(
                "Invalid persisted repair bounds/workspace; preserve journal for recovery.".into(),
            );
        }
        session.recover();
    }
    super::config::write_json_atomic(&file, &store)?;
    FILE.set(file)
        .map_err(|_| "Repair journal already initialized")?;
    STORE
        .set(Mutex::new(store))
        .map_err(|_| "Repair state already initialized".into())
}
fn with<T>(action: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
    #[cfg(test)]
    let store = STORE.get_or_init(|| {
        Mutex::new(Store {
            schema: 1,
            ..Default::default()
        })
    });
    #[cfg(not(test))]
    let store = STORE
        .get()
        .ok_or("Compiler repair recovery is unavailable")?;
    let mut guard = store
        .lock()
        .map_err(|_| "Compiler repair state unavailable")?;
    let mut next = guard.clone();
    let result = action(&mut next)?;
    if serde_json::to_vec(&next).map_err(|e| e.to_string())?.len() > 16 * 1024 * 1024 {
        return Err("Compiler repair history exceeds safe recovery limit".into());
    }
    if let Some(file) = FILE.get() {
        super::config::write_json_atomic(file, &next)?;
    }
    *guard = next;
    Ok(result)
}
fn root(path: &str) -> Result<PathBuf, String> {
    dunce::canonicalize(crate::ai_chat::normalize_project_dir(path)).map_err(|e| e.to_string())
}
fn emit(app: &AppHandle, review: &Review) {
    let _ = app.emit("compiler-repair-changed", review);
}
fn path_key(path: &Path) -> String {
    let text = dunce::simplified(path).to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        text.to_ascii_lowercase()
    } else {
        text
    }
}
fn patch_hash(changes: &[Change]) -> String {
    let files: BTreeMap<_, _> = changes
        .iter()
        .map(|c| (path_key(&c.path), &c.proposed))
        .collect();
    digest(&serde_json::to_vec(&files).unwrap())
}
fn validate(session: &Session, message: &str, files: &[(PathBuf, String)]) -> Result<(), String> {
    if !matches!(
        session.phase,
        Phase::AwaitingProposal | Phase::AwaitingApproval
    ) {
        return Err("Repair is not accepting another patch. Finish verification or roll back the previous attempt first.".into());
    }
    if session.attempts >= session.max_attempts {
        return Err("Compiler repair attempt limit reached".into());
    }
    if session.message.as_deref().is_some_and(|id| id != message) {
        return Err(
            "One repair proposal is already awaiting approval; do not merge another AI turn."
                .into(),
        );
    }
    let mut combined: BTreeMap<String, String> = session
        .changes
        .iter()
        .map(|c| (path_key(&c.path), c.proposed.clone()))
        .collect();
    for (path, content) in files {
        let relative = path
            .strip_prefix(&session.root)
            .map_err(|_| "Repair file escaped project")?;
        if !(relative.starts_with("main") || relative.starts_with("components"))
            || !(matches!(path.extension().and_then(|s| s.to_str()), Some("c" | "h"))
                || path.file_name().is_some_and(|s| s == "CMakeLists.txt"))
        {
            return Err("Compiler repair changes are limited to project main/components C/H and CMakeLists.txt.".into());
        }
        combined.insert(path_key(path), content.clone());
    }
    if session
        .failed
        .contains(&digest(&serde_json::to_vec(&combined).unwrap()))
    {
        return Err("Repeated unsuccessful patch rejected. Propose a different correction.".into());
    }
    Ok(())
}
pub(crate) fn validate_proposal(
    project: &Path,
    message: &str,
    files: &[(PathBuf, String)],
) -> Result<(), String> {
    let root = dunce::canonicalize(project).map_err(|e| e.to_string())?;
    with(|store| {
        if let Some(session) = store.sessions.get(&root).filter(|s| {
            !matches!(
                s.phase,
                Phase::Succeeded | Phase::Cancelled | Phase::Exhausted
            )
        }) {
            validate(session, message, files)
        } else {
            Ok(())
        }
    })
}
pub(crate) fn record_proposal(
    app: &AppHandle,
    project: &Path,
    message: &str,
    changes: &[Change],
) -> Result<(), String> {
    let root = dunce::canonicalize(project).map_err(|e| e.to_string())?;
    let review = with(|store| {
        let Some(session) = store.sessions.get_mut(&root).filter(|s| {
            !matches!(
                s.phase,
                Phase::Succeeded | Phase::Cancelled | Phase::Exhausted
            )
        }) else {
            return Ok(None);
        };
        validate(
            session,
            message,
            &changes
                .iter()
                .map(|c| (c.path.clone(), c.proposed.clone()))
                .collect::<Vec<_>>(),
        )?;
        for change in changes {
            if !session
                .changes
                .iter()
                .any(|c| c.revision == change.revision)
            {
                session.changes.push(change.clone());
            }
        }
        if session.failed.contains(&patch_hash(&session.changes)) {
            return Err("Repeated unsuccessful patch rejected".into());
        }
        session.message = Some(message.into());
        session.phase = Phase::AwaitingApproval;
        Ok(Some(session.review()))
    })?;
    if let Some(review) = review {
        emit(app, &review);
    }
    Ok(())
}
pub(crate) fn require_group_review(path: &Path) -> Result<(), String> {
    with(|store| {
        if store.sessions.values().any(|s| {
            matches!(
                s.phase,
                Phase::AwaitingProposal
                    | Phase::AwaitingApproval
                    | Phase::Applying
                    | Phase::Building
                    | Phase::RecoveryRequired
            ) && path.starts_with(&s.root)
        }) {
            Err("This compiler repair requires separate approval of the whole current attempt in Compiler Repair. Inspect its file diffs there first.".into())
        } else {
            Ok(())
        }
    })
}
pub(crate) fn require_message_review(message: &str) -> Result<(), String> {
    with(|store| {
        if store
            .sessions
            .values()
            .any(|s| s.message.as_deref() == Some(message) && !s.changes.is_empty())
        {
            Err("Use the revision-bound Compiler Repair rollback for this attempt.".into())
        } else {
            Ok(())
        }
    })
}
pub(crate) fn diagnostic_context(project: &Path) -> Result<serde_json::Value, String> {
    let root = dunce::canonicalize(project).map_err(|e| e.to_string())?;
    with(|store| {
        let s=store.sessions.get(&root).ok_or("No compiler repair diagnostic session. The user must start Compiler Repair from the build errors.")?;
        Ok(
            serde_json::json!({"diagnostics":s.diagnostics,"phase":s.phase,"attempts":s.attempts,"max_attempts":s.max_attempts,"instructions":"Propose a different correction using write_file. Every attempt needs a separate user approval in the native Compiler Repair card."}),
        )
    })
}
#[tauri::command]
pub(crate) fn set_compiler_repair_limit(
    app_handle: AppHandle,
    project_dir: String,
    expected_revision: String,
    max_attempts: u8,
) -> Result<Review, String> {
    if !(1..=3).contains(&max_attempts) {
        return Err("Repair maximum must be 1..3 attempts".into());
    }
    let root = root(&project_dir)?;
    let review = with(|store| {
        let session = store.sessions.get_mut(&root).ok_or("No repair session")?;
        session.check(&expected_revision)?;
        if session.attempts != 0
            || !matches!(
                session.phase,
                Phase::AwaitingProposal | Phase::AwaitingApproval
            )
        {
            return Err("Choose the attempt limit before applying the first correction".into());
        }
        session.max_attempts = max_attempts;
        Ok(session.review())
    })?;
    emit(&app_handle, &review);
    Ok(review)
}
#[tauri::command]
pub(crate) fn get_compiler_repair(project_dir: String) -> Result<Option<Review>, String> {
    let root = root(&project_dir)?;
    with(|store| Ok(store.sessions.get(&root).map(Session::review)))
}
#[tauri::command]
pub(crate) fn begin_compiler_repair(
    app_handle: AppHandle,
    project_dir: String,
    output: String,
    max_attempts: u8,
) -> Result<Review, String> {
    if !(1..=3).contains(&max_attempts) {
        return Err("Repair maximum must be 1..3 attempts".into());
    }
    if output.len() > 256 * 1024 {
        return Err("Diagnostic input exceeds limit".into());
    }
    let root = root(&project_dir)?;
    crate::esp_idf::ensure_project_reviewed(&root)?;
    let review = with(|store| {
        if let Some(session) = store.sessions.get(&root) {
            if session.phase == Phase::AwaitingProposal && session.changes.is_empty() {
                return Ok(session.review());
            }
            if !matches!(
                session.phase,
                Phase::Succeeded | Phase::Cancelled | Phase::Exhausted
            ) || !session.changes.is_empty() && session.phase != Phase::Succeeded
            {
                return Err(
                    "A repair session still needs review or rollback before starting another."
                        .into(),
                );
            }
        }
        let id =
            digest(format!("{}:{:?}", root.display(), std::time::SystemTime::now()).as_bytes());
        let session = Session {
            root: root.clone(),
            id,
            phase: Phase::AwaitingProposal,
            max_attempts,
            attempts: 0,
            changes: vec![],
            message: None,
            diagnostics: diagnostics::parse(&output),
            failed: BTreeSet::new(),
            last_error: None,
        };
        let review = session.review();
        store.sessions.insert(root, session);
        Ok(review)
    })?;
    emit(&app_handle, &review);
    Ok(review)
}
#[tauri::command]
pub(crate) async fn apply_compiler_repair(
    app_handle: AppHandle,
    project_dir: String,
    expected_revision: String,
) -> Result<Review, String> {
    let _generation = crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root = root(&project_dir)?;
    let (session, auth) = with(|store| {
        let s = store.sessions.get_mut(&root).ok_or("No repair session")?;
        s.check(&expected_revision)?;
        let auth = s.authorize(&expected_revision)?;
        Ok((s.clone(), auth))
    })?;
    emit(&app_handle, &session.review());
    let applied = change_journal::with(|journal| journal.apply(&auth));
    if let Err(error) = applied {
        return finish(&app_handle, &root, Phase::RecoveryRequired, Some(error));
    }
    for change in &session.changes {
        let _ = app_handle.emit(
            "file-modified",
            serde_json::json!({"path":change.path}).to_string(),
        );
        let _ = app_handle.emit(
            "ai-diff-pending",
            serde_json::json!({"fullPath":change.path}).to_string(),
        );
    }
    finish(&app_handle, &root, Phase::Building, None)?;
    let result = crate::esp_idf::run_idf_command(
        app_handle.clone(),
        "build".into(),
        vec![],
        Some(root.to_string_lossy().into_owned()),
    )
    .await;
    match result {
        Ok(_) => finish(&app_handle, &root, Phase::Succeeded, None),
        Err(error) => {
            let phase = if error.contains("cancelled") {
                Phase::Cancelled
            } else {
                Phase::Failed
            };
            with(|store| {
                let s = store.sessions.get_mut(&root).unwrap();
                s.failed.insert(patch_hash(&s.changes));
                s.diagnostics = diagnostics::parse(&error);
                Ok(())
            })?;
            finish(&app_handle, &root, phase, Some(error))
        }
    }
}
fn finish(
    app: &AppHandle,
    root: &Path,
    phase: Phase,
    error: Option<String>,
) -> Result<Review, String> {
    let review = with(|store| {
        let s = store.sessions.get_mut(root).ok_or("No repair session")?;
        s.phase = phase;
        s.last_error = error.map(|s| s.chars().take(12000).collect());
        Ok(s.review())
    })?;
    emit(app, &review);
    Ok(review)
}
#[tauri::command]
pub(crate) fn rollback_compiler_repair(
    app_handle: AppHandle,
    project_dir: String,
    expected_revision: String,
) -> Result<Review, String> {
    let _generation = crate::ai_chat::acquire_ai_generation_guard(&app_handle)?;
    let root = root(&project_dir)?;
    let session = with(|store| {
        let s = store.sessions.get(&root).ok_or("No repair session")?;
        s.check(&expected_revision)?;
        if matches!(s.phase, Phase::Applying | Phase::Building) {
            return Err("Cancel and wait for SDK termination before rollback".into());
        }
        Ok(s.clone())
    })?;
    if let Some(message) = session.message {
        let paths = change_journal::with(|j| j.undo_message(&message))?;
        for path in paths {
            let _ = app_handle.emit(
                "file-modified",
                serde_json::json!({"path":path}).to_string(),
            );
            let _ = app_handle.emit(
                "ai-diff-pending",
                serde_json::json!({"fullPath":path}).to_string(),
            );
        }
    }
    let review = with(|store| {
        let s = store.sessions.get_mut(&root).unwrap();
        s.changes.clear();
        s.message = None;
        s.last_error = None;
        s.phase = if s.attempts >= s.max_attempts {
            Phase::Exhausted
        } else {
            Phase::AwaitingProposal
        };
        Ok(s.review())
    })?;
    emit(&app_handle, &review);
    Ok(review)
}
#[tauri::command]
pub(crate) fn cancel_compiler_repair(
    app_handle: AppHandle,
    project_dir: String,
    expected_revision: String,
) -> Result<Review, String> {
    let root = root(&project_dir)?;
    let session = with(|store| {
        let s = store.sessions.get(&root).ok_or("No repair session")?;
        s.check(&expected_revision)?;
        Ok(s.clone())
    })?;
    if session.phase == Phase::Building {
        crate::sdk_process::cancel_sdk_command(project_dir)?;
        return Ok(session.review());
    }
    if session.phase == Phase::Applying {
        return Err("File transaction is committing; wait for it before cancellation".into());
    }
    if session.phase == Phase::AwaitingApproval {
        rollback_compiler_repair(app_handle.clone(), project_dir, expected_revision)?;
    }
    finish(
        &app_handle,
        &root,
        Phase::Cancelled,
        Some(
            "Repair cancelled. Applied changes, if any, retain their Undo recovery records.".into(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session {
            root: PathBuf::from("/project"),
            id: "test".into(),
            phase: Phase::AwaitingProposal,
            max_attempts: 3,
            attempts: 0,
            changes: vec![],
            message: None,
            diagnostics: vec![],
            failed: BTreeSet::new(),
            last_error: None,
        }
    }
    #[test]
    fn authorization_is_bound_to_the_entire_state_and_attempt_count() {
        let mut s = session();
        let r = s.review();
        assert!(s.check(&r.revision).is_ok());
        s.attempts = 1;
        assert!(s.check(&r.revision).is_err());
        s.attempts = 3;
        assert!(validate(&s, "m", &[]).is_err());
    }
    #[test]
    fn non_source_changes_are_rejected() {
        let s = session();
        let path = s.root.join("main/main.c");
        assert!(validate(&s, "m", &[(path, "better".into())]).is_ok());
        assert!(validate(&s, "m", &[(s.root.join("sdkconfig"), "bad".into())]).is_err());
    }
    #[test]
    fn failed_build_cannot_accept_a_new_patch_before_rollback() {
        let mut s = session();
        for phase in [
            Phase::Failed,
            Phase::Building,
            Phase::RecoveryRequired,
            Phase::Exhausted,
        ] {
            s.phase = phase;
            assert!(validate(&s, "m", &[]).is_err());
        }
    }
    #[test]
    fn persisted_attempts_and_failure_fingerprints_survive_serialization() {
        let mut s = session();
        s.attempts = 2;
        s.failed.insert("failed-digest".into());
        let reopened: Session = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(reopened.attempts, 2);
        assert!(reopened.failed.contains("failed-digest"));
    }
    #[test]
    fn real_file_attempts_require_new_authorization_and_stop_after_three() {
        let base = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(base.path()).unwrap();
        std::fs::create_dir(root.join("main")).unwrap();
        let path = root.join("main/main.c");
        std::fs::write(&path, "original").unwrap();
        let unrelated = root.join("main/unrelated.h");
        std::fs::write(&unrelated, "untouched").unwrap();
        let mut journal = change_journal::Journal::open(root.join("review.json")).unwrap();
        let mut s = session();
        s.root = root.clone();
        for attempt in 0..3 {
            let message = format!("attempt-{attempt}");
            let proposed = format!("correction-{attempt}");
            validate(&s, &message, &[(path.clone(), proposed.clone())]).unwrap();
            s.changes = journal
                .propose_batch(&root, &message, &[(path.clone(), proposed.clone())])
                .unwrap();
            s.message = Some(message.clone());
            s.phase = Phase::AwaitingApproval;
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
            assert!(s.authorize("old-revision").is_err());
            let revision = s.review().revision;
            let auth = s.authorize(&revision).unwrap();
            assert!(s.authorize(&revision).is_err());
            journal.apply(&auth).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), proposed);
            s.failed.insert(patch_hash(&s.changes));
            s.phase = Phase::Failed;
            journal.undo_message(&message).unwrap();
            s.changes.clear();
            s.message = None;
            s.phase = Phase::AwaitingProposal;
            // Reopen both persisted representations before retry checks. This
            // simulates store restart, not real power-loss or an OS reboot.
            s = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
            journal = change_journal::Journal::open(root.join("review.json")).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
            assert_eq!(std::fs::read_to_string(&unrelated).unwrap(), "untouched");
            assert!(validate(&s, "retry", &[(path.clone(), proposed)]).is_err());
        }
        assert_eq!(s.attempts, 3);
        assert!(validate(&s, "fourth", &[(path, "new correction".into())]).is_err());
    }
    #[test]
    fn restart_never_infers_build_success_and_retains_rollback_evidence() {
        let mut s = session();
        s.phase = Phase::Building;
        s.attempts = 1;
        let mut reopened: Session =
            serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        reopened.recover();
        assert_eq!(reopened.phase, Phase::RecoveryRequired);
        assert_eq!(reopened.attempts, 1);
        assert!(reopened.last_error.unwrap().contains("unknown"));
    }

    #[test]
    fn knowing_current_revision_does_not_authorize_other_repair_phases() {
        for phase in [Phase::AwaitingProposal,Phase::Applying,Phase::Building,Phase::Failed,Phase::Succeeded,Phase::Cancelled,Phase::RecoveryRequired,Phase::Exhausted] {
            let mut s=session();s.phase=phase;
            let revision=s.review().revision;
            assert!(s.authorize(&revision).is_err());
            assert_eq!(s.attempts,0);
        }
    }
}
