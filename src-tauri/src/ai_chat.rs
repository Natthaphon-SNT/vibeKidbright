use futures::StreamExt;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

// ── Data types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value, // String or null (for tool call messages)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeChunk {
    file_name: String,
    content: String,
    embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct VectorIndex {
    chunks: Vec<KnowledgeChunk>,
    last_indexed: std::collections::HashMap<String, u64>, // FileName -> Modification Time
}

#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

// ── Config helpers ───────────────────────────────────────────────────────────

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vibekidbright")
}

fn read_config() -> Value {
    let path = config_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or(json!({}))
    } else {
        json!({})
    }
}

fn write_config(config: &Value) {
    let dir = config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(
        config_path(),
        serde_json::to_string_pretty(config).unwrap_or_default(),
    );
}

fn normalize_project_dir(project_dir: &str) -> String {
    project_dir
        .trim_start_matches("file://")
        .trim()
        .to_string()
}

fn resolve_project_root(project_dir: &str) -> PathBuf {
    let normalized = normalize_project_dir(project_dir);
    if !normalized.is_empty() && normalized != "." {
        return PathBuf::from(normalized);
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("knowledge_base").exists() {
        return cwd;
    }

    if let Some(parent) = cwd.parent() {
        let parent_path = parent.to_path_buf();
        if parent_path.join("knowledge_base").exists() {
            return parent_path;
        }
    }

    if cwd.file_name().is_some_and(|n| n == "src-tauri") {
        if let Some(parent) = cwd.parent() {
            return parent.to_path_buf();
        }
    }

    cwd
}

fn resolve_kb_path(project_dir: &str) -> PathBuf {
    resolve_project_root(project_dir).join("knowledge_base")
}

fn resolve_idf_paths_for_ai(app_handle: &AppHandle) -> Option<(PathBuf, PathBuf)> {
    if let (Some(idf), Some(tools)) = (
        std::env::var_os("VIBEKIDBRIGHT_IDF_PATH"),
        std::env::var_os("VIBEKIDBRIGHT_TOOLS_PATH"),
    ) {
        let idf_path = PathBuf::from(idf);
        let tools_path = PathBuf::from(tools);
        if idf_path.join("tools/idf.py").exists() && tools_path.exists() {
            return Some((idf_path, tools_path));
        }
    }

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let runtime_root = app_data_dir.join("esp-idf-runtime");
        let tools_path = runtime_root.join(".espressif");
        if tools_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&runtime_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("esp-idf-"))
                        && path.join("tools/idf.py").exists()
                    {
                        return Some((path, tools_path.clone()));
                    }
                }
            }
        }
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let idf_path = resource_dir.join("esp-idf");
        let tools_path = resource_dir.join(".espressif");
        if idf_path.join("tools/idf.py").exists() && tools_path.exists() {
            return Some((idf_path, tools_path));
        }
    }

    let dev_idf = PathBuf::from("../resources/esp-idf");
    let dev_tools = PathBuf::from("../resources/.espressif");
    if dev_idf.join("tools/idf.py").exists() && dev_tools.exists() {
        return Some((dev_idf, dev_tools));
    }

    None
}

