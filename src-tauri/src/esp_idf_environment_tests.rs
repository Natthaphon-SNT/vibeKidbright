use super::*;

fn executable(path: &Path, architecture: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = vec![0xcf, 0xfa, 0xed, 0xfe];
    let cpu: u32 = if architecture == "aarch64" {
        0x0100000c
    } else {
        0x01000007
    };
    bytes.extend(cpu.to_le_bytes());
    std::fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn native_sdk(root: &Path, architecture: &str) -> (PathBuf, PathBuf) {
    let idf = root.join("esp-idf");
    let tools = root.join(".espressif");
    std::fs::create_dir_all(idf.join("tools")).unwrap();
    std::fs::write(idf.join("tools/idf.py"), "# fixture").unwrap();
    std::fs::write(idf.join("version.txt"), "v5.5.1").unwrap();
    executable(
        &tools.join("python_env/idf5.5_py3.11_env/bin/python3"),
        architecture,
    );
    for name in ["cmake", "ninja", "xtensa-esp-elf-gcc"] {
        executable(&tools.join("tools/native/1/bin").join(name), architecture);
    }
    (idf, tools)
}

#[test]
fn incomplete_managed_sdk_with_ready_marker_falls_back_to_native_mac_sdk() {
    for architecture in ["aarch64", "x86_64"] {
        let temporary = tempfile::tempdir().unwrap();
        let broken = temporary.path().join("legacy");
        let (idf, tools) = native_sdk(&broken, architecture);
        std::fs::write(broken.join(".toolchain_ready"), "1.0.1").unwrap();
        std::fs::remove_file(tools.join("python_env/idf5.5_py3.11_env/bin/python3")).unwrap();
        let valid = native_sdk(&temporary.path().join("native"), architecture);
        let chosen = choose(&[(idf, tools), valid.clone()], "macos", architecture).unwrap();
        assert_eq!(
            chosen,
            (
                dunce::canonicalize(valid.0).unwrap(),
                dunce::canonicalize(valid.1).unwrap()
            )
        );
    }
}

#[test]
fn windows_environment_cannot_be_marked_ready_on_either_mac_architecture() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(temporary.path(), "aarch64");
    std::fs::remove_file(tools.join("python_env/idf5.5_py3.11_env/bin/python3")).unwrap();
    let python = tools.join("python_env/idf5.5_py3.11_env/Scripts/python.exe");
    std::fs::create_dir_all(python.parent().unwrap()).unwrap();
    std::fs::write(python, b"MZ Windows Python").unwrap();
    for architecture in ["aarch64", "x86_64"] {
        assert!(validate(&idf, &tools, "macos", architecture)
            .unwrap_err()
            .contains("missing or broken"));
    }
}

#[test]
fn wrong_architecture_or_missing_compiler_never_reports_ready() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(temporary.path(), "aarch64");
    assert!(validate(&idf, &tools, "macos", "x86_64")
        .unwrap_err()
        .contains("not be executable"));
    executable(
        &tools.join("tools/native/1/bin/xtensa-esp-elf-gcc"),
        "x86_64",
    );
    assert!(validate(&idf, &tools, "macos", "aarch64")
        .unwrap_err()
        .contains("xtensa"));
}

#[test]
fn application_support_sdk_path_is_rejected_before_build() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(&temporary.path().join("Application Support"), "aarch64");
    assert!(validate(&idf, &tools, "macos", "aarch64")
        .unwrap_err()
        .contains("without spaces"));
}

#[test]
fn python_environment_must_match_sdk_major_and_minor() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(temporary.path(), "aarch64");
    std::fs::write(idf.join("version.txt"), "v5.4.2").unwrap();
    assert!(python(&idf, &tools, false).is_err());
    executable(
        &tools.join("python_env/idf5.4_py3.11_env/bin/python"),
        "aarch64",
    );
    let (python, _) = python(&idf, &tools, false).unwrap();
    assert!(python.to_string_lossy().contains("idf5.4_py3.11_env"));
}

#[test]
fn official_cmake_version_is_used_when_checkout_has_no_version_txt() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temporary.path().join("tools/cmake")).unwrap();
    std::fs::write(
        temporary.path().join("tools/cmake/version.cmake"),
        "set(IDF_VERSION_MAJOR 5)\nset(IDF_VERSION_MINOR 5)\nset(IDF_VERSION_PATCH 1)\n",
    )
    .unwrap();
    assert_eq!(version(temporary.path()), "5.5.1");
}

#[test]
fn finder_discovers_espressif_registry_without_shell_environment() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(temporary.path(), "x86_64");
    std::fs::write(
        tools.join("idf-env.json"),
        serde_json::to_vec(&serde_json::json!({
            "idfInstalled":{"registered-sdk":{"path":idf,"version":"5.5.1"}}
        }))
        .unwrap(),
    )
    .unwrap();
    let candidates = mac_candidates(temporary.path());
    let chosen = choose(&candidates, "macos", "x86_64").unwrap();
    assert_eq!(chosen.0, dunce::canonicalize(idf).unwrap());
}

#[test]
fn malformed_registry_does_not_hide_standard_native_installation() {
    let temporary = tempfile::tempdir().unwrap();
    let (source, tools) = native_sdk(temporary.path(), "aarch64");
    let idf = temporary.path().join("esp/esp-idf");
    std::fs::create_dir_all(idf.parent().unwrap()).unwrap();
    std::fs::rename(source, &idf).unwrap();
    std::fs::write(tools.join("idf-env.json"), b"invalid").unwrap();
    assert!(choose(&mac_candidates(temporary.path()), "macos", "aarch64").is_ok());
}

#[test]
fn mac_cmake_app_bundle_uses_espressif_platform_export_paths() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(temporary.path(), "aarch64");
    let cmake = tools.join("tools/cmake/3.30.2/CMake.app/Contents/bin");
    executable(&cmake.join("cmake"), "aarch64");
    std::fs::remove_file(tools.join("tools/native/1/bin/cmake")).unwrap();
    let metadata = serde_json::json!({"tools":[{
        "name":"cmake", "export_paths":[["bin"]], "versions":[{"name":"3.30.2","status":"recommended"}],
        "platform_overrides":[{"platforms":["macos","macos-arm64"],"export_paths":[["CMake.app","Contents","bin"]]}]
    }]});
    std::fs::write(
        idf.join("tools/tools.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    assert_eq!(
        exported_paths(&idf, &tools, "macos", "aarch64"),
        vec![cmake.clone()]
    );
    assert_eq!(exported_paths(&idf, &tools, "macos", "x86_64"), vec![cmake]);
    assert!(validate(&idf, &tools, "macos", "aarch64").is_ok());
    assert!(exported_paths(&idf, &tools, "windows", "x86_64").is_empty());
}

#[test]
fn multiple_python_environments_choose_one_that_matches_the_mac_cpu() {
    let temporary = tempfile::tempdir().unwrap();
    let (idf, tools) = native_sdk(temporary.path(), "aarch64");
    executable(
        &tools.join("python_env/idf5.5_py3.13_env/bin/python"),
        "x86_64",
    );
    let (python, _) = validate(&idf, &tools, "macos", "aarch64").unwrap();
    assert!(python.to_string_lossy().contains("idf5.5_py3.11_env"));
}
