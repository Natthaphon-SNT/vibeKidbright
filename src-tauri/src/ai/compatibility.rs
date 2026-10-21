//! Versioned, deterministic KidBright pilot recipes shared by AI and build checks.
use serde_json::{json, Value};
use std::path::Path;

pub(crate) const PROJECT_METADATA_PATH: &str = ".vibekidbright/project.json";
pub(crate) const RULES: &str = include_str!("../../../knowledge_base/compatibility/README.md");
pub(crate) const SUPPORT_SCOPE: &str = include_str!("../../../knowledge_base/support_scope.md");
const CORE_H: &str = include_str!("../../../knowledge_base/compatibility/kb_core.h");
const CORE_C: &str = include_str!("../../../knowledge_base/compatibility/kb_core.c");
const BUTTON: &str = include_str!("../../../knowledge_base/compatibility/button_buzzer.c");
const TEMPERATURE: &str =
    include_str!("../../../knowledge_base/compatibility/temperature_serial.c");
const BASIC_H: &str = include_str!("../../../knowledge_base/compatibility/kb_basic.h");
const BASIC_C: &str = include_str!("../../../knowledge_base/compatibility/kb_basic.c");
const DISPLAY_H: &str = include_str!("../../../knowledge_base/compatibility/kb_display_idf.h");
const DISPLAY_C: &str = include_str!("../../../knowledge_base/compatibility/kb_display_idf.c");
pub(crate) const EXAMPLE_IDS: &[&str] = &[
    "button_buzzer",
    "temperature_serial",
    "music_scale",
    "matrix_counter",
];

fn profile(id: &str) -> Result<Value, String> {
    match id {
        "kidbright32_v13" | "kidbright32_rev31" => Ok(json!({
            "id": id, "target": "esp32", "sw1": 16, "buzzer": 13,
            "sensor_sda": 4, "sensor_scl": 5, "lm73_address": 77,
            "scope": "S1/buzzer/LM73 and music software pilot; matrix_counter only on v1.3; no physical equivalence or other peripherals certified"
        })),
        _ => Err(format!("Unsupported board_profile '{}'. Confirm the board; supported pilot profiles: kidbright32_v13, kidbright32_rev31. Do not substitute another board.", id)),
    }
}

pub(crate) fn files(board: &str, example: &str) -> Result<Vec<(String, String)>, String> {
    let board_info = profile(board)?;
    let main = match example {
        "button_buzzer" => BUTTON,
        "temperature_serial" => TEMPERATURE,
        "music_scale" => include_str!("../../../knowledge_base/compatibility/music_scale.c"),
        "matrix_counter" if board == "kidbright32_v13" => include_str!("../../../knowledge_base/compatibility/matrix_counter.c"),
        "matrix_counter" => return Err("matrix_counter currently requires confirmed kidbright32_v13. The provided v1.5 design has a revision identity gap; do not substitute a profile.".into()),
        _ => {
            return Err(format!(
                "Unsupported reference_example '{}'. Read kidbright_reference for the current recipe catalog.",
                example
            ))
        }
    };
    let mut header = format!(
        "#ifndef KB_BOARD_H\n#define KB_BOARD_H\n/* Explicit pilot profile: {} */\n#define KB_SW1_GPIO {}\n#define KB_BUZZER_GPIO {}\n#define KB_SENSOR_SDA {}\n#define KB_SENSOR_SCL {}\n#define KB_LM73_ADDRESS {}\n#endif\n",
        board, board_info["sw1"], board_info["buzzer"], board_info["sensor_sda"],
        board_info["sensor_scl"], board_info["lm73_address"]
    );
    if example == "matrix_counter" {
        header = header.replace("#endif\n", "/* Official v1.3 schematic: SDA0/SCL0. Not inferred from MCU type. */\n#define KB_MATRIX_SDA 21\n#define KB_MATRIX_SCL 22\n#define KB_MATRIX_ADDRESS 0x70\n#endif\n");
    }
    let manifest = json!({
        "schema_version": 1, "board_profile": board, "reference_example": example,
        "target": "esp32", "sdk_series": "5.5", "protocol": "none",
        "source": "KidBright Desktop 1.7 generator contracts; read rules with kidbright_reference",
        "validation": "software-tested; physical board behavior not yet verified"
    });
    let mut output = vec![
        ("main/main.c".into(), main.into()),
        ("main/kb_board.h".into(), header),
        ("main/CMakeLists.txt".into(), "idf_component_register(\n    SRCS\n    # VIBEKIDBRIGHT_MANAGED_SOURCES_BEGIN\n    \"main.c\"\n    # VIBEKIDBRIGHT_MANAGED_SOURCES_END\n    INCLUDE_DIRS \".\"\n    PRIV_REQUIRES driver esp_driver_i2c kb_reference\n)\n".into()),
        ("components/kb_reference/CMakeLists.txt".into(), "idf_component_register(SRCS \"kb_core.c\" INCLUDE_DIRS \"include\")\n".into()),
        ("components/kb_reference/include/kb_core.h".into(), CORE_H.into()),
        ("components/kb_reference/kb_core.c".into(), CORE_C.into()),
        (PROJECT_METADATA_PATH.into(), serde_json::to_string_pretty(&manifest).unwrap() + "\n"),
    ];
    if matches!(example, "music_scale" | "matrix_counter") {
        output.push((
            "components/kb_reference/include/kb_basic.h".into(),
            BASIC_H.into(),
        ));
        output.push(("components/kb_reference/kb_basic.c".into(), BASIC_C.into()));
        let component = output
            .iter_mut()
            .find(|(path, _)| path == "components/kb_reference/CMakeLists.txt")
            .unwrap();
        component.1 =
            "idf_component_register(SRCS \"kb_core.c\" \"kb_basic.c\" INCLUDE_DIRS \"include\")\n"
                .into();
        if example == "matrix_counter" {
            component.1 = "idf_component_register(SRCS \"kb_core.c\" \"kb_basic.c\" \"kb_display_idf.c\" INCLUDE_DIRS \"include\" REQUIRES esp_driver_i2c)\n".into();
            output.push((
                "components/kb_reference/include/kb_display_idf.h".into(),
                DISPLAY_H.into(),
            ));
            output.push((
                "components/kb_reference/kb_display_idf.c".into(),
                DISPLAY_C.into(),
            ));
        }
    }
    Ok(output)
}

