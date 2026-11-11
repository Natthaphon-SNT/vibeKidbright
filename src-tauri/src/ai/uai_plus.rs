//! Python workspace support for the SG2002 Plus; never an ESP-IDF profile.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn test_registry() -> &'static Value {
    static REGISTRY: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| serde_json::from_str(include_str!(
        "../../../resources/knowledge_base/compatibility/uai_plus_test_contracts.json"
    )).expect("checked Plus test registry"))
}

fn test_case(id: &str) -> Result<&'static Value, String> {
    let id = id.trim().to_ascii_uppercase();
    if id.len() != 6 || !id.starts_with("MP-") || !id[3..].bytes().all(|b| b.is_ascii_digit()) {
        return Err("Use an exact Plus test ID such as MP-021.".into());
    }
    test_registry()["tests"].as_array().unwrap().iter().find(|r| r["id"] == id)
        .ok_or_else(|| "Unknown Plus test ID; audited IDs are MP-001 through MP-128.".into())
}

pub(crate) fn test_reference(input: &Value) -> Value {
    let registry = test_registry();
    if input.get("test_id").is_none() {
        return json!({"guide":registry["guide"],"summary":registry["summary"],
            "tests":registry["tests"].as_array().unwrap().iter().map(|r|json!({"id":r["id"],"state":r["state"]})).collect::<Vec<_>>(),
            "physical_results":"NOT_RUN","live_model_generation_tested":false});
    }
    let Some(id) = input["test_id"].as_str() else { return json!({"error":"test_id must be a string."}); };
    match test_case(id) {
        Err(error) => json!({"error":error}),
        Ok(case) => {
            let contracts: serde_json::Map<String,Value> = case["contracts"].as_array().unwrap().iter()
                .filter_map(|key|key.as_str()).map(|key|(key.into(),registry["contracts"][key].clone())).collect();
            json!({"test":case,"contracts":contracts,"guide":registry["guide"],
                "workflow":"Use the matching template when supplied. Otherwise provide the complete requested main_python in create_project_workspace; do not create another test's seed. Finish all write_file proposals before asking for Keep; proposals are allowed during creation review. NEEDS_SETUP requires the listed fixtures/data; HOLD permits unchanged capabilities/manual planning only. No seed, API presence, local fixture or upload is physical success. User starts board Run/Stop.",
                "source":"bundled reviewed public reference; no private project files read"})
        }
    }
}

fn custom_main(input: &Value) -> Result<Option<&str>, String> {
    let Some(value) = input.get("main_python") else { return Ok(None); };
    let source = value.as_str().ok_or("main_python must contain the full Python source as a string.")?;
    if source.trim().is_empty() || source.len() > 262144 || source.contains("TODO_REPLACE")
        || source.contains("<your_code_here>") || source.trim() == "updated_content" {
        return Err("main_python must contain complete non-placeholder Python source (at most 256 KiB).".into());
    }
    Ok(Some(source))
}

fn example_id(input: &Value) -> Result<&str, String> {
    let selected = input.get("python_example").and_then(Value::as_str);
    let custom = custom_main(input)?;
    if let Some(id) = input.get("test_id") {
        let id = id.as_str().ok_or("test_id must be an MP-xxx string.")?;
        let case = test_case(id)?;
        if let Some(expected) = case["template"].as_str() {
            if selected.is_some_and(|s| s != expected) {
                return Err(format!("{} requires python_example={expected}; a different seed does not implement this test.",case["id"]));
            }
            return Ok(expected);
        }
        if case["state"] == "HOLD" && selected != Some("capabilities") {
            return Err("This audited test is HOLD. Only the read-only capabilities template may be created until its blocker is resolved; do not generate actuation from a guessed mapping.".into());
        }
        if case["state"] == "HOLD" && custom.is_some() {
            return Err("HOLD allows the unchanged capabilities template only; resolve its blocker before custom hardware code.".into());
        }
        if case["state"] != "HOLD" && custom.is_none() {
            return Err("This test has no complete matching template. Read uai_plus_test_reference and pass the complete main_python in this creation call. Do not create an MP-021/circle seed for another test or ask the user to Keep an unfinished seed before implementing the requested test.".into());
        }
        if let Some(source) = custom {
            let normalized = source.replace("\r\n", "\n");
            if EXAMPLES.iter().any(|seed| example(seed).is_ok_and(|code|
                code.replace("\r\n", "\n").trim() == normalized.trim())) {
                return Err("main_python is an unchanged stock example, not the requested named test. Implement the actual program before creating this project.".into());
            }
        }
    }
    Ok(selected.unwrap_or("display_text_circle"))
}

