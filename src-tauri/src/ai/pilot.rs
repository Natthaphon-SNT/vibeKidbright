//! Native pilot boundary. Synthetic execution exercises the real reservation,
//! provenance, serialization and evidence path, never provider/model quality.
//! Actual execution remains sealed until a provider-specific cost verifier exists.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

const REQUEST_LIMIT: u32 = 10;
const USD_LIMIT_NANO: u64 = 1_000_000_000;
const MAX_BYTES: usize = 256 * 1024;
const CASES: &str = include_str!("../../../docs/ai-engineering/real-vibe-coder-cases.json");
pub(crate) const ACTUAL_BLOCKED: &str = "Inference blocked: native pilot safeguards require a verified provider/model cost bound and explicitly authorized target. No desktop connection or model discovery was attempted. Synthetic pricing cannot authorize an actual run.";

pub(crate) fn actual_preflight() -> std::result::Result<(), String> {
    // Deliberately no bool, environment variable, CLI option or serialized
    // attestation can manufacture verified provider pricing. Localhost is not free.
    Err(ACTUAL_BLOCKED.into())
}

pub(crate) fn implementation_revision() -> String {
    let mut bytes = include_bytes!("pilot.rs").to_vec();
    bytes.extend_from_slice(include_bytes!("providers.rs"));
    bytes.extend_from_slice(include_bytes!("tools.rs"));
    bytes.extend_from_slice(include_bytes!("../ai_chat.rs"));
    bytes.extend_from_slice(include_bytes!("config.rs"));
    bytes.extend_from_slice(include_bytes!("wiki.rs"));
    bytes.extend_from_slice(include_bytes!("../lib.rs"));
    bytes.extend_from_slice(include_bytes!("../intelligence/block_to_c.rs"));
    bytes.extend_from_slice(CASES.as_bytes());
    crate::intelligence::digest(&bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    Scope,
    Revision,
    Payload,
    Cost,
    Ceiling,
    Ledger,
    Recovery,
    Transport,
    Redirect,
    Response,
    Tool,
}
impl Fault {
    fn code(self) -> &'static str {
        match self {
            Self::Scope => "public_provenance_rejected",
            Self::Revision => "source_revision_changed",
            Self::Payload => "serialized_payload_rejected",
            Self::Cost => "cost_bound_unverifiable",
            Self::Ceiling => "session_ceiling_reached",
            Self::Ledger => "session_state_unverifiable",
            Self::Recovery => "explicit_reauthorization_required_no_automatic_recovery",
            Self::Transport => "transport_failure_details_omitted",
            Self::Redirect => "redirect_blocked",
            Self::Response => "response_rejected",
            Self::Tool => "tool_not_authorized",
        }
    }
}
type Result<T> = std::result::Result<T, Fault>;

