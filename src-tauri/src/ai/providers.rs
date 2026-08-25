//! LLM provider integration: OpenAI-compatible + Google Generative AI.
//!
//! Extracted from ai_chat.rs (refactoring roadmap step 5).
//! Contains the streaming conversation loops (with tool-calling),
//! tool schemas, message builders, shared HTTP clients and rate-limit cache.

use crate::ai::tools::execute_tool;
use crate::ai_chat::{AiAbortState, ChatMessage};
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{atomic::Ordering, Arc, Mutex, OnceLock};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
    /// Preserved from Gemini thinking-mode responses — must be echoed back verbatim.
    thought_signature: Option<String>,
}

static RATE_LIMITED_MODELS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
pub(crate) fn get_rate_limited_models() -> &'static Mutex<HashMap<String, Instant>> {
    RATE_LIMITED_MODELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Static HTTP client for cloud APIs (OpenRouter, OpenAI) — shared connection pool.
/// Timeout: 120s total. Recreating a Client per request causes socket exhaustion
/// and causes each subsequent request to take progressively longer.
static CLOUD_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
fn get_cloud_client() -> &'static Client {
    CLOUD_HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

/// Static HTTP client for local LLM servers (Ollama, LM Studio).
/// Auto-decompression disabled — local SSE is plain text, not gzip/brotli.
/// Timeout: 600s to allow slow models to finish.
static LOCAL_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
fn get_local_client() -> &'static Client {
    LOCAL_HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(std::time::Duration::from_secs(15))
            .timeout(std::time::Duration::from_secs(600))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

const MAX_TOOL_TURNS: u32 = 20;

// ── Conversation loop ─────────────────────────────────────────────────────────
// D2: SYSTEM_PROMPT is now stored in ai/system_prompt.txt and loaded at compile
// time via include_str!. This makes the prompt editable independently of the
// Rust source, and reduces this file by ~812 lines.
pub(crate) const SYSTEM_PROMPT: &str = include_str!("system_prompt.txt");



fn get_tools() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read file content. Always use this before editing a file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to project root" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "CRITICAL: You MUST use this tool to write or modify any code. DO NOT put code blocks in your chat response. Overwrites existing content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to project root (e.g. main/main.c)" },
                        "content": { "type": "string", "description": "The FULL complete new file content. DO NOT truncate. DO NOT put comments like '// rest of code here', write the entire file." }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in a directory (shallow, one level).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to list (use '.' for root)" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "get_file_tree",
                "description": "Recursively list all files in the project as an indented tree. Use this to understand the full project structure at a glance before making changes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum depth to recurse (default 4, max 8)"
                        }
                    },
                    "required": []
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_in_files",
                "description": "Search for a text pattern (regex) across all source files in the project. Returns matching lines with file path and line number. Use this to find function definitions, usages, or error strings.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search for"
                        },
                        "file_extension": {
                            "type": "string",
                            "description": "Optional file extension filter, e.g. 'c', 'h', 'cmake'. Leave empty to search all text files."
                        }
                    },
                    "required": ["pattern"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "diff_file",
                "description": "Compute a unified diff between the current content of a file and proposed new content, without writing anything. Use this to preview changes before calling write_file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path relative to project root" },
                        "new_content": { "type": "string", "description": "The proposed new file content" }
                    },
                    "required": ["path", "new_content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Run a shell command (e.g. 'idf.py build'). Output is returned after the command completes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "The command string" }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "esp_idf_info",
                "description": "Get resolved ESP-IDF paths and execution hints for this runtime.",
                "parameters": { "type": "object", "properties": {}, "required": [] }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "install_idf_library",
                "description": "Install an ESP-IDF component dependency using idf.py add-dependency.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "component": { "type": "string", "description": "Component identifier, e.g. espressif/led_strip or espressif/led_strip^2.5.3" }
                    },
                    "required": ["component"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "CRITICAL: Search the internet for latest technical documentation, ESP-IDF API changes, hardware specs, or code examples when your internal knowledge is insufficient or potentially outdated.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The technical search query" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "knowledge_search",
                "description": "Search the local knowledge_base folder for project-specific documentation, rules, or technical notes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query or keywords" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_knowledge_file",
                "description": "Read the FULL content of a file from the knowledge_base folder. Use this when knowledge_search returns partial/truncated results or says the KB is not loaded.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_name": {
                            "type": "string",
                            "description": "File name relative to knowledge_base/, e.g. 'formula_kid_controller.md'"
                        }
                    },
                    "required": ["file_name"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "create_project_workspace",
                "description": "Create a new project workspace directory. Prompts the user to pick a folder, then makes a subfolder with project_name inside it, and switches the IDE workspace to it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "project_name": { "type": "string", "description": "The name of the new project (e.g. my_esp_project)." }
                    },
                    "required": ["project_name"]
                }
            }
        }
    ])
}