fn find_idf_python_bin(tools_path: &Path) -> Option<PathBuf> {
    let python_env_dir = tools_path.join("python_env");
    let entries = std::fs::read_dir(&python_env_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with("idf") && name.contains("_py") && name.ends_with("_env")) {
            continue;
        }
        let venv = entry.path();
        let candidates = if cfg!(target_os = "windows") {
            vec![venv.join("Scripts/python.exe")]
        } else {
            vec![venv.join("bin/python"), venv.join("bin/python3")]
        };
        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn build_ai_idf_path(tools_path: &Path) -> OsString {
    let mut paths: Vec<PathBuf> = Vec::new();
    let tools_dir = tools_path.join("tools");
    if let Ok(entries) = std::fs::read_dir(&tools_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            if let Ok(versions) = std::fs::read_dir(entry.path()) {
                for ver in versions.flatten() {
                    let bin = ver.path().join("bin");
                    if bin.exists() {
                        paths.push(bin);
                    }
                    let elf_bin = ver.path().join(entry.file_name().to_string_lossy().to_string());
                    let tool_bin = elf_bin.join("bin");
                    if tool_bin.exists() {
                        paths.push(tool_bin);
                    }
                }
            }
        }
    }

    let python_env_dir = tools_path.join("python_env");
    if let Ok(entries) = std::fs::read_dir(&python_env_dir) {
        for entry in entries.flatten() {
            let bin = if cfg!(target_os = "windows") {
                entry.path().join("Scripts")
            } else {
                entry.path().join("bin")
            };
            if bin.exists() {
                paths.push(bin);
            }
        }
    }

    if let Some(system_path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&system_path));
    }

    std::env::join_paths(paths).unwrap_or_else(|_| OsString::from(""))
}

// ── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_api_key() -> Result<String, String> {
    let config = read_config();
    Ok(config["api_key"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn set_api_key(key: String) -> Result<(), String> {
    let mut config = read_config();
    config["api_key"] = json!(key);
    write_config(&config);
    Ok(())
}

#[tauri::command]
pub async fn get_model() -> Result<String, String> {
    let config = read_config();
    Ok(config["model"].as_str().unwrap_or("gpt-4o").to_string())
}

#[tauri::command]
pub async fn set_model(model: String) -> Result<(), String> {
    let mut config = read_config();
    config["model"] = json!(model);
    write_config(&config);
    Ok(())
}

#[tauri::command]
pub async fn get_base_url() -> Result<String, String> {
    let config = read_config();
    Ok(config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string())
}

#[tauri::command]
pub async fn set_base_url(url: String) -> Result<(), String> {
    let mut config = read_config();
    config["base_url"] = json!(url);
    write_config(&config);
    Ok(())
}

#[tauri::command]
pub async fn get_provider() -> Result<String, String> {
    let config = read_config();
    Ok(config["provider"].as_str().unwrap_or("openai").to_string())
}

#[tauri::command]
pub async fn set_provider(provider: String) -> Result<(), String> {
    let mut config = read_config();
    config["provider"] = json!(provider);
    write_config(&config);
    Ok(())
}

#[tauri::command]
pub async fn get_search_api_key() -> Result<String, String> {
    let config = read_config();
    Ok(config["search_api_key"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn set_search_api_key(key: String) -> Result<(), String> {
    let mut config = read_config();
    config["search_api_key"] = json!(key);
    write_config(&config);
    Ok(())
}

#[tauri::command]
pub async fn refresh_knowledge_base(project_dir: String) -> Result<usize, String> {
    let project_path = resolve_project_root(&project_dir);
    reindex_knowledge_base(&project_path).await
}

#[tauri::command]
pub fn get_knowledge_base_files(project_dir: String) -> Vec<String> {
    let project_path = resolve_project_root(&project_dir);
    println!("DEBUG: Listing KB files for project_dir: {}", project_path.display());
    let kb_path = project_path.join("knowledge_base");
    
    if !kb_path.exists() {
        println!("DEBUG: KB path does not exist: {:?}", kb_path);
        return Vec::new();
    }

    let mut files = Vec::new();
    println!("Knowledge Search - Project Dir: {}", project_dir);
    println!("Knowledge Search - KB Path: {:?}", kb_path);
    if let Ok(entries) = std::fs::read_dir(&kb_path) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                println!("Found file: {}", name);
                if name.ends_with(".txt") || name.ends_with(".md") {
                    files.push(name);
                }
            }
        }
    } else {
        println!("Failed to read KB dir: {:?}", kb_path);
    }
    files
}

#[tauri::command]
pub fn open_knowledge_base_folder(project_dir: String) {
    let kb_path = resolve_kb_path(&project_dir);
    if !kb_path.exists() {
        let _ = std::fs::create_dir_all(&kb_path);
    }
    let _ = tauri_plugin_opener::open_path(kb_path.to_string_lossy().to_string(), None::<String>);
}

#[tauri::command]
pub async fn send_ai_message(
    app_handle: AppHandle,
    messages: Vec<ChatMessage>,
    project_dir: String,
) -> Result<(), String> {
    let (api_key, raw_model, mut base_url) = {
        let config = read_config();
        (
            config["api_key"].as_str().unwrap_or("").to_string(),
            config["model"].as_str().unwrap_or("gpt-4o").to_string(),
            config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string()
        )
    };

    // Ensure protocol is present
    if !base_url.starts_with("http") && !base_url.is_empty() {
        base_url = format!("http://{}", base_url);
    }

    // Auto-fix for local servers (LM Studio, Ollama, etc.)
    // If it's a local address and ends with a port (no path), append /v1
    if (base_url.contains("localhost") || base_url.contains("127.0.0.1")) 
       && !base_url.contains("/v1") 
       && !base_url.ends_with("/v1") 
    {
        base_url = format!("{}/v1", base_url.trim_end_matches('/'));
    }

    // Sanitize model ID for OpenAI (strip OpenRouter prefixes)
    let model = raw_model.replace("openai/", "");

    if api_key.is_empty() && !base_url.contains("localhost") && !base_url.contains("127.0.0.1") {
        return Err("API key not set. Please configure your OpenAI API key.".to_string());
    }

    let project_path = resolve_project_root(&project_dir);

    tokio::spawn(async move {
        let _ = app_handle.emit("terminal-output", format!("[AI] Calling {} (Model: {})", base_url, model));
        if let Err(e) =
            run_conversation_loop(&app_handle, &api_key, &model, &base_url, messages, &project_path).await
        {
            let err_msg = e.to_string();
            let _ = app_handle.emit("ai-chat-error", err_msg.clone());
            let _ = app_handle.emit("terminal-output", format!("[AI ERROR] {}", err_msg));
        }
    });

    Ok(())
}

// ── Conversation loop with tool use ──────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are an expert ESP-IDF coding assistant. You help users build firmware for ESP32 and KidBright boards.

YOU HAVE TWO WAYS TO HELP:
1. EXPLAIN & GENERATE: Provide clear explanations and code blocks.
2. DIRECT ACTION: Use tools to read/write files and run commands.

### TOOL USE RULES:
- You MUST use tools to see the project state before making changes.
- To edit code, use `read_file` first, then `write_file` with the COMPLETE updated content.
- When calling a tool, do not explain what you are doing first. Just call the tool.
- If the user asks to "fix" or "add" something, always use `write_file` to apply the change.
- NEVER use `esptool.py` directly. Always use `idf.py` commands.
- After code changes are complete, program the board with `idf.py build flash` (do not run monitor unless the user asks).
- When new ESP-IDF components are required, install them with `install_idf_library` before build/flash.

### AUTONOMY & RESEARCH:
- DO NOT say "I don't know" or "I couldn't find info" without using `web_search` first.
- Be PROACTIVE: If you are unsure about the latest ESP-IDF v5.x APIs, component details, or hardware pinouts, SEARCH the web immediately.
- If the user mentions a specific library or version you aren't 100% sure about, SEARCH for its documentation.
- Check the local **Knowledge Base** (`knowledge_search`) for project-specific instructions, coding standards, or hardware mappings before searching the web.

### FORMATTING:
- Responses should be in Markdown.
- Use standard tool call blocks (function calling).

ENVIRONMENT:
- Framework: ESP-IDF
- Build Tools: idf.py, cmake, ninja
- Board: ESP32 / KidBright

IMPORTANT ESP-IDF CONTEXT:
- The app already resolves ESP-IDF paths internally.
- When you need ESP-IDF, use the run_command tool with commands like `idf.py build`, `idf.py flash`, `idf.py set-target esp32`.
- Do NOT ask the user to install ESP-IDF again unless the tool result explicitly says ESP-IDF is missing."#;

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
                        "path": {
                            "type": "string",
                            "description": "Path relative to project root"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write text to a file. Overwrites existing content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path relative to project root"
                        },
                        "content": {
                            "type": "string",
                            "description": "The new file content"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in a directory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to list (use '.' for root)"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Run a shell command (e.g. 'idf.py build').",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command string"
                        }
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
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "install_idf_library",
                "description": "Install an ESP-IDF component dependency for this project using idf.py add-dependency.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "component": {
                            "type": "string",
                            "description": "Component identifier, e.g. espressif/led_strip or espressif/led_strip^2.5.3"
                        }
                    },
                    "required": ["component"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "web_search",
                "description": "CRITICAL: Use this to search the internet for latest technical documentation, ESP-IDF API changes, hardware specs, or code examples when your internal knowledge is insufficient or potentially outdated.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The technical search query"
                        }
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
                        "query": {
                            "type": "string",
                            "description": "The search query or keywords"
                        }
                    },
                    "required": ["query"]
                }
            }
        }
    ])
}

