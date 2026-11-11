//! User-operated Plus USB/RNDIS WebSocket shell. No AI tool exposes execution.
//! Wire contract: KidBrightAI/kidbright_MAI_ide 7b549397, websocket-shell.js.
use base64::{engine::general_purpose::STANDARD, Engine};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

const HOST: &str = "10.155.55.1";
const URL: &str = "wss://10.155.55.1:5050";
const RUNNER: &str = include_str!("uai_plus_runner.py");
const RUNTIME: &str = include_str!("uai_plus_runtime.py");
const PROBE: &str = include_str!("uai_plus_probe.py");
const RUNTIME_PREFIX: &str = "__VIBE_PLUS_RUNTIME__:";
const STOP_FILENAME: &str = ".vibekidbright-stop";
type Reply = oneshot::Sender<Result<Value, String>>;
enum Request {
    System(Value, String, Option<String>, Reply),
    Input(String, Reply),
    Disconnect(Reply),
}
#[derive(Clone)]
struct ActiveRun {
    marker: String,
    destination: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct RuntimeInfo {
    schema: u8,
    python: Option<String>,
    maixpy: Option<String>,
    runtime: Option<String>,
    api_checked: bool,
    missing_apis: Vec<String>,
}

fn parse_runtime_info(text: &str) -> Option<RuntimeInfo> {
    text.lines().rev().find_map(|line| {
        let raw = line.trim_end_matches('\r').strip_prefix(RUNTIME_PREFIX)?;
        if raw.len() > 8192 {
            return None;
        }
        let info: RuntimeInfo = serde_json::from_str(raw).ok()?;
        let version = |value: &Option<String>| {
            value.as_ref().is_none_or(|s| {
                s.len() <= 64
                    && s.bytes().next().is_some_and(|b| b.is_ascii_digit())
                    && s.contains('.')
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".+_-".contains(&b))
            })
        };
        (info.schema == 1
            && version(&info.python)
            && version(&info.maixpy)
            && version(&info.runtime)
            && info.missing_apis.len() <= 32
            && info.missing_apis.iter().all(|name| {
                !name.is_empty()
                    && name.len() <= 100
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._".contains(&b))
            }))
        .then_some(info)
    })
}

async fn prepare_connection<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
) -> Result<(), String> {
    // Connect must also work while the interactive shell is starting, has a
    // short input-line limit, or cannot execute Python. The server's separate
    // JSON channel does not depend on the PTY. Detect versions from the uploaded
    // probe file only when the user starts a run; never paste it into the shell.
    let stat = system(
        socket,
        json!({"cmd":"stat","path":"/tmp"}),
        "stat",
        Some("/tmp"),
        |_| {},
    )
    .await?;
    if stat["exists"] != true {
        return Err("Plus runtime did not confirm its /tmp directory. No code or system scripts were changed.".into());
    }
    Ok(())
}
#[derive(Default)]
pub(crate) struct PlusDeviceState {
    connection: tokio::sync::Mutex<Option<mpsc::Sender<Request>>>,
    operation: tokio::sync::Mutex<()>,
    running: Arc<AtomicBool>,
    cancelled: AtomicBool,
    session: Arc<Mutex<Option<ActiveRun>>>,
    runtime_info: Arc<Mutex<Option<RuntimeInfo>>>,
}

async fn tls() -> Result<(tokio_native_tls::TlsStream<tokio::net::TcpStream>, String), String> {
    #[cfg(debug_assertions)]
    if crate::ai::config::evaluation_isolated() {
        return Err("Device access is disabled in isolated evaluation.".into());
    }
    // This dedicated connector is restricted to the board's fixed USB address.
    // Self-signed TLS is accepted only to obtain its certificate; all operational
    // connections require an exact SHA-256 fingerprint approved in the device UI.
    let connector = native_tls::TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|e| e.to_string())?;
    let tcp = tokio::net::TcpStream::connect((HOST, 5050)).await.map_err(|e| format!("Plus USB/RNDIS is unavailable: {e}. Check USB-OTG, power switch and RNDIS adapter."))?;
    let stream = tokio_native_tls::TlsConnector::from(connector)
        .connect(HOST, tcp)
        .await
        .map_err(|e| e.to_string())?;
    let certificate = stream
        .get_ref()
        .peer_certificate()
        .map_err(|e| e.to_string())?
        .ok_or("The board did not provide a TLS certificate.")?;
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(certificate.to_der().map_err(|e| e.to_string())?)
    );
    Ok((stream, fingerprint))
}