/// Native IPC owns the route, not the webview/CLI. No IPC or environment
/// constructor installs a session, credential or synthetic pricing as authority.
/// A provider-specific verified-cost provisioner is intentionally still absent.
pub(crate) struct PilotState(Arc<Mutex<Option<Route>>>);
impl PilotState {
    pub(crate) fn closed() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

pub(crate) async fn send_isolated_message(
    app: &tauri::AppHandle,
    messages: &[crate::ai_chat::ChatMessage],
    project: &str,
) -> std::result::Result<(), String> {
    // Must precede any credential lookup, metadata query or config/context read.
    actual_preflight()?;
    use tauri::{Emitter, Manager};
    let state = app
        .try_state::<PilotState>()
        .ok_or_else(|| ACTUAL_BLOCKED.to_owned())?;
    let route = Arc::clone(&state.0);
    let latest = messages
        .last()
        .filter(|m| m.role == "user")
        .and_then(|m| m.content.as_str())
        .ok_or_else(|| "Pilot requires an unchanged authorized public case prompt".to_owned())?
        .to_owned();
    let project = project.to_owned();
    let value = tokio::task::spawn_blocking(move || -> Result<Value> {
        let mut state = route.lock().map_err(|_| Fault::Ledger)?;
        let route = state.as_mut().ok_or(Fault::Cost)?;
        if route.session.mode != Mode::Actual {
            return Err(Fault::Cost);
        }
        if std::fs::canonicalize(project).map_err(|_| Fault::Scope)?
            != route.session.scope.fixture_root
        {
            return Err(Fault::Scope);
        }
        let cases: Value = serde_json::from_str(CASES).map_err(|_| Fault::Scope)?;
        let case = cases["cases"]
            .as_array()
            .ok_or(Fault::Scope)?
            .iter()
            .find(|v| v["prompt"].as_str() == Some(&latest))
            .and_then(|v| v["id"].as_str())
            .ok_or(Fault::Scope)?;
        let transport = NativeHttpTransport::new()?;
        // Metadata and all following tool/retry calls share this native route's
        // session. Recreating the HTTP connection cannot create another budget.
        route.metadata(case, &transport)?;
        route.turn(case, &transport, &Credential(String::new()))
    })
    .await
    .map_err(|_| "Pilot worker stopped; session requires verification".to_owned())?
    .map_err(|fault| fault.code().to_owned())?;
    let text = value["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("");
    app.emit("ai-chat-delta", text)
        .map_err(|_| "Pilot event delivery failed".to_owned())?;
    app.emit("ai-chat-done", json!({"text":text}).to_string())
        .map_err(|_| "Pilot event delivery failed".to_owned())?;
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    crate::intelligence::digest(bytes)
}
fn no_link(path: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| Fault::Scope)?;
    if meta.file_type().is_symlink() {
        return Err(Fault::Scope);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(Fault::Scope);
        }
    }
    Ok(())
}
fn no_link_ancestry(path: &Path) -> Result<()> {
    let absolute = std::path::absolute(path).map_err(|_| Fault::Scope)?;
    let mut cursor = PathBuf::new();
    for part in absolute.components() {
        cursor.push(part);
        if matches!(part, Component::Normal(_)) {
            no_link(&cursor)?;
        }
    }
    Ok(())
}
fn relative_path(name: &str) -> Result<&Path> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err(Fault::Scope);
    }
    Ok(path)
}
fn public_file(root: &Path, name: &str) -> Result<Vec<u8>> {
    no_link_ancestry(root)?;
    let relative = relative_path(name)?;
    let mut cursor = root.to_owned();
    for component in relative.components() {
        cursor.push(component);
        no_link(&cursor)?;
    }
    let canonical = std::fs::canonicalize(&cursor).map_err(|_| Fault::Scope)?;
    if !canonical.starts_with(root) {
        return Err(Fault::Scope);
    }
    let meta = std::fs::metadata(&canonical).map_err(|_| Fault::Scope)?;
    if !meta.is_file() || meta.len() > MAX_BYTES as u64 {
        return Err(Fault::Scope);
    }
    std::fs::read(canonical).map_err(|_| Fault::Scope)
}