/// Validate before showing a picker or writing anything to disk.
pub(crate) fn requested_files(input: &Value) -> Result<Vec<(String, String)>, String> {
    match (input.get("board_profile"), input.get("reference_example")) {
        (None, None) => Ok(vec![]),
        (Some(board), Some(example)) => files(
            board.as_str().ok_or("board_profile must be a string")?,
            example.as_str().ok_or("reference_example must be a string")?,
        ),
        _ => Err("For a reference project, provide both board_profile and reference_example. Confirm the board first.".into()),
    }
}

pub(crate) fn reference(input: &Value) -> Value {
    let recipes = match requested_files(input) {
        Ok(files) => files,
        Err(error) => return json!({ "error": error }),
    };
    json!({
        "rules": RULES,
        "support_scope": SUPPORT_SCOPE,
        "board_catalog": serde_json::from_str::<Value>(include_str!("../../../knowledge_base/compatibility/board_catalog.json")).expect("validated bundled board catalog"),
        "development_scope": "Complete classic KidBright support first. KidBright muAI and muAI Plus are future work, excluded from active implementation and deployment. A catalog entry is not an executable or physically verified board profile.",
        "board_profiles": [profile("kidbright32_v13").unwrap(), profile("kidbright32_rev31").unwrap()],
        "examples": [
            { "id": "button_buzzer", "behavior": "wait S1 pressed -> A4 440 Hz quarter note 500 ms -> wait released" },
            { "id": "temperature_serial", "behavior": "read LM73 signed raw /128 every 500 ms; log temperature or explicit read error" },
            { "id": "music_scale", "behavior": "S1 -> C4 to C5 MIDI notes at 120 BPM quarter notes -> wait release; software implementation, pending physical measurement" },
            { "id": "matrix_counter", "boards": ["kidbright32_v13"], "behavior": "new 5x7 numeric font displays 00..99 every 500 ms; classic HT16K33 wiring, pending physical orientation verification; no original font/scroll equivalence" }
        ],
        "creation": "Call create_project_workspace with project_name, board_profile and reference_example to install these exact files into a NEW project. For an existing project read_file first and propose changes with write_file.",
        "files": recipes.into_iter().map(|(path, content)| json!({ "path": path, "content": content })).collect::<Vec<_>>(),
        "limitations": "Scoped software recipes only. Matrix/music helpers are not complete original runtime/font/scheduler/block adapters. No physical certification, motors, radio interoperability, Blockly import or muAI deployment. Never guess a missing board or switch ASCII/binary protocols."
    })
}

