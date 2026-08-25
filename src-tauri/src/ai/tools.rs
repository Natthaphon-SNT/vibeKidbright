//! AI tool execution: read/write files, search, diff, web search, IDF helpers.
//!
//! Extracted from ai_chat.rs (refactoring roadmap step 4).
//! execute_tool is invoked by the provider conversation loops in ai_chat.rs.

use crate::ai::kb::knowledge_search;
use crate::ai_chat::{
    build_ai_idf_path_cached, find_idf_python_bin, get_pending_diffs, resolve_idf_paths_for_ai,
    resolve_kb_path, AiBackupState,
};
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

// ── Tool execution ─────────────────────────────────────────────────────────────

/// execute_tool takes message_id as &str — it doesn't store or spawn, so no Arc needed here.
pub(crate) async fn execute_tool(
    app_handle: &AppHandle,
    name: &str,
    input: &Value,
    project_path: &mut PathBuf,
    message_id: &str,
    no_workspace: &mut bool,
) -> Value {
    if *no_workspace && matches!(name, "write_file" | "run_command" | "read_file" | "list_files" | "get_file_tree" | "search_in_files" | "diff_file") {
        return json!({
            "error": "BLOCKED: No project workspace is open. You MUST call 'create_project_workspace' FIRST."
        });
    }

    // HARD BLOCK: Prevent the model from calling create_project_workspace when a workspace
    // is already open. This stops disruptive folder-picker dialogs from appearing mid-task.
    if name == "create_project_workspace" && !*no_workspace {
        return json!({
            "error": "BLOCKED: A workspace is already open. Do NOT call create_project_workspace again. Use write_file to create or modify files in the current workspace instead."
        });
    }

    match name {
        // ── read_file ──────────────────────────────────────────────────────────
        "read_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            match std::fs::read_to_string(&full_path) {
                Ok(c) => json!({ "result": c }),
                Err(e) => json!({ "error": format!("Error reading file: {}", e) }),
            }
        }

        // ── write_file ─────────────────────────────────────────────────────────
        "write_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let content = input["content"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            let trimmed = content.trim();
            if trimmed.is_empty() {
                return json!({ "error": "write_file rejected: empty content." });
            }
            let looks_like_placeholder = (trimmed.starts_with('<') && trimmed.ends_with('>'))
                || trimmed.contains("updated_content")
                || trimmed.contains("<your_code_here>")
                || trimmed.contains("TODO_REPLACE");
            if looks_like_placeholder {
                return json!({ "error": "write_file rejected: placeholder content detected. Send the full, real file contents." });
            }
            if let Some(parent) = full_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let file_exists = full_path.exists();
            if file_exists {
                let old_content = std::fs::read_to_string(&full_path).unwrap_or_default();
                if old_content == content {
                    return json!({ "result": format!("File {} is already up to date.", rel_path) });
                }
                // FIX: Back up the old content so undo_ai_changes can restore it.
                // Previously only new files were backed up (with None), leaving existing-file
                // edits un-undoable.
                if let Some(state) = app_handle.try_state::<AiBackupState>() {
                    let mut backups = state.backups.lock().unwrap();
                    let message_backups = backups.entry(message_id.to_string()).or_insert_with(HashMap::new);
                    // Only record the *first* backup for this path in this message turn.
                    message_backups.entry(full_path.clone()).or_insert_with(|| Some(old_content.clone()));
                }
                {
                    let mut diffs = get_pending_diffs().lock().unwrap();
                    diffs.insert(full_path.clone(), content.to_string());
                }
                let _ = app_handle.emit("ai-diff-pending", json!({
                    "fullPath": full_path.to_string_lossy(),
                    "relPath": rel_path
                }).to_string());
                json!({ "result": format!("I have proposed changes to '{}'. Please review the diff in the editor and click Keep or Undo.", rel_path) })
            } else {
                if let Some(state) = app_handle.try_state::<AiBackupState>() {
                    let mut backups = state.backups.lock().unwrap();
                    let message_backups = backups.entry(message_id.to_string()).or_insert_with(HashMap::new);
                    if !message_backups.contains_key(&full_path) {
                        message_backups.insert(full_path.clone(), None);
                    }
                }
                match std::fs::write(&full_path, content) {
                    Ok(_) => {
                        let _ = app_handle.emit("file-modified", json!({ "path": full_path.to_string_lossy() }).to_string());
                        json!({ "result": format!("Created new file: {}", rel_path) })
                    }
                    Err(e) => json!({ "error": format!("Error writing file: {}", e) }),
                }
            }
        }

        // ── list_files ─────────────────────────────────────────────────────────
        "list_files" => {
            let rel_path = input["path"].as_str().unwrap_or(".");
            let full_path = project_path.join(rel_path);
            match std::fs::read_dir(&full_path) {
                Ok(entries) => {
                    let mut items: Vec<String> = entries.flatten().filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') || name == "node_modules" || name == "target" { return None; }
                        Some(if e.path().is_dir() { format!("{}/", name) } else { name })
                    }).collect();
                    items.sort();
                    json!({ "result": items.join("\n") })
                }
                Err(e) => json!({ "error": format!("Error listing directory: {}", e) }),
            }
        }

        // ── get_file_tree (NEW) ────────────────────────────────────────────────
        "get_file_tree" => {
            let max_depth = input["max_depth"].as_u64().unwrap_or(4).min(8) as usize;
            let mut output = String::new();
            build_file_tree(project_path, project_path, 0, max_depth, &mut output);
            if output.is_empty() {
                json!({ "result": "(empty directory)" })
            } else {
                json!({ "result": output })
            }
        }

        // ── search_in_files (NEW) ──────────────────────────────────────────────
        "search_in_files" => {
            let pattern_str = input["pattern"].as_str().unwrap_or("");
            let ext_filter = input["file_extension"].as_str().unwrap_or("").to_lowercase();
            let regex = match regex::Regex::new(pattern_str) {
                Ok(r) => r,
                Err(e) => return json!({ "error": format!("Invalid regex '{}': {}", pattern_str, e) }),
            };
            let mut matches: Vec<String> = Vec::new();
            search_files_recursive(project_path, project_path, &regex, &ext_filter, &mut matches, 0);
            if matches.is_empty() {
                json!({ "result": format!("No matches found for pattern '{}'", pattern_str) })
            } else {
                // Cap output to avoid overwhelming the context window.
                matches.truncate(100);
                json!({ "result": matches.join("\n"), "note": "Results capped at 100 matches." })
            }
        }

        // ── diff_file (NEW) ────────────────────────────────────────────────────
        "diff_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let new_content = input["new_content"].as_str().unwrap_or("");
            let full_path = project_path.join(rel_path);
            let old_content = if full_path.exists() {
                std::fs::read_to_string(&full_path).unwrap_or_default()
            } else {
                String::new()
            };
            if old_content == new_content {
                return json!({ "result": "No differences — files are identical." });
            }
            let diff = compute_unified_diff(&old_content, new_content, rel_path);
            json!({ "result": diff })
        }

        // ── web_search ─────────────────────────────────────────────────────────
        "web_search" => {
            let query = input["query"].as_str().unwrap_or_default();
            match search_the_web(query).await {
                Ok(results) => json!({ "results": results }),
                Err(e) => json!({ "error": format!("Search failed: {}", e) }),
            }
        }

        // ── knowledge_search ───────────────────────────────────────────────────
        "knowledge_search" => {
            let query = input["query"].as_str().unwrap_or_default();
            knowledge_search(app_handle, project_path, query).await
        }

        "read_knowledge_file" => {
            let file_name = input["file_name"].as_str().unwrap_or("");
            let kb_path = resolve_kb_path(&project_path.to_string_lossy());
            let full_path = kb_path.join(file_name);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => json!({ "result": content }),
                Err(e) => json!({ 
                    "error": format!("Cannot read '{}' from knowledge_base: {}", file_name, e) 
                }),
            }
        }

        // ── create_project_workspace ───────────────────────────────────────────
        "create_project_workspace" => {
            let project_name = input["project_name"].as_str().unwrap_or("my_esp_project");
            if let Some(picked_dir) = rfd::FileDialog::new()
                .set_title("Select a Base Directory for Your New Project")
                .pick_folder()
            {
                let new_proj_path = picked_dir.join(project_name);
                if let Err(e) = std::fs::create_dir_all(&new_proj_path) {
                    return json!({ "error": format!("Failed to create directory: {}", e) });
                }
                *project_path = new_proj_path.clone();
                *no_workspace = false;
                let root_cmake = format!(
                    "cmake_minimum_required(VERSION 3.16)\ninclude($ENV{{IDF_PATH}}/tools/cmake/project.cmake)\nproject({})\n",
                    project_name
                );
                let _ = std::fs::write(new_proj_path.join("CMakeLists.txt"), root_cmake);
                let _ = std::fs::create_dir_all(new_proj_path.join("main"));
                let _ = std::fs::write(
                    new_proj_path.join("main/CMakeLists.txt"),
                    "idf_component_register(SRCS \"main.c\"\n                    INCLUDE_DIRS \".\")\n",
                );
                let _ = std::fs::write(
                    new_proj_path.join("main/main.c"),
                    "#include <stdio.h>\nvoid app_main(void) {\n    printf(\"Hello\\n\");\n}\n",
                );
                let _ = std::fs::write(
                    new_proj_path.join("sdkconfig"),
                    "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n",
                );
                let _ = app_handle.emit("force-project-dir", new_proj_path.to_string_lossy().to_string());
                json!({
                    "result": format!(
                        "Successfully created project workspace: {}. Boilerplate files were automatically created. Use write_file to populate main/main.c.",
                        new_proj_path.display()
                    ),
                    "workspace_path": new_proj_path.to_string_lossy().to_string()
                })
            } else {
                json!({ "error": "User cancelled the folder selection dialog." })
            }
        }

        // ── esp_idf_info ───────────────────────────────────────────────────────
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
                        "hint": "Use run_command with `idf.py ...`; runtime injects IDF env vars."
                    }
                })
            } else {
                json!({ "error": "ESP-IDF paths are not resolved in this runtime." })
            }
        }

        // ── install_idf_library ────────────────────────────────────────────────
        "install_idf_library" => {
            let component = input["component"].as_str().unwrap_or("").trim();
            if component.is_empty() {
                return json!({ "error": "component is required, e.g. espressif/led_strip" });
            }
            let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) else {
                return json!({ "error": "ESP-IDF paths are not resolved in this runtime." });
            };
            let Some(python_bin) = find_idf_python_bin(&tools_path) else {
                return json!({ "error": "ESP-IDF python environment not found." });
            };
            let idf_py = idf_path.join("tools/idf.py");
            let idf_version = std::fs::read_to_string(idf_path.join("version.txt"))
                .unwrap_or_default().trim().to_string();
            let mut cmd = tokio::process::Command::new(&python_bin);
            cmd.arg(&idf_py).arg("add-dependency").arg(component)
                .current_dir(project_path)
                .env("IDF_PATH", &idf_path)
                .env("IDF_TOOLS_PATH", &tools_path)
                .env("ESP_IDF_VERSION", &idf_version)
                .env("PATH", build_ai_idf_path_cached(&tools_path));
            if let Some(rom_elf_dir) = crate::esp_idf::find_esp_rom_elf_dir(&tools_path) {
                cmd.env("ESP_ROM_ELF_DIR", rom_elf_dir);
            }
            match cmd.output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    if output.status.success() {
                        let result = [stdout.trim(), stderr.trim()]
                            .iter().filter(|s| !s.is_empty())
                            .cloned().collect::<Vec<_>>().join("\n");
                        let result = if result.is_empty() {
                            format!("Installed dependency: {}", component)
                        } else {
                            result
                        };
                        json!({ "result": result })
                    } else {
                        let err = if !stderr.trim().is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
                        json!({ "error": format!("Failed to install {}: {}", component, err) })
                    }
                }
                Err(e) => json!({ "error": format!("Failed to run idf.py add-dependency: {}", e) }),
            }
        }

        // ── run_command ────────────────────────────────────────────────────────
        "run_command" => {
            let command = input["command"].as_str().unwrap_or("");
            if command.contains("esptool.py") {
                return json!({ "error": "Direct esptool.py usage is disabled. Use `idf.py build flash` instead." });
            }
            let mut exec_command = command.to_string();
            let mut process = if cfg!(target_os = "windows") {
                let mut c = tokio::process::Command::new("cmd");
                c.arg("/C"); c
            } else {
                let mut c = tokio::process::Command::new("sh");
                c.arg("-c"); c
            };

            if let Some((idf_path, tools_path)) = resolve_idf_paths_for_ai(app_handle) {
                let idf_version = std::fs::read_to_string(idf_path.join("version.txt"))
                    .unwrap_or_default().trim().to_string();
                // FIX: Use cached IDF path — no directory scan on every command.
                process
                    .env("IDF_PATH", &idf_path)
                    .env("IDF_TOOLS_PATH", &tools_path)
                    .env("ESP_IDF_VERSION", idf_version)
                    .env("PATH", build_ai_idf_path_cached(&tools_path));
                if let Some(rom_elf_dir) = crate::esp_idf::find_esp_rom_elf_dir(&tools_path) {
                    process.env("ESP_ROM_ELF_DIR", rom_elf_dir);
                }
                if let Some(python_bin) = find_idf_python_bin(&tools_path) {
                    if let Some(rel) = command.trim_start().strip_prefix("idf.py") {
                        let tail = rel.trim();
                        let idf_py = idf_path.join("tools/idf.py");
                        exec_command = if tail.is_empty() {
                            format!("\"{}\" \"{}\"", python_bin.to_string_lossy(), idf_py.to_string_lossy())
                        } else {
                            format!("\"{}\" \"{}\" {}", python_bin.to_string_lossy(), idf_py.to_string_lossy(), tail)
                        };
                    }
                }
            }

            match process.arg(&exec_command).current_dir(project_path).output().await {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let mut result = String::new();
                    if !stdout.is_empty() { result.push_str(&stdout); }
                    if !stderr.is_empty() {
                        if !result.is_empty() { result.push_str("\n--- stderr ---\n"); }
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

// ── Helper: recursive file tree ───────────────────────────────────────────────

fn build_file_tree(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    output: &mut String,
) {
    if depth > max_depth { return; }
    let skip = ["node_modules", "target", ".git", "build", ".embeddings.json"];
    let mut entries: Vec<_> = match std::fs::read_dir(current) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || skip.iter().any(|&s| s == name) { continue; }
        let indent = "  ".repeat(depth);
        let is_dir = entry.path().is_dir();
        output.push_str(&format!("{}{}{}\n", indent, name, if is_dir { "/" } else { "" }));
        if is_dir {
            build_file_tree(root, &entry.path(), depth + 1, max_depth, output);
        }
    }
}

// ── Helper: recursive file search ─────────────────────────────────────────────

fn search_files_recursive(
    root: &Path,
    current: &Path,
    regex: &regex::Regex,
    ext_filter: &str,
    matches: &mut Vec<String>,
    depth: usize,
) {
    if depth > 8 { return; }
    let skip = ["node_modules", "target", ".git", "build"];
    let entries: Vec<_> = match std::fs::read_dir(current) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || skip.iter().any(|&s| s == name) { continue; }
        if path.is_dir() {
            search_files_recursive(root, &path, regex, ext_filter, matches, depth + 1);
        } else {
            if !ext_filter.is_empty() {
                let file_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if file_ext != ext_filter { continue; }
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                let rel = path.strip_prefix(root).unwrap_or(&path);
                for (i, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        matches.push(format!("{}:{}: {}", rel.display(), i + 1, line.trim()));
                        if matches.len() >= 100 { return; }
                    }
                }
            }
        }
    }
}