pub(crate) const PROJECT_TYPE: &str = "kidbright_uai_plus";
pub(crate) const EXAMPLES: &[&str] = &["display_text_circle", "camera", "camera_overlay", "touchscreen", "adc_buttons", "capabilities"];
pub(crate) const DISPLAY: &str =
    include_str!("../../../samples/kidbright-uai-plus/display_text_circle.py");

pub(crate) fn requested(input: &Value) -> Result<bool, String> {
    match input.get("project_type").and_then(Value::as_str) {
        None if input.get("project_type").is_none() => {
            if input.get("main_python").is_some() { return Err("main_python requires project_type=kidbright_uai_plus.".into()); }
            if input.get("test_id").is_some() { return Err("test_id requires project_type=kidbright_uai_plus.".into()); }
            if input.get("python_example").is_some() { return Err("python_example requires project_type=kidbright_uai_plus.".into()); }
            Ok(false)
        }
        Some("esp_idf") => {
            if input.get("main_python").is_some() { return Err("main_python requires project_type=kidbright_uai_plus.".into()); }
            if input.get("test_id").is_some() { return Err("test_id requires project_type=kidbright_uai_plus.".into()); }
            if input.get("python_example").is_some() { return Err("python_example requires project_type=kidbright_uai_plus.".into()); }
            Ok(false)
        }
        Some(PROJECT_TYPE) => {
            if input.get("board_profile").is_some() || input.get("reference_example").is_some() {
                return Err("Plus Python projects cannot use classic ESP32 board_profile/reference_example.".into());
            }
            example(example_id(input)?)?;
            if input.get("python_example").is_some_and(|value| !value.is_string()) {
                return Err("python_example must be a supported example name.".into());
            }
            Ok(true)
        }
        _ => Err("Unsupported project_type. Use esp_idf or kidbright_uai_plus; do not substitute boards.".into()),
    }
}

fn example(id: &str) -> Result<&'static str, String> {
    match id {
        "display_text_circle" => Ok(DISPLAY),
        "camera" => Ok(include_str!(
            "../../../samples/kidbright-uai-plus/camera.py"
        )),
        "camera_overlay" => Ok(include_str!(
            "../../../samples/kidbright-uai-plus/camera_overlay.py"
        )),
        "touchscreen" => Ok(include_str!(
            "../../../samples/kidbright-uai-plus/touchscreen.py"
        )),
        "adc_buttons" => Ok(include_str!(
            "../../../samples/kidbright-uai-plus/adc_buttons.py"
        )),
        "capabilities" => Ok(include_str!(
            "../../../samples/kidbright-uai-plus/capabilities.py"
        )),
        _ => {
            Err("Unknown python_example. Use display_text_circle, camera, camera_overlay, touchscreen, adc_buttons or capabilities.".into())
        }
    }
}