/// Re-read for each provider request: a tool may have switched the workspace.
pub(crate) fn with_project_context(base_prompt: &str, path: &Path, private_allowed:bool) -> String {
    if !private_allowed {return base_prompt.to_owned();}
    format!("{}{}", base_prompt, project_context(path))
}

/// Enforce classic-ESP32 scope before configuring or flashing. Generic legacy
/// projects remain usable without adding private metadata files.
pub(crate) fn validate_build_profile(path: &Path) -> Result<(), String> {
    if project_context(path).contains("[PROJECT PROFILE ERROR]") {
        return Err("The KidBright project profile is invalid, unknown or inconsistent with its recipe. Confirm the board before build; no fallback profile was selected.".into());
    }
    let private = path.join(PROJECT_METADATA_PATH);
    let manifest = if private.exists() {
        private
    } else {
        path.join("kidbright-project.json")
    };
    if manifest.exists() {
        let value: Value = serde_json::from_str(
            &std::fs::read_to_string(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if value.get("target").is_some_and(|target| target != "esp32") {
            return Err("This KidBright build path currently supports classic ESP32 profiles only. muAI and muAI Plus remain future work.".into());
        }
    }
    for name in ["sdkconfig", "sdkconfig.defaults"] {
        match std::fs::read_to_string(path.join(name)) {
            Ok(text) => {
                for line in text.lines() {
                    if let Some(target) = line.trim().strip_prefix("CONFIG_IDF_TARGET=") {
                        if target.trim().trim_matches('"') != "esp32" {
                            return Err(format!("{} declares target {}. Do not build this project through the classic ESP32 KidBright path.", name, target));
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Cannot verify {}: {}", name, error)),
        }
    }
    Ok(())
}

/// Private metadata is still portable with the project, but is not a code file.
pub(crate) fn is_internal_project_file(relative: &Path) -> bool {
    if relative.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".vibekidbright")
    }) {
        return true;
    }
    // Tool paths may contain `main/../`; apply the root-file policy to their
    // normalized location too. Preserve leading parents rather than treating
    // paths outside the project as project-root files.
    let mut normalized = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
                if matches!(normalized.last(), Some(std::path::Component::Normal(_))) =>
            {
                normalized.pop();
            }
            _ => normalized.push(component),
        }
    }
    normalized.len() == 1
        && normalized.last().is_some_and(|part| {
            let std::path::Component::Normal(name) = part else {
                return false;
            };
            let name = name.to_string_lossy();
            name.eq_ignore_ascii_case("kidbright-project.json")
                || name.eq_ignore_ascii_case("KIDBRIGHT_COMPATIBILITY.md")
        })
}

/// Dot directories are hidden automatically on Unix. Windows needs its native
/// Hidden attribute too, so the default Explorer view stays focused on code.
pub(crate) fn hide_internal_metadata(project: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::os::windows::fs::MetadataExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn SetFileAttributesW(path: *const u16, attributes: u32) -> i32;
        }
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        let directory = project.join(".vibekidbright");
        let metadata = match std::fs::symlink_metadata(&directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Private metadata directory must not be a link",
            ));
        }
        let wide: Vec<u16> = directory
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // The buffer is NUL-terminated and lives through the synchronous Win32 call.
        if unsafe {
            SetFileAttributesW(
                wide.as_ptr(),
                metadata.file_attributes() | FILE_ATTRIBUTE_HIDDEN,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(not(windows))]
    let _ = project;
    Ok(())
}