// ── Helper: Myers unified diff (via `similar` crate) ─────────────────────────
// Replaces the previous O(n²) naive implementation.
// Produces standard unified diffs with proper hunk headers and context lines.

fn compute_unified_diff(old: &str, new: &str, path: &str) -> String {
    use similar::{Algorithm, ChangeTag, TextDiff};

    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_lines(old, new);

    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    let mut has_diff = false;

    for group in diff.grouped_ops(3) {
        has_diff = true;
        let first = &group[0];
        let _last = &group[group.len() - 1];

        let old_start = first.old_range().start + 1;
        let old_len: usize = group.iter().map(|op| op.old_range().len()).sum();
        let new_start = first.new_range().start + 1;
        let new_len: usize = group.iter().map(|op| op.new_range().len()).sum();

        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            old_start, old_len, new_start, new_len
        ));

        for op in &group {
            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal  => " ",
                };
                out.push_str(prefix);
                out.push_str(change.value());
                if !change.value().ends_with('\n') {
                    out.push('\n');
                }
            }
        }
    }

    if !has_diff {
        out.push_str("(no differences)\n");
    }

    out
}
// ── Web search ────────────────────────────────────────────────────────────────

async fn search_the_web(query: &str) -> Result<Value, String> {
    let user_agents = [
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64; rv:125.0) Gecko/20100101 Firefox/125.0",
    ];
    for ua in &user_agents {
        if let Ok(results) = try_ddg_search(query, ua).await {
            if !results.as_array().map(|a| a.is_empty()).unwrap_or(true) {
                return Ok(results);
            }
        }
    }
    try_bing_search(query).await
}

