//! AI tool execution: read/write files, search, diff, web search, IDF helpers.
//!
//! Extracted from ai_chat.rs (refactoring roadmap step 4).
//! execute_tool is invoked by the provider conversation loops in ai_chat.rs.

use crate::ai_chat::{find_idf_python_bin, resolve_idf_paths_for_ai, resolve_kb_path};
use reqwest::Client;
use scraper::{Html, Selector};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter};

// ── Tool execution ─────────────────────────────────────────────────────────────

fn allowed_ai_idf_action(command: &str) -> Result<&'static str,String> {
    let parts: Vec<_> = command.split_whitespace().collect();
    match parts.as_slice() {
        ["idf.py", "build"] => Ok("build"),
        ["idf.py", "size"] => Ok("size"),
        ["idf.py", "--version"] => Ok("--version"),
        _ => Err("AI run_command allows only idf.py build, idf.py size, or idf.py --version. Use file/search tools for inspection. Flash/monitor/clean and arbitrary shell commands require the user-operated application controls; shell operators and extra arguments are rejected.".into()),
    }
}
fn dependency_proposal(root:&Path,component:&str)->Result<(PathBuf,String),String>{
    let pattern=regex::Regex::new(r"^([A-Za-z0-9_-]+/[A-Za-z0-9_-]+)([\^~<>=0-9.*,-]*)$").unwrap();
    let parsed=pattern.captures(component).ok_or("Invalid component identifier/version. Expected namespace/name or namespace/name^1.2.3.")?;
    let path=workspace_file_path(root,"main/idf_component.yml")?;
    let mut value=if path.exists(){serde_json::from_str::<Value>(&std::fs::read_to_string(&path).map_err(|e|e.to_string())?).map_err(|_|"Existing manifest is YAML. Read it and propose its complete content with write_file so existing settings remain reviewable.")?}else{json!({"dependencies":{}})};
    if !value.is_object()||!value.get("dependencies").is_some_and(Value::is_object){return Err("Dependency manifest requires a dependencies object; propose a full reviewed edit.".into());}
    let version=if parsed[2].is_empty(){"*"}else{&parsed[2]};
    value["dependencies"][&parsed[1]]=json!(version);
    Ok((path,format!("{}\n",serde_json::to_string_pretty(&value).map_err(|e|e.to_string())?)))
}