pub(crate) fn files(input: &Value) -> Result<Vec<(String, String)>, String> {
    let id = example_id(input)?;
    let custom = custom_main(input)?;
    let mut result = vec![
        ("main/main.py".into(), custom.unwrap_or(example(id)?).into()),
        (
            super::compatibility::PROJECT_METADATA_PATH.into(),
            serde_json::to_string_pretty(&json!({
                "schema_version": 1, "project_type": PROJECT_TYPE, "target": "sg2002",
                "runtime": "maixpy4", "entrypoint": "main/main.py", "python_example": id,
                "requested_test": input.get("test_id"),
                "test_implementation": if custom.is_some() {"custom_source_requires_review"} else if let Some(test_id)=input["test_id"].as_str() {
                    if test_case(test_id)?["template"].as_str()==Some(id) {"matching_reference_template"} else {"seed_requires_custom_implementation"}
                } else {"unscoped_example"},
                "validation": "source-grounded draft; physical board unverified"
            }))
            .unwrap()
                + "\n",
        ),
    ];
    if id == "capabilities" {
        result.push(("main/plus_metadata.py".into(), include_str!("../uai_plus_probe.py").into()));
    }
    Ok(result)
}

pub(crate) fn is_manifest(value: &Value) -> bool {
    value["schema_version"] == 1
        && value["project_type"] == PROJECT_TYPE
        && value["target"] == "sg2002"
        && value["runtime"] == "maixpy4"
        && value["entrypoint"] == "main/main.py"
}

pub(crate) fn is_project(root: &Path) -> bool {
    let path = root.join(super::compatibility::PROJECT_METADATA_PATH);
    std::fs::symlink_metadata(&path)
        .is_ok_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() <= 8192)
        && std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .is_some_and(|value| is_manifest(&value))
}

pub(crate) fn initialize(base: &Path, name: &str, input: &Value) -> Result<PathBuf, String> {
    if !requested(input)? {
        return Err("A Plus Python project_type is required.".into());
    }
    let files = files(input)?;
    super::tools::initialize_workspace_files(base, name, &files)
}