/// Constructed from the deterministic public generator and pinned KB manifest,
/// never from a frontend-supplied "public": true or an arbitrary directory hash.
struct Scope {
    fixture_root: PathBuf,
    kb_root: PathBuf,
    revision: String,
    files: BTreeMap<String, String>,
}
fn fixture_files(name: &str) -> Result<BTreeMap<String, String>> {
    let text = match name {
        "repeat_delay" => {
            include_str!("../../../docs/ai-engineering/block-fixtures/repeat-delay.json")
        }
        "forever_delay" => {
            include_str!("../../../docs/ai-engineering/block-fixtures/forever-delay.json")
        }
        _ => return Err(Fault::Scope),
    };
    let program = serde_json::from_str(text).map_err(|_| Fault::Scope)?;
    let mut files: BTreeMap<_, _> = crate::intelligence::block_to_c::project_files(&program)
        .map_err(|_| Fault::Scope)?
        .into_iter()
        .collect();
    files.insert("CMakeLists.txt".into(), "cmake_minimum_required(VERSION 3.16)\ninclude($ENV{IDF_PATH}/tools/cmake/project.cmake)\nproject(kidbright_block_pilot)\n".into());
    files.insert(
        "sdkconfig.defaults".into(),
        "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n".into(),
    );
    Ok(files)
}
impl Scope {
    fn bind(fixture: &Path, kb: &Path, name: &str, revision: &str) -> Result<Self> {
        if revision != implementation_revision() {
            return Err(Fault::Revision);
        }
        no_link_ancestry(fixture)?;
        no_link_ancestry(kb)?;
        let fixture_root = std::fs::canonicalize(fixture).map_err(|_| Fault::Scope)?;
        let kb_root = std::fs::canonicalize(kb).map_err(|_| Fault::Scope)?;
        // In non-test builds the roots are fixed, public, repo-owned sources.
        #[cfg(not(test))]
        {
            let base = Path::new(env!("CARGO_MANIFEST_DIR"));
            if fixture_root
                != std::fs::canonicalize(base.join("target/block-c-evaluation-20261018").join(name))
                    .map_err(|_| Fault::Scope)?
                || kb_root
                    != std::fs::canonicalize(base.join("../resources/knowledge_base"))
                        .map_err(|_| Fault::Scope)?
            {
                return Err(Fault::Scope);
            }
        }
        let mut files = BTreeMap::new();
        for (relative, expected) in fixture_files(name)? {
            let bytes = public_file(&fixture_root, &relative)?;
            if super::privacy::normalized_text_hash(&bytes)
                != super::privacy::normalized_text_hash(expected.as_bytes())
            {
                return Err(Fault::Scope);
            }
            files.insert(format!("fixture/{relative}"), hash(&bytes));
        }
        let pinned: BTreeMap<String, String> = serde_json::from_str(include_str!(
            "../../../docs/ai-engineering/bundled-source-hashes.json"
        ))
        .map_err(|_| Fault::Scope)?;
        for (relative, expected) in pinned {
            if !matches!(
                Path::new(&relative).extension().and_then(|s| s.to_str()),
                Some("c" | "h" | "md" | "txt")
            ) {
                continue;
            }
            // Test fixture includes one real pinned public KB document.
            #[cfg(test)]
            if !kb_root.join(&relative).exists() {
                continue;
            }
            let bytes = public_file(&kb_root, &relative)?;
            if super::privacy::normalized_text_hash(&bytes).as_deref() != Some(&expected) {
                return Err(Fault::Scope);
            }
            files.insert(format!("kb/{relative}"), hash(&bytes));
        }
        Ok(Self {
            fixture_root,
            kb_root,
            revision: revision.into(),
            files,
        })
    }
    fn read(&self, id: &str) -> Result<Fragment> {
        let expected = self.files.get(id).ok_or(Fault::Scope)?;
        let (root, relative) = if let Some(s) = id.strip_prefix("fixture/") {
            (&self.fixture_root, s)
        } else if let Some(s) = id.strip_prefix("kb/") {
            (&self.kb_root, s)
        } else {
            return Err(Fault::Scope);
        };
        let bytes = public_file(root, relative)?;
        if hash(&bytes) != *expected {
            return Err(Fault::Revision);
        }
        Ok(Fragment {
            source: id.into(),
            sha256: expected.clone(),
            text: String::from_utf8(bytes).map_err(|_| Fault::Scope)?,
        })
    }
    fn verify(&self) -> Result<()> {
        if self.revision != implementation_revision() {
            return Err(Fault::Revision);
        }
        for id in self.files.keys() {
            self.read(id)?;
        }
        Ok(())
    }
    fn approve_tool_result(&self, id: &str, reported: &str) -> Result<Fragment> {
        let fragment = self.read(id)?;
        if fragment.text != reported {
            return Err(Fault::Scope);
        }
        Ok(fragment)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fragment {
    source: String,
    sha256: String,
    text: String,
}

/// Pricing units below are SYNTHETIC fixtures, not actual provider prices.
/// No actual-provider proof variant/constructor is available in this increment.
#[derive(Clone)]
struct SyntheticPricing {
    input_tokens_per_byte_upper: u64,
    input_nano_per_token: u64,
    output_nano_per_token: u64,
    max_output_tokens: u64,
    other_nano_per_attempt: u64,
    metadata_nano_upper: Option<u64>,
    all_charges_bounded: bool,
}
impl SyntheticPricing {
    fn reserve(&self, kind: Kind, bytes: usize) -> Result<u64> {
        if !self.all_charges_bounded {
            return Err(Fault::Cost);
        }
        if kind == Kind::Metadata {
            return self.metadata_nano_upper.ok_or(Fault::Cost);
        }
        if self.max_output_tokens == 0 || self.input_tokens_per_byte_upper == 0 {
            return Err(Fault::Cost);
        }
        (bytes as u64)
            .checked_mul(self.input_tokens_per_byte_upper)
            .and_then(|v| v.checked_mul(self.input_nano_per_token))
            .and_then(|v| {
                v.checked_add(
                    self.max_output_tokens
                        .checked_mul(self.output_nano_per_token)?,
                )
            })
            .and_then(|v| v.checked_add(self.other_nano_per_attempt))
            .ok_or(Fault::Cost)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    Metadata,
    Inference,
    ToolLoop,
    Retry,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Actual,
    Synthetic,
}
impl Mode {
    fn label(self) -> &'static str {
        match self {
            Self::Actual => "actual_provider",
            Self::Synthetic => "synthetic_transport_not_model_evidence",
        }
    }
}

struct Endpoint(reqwest::Url);
impl Endpoint {
    fn parse(value: &str) -> Result<Self> {
        let url = reqwest::Url::parse(value).map_err(|_| Fault::Scope)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path().trim_end_matches('/') != "/v1"
        {
            return Err(Fault::Scope);
        }
        Ok(Self(url))
    }
    fn url(&self, kind: Kind) -> String {
        format!(
            "{}/{}",
            self.0.as_str().trim_end_matches('/'),
            if kind == Kind::Metadata {
                "models"
            } else {
                "chat/completions"
            }
        )
    }
}
// Never Serialize or Debug credentials. Only the transport receives this channel.
struct Credential(String);
struct WireReply {
    status: u16,
    bytes: Vec<u8>,
}
trait Transport: Send + Sync {
    fn mode(&self) -> Mode;
    fn send(
        &self,
        endpoint: &Endpoint,
        kind: Kind,
        bytes: &[u8],
        credential: &Credential,
    ) -> Result<WireReply>;
}
struct NativeHttpTransport {
    client: reqwest::blocking::Client,
}
impl NativeHttpTransport {
    fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .retry(reqwest::retry::never())
                .http1_only()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|_| Fault::Transport)?,
        })
    }
}
impl Transport for NativeHttpTransport {
    fn mode(&self) -> Mode {
        Mode::Actual
    }
    fn send(
        &self,
        endpoint: &Endpoint,
        kind: Kind,
        bytes: &[u8],
        credential: &Credential,
    ) -> Result<WireReply> {
        let mut request = if kind == Kind::Metadata {
            self.client.get(endpoint.url(kind))
        } else {
            self.client
                .post(endpoint.url(kind))
                .header("Content-Type", "application/json")
                .body(bytes.to_vec())
        };
        if !credential.0.is_empty() {
            request = request.bearer_auth(&credential.0);
        }
        let mut response = request.send().map_err(|_| Fault::Transport)?;
        let status = response.status().as_u16();
        // No headers, redirect destinations or request URLs enter evidence/errors.
        use std::io::Read;
        let mut body = Vec::new();
        response
            .by_ref()
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|_| Fault::Transport)?;
        if body.len() > MAX_BYTES {
            return Err(Fault::Response);
        }
        Ok(WireReply {
            status,
            bytes: body,
        })
    }
}