/// File tools accept paths relative to their workspace, including not-yet-created files.
/// Resolve the nearest existing ancestor so links cannot redirect access outside it.
fn workspace_file_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative);
    if relative.components().any(|part| !matches!(part, Component::Normal(_) | Component::CurDir)) {
        return Err("File tool paths must stay relative to the selected workspace.".into());
    }
    #[cfg(windows)]
    for part in relative.components() {
        if let Component::Normal(name) = part {
            let name = name.to_string_lossy();
            let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            if name.trim_end_matches([' ', '.']) != name || name.contains(':')
                || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9')) {
                return Err("Windows device names and path aliases are not valid workspace files.".into());
            }
        }
    }
    let resolved_root = dunce::canonicalize(root).map_err(|error| error.to_string())?;
    let candidate = resolved_root.join(relative);
    let mut existing = candidate.as_path();
    loop {
        match std::fs::symlink_metadata(existing) {
            Ok(_) => {
                let resolved = dunce::canonicalize(existing).map_err(|error| error.to_string())?;
                if !resolved.starts_with(&resolved_root) {
                    return Err("File path resolves outside the selected workspace.".into());
                }
                return Ok(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                existing = existing.parent().ok_or("Cannot resolve workspace file parent.")?;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

#[cfg(test)]
fn command_result(success: bool, code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Value {
    let mut text = String::from_utf8_lossy(stdout).into_owned();
    if !stderr.is_empty() {
        if !text.is_empty() { text.push_str("\n--- stderr ---\n"); }
        text.push_str(&String::from_utf8_lossy(stderr));
    }
    if text.len() > 10_000 {
        let mut end = 10_000;
        while !text.is_char_boundary(end) { end -= 1; }
        text.truncate(end);
        text.push_str("\n... (output truncated)");
    }
    if !success {
        json!({ "error": format!("Command failed (exit code {}).\n{}", code.map(|code| code.to_string()).unwrap_or_else(|| "unknown".into()), text), "exit_code": code })
    } else {
        json!({ "result": if text.is_empty() { "Command completed successfully (no output)".into() } else { text } })
    }
}

/// Literal public boilerplate, not a read of the user's filesystem.
fn starter_files(project_name: &str) -> Vec<(String, String)> {
    vec![
        ("CMakeLists.txt".into(), format!("cmake_minimum_required(VERSION 3.16)\ninclude($ENV{{IDF_PATH}}/tools/cmake/project.cmake)\nproject({})\n", project_name)),
        ("main/CMakeLists.txt".into(), "idf_component_register(SRCS \"main.c\"\n                    INCLUDE_DIRS \".\")\n".into()),
        ("main/main.c".into(), "#include <stdio.h>\nvoid app_main(void) {\n    printf(\"Hello\\n\");\n}\n".into()),
        ("sdkconfig".into(), "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n".into()),
    ]
}

/// Safe fallback: never resolve paths, read config/headers or run SDK processes.
fn public_sdk_reference() -> Value {
    json!({"result": {
        "context_kind": "bundled_reference_only",
        "installed_sdk_inspected": false,
        "reference_target": "esp32",
        "reference_sdk_series": "5.5",
        "source": "Bundled compatibility/README.md; API target for a draft, not the installed version",
        "hint": "You may create a new reviewed custom ESP-IDF draft from public board evidence and generated starter files. Private sharing is not required for drafting. Do not claim installed SDK compatibility or build success. Existing private file reads and configured SDK/build inspection still require consent; Keep/Undo applies."
    }})
}

fn private_context_guard(name: &str, allowed: bool) -> Option<Value> {
    if allowed { return None; }
    if name == "esp_idf_info" { return Some(public_sdk_reference()); }
    if matches!(name,"read_file"|"list_files"|"get_file_tree"|"search_in_files"|"project_search"|"compiler_diagnostics"|"diff_file"|"run_command"|"install_idf_library"|"web_search") {
        return Some(json!({"error":super::privacy::REQUIRED}));
    }
    None
}

/// Initialize a fresh directory without overwriting an existing project.
fn initialize_project_workspace(base_dir: &Path, project_name: &str) -> Result<PathBuf, String> {
    if project_name.is_empty()
        || !project_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid project_name. Use only ASCII letters, digits, underscores or hyphens.".to_string());
    }
    let new_proj_path = base_dir.join(project_name);
    std::fs::create_dir(&new_proj_path).map_err(|e| {
        format!("Cannot create new project '{}': {}. Choose a different name or base directory; do not write the new project into the previous workspace.", new_proj_path.display(), e)
    })?;
    let result = (|| -> std::io::Result<()> {
        std::fs::create_dir(new_proj_path.join("main"))?;
        for (relative, content) in starter_files(project_name) {
            std::fs::write(new_proj_path.join(relative), content)?;
        }
        Ok(())
    })();
    result.map_err(|e| format!("Failed to initialize '{}': {}. Workspace was not switched. Stop creation; do not write into the previous workspace.", new_proj_path.display(), e))?;
    Ok(new_proj_path)
}

/// execute_tool takes message_id as &str — it doesn't store or spawn, so no Arc needed here.
pub(crate) async fn execute_tool(
    app_handle: &AppHandle,
    name: &str,
    input: &Value,
    project_path: &mut PathBuf,
    message_id: &str,
    no_workspace: &mut bool,
    private_context_allowed: bool,
) -> Value {
    if let Some(result) = private_context_guard(name, private_context_allowed) { return result; }
    if *no_workspace && matches!(name, "write_file" | "run_command" | "read_file" | "list_files" | "get_file_tree" | "search_in_files" | "project_search" | "diff_file") {
        return json!({
            "error": "BLOCKED: No project workspace is open. You MUST call 'create_project_workspace' FIRST."
        });
    }

    // Creating a project is allowed even with an open workspace. The workflow prompt
    // distinguishes creation requests from edits; the folder picker selects the destination.

    match name {
        "kidbright_block_to_c" => {
            let Some(raw)=input["program_json"].as_str() else{return crate::intelligence::block_to_c::mapping_contract()};
            if raw.len()>256*1024{return json!({"error":"Block program exceeds size limit"});}
            let program=match serde_json::from_str::<crate::intelligence::block_to_c::Program>(raw){Ok(p)=>p,Err(error)=>return json!({"error":error.to_string()})};
            match crate::intelligence::block_to_c::compile(&program){Ok(code)=>json!({"main_c":code,"board_profile":program.board_profile,"sdk_series":"5.5","physical_behavior":"unverified","mapping":"normalized v1 nine-ID typed subset; exact fields and bounds in block-mapping.schema.json. No generic Blockly import, POWER, strings, elseif or computed repeat counts.","workflow":"Create a confirmed matching reference workspace first if requested, then propose this main/main.c with write_file. Use the existing kb_core component; every edit still requires review."}),Err(error)=>json!({"error":error})}
        }
        "compiler_diagnostics" => match super::repair::diagnostic_context(project_path){Ok(value)=>value,Err(error)=>json!({"error":error})},
        "project_search" => {
            use tauri::Manager;
            use std::sync::{Arc,atomic::AtomicBool};
            let query=input["query"].as_str().unwrap_or("").to_owned();
            let root=project_path.clone();let cache=super::config::config_dir();
            let cancel=app_handle.try_state::<crate::ai_chat::AiAbortState>().map(|s|s.0.clone()).unwrap_or_else(||Arc::new(AtomicBool::new(false)));
            match tokio::task::spawn_blocking(move||crate::intelligence::project::search(&root,&cache,&query,&cancel)).await {
                Ok(Ok((report,hits)))=>json!({"index":report,"results":hits,"scope":"private project main/components C/H; syntax search, not semantic references"}),
                Ok(Err(error))=>json!({"error":error}),Err(error)=>json!({"error":error.to_string()}),
            }
        }
        "kidbright_reference" => crate::ai::compatibility::reference(input),
        // ── read_file ──────────────────────────────────────────────────────────
        "read_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let full_path = match workspace_file_path(project_path, rel_path) {
                Ok(path) => path,
                Err(error) => return json!({ "error": error }),
            };
            match std::fs::read_to_string(&full_path) {
                Ok(c) => json!({ "result": c }),
                Err(e) => json!({ "error": format!("Error reading file: {}", e) }),
            }
        }

        // ── write_file ─────────────────────────────────────────────────────────
        "write_file" => {
            let rel_path = input["path"].as_str().unwrap_or("");
            let content = input["content"].as_str().unwrap_or("");
            let full_path = match workspace_file_path(project_path, rel_path) {
                Ok(path) => path,
                Err(error) => return json!({ "error": error }),
            };
            let relative = full_path.strip_prefix(project_path.as_path()).unwrap_or(Path::new(rel_path));
            if crate::ai::compatibility::is_internal_project_file(relative) {
                return json!({ "error": "Internal project metadata is managed by workspace tools. Edit the firmware source; read shared rules with kidbright_reference instead of creating or modifying internal reference files." });
            }
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
            // Plan registration before touching a new source. Preserve dependencies
            // already proposed by this turn; accepting the CMake diff remains a
            // separate reviewed action rather than a silent configuration write.
            let source_plan = if !full_path.exists() && relative.starts_with("main") {
                let cmake = project_path.join("main/CMakeLists.txt");
                let canonical = dunce::canonicalize(&cmake).unwrap_or(cmake);
                let pending_map = match super::change_journal::with(|journal|Ok(journal.pending_map())) {
                    Ok(map)=>map,Err(error)=>return json!({"error":error}),
                };
                let pending = pending_map.iter()
                    .find(|(path, _)| dunce::canonicalize(path).is_ok_and(|path| path == canonical))
                    .map(|(_, content)| content.clone());
                match crate::esp_idf::source_list_plan_with_content(
                    project_path, &[relative.to_path_buf()], &[], pending.as_deref(),
                ) {
                    Ok(plan) => plan,
                    Err(error) => return json!({ "error": format!("Source registration was not changed and the new file was not written: {}. Read main/CMakeLists.txt and propose a compatible source list first.", error) }),
                }
            } else { None };
            if full_path.is_file() {
                match std::fs::read_to_string(&full_path) {
                    Ok(old) if old == content => return json!({"result":"File is already up to date."}),
                    Err(error) => return json!({"error":format!("Cannot read original file: {error}")}),
                    _ => (),
                }
            }
            let mut files=vec![(full_path.clone(),content.to_string())];
            if let Some((path,content))=source_plan {files.push((path,content));}
            if let Err(error)=super::repair::validate_proposal(project_path,message_id,&files){return json!({"error":error});}
            let changes=match super::change_journal::with(|journal|journal.propose_batch(project_path,message_id,&files)) {
                Ok(changes)=>changes,Err(error)=>return json!({"error":error}),
            };
            if let Err(error)=super::repair::record_proposal(app_handle,project_path,message_id,&changes){return json!({"error":error});}
            for change in changes {
                let _=app_handle.emit("ai-diff-pending",json!({"fullPath":change.path,"relPath":change.path.strip_prefix(&change.root).unwrap_or(&change.path),"revision":change.revision}).to_string());
            }
            if let Err(error)=super::project_review::emit_pending(app_handle,project_path) {return json!({"error":error});}
            json!({"result":"Changes saved for review, including new files and any CMake update. No source was written. The user must Keep each current revision or Keep the project before building."})
        }
        // ── list_files ─────────────────────────────────────────────────────────
        "list_files" => {
            let rel_path = input["path"].as_str().unwrap_or(".");
            let full_path = match workspace_file_path(project_path, rel_path) {
                Ok(path) => path,
                Err(error) => return json!({ "error": error }),
            };
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
            let full_path = match workspace_file_path(project_path, rel_path) {
                Ok(path) => path,
                Err(error) => return json!({ "error": error }),
            };
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
            let mut filter = match input.get("filter") {
                Some(value) => match serde_json::from_value::<crate::intelligence::SearchFilter>(value.clone()) {
                    Ok(value) => value,
                    Err(error) => return json!({"error":format!("Invalid knowledge filter: {error}")}),
                },
                None => crate::intelligence::SearchFilter { include_unknown: true, ..Default::default() },
            };
            filter.bundled_only = !private_context_allowed;
            crate::ai::kb::knowledge_search_filtered(app_handle, project_path, query, filter).await
        }

        "read_knowledge_file" => {
            let file_name = input["file_name"].as_str().unwrap_or("");
            let kb_path = resolve_kb_path(&project_path.to_string_lossy());
            let full_path = match workspace_file_path(&kb_path, file_name) {
                Ok(path) => path,
                Err(error) => return json!({ "error": error }),
            };
            match std::fs::read_to_string(&full_path) {
                Ok(content) if private_context_allowed || super::privacy::bundled_source(file_name,content.as_bytes()) => json!({ "result": content }),
                Ok(_) => json!({"error":super::privacy::REQUIRED}),
                Err(e) => json!({ 
                    "error": format!("Cannot read '{}' from knowledge_base: {}", file_name, e) 
                }),
            }
        }

        // ── create_project_workspace ───────────────────────────────────────────
        "create_project_workspace" => {
            let project_name = input["project_name"].as_str().unwrap_or("my_esp_project");
            let reference_files = match crate::ai::compatibility::requested_files(input) {
                Ok(files) => files,
                Err(error) => return json!({ "error": error }),
            };
            // Preserve the generated text in memory. Never export a later disk
            // read: it might include a user's concurrent edits or private paths.
            let generated_files = if reference_files.is_empty() {
                Some(starter_files(project_name).into_iter().map(|(path, text)| (path, json!(text))).collect::<serde_json::Map<String, Value>>())
            } else { None };
            if let Some(picked_dir) = rfd::FileDialog::new()
                .set_title("Select a Base Directory for Your New Project")
                .pick_folder()
            {
                let new_proj_path = match initialize_project_workspace(&picked_dir, project_name) {
                    Ok(path) => path,
                    Err(error) => return json!({ "error": error }),
                };
                for (relative, content) in reference_files {
                    let path = new_proj_path.join(relative);
                    let result = std::fs::create_dir_all(path.parent().unwrap())
                        .and_then(|_| std::fs::write(&path, content));
                    if let Err(error) = result {
                        return json!({ "error": format!("Reference project initialization failed: {}. Workspace was not switched. Do not write into the previous workspace.", error) });
                    }
                }
                if let Err(error) = crate::ai::compatibility::hide_internal_metadata(&new_proj_path) {
                    return json!({ "error": format!("Could not finish private project metadata: {}. Workspace was not switched.", error) });
                }
                let previous = if *no_workspace { None } else { Some(project_path.clone()) };
                let review = match crate::ai::project_review::register_created_project(&new_proj_path, previous, message_id) {
                    Ok(review) => review,
                    Err(error) => return json!({ "error": format!("Could not register project review: {}. Workspace was not switched. Do not write into the previous workspace.", error) }),
                };
                // Switch only after the files and their Undo snapshot are ready.
                *project_path = new_proj_path.clone();
                *no_workspace = false;
                let _ = app_handle.emit("force-project-dir", new_proj_path.to_string_lossy().to_string());
                let _ = app_handle.emit("ai-project-review-pending", serde_json::to_string(&review).unwrap());
                let reported_workspace=if private_context_allowed{new_proj_path.to_string_lossy().into_owned()}else{".".into()};
                json!({
                    "result": format!(
                        "Created project workspace for review: {}. Reference files (when selected) and boilerplate are ready. For initial custom proposals use the literal generated_files returned here; reference recipes are available via kidbright_reference. These are creation-time contents, not a read of current private files. Preserve the confirmed board and timing. The user must choose Keep or Undo in the project review card. Do not run commands, build or flash while this review is pending.",
                        reported_workspace
                    ),
                    "workspace_path": reported_workspace,
                    "generated_files": generated_files,
                    "review_pending": true
                })
            } else {
                json!({ "error": "User cancelled project creation. Stop this creation request; do not write the requested new project into the previous workspace." })
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
                        "hint": "AI run_command supports idf.py build, idf.py size, and idf.py --version through the configured SDK. The user operates Flash/Monitor/Clean in the application."
                    }
                })
            } else {
                json!({ "error": "ESP-IDF paths are not resolved in this runtime." })
            }
        }

        // ── install_idf_library ────────────────────────────────────────────────
        "install_idf_library" => {
            let component=input["component"].as_str().unwrap_or("").trim();
            let (path,content)=match dependency_proposal(project_path,component){Ok(file)=>file,Err(error)=>return json!({"error":error})};
            let files=vec![(path,content)];
            if let Err(error)=super::repair::validate_proposal(project_path,message_id,&files){return json!({"error":error});}
            let changes=match super::change_journal::with(|j|j.propose_batch(project_path,message_id,&files)){Ok(c)=>c,Err(error)=>return json!({"error":error})};
            for change in changes {let _=app_handle.emit("ai-diff-pending",json!({"fullPath":change.path,"revision":change.revision}).to_string());}
            if let Err(error)=super::project_review::emit_pending(app_handle,project_path){return json!({"error":error});}
            json!({"result":"Dependency manifest proposed for review. No SDK subprocess or dependency installation was started. User must Keep the manifest, then run the configured build to resolve the dependency. Existing YAML requires a full reviewed write_file proposal."})
        }

        "run_command" => {
            use tauri::Manager;
            if let Err(error) = crate::esp_idf::ensure_project_reviewed(project_path) {
                return json!({ "error": error });
            }
            let command = input["command"].as_str().unwrap_or("");
            let action = match allowed_ai_idf_action(command) {
                Ok(action) => action,
                Err(error) => return json!({"error":error}),
            };
            // Reuse the configured, argv-based SDK route and all existing review/
            // profile gates. Raw model output never reaches cmd /C or sh -c.
            let abort=app_handle.try_state::<crate::ai_chat::AiAbortState>().map(|s|s.0.clone()).unwrap_or_else(||std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
            match crate::sdk_process::until_cancelled(crate::esp_idf::run_idf_command(
                app_handle.clone(), action.into(), vec![],
                Some(project_path.to_string_lossy().into_owned()),
            ),abort).await {
                Ok(output) => json!({"result":output,"verification":"command-completed; hardware behavior not verified"}),
                Err(error) => json!({"error":error,"verification":"build-not-verified"}),
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
        let path = entry.path();
        if path.strip_prefix(root).ok().and_then(|relative| workspace_file_path(root, &relative.to_string_lossy()).ok()).is_none() { continue; }
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
        if path.strip_prefix(root).ok().and_then(|relative| workspace_file_path(root, &relative.to_string_lossy()).ok()).is_none() { continue; }
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

#[cfg(test)]
mod workspace_tests {
    use super::{allowed_ai_idf_action, command_result, initialize_project_workspace, workspace_file_path};

    #[test]
    fn remote_sdk_context_is_reference_only_and_private_tools_stay_blocked() {
        let result = super::private_context_guard("esp_idf_info", false).unwrap();
        assert_eq!(result["result"]["installed_sdk_inspected"], false);
        assert_eq!(result["result"]["context_kind"], "bundled_reference_only");
        assert_eq!(result["result"]["reference_sdk_series"], "5.5");
        for key in ["idf_path", "tools_path", "python", "version", "project_dir"] {
            assert!(result["result"].get(key).is_none(), "Leaked {key}");
        }
        for tool in ["read_file", "list_files", "get_file_tree", "search_in_files", "project_search", "compiler_diagnostics", "diff_file", "run_command", "install_idf_library", "web_search"] {
            assert_eq!(super::private_context_guard(tool, false).unwrap()["error"], crate::ai::privacy::REQUIRED);
            assert!(super::private_context_guard(tool, true).is_none());
        }
        assert!(super::private_context_guard("esp_idf_info", true).is_none());
        for tool in ["create_project_workspace", "write_file", "kidbright_reference"] {
            assert!(super::private_context_guard(tool, false).is_none());
        }
    }

    #[test]
    fn custom_project_has_exact_public_starter_without_exporting_later_edits() {
        let base = tempfile::tempdir().unwrap();
        let root = initialize_project_workspace(base.path(), "ip_scroll").unwrap();
        let generated = super::starter_files("ip_scroll");
        assert_eq!(generated.len(), 4);
        for (relative, text) in &generated {
            assert_eq!(std::fs::read_to_string(root.join(relative)).unwrap(), *text);
            assert!(!text.contains(&base.path().to_string_lossy().to_string()));
        }
        std::fs::write(root.join("main/main.c"), "private concurrent user edit").unwrap();
        assert_eq!(super::starter_files("ip_scroll"), generated);
        assert!(!serde_json::to_string(&generated).unwrap().contains("private concurrent user edit"));
        // No unsupported board/profile substitution is necessary to create it.
        assert!(crate::ai::compatibility::requested_files(&serde_json::json!({"project_name":"ip_scroll"})).unwrap().is_empty());
        assert!(crate::ai::compatibility::requested_files(&serde_json::json!({"board_profile":"kidbright32_ip","reference_example":"matrix_counter"})).is_err());
    }

    #[test]
    fn file_tools_reject_escape_paths_and_allow_new_nested_sources() {
        let root = tempfile::tempdir().unwrap();
        for relative in ["../outside.c", "main/../../outside.c"] {
            assert!(workspace_file_path(root.path(), relative).is_err());
        }
        assert!(workspace_file_path(root.path(), &root.path().join("absolute.c").to_string_lossy()).is_err());
        assert_eq!(workspace_file_path(root.path(), "main/tasks/new.c").unwrap(), root.path().join("main/tasks/new.c"));
        assert!(!root.path().join("main").exists());
        assert!(workspace_file_path(root.path(), ".").is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn file_tools_reject_windows_aliases_and_device_names() {
        let root = tempfile::tempdir().unwrap();
        for path in ["main/../escape.c", "main/.. /escape.c", "main/source.c:stream", "main/NUL.c", "main/CON", "main/COM1.h", "main/file.c."] {
            assert!(workspace_file_path(root.path(), path).is_err(), "Accepted {path}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn file_tools_reject_links_to_external_files_and_new_children() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("external")).unwrap();
        assert!(workspace_file_path(root.path(), "external/new.c").is_err());
        std::fs::write(outside.path().join("secret.txt"), "private").unwrap();
        assert!(workspace_file_path(root.path(), "external/secret.txt").is_err());
    }

    #[test]
    fn failed_ai_command_reports_failure_even_without_output() {
        let result = command_result(false, Some(2), b"", b"");
        assert!(result.get("result").is_none());
        assert_eq!(result["exit_code"], 2);
        assert!(result["error"].as_str().unwrap().contains("exit code 2"));
        let result = command_result(false, Some(1), b"compiler output", b"build failed");
        assert!(result["error"].as_str().unwrap().contains("build failed"));
    }

    #[test]
    fn ai_command_output_truncation_keeps_unicode_boundaries() {
        let text = format!("{}{}", "a".repeat(9_999), "ไทย".repeat(10));
        let result = command_result(true, Some(0), text.as_bytes(), b"");
        let text = result["result"].as_str().unwrap();
        assert!(text.starts_with(&"a".repeat(9_999)));
        assert!(text.ends_with("(output truncated)"));
        assert_eq!(command_result(true, Some(0), b"", b"")["result"], "Command completed successfully (no output)");
    }

    #[test]
    fn model_command_policy_rejects_shell_injection_and_hardware_actions() {
        assert_eq!(allowed_ai_idf_action("idf.py build").unwrap(),"build");
        assert_eq!(allowed_ai_idf_action("idf.py --version").unwrap(),"--version");
        for command in ["idf.py build && whoami", "idf.py build; rm x", "idf.py build\nwhoami", "idf.py flash", "idf.py fullclean", "idf.py -C ../outside build", "python x.py", "cmd /C dir", "idf.py build > secret.txt"] {
            assert!(allowed_ai_idf_action(command).is_err(),"accepted {command}");
        }
    }

    #[test]
    fn creation_with_existing_project_uses_a_fresh_directory() {
        let base = tempfile::tempdir().unwrap();
        let old = initialize_project_workspace(base.path(), "old_project").unwrap();
        std::fs::write(old.join("main/main.c"), "existing firmware").unwrap();

        let new = initialize_project_workspace(base.path(), "new_project").unwrap();

        assert_ne!(old, new);
        assert_eq!(std::fs::read_to_string(old.join("main/main.c")).unwrap(), "existing firmware");
        for file in ["CMakeLists.txt", "main/CMakeLists.txt", "main/main.c", "sdkconfig"] {
            assert!(new.join(file).is_file(), "Missing {}", file);
        }
        assert!(!new.join("build").exists());
    }

    #[test]
    fn duplicate_project_name_does_not_overwrite_files() {
        let base = tempfile::tempdir().unwrap();
        let old = initialize_project_workspace(base.path(), "same_name").unwrap();
        std::fs::write(old.join("main/main.c"), "existing firmware").unwrap();

        assert!(initialize_project_workspace(base.path(), "same_name").is_err());
        assert_eq!(std::fs::read_to_string(old.join("main/main.c")).unwrap(), "existing firmware");
    }

    #[test]
    fn project_name_cannot_escape_or_nest_inside_base_directory() {
        let base = tempfile::tempdir().unwrap();
        for name in ["", ".", "..", "../escape", "nested/project", "nested\\project", "C:\\escape"] {
            assert!(initialize_project_workspace(base.path(), name).is_err(), "Accepted {}", name);
        }
        assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 0);
    }
    #[test]
    fn dependency_changes_are_proposals_and_preserve_existing_manifest_data(){
        let base=tempfile::tempdir().unwrap();std::fs::create_dir(base.path().join("main")).unwrap();
        let (path,content)=super::dependency_proposal(base.path(),"espressif/led_strip^2.5.3").unwrap();assert!(!path.exists());
        let value:serde_json::Value=serde_json::from_str(&content).unwrap();assert_eq!(value["dependencies"]["espressif/led_strip"],"^2.5.3");
        std::fs::write(&path,"dependencies:\n  idf: '>=5.5'\n").unwrap();assert!(super::dependency_proposal(base.path(),"espressif/led_strip").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(),"dependencies:\n  idf: '>=5.5'\n");
        for bad in ["../../escape","name;calc","-x","espressif/led_strip & calc"]{assert!(super::dependency_proposal(base.path(),bad).is_err());}
    }
}
