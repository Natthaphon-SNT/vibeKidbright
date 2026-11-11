//! Bundled iKB-1Z files for an explicitly requested host workspace.
use serde_json::{json, Value};
type Files = Vec<(String, String)>;

fn selected(input: &Value) -> Result<bool, String> {
    let Some(value) = input.get("expansion_boards") else {
        if input.get("ikb_i2c_driver").is_some() { return Err("ikb_i2c_driver requires expansion_boards=[ikb_1z].".into()); }
        return Ok(false);
    };
    let values = value.as_array().ok_or("expansion_boards must be an array.")?;
    if values.is_empty() && input.get("ikb_i2c_driver").is_none() { return Ok(false); }
    if values.len() != 1 || values[0] != "ikb_1z" {
        return Err("The supported explicit expansion selection is [\"ikb_1z\"]. Do not substitute boards.".into());
    }
    Ok(true)
}

pub(crate) fn add(input: &Value, plus: bool, initial: &mut Files, references: &mut Files) -> Result<(), String> {
    if !selected(input)? { return Ok(()); }
    let family = input.get("ikb_i2c_driver").and_then(Value::as_str);
    if plus && input.get("ikb_i2c_driver").is_some() {
        return Err("Plus uses Python MaixI2CTransport, not an ESP-IDF I2C driver family.".into());
    }
    if !plus && !matches!(family, Some("master" | "legacy")) {
        return Err("For classic iKB-1Z select ikb_i2c_driver=master or legacy to match the whole app. No default bus/driver is guessed.".into());
    }
    if !plus && family == Some("legacy") && !references.is_empty() {
        return Err("The bundled reference recipes use the master I2C family. For a legacy custom draft omit board_profile/reference_example; never link both families.".into());
    }
    if plus {
        initial.push(("main/ikb1z.py".into(), include_str!("../../../knowledge_base/drivers/ikb1z.py").into()));
    } else {
        let component = "components/ikb1z";
        for (name, content) in [
            ("ikb1z.h", include_str!("../../../knowledge_base/drivers/ikb1z.h")),
            ("ikb1z.c", include_str!("../../../knowledge_base/drivers/ikb1z.c")),
            ("ikb1z_esp_idf.h", include_str!("../../../knowledge_base/drivers/ikb1z_esp_idf.h")),
            ("ikb1z_esp_idf.c", include_str!("../../../knowledge_base/drivers/ikb1z_esp_idf.c")),
        ] { initial.push((format!("{component}/{name}"), content.into())); }
        let legacy = family == Some("legacy");
        initial.push((format!("{component}/CMakeLists.txt"), format!(
            "idf_component_register(SRCS \"ikb1z.c\" \"ikb1z_esp_idf.c\" INCLUDE_DIRS \".\" REQUIRES {} freertos)\n{}",
            if legacy { "driver" } else { "esp_driver_i2c" },
            if legacy { "target_compile_definitions(${COMPONENT_LIB} PUBLIC IKB1Z_USE_LEGACY_I2C)\n" } else { "" }
        )));
        let files = if references.is_empty() { &mut *initial } else { &mut *references };
        let (_, cmake) = files.iter_mut().find(|(name, _)| name == "main/CMakeLists.txt")
            .ok_or("Generated main/CMakeLists.txt is missing; expansion was not installed.")?;
        if cmake.contains("PRIV_REQUIRES ") {
            *cmake = cmake.replacen("PRIV_REQUIRES ", "PRIV_REQUIRES ikb1z ", 1);
        } else if cmake.contains("REQUIRES ") {
            *cmake = cmake.replacen("REQUIRES ", "REQUIRES ikb1z ", 1);
        } else {
            let end = cmake.rfind(')').ok_or("Generated CMake registration is incomplete.")?;
            cmake.insert_str(end, "\n    PRIV_REQUIRES ikb1z\n");
        }
    }
    // Only generated metadata is modified. Existing projects still use reviewed
    // write_file proposals; no private file reads or implicit actuator commands.
    if !plus && !initial.iter().chain(references.iter()).any(|(name,_)|name==super::compatibility::PROJECT_METADATA_PATH) {
        initial.push((super::compatibility::PROJECT_METADATA_PATH.into(),json!({
            "schema_version":1,"project_type":"esp_idf","target":"esp32","entrypoint":"main/main.c",
            "validation":"custom source draft; no executable board profile or physical certification"
        }).to_string()));
    }
    for (name, content) in initial.iter_mut().chain(references.iter_mut()) {
        if name == super::compatibility::PROJECT_METADATA_PATH {
            let mut value: Value = serde_json::from_str(content).map_err(|e|format!("Generated metadata: {e}"))?;
            value["expansion_boards"] = json!(["ikb_1z"]);
            if !plus { value["ikb_i2c_driver"] = json!(family); }
            *content = serde_json::to_string_pretty(&value).unwrap() + "\n";
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn files_follow_host_and_do_not_add_fake_pin_config_or_actuate() {
        for plus in [false, true] {
            let input = if plus {json!({"project_type":"kidbright_uai_plus","expansion_boards":["ikb_1z"]})}
                else {json!({"expansion_boards":["ikb_1z"],"ikb_i2c_driver":"master"})};
            let mut files = if plus {super::super::uai_plus::files(&input).unwrap()}
                else {vec![("main/CMakeLists.txt".into(),"idf_component_register(SRCS \"main.c\" INCLUDE_DIRS \".\")\n".into())]};
            add(&input, plus, &mut files, &mut vec![]).unwrap();
            if plus {
                assert!(files.iter().any(|(name,_)| name == "main/ikb1z.py"));
                assert!(!files.iter().any(|(name,_)| name.ends_with(".c") || name.ends_with("CMakeLists.txt")));
                let metadata:Value=serde_json::from_str(&files.iter().find(|(n,_)|n==super::super::compatibility::PROJECT_METADATA_PATH).unwrap().1).unwrap();
                assert_eq!(metadata["expansion_boards"], json!(["ikb_1z"]));
                assert_eq!(metadata["target"],"sg2002");
            } else {
                assert!(files.iter().any(|(name,_)| name == "components/ikb1z/ikb1z_esp_idf.c"));
                assert!(files[0].1.contains("PRIV_REQUIRES ikb1z"));
                let directory=tempfile::tempdir().unwrap();
                let metadata=directory.path().join(super::super::compatibility::PROJECT_METADATA_PATH);
                std::fs::create_dir_all(metadata.parent().unwrap()).unwrap();
                std::fs::write(metadata,&files.iter().find(|(n,_)|n==super::super::compatibility::PROJECT_METADATA_PATH).unwrap().1).unwrap();
                let context=super::super::compatibility::project_context(directory.path());
                assert!(context.contains("[CURRENT KIDBRIGHT PROJECT]"));
                assert!(context.contains("[CURRENT PROJECT EXPANSION: IKB-1Z]"));
                assert!(!context.contains("PROFILE ERROR"));
            }
        }
    }
    #[test]
    fn invalid_or_mixed_expansions_fail_before_workspace_creation() {
        for input in [json!({"expansion_boards":true}),json!({"expansion_boards":["ikb_1"]}),
            json!({"expansion_boards":["ikb_1z","ikb_1z"]}),json!({"expansion_boards":["ikb_1z"]}),
            json!({"ikb_i2c_driver":"master"})] {
            assert!(add(&input,false,&mut vec![],&mut vec![]).is_err(),"{input}");
        }
        assert!(add(&json!({"expansion_boards":["ikb_1z"],"ikb_i2c_driver":"master"}),true,&mut vec![],&mut vec![]).is_err());
        assert!(add(&json!({"expansion_boards":["ikb_1z"],"ikb_i2c_driver":"legacy","reference_example":"matrix_counter"}),false,&mut vec![],&mut vec![]).is_err());
    }
    #[test]
    fn master_reference_and_legacy_custom_cmake_register_the_correct_family() {
        for family in ["master","legacy"] {
            let mut initial=vec![];
            let mut references=if family=="master" {super::super::compatibility::files("kidbright32_v13","matrix_counter").unwrap()}
                else {vec![]};
            if family=="legacy" {initial.push(("main/CMakeLists.txt".into(),"idf_component_register(SRCS \"main.c\" INCLUDE_DIRS \".\")\n".into()));}
            let input=json!({"expansion_boards":["ikb_1z"],"ikb_i2c_driver":family});
            add(&input,false,&mut initial,&mut references).unwrap();
            let component=&initial.iter().find(|(n,_)|n=="components/ikb1z/CMakeLists.txt").unwrap().1;
            assert_eq!(component.contains("IKB1Z_USE_LEGACY_I2C"),family=="legacy");
            assert!(initial.iter().chain(references.iter()).find(|(n,_)|n=="main/CMakeLists.txt").unwrap().1.contains("PRIV_REQUIRES ikb1z"));
        }
    }
}