#[tauri::command]
pub async fn probe_uai_plus() -> Result<String, String> {
    // Read-only TLS handshake; never sends shell input, files or code.
    let (_, fingerprint) = tokio::time::timeout(Duration::from_secs(10), tls())
        .await
        .map_err(|_| "Plus USB connection timed out.")??;
    Ok(fingerprint)
}

#[tauri::command]
pub async fn get_uai_plus_status(app: tauri::AppHandle) -> Value {
    let state = app.state::<PlusDeviceState>();
    let connected = state
        .connection
        .lock()
        .await
        .as_ref()
        .is_some_and(|sender| !sender.is_closed());
    let runtime_info = state.runtime_info.lock().ok().and_then(|info| info.clone());
    json!({"connected":connected,"running":connected && state.running.load(Ordering::SeqCst),"runtimeInfo":runtime_info})
}

fn check_fingerprint(expected: &str, actual: &str) -> Result<(), String> {
    if expected.len() != 64
        || !expected.bytes().all(|b| b.is_ascii_hexdigit())
        || !expected.eq_ignore_ascii_case(actual)
    {
        return Err("Plus certificate differs from the trusted USB device. Disconnect and verify the device certificate again; no code was sent.".into());
    }
    Ok(())
}

fn system_frame(message: &Message) -> Option<Value> {
    let Message::Text(text) = message else {
        return None;
    };
    serde_json::from_str(
        text.strip_prefix("\r\n>>> System: ")?
            .strip_suffix("\r\n")?,
    )
    .ok()
}

fn output(
    app: &tauri::AppHandle,
    message: &Message,
    running: &AtomicBool,
    session: &Mutex<Option<ActiveRun>>,
    runtime_info: &Mutex<Option<RuntimeInfo>>,
    tail: &mut String,
    utf8: &mut Vec<u8>,
) {
    let text = match message {
        Message::Binary(bytes) => decode_output(bytes, utf8),
        Message::Text(text) if system_frame(message).is_none() => text.to_string(),
        _ => return,
    };
    // Bound logs and preserve Unicode boundaries. The full program output is
    // never accumulated on the host; only a small suffix is used for exit codes.
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + 8192).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let _ = app.emit("terminal-output", text[start..end].to_string());
        start = end;
    }
    tail.push_str(&text);
    if tail.len() > 32768 {
        let mut start = tail.len() - 16384;
        while !tail.is_char_boundary(start) {
            start += 1;
        }
        tail.drain(..start);
    }
    if let Some(end) = tail.rfind('\n') {
        if let Some(info) = parse_runtime_info(&tail[..=end]) {
            if let Ok(mut current) = runtime_info.lock() {
                if current.as_ref() != Some(&info) {
                    *current = Some(info.clone());
                    let _ = app.emit("uai-plus-state", json!({"connected":true,"running":running.load(Ordering::SeqCst),"runtimeInfo":info}));
                }
            }
        }
    }
    let expected = session
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.marker.clone()));
    if let Some(expected) = expected {
        if let Some(code) = exit_code(tail, &expected) {
            running.store(false, Ordering::SeqCst);
            if let Ok(mut session) = session.lock() {
                *session = None;
            }
            let _ = app.emit(
                "uai-plus-state",
                json!({"connected":true,"running":false,"exitCode":code,"runtimeInfo":runtime_info.lock().ok().and_then(|info| info.clone())}),
            );
        }
    }
}

fn decode_output(bytes: &[u8], pending: &mut Vec<u8>) -> String {
    pending.extend_from_slice(bytes);
    let mut text = String::new();
    loop {
        match std::str::from_utf8(pending) {
            Ok(value) => {
                text.push_str(value);
                pending.clear();
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                text.push_str(std::str::from_utf8(&pending[..valid]).unwrap());
                if let Some(invalid) = error.error_len() {
                    text.push(char::REPLACEMENT_CHARACTER);
                    pending.drain(..valid + invalid);
                } else {
                    pending.drain(..valid);
                    break;
                }
            }
        }
    }
    text
}