// ── Conversation loop (OpenAI-compatible) ─────────────────────────────────────

pub(crate) async fn run_conversation_loop(
    app_handle: &AppHandle,
    api_key: &str,
    model: &str,
    base_url: &str,
    mut messages: Vec<ChatMessage>,
    project_path: &mut PathBuf,
    is_openrouter: bool,
    // FIX: Arc<str> instead of &str — survives async boundaries, zero-cost to clone.
    message_id: Arc<str>,
    no_workspace: &mut bool,
    system_prompt: &str,
) -> Result<(), String> {
    // Detect local/LAN server early — affects client config, timeout, and encoding headers.
    // Covers: localhost, 127.x.x.x, 10.x.x.x, 172.16-31.x.x, 192.168.x.x, bare IP URLs.
    let is_local_url = {
        let host = base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");
        host == "localhost"
            || host.starts_with("127.")
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || {
                // 172.16.0.0/12 → 172.16.x.x … 172.31.x.x
                if let Some(second_octet_str) = host.strip_prefix("172.") {
                    second_octet_str
                        .split('.')
                        .next()
                        .and_then(|s| s.parse::<u8>().ok())
                        .map(|n| (16..=31).contains(&n))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            || !is_openrouter && {
                // Treat any bare IPv4 address (no letters in hostname) as local/LAN.
                host.chars().all(|c| c.is_ascii_digit() || c == '.')
                    && host.split('.').count() == 4
            }
    };

    // Use a static shared HTTP client to reuse connection pool across requests.
    // Creating a new Client per call causes socket accumulation:
    //   call 1 = 30s, call 2 = 2min, call 3 = 5min → timeout
    // Static clients share the connection pool and keep-alive sockets.
    let client: &Client = if is_local_url {
        get_local_client()
    } else {
        get_cloud_client()
    };
    let tools = get_tools();

    let model_supports_tools = if model.ends_with(":free") {
        model.contains("deepseek")
            || model.contains("qwen")
            || model.contains("devstral")
            || model.contains("mimo")
            || model.contains("arcee")
            || model.contains("nemotron")
            || model.contains("hermes")
            || model.contains("llama-3.3")
            || model.contains("gpt-oss")
    } else {
        true
    };
    const RETRY_DELAY_SECS: u64 = 15;  // Wait 15s between timeouts/connection errors
    const RATE_LIMIT_DELAY_SECS: u64 = 30; // Wait 30s before retrying on 429
    let mut retry_count: u32 = 0;
    // Guard against infinite tool-call loops.
    let mut tool_turns: u32 = 0;

    loop {
        let api_messages = build_api_messages(system_prompt, &messages, model);
        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true
        });
        if model_supports_tools {
            body["tools"] = tools.clone();
        }

        // Qwen3 models default to thinking mode — all output goes into <think> blocks
        // and delta.content is empty, making the response appear blank in the UI.
        // Disable thinking for Qwen3 on local servers so output goes directly to delta.content.
        if is_local_url && model.to_lowercase().contains("qwen3") {
            body["chat_template_kwargs"] = json!({ "enable_thinking": false });
        }

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let mut request = client.post(&url).header("Content-Type", "application/json");
        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }
        // For local servers: explicitly request no compression so the SSE plain-text
        // stream is never gzip/brotli-encoded — prevents "error decoding response body".
        if is_local_url {
            request = request.header("Accept-Encoding", "identity");
        }
        if is_openrouter {
            request = request
                .header("HTTP-Referer", "https://github.com/vibeKidbright")
                .header("X-Title", "vibeKidbright IDE");
        }

        let current_timeout = if is_local_url || is_openrouter {
            match retry_count {
                0 => std::time::Duration::from_secs(120),
                1 => std::time::Duration::from_secs(300),
                _ => std::time::Duration::from_secs(600),
            }
        } else {
            std::time::Duration::from_secs(180)
        };

        let send_future = request.body(body.to_string()).send();
        let response_result = if is_local_url || is_openrouter {
            match tokio::time::timeout(current_timeout, send_future).await {
                Ok(Ok(res)) => Ok(res),
                Ok(Err(e)) => Err(format!("Connection error: {}", e)),
                Err(_) => Err("timeout".to_string()),
            }
        } else {
            send_future.await.map_err(|e| format!("Connection to {} failed: {}", url, e))
        };

        let response = match response_result {
            Ok(res) => res,
            Err(e) => {
                if is_local_url || is_openrouter {
                    let is_timeout = e == "timeout" || e.to_lowercase().contains("timeout") || e.to_lowercase().contains("timed out");
                    retry_count += 1;
                    if retry_count >= 5 {
                        if is_timeout {
                            return Err("⏱️ Connection timed out after 5 attempts. Please check your local server or OpenRouter connection.".to_string());
                        } else {
                            return Err(format!("Connection to {} failed after 5 attempts: {}", url, e));
                        }
                    }
                    let err_desc = if is_timeout { "Timed out" } else { "Connection failed" };
                    let _ = app_handle.emit(
                        "terminal-output",
                        format!("[AI] ⚠️ {} (attempt {}/5). Retrying in {}s...", err_desc, retry_count, RETRY_DELAY_SECS),
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                    continue;
                } else {
                    return Err(e);
                }
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            let provider_name = if base_url.contains("googleapis") {
                "Google AI"
            } else if is_openrouter {
                "OpenRouter"
            } else {
                "Cloud/Local"
            };
            if status.as_u16() == 429 {
                retry_count += 1;
                if retry_count >= 5 {
                    return Err(format!(
                        "❌ Model '{}' is rate-limited on {} and all 5 retries failed.",
                        model, provider_name
                    ));
                }
                // Use progressive wait: 30s for first 429, 60s for subsequent
                let wait_secs = if retry_count == 1 { RATE_LIMIT_DELAY_SECS } else { RATE_LIMIT_DELAY_SECS * 2 };
                let _ = app_handle.emit(
                    "terminal-output",
                    format!("[AI] ⚠️ Rate limited (attempt {}/5). Retrying in {}s...", retry_count, wait_secs),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                continue;
            }
            if status.as_u16() == 402 {
                return Err(format!("💳 {}: No credits remaining. Check your billing dashboard or switch to a free model.", provider_name));
            }
            if status.as_u16() == 401 {
                return Err(format!("🔑 {}: Invalid API key.", provider_name));
            }
            if status.as_u16() == 404 {
                return Err(format!("❌ Model '{}' not found on {}.", model, provider_name));
            }
            return Err(format!(
                "Server error {} from {}: {}",
                status.as_u16(),
                provider_name,
                body_text.chars().take(300).collect::<String>()
            ));
        }

        // A successful response resets the retry counter so a transient 429 earlier
        // in this conversation doesn't bleed into subsequent tool-call turns.
        retry_count = 0;

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new(); // Final answer (outside <think>)
        let mut think_text = String::new();       // Thinking content (inside <think>)
        let mut in_think_block = false;            // Track if we're inside <think>...</think>
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
        let mut buffer = String::new();

        let mut stream_failed_or_timed_out = false;
        let mut stream_error_msg = String::new();

        while let Some(chunk_result) = {
            if is_local_url {
                match tokio::time::timeout(current_timeout, stream.next()).await {
                    Ok(item) => item,
                    Err(_) => {
                        stream_failed_or_timed_out = true;
                        stream_error_msg = "timeout".to_string();
                        None
                    }
                }
            } else {
                let stream_timeout_duration = if is_openrouter { current_timeout } else { current_timeout };
                match tokio::time::timeout(stream_timeout_duration, stream.next()).await {
                    Ok(item) => item,
                    Err(_) => {
                        stream_failed_or_timed_out = true;
                        stream_error_msg = "timeout".to_string();
                        None
                    }
                }
            }
        } {
            if let Some(state) = app_handle.try_state::<AiAbortState>() {
                if state.0.load(Ordering::SeqCst) {
                    return Err("Generation stopped by user.".to_string());
                }
            }
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    // Local servers sometimes send a malformed final chunk or
                    // close the connection abruptly. If we already received text,
                    // treat this as a natural end-of-stream instead of an error.
                    let err_str = e.to_string();
                    if is_local_url && !accumulated_text.is_empty() {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] Local stream ended early (ignored): {}", err_str),
                        );
                        break;
                    }
                    stream_failed_or_timed_out = true;
                    stream_error_msg = err_str;
                    break;
                }
            };
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                let event: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let choice = &event["choices"][0];
                let delta = &choice["delta"];
                if let Some(raw_content) = delta["content"].as_str() {
                    if !raw_content.is_empty() {
                        // Filter <think>...</think> blocks in real-time.
                        // Qwen3/DeepSeek send reasoning inside these tags;
                        // only the content OUTSIDE them is the actual answer.
                        let mut remaining = raw_content;
                        while !remaining.is_empty() {
                            if in_think_block {
                                if let Some(end_pos) = remaining.find("</think>") {
                                    // Found end of think block
                                    think_text.push_str(&remaining[..end_pos]);
                                    remaining = &remaining[end_pos + 8..]; // skip </think>
                                    in_think_block = false;
                                } else {
                                    // Still inside think block, buffer it
                                    think_text.push_str(remaining);
                                    remaining = "";
                                }
                            } else {
                                if let Some(start_pos) = remaining.find("<think>") {
                                    // Emit content before <think>
                                    let before = &remaining[..start_pos];
                                    if !before.is_empty() {
                                        accumulated_text.push_str(before);
                                        let _ = app_handle.emit("ai-chat-delta", before.to_string());
                                    }
                                    remaining = &remaining[start_pos + 7..]; // skip <think>
                                    in_think_block = true;
                                } else {
                                    // No think tags — emit as normal
                                    accumulated_text.push_str(remaining);
                                    let _ = app_handle.emit("ai-chat-delta", remaining.to_string());
                                    remaining = "";
                                }
                            }
                        }
                    }
                }
                // Qwen3 / DeepSeek-R1 via some servers: reasoning in separate field
                if delta["content"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
                    if let Some(reasoning) = delta["reasoning_content"].as_str() {
                        if !reasoning.is_empty() {
                            think_text.push_str(reasoning);
                        }
                    }
                }
                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                    for tc in tool_calls {
                        let index = tc["index"].as_u64().unwrap_or(0) as usize;
                        while pending_tool_calls.len() <= index {
                            pending_tool_calls.push(PendingToolCall {
                                id: String::new(), name: String::new(), arguments: String::new(),
                                thought_signature: None,
                            });
                        }
                        if let Some(id) = tc["id"].as_str() {
                            pending_tool_calls[index].id = id.to_string();
                        }
                        if let Some(func) = tc["function"].as_object() {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                pending_tool_calls[index].name = name.to_string();
                                let _ = app_handle.emit(
                                    "ai-chat-tool-start",
                                    json!({ "name": name, "id": &pending_tool_calls[index].id }).to_string(),
                                );
                            }
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                pending_tool_calls[index].arguments.push_str(args);
                            }
                        }
                    }
                }
                // Legacy function_call delta (some local servers)
                if let Some(function_call) = delta["function_call"].as_object() {
                    if pending_tool_calls.is_empty() {
                        pending_tool_calls.push(PendingToolCall {
                            id: "call_0".to_string(), name: String::new(), arguments: String::new(),
                            thought_signature: None,
                        });
                    }
                    if let Some(name) = function_call.get("name").and_then(|v| v.as_str()) {
                        pending_tool_calls[0].name = name.to_string();
                        let _ = app_handle.emit(
                            "ai-chat-tool-start",
                            json!({ "name": name, "id": &pending_tool_calls[0].id }).to_string(),
                        );
                    }
                    if let Some(args) = function_call.get("arguments").and_then(|v| v.as_str()) {
                        pending_tool_calls[0].arguments.push_str(args);
                    }
                }
            }
        }

        if stream_failed_or_timed_out {
            if (is_local_url || is_openrouter) && accumulated_text.is_empty() && think_text.is_empty() {
                let is_timeout = stream_error_msg == "timeout" || stream_error_msg.to_lowercase().contains("timeout") || stream_error_msg.to_lowercase().contains("timed out");
                retry_count += 1;
                if retry_count < 3 {
                    let err_desc = if is_timeout { "Timed out" } else { "Connection failed" };
                    let _ = app_handle.emit(
                        "terminal-output",
                        format!("[AI] ⚠️ Stream {} (attempt {}/3). Retrying in {}s...", err_desc, retry_count, RETRY_DELAY_SECS),
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                    continue;
                } else {
                    if is_timeout {
                        return Err("⏱️ Connection timed out. Please check your local server or OpenRouter connection.".to_string());
                    } else {
                        return Err(format!("Stream failed: {}", stream_error_msg));
                    }
                }
            } else {
                if stream_error_msg == "timeout" {
                    return Err("⏱️ Stream timeout: The AI server stopped responding. Please retry.".to_string());
                } else {
                    return Err(format!("Stream error: {}", stream_error_msg));
                }
            }
        }

        if !pending_tool_calls.is_empty() {
            // FIX: Max tool-turn guard.
            tool_turns += 1;
            if tool_turns > MAX_TOOL_TURNS {
                let _ = app_handle.emit(
                    "ai-chat-error",
                    format!("⚠️ Stopped after {} tool-call turns to prevent an infinite loop. Please rephrase your request.", MAX_TOOL_TURNS),
                );
                break;
            }

            let tool_calls_json: Vec<Value> = pending_tool_calls.iter().map(|tc| json!({
                "id": tc.id, "type": "function",
                "function": { "name": tc.name, "arguments": tc.arguments }
            })).collect();

            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: json!({
                    "__tool_calls__": tool_calls_json,
                    "__text__": if accumulated_text.is_empty() { Value::Null } else { json!(accumulated_text) }
                }),
            });

            for tc in &pending_tool_calls {
                // FIX: parse errors are now logged, not silently dropped.
                let input: Value = match serde_json::from_str(&tc.arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] ⚠️ Failed to parse args for tool '{}': {} — args: {}", tc.name, e, tc.arguments),
                        );
                        json!({})
                    }
                };
                // FIX: pass &message_id — Arc<str> derefs to &str cleanly.
                let result = execute_tool(app_handle, &tc.name, &input, project_path, &message_id, no_workspace).await;
                let _ = app_handle.emit(
                    "ai-chat-tool-result",
                    json!({ "name": tc.name, "id": tc.id, "result": &result }).to_string(),
                );
                let result_str = if result.is_string() {
                    result.as_str().unwrap().to_string()
                } else {
                    result.to_string()
                };
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: json!({
                        "__tool_response__": true,
                        "tool_call_id": tc.id,
                        "__func_name__": tc.name,
                        "content": result_str
                    }),
                });
            }
            pending_tool_calls.clear();
        } else {
            // Fallback: if the model ONLY output thinking (no final answer after </think>),
            // show the thinking content so the UI isn't blank.
            if accumulated_text.is_empty() && !think_text.is_empty() {
                let _ = app_handle.emit("ai-chat-delta", think_text.clone());
                accumulated_text = think_text.clone();
            }
            if !accumulated_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: json!(accumulated_text),
                });
            }
            let _ = app_handle.emit("ai-chat-done", json!({
                "text": &accumulated_text,
                "history": &messages
            }).to_string());
            break;
        }
    }

    Ok(())
}

