//! Read-only SDK validation and macOS discovery. Never install or repair a
//! Python environment by substituting an unrelated system Python interpreter.
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) fn version(idf: &Path) -> String {
    if let Ok(text) = std::fs::read_to_string(idf.join("version.txt")) {
        if !text.trim().is_empty() {
            return text.trim().to_owned();
        }
    }
    let cmake = std::fs::read_to_string(idf.join("tools/cmake/version.cmake")).unwrap_or_default();
    let value = |part: &str| -> Option<String> {
        regex::Regex::new(&format!(r"set\(IDF_VERSION_{part}\s+(\d+)\s*\)"))
            .ok()?
            .captures(&cmake)
            .map(|capture| capture[1].to_owned())
    };
    match (value("MAJOR"), value("MINOR"), value("PATCH")) {
        (Some(major), Some(minor), Some(patch)) => format!("{major}.{minor}.{patch}"),
        _ => String::new(),
    }
}

#[cfg(test)]
pub(super) fn python(
    idf: &Path,
    tools: &Path,
    windows: bool,
) -> Result<(PathBuf, PathBuf), String> {
    python_matching(idf, tools, windows, None)
}

fn python_matching(
    idf: &Path,
    tools: &Path,
    windows: bool,
    native: Option<&str>,
) -> Result<(PathBuf, PathBuf), String> {
    let version = version(idf);
    let parts: Vec<_> = version.trim_start_matches('v').split('.').collect();
    let prefix = if parts.len() >= 2
        && parts[..2]
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|c| c.is_ascii_digit()))
    {
        format!("idf{}.{}_py", parts[0], parts[1])
    } else {
        "idf".into()
    };
    let directory = tools.join("python_env");
    let mut environments: Vec<_> = std::fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().is_some_and(|name| {
                let name = name.to_string_lossy();
                name.starts_with(&prefix) && name.contains("_py") && name.ends_with("_env")
            })
        })
        .collect();
    environments.sort();
    environments.reverse();
    for environment in environments {
        let candidates = if windows {
            vec![environment.join("Scripts/python.exe")]
        } else {
            vec![
                environment.join("bin/python"),
                environment.join("bin/python3"),
            ]
        };
        if let Some(python) = candidates.into_iter().find(|path| {
            path.is_file() && native.is_none_or(|architecture| mac_executable(path, architecture))
        }) {
            return Ok((python, environment));
        }
    }
    Err(format!("ESP-IDF {} Python environment is missing or broken in {}. Python may not be executable for this machine; a copied Windows environment cannot run on macOS. Install the native ESP-IDF tools, then use Setup / Repair ESP-IDF or recheck the environment.",
        if version.is_empty() { "SDK" } else { &version }, directory.display()))
}

fn mac_executable(path: &Path, architecture: &str) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !path
            .metadata()
            .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
        {
            return false;
        }
    }
    let mut bytes = [0u8; 4096];
    let length = match std::fs::File::open(path).and_then(|mut file| file.read(&mut bytes)) {
        Ok(length) => length,
        Err(_) => return false,
    };
    let bytes = &bytes[..length];
    let expected = match architecture {
        "aarch64" => 0x0100000cu32,
        "x86_64" => 0x01000007u32,
        _ => return false,
    };
    if bytes.len() < 8 {
        return false;
    }
    let word = |offset: usize, little: bool| -> Option<u32> {
        let data: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
        Some(if little {
            u32::from_le_bytes(data)
        } else {
            u32::from_be_bytes(data)
        })
    };
    match &bytes[..4] {
        [0xcf, 0xfa, 0xed, 0xfe] => word(4, true) == Some(expected),
        [0xfe, 0xed, 0xfa, 0xcf] => word(4, false) == Some(expected),
        [0xca, 0xfe, 0xba, 0xbe] | [0xca, 0xfe, 0xba, 0xbf] => {
            let stride = if bytes[3] == 0xbf { 32 } else { 20 };
            let count = word(4, false).unwrap_or(0).min(64) as usize;
            (0..count).any(|index| word(8 + index * stride, false) == Some(expected))
        }
        _ => false, // In particular: Windows PE, Linux ELF and text placeholders.
    }
}

pub(super) fn validate(
    idf: &Path,
    tools: &Path,
    os: &str,
    arch: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let (python, environment) =
        python_matching(idf, tools, os == "windows", (os == "macos").then_some(arch))?;
    if os == "macos" {
        if idf.to_string_lossy().chars().any(char::is_whitespace)
            || tools.to_string_lossy().chars().any(char::is_whitespace)
        {
            return Err("ESP-IDF on macOS requires SDK/tools paths without spaces. Do not use Library/Application Support/toolchain. Install native tools under $HOME/esp and $HOME/.espressif, then recheck.".into());
        }
        if !mac_executable(&python, arch) {
            return Err(format!("ESP-IDF Python is not executable for this Mac ({arch}). Reinstall the native environment; Windows Scripts/python.exe and broken or copied venvs cannot be repaired by changing their home path."));
        }
        native_tools(idf, tools, os, arch)?;
    }
    Ok((python, environment))
}