fn exit_code(text: &str, marker: &str) -> Option<i32> {
    text.split_inclusive('\n').find_map(|line| {
        line.strip_suffix('\n')?
            .trim_end_matches('\r')
            .strip_prefix(marker)?
            .parse()
            .ok()
    })
}

async fn next_message<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
) -> Result<Message, String> {
    loop {
        let msg = socket
            .next()
            .await
            .ok_or("Plus disconnected.")?
            .map_err(|e| e.to_string())?;
        match msg {
            Message::Close(_) => return Err("Plus disconnected.".into()),
            Message::Ping(bytes) => socket
                .send(Message::Pong(bytes))
                .await
                .map_err(|e| e.to_string())?,
            Message::Pong(_) => (),
            _ => return Ok(msg),
        }
    }
}

async fn system<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    socket: &mut WebSocketStream<S>,
    value: Value,
    kind: &str,
    path: Option<&str>,
    mut log: impl FnMut(&Message),
) -> Result<Value, String> {
    socket
        .send(Message::Text(format!("__SYSTEM__:{value}").into()))
        .await
        .map_err(|e| e.to_string())?;
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let msg = next_message(socket).await?;
            if let Some(value) = system_frame(&msg) {
                if value["type"] == "error" {
                    return Err(format!(
                        "Plus command failed: {}",
                        value
                            .get("message")
                            .or_else(|| value.get("error"))
                            .unwrap_or(&Value::Null)
                    ));
                }
                if value["type"] == kind && path.is_none_or(|path| value["path"] == path) {
                    return Ok(value);
                }
            } else {
                log(&msg);
            }
        }
    })
    .await
    .map_err(|_| {
        "Plus command timed out; operation result is unknown. Reconnect before retrying."
            .to_string()
    })?
}

#[tauri::command]
pub async fn connect_uai_plus(
    app: tauri::AppHandle,
    fingerprint: String,
) -> Result<String, String> {
    let state = app.state::<PlusDeviceState>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another Plus operation is in progress.")?;
    state.cancelled.store(false, Ordering::SeqCst);
    let mut connection = state.connection.lock().await;
    if connection
        .as_ref()
        .is_some_and(|sender| !sender.is_closed())
    {
        return Ok("Plus is already connected.".into());
    }
    let (stream, actual) = tokio::time::timeout(Duration::from_secs(10), tls())
        .await
        .map_err(|_| "Plus connection timed out.")??;
    check_fingerprint(&fingerprint, &actual)?;
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(8),
        tokio_tungstenite::client_async(URL, stream),
    )
    .await
    .map_err(|_| "Plus WebSocket handshake timed out.")?
    .map_err(|e| e.to_string())?;
    if let Err(error) = prepare_connection(&mut socket).await {
        let _ = tokio::time::timeout(Duration::from_secs(1), socket.close(None)).await;
        return Err(error);
    }
    *state
        .runtime_info
        .lock()
        .map_err(|_| "Plus runtime state lock failed.")? = None;
    let (sender, mut receiver) = mpsc::channel::<Request>(8);
    *connection = Some(sender);
    state.running.store(false, Ordering::SeqCst);
    *state
        .session
        .lock()
        .map_err(|_| "Plus state lock failed.")? = None;
    let running = state.running.clone();
    let session = state.session.clone();
    let runtime_info = state.runtime_info.clone();
    let actor_app = app.clone();
    tokio::spawn(async move {
        let mut tail = String::new();
        let mut utf8 = Vec::new();
        loop {
            tokio::select! {
                request = receiver.recv() => match request {
                    Some(Request::System(value,kind,path,reply)) => {
                        let result = system(&mut socket, value, &kind, path.as_deref(), |msg| output(&actor_app,msg,&running,&session,&runtime_info,&mut tail,&mut utf8)).await;
                        let failed = result.is_err(); let _ = reply.send(result);
                        if failed { break; }
                    }
                    Some(Request::Input(text,reply)) => {
                        let result = socket.send(Message::Text(text.into())).await.map(|_| json!({"sent":true})).map_err(|e| e.to_string());
                        let failed = result.is_err(); let _ = reply.send(result);
                        if failed { break; }
                    }
                    Some(Request::Disconnect(reply)) => {
                        let active = session.lock().ok().and_then(|s| s.clone());
                        if running.load(Ordering::SeqCst) {
                            if let Some(active) = active {
                                let path = format!("{}/{}", active.destination, STOP_FILENAME);
                                let _ = system(&mut socket, stop_request(&path), "uploaded", Some(&path), |msg| output(&actor_app,msg,&running,&session,&runtime_info,&mut tail,&mut utf8)).await;
                            }
                        }
                        let _ = socket.close(None).await; let _ = reply.send(Ok(json!(true))); break;
                    }
                    None => break,
                },
                msg = next_message(&mut socket) => match msg {
                    Ok(msg) => output(&actor_app,&msg,&running,&session,&runtime_info,&mut tail,&mut utf8),
                    Err(error) => { let _ = actor_app.emit("terminal-output", format!("Plus disconnected: {error}")); break; }
                }
            }
        }
        let unknown = running.swap(false, Ordering::SeqCst);
        let _ = actor_app.emit(
            "uai-plus-state",
            json!({"connected":false,"running":false,"executionUnknown":unknown}),
        );
    });
    Ok("Plus connected over USB RNDIS / WebSocket. Runtime versions and APIs will be checked during Run Python.".into())
}