async fn run_conversation_loop(
    app_handle: &AppHandle,
    api_key: &str,
    model: &str,
    base_url: &str,
    mut messages: Vec<ChatMessage>,
    project_path: &Path,
) -> Result<(), String> {
    let client = Client::new();
    let tools = get_tools();

    loop {
        // Build API messages using helper that handles tool call format
        let api_messages = build_api_messages(SYSTEM_PROMPT, &messages);

        let body = json!({
            "model": model,
            "messages": api_messages,
            "tools": tools,
            "stream": true
        });

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let mut request = client.post(&url)
            .header("Content-Type", "application/json");

        if !api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| format!("Connection to {} failed: {}", url, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(format!("Server returned error {} for {}: {}", status, url, body_text));
        }

        // Parse SSE stream (OpenAI format)
        let mut stream = response.bytes_stream();
        let mut accumulated_text = String::new();
        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            buffer.push_str(&chunk_str);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                if data == "[DONE]" {
                    continue;
                }

                let event: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Process choices[0]
                let choice = &event["choices"][0];
                let delta = &choice["delta"];

                // Text content delta
                if let Some(content) = delta["content"].as_str() {
                    accumulated_text.push_str(content);
                    let _ = app_handle.emit("ai-chat-delta", content.to_string());
                }

                // Tool calls delta (OpenAI sends incremental chunks)
                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                    for tc in tool_calls {
                        let index = tc["index"].as_u64().unwrap_or(0) as usize;

                        // Ensure we have enough slots
                        while pending_tool_calls.len() <= index {
                            pending_tool_calls.push(PendingToolCall {
                                id: String::new(),
                                name: String::new(),
                                arguments: String::new(),
                            });
                        }

                        // Accumulate fields
                        if let Some(id) = tc["id"].as_str() {
                            pending_tool_calls[index].id = id.to_string();
                        }
                        if let Some(func) = tc["function"].as_object() {
                            if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                pending_tool_calls[index].name = name.to_string();
                                // Notify frontend
                                let _ = app_handle.emit(
                                    "ai-chat-tool-start",
                                    json!({ "name": name, "id": &pending_tool_calls[index].id })
                                        .to_string(),
                                );
                            }
                            if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                pending_tool_calls[index].arguments.push_str(args);
                            }
                        }
                    }
                }

                // Legacy function_call delta compatibility (some local servers/models)
                if let Some(function_call) = delta["function_call"].as_object() {
                    if pending_tool_calls.is_empty() {
                        pending_tool_calls.push(PendingToolCall {
                            id: "call_0".to_string(),
                            name: String::new(),
                            arguments: String::new(),
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

        // Build the assistant message to add to history
        if !pending_tool_calls.is_empty() {
            // Assistant message with tool_calls field (OpenAI format)
            let tool_calls_json: Vec<Value> = pending_tool_calls
                .iter()
                .map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments
                        }
                    })
                })
                .collect();

            // Add assistant message with tool calls
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: json!(""),
            });
            // We need to store the full message with tool_calls — modify the last entry
            let last_idx = messages.len() - 1;
            // We'll reconstruct the messages when sending, so store tool_calls info separately
            // Actually, OpenAI requires the assistant message to have tool_calls field
            // Let's store a special format
            messages[last_idx].content = json!({
                "__tool_calls__": tool_calls_json,
                "__text__": if accumulated_text.is_empty() { Value::Null } else { json!(accumulated_text) }
            });

            // Execute each tool and add tool response messages
            for tc in &pending_tool_calls {
                let input: Value =
                    serde_json::from_str(&tc.arguments).unwrap_or(json!({}));
                let result = execute_tool(app_handle, &tc.name, &input, project_path).await;

                let _ = app_handle.emit(
                    "ai-chat-tool-result",
                    json!({
                        "name": tc.name,
                        "id": tc.id,
                        "result": &result
                    })
                    .to_string(),
                );

                // Add tool response message (OpenAI format)
                // content MUST be a string for tool responses
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
                        "content": result_str
                    }),
                });
            }

            pending_tool_calls.clear();
            // Loop continues — will call OpenAI again with tool results
        } else {
            // Normal end of conversation
            if !accumulated_text.is_empty() {
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: json!(accumulated_text),
                });
            }

            let _ = app_handle.emit(
                "ai-chat-done",
                json!({ "text": &accumulated_text }).to_string(),
            );
            break;
        }
    }

    Ok(())
}

