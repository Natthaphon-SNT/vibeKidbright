//! Diagnostic classification is evidence, never an instruction to run a command.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub severity: String,
    pub category: String,
    pub message: String,
}
pub fn parse(output: &str) -> Vec<Diagnostic> {
    let ansi = regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap();
    let gcc=regex::Regex::new(r"^(.+?\.(?:c|h|cpp|cc|cxx|hpp|s|S)):(\d+):(?:(\d+):)?\s*(fatal error|error|warning|note):\s*(.*)$").unwrap();
    let cmake =
        regex::Regex::new(r"^CMake Error at (.+?):(\d+)(?:\s*\([^)]*\))?:\s*(.*)$").unwrap();
    let mut diagnostics = Vec::new();
    for raw in output.lines().take(20000) {
        let line = ansi.replace_all(raw, "");
        let text = line.trim();
        let mut diagnostic = if let Some(c) = gcc.captures(text) {
            let message = c[5].to_owned();
            Diagnostic {
                file: Some(c[1].into()),
                line: c[2].parse().ok(),
                column: c.get(3).and_then(|v| v.as_str().parse().ok()),
                severity: if &c[4] == "fatal error" {
                    "error".into()
                } else {
                    c[4].into()
                },
                category: if message.contains("No such file or directory") {
                    "dependency".into()
                } else {
                    "compiler".into()
                },
                message,
            }
        } else if let Some(c) = cmake.captures(text) {
            Diagnostic {
                file: Some(c[1].into()),
                line: c[2].parse().ok(),
                column: None,
                severity: "error".into(),
                category: "cmake".into(),
                message: c[3].into(),
            }
        } else if text.contains("undefined reference") || text.contains("multiple definition") {
            Diagnostic {
                file: None,
                line: None,
                column: None,
                severity: "error".into(),
                category: "linker".into(),
                message: text.into(),
            }
        } else if text.contains("CMake Error")
            || text.contains("fatal error:")
            || text.contains("ninja: build stopped")
            || text.contains("Failed to resolve component")
        {
            Diagnostic {
                file: None,
                line: None,
                column: None,
                severity: "error".into(),
                category: "build_system".into(),
                message: text.into(),
            }
        } else {
            continue;
        };
        diagnostic.message = diagnostic.message.chars().take(4000).collect();
        diagnostics.push(diagnostic);
        if diagnostics.len() >= 100 {
            break;
        }
    }
    diagnostics
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_paths_unicode_ansi_and_warning_severity_are_preserved() {
        let d=parse("\x1b[31mD:/lesson ไทย/main/main.c:12:7: error: expected ';'\x1b[0m\nC:\\lesson\\main.c:13: warning: unused variable\nnoise");
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].file.as_deref(), Some("D:/lesson ไทย/main/main.c"));
        assert_eq!(d[0].column, Some(7));
        assert_eq!(d[1].severity, "warning");
    }
    #[test]
    fn build_phases_do_not_guess_source_paths_from_linker_text() {
        let d=parse("main.c:2: fatal error: missing.h: No such file or directory\nCMake Error at main/CMakeLists.txt:3 (idf_component_register):\nobj/main.o: undefined reference to `missing'\n");
        assert_eq!(d[0].category, "dependency");
        assert_eq!(d[1].category, "cmake");
        assert_eq!(d[2].category, "linker");
        assert!(d[2].file.is_none());
    }
}