async fn sender(app: &tauri::AppHandle) -> Result<mpsc::Sender<Request>, String> {
    app.state::<PlusDeviceState>()
        .connection
        .lock()
        .await
        .as_ref()
        .filter(|s| !s.is_closed())
        .cloned()
        .ok_or("Connect the KidBright uAI Plus USB device first.".into())
}
async fn ask(
    sender: &mpsc::Sender<Request>,
    make: impl FnOnce(Reply) -> Request,
) -> Result<Value, String> {
    let (tx, rx) = oneshot::channel();
    sender
        .send(make(tx))
        .await
        .map_err(|_| "Plus connection closed.")?;
    tokio::time::timeout(Duration::from_secs(12), rx)
        .await
        .map_err(|_| "Plus request timed out; result unknown.")?
        .map_err(|_| "Plus connection closed.")?
}

fn program_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    if !crate::ai::uai_plus::is_project(root) {
        return Err(
            "Select a KidBright uAI Plus Python project; no ESP-IDF files were uploaded.".into(),
        );
    }
    fn walk(
        root: &Path,
        dir: &Path,
        files: &mut Vec<(String, Vec<u8>)>,
        total: &mut usize,
    ) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(dir).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("Plus upload cannot follow links or junctions.".into());
            }
        }
        if metadata.file_type().is_symlink() {
            return Err("Plus upload cannot follow links.".into());
        }
        if metadata.is_dir() {
            for item in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                let item = item.map_err(|e| e.to_string())?;
                let name = item.file_name();
                let name = name.to_string_lossy();
                if name.starts_with('.') || name == "__pycache__" {
                    continue;
                }
                if !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                {
                    return Err("Plus upload filenames must use ASCII letters, numbers, dot, underscore or hyphen.".into());
                }
                walk(root, &item.path(), files, total)?;
            }
        } else if metadata.is_file() {
            if files.len() >= 64 || metadata.len() > 4 * 1024 * 1024 {
                return Err("Plus prototype supports at most 64 files, 4 MiB per file.".into());
            }
            let data = std::fs::read(dir).map_err(|e| e.to_string())?;
            *total += data.len();
            if *total > 8 * 1024 * 1024 {
                return Err("Plus prototype upload exceeds 8 MiB.".into());
            }
            files.push((
                dir.strip_prefix(root)
                    .map_err(|_| "Upload escaped main.")?
                    .to_string_lossy()
                    .replace('\\', "/"),
                data,
            ));
        } else {
            return Err("Plus upload requires regular files.".into());
        }
        Ok(())
    }
    let mut files = Vec::new();
    let mut total = 0;
    walk(
        &root.join("main"),
        &root.join("main"),
        &mut files,
        &mut total,
    )?;
    if !files.iter().any(|(p, _)| p == "main.py") {
        return Err("The Python entrypoint main/main.py is missing.".into());
    }
    Ok(files)
}