async fn try_ddg_search(query: &str, user_agent: &str) -> Result<Value, String> {
    let client = Client::builder()
        .user_agent(user_agent)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;
    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let response = client.get(&url).send().await
        .map_err(|e| format!("DDG request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("DDG error: {}", response.status()));
    }
    let html_content = response.text().await
        .map_err(|e| format!("Failed to read DDG body: {}", e))?;
    let document = Html::parse_document(&html_content);
    let result_selector = Selector::parse(".result").map_err(|_| "Bad selector")?;
    let title_selector = Selector::parse(".result__a").map_err(|_| "Bad selector")?;
    let snippet_selector = Selector::parse(".result__snippet").map_err(|_| "Bad selector")?;
    let mut results = Vec::new();
    for element in document.select(&result_selector).take(5) {
        let title = element.select(&title_selector).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let link = element.select(&title_selector).next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("").to_string();
        let snippet = element.select(&snippet_selector).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if !title.is_empty() {
            results.push(json!({ "title": title, "link": link, "snippet": snippet }));
        }
    }
    Ok(json!(results))
}

async fn try_bing_search(query: &str) -> Result<Value, String> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (compatible; Googlebot/2.1)")
        .build()
        .map_err(|e| format!("Failed to build Bing client: {}", e))?;
    let url = format!("https://www.bing.com/search?q={}", urlencoding::encode(query));
    let response = client.get(&url).send().await
        .map_err(|e| format!("Bing request failed: {}", e))?;
    if !response.status().is_success() {
        return Err(format!("Bing error: {}", response.status()));
    }
    let html = response.text().await
        .map_err(|e| format!("Failed to read Bing body: {}", e))?;
    let document = Html::parse_document(&html);
    let result_sel = Selector::parse("li.b_algo").map_err(|_| "Bad selector")?;
    let title_sel = Selector::parse("h2 a").map_err(|_| "Bad selector")?;
    let snippet_sel = Selector::parse(".b_caption p").map_err(|_| "Bad selector")?;
    let mut results = Vec::new();
    for element in document.select(&result_sel).take(5) {
        let title = element.select(&title_sel).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let link = element.select(&title_sel).next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or("").to_string();
        let snippet = element.select(&snippet_sel).next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if !title.is_empty() {
            results.push(json!({ "title": title, "link": link, "snippet": snippet, "source": "bing" }));
        }
    }
    Ok(json!(results))
}
