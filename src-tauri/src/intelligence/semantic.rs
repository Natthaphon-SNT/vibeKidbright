//! Optional, request-scoped local clangd. No service, shared project state,
//! automatic download, arbitrary query-driver glob or remote context export.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Buffer {
    pub path: String,
    pub text: String,
}
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Definition,
    References,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub file_path: String,
    pub buffers: Vec<Buffer>,
    pub line: u32,
    pub character: u32,
    pub operation: Operation,
    pub clangd_path: String,
}
#[derive(Clone, Serialize)]
pub struct Location {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
}
#[derive(Serialize)]
pub struct Answer {
    pub engine: String,
    pub locations: Vec<Location>,
    pub limitation: String,
    pub buffer_sha256: String,
    pub compile_commands_sha256: Option<String>,
}

fn scoped(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let path = dunce::simplified(path);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let path = crate::ai::change_journal::checked_path(root, &path)?;
    let relative = path.strip_prefix(root).map_err(|_| "Outside workspace")?;
    if !matches!(
        relative
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str()),
        Some("main" | "components")
    ) || !matches!(path.extension().and_then(|x| x.to_str()), Some("c" | "h"))
        || !path.is_file()
    {
        return Err("C intelligence only reads existing main/components C/H files".into());
    }
    dunce::canonicalize(path).map_err(|e| e.to_string())
}
fn position(text: &str, line: u32, character: u32) -> Result<usize, String> {
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n')
            .nth((line - 1) as usize)
            .map(|(i, _)| i + 1)
            .ok_or("Invalid source line")?
    };
    let row = text[start..].split(['\r', '\n']).next().unwrap_or("");
    let mut units = 0;
    for (offset, c) in row.char_indices() {
        if units == character {
            return Ok(start + offset);
        }
        units += c.len_utf16() as u32;
        if units > character {
            return Err("Position splits a UTF-16 surrogate pair".into());
        }
    }
    if units == character {
        Ok(start + row.len())
    } else {
        Err("Invalid UTF-16 column".into())
    }
}
fn source_buffers(
    root: &Path,
    query: &Query,
) -> Result<(PathBuf, BTreeMap<PathBuf, String>), String> {
    if query.buffers.len() > 16 {
        return Err("At most 16 open C buffers are supported".into());
    }
    let selected = scoped(root, Path::new(&query.file_path))?;
    let mut buffers = BTreeMap::new();
    for buffer in &query.buffers {
        if buffer.text.len() > 512 * 1024 {
            return Err("Open source buffer exceeds 512 KiB".into());
        }
        let path = scoped(root, Path::new(&buffer.path))?;
        if buffers.insert(path, buffer.text.clone()).is_some() {
            return Err("Duplicate source buffer".into());
        }
    }
    if !buffers.contains_key(&selected) {
        if std::fs::metadata(&selected)
            .map_err(|e| e.to_string())?
            .len()
            > 512 * 1024
        {
            return Err("Source exceeds 512 KiB".into());
        }
        buffers.insert(
            selected.clone(),
            std::fs::read_to_string(&selected).map_err(|e| e.to_string())?,
        );
    }
    position(&buffers[&selected], query.line, query.character)?;
    Ok((selected, buffers))
}
fn syntax(
    root: &Path,
    query: &Query,
    selected: &Path,
    buffers: &BTreeMap<PathBuf, String>,
    reason: String,
) -> Answer {
    let source = &buffers[selected];
    let offset = position(source, query.line, query.character).unwrap_or(0);
    let identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let start = source[..offset]
        .char_indices()
        .rev()
        .find(|(_, c)| !identifier(*c))
        .map_or(0, |(i, c)| i + c.len_utf8());
    let end = source[offset..]
        .char_indices()
        .find(|(_, c)| !identifier(*c))
        .map_or(source.len(), |(i, _)| offset + i);
    let symbol = &source[start..end];
    let mut locations = Vec::new();
    if !symbol.is_empty() {
        for (path, text) in buffers {
            for chunk in super::chunk_source("source.c", text, 8000) {
                if matches!(query.operation, Operation::Definition)
                    && chunk.kind != "function_definition"
                {
                    continue;
                }
                for (relative, _) in chunk.content.match_indices(symbol) {
                    let byte = chunk.start_byte + relative;
                    if text[..byte].chars().next_back().is_some_and(identifier)
                        || text[byte + symbol.len()..]
                            .chars()
                            .next()
                            .is_some_and(identifier)
                    {
                        continue;
                    }
                    if matches!(query.operation, Operation::Definition)
                        && chunk.signature.as_ref().is_none_or(|s| !s.contains(symbol))
                    {
                        continue;
                    }
                    let prefix = &text[..byte];
                    let line = prefix.bytes().filter(|b| *b == b'\n').count() as u32;
                    let character = prefix
                        .rsplit('\n')
                        .next()
                        .unwrap_or("")
                        .encode_utf16()
                        .count() as u32;
                    locations.push(Location {
                        path: path.to_string_lossy().into(),
                        line,
                        character,
                        end_line: line,
                        end_character: character + symbol.len() as u32,
                    });
                    if locations.len() >= 128 {
                        break;
                    }
                }
                if locations.len() >= 128 {
                    break;
                }
            }
            if locations.len() >= 128 {
                break;
            }
        }
    }
    let _ = root;
    Answer{engine:"tree_sitter_syntax".into(),locations,limitation:format!("{reason}. Syntax candidates in open buffers only; comments/macros/uses are not semantic references."),buffer_sha256:super::digest(source.as_bytes()),compile_commands_sha256:None}
}
fn first_argument(command: &str) -> Result<&str, String> {
    let command = command.trim_start();
    if let Some(s) = command.strip_prefix('"') {
        return s
            .split_once('"')
            .map(|(s, _)| s)
            .ok_or("Unclosed compiler path".into());
    }
    command
        .split_whitespace()
        .next()
        .ok_or("Empty compiler command".into())
}
fn compile_context(
    root: &Path,
    selected: &Path,
    buffers: &BTreeMap<PathBuf, String>,
    tools: &Path,
) -> Result<(PathBuf, PathBuf, String, BTreeMap<PathBuf, String>), String> {
    let database =
        crate::ai::change_journal::checked_path(root, &root.join("build/compile_commands.json"))?;
    if std::fs::metadata(&database)
        .map_err(|_| "Build the project with the configured ESP-IDF first")?
        .len()
        > 32 * 1024 * 1024
    {
        return Err("Compilation database exceeds limit".into());
    }
    let bytes = std::fs::read(&database).map_err(|e| e.to_string())?;
    let entries: Vec<Value> =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid compilation database")?;
    if entries.is_empty() || entries.len() > 20000 {
        return Err("Invalid compilation database entry count".into());
    }
    if tools.as_os_str().is_empty() {
        return Err("Configure the installed ESP-IDF tools before enabling clangd".into());
    }
    let tools =
        dunce::canonicalize(tools).map_err(|_| "Configured ESP-IDF tools are unavailable")?;
    let mut selected_found = false;
    let mut driver = None;
    let mut responses = BTreeMap::new();
    for entry in entries {
        let directory = PathBuf::from(
            entry["directory"]
                .as_str()
                .ok_or("Missing compiler working directory")?,
        );
        let file = PathBuf::from(entry["file"].as_str().ok_or("Missing compiler source")?);
        let file = if file.is_absolute() {
            file
        } else {
            directory.join(file)
        };
        let Ok(file) = dunce::canonicalize(file) else {
            continue;
        };
        if !buffers.contains_key(&file) {
            continue;
        }
        // Do not accept project-supplied config/response/plugin loading flags.
        let command = entry["command"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| {
                entry["arguments"].as_array().map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
            })
            .ok_or("Missing compiler invocation")?;
        let lower = command.to_ascii_lowercase();
        let dangerous = |text: &str| {
            [
                "-fplugin", "-load", "-plugin", "--config", "-specs", "-wrapper",
            ]
            .iter()
            .any(|s| text.to_ascii_lowercase().contains(s))
        };
        if dangerous(&lower) {
            return Err("Compiler plugin/config flags require separate review".into());
        }
        // ESP-IDF's generated GCC toolchain cflags response file is part of the
        // actual compilation context. Permit only that bounded, checked file.
        let pattern = regex::Regex::new(r#"@(?:"([^"]+)"|(\S+))"#).unwrap();
        for item in pattern.captures_iter(&command) {
            let response = PathBuf::from(item.get(1).or_else(|| item.get(2)).unwrap().as_str());
            let response = if response.is_absolute() {
                response
            } else {
                directory.join(response)
            };
            let response =
                crate::ai::change_journal::checked_path(root, dunce::simplified(&response))?;
            if response != root.join("build/toolchain/cflags")
                || std::fs::metadata(&response)
                    .map_err(|e| e.to_string())?
                    .len()
                    > 65536
            {
                return Err("Unreviewed compiler response file".into());
            }
            let content = std::fs::read_to_string(&response).map_err(|e| e.to_string())?;
            if content.contains('@') || dangerous(&content) {
                return Err("Nested/unsafe compiler response file".into());
            }
            responses.insert(response, super::digest(content.as_bytes()));
        }
        let compiler = entry["arguments"]
            .get(0)
            .and_then(Value::as_str)
            .map(Ok)
            .unwrap_or_else(|| first_argument(&command))?;
        let compiler = Path::new(compiler);
        if !compiler.is_absolute() {
            return Err("Compiler path must be absolute from the configured ESP-IDF".into());
        }
        let compiler =
            dunce::canonicalize(compiler).map_err(|_| "Configured compiler unavailable")?;
        let name = compiler.file_name().unwrap().to_string_lossy();
        if !compiler.starts_with(&tools)
            || !(name.ends_with("-gcc.exe")
                || name.ends_with("-gcc")
                || name.ends_with("-g++.exe")
                || name.ends_with("-g++"))
            || compiler
                .to_string_lossy()
                .contains([',', '*', '?', '[', ']'])
        {
            return Err(
                "Compiler is outside the configured SDK tools or is not an explicit GCC driver"
                    .into(),
            );
        }
        if driver.as_ref().is_some_and(|d| d != &compiler) {
            return Err("Open buffers use different drivers; narrow the request".into());
        }
        driver = Some(compiler);
        if file == selected {
            selected_found = true;
        }
    }
    if !selected_found {
        return Err(
            "No exact compile command for this source; header inference is not enabled".into(),
        );
    }
    Ok((
        database,
        driver.ok_or("No verified compiler")?,
        super::digest(&bytes),
        responses,
    ))
}
fn frame(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut length = None;
    let mut total = 0;
    loop {
        let mut line = Vec::new();
        let count = (&mut *reader)
            .take(1025)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            return Ok(None);
        }
        total += count;
        if count > 1024 || total > 8192 {
            return Err("LSP header exceeds limit".into());
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
        let line = std::str::from_utf8(&line).map_err(|_| "Invalid LSP header")?;
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("Content-Length") {
                if length.is_some() {
                    return Err("Duplicate LSP content length".into());
                }
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "Invalid LSP length")?,
                );
            }
        }
    }
    let length = length
        .filter(|n| *n > 0 && *n <= 4 * 1024 * 1024)
        .ok_or("Missing/oversized LSP content length")?;
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}
fn uri(path: &Path) -> Result<String, String> {
    reqwest::Url::from_file_path(path)
        .map(String::from)
        .map_err(|_| "Invalid local file URI".into())
}
fn locations(
    root: &Path,
    buffers: &BTreeMap<PathBuf, String>,
    value: Value,
) -> Result<Vec<Location>, String> {
    let items = match value {
        Value::Array(a) => a,
        Value::Null => vec![],
        v => vec![v],
    };
    let mut found = Vec::new();
    for item in items.into_iter().take(256) {
        let Some(url) = reqwest::Url::parse(
            item["uri"]
                .as_str()
                .or_else(|| item["targetUri"].as_str())
                .unwrap_or(""),
        )
        .ok() else {
            continue;
        };
        let Ok(local) = url.to_file_path() else {
            continue;
        };
        let Ok(path) = scoped(root, &local) else {
            continue;
        };
        let range = item
            .get("range")
            .or_else(|| item.get("targetSelectionRange"))
            .ok_or("clangd location has no range")?;
        let number = |key: &str, end: &str| {
            range[key][end]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or("Invalid clangd position")
        };
        let location = Location {
            path: path.to_string_lossy().into(),
            line: number("start", "line")?,
            character: number("start", "character")?,
            end_line: number("end", "line")?,
            end_character: number("end", "character")?,
        };
        let disk;
        let source = if let Some(text) = buffers.get(&path) {
            text
        } else {
            if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > 512 * 1024 {
                return Err("clangd location source exceeds 512 KiB".into());
            }
            disk = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            &disk
        };
        let start = position(source, location.line, location.character)?;
        let end = position(source, location.end_line, location.end_character)?;
        if end < start {
            return Err("Reversed clangd location range".into());
        }
        found.push(location);
    }
    Ok(found)
}
fn lsp(
    root: &Path,
    selected: &Path,
    buffers: &BTreeMap<PathBuf, String>,
    query: &Query,
    binary: &Path,
    database: &Path,
    driver: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<Location>, String> {
    let mut command = Command::new(binary);
    command
        .current_dir(root)
        .args([
            "--enable-config=false",
            "--background-index=false",
            "--pch-storage=memory",
            "--log=error",
        ])
        .arg(format!(
            "--compile-commands-dir={}",
            database.parent().unwrap().display()
        ))
        .arg(format!("--query-driver={}", driver.display()));
    let mut server = crate::sdk_process::InteractiveChild::spawn(command)?;
    let output = server.child.stdout.take().ok_or("LSP stdout unavailable")?;
    let mut input = server.child.stdin.take().ok_or("LSP stdin unavailable")?;
    let (tx, rx) = mpsc::sync_channel(32);
    let reader = std::thread::spawn(move || {
        let mut output = std::io::BufReader::new(output);
        loop {
            match frame(&mut output) {
                Ok(Some(v)) => {
                    if tx.send(Ok(v)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });
    let (writer_tx, writer_rx) = mpsc::sync_channel::<Value>(32);
    let writer = std::thread::spawn(move || {
        for value in writer_rx {
            let bytes = value.to_string().into_bytes();
            if write!(input, "Content-Length: {}\r\n\r\n", bytes.len())
                .and_then(|_| input.write_all(&bytes))
                .and_then(|_| input.flush())
                .is_err()
            {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let send = |method: &str, params: Value, id: Option<u32>| -> Result<(), String> {
        let mut msg = json!({"jsonrpc":"2.0","method":method,"params":params});
        if let Some(id) = id {
            msg["id"] = json!(id);
        }
        writer_tx
            .try_send(msg)
            .map_err(|_| "LSP input queue unavailable".into())
    };
    let response = |id: u32| -> Result<Value, String> {
        loop {
            if cancel.load(Ordering::SeqCst) {
                return Err("C intelligence cancelled".into());
            }
            if Instant::now() > deadline {
                return Err("C intelligence timed out".into());
            }
            match rx.recv_timeout(Duration::from_millis(25)) {
                Ok(Ok(v)) => {
                    if v["id"] == id {
                        if let Some(e) = v.get("error") {
                            return Err(format!("clangd rejected request: {e}"));
                        }
                        return Ok(v["result"].clone());
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err("clangd closed the protocol".into()),
            }
        }
    };
    let result = (|| {
        send(
            "initialize",
            json!({"processId":null,"rootUri":uri(root)?,"capabilities":{"general":{"positionEncodings":["utf-16"]}}}),
            Some(1),
        )?;
        let initialized = response(1)?;
        if initialized["capabilities"]["positionEncoding"]
            .as_str()
            .is_some_and(|s| s != "utf-16")
        {
            return Err("Unsupported LSP position encoding".into());
        }
        send("initialized", json!({}), None)?;
        for (path, text) in buffers {
            send(
                "textDocument/didOpen",
                json!({"textDocument":{"uri":uri(path)?,"languageId":"c","version":1,"text":text}}),
                None,
            )?;
        }
        let expected: std::collections::BTreeSet<_> =
            buffers.keys().map(|p| uri(p)).collect::<Result<_, _>>()?;
        let mut ready = std::collections::BTreeSet::new();
        while ready.len() < expected.len() {
            if cancel.load(Ordering::SeqCst) {
                return Err("C intelligence cancelled".into());
            }
            if Instant::now() > deadline {
                return Err("clangd document preparation timed out".into());
            }
            match rx.recv_timeout(Duration::from_millis(25)) {
                Ok(Ok(v)) => {
                    if v["method"] == "textDocument/publishDiagnostics" {
                        if let Some(uri) = v["params"]["uri"].as_str() {
                            if expected.contains(uri) {
                                if v["params"]["diagnostics"]
                                    .as_array()
                                    .is_some_and(|a| a.iter().any(|d| d["severity"] == 1))
                                {
                                    return Err("clangd reports source/configuration errors; use syntax candidates until the source builds".into());
                                }
                                ready.insert(uri.to_owned());
                            }
                        }
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err("clangd closed during document preparation".into()),
            }
        }
        // Exact request-scoped snapshots include unsaved text; no disk writes.
        let mut params = json!({"textDocument":{"uri":uri(selected)?},"position":{"line":query.line,"character":query.character}});
        let method = match query.operation {
            Operation::Definition => "textDocument/definition",
            Operation::References => {
                params["context"] = json!({"includeDeclaration":true});
                "textDocument/references"
            }
        };
        send(method, params, Some(2))?;
        let result = response(2)?;
        locations(root, buffers, result)
    })();
    drop(writer_tx);
    drop(server);
    drop(rx);
    let _ = writer.join();
    let _ = reader.join();
    result
}

/// Public for the development probe; the UI obtains tools_root from the existing SDK resolver.
pub fn query(
    root: &Path,
    tools_root: &Path,
    query: &Query,
    cancel: &AtomicBool,
) -> Result<Answer, String> {
    let root = dunce::canonicalize(root).map_err(|e| e.to_string())?;
    let (selected, buffers) = source_buffers(&root, query)?;
    if cancel.load(Ordering::SeqCst) {
        return Err("C intelligence cancelled".into());
    }
    let fallback = |reason| syntax(&root, query, &selected, &buffers, reason);
    if query.clangd_path.is_empty() {
        return Ok(fallback("clangd is not enabled".into()));
    }
    let _workspace_lease = match crate::sdk_process::lease(&root, false) {
        Ok(lease) => lease,
        Err(error) => return Ok(fallback(error)),
    };
    let binary = match dunce::canonicalize(&query.clangd_path) {
        Ok(p) => p,
        Err(_) => return Ok(fallback("Selected clangd is unavailable".into())),
    };
    if !matches!(
        binary.file_name().and_then(|s| s.to_str()),
        Some("clangd" | "clangd.exe")
    ) {
        return Err("Select an explicit clangd executable".into());
    }
    let (database, driver, hash, responses) =
        match compile_context(&root, &selected, &buffers, tools_root) {
            Ok(v) => v,
            Err(e) => return Ok(fallback(e)),
        };
    let result = lsp(
        &root, &selected, &buffers, query, &binary, &database, &driver, cancel,
    );
    if cancel.load(Ordering::SeqCst) {
        return Err("C intelligence cancelled".into());
    }
    match result {
        Ok(locations) => {
            if super::digest(&std::fs::read(database).map_err(|e| e.to_string())?) != hash {
                return Ok(fallback(
                    "Compile commands changed during request; retry".into(),
                ));
            }
            for (path, expected) in responses {
                if super::digest(&std::fs::read(path).map_err(|e| e.to_string())?) != expected {
                    return Ok(fallback(
                        "Compiler response file changed during request; retry".into(),
                    ));
                }
            }
            Ok(Answer{engine:"clangd".into(),locations,limitation:"Local request-scoped semantic results using exact SDK compile command and open unsaved buffers. References cover opened translation units; background indexing and header command inference are disabled. No physical correctness claim.".into(),buffer_sha256:super::digest(buffers[&selected].as_bytes()),compile_commands_sha256:Some(hash)})
        }
        Err(e) => Ok(fallback(e)),
    }
}
static ACTIVE: OnceLock<Mutex<BTreeMap<String, Arc<AtomicBool>>>> = OnceLock::new();
static CANCELLED_BEFORE_START: OnceLock<Mutex<BTreeMap<String, Instant>>> = OnceLock::new();
fn register_request(id: &str, cancel: Arc<AtomicBool>) -> Result<(), String> {
    let mut map = ACTIVE
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "C intelligence state unavailable")?;
    if map.len() >= 4 || map.contains_key(id) {
        return Err("Too many active C intelligence requests".into());
    }
    let mut early = CANCELLED_BEFORE_START
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "C intelligence cancellation state unavailable")?;
    early.retain(|_, time| time.elapsed() < Duration::from_secs(30));
    if early.remove(id).is_some() {
        cancel.store(true, Ordering::SeqCst);
    }
    map.insert(id.to_owned(), cancel);
    Ok(())
}
#[tauri::command]
pub(crate) async fn query_c_intelligence(
    app_handle: tauri::AppHandle,
    project_dir: String,
    request_id: String,
    request: Query,
) -> Result<Answer, String> {
    if request_id.len() > 128 || request_id.is_empty() {
        return Err("Invalid C intelligence request ID".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    register_request(&request_id, cancel.clone())?;
    let _cancel_on_drop = crate::sdk_process::CancelOnDrop(cancel.clone());
    let root = PathBuf::from(crate::ai_chat::normalize_project_dir(&project_dir));
    let tools = crate::esp_idf::resolve_idf_paths(&app_handle)
        .map(|(_, tools)| tools)
        .unwrap_or_default();
    let result = tauri::async_runtime::spawn_blocking(move || {
        struct Remove(String);
        impl Drop for Remove {
            fn drop(&mut self) {
                if let Ok(mut m) = ACTIVE.get_or_init(Default::default).lock() {
                    m.remove(&self.0);
                }
            }
        }
        let _remove = Remove(request_id);
        query(&root, &tools, &request, &cancel)
    })
    .await
    .map_err(|e| e.to_string())?;
    result
}
#[tauri::command]
pub(crate) fn cancel_c_intelligence(request_id: String) {
    if request_id.is_empty() || request_id.len() > 128 {
        return;
    }
    if let Ok(map) = ACTIVE.get_or_init(Default::default).lock() {
        if let Some(cancel) = map.get(&request_id) {
            cancel.store(true, Ordering::SeqCst);
        } else if let Ok(mut early) = CANCELLED_BEFORE_START.get_or_init(Default::default).lock() {
            early.retain(|_, time| time.elapsed() < Duration::from_secs(30));
            if early.len() < 64 {
                early.insert(request_id, Instant::now());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_before_async_registration_is_retained_and_does_not_start_a_query() {
        let id = "regression-early-cancel".to_owned();
        cancel_c_intelligence(id.clone());
        let flag = Arc::new(AtomicBool::new(false));
        register_request(&id, flag.clone()).unwrap();
        assert!(flag.load(Ordering::SeqCst));
        ACTIVE.get().unwrap().lock().unwrap().remove(&id);
    }
    #[test]
    fn utf16_positions_are_exact_and_surrogates_cannot_be_split() {
        let text = "// ก😀\r\nint x;\n";
        assert_eq!(position(text, 1, 0).unwrap(), text.find("int").unwrap());
        assert!(position(text, 0, 5).is_err());
        assert!(position(text, 12, 0).is_err());
        for (source, invalid_line) in [("x", 1), ("x\n", 2), ("", 1), ("x\r\ny", 2)] {
            assert!(position(source, invalid_line, 0).is_err());
        }
        assert_eq!(position("x\n", 1, 0).unwrap(), 2);
        assert_eq!(position("", 0, 0).unwrap(), 0);
    }
    #[test]
    fn semantic_ranges_validate_unsaved_utf16_and_containment() {
        let d = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(d.path()).unwrap();
        std::fs::create_dir(root.join("main")).unwrap();
        let file = root.join("main/a.c");
        std::fs::write(&file, "x").unwrap();
        let text = "/* ไทย😀 */ int symbol;";
        let buffers = BTreeMap::from([(file.clone(), text.to_owned())]);
        let start = text[..text.find("symbol").unwrap()].encode_utf16().count();
        let split_surrogate = text[..text.find('😀').unwrap()].encode_utf16().count() + 1;
        let valid = json!({"uri":uri(&file).unwrap(),"range":{"start":{"line":0,"character":start},"end":{"line":0,"character":start+6}}});
        assert_eq!(locations(&root, &buffers, valid.clone()).unwrap().len(), 1);
        // The unsaved range is valid although the saved file has only one byte.
        assert!(locations(&root, &BTreeMap::new(), valid.clone()).is_err());
        for range in [
            json!({"start":{"line":1,"character":0},"end":{"line":1,"character":0}}),
            json!({"start":{"line":0,"character":split_surrogate},"end":{"line":0,"character":start}}),
            json!({"start":{"line":0,"character":start+6},"end":{"line":0,"character":start}}),
            json!({"start":{"line":-1,"character":0},"end":{"line":0,"character":0}}),
        ] {
            let mut bad = valid.clone();
            bad["range"] = range;
            assert!(locations(&root, &buffers, bad).is_err());
        }
        let outside = tempfile::NamedTempFile::new().unwrap();
        let mut foreign = valid.clone();
        foreign["uri"] = json!(uri(outside.path()).unwrap());
        assert!(locations(&root, &buffers, foreign).unwrap().is_empty());
        let link = json!({"targetUri":uri(&file).unwrap(),"targetSelectionRange":valid["range"]});
        assert_eq!(locations(&root, &buffers, link).unwrap().len(), 1);
    }
    #[test]
    fn protocol_frames_are_bounded_and_reject_duplicate_lengths() {
        for bytes in [
            b"Content-Length: 999999999\r\n\r\n".as_slice(),
            b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        ] {
            assert!(frame(&mut std::io::Cursor::new(bytes)).is_err());
        }
    }
    #[test]
    fn fallback_uses_unsaved_buffers_and_rejects_other_workspaces() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        std::fs::create_dir(a.path().join("main")).unwrap();
        let file = a.path().join("main/main.c");
        std::fs::write(&file, "void old(void) {}\n").unwrap();
        let mut q = Query {
            file_path: file.to_string_lossy().into(),
            buffers: vec![Buffer {
                path: file.to_string_lossy().into(),
                text: "void fresh(void) {}\n".into(),
            }],
            line: 0,
            character: 6,
            operation: Operation::Definition,
            clangd_path: String::new(),
        };
        let answer = query(a.path(), a.path(), &q, &AtomicBool::new(false)).unwrap();
        assert_eq!(answer.engine, "tree_sitter_syntax");
        assert!(!answer.locations.is_empty());
        assert!(std::fs::read_to_string(&file).unwrap().contains("old"));
        q.buffers[0].path = b.path().join("secret.c").to_string_lossy().into();
        assert!(query(a.path(), a.path(), &q, &AtomicBool::new(false)).is_err());
    }
    #[test]
    fn compiler_command_requires_exact_configured_driver() {
        assert_eq!(
            first_argument("\"C:/Program Files/gcc.exe\" -Ifoo").unwrap(),
            "C:/Program Files/gcc.exe"
        );
        assert!(first_argument("\"unclosed").is_err());
    }
    #[test]
    fn compilation_context_rejects_foreign_drivers_plugins_and_nested_responses() {
        let d = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(d.path()).unwrap();
        std::fs::create_dir(root.join("main")).unwrap();
        std::fs::create_dir_all(root.join("build/toolchain")).unwrap();
        std::fs::create_dir(root.join("sdk")).unwrap();
        let main = root.join("main/main.c");
        std::fs::write(&main, "int x;").unwrap();
        let driver = root.join("sdk/xtensa-esp32-elf-gcc.exe");
        std::fs::write(&driver, "validation-only fixture; never executed").unwrap();
        let flags = root.join("build/toolchain/cflags");
        std::fs::write(&flags, "-mlongcalls\n").unwrap();
        let buffers = BTreeMap::from([(main.clone(), "int x;".into())]);
        let make = |command: String| {
            std::fs::write(
                root.join("build/compile_commands.json"),
                json!([{"directory":root.join("build"),"file":main,"command":command}]).to_string(),
            )
            .unwrap()
        };
        make(format!("\"{}\" @\"{}\"", driver.display(), flags.display()));
        assert!(compile_context(&root, &main, &buffers, &root.join("sdk")).is_ok());
        assert!(compile_context(&root, &main, &buffers, &root.join("build")).is_err());
        make(format!("\"{}\" -fplugin=malicious.dll", driver.display()));
        assert!(compile_context(&root, &main, &buffers, &root.join("sdk")).is_err());
        make(format!("\"{}\" @\"{}\"", driver.display(), flags.display()));
        std::fs::write(&flags, "@private.rsp").unwrap();
        assert!(compile_context(&root, &main, &buffers, &root.join("sdk")).is_err());
    }
}