// Override message serialization for OpenAI's specific format
fn build_api_messages(system_prompt: &str, messages: &[ChatMessage]) -> Vec<Value> {
    let mut api_msgs: Vec<Value> = vec![json!({
        "role": "system",
        "content": system_prompt
    })];

    for m in messages {
        if let Some(obj) = m.content.as_object() {
            if obj.contains_key("__tool_calls__") {
                // This is an assistant message with tool calls
                let mut msg = json!({
                    "role": "assistant",
                });
                if let Some(text) = obj.get("__text__").and_then(|t| t.as_str()) {
                    msg["content"] = json!(text);
                } else {
                    msg["content"] = json!("");
                }
                msg["tool_calls"] = obj["__tool_calls__"].clone();
                api_msgs.push(msg);
            } else if obj.contains_key("__tool_response__") {
                // This is a tool response message
                api_msgs.push(json!({
                    "role": "tool",
                    "tool_call_id": obj["tool_call_id"],
                    "content": obj["content"]
                }));
            } else {
                api_msgs.push(json!({
                    "role": m.role,
                    "content": m.content
                }));
            }
        } else {
            api_msgs.push(json!({
                "role": m.role,
                "content": m.content
            }));
        }
    }

    api_msgs
}

// ── Tool execution ───────────────────────────────────────────────────────────

