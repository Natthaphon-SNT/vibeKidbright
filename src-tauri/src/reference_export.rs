//! Deterministic reference export for development build checks. No SDK or device I/O.
use std::path::Path;

/// Export the same scoped recipes used by the AI tool into a new directory.
/// Validate the profile/recipe before creating anything; never overwrite a project.
pub fn export(directory: &Path, board: &str, example: &str) -> Result<(), String> {
    let files = crate::ai::compatibility::files(board, example)?;
    let write = || -> std::io::Result<()> {
        std::fs::create_dir(directory)?;
        std::fs::write(directory.join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.16)\ninclude($ENV{IDF_PATH}/tools/cmake/project.cmake)\nproject(kidbright_reference)\n")?;
        std::fs::write(
            directory.join("sdkconfig.defaults"),
            "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n",
        )?;
        for (relative, content) in files {
            let path = directory.join(relative);
            std::fs::create_dir_all(path.parent().expect("recipe file has a parent"))?;
            std::fs::write(path, content)?;
        }
        crate::ai::compatibility::hide_internal_metadata(directory)
    };
    write().map_err(|error| format!("Cannot export reference project: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_export_matches_the_actual_recipe_and_build_metadata() {
        let root = tempfile::tempdir().unwrap();
        for board in ["kidbright32_v13", "kidbright32_rev31"] {
            for example in crate::ai::compatibility::EXAMPLE_IDS {
                if board == "kidbright32_rev31" && *example == "matrix_counter" {
                    continue; // Not a supported recipe for this profile.
                }
                let directory = root.path().join(format!("{board}-{example}"));
                export(&directory, board, example).unwrap();
                for (relative, expected) in crate::ai::compatibility::files(board, example).unwrap()
                {
                    assert_eq!(
                        std::fs::read_to_string(directory.join(relative)).unwrap(),
                        expected
                    );
                }
                assert!(std::fs::read_to_string(directory.join("CMakeLists.txt"))
                    .unwrap()
                    .contains("project(kidbright_reference)"));
                assert!(
                    std::fs::read_to_string(directory.join("sdkconfig.defaults"))
                        .unwrap()
                        .contains("CONFIG_IDF_TARGET=\"esp32\"")
                );
                crate::ai::compatibility::validate_build_profile(&directory).unwrap();
            }
        }
    }

    #[test]
    fn unsupported_inputs_create_nothing_and_existing_projects_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("project");
        for (board, example) in [
            ("unknown", "button_buzzer"),
            ("kidbright32_v13", "unknown"),
            ("kidbright32_rev31", "matrix_counter"),
        ] {
            assert!(export(&directory, board, example).is_err());
            assert!(!directory.exists());
        }
        std::fs::create_dir(&directory).unwrap();
        let existing = directory.join("CMakeLists.txt");
        std::fs::write(&existing, "user project\n").unwrap();
        assert!(export(&directory, "kidbright32_v13", "button_buzzer").is_err());
        assert_eq!(std::fs::read_to_string(existing).unwrap(), "user project\n");
        assert_eq!(std::fs::read_dir(directory).unwrap().count(), 1);
    }
}