#[tauri::command]
pub async fn create_uai_plus_project(path: String, name: String) -> Result<String, String> {
    let root = initialize(
        Path::new(&path),
        &name,
        &json!({"project_type":PROJECT_TYPE}),
    )?;
    super::compatibility::hide_internal_metadata(&root).map_err(|error| error.to_string())?;
    Ok(root.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn get_project_kind(path: String) -> Option<String> {
    let root = Path::new(&path);
    if is_project(root) && root.join("main/main.py").is_file() {
        Some(PROJECT_TYPE.into())
    } else if !is_project(root) && root.join("CMakeLists.txt").is_file() {
        Some("esp_idf".into())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plus_creation_is_fresh_python_and_does_not_overwrite_classic_or_duplicate() {
        let base = tempfile::tempdir().unwrap();
        let old = base.path().join("old");
        std::fs::create_dir(&old).unwrap();
        std::fs::write(old.join("main.c"), "old code").unwrap();
        for id in EXAMPLES {
            let input = json!({"project_type":PROJECT_TYPE,"python_example":id});
            let root = initialize(base.path(), id, &input).unwrap();
            assert!(is_project(&root));
            assert!(root.join("main/main.py").is_file());
            assert!(!root.join("CMakeLists.txt").exists());
            assert!(!root.join("sdkconfig").exists());
            assert!(super::super::compatibility::validate_build_profile(&root).is_err());
            assert!(!super::super::compatibility::project_context(&root).contains("PROFILE ERROR"));
            assert!(initialize(base.path(), id, &input).is_err());
        }
        assert_eq!(
            std::fs::read_to_string(old.join("main.c")).unwrap(),
            "old code"
        );
    }

    #[test]
    fn invalid_or_mixed_targets_are_rejected_before_creation() {
        for input in [
            json!({"project_type":null}),
            json!({"project_type":"kidbright_uai"}),
            json!({"project_type":PROJECT_TYPE,"board_profile":"kidbright32_v13"}),
            json!({"project_type":PROJECT_TYPE,"python_example":"unknown"}),
            json!({"project_type":PROJECT_TYPE,"python_example":12}),
            json!({"project_type":"esp_idf","python_example":"camera"}),
        ] {
            assert!(requested(&input).is_err(), "{input}");
        }
        assert!(!requested(&json!({})).unwrap());
        assert!(!requested(&json!({"project_type":"esp_idf"})).unwrap());
    }

    #[test]
    fn every_audited_test_resolves_to_public_contracts_without_a_device() {
        let summary = test_reference(&json!({}));
        assert_eq!(summary["tests"].as_array().unwrap().len(), 128);
        for n in 1..=128 {
            let id = format!("MP-{n:03}");
            let reference = test_reference(&json!({"test_id":id}));
            assert_eq!(reference["test"]["id"], id);
            assert_eq!(reference["test"]["physical_result"], "NOT_RUN");
            assert!(!reference["contracts"].as_object().unwrap().is_empty());
        }
        for id in ["MP-000","MP-129","MP-01","MP-015; reboot","../private"] {
            assert!(test_reference(&json!({"test_id":id})).get("error").is_some());
        }
        assert!(test_reference(&json!({"test_id":null})).get("error").is_some());
    }

    #[test]
    fn named_tests_choose_complete_templates_and_include_probe_helper() {
        for (id, expected) in [("MP-001","capabilities"),("MP-005","capabilities"),("MP-015","touchscreen"),("MP-021","adc_buttons")] {
            let input = json!({"project_type":PROJECT_TYPE,"test_id":id});
            assert!(requested(&input).unwrap());
            let generated = files(&input).unwrap();
            let metadata: Value = serde_json::from_str(&generated[1].1).unwrap();
            assert_eq!(metadata["python_example"],expected);
            assert_eq!(metadata["test_implementation"],"matching_reference_template");
            if expected=="capabilities" {assert!(generated.iter().any(|(path,_)|path=="main/plus_metadata.py"));}
            assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":id,"python_example":"display_text_circle"})).is_err());
        }
    }

    #[test]
    fn held_and_custom_cases_cannot_silently_fall_back_to_circle() {
        assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-037"})).is_err());
        assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-037","python_example":"camera"})).is_err());
        assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-037","python_example":"capabilities"})).unwrap());
        assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-079"})).is_err());
        assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-079","python_example":"camera"})).is_err());
        let draft=files(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-079","python_example":"camera","main_python":"# Full custom camera draft\nprint('review this draft')\n"})).unwrap();
        let metadata: Value=serde_json::from_str(&draft[1].1).unwrap();
        assert_eq!(metadata["test_implementation"],"custom_source_requires_review");
        assert!(requested(&json!({"project_type":"esp_idf","test_id":"MP-021"})).is_err());
    }

    #[test]
    fn custom_named_test_has_actual_source_before_the_first_project_review() {
        let base = tempfile::tempdir().unwrap();
        let seed = json!({"project_type":PROJECT_TYPE,"test_id":"MP-022","python_example":"adc_buttons"});
        assert!(initialize(base.path(), "mai_test_022", &seed).is_err());
        assert!(!base.path().join("mai_test_022").exists());
        let source = "# MP-022 S1 event counter draft\nprint('S1 events, not ADC calibration')\n";
        let mut input = seed;
        input["main_python"] = json!(source);
        let project = initialize(base.path(), "mai_test_022", &input).unwrap();
        assert_eq!(std::fs::read_to_string(project.join("main/main.py")).unwrap(),source);
        assert!(!std::fs::read_to_string(project.join("main/main.py")).unwrap().contains("PREPARE_SECONDS"));
        for bad in [Value::Null,json!(12),json!(""),json!("TODO_REPLACE"),json!(example("adc_buttons").unwrap())] {
            input["main_python"] = bad;
            assert!(requested(&input).is_err());
        }
        assert!(requested(&json!({"project_type":"esp_idf","main_python":source})).is_err());
        assert!(requested(&json!({"project_type":PROJECT_TYPE,"test_id":"MP-037","python_example":"capabilities","main_python":source})).is_err());
    }
}