#[tauri::command]
pub async fn run_uai_plus_project(
    app: tauri::AppHandle,
    project_dir: String,
) -> Result<String, String> {
    let _generation = crate::ai_chat::acquire_ai_generation_guard(&app)?;
    let state = app.state::<PlusDeviceState>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| "Another Plus operation is in progress.")?;
    if state.running.load(Ordering::SeqCst) {
        return Err("Stop the running Plus program before uploading another.".into());
    }
    state.cancelled.store(false, Ordering::SeqCst);
    let root = dunce::canonicalize(&project_dir).map_err(|e| e.to_string())?;
    crate::esp_idf::ensure_project_reviewed(&root)?;
    let mut files = program_files(&root)?;
    // Private supervisor stays in this temporary run only; it is never copied
    // into the user's project, board services or boot configuration.
    files.push((
        ".vibekidbright-runner.py".into(),
        RUNNER.as_bytes().to_vec(),
    ));
    files.push((
        ".vibekidbright-runtime.py".into(),
        RUNTIME.as_bytes().to_vec(),
    ));
    files.push((".vibekidbright-probe.py".into(), PROBE.as_bytes().to_vec()));
    let sender = sender(&app).await?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let destination = format!("/tmp/vibekidbright-{nonce}");
    for (relative, bytes) in files {
        let path = format!("{destination}/{relative}");
        let chunks: Vec<_> = if bytes.is_empty() {
            vec![&[][..]]
        } else {
            bytes.chunks(128 * 1024).collect()
        };
        for (i, chunk) in chunks.iter().enumerate() {
            if state.cancelled.load(Ordering::SeqCst) {
                return Err("Plus upload cancelled; no program was started.".into());
            }
            let value = json!({"cmd":"upload","path":path,"mode":if i==0 {"wb"} else {"ab"},"data":STANDARD.encode(chunk)});
            ask(&sender, |reply| {
                Request::System(value, "uploaded".into(), Some(path.clone()), reply)
            })
            .await?;
        }
        let stat = ask(&sender, |reply| {
            Request::System(
                json!({"cmd":"stat","path":path}),
                "stat".into(),
                Some(path.clone()),
                reply,
            )
        })
        .await?;
        if stat["exists"] != true || stat["size"].as_u64() != Some(bytes.len() as u64) {
            return Err("Plus upload size verification failed. No program was started.".into());
        }
    }
    let marker = format!("__VIBE_PLUS_EXIT_{nonce}:");
    if state.cancelled.load(Ordering::SeqCst) {
        return Err("Plus upload cancelled; no program was started.".into());
    }
    *state
        .session
        .lock()
        .map_err(|_| "Plus state lock failed.")? = Some(ActiveRun {
        marker: marker.clone(),
        destination: destination.clone(),
    });
    state.running.store(true, Ordering::SeqCst);
    // Destination/marker are generated numeric strings, not user filenames or
    // shell interpolation. No persistent app, startup or OS files are modified.
    let command = format!("cd '{destination}' && python3 -u .vibekidbright-runner.py '{marker}'; _vibe_exit=$?; printf '\\n{marker}%s\\n' \"$_vibe_exit\"\r");
    if let Err(error) = ask(&sender, |reply| Request::Input(command, reply)).await {
        // The command may have reached the board before its reply was lost.
        // Keep tracking this run until exit or the actor's unknown-state event.
        return Err(format!(
            "{error} Python launch status is unknown; no exit was confirmed."
        ));
    }
    Ok("Python uploaded; execution requested. Watch terminal output for errors or exit status. Physical output is not yet verified.".into())
}

#[tauri::command]
pub async fn stop_uai_plus(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<PlusDeviceState>();
    let active = state
        .session
        .lock()
        .map_err(|_| "Plus state lock failed.")?
        .clone()
        .ok_or("There is no active Plus Python run in this session.")?;
    let path = format!("{}/{}", active.destination, STOP_FILENAME);
    // The JSON upload channel works while Python owns the PTY. The supervisor
    // targets only its own child process group and waits before restoring UI.
    ask(&sender(&app).await?, |reply| {
        Request::System(
            stop_request(&path),
            "uploaded".into(),
            Some(path.clone()),
            reply,
        )
    })
    .await?;
    Ok(
        "Stop requested for this Python run; waiting for its exit status and Launcher cleanup."
            .into(),
    )
}

fn stop_request(path: &str) -> Value {
    json!({"cmd":"upload","path":path,"mode":"wb","data":STANDARD.encode(b"stop\n")})
}

