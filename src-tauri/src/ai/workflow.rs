//! Creation requests must select a fresh workspace before touching user files.
use crate::ai_chat::ChatMessage;
use std::path::{Path, PathBuf};

pub(crate) fn requests_new_project(messages: &[ChatMessage]) -> bool {
    let Some(message) = messages.iter().rev().find(|m| m.role == "user") else { return false; };
    let Some(text) = message.content.as_str() else { return false; };
    let mut fenced = false;
    let prose = text.lines().filter_map(|line| {
        let line = line.trim();
        if line.starts_with("```") || line.starts_with("~~~") { fenced = !fenced; return None; }
        (!fenced && !line.starts_with('>')).then_some(line)
    }).collect::<Vec<_>>().join(" ").to_lowercase();
    if ["don't create", "do not create", "don't start", "do not start", "ไม่ต้องสร้าง", "อย่าสร้าง", "ห้ามสร้าง", "why ", "explain ", "ทำไม", "อธิบาย"].iter().any(|s| prose.contains(s)) {
        return false;
    }
    ["create", "start", "generate", "make", "build", "สร้าง", "เริ่ม", "ทำโปรเจ"].iter().any(|s| prose.contains(s))
        && ["project", "โปรเจกต์", "โปรเจคต์", "โปรเจค", "โปรเจก"].iter().any(|s| prose.contains(s))
        && !["rebuild", "build the current", "build this project", "existing project", "โปรเจคต์เดิม", "โปรเจกต์เดิม"].iter().any(|s| prose.contains(s))
}

pub(crate) fn workspace_tool_blocked(name: &str, no_workspace: bool) -> bool {
    no_workspace && matches!(name, "write_file" | "run_command" | "read_file" | "list_files" | "get_file_tree" | "search_in_files" | "diff_file" | "install_idf_library")
}

pub(crate) fn initialize_project(base: &Path, name: &str) -> Result<PathBuf, String> {
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
    if name.is_empty() || name.len() > 80 || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-') || reserved.contains(&name.to_ascii_uppercase().as_str()) {
        return Err("Choose a project name using ASCII letters, digits, underscores or hyphens.".into());
    }
    let path = base.join(name);
    std::fs::create_dir(&path).map_err(|e| format!("Cannot create fresh project: {e}. Choose a different name; do not use the previous workspace."))?;
    let result = (|| -> Result<(), std::io::Error> {
        std::fs::create_dir(path.join("main"))?;
        std::fs::write(path.join("CMakeLists.txt"), format!("cmake_minimum_required(VERSION 3.16)\ninclude($ENV{{IDF_PATH}}/tools/cmake/project.cmake)\nproject({name})\n"))?;
        std::fs::write(path.join("main/CMakeLists.txt"), "idf_component_register(SRCS \"main.c\" INCLUDE_DIRS \".\")\n")?;
        std::fs::write(path.join("main/main.c"), "#include <stdio.h>\nvoid app_main(void) { printf(\"Hello\\n\"); }\n")?;
        std::fs::write(path.join("sdkconfig.defaults"), "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n")?;
        Ok(())
    })();
    result.map_err(|e| format!("Project initialization incomplete: {e}. The previous project was not changed."))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn user(text: &str) -> ChatMessage { ChatMessage { role: "user".into(), content: json!(text) } }
    #[test]
    fn explicit_creation_is_fresh_even_with_an_open_workspace() {
        for text in ["Create a new project to blink the Wi-Fi LED", "ช่วยสร้างโปรเจคต์ใหม่ให้ไฟ Wi-Fi กับ IoT กระพริบ", "สร้างโปรเจกต์อ่านอุณหภูมิ"] {
            assert!(requests_new_project(&[user(text)]), "{text}");
            for tool in ["write_file", "read_file", "run_command", "install_idf_library"] { assert!(workspace_tool_blocked(tool, true)); }
            assert!(!workspace_tool_blocked("create_project_workspace", true));
            assert!(!workspace_tool_blocked("knowledge_search", true));
        }
    }
    #[test]
    fn edits_negations_explanations_and_reference_text_do_not_create_projects() {
        for text in ["Change the blink delay in the existing project", "Build this project", "ไม่ต้องสร้างโปรเจคต์ใหม่ แก้ main.c", "Don't create a new project; edit main.c", "Explain how to create a new project", "> Create a new project\nแก้ main.c", "```c\n// create new project\n```\nFix this code"] {
            assert!(!requests_new_project(&[user(text)]), "{text}");
        }
        assert!(!requests_new_project(&[user("Create a new project"), user("Change GPIO2 to GPIO12")]));
    }
    #[test]
    fn duplicate_and_invalid_names_never_overwrite_a_project() {
        let temp = tempfile::tempdir().unwrap();
        let old = initialize_project(temp.path(), "old_project").unwrap();
        let original = std::fs::read(old.join("main/main.c")).unwrap();
        assert!(initialize_project(temp.path(), "old_project").is_err());
        for name in ["..", "../escape", "a/b", "C:\\escape", "NUL", "a.c", ""] { assert!(initialize_project(temp.path(), name).is_err()); }
        let new = initialize_project(temp.path(), "new_project").unwrap();
        assert!(new.join("sdkconfig.defaults").is_file());
        assert_eq!(std::fs::read(old.join("main/main.c")).unwrap(), original);
    }
}