// ── Google Generative AI conversation loop ────────────────────────────────────

fn build_google_contents(messages: &[ChatMessage]) -> Vec<Value> {
    let mut contents: Vec<Value> = Vec::new();
    for m in messages {
        let role = match m.role.as_str() {
            "assistant" => "model",
            _ => "user",
        };
        if let Some(obj) = m.content.as_object() {
            if obj.contains_key("__tool_calls__") {
                let tool_calls = obj["__tool_calls__"].as_array().cloned().unwrap_or_default();
                let mut parts: Vec<Value> = Vec::new();
                if let Some(text) = obj.get("__text__").and_then(|t| t.as_str()) {
                    if !text.is_empty() { parts.push(json!({ "text": text })); }
                }
                for tc in &tool_calls {
                    let func_name = tc["function"]["name"].as_str().unwrap_or("");
                    // FIX: log parse errors instead of silently using json!({}).
                    let args: Value = tc["function"]["arguments"]
                        .as_str()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_else(|| {
                            eprintln!("[AI] Could not parse Google tool args for '{}'", func_name);
                            json!({})
                        });
                    // Echo back thought_signature at the PART level — NOT inside functionCall.
                    // Google docs: "return this signature in the exact part where it was received"
                    // Gemini 3.x places it at: part["thoughtSignature"]  (part level)
                    // Gemini 2.5 places it at: part["functionCall"]["thoughtSignature"]  (inside fc)
                    // Sending it inside functionCall causes: "Unknown name thoughtSignature at function_call"
                    let fc_obj = json!({ "name": func_name, "args": args });
                    let mut part = json!({ "functionCall": fc_obj });
                    if let Some(sig) = tc["function"]["thought_signature"].as_str() {
                        part["thoughtSignature"] = json!(sig);  // ← part level (correct)
                    }
                    parts.push(part);
                }
                if !parts.is_empty() {
                    contents.push(json!({ "role": "model", "parts": parts }));
                }
                continue;
            }
            if obj.contains_key("__tool_response__") {
                let func_name = obj.get("__func_name__")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| obj["tool_call_id"].as_str().unwrap_or(""));
                let content_str = obj["content"].as_str().unwrap_or("");
                let response_value: Value = serde_json::from_str(content_str)
                    .unwrap_or_else(|_| json!({ "result": content_str }));
                let response_obj = if response_value.is_object() {
                    response_value
                } else {
                    json!({ "result": response_value })
                };
                contents.push(json!({
                    "role": "user",
                    "parts": [{ "functionResponse": { "name": func_name, "response": response_obj } }]
                }));
                continue;
            }
        }
        let text = m.content.as_str().unwrap_or("");
        if !text.is_empty() {
            contents.push(json!({ "role": role, "parts": [{ "text": text }] }));
        }
    }
    contents
}