async fn execute_tool(app_handle: &AppHandle, name: &str, input: &Value, project_path: &Path) -> Value {
    match name {
        "read_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            let content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(e) => format!("Error reading file: {}", e),
            };
            json!({ "result": content })
        }
        "web_search" => {
            let query = input["query"].as_str().unwrap_or_default();
            match search_the_web(query).await {
                Ok(results) => json!({ "results": results }),
                Err(e) => json!({ "error": format!("Search failed: {}", e) }),
            }
        }
        "knowledge_search" => {
            let query = input["query"].as_str().unwrap_or_default();
            knowledge_search(app_handle, project_path, query).await
        }
        "write_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let content = input["content"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);

            let trimmed = content.trim();
            if trimmed.is_empty() {
                return json!({
                    "error": "write_file rejected: empty content. Provide the complete file content."
                });
            }

            let looks_like_placeholder =
                (trimmed.starts_with('<') && trimmed.ends_with('>'))
                    || trimmed.contains("updated_content")
                    || trimmed.contains("<your_code_here>")
                    || trimmed.contains("TODO_REPLACE");
            if looks_like_placeholder {
                return json!({
                    "error": "write_file rejected: placeholder content detected. Send the full, real file contents, not template placeholders."
                });
            }

            if let Some(parent) = full_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            match std::fs::write(&full_path, content) {
                Ok(_) => {
                    // Notify frontend that a file was changed on DISK
                    let _ = app_handle.emit("file-modified", json!({ "path": full_path.to_string_lossy() }).to_string());
                    json!({ "result": format!("Successfully wrote to {}", rel_path) })
                },
                Err(e) => json!({ "error": format!("Error writing file: {}", e) }),
            }
        }
        "list_files" => {
            let rel_path = input["path"].as_str().unwrap_or(".");
            let full_path = project_path.join(rel_path);
            match std::fs::read_dir(&full_path) {
                Ok(entries) => {
                    let mut items: Vec<String> = Vec::new();
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.path().is_dir();
                        if name.starts_with('.')
                            || name == "node_modules"
                            || name == "target"
                        {
                            continue;
                        }
                        items.push(if is_dir {
                            format!("{}/", name)
                        } else {
                            name
                        });
                    }
                    items.sort();
                    json!({ "result": items.join("\n") })
                }
                Err(e) => json!({ "error": format!("Error listing directory: {}", e) }),
            }
        }
        "esp_idf_info" => {
            if let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) {
                let python = find_idf_python_bin(&tools_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "not found".to_string());
                json!({
                    "result": {
                        "idf_path": idf_path.to_string_lossy(),
                        "tools_path": tools_path.to_string_lossy(),
                        "python": python,
                        "hint": "Use run_command with `idf.py ...`; runtime will inject IDF env vars."
                    }
                })
            } else {
                json!({ "error": "ESP-IDF paths are not resolved in this runtime." })
            }
        }
        "install_idf_library" => {
            let component = input["component"].as_str().unwrap_or("").trim();
            if component.is_empty() {
                return json!({
                    "error": "component is required, e.g. espressif/led_strip or espressif/led_strip^2.5.3"
                });
            }

            let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) else {
                return json!({ "error": "ESP-IDF paths are not resolved in this runtime." });
            };

            let Some(python_bin) = find_idf_python_bin(&tools_path) else {
                return json!({ "error": "ESP-IDF python environment not found." });
            };

            let idf_py = idf_path.join("tools/idf.py");
            let idf_version = std::fs::read_to_string(idf_path.join("version.txt"))
                .unwrap_or_default()
                .trim()
                .to_string();

            let mut cmd = tokio::process::Command::new(&python_bin);
            cmd.arg(&idf_py)
                .arg("add-dependency")
                .arg(component)
                .current_dir(project_path)
                .env("IDF_PATH", &idf_path)
                .env("IDF_TOOLS_PATH", &tools_path)
                .env("ESP_IDF_VERSION", &idf_version)
                .env("PATH", build_ai_idf_path(&tools_path));

            match cmd.output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    if output.status.success() {
                        let mut result = String::new();
                        if !stdout.trim().is_empty() {
                            result.push_str(stdout.trim());
                        }
                        if !stderr.trim().is_empty() {
                            if !result.is_empty() {
                                result.push_str("\n");
                            }
                            result.push_str(stderr.trim());
                        }
                        if result.is_empty() {
                            result = format!("Installed dependency: {}", component);
                        }
                        json!({ "result": result })
                    } else {
                        let err = if !stderr.trim().is_empty() {
                            stderr.trim().to_string()
                        } else {
                            stdout.trim().to_string()
                        };
                        json!({ "error": format!("Failed to install dependency {}: {}", component, err) })
                    }
                }
                Err(e) => json!({ "error": format!("Failed to run idf.py add-dependency: {}", e) }),
            }
        }
        "run_command" => {
            let command = input["command"].as_str().unwrap_or("");
            if command.contains("esptool.py") {
                return json!({
                    "error": "Direct esptool.py usage is disabled. Use `idf.py build flash` instead."
                });
            }
            let mut exec_command = command.to_string();
            let mut process = if cfg!(target_os = "windows") {
                let mut c = tokio::process::Command::new("cmd");
                c.arg("/C");
                c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.arg("-c");
                c
            };

            if let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) {
                let idf_version = std::fs::read_to_string(idf_path.join("version.txt"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                process
                    .env("IDF_PATH", &idf_path)
                    .env("IDF_TOOLS_PATH", &tools_path)
                    .env("ESP_IDF_VERSION", idf_version)
                    .env("PATH", build_ai_idf_path(&tools_path));

                if let Some(python_bin) = find_idf_python_bin(&tools_path) {
                    if let Some(rel) = command.trim_start().strip_prefix("idf.py") {
                        let tail = rel.trim();
                        let idf_py = idf_path.join("tools/idf.py");
                        exec_command = if tail.is_empty() {
                            format!(
                                "\"{}\" \"{}\"",
                                python_bin.to_string_lossy(),
                                idf_py.to_string_lossy()
                            )
                        } else {
                            format!(
                                "\"{}\" \"{}\" {}",
                                python_bin.to_string_lossy(),
                                idf_py.to_string_lossy(),
                                tail
                            )
                        };
                    }
                }
            }

            match process
                .arg(&exec_command)
                .current_dir(project_path)
                .output()
                .await
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() {
                        result.push_str(&stdout);
                    }
                    if !stderr.is_empty() {
                        if !result.is_empty() {
                            result.push_str("\n--- stderr ---\n");
                        }
                        result.push_str(&stderr);
                    }
                    if result.is_empty() {
                        json!({ "result": "Command completed successfully (no output)" })
                    } else {
                        if result.len() > 10000 {
                            result.truncate(10000);
                            result.push_str("\n... (output truncated)");
                        }
                        json!({ "result": result })
                    }
                }
                Err(e) => json!({ "error": format!("Error running command: {}", e) }),
            }
        }
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

