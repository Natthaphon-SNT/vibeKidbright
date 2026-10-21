//! Bounded subprocess trees and per-workspace exclusion. No arbitrary PID killing.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ExecutionConfig {
    pub timeout_seconds: u64,
}
impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 1800,
        }
    }
}
impl ExecutionConfig {
    fn validate(&self) -> Result<(), String> {
        if !(1..=7200).contains(&self.timeout_seconds) {
            Err("SDK timeout must be 1..7200 seconds.".into())
        } else {
            Ok(())
        }
    }
}
#[tauri::command]
pub(crate) fn get_sdk_execution_config() -> Result<ExecutionConfig, String> {
    let value = crate::ai::config::read_config()
        .get("sdk_execution")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let config: ExecutionConfig = serde_json::from_value(value).map_err(|e| e.to_string())?;
    config.validate()?;
    Ok(config)
}
#[tauri::command]
pub(crate) fn set_sdk_execution_config(config: ExecutionConfig) -> Result<(), String> {
    config.validate()?;
    crate::ai::config::update_config(|c| c["sdk_execution"] = serde_json::json!(config))
}
struct Active {
    cancel: Arc<AtomicBool>,
    process: bool,
}
static ACTIVE: OnceLock<Mutex<BTreeMap<PathBuf, Active>>> = OnceLock::new();
fn active() -> &'static Mutex<BTreeMap<PathBuf, Active>> {
    ACTIVE.get_or_init(|| Mutex::new(BTreeMap::new()))
}
pub(crate) struct WorkspaceLease {
    root: PathBuf,
    pub cancel: Arc<AtomicBool>,
}
pub(crate) fn lease(root: &Path, process: bool) -> Result<WorkspaceLease, String> {
    let root = dunce::canonicalize(root).map_err(|e| e.to_string())?;
    let mut running = active()
        .lock()
        .map_err(|_| "Workspace operation state unavailable.")?;
    if running
        .keys()
        .any(|other| other.starts_with(&root) || root.starts_with(other))
    {
        return Err("Another SDK or file operation is active in this workspace. Wait for cancellation/completion.".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    running.insert(
        root.clone(),
        Active {
            cancel: cancel.clone(),
            process,
        },
    );
    Ok(WorkspaceLease { root, cancel })
}
impl Drop for WorkspaceLease {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Ok(mut map) = active().lock() {
            if map
                .get(&self.root)
                .is_some_and(|state| Arc::ptr_eq(&state.cancel, &self.cancel))
            {
                map.remove(&self.root);
            }
        }
    }
}
#[tauri::command]
pub(crate) fn cancel_sdk_command(project_dir: String) -> Result<(), String> {
    let root = dunce::canonicalize(crate::ai_chat::normalize_project_dir(&project_dir))
        .map_err(|e| e.to_string())?;
    let map = active()
        .lock()
        .map_err(|_| "Workspace operation state unavailable.")?;
    let state = map
        .get(&root)
        .filter(|state| state.process)
        .ok_or("No cancellable SDK command for this workspace.")?;
    state.cancel.store(true, Ordering::SeqCst);
    Ok(())
}
pub(crate) struct CancelOnDrop(pub Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
/// Drop only this command's future when the owning AI request is cancelled.
/// Its CancelOnDrop signals the owned workspace lease; unrelated user jobs stay
/// active, and write exclusion remains until the blocking worker has terminated.
pub(crate) async fn until_cancelled<T>(
    future: impl std::future::Future<Output = Result<T, String>>,
    cancel: Arc<AtomicBool>,
) -> Result<T, String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("AI SDK request cancelled before start".into());
    }
    tokio::pin!(future);
    loop {
        tokio::select! {
            result=&mut future=>return result,
            _=tokio::time::sleep(Duration::from_millis(25))=>{
                if cancel.load(Ordering::SeqCst){return Err("AI SDK cancellation requested; owned process tree is stopping, workspace writes remain locked until cleanup completes".into());}
            }
        }
    }
}
pub(crate) type Emit = Arc<dyn Fn(String, bool) + Send + Sync>;
pub(crate) struct Captured {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(windows)]
struct Tree {
    _job: std::os::windows::io::OwnedHandle,
}
#[cfg(windows)]
impl Tree {
    fn spawn(command: &mut Command) -> Result<(Child, Self), String> {
        use std::os::windows::{
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
            process::CommandExt,
        };
        use windows_sys::Win32::{
            Foundation::INVALID_HANDLE_VALUE,
            System::{Diagnostics::ToolHelp::*, JobObjects::*, Threading::*},
        };
        // Primary thread stays suspended until assigned to a kill-on-close job,
        // so it cannot spawn an uncontained compiler before job assignment.
        unsafe {
            let raw = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if raw.is_null() {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let tree = Self {
                _job: OwnedHandle::from_raw_handle(raw),
            };
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                raw,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            command.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
            let mut child = command.spawn().map_err(|e| e.to_string())?;
            let setup = (|| -> Result<(), String> {
                if AssignProcessToJobObject(raw, child.as_raw_handle()) == 0 {
                    return Err(format!(
                        "Cannot contain SDK process: {}",
                        std::io::Error::last_os_error()
                    ));
                }
                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
                if snapshot == INVALID_HANDLE_VALUE {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                let snapshot = OwnedHandle::from_raw_handle(snapshot);
                let mut entry: THREADENTRY32 = std::mem::zeroed();
                entry.dwSize = std::mem::size_of_val(&entry) as u32;
                let mut found = false;
                let mut ok = Thread32First(snapshot.as_raw_handle(), &mut entry);
                while ok != 0 {
                    if entry.th32OwnerProcessID == child.id() {
                        let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                        if thread.is_null() {
                            return Err(std::io::Error::last_os_error().to_string());
                        }
                        let thread = OwnedHandle::from_raw_handle(thread);
                        if ResumeThread(thread.as_raw_handle()) == u32::MAX {
                            return Err(std::io::Error::last_os_error().to_string());
                        }
                        found = true;
                    }
                    ok = Thread32Next(snapshot.as_raw_handle(), &mut entry);
                }
                if !found {
                    return Err("Cannot resume the contained SDK primary thread.".into());
                }
                Ok(())
            })();
            if let Err(error) = setup {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            Ok((child, tree))
        }
    }
}
#[cfg(unix)]
struct Tree(i32);
#[cfg(unix)]
impl Tree {
    fn spawn(command: &mut Command) -> Result<(Child, Self), String> {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        let child = command.spawn().map_err(|e| e.to_string())?;
        let tree = Self(child.id() as i32);
        Ok((child, tree))
    }
}
#[cfg(unix)]
impl Drop for Tree {
    fn drop(&mut self) {
        unsafe {
            libc::kill(-self.0, libc::SIGKILL);
        }
    }
}
// OwnedHandle Drop closes the Windows job and terminates all contained children.

/// Interactive local protocol clients share SDK process-tree containment.
/// Dropping this owner closes the tree before waiting, including inherited pipes.
pub(crate) struct InteractiveChild {
    pub child: Child,
    tree: Option<Tree>,
}
impl InteractiveChild {
    pub fn spawn(mut command: Command) -> Result<Self,String> {
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        let (child,tree)=Tree::spawn(&mut command)?;
        Ok(Self{child,tree:Some(tree)})
    }
}
impl Drop for InteractiveChild {
    fn drop(&mut self) {
        self.tree.take();
        let _=self.child.kill();
        let _=self.child.wait();
    }
}

pub(crate) fn execute(
    mut command: Command,
    emit: Emit,
    cancel: Arc<AtomicBool>,
    timeout: Duration,
) -> Result<Captured, String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("SDK command cancelled before spawn.".into());
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let (mut child, tree) = Tree::spawn(&mut command)?;
    fn drain<R: std::io::Read>(pipe: R, error: bool, emit: Emit) -> Result<String, String> {
        use std::io::{BufRead, Read};
        let mut reader = std::io::BufReader::new(pipe);
        let mut captured = Vec::new();
        let mut incomplete = Vec::new();
        loop {
            let mut bytes = Vec::new();
            let count = reader
                .by_ref()
                .take(8192)
                .read_until(b'\n', &mut bytes)
                .map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            captured.extend_from_slice(&bytes);
            if captured.len() > 65536 {
                captured.drain(..captured.len() - 65536);
            }
            incomplete.extend_from_slice(&bytes);
            let boundary = match std::str::from_utf8(&incomplete) {
                Err(e) if e.error_len().is_none() => e.valid_up_to(),
                _ => incomplete.len(),
            };
            if boundary > 0 {
                emit(
                    String::from_utf8_lossy(&incomplete[..boundary])
                        .trim_end_matches(['\r', '\n'])
                        .into(),
                    error,
                );
                incomplete.drain(..boundary);
            }
        }
        if !incomplete.is_empty() {
            emit(String::from_utf8_lossy(&incomplete).into_owned(), error);
        }
        // The bounded tail may start in a UTF-8 continuation byte. Discard only
        // that incomplete leading character, not the remainder of the message.
        let leading = captured.iter().take_while(|&&b| b & 0xC0 == 0x80).count();
        Ok(String::from_utf8_lossy(&captured[leading..]).into_owned())
    }
    let stdout = child.stdout.take().ok_or("SDK stdout unavailable.")?;
    let stderr = child.stderr.take().ok_or("SDK stderr unavailable.")?;
    let out_emit = emit.clone();
    let out = std::thread::spawn(move || drain(stdout, false, out_emit));
    let err = std::thread::spawn(move || drain(stderr, true, emit));
    let started = Instant::now();
    let mut reason = None;
    let status = loop {
        if cancel.load(Ordering::SeqCst) {
            reason = Some("SDK command cancelled");
            break None;
        }
        if started.elapsed() >= timeout {
            reason = Some("SDK command timed out");
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                reason = Some("SDK process wait failed");
                break None;
            }
        }
    };
    drop(tree); // also stops grandchildren retaining pipe handles after parent exit
    let _ = child.kill();
    let _ = child.wait();
    let stdout = out.join().map_err(|_| "SDK stdout reader failed.")??;
    let stderr = err.join().map_err(|_| "SDK stderr reader failed.")??;
    if let Some(reason) = reason {
        return Err(format!(
            "{reason}; process tree terminated.\n{stdout}{stderr}"
        ));
    }
    let status = status.ok_or("SDK status unavailable.")?;
    Ok(Captured {
        success: status.success(),
        code: status.code(),
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn python(script: &str) -> Command {
        let mut cmd = Command::new(if cfg!(windows) { "python" } else { "python3" });
        cmd.args(["-u", "-c", script])
            .env("PYTHONIOENCODING", "utf-8");
        cmd
    }
    #[test]
    fn captures_both_pipes_and_actual_exit_status() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        let out=execute(python("import sys;print('ไทย stdout');print('warning stderr',file=sys.stderr);sys.exit(7)"),Arc::new(move|s,e|sink.lock().unwrap().push((s,e))),Arc::new(AtomicBool::new(false)),Duration::from_secs(10)).unwrap();
        assert!(!out.success);
        assert_eq!(out.code, Some(7));
        assert!(out.stdout.contains("ไทย"));
        assert!(out.stderr.contains("warning"));
        assert!(events.lock().unwrap().iter().any(|(_, error)| *error));
    }
    #[test]
    fn timeout_and_pre_spawn_cancellation_are_bounded() {
        let flag = Arc::new(AtomicBool::new(true));
        assert!(execute(
            python("raise RuntimeError('must not start')"),
            Arc::new(|_, _| {}),
            flag,
            Duration::from_secs(1)
        )
        .err()
        .unwrap()
        .contains("before spawn"));
        let start = Instant::now();
        let error = execute(
            python("import time; time.sleep(60)"),
            Arc::new(|_, _| {}),
            Arc::new(AtomicBool::new(false)),
            Duration::from_millis(200),
        )
        .err()
        .unwrap();
        assert!(error.contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(5));
    }
    #[test]
    fn workspace_leases_exclude_nested_builds_and_writes_and_release_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("main")).unwrap();
        let job = lease(dir.path(), true).unwrap();
        assert!(lease(&dir.path().join("main"), false).is_err());
        cancel_sdk_command(dir.path().to_string_lossy().into_owned()).unwrap();
        assert!(job.cancel.load(Ordering::SeqCst));
        drop(job);
        let _write = lease(dir.path(), false).unwrap();
        assert!(cancel_sdk_command(dir.path().to_string_lossy().into_owned()).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn cancellation_terminates_a_real_grandchild_and_closes_inherited_pipes() {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
        };
        let flag = Arc::new(AtomicBool::new(false));
        let cancel = flag.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let task = std::thread::spawn(move || {
            execute(python("import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print(p.pid,flush=True); time.sleep(60)"),Arc::new(move|line,error|{if !error{let _=send.send(line);}}),flag,Duration::from_secs(15))
        });
        let pid: u32 = receive
            .recv_timeout(Duration::from_secs(10))
            .expect("test grandchild PID")
            .trim()
            .parse()
            .unwrap();
        let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        assert!(!process.is_null());
        let process = unsafe { OwnedHandle::from_raw_handle(process) };
        cancel.store(true, Ordering::SeqCst);
        let error = task.join().unwrap().err().unwrap();
        assert!(error.contains("cancelled"));
        assert_eq!(
            unsafe { WaitForSingleObject(process.as_raw_handle(), 5000) },
            0,
            "grandchild must exit, not remain an orphan"
        );
    }
    #[test]
    fn output_without_newlines_does_not_keep_unbounded_buffers() {
        let out = execute(
            python("print('x'*200000,end='')"),
            Arc::new(|_, _| {}),
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(10),
        )
        .unwrap();
        assert!(out.stdout.len() <= 65536);
    }
    #[cfg(windows)]
    #[test]
    fn dropping_interactive_protocol_owner_terminates_its_grandchild() {
        use std::io::BufRead;
        use std::os::windows::io::{AsRawHandle,FromRawHandle,OwnedHandle};
        use windows_sys::Win32::System::Threading::{OpenProcess,WaitForSingleObject,PROCESS_SYNCHRONIZE};
        let mut child=InteractiveChild::spawn(python("import subprocess,sys; p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); print(p.pid,flush=True); sys.stdin.read()" )).unwrap();
        let stdout=child.child.stdout.take().unwrap();let (tx,rx)=std::sync::mpsc::channel();
        let reader=std::thread::spawn(move||{let mut line=String::new();std::io::BufReader::new(stdout).read_line(&mut line).unwrap();let _=tx.send(line);});
        let pid:u32=rx.recv_timeout(Duration::from_secs(10)).unwrap().trim().parse().unwrap();
        let process=unsafe{OpenProcess(PROCESS_SYNCHRONIZE,0,pid)};assert!(!process.is_null());let process=unsafe{OwnedHandle::from_raw_handle(process)};
        drop(child);reader.join().unwrap();
        assert_eq!(unsafe{WaitForSingleObject(process.as_raw_handle(),5000)},0);
    }
    #[test]
    fn utf8_characters_split_by_the_output_bound_are_reassembled() {
        let events = Arc::new(Mutex::new(String::new()));
        let sink = events.clone();
        let out = execute(
            python(r"import sys;sys.stdout.write('\u0e44\u0e17\u0e22'*30000)"),
            Arc::new(move |text, _| sink.lock().unwrap().push_str(&text)),
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(10),
        )
        .unwrap();
        assert_eq!(*events.lock().unwrap(), "ไทย".repeat(30000));
        assert!(!out.stdout.contains('\u{fffd}'));
        assert!(out.stdout.len() <= 65536);
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ai_stop_drops_only_its_command_and_terminates_the_native_worker() {
        let ai = Arc::new(AtomicBool::new(false));
        let native = Arc::new(AtomicBool::new(false));
        let worker_cancel = native.clone();
        let (ready, started) = std::sync::mpsc::channel();
        let (ended, finished) = std::sync::mpsc::channel();
        let future = async move {
            let _cancel_on_drop = CancelOnDrop(worker_cancel.clone());
            tokio::task::spawn_blocking(move || {
                let result = execute(
                    python("import time;print('ready',flush=True);time.sleep(60)"),
                    Arc::new(move |line, _| {
                        let _ = ready.send(line);
                    }),
                    worker_cancel,
                    Duration::from_secs(15),
                );
                let stopped = result.err().is_some_and(|e| e.contains("cancelled"));
                let _ = ended.send(stopped);
                Ok::<(), String>(())
            })
            .await
            .map_err(|e| e.to_string())?
        };
        let relay = tokio::spawn(until_cancelled(future, ai.clone()));
        tokio::task::spawn_blocking(move || started.recv_timeout(Duration::from_secs(10)).unwrap())
            .await
            .unwrap();
        ai.store(true, Ordering::SeqCst);
        assert!(relay
            .await
            .unwrap()
            .unwrap_err()
            .contains("cancellation requested"));
        assert!(native.load(Ordering::SeqCst));
        assert!(tokio::task::spawn_blocking(move || finished
            .recv_timeout(Duration::from_secs(5))
            .unwrap())
        .await
        .unwrap());
    }
}