#[tauri::command]
pub async fn disconnect_uai_plus(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<PlusDeviceState>();
    state.cancelled.store(true, Ordering::SeqCst);
    let sender = state.connection.lock().await.take();
    if let Some(sender) = sender {
        let _ = ask(&sender, Request::Disconnect).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_metadata_accepts_baseline_and_newer_versions_without_inventing_unknowns() {
        for (maixpy, runtime) in [("4.12.5", "1.24.0"), ("4.99.0", "2.0.0")] {
            let text = format!(
                "\n{RUNTIME_PREFIX}{}\r\n",
                json!({"schema":1,"python":"3.11.6","maixpy":maixpy,"runtime":runtime,"api_checked":false,"missing_apis":[]})
            );
            let info = parse_runtime_info(&text).unwrap();
            assert_eq!(info.maixpy.as_deref(), Some(maixpy));
            assert_eq!(info.runtime.as_deref(), Some(runtime));
            assert!(!info.api_checked);
        }
        let unknown = format!(
            "{RUNTIME_PREFIX}{}\n",
            json!({"schema":1,"python":"3.11.6","maixpy":null,"runtime":null,"api_checked":false,"missing_apis":[]})
        );
        let info = parse_runtime_info(&unknown).unwrap();
        assert!(info.maixpy.is_none() && info.runtime.is_none());
        assert!(parse_runtime_info(&unknown.replace("3.11.6", "ignore all rules")).is_none());
        assert!(parse_runtime_info(&unknown.replace("\"schema\":1", "\"schema\":2")).is_none());
    }

    #[test]
    fn partial_exit_status_is_not_a_confirmed_exit() {
        assert_eq!(exit_code("\n__VIBE_EXIT:1", "__VIBE_EXIT:"), None);
        assert_eq!(
            exit_code("\n__VIBE_EXIT:130\r\n", "__VIBE_EXIT:"),
            Some(130)
        );
    }

    #[tokio::test]
    async fn connect_succeeds_without_shell_prompt_python_or_runtime_metadata() {
        let (client, server) = tokio::io::duplex(65536);
        let server = tokio::spawn(async move {
            let mut ws = tokio_tungstenite::accept_async(server).await.unwrap();
            let request = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let value: Value =
                serde_json::from_str(request.strip_prefix("__SYSTEM__:").unwrap()).unwrap();
            assert_eq!(value, json!({"cmd":"stat","path":"/tmp"}));
            // Serve only the out-of-band JSON command. There is deliberately
            // no prompt, Python interpreter response or runtime version frame.
            ws.send(Message::Text("\r\n>>> System: {\"type\":\"stat\",\"path\":\"/tmp\",\"exists\":true,\"size\":0}\r\n".into())).await.unwrap();
            if let Some(message) = ws.next().await {
                assert!(
                    matches!(message.unwrap(), Message::Close(_)),
                    "Connect must not send PTY input or upload files"
                );
            }
        });
        let (mut ws, _) = tokio_tungstenite::client_async("ws://local-fixture", client)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), prepare_connection(&mut ws))
            .await
            .expect("Connect must not wait for Python metadata")
            .unwrap();
        ws.close(None).await.unwrap();
        server.await.unwrap();
    }
    #[test]
    fn stop_uses_the_control_channel_and_a_private_per_run_file() {
        let path = "/tmp/vibekidbright-123/.vibekidbright-stop";
        let request = stop_request(path);
        assert_eq!(request["cmd"], "upload");
        assert_eq!(request["path"], path);
        assert_eq!(request["mode"], "wb");
        assert_eq!(
            STANDARD.decode(request["data"].as_str().unwrap()).unwrap(),
            b"stop\n"
        );
    }
    #[test]
    fn certificate_change_or_missing_trust_cannot_connect() {
        assert!(check_fingerprint(&"a".repeat(64), &"a".repeat(64)).is_ok());
        assert!(check_fingerprint("", &"a".repeat(64)).is_err());
        assert!(check_fingerprint(&"b".repeat(64), &"a".repeat(64)).is_err());
    }
    #[test]
    fn system_json_is_not_confused_with_program_output() {
        assert_eq!(
            system_frame(&Message::Text(
                "\r\n>>> System: {\"type\":\"uploaded\",\"path\":\"/tmp/a\"}\r\n".into()
            ))
            .unwrap()["type"],
            "uploaded"
        );
        assert!(
            system_frame(&Message::Binary(b"\r\n>>> System: {}\r\n".to_vec().into())).is_none()
        );
        assert!(system_frame(&Message::Text("user output".into())).is_none());
        assert_eq!(
            exit_code("echo __VIBE_EXIT:%s\n__VIBE_EXIT:2\r\n", "__VIBE_EXIT:"),
            Some(2)
        );
        assert_eq!(exit_code("__VIBE_EXIT:%s\n", "__VIBE_EXIT:"), None);
        let mut pending = Vec::new();
        let bytes = "ภาษาไทย".as_bytes();
        let mut text = String::new();
        for chunk in bytes.chunks(2) {
            text.push_str(&decode_output(chunk, &mut pending));
        }
        assert_eq!(text, "ภาษาไทย");
        assert!(pending.is_empty());
    }
    #[test]
    fn uploads_only_main_files_and_rejects_classic_projects() {
        let base = tempfile::tempdir().unwrap();
        assert!(program_files(base.path()).is_err());
        let root = crate::ai::uai_plus::initialize(
            base.path(),
            "plus",
            &json!({"project_type":"kidbright_uai_plus"}),
        )
        .unwrap();
        std::fs::write(root.join("private.txt"), "do not upload").unwrap();
        let files = program_files(&root).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "main.py");
        assert!(!String::from_utf8_lossy(&files[0].1).contains("private.txt"));
    }

    #[tokio::test]
    async fn wire_protocol_waits_for_matching_ack_and_keeps_binary_logs_separate() {
        let (client, server) = tokio::io::duplex(1024 * 1024);
        let server = tokio::spawn(async move {
            let mut ws = tokio_tungstenite::accept_async(server).await.unwrap();
            let request = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let value: Value =
                serde_json::from_str(request.strip_prefix("__SYSTEM__:").unwrap()).unwrap();
            assert_eq!(value["cmd"], "upload");
            assert_eq!(
                STANDARD.decode(value["data"].as_str().unwrap()).unwrap(),
                b"print('hello')\n"
            );
            ws.send(Message::Binary(b"upload log\n".to_vec().into()))
                .await
                .unwrap();
            // Wrong path is not an acknowledgement for this upload.
            ws.send(Message::Text(
                "\r\n>>> System: {\"type\":\"uploaded\",\"path\":\"/tmp/other\"}\r\n".into(),
            ))
            .await
            .unwrap();
            ws.send(Message::Text(
                "\r\n>>> System: {\"type\":\"uploaded\",\"path\":\"/tmp/main.py\"}\r\n".into(),
            ))
            .await
            .unwrap();
            let request = ws.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(request.contains("\"cmd\":\"stat\""));
            ws.send(Message::Text("\r\n>>> System: {\"type\":\"stat\",\"path\":\"/tmp/main.py\",\"exists\":true,\"size\":15}\r\n".into())).await.unwrap();
        });
        let (mut ws, _) = tokio_tungstenite::client_async("ws://local-fixture", client)
            .await
            .unwrap();
        let mut logs = Vec::new();
        let uploaded = system(&mut ws, json!({"cmd":"upload","path":"/tmp/main.py","data":STANDARD.encode(b"print('hello')\n")}), "uploaded", Some("/tmp/main.py"), |msg| logs.push(msg.clone())).await.unwrap();
        assert_eq!(uploaded["path"], "/tmp/main.py");
        assert_eq!(logs.len(), 1);
        let stat = system(
            &mut ws,
            json!({"cmd":"stat","path":"/tmp/main.py"}),
            "stat",
            Some("/tmp/main.py"),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(stat["size"], 15);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn board_error_never_becomes_upload_success() {
        let (client, server) = tokio::io::duplex(8192);
        let server = tokio::spawn(async move {
            let mut ws = tokio_tungstenite::accept_async(server).await.unwrap();
            ws.next().await.unwrap().unwrap();
            ws.send(Message::Text(
                "\r\n>>> System: {\"type\":\"error\",\"message\":\"disk full\"}\r\n".into(),
            ))
            .await
            .unwrap();
        });
        let (mut ws, _) = tokio_tungstenite::client_async("ws://local-fixture", client)
            .await
            .unwrap();
        assert!(
            system(&mut ws, json!({"cmd":"upload"}), "uploaded", None, |_| {})
                .await
                .unwrap_err()
                .contains("disk full")
        );
        server.await.unwrap();
    }
}