async fn search_the_web(query: &str) -> Result<Value, String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Search server returned error: {}", response.status()));
    }

    let html_content = response.text().await.map_err(|e| format!("Failed to read body: {}", e))?;
    let document = Html::parse_document(&html_content);
    
    let result_selector = Selector::parse(".result").map_err(|_| "Failed to parse selector")?;
    let title_selector = Selector::parse(".result__a").map_err(|_| "Failed to parse title selector")?;
    let snippet_selector = Selector::parse(".result__snippet").map_err(|_| "Failed to parse snippet selector")?;

    let mut results = Vec::new();

    for element in document.select(&result_selector).take(5) {
        let mut title = String::new();
        let mut link = String::new();
        let mut snippet = String::new();

        if let Some(t_elem) = element.select(&title_selector).next() {
            title = t_elem.text().collect::<Vec<_>>().join(" ").trim().to_string();
            link = t_elem.value().attr("href").unwrap_or("").to_string();
        }

        if let Some(s_elem) = element.select(&snippet_selector).next() {
            snippet = s_elem.text().collect::<Vec<_>>().join(" ").trim().to_string();
        }

        if !title.is_empty() {
            results.push(json!({
                "title": title,
                "link": link,
                "snippet": snippet
            }));
        }
    }

    Ok(json!(results))
}


async fn get_embeddings_internal(api_key: &str, mut base_url: String, text: &str) -> Result<Vec<f32>, String> {
    if !base_url.starts_with("http") && !base_url.is_empty() {
        base_url = format!("http://{}", base_url);
    }
    
    // Auto-fix for local servers
    if (base_url.contains("localhost") || base_url.contains("127.0.0.1")) 
       && !base_url.contains("/v1") 
    {
        base_url = format!("{}/v1", base_url.trim_end_matches('/'));
    }

    let client = Client::new();
    let res = client.post(format!("{}/embeddings", base_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&json!({
            "input": text,
            "model": "text-embedding-3-small"
        }))
        .send()
        .await
        .map_err(|e| format!("Embedding request failed: {}", e))?;

    let data: Value = res.json().await.map_err(|e| format!("Failed to parse embedding response: {}", e))?;
    
    if let Some(err) = data["error"].as_object() {
        return Err(err["message"].as_str().unwrap_or("Unknown API error").to_string());
    }

    let embedding = data["data"][0]["embedding"]
        .as_array()
        .ok_or("No embedding data in response")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    Ok(embedding)
}

async fn get_embeddings(_app_handle: &AppHandle, text: &str) -> Result<Vec<f32>, String> {
    let (api_key, base_url) = {
        let config = read_config();
        (
            config["api_key"].as_str().unwrap_or("").to_string(),
            config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string()
        )
    };
    get_embeddings_internal(&api_key, base_url, text).await
}

fn cosine_similarity(v1: &[f32], v2: &[f32]) -> f32 {
    let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let norm1: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let norm2: f32 = v2.iter().map(|a| a * a).sum::<f32>().sqrt();
    if norm1 > 0.0 && norm2 > 0.0 {
        dot_product / (norm1 * norm2)
    } else {
        0.0
    }
}

fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);
        if end == chars.len() { break; }
        start += chunk_size - overlap;
    }
    chunks
}