fn suspicious(text: &str) -> bool {
    // Supplemental deny heuristics only. Provenance is the authorization boundary.
    let lower = text.to_lowercase();
    ["/users/", "/home/", "/opt/", "/tmp/", "appdata", "idf_path=", "idf_tools_path="].iter().any(|s| lower.contains(s))
        || regex::Regex::new(r"(?i)\b(authorization\s*[:=]|bearer\s+[a-z0-9_-]{6}|(api[_-]?key|access_token|password)\s*[:=]|sk-[a-z0-9_-]{8,}|-----begin private)").unwrap().is_match(text)
        || regex::Regex::new(r"\b[a-zA-Z]:[\\/]").unwrap().is_match(text)
        || regex::Regex::new(r#"https?://[^\s\"]*[?&][^\s\"]*="#).unwrap().is_match(text)
}
fn redact(value: &Value, credential: &Credential) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    let lower = key.to_lowercase();
                    let value = if [
                        "authorization",
                        "api_key",
                        "api-key",
                        "key",
                        "token",
                        "access_token",
                        "password",
                        "secret",
                    ]
                    .contains(&lower.as_str())
                    {
                        json!("[REDACTED]")
                    } else {
                        redact(value, credential)
                    };
                    let key = if suspicious(key)
                        || (!credential.0.is_empty() && key.contains(&credential.0))
                    {
                        "[REDACTED_FIELD]".into()
                    } else {
                        key.clone()
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(items) => json!(items
            .iter()
            .map(|v| redact(v, credential))
            .collect::<Vec<_>>()),
        Value::String(text) => {
            let text = if credential.0.is_empty() {
                text.clone()
            } else {
                text.replace(&credential.0, "[REDACTED]")
            };
            if suspicious(&text) {
                json!("[REDACTED_UNTRUSTED_TEXT]")
            } else {
                json!(text)
            }
        }
        _ => value.clone(),
    }
}