fn get_google_tools() -> Value {
    json!([{
        "functionDeclarations": [
            { "name": "read_file", "description": "Read file content. Always use this before editing a file.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING", "description": "Path relative to project root" } }, "required": ["path"] } },
            { "name": "write_file", "description": "CRITICAL: You MUST use this tool to write or modify any code. DO NOT put code blocks in your chat response. Overwrites existing content.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING", "description": "Path relative to project root" }, "content": { "type": "STRING", "description": "The FULL complete new file content. DO NOT truncate or omit code." } }, "required": ["path", "content"] } },
            { "name": "list_files", "description": "List files in a directory.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING" } }, "required": ["path"] } },
            { "name": "get_file_tree", "description": "Recursively list all files in the project as an indented tree.", "parameters": { "type": "OBJECT", "properties": { "max_depth": { "type": "INTEGER", "description": "Max depth (default 4)" } } } },
            { "name": "search_in_files", "description": "Search for a regex pattern across all source files. Returns matches with file path and line number.", "parameters": { "type": "OBJECT", "properties": { "pattern": { "type": "STRING" }, "file_extension": { "type": "STRING" } }, "required": ["pattern"] } },
            { "name": "diff_file", "description": "Preview a unified diff between the current file and proposed new content, without writing.", "parameters": { "type": "OBJECT", "properties": { "path": { "type": "STRING" }, "new_content": { "type": "STRING" } }, "required": ["path", "new_content"] } },
            { "name": "run_command", "description": "Run a shell command.", "parameters": { "type": "OBJECT", "properties": { "command": { "type": "STRING" } }, "required": ["command"] } },
            { "name": "esp_idf_info", "description": "Get resolved ESP-IDF paths.", "parameters": { "type": "OBJECT", "properties": {} } },
            // FIX M2: install_idf_library was missing from Google tools — now parity with OpenAI tools.
            { "name": "install_idf_library", "description": "Install an ESP-IDF component dependency using idf.py add-dependency.", "parameters": { "type": "OBJECT", "properties": { "component": { "type": "STRING", "description": "Component identifier, e.g. espressif/led_strip or espressif/led_strip^2.5.3" } }, "required": ["component"] } },
            { "name": "web_search", "description": "Search the internet for technical docs.", "parameters": { "type": "OBJECT", "properties": { "query": { "type": "STRING" } }, "required": ["query"] } },
            { "name": "knowledge_search", "description": "Search the local knowledge_base folder.", "parameters": { "type": "OBJECT", "properties": { "query": { "type": "STRING" } }, "required": ["query"] } },
            { "name": "read_knowledge_file", "description": "Read the FULL content of a file from the knowledge_base folder. Use this when knowledge_search returns partial/truncated results or says the KB is not loaded.", "parameters": { "type": "OBJECT", "properties": { "file_name": { "type": "STRING", "description": "File name relative to knowledge_base/, e.g. 'formula_kid_controller.md'" } }, "required": ["file_name"] } },
            { "name": "create_project_workspace", "description": "Create a new project workspace directory. Call FIRST when no workspace is open.", "parameters": { "type": "OBJECT", "properties": { "project_name": { "type": "STRING" } }, "required": ["project_name"] } }
        ]
    }])
}