pub(crate) fn project_context(path: &Path) -> String {
    const PROFILE_ERROR: &str = "\n[PROJECT PROFILE ERROR] Invalid or unknown KidBright metadata. Ask for the correct board/profile; do not fall back to another board.\n";
    let private = path.join(PROJECT_METADATA_PATH);
    let manifest = match std::fs::symlink_metadata(&private) {
        Ok(_) => private,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            path.join("kidbright-project.json")
        }
        Err(_) => return PROFILE_ERROR.into(),
    };
    match std::fs::symlink_metadata(&manifest) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return String::new(),
        Ok(metadata)
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= 8192 => {}
        _ => return PROFILE_ERROR.into(),
    }
    let value = std::fs::read_to_string(&manifest)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let Some(value) = value else {
        return PROFILE_ERROR.into();
    };
    let board = value["board_profile"].as_str().unwrap_or_default();
    let example = value["reference_example"].as_str().unwrap_or_default();
    let valid_example = if example=="typed_block_pilot" {value["block_mapping_schema"]==1&&profile(board).is_ok()}else{files(board,example).is_ok()};
    if value["schema_version"] != 1 || !valid_example {
        return PROFILE_ERROR.into();
    }
    // Only validated identifiers are inserted, never arbitrary manifest prose.
    let manifest_name = manifest
        .strip_prefix(path)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    format!("\n[CURRENT KIDBRIGHT PROJECT]\nBoard profile: {}. Initial reference example: {}. Read {} and main/kb_board.h before edits; keep the confirmed pins. Private metadata is not a learner-editable source file; do not copy reference Markdown into the project. Read shared rules with kidbright_reference. No Formula Kid or minibike motor wiring is implied by this profile. Protocol changes require an explicit matching sender/receiver contract.\n", board, example, manifest_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_reject_unknown_profiles_and_nonclassic_targets_without_rewriting_them() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("sdkconfig");
        std::fs::write(&config, "CONFIG_IDF_TARGET=\"esp32s3\"\n").unwrap();
        assert!(validate_build_profile(directory.path()).is_err());
        assert!(std::fs::read_to_string(&config)
            .unwrap()
            .contains("esp32s3"));
        std::fs::write(&config, "CONFIG_IDF_TARGET=\"esp32\"\n").unwrap();
        assert!(validate_build_profile(directory.path()).is_ok());
        let private = directory.path().join(PROJECT_METADATA_PATH);
        std::fs::create_dir_all(private.parent().unwrap()).unwrap();
        for (board, recipe) in [
            ("kidbright32_v16", "music_scale"),
            ("kidbright32_rev31", "matrix_counter"),
        ] {
            std::fs::write(
                &private,
                json!({"schema_version":1,"board_profile":board,"reference_example":recipe})
                    .to_string(),
            )
            .unwrap();
            assert!(validate_build_profile(directory.path()).is_err());
        }
        std::fs::write(&private, json!({"schema_version":1,"board_profile":"kidbright32_v13","reference_example":"music_scale","target":"esp32"}).to_string()).unwrap();
        assert!(validate_build_profile(directory.path()).is_ok());
    }

    #[test]
    fn no_board_fallback_or_half_selected_recipe() {
        assert!(files("kidbright32_v16", "button_buzzer").is_err());
        assert!(files("kidbright32_v13", "unknown").is_err());
        assert!(requested_files(&json!({"reference_example":"button_buzzer"})).is_err());
        assert!(
            requested_files(&json!({"board_profile":42,"reference_example":"button_buzzer"}))
                .is_err()
        );
        assert!(requested_files(&json!({})).unwrap().is_empty());
    }

    #[test]
    fn reference_projects_use_c_and_isolated_component_paths() {
        for board in ["kidbright32_v13", "kidbright32_rev31"] {
            for &example in EXAMPLE_IDS {
                if example == "matrix_counter" && board != "kidbright32_v13" {
                    assert!(files(board, example).is_err());
                    continue;
                }
                let files = files(board, example).unwrap();
                let manifest: Value = serde_json::from_str(
                    &files
                        .iter()
                        .find(|(p, _)| p == PROJECT_METADATA_PATH)
                        .unwrap()
                        .1,
                )
                .unwrap();
                assert_eq!(manifest["board_profile"], board);
                assert_eq!(manifest["protocol"], "none");
                assert!(!files
                    .iter()
                    .any(|(path, _)| path.ends_with(".md") || path == "kidbright-project.json"));
                for (path, _) in files {
                    assert!(Path::new(&path).is_relative());
                }
            }
        }
    }

    #[test]
    fn compared_boards_are_separate_from_executable_profiles_and_defer_ai_boards() {
        let result = reference(&json!({}));
        let catalog = &result["board_catalog"];
        assert_eq!(catalog["schema_version"], 1);
        assert!(catalog["boards"]
            .as_array()
            .is_some_and(|boards| !boards.is_empty()));
        assert_eq!(result["board_profiles"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["examples"].as_array().unwrap().len(),
            EXAMPLE_IDS.len()
        );
        assert!(result["development_scope"]
            .as_str()
            .unwrap()
            .contains("future work"));
        assert!(files("kidbright32_v16", "music_scale").is_err());
        assert!(files("kidbright_uai", "music_scale").is_err());
    }

    #[test]
    fn private_profile_survives_hidden_attributes_and_takes_precedence_over_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join(PROJECT_METADATA_PATH);
        std::fs::create_dir_all(private.parent().unwrap()).unwrap();
        std::fs::write(&private, json!({
            "schema_version":1,"board_profile":"kidbright32_rev31","reference_example":"button_buzzer"
        }).to_string()).unwrap();
        std::fs::write(dir.path().join("kidbright-project.json"), json!({
            "schema_version":1,"board_profile":"kidbright32_v13","reference_example":"button_buzzer"
        }).to_string()).unwrap();
        hide_internal_metadata(dir.path()).unwrap();
        let context = project_context(dir.path());
        assert!(context.contains("kidbright32_rev31"));
        assert!(context.contains(PROJECT_METADATA_PATH));
        assert!(!context.contains("kidbright32_v13"));
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            assert_ne!(
                std::fs::metadata(private.parent().unwrap())
                    .unwrap()
                    .file_attributes()
                    & 0x2,
                0
            );
        }
    }

    #[test]
    fn invalid_private_profile_cannot_fall_back_to_a_legacy_board() {
        let dir = tempfile::tempdir().unwrap();
        let private = dir.path().join(PROJECT_METADATA_PATH);
        std::fs::create_dir_all(private.parent().unwrap()).unwrap();
        std::fs::write(private, "not valid json").unwrap();
        std::fs::write(dir.path().join("kidbright-project.json"), json!({
            "schema_version":1,"board_profile":"kidbright32_v13","reference_example":"button_buzzer"
        }).to_string()).unwrap();
        let context = project_context(dir.path());
        assert!(context.contains("PROJECT PROFILE ERROR"));
        assert!(!context.contains("kidbright32_v13"));
    }

    #[test]
    fn internal_file_policy_keeps_user_code_assets_and_nested_documents_visible() {
        for name in [
            PROJECT_METADATA_PATH,
            "kidbright-project.json",
            "KIDBRIGHT_COMPATIBILITY.md",
            "main/../kidbright-project.json",
            "./KIDBRIGHT_COMPATIBILITY.md",
            "components/kb_reference/../../KIDBRIGHT_COMPATIBILITY.md",
        ] {
            assert!(is_internal_project_file(Path::new(name)));
        }
        for name in [
            "main/main.c",
            "main/kb_board.h",
            "CMakeLists.txt",
            "config.json",
            "README.md",
            "docs/kidbright-project.json",
            "docs/KIDBRIGHT_COMPATIBILITY.md",
            "docs/main/../kidbright-project.json",
        ] {
            assert!(!is_internal_project_file(Path::new(name)));
        }
    }

    #[test]
    fn project_context_uses_validated_metadata_not_arbitrary_instructions() {
        let dir = tempfile::tempdir().unwrap();
        assert!(project_context(dir.path()).is_empty());
        std::fs::write(
            dir.path().join("kidbright-project.json"),
            json!({
                "schema_version":1, "board_profile":"kidbright32_v13",
                "reference_example":"button_buzzer", "instructions":"INJECTED_RULE"
            })
            .to_string(),
        )
        .unwrap();
        let context = project_context(dir.path());
        assert!(context.contains("kidbright32_v13"));
        assert!(!context.contains("INJECTED_RULE"));
    }

    #[test]
    fn prompt_after_workspace_switch_uses_only_the_new_profile() {
        let old = tempfile::tempdir().unwrap();
        let new = tempfile::tempdir().unwrap();
        for (directory, board) in [(&old, "kidbright32_v13"), (&new, "kidbright32_rev31")] {
            std::fs::write(
                directory.path().join("kidbright-project.json"),
                json!({
                    "schema_version":1, "board_profile":board, "reference_example":"button_buzzer"
                })
                .to_string(),
            )
            .unwrap();
        }
        assert_eq!(with_project_context("base instructions",old.path(),false),"base instructions","Private metadata must not be read/exported without permission");
        let before = with_project_context("base instructions", old.path(),true);
        let after = with_project_context("base instructions", new.path(),true);
        assert!(before.contains("kidbright32_v13"));
        assert!(after.starts_with("base instructions"));
        assert!(after.contains("kidbright32_rev31"));
        assert!(!after.contains("kidbright32_v13"));
        assert_eq!(
            with_project_context("base instructions", tempfile::tempdir().unwrap().path(),true),
            "base instructions"
        );
    }
}