fn serialize_request(
    history: &[crate::ai_chat::ChatMessage],
    model: &str,
    max_tokens: u64,
) -> Result<Vec<u8>> {
    for message in history {
        if message.role == "tool"
            && message.content.get("__tool_response__").is_some()
            && (message.content.get("tool_call_id").is_none()
                || message.content.get("content").is_none())
        {
            return Err(Fault::Scope);
        }
    }
    let messages =
        super::providers::build_api_messages(super::providers::SYSTEM_PROMPT, history, model);
    serde_json::to_vec(&json!({"model":model,"messages":messages,"max_tokens":max_tokens,"stream":false,
        "tools":[{"type":"function","function":{"name":"read_file","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}}]})).map_err(|_|Fault::Payload)
}
fn assistant_record(message: &Value) -> Option<crate::ai_chat::ChatMessage> {
    if message["tool_calls"]
        .as_array()
        .is_some_and(|calls| !calls.is_empty())
    {
        Some(crate::ai_chat::ChatMessage {
            role: "assistant".into(),
            content: json!({"__tool_calls__":message["tool_calls"],"__text__":message["content"]}),
        })
    } else if message["content"].is_string() {
        Some(crate::ai_chat::ChatMessage {
            role: "assistant".into(),
            content: message["content"].clone(),
        })
    } else {
        None
    }
}
fn message_hash(message: &crate::ai_chat::ChatMessage) -> Result<String> {
    Ok(hash(
        &serde_json::to_vec(message).map_err(|_| Fault::Payload)?,
    ))
}
struct Envelope {
    bytes: Vec<u8>,
    history: Vec<crate::ai_chat::ChatMessage>,
    model: String,
    max_tokens: u64,
}
impl Envelope {
    fn metadata() -> Self {
        Self {
            bytes: Vec::new(),
            history: Vec::new(),
            model: String::new(),
            max_tokens: 0,
        }
    }
    fn validate(
        &self,
        scope: &Scope,
        credential: &Credential,
        approved_assistant: &BTreeSet<String>,
    ) -> Result<()> {
        scope.verify()?;
        if self.bytes.len() > MAX_BYTES {
            return Err(Fault::Payload);
        }
        if !self.bytes.is_empty() {
            if self.bytes != serialize_request(&self.history, &self.model, self.max_tokens)? {
                return Err(Fault::Payload);
            }
            for message in &self.history {
                match message.role.as_str() {
                    "user" => {
                        let cases: Value = serde_json::from_str(CASES).map_err(|_| Fault::Scope)?;
                        if !cases["cases"]
                            .as_array()
                            .ok_or(Fault::Scope)?
                            .iter()
                            .any(|c| c["prompt"].is_string() && c["prompt"] == message.content)
                        {
                            return Err(Fault::Scope);
                        }
                    }
                    "assistant" => {
                        if !approved_assistant.contains(&message_hash(message)?) {
                            return Err(Fault::Scope);
                        }
                    }
                    "tool" => {
                        let object = &message.content;
                        if object["__tool_response__"] != true
                            || object["__func_name__"] != "read_file"
                        {
                            return Err(Fault::Scope);
                        }
                        let fragment: Fragment =
                            serde_json::from_str(object["content"].as_str().ok_or(Fault::Scope)?)
                                .map_err(|_| Fault::Scope)?;
                        let verified =
                            scope.approve_tool_result(&fragment.source, &fragment.text)?;
                        if verified.sha256 != fragment.sha256 {
                            return Err(Fault::Scope);
                        }
                    }
                    _ => return Err(Fault::Scope),
                }
            }
        } else if !self.history.is_empty() {
            return Err(Fault::Payload);
        }
        let text = std::str::from_utf8(&self.bytes).map_err(|_| Fault::Payload)?;
        if suspicious(text) || (!credential.0.is_empty() && text.contains(&credential.0)) {
            return Err(Fault::Payload);
        }
        Ok(())
    }
}

/// An append-only, fsync-before-send ledger, exclusively owned for this session.
/// No Drop cleanup; crashes cannot silently recycle an authorization directory.
struct Ledger {
    attempts: u32,
    reserved_nano: u64,
    expected_bytes: Vec<u8>,
    poisoned: bool,
    approved_assistant: BTreeSet<String>,
}
struct Session {
    id: String,
    scope: Scope,
    endpoint: Endpoint,
    mode: Mode,
    pricing: Option<SyntheticPricing>,
    directory: PathBuf,
    ledger: Mutex<Ledger>,
    // A second independently opened Session cannot acquire this directory.
    _owner: File,
}
static CONSUMED_AUTHORIZATIONS: OnceLock<Mutex<BTreeMap<String, PathBuf>>> = OnceLock::new();
impl Session {
    fn create(
        directory: &Path,
        id: &str,
        scope: Scope,
        endpoint: Endpoint,
        mode: Mode,
        pricing: Option<SyntheticPricing>,
    ) -> Result<Arc<Self>> {
        if id.is_empty()
            || id.len() > 80
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err(Fault::Scope);
        }
        if mode == Mode::Actual {
            actual_preflight().map_err(|_| Fault::Cost)?;
        }
        let absolute = std::path::absolute(directory).map_err(|_| Fault::Ledger)?;
        #[cfg(not(test))]
        {
            let permitted = std::fs::canonicalize(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("target/desktop-evaluation"),
            )
            .map_err(|_| Fault::Ledger)?;
            let parent = std::fs::canonicalize(absolute.parent().ok_or(Fault::Ledger)?)
                .map_err(|_| Fault::Ledger)?;
            if !parent.starts_with(permitted) {
                return Err(Fault::Scope);
            }
            no_link_ancestry(&parent)?;
        }
        let mut consumed = CONSUMED_AUTHORIZATIONS
            .get_or_init(|| Mutex::new(BTreeMap::new()))
            .lock()
            .map_err(|_| Fault::Ledger)?;
        if consumed.contains_key(id) || consumed.values().any(|path| path == &absolute) {
            return Err(Fault::Recovery);
        }
        // Consume even a failed initialization; never silently retry as a new grant.
        consumed.insert(id.into(), absolute);
        drop(consumed);
        std::fs::create_dir(directory).map_err(|_| Fault::Recovery)?;
        let directory = std::fs::canonicalize(directory).map_err(|_| Fault::Ledger)?;
        let owner = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("owner.lock"))
            .map_err(|_| Fault::Ledger)?;
        let initial = json!({"schema":1,"session":id,"source_revision":scope.revision,"fixture_snapshot_sha256":hash(&serde_json::to_vec(&scope.files).map_err(|_| Fault::Ledger)?),"mode":mode.label(),"request_limit":REQUEST_LIMIT,"usd_limit_nano":USD_LIMIT_NANO,"estimated_cost":null}).to_string()+"\n";
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("ledger.jsonl"))
            .map_err(|_| Fault::Ledger)?;
        file.write_all(initial.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| Fault::Ledger)?;
        Ok(Arc::new(Self {
            id: id.into(),
            scope,
            endpoint,
            mode,
            pricing,
            directory,
            _owner: owner,
            ledger: Mutex::new(Ledger {
                attempts: 0,
                reserved_nano: 0,
                expected_bytes: initial.into_bytes(),
                poisoned: false,
                approved_assistant: BTreeSet::new(),
            }),
        }))
    }
    fn recover(_directory: &Path) -> Result<Arc<Self>> {
        // A disk hash chain alone cannot prove no valid suffix was truncated.
        // Until an independently trusted checkpoint/re-authorization exists,
        // restart is fail-closed, not a reset to zero or best-effort replay.
        Err(Fault::Recovery)
    }
    fn intact(&self, ledger: &mut Ledger) -> Result<()> {
        if ledger.poisoned
            || no_link_ancestry(&self.directory).is_err()
            || no_link(&self.directory.join("owner.lock")).is_err()
            || no_link(&self.directory.join("ledger.jsonl")).is_err()
            || std::fs::read(self.directory.join("ledger.jsonl"))
                .ok()
                .as_deref()
                != Some(ledger.expected_bytes.as_slice())
        {
            ledger.poisoned = true;
            return Err(Fault::Ledger);
        }
        Ok(())
    }
    fn append(&self, ledger: &mut Ledger, event: Value) -> Result<()> {
        self.intact(ledger)?;
        let record = json!({"previous_sha256":hash(&ledger.expected_bytes),"event":event});
        let line = record.to_string() + "\n";
        let result = OpenOptions::new()
            .append(true)
            .open(self.directory.join("ledger.jsonl"))
            .and_then(|mut file| {
                file.write_all(line.as_bytes())
                    .and_then(|_| file.sync_all())
            });
        if result.is_err() {
            ledger.poisoned = true;
            return Err(Fault::Ledger);
        }
        ledger.expected_bytes.extend_from_slice(line.as_bytes());
        Ok(())
    }
    fn event(
        &self,
        case: &str,
        turn: u32,
        kind: Kind,
        envelope: &Envelope,
        attempt: Option<u32>,
        outcome: &str,
    ) -> Value {
        json!({"session_id":self.id,"case_id":case,"turn":turn,"kind":kind,"attempt_number":attempt,
            "source_revision":self.scope.revision,"payload_sha256":hash(&envelope.bytes),"outcome":outcome,
            "source_revision_kind":"compiled_native_sources_and_public_cases",
            "evidence_kind":self.mode.label(),"actual_model_quality_evidence":false})
    }
    fn blocked(
        &self,
        case: &str,
        turn: u32,
        kind: Kind,
        envelope: &Envelope,
        fault: Fault,
    ) -> Result<()> {
        let mut ledger = self.ledger.lock().map_err(|_| Fault::Ledger)?;
        let mut event = self.event(case, turn, kind, envelope, None, "blocked_before_send");
        event["reason"] = json!(fault.code());
        event["next_attempt_number"] = json!(ledger.attempts + 1);
        // Unapproved serialized bytes are never copied into the evidence log.
        self.append(&mut ledger, event)
    }
    fn dispatch(
        &self,
        case: &str,
        turn: u32,
        kind: Kind,
        envelope: &Envelope,
        credential: &Credential,
        transport: &dyn Transport,
    ) -> Result<WireReply> {
        // Only predefined public cases/IDs enter evidence, never attacker labels.
        case_prompt(case)?;
        let checked = (|| {
            if self.mode == Mode::Actual {
                actual_preflight().map_err(|_| Fault::Cost)?;
            }
            if transport.mode() != self.mode {
                return Err(Fault::Cost);
            }
            if (kind == Kind::Metadata) != envelope.bytes.is_empty() {
                return Err(Fault::Payload);
            }
            let ledger = self.ledger.lock().map_err(|_| Fault::Ledger)?;
            envelope.validate(&self.scope, credential, &ledger.approved_assistant)?;
            if kind != Kind::Metadata
                && envelope.max_tokens
                    != self.pricing.as_ref().ok_or(Fault::Cost)?.max_output_tokens
            {
                return Err(Fault::Cost);
            }
            self.pricing
                .as_ref()
                .ok_or(Fault::Cost)?
                .reserve(kind, envelope.bytes.len())
        })();
        let cost = match checked {
            Ok(cost) => cost,
            Err(fault) => {
                self.blocked(case, turn, kind, envelope, fault)?;
                return Err(fault);
            }
        };
        let attempt = {
            // The mutex transaction is the atomic reservation across all cases,
            // turns, metadata, retries and connections. Persist before releasing.
            let mut ledger = self.ledger.lock().map_err(|_| Fault::Ledger)?;
            self.intact(&mut ledger)?;
            if ledger.attempts >= REQUEST_LIMIT
                || ledger
                    .reserved_nano
                    .checked_add(cost)
                    .is_none_or(|v| v > USD_LIMIT_NANO)
            {
                let mut event = self.event(case, turn, kind, envelope, None, "blocked_before_send");
                event["reason"] = json!(Fault::Ceiling.code());
                event["next_attempt_number"] = json!(ledger.attempts + 1);
                self.append(&mut ledger, event)?;
                return Err(Fault::Ceiling);
            }
            let attempt = ledger.attempts + 1;
            let mut event = self.event(
                case,
                turn,
                kind,
                envelope,
                Some(attempt),
                "reserved_before_send",
            );
            event["serialized_request_redacted"] = if envelope.bytes.is_empty() {
                Value::Null
            } else {
                redact(
                    &serde_json::from_slice::<Value>(&envelope.bytes)
                        .map_err(|_| Fault::Payload)?,
                    credential,
                )
            };
            event["reserved_cost_nano"] = json!(cost);
            event["pricing_evidence"] = json!("synthetic_fixture_not_provider_pricing");
            self.append(&mut ledger, event)?;
            ledger.attempts = attempt;
            ledger.reserved_nano += cost;
            attempt
        };
        // Once reserved, a cancellation, crash, timeout or failed send is charged
        // to the ceiling. Usage after a response never refunds the reservation.
        let result = transport.send(&self.endpoint, kind, &envelope.bytes, credential);
        let mut event = self.event(
            case,
            turn,
            kind,
            envelope,
            Some(attempt),
            "transport_attempted",
        );
        match &result {
            Ok(reply) => {
                event["status"] = json!(reply.status);
                event["response_sha256"] = json!(hash(&reply.bytes));
                // Response text/keys may contain unknown secrets. Preserve the
                // real bytes' hash/length and numeric usage, omit ALL untrusted
                // free text instead of claiming regex is a complete detector.
                let value = serde_json::from_slice::<Value>(&reply.bytes).unwrap_or(Value::Null);
                let mut usage = serde_json::Map::new();
                for key in [
                    "prompt_tokens",
                    "completion_tokens",
                    "total_tokens",
                    "input_tokens",
                    "output_tokens",
                ] {
                    if value["usage"][key].is_u64() {
                        usage.insert(key.into(), value["usage"][key].clone());
                    }
                }
                event["response_redacted"] = json!({"body":"[UNTRUSTED_RESPONSE_TEXT_OMITTED]","bytes":reply.bytes.len(),"usage":usage});
                if (300..400).contains(&reply.status) {
                    event["outcome"] = json!("redirect_blocked_after_single_send");
                }
            }
            Err(fault) => event["error"] = json!(fault.code()),
        }
        let mut ledger = self.ledger.lock().map_err(|_| Fault::Ledger)?;
        self.append(&mut ledger, event)?;
        let reply = result?;
        if (200..300).contains(&reply.status) {
            if let Ok(value) = serde_json::from_slice::<Value>(&reply.bytes) {
                if let Some(message) = assistant_record(&value["choices"][0]["message"]) {
                    ledger.approved_assistant.insert(message_hash(&message)?);
                }
            }
        }
        if (300..400).contains(&reply.status) {
            return Err(Fault::Redirect);
        }
        Ok(reply)
    }
}

fn case_prompt(case: &str) -> Result<String> {
    let value: Value = serde_json::from_str(CASES).map_err(|_| Fault::Scope)?;
    value["cases"]
        .as_array()
        .ok_or(Fault::Scope)?
        .iter()
        .find(|v| v["id"] == case)
        .and_then(|v| v["prompt"].as_str())
        .map(str::to_owned)
        .ok_or(Fault::Scope)
}

/// Evaluation provider/tool loop. It never accepts arbitrary frontend history or
/// private tool-result JSON. The last serialized bytes are validated at dispatch.
struct Route {
    session: Arc<Session>,
    history: Vec<crate::ai_chat::ChatMessage>,
    model: String,
}
impl Route {
    fn new(session: Arc<Session>, model: &str) -> Result<Self> {
        if model.is_empty()
            || model.len() > 100
            || !model
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._/".contains(c))
        {
            return Err(Fault::Scope);
        }
        Ok(Self {
            session,
            history: Vec::new(),
            model: model.into(),
        })
    }
    fn envelope(&self) -> Result<Envelope> {
        let proof = self.session.pricing.as_ref().ok_or(Fault::Cost)?;
        let bytes = serialize_request(&self.history, &self.model, proof.max_output_tokens)?;
        Ok(Envelope {
            bytes,
            history: self.history.clone(),
            model: self.model.clone(),
            max_tokens: proof.max_output_tokens,
        })
    }
    fn metadata(&self, case: &str, transport: &dyn Transport) -> Result<WireReply> {
        self.session.dispatch(
            case,
            0,
            Kind::Metadata,
            &Envelope::metadata(),
            &Credential(String::new()),
            transport,
        )
    }
    fn turn(
        &mut self,
        case: &str,
        transport: &dyn Transport,
        credential: &Credential,
    ) -> Result<Value> {
        self.history.push(crate::ai_chat::ChatMessage {
            role: "user".into(),
            content: json!(case_prompt(case)?),
        });
        let mut tool_turn = 0;
        let mut retries = 0;
        loop {
            let envelope = self.envelope()?;
            let kind = if retries > 0 {
                Kind::Retry
            } else if tool_turn > 0 {
                Kind::ToolLoop
            } else {
                Kind::Inference
            };
            let reply = match self
                .session
                .dispatch(case, tool_turn, kind, &envelope, credential, transport)
            {
                Err(Fault::Transport) if retries < 2 => {
                    retries += 1;
                    continue;
                }
                result => result?,
            };
            if matches!(reply.status, 429 | 500..=599) && retries < 2 {
                retries += 1;
                continue;
            }
            if !(200..300).contains(&reply.status) {
                return Err(Fault::Response);
            }
            let value: Value = serde_json::from_slice(&reply.bytes).map_err(|_| Fault::Response)?;
            let message = &value["choices"][0]["message"];
            let calls = message["tool_calls"].as_array();
            if let Some(calls) = calls.filter(|c| !c.is_empty()) {
                if tool_turn >= 3 || calls.len() > 4 {
                    return Err(Fault::Tool);
                }
                // Model bytes are untrusted too: reject suspicious output before
                // reflecting it, and never interpret them as a grant for file IO.
                if suspicious(&message.to_string()) {
                    return Err(Fault::Payload);
                }
                self.history.push(crate::ai_chat::ChatMessage {
                    role: "assistant".into(),
                    content: json!({"__tool_calls__":calls,"__text__":message["content"]}),
                });
                for call in calls {
                    if call["function"]["name"] != "read_file" {
                        return Err(Fault::Tool);
                    }
                    let args: Value = serde_json::from_str(
                        call["function"]["arguments"].as_str().ok_or(Fault::Tool)?,
                    )
                    .map_err(|_| Fault::Tool)?;
                    let id = args["path"].as_str().ok_or(Fault::Scope)?;
                    let fragment = match self.session.scope.read(id) {
                        Ok(v) => v,
                        Err(fault) => {
                            self.session.blocked(
                                case,
                                tool_turn,
                                Kind::ToolLoop,
                                &envelope,
                                fault,
                            )?;
                            return Err(fault);
                        }
                    };
                    let result = serde_json::to_string(&fragment).map_err(|_| Fault::Payload)?;
                    self.history.push(crate::ai_chat::ChatMessage {role:"tool".into(),content:json!({"__tool_response__":true,"tool_call_id":call["id"],"__func_name__":"read_file","content":result})});
                }
                retries = 0;
                tool_turn += 1;
            } else {
                // Retain real response/usage only; no fabricated grades or usage.
                if !message["content"].is_string() {
                    return Err(Fault::Response);
                }
                self.history.push(crate::ai_chat::ChatMessage {
                    role: "assistant".into(),
                    content: message["content"].clone(),
                });
                return Ok(value);
            }
        }
    }
}

#[cfg(test)]
mod tests;