async fn reindex_knowledge_base(project_path: &Path) -> Result<usize, String> {
    let kb_path = project_path.join("knowledge_base");
    if !kb_path.exists() { return Ok(0); }

    let index_file = kb_path.join(".embeddings.json");
    let mut index: VectorIndex = if index_file.exists() {
        let data = std::fs::read_to_string(&index_file).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        VectorIndex::default()
    };

    let (api_key, base_url) = {
        let config = read_config();
        (
            config["api_key"].as_str().unwrap_or("").to_string(),
            config["base_url"].as_str().unwrap_or("https://api.openai.com/v1").to_string()
        )
    };

    let mut changed = false;

    if let Ok(entries) = std::fs::read_dir(&kb_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if name == ".embeddings.json" || !(name.ends_with(".txt") || name.ends_with(".md")) {
                    continue;
                }

                let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                }).unwrap_or(0);

                if index.last_indexed.get(&name).cloned().unwrap_or(0) < mtime {
                    // Re-index this file
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        index.chunks.retain(|c| c.file_name != name);
                        
                        let chunks = chunk_text(&content, 800, 100);
                        for chunk_content in chunks {
                            if let Ok(embedding) = get_embeddings_internal(&api_key, base_url.clone(), &chunk_content).await {
                                index.chunks.push(KnowledgeChunk {
                                    file_name: name.clone(),
                                    content: chunk_content,
                                    embedding,
                                });
                            }
                        }
                        index.last_indexed.insert(name.clone(), mtime);
                        changed = true;
                    }
                }
            }
        }
    }

    if changed {
        let data = serde_json::to_string_pretty(&index).unwrap_or_default();
        let _ = std::fs::write(&index_file, data);
    }

    Ok(index.chunks.len())
}

pub async fn knowledge_search(app_handle: &AppHandle, project_path: &Path, query: &str) -> Value {
    // Ensure index is up to date
    let _ = reindex_knowledge_base(&project_path).await;
    
    let query_embedding = match get_embeddings(&app_handle, &query).await {
        Ok(e) => e,
        Err(e) => return json!({ "error": format!("Could not embed query: {}", e) }),
    };

    let kb_path = project_path.join("knowledge_base");
    let index_file = kb_path.join(".embeddings.json");
    if !index_file.exists() {
        return json!({ "message": "No local documents indexed yet." });
    }

    let data = std::fs::read_to_string(&index_file).unwrap_or_default();
    let index: VectorIndex = serde_json::from_str(&data).unwrap_or_default();

    let mut matches: Vec<(f32, &KnowledgeChunk)> = index.chunks
        .iter()
        .map(|chunk| (cosine_similarity(&query_embedding, &chunk.embedding), chunk))
        .filter(|(score, _)| *score > 0.3) // Similarity threshold
        .collect();

    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    let results: Vec<Value> = matches.iter().take(5).map(|(score, chunk)| {
        json!({
            "file": chunk.file_name,
            "score": score,
            "content": chunk.content
        })
    }).collect();

    if results.is_empty() {
        json!({ "message": "No relevant local documents found for your search query." })
    } else {
        json!(results)
    }
}