/// Strip large embedded C code blocks from the system prompt for Gemini 3.x thinking models.
/// Rules, GPIO tables, and short examples (<= MAX_CODE_LINES) are preserved.
/// Long code templates are replaced with a one-line hint so the model calls knowledge_search.
/// Result: ~50% fewer system-prompt tokens while maintaining all behavioral constraints.
fn compact_prompt_for_thinking_model(prompt: &str) -> String {
    const MAX_CODE_LINES: usize = 8; // keep examples this short or shorter
    let mut result = String::with_capacity(prompt.len() / 2);
    let mut in_code = false;
    let mut fence_marker = String::new();
    let mut code_buf: Vec<&str> = Vec::new();

    for line in prompt.lines() {
        let trimmed = line.trim();
        if !in_code && (trimmed.starts_with("```")) {
            // Start of a code block — buffer it
            in_code = true;
            fence_marker = trimmed[..3].to_string();
            code_buf.clear();
            code_buf.push(line);
        } else if in_code && trimmed.starts_with(&fence_marker) && trimmed.len() == fence_marker.len() {
            // End of code block
            code_buf.push(line);
            let body_lines = code_buf.len().saturating_sub(2); // exclude ``` markers
            if body_lines <= MAX_CODE_LINES {
                // Short example → keep as-is
                for l in &code_buf { result.push_str(l); result.push('\n'); }
            } else {
                // Long example → replace with a reminder to use KB tools
                result.push_str(&format!(
                    "[Code example omitted ({} lines). Use knowledge_search or read_knowledge_file to retrieve the actual implementation.]\n",
                    body_lines
                ));
            }
            in_code = false;
            code_buf.clear();
        } else if in_code {
            code_buf.push(line);
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

pub(crate) async fn run_google_conversation_loop(
    app_handle: &AppHandle,
    api_key: &str,
    model: &str,
    mut messages: Vec<ChatMessage>,
    project_path: &mut PathBuf,
    // FIX: Arc<str> instead of &str.
    message_id: Arc<str>,
    no_workspace: &mut bool,
    system_prompt: &str,
) -> Result<(), String> {
    // FIX H5: Use shared static HTTP client (get_cloud_client) instead of creating
    // a new Client per invocation. Creating a new Client per request causes socket
    // accumulation and progressively slower responses (same issue fixed in OpenAI loop).
    let client = get_cloud_client();
    let google_tools = get_google_tools();

    // For Gemini 3.x thinking models: strip large embedded code examples from
    // the system prompt. The model's thinking + knowledge_search replaces them.
    // This saves ~4000-6000 tokens per request while keeping all behavioral rules.
    let compacted;
    let effective_prompt = if model.starts_with("gemini-3") {
        compacted = compact_prompt_for_thinking_model(system_prompt);
        &compacted
    } else {
        system_prompt
    };

    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_SECS: u64 = 4;
    let mut retry_count: u32 = 0;
    // FIX: Guard against infinite tool-call loops.
    let mut tool_turns: u32 = 0;

    loop {
        let contents = build_google_contents(&messages);

        // Set thinkingBudget for Gemini 3.x thinking models.
        // Gemini 3.x always thinks internally — unlimited budget burns quota fast.
        // 2048 tokens = enough for tool routing + code logic decisions, without overrun.
        // Set to 0 to completely disable thinking (faster, fewer tokens, but less smart).
        let gen_config = if model.starts_with("gemini-3") {
            json!({
                "temperature": 0.7,
                "thinkingConfig": { "thinkingBudget": 2048 }
            })
        } else {
            json!({ "temperature": 0.7 })
        };

        let body = json!({
            "systemInstruction": { "parts": [{ "text": effective_prompt }] },
            "contents": contents,
            "tools": google_tools,
            "generationConfig": gen_config
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, api_key
        );

        let response = client.post(&url)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("Connection to Google AI failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                retry_count += 1;
                if retry_count > MAX_RETRIES {
                    return Err(format!("❌ Google AI model '{}' rate-limited. All {} retries failed.", model, MAX_RETRIES));
                }
                let _ = app_handle.emit(
                    "terminal-output",
                    format!("[AI] ⚠️ Rate limited (attempt {}/{}). Retrying in {}s...", retry_count, MAX_RETRIES, RETRY_DELAY_SECS),
                );
                tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                continue;
            }
            if status.as_u16() == 400 {
                return Err(format!("❌ Google AI 400 Bad Request: {}", body_text.chars().take(400).collect::<String>()));
            }
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err("🔑 Google AI: Invalid or unauthorized API key.".to_string());
            }
            if status.as_u16() == 404 {
                return Err(format!("❌ Google AI model '{}' not found.", model));
            }
            return Err(format!("Google AI error {}: {}", status.as_u16(), body_text.chars().take(300).collect::<String>()));
        }

        // Reset retry counter on a successful response (same logic as OpenAI loop).
        retry_count = 0;

        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new();
        let mut buffer = String::new();
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();

        while let Some(chunk_result) = {
            match tokio::time::timeout(std::time::Duration::from_secs(90), stream.next()).await {
                Ok(item) => item,
                Err(_) => {
                    return Err("⏱️ Stream timeout: The AI server stopped responding for 90 seconds. Please retry.".to_string());
                }
            }
        } {
            if let Some(state) = app_handle.try_state::<AiAbortState>() {
                if state.0.load(Ordering::SeqCst) {
                    return Err("Generation stopped by user.".to_string());
                }
            }
            let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();
                if !line.starts_with("data: ") { continue; }
                let data = &line[6..];
                if data == "[DONE]" { continue; }
                let event: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(candidates) = event["candidates"].as_array() {
                    for candidate in candidates {
                        if let Some(parts) = candidate["content"]["parts"].as_array() {
                            for part in parts {
                                if let Some(text) = part["text"].as_str() {
                                    accumulated_text.push_str(text);
                                    let _ = app_handle.emit("ai-chat-delta", text.to_string());
                                }
                                if let Some(fc) = part["functionCall"].as_object() {
                                    let name = fc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let args = fc.get("args").cloned().unwrap_or(json!({}));
                                    // Capture thought_signature — MANDATORY for Gemini 3.x, optional for 2.5.
                                    // Gemini 3.x: signature is at the PART level (part["thoughtSignature"])
                                    // Gemini 2.5: signature may be inside functionCall (fc["thoughtSignature"])
                                    // We check both locations for maximum compatibility.
                                    let thought_signature = part["thoughtSignature"]
                                        .as_str()
                                        .or_else(|| fc.get("thoughtSignature").and_then(|v| v.as_str()))
                                        .map(|s| s.to_string());
                                    let id = format!("call_{}", pending_tool_calls.len());
                                    let _ = app_handle.emit(
                                        "ai-chat-tool-start",
                                        json!({ "name": &name, "id": &id }).to_string(),
                                    );
                                    pending_tool_calls.push(PendingToolCall {
                                        id, name, arguments: args.to_string(), thought_signature,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if !pending_tool_calls.is_empty() {
            // FIX: Max tool-turn guard.
            tool_turns += 1;
            if tool_turns > MAX_TOOL_TURNS {
                let _ = app_handle.emit(
                    "ai-chat-error",
                    format!("⚠️ Stopped after {} tool-call turns to prevent an infinite loop.", MAX_TOOL_TURNS),
                );
                break;
            }

            let tool_calls_json: Vec<Value> = pending_tool_calls.iter().map(|tc| {
                let mut func = json!({ "name": tc.name, "arguments": tc.arguments });
                // Preserve thought_signature so build_google_contents can echo it back.
                if let Some(sig) = &tc.thought_signature {
                    func["thought_signature"] = json!(sig);
                }
                json!({ "id": tc.id, "type": "function", "function": func })
            }).collect();

            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: json!({
                    "__tool_calls__": tool_calls_json,
                    "__text__": if accumulated_text.is_empty() { Value::Null } else { json!(accumulated_text) }
                }),
            });

            for tc in &pending_tool_calls {
                // FIX: log parse errors.
                let input: Value = match serde_json::from_str(&tc.arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = app_handle.emit(
                            "terminal-output",
                            format!("[AI] ⚠️ Failed to parse Google tool args for '{}': {}", tc.name, e),
                        );
                        json!({})
                    }
                };
                // FIX: &message_id derefs Arc<str> to &str cleanly.
                let result = execute_tool(app_handle, &tc.name, &input, project_path, &message_id, no_workspace).await;
                let _ = app_handle.emit(
                    "ai-chat-tool-result",
                    json!({ "name": tc.name, "id": tc.id, "result": &result }).to_string(),
                );
                let result_str = if result.is_string() {
                    result.as_str().unwrap().to_string()
                } else {
                    result.to_string()
                };
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: json!({
                        "__tool_response__": true,
                        "tool_call_id": tc.id,   // FIX: was tc.name — must be the call ID, not the function name
                        "__func_name__": tc.name,
                        "content": result_str
                    }),
                });
            }
            pending_tool_calls.clear();
        } else {
            if !accumulated_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: json!(accumulated_text),
                });
            }
            let _ = app_handle.emit("ai-chat-done", json!({
                "text": &accumulated_text,
                "history": &messages
            }).to_string());
            break;
        }
    }

    Ok(())
}

// ── API message builder ────────────────────────────────────────────────────────

fn build_api_messages(system_prompt: &str, messages: &[ChatMessage], model: &str) -> Vec<Value> {
    let mut api_msgs: Vec<Value> = Vec::new();
    let unsupported_system_role =
        model.to_lowercase().contains("gemma") || model.to_lowercase().contains("o1-");
    if !unsupported_system_role {
        api_msgs.push(json!({ "role": "system", "content": system_prompt }));
    }
    let mut first_user_found = false;
    for m in messages {
        if let Some(obj) = m.content.as_object() {
            if obj.contains_key("__tool_calls__") {
                let mut msg = json!({ "role": "assistant" });
                // OpenAI rejects `content: null` when tool_calls are present — use "".
                msg["content"] = obj.get("__text__").and_then(|t| t.as_str())
                    .map(|t| json!(t))
                    .unwrap_or(json!(""));
                msg["tool_calls"] = obj["__tool_calls__"].clone();
                api_msgs.push(msg);
            } else if obj.contains_key("__tool_response__") {
                api_msgs.push(json!({
                    "role": "tool",
                    "tool_call_id": obj["tool_call_id"],
                    "content": obj["content"]
                }));
            } else {
                let mut content = m.content.clone();
                // Inject system prompt into first user message for models that
                // don't accept a "system" role (e.g. Gemma, o1-*).
                // Only applies to plain-text content; multimodal content is passed through.
                if unsupported_system_role && !first_user_found && m.role == "user" {
                    first_user_found = true;
                    if let Some(text) = content.as_str() {
                        content = json!(format!(
                            "[SYSTEM INSTRUCTIONS]\n{}\n\n[USER INPUT]\n{}", system_prompt, text
                        ));
                    }
                    // If content is an array (multimodal), prepend a text part.
                    else if let Some(arr) = content.as_array() {
                        let mut new_arr = vec![json!({ "type": "text", "text": format!("[SYSTEM INSTRUCTIONS]\n{}\n\n", system_prompt) })];
                        new_arr.extend(arr.iter().cloned());
                        content = json!(new_arr);
                    }
                }
                api_msgs.push(json!({ "role": m.role.clone(), "content": content }));
            }
        } else {
            let mut content = m.content.clone();
            if unsupported_system_role && !first_user_found && m.role == "user" {
                first_user_found = true;
                if let Some(text) = content.as_str() {
                    content = json!(format!(
                        "[SYSTEM INSTRUCTIONS]\n{}\n\n[USER INPUT]\n{}", system_prompt, text
                    ));
                }
            }
            api_msgs.push(json!({ "role": m.role.clone(), "content": content }));
        }
    }
    api_msgs
}