pub(super) fn native_tools(
    idf: &Path,
    tools: &Path,
    os: &str,
    arch: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut paths = exported_paths(idf, tools, os, arch);
    paths.extend(std::env::split_paths(&super::build_idf_path(tools)));
    paths.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ]);
    let mut executables = Vec::new();
    for names in [
        &["cmake"][..],
        &["ninja"][..],
        &["xtensa-esp-elf-gcc", "xtensa-esp32-elf-gcc"][..],
    ] {
        let executable = paths
            .iter()
            .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
            .find(|path| path.is_file());
        match executable {
            Some(path) if mac_executable(&path, arch) => executables.push(path),
            _ => return Err(format!("Native macOS tool {} is missing or has the wrong CPU architecture. Complete the ESP-IDF macOS setup before building.", names.join(" / "))),
        }
    }
    Ok(executables)
}

pub(super) fn exported_paths(idf: &Path, tools: &Path, os: &str, arch: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let metadata = std::fs::read(idf.join("tools/tools.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let Some(entries) = metadata
        .as_ref()
        .and_then(|value| value["tools"].as_array())
    else {
        return paths;
    };
    let safe = |part: &str| {
        !part.is_empty()
            && Path::new(part)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_)))
    };
    for entry in entries {
        let Some(name) = entry["name"].as_str().filter(|name| safe(name)) else {
            continue;
        };
        let Some(versions) = entry["versions"].as_array() else {
            continue;
        };
        let platform = match (os, arch) {
            ("macos", "aarch64") => "macos-arm64",
            ("macos", _) => "macos",
            ("windows", "x86_64") => "win64",
            ("windows", _) => "win32",
            ("linux", "aarch64") => "linux-arm64",
            _ => "linux-amd64",
        };
        let mut exports = entry["export_paths"].as_array();
        if let Some(overrides) = entry["platform_overrides"].as_array() {
            for override_ in overrides {
                if override_["platforms"].as_array().is_some_and(|platforms| {
                    platforms
                        .iter()
                        .any(|value| value.as_str() == Some(platform))
                }) {
                    if let Some(paths) = override_["export_paths"].as_array() {
                        exports = Some(paths);
                    }
                }
            }
        }
        let Some(exports) = exports else {
            continue;
        };
        let mut versions: Vec<_> = versions.iter().collect();
        versions.sort_by_key(|version| version["status"].as_str() != Some("recommended"));
        for version in versions {
            let Some(version) = version["name"].as_str().filter(|version| safe(version)) else {
                continue;
            };
            for root in [tools.join("tools"), tools.to_path_buf()] {
                for export in exports {
                    let Some(parts) = export.as_array() else {
                        continue;
                    };
                    let mut path = root.join(name).join(version);
                    let mut valid = true;
                    for part in parts {
                        match part.as_str().filter(|part| safe(part)) {
                            Some(part) => path.push(part),
                            None => {
                                valid = false;
                                break;
                            }
                        }
                    }
                    if valid && path.is_dir() && !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }
        }
    }
    paths
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn mac_candidates(home: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut candidates = Vec::new();
    // idf-env.json is written by Espressif's own installer. Finder launches do
    // not inherit the IDF_PATH or PATH exported in an interactive shell.
    for tools in [
        home.join(".espressif"),
        home.join(".vibekidbright/.espressif"),
    ] {
        if let Ok(bytes) = std::fs::read(tools.join("idf-env.json")) {
            if let Ok(registry) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(records) = registry["idfInstalled"].as_object() {
                    let mut paths: Vec<_> = records
                        .values()
                        .filter_map(|record| record["path"].as_str())
                        .map(PathBuf::from)
                        .filter(|path| path.is_absolute())
                        .collect();
                    paths.sort();
                    paths.reverse();
                    candidates.extend(paths.into_iter().map(|idf| (idf, tools.clone())));
                }
            }
        }
    }
    let mut sources = vec![home.join("esp/esp-idf")];
    if let Ok(entries) = std::fs::read_dir(home.join("esp")) {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("esp-idf"))
            })
            .collect();
        paths.sort();
        paths.reverse();
        sources.extend(paths);
    }
    for tools in [
        home.join(".vibekidbright/.espressif"),
        home.join(".espressif"),
    ] {
        candidates.extend(sources.iter().cloned().map(|idf| (idf, tools.clone())));
    }
    candidates
}

pub(super) fn choose(
    candidates: &[(PathBuf, PathBuf)],
    os: &str,
    arch: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let mut failures = Vec::new();
    for (idf, tools) in candidates {
        if !idf.join("tools/idf.py").is_file() {
            continue;
        }
        match super::canonical_idf_pair(idf, tools).and_then(|(idf, tools)| {
            validate(&idf, &tools, os, arch)?;
            Ok((idf, tools))
        }) {
            Ok(pair) => return Ok(pair),
            Err(error) => failures.push(error),
        }
    }
    Err(failures.into_iter().next().unwrap_or_else(|| "ESP-IDF is not installed. Open Setup / Repair ESP-IDF and install the native SDK for this computer.".into()))
}

#[cfg(test)]
#[path = "esp_idf_environment_tests.rs"]
mod tests;
