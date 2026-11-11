//! Filesystem operations whose boundaries are the selected workspace.
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub(crate) fn scoped_file(project: Option<&str>, path: &str) -> Result<PathBuf, String> {
    let Some(project) = project else { return Ok(PathBuf::from(path)); };
    if project.is_empty() || project == "." { return Err("No project workspace is selected.".into()); }
    let root = dunce::canonicalize(crate::ai_chat::normalize_project_dir(project)).map_err(|e| e.to_string())?;
    let path = PathBuf::from(path);
    if path.components().any(|part| matches!(part, Component::ParentDir)) {
        return Err("File paths cannot contain parent traversal.".into());
    }
    let relative = if path.is_absolute() {
        let parent = dunce::canonicalize(path.parent().ok_or("Missing file parent.")?).map_err(|e|e.to_string())?;
        parent.strip_prefix(&root).map_err(|_|"File is outside the selected workspace.")?
            .join(path.file_name().ok_or("Missing file name.")?)
    } else { path };
    if crate::ai::compatibility::is_internal_project_file(&relative) {
        return Err("Internal project metadata is managed by the application.".into());
    }
    crate::ai::tools::workspace_file_path(&root, &relative.to_string_lossy())
}

pub(crate) fn reject_link(path: &Path) -> Result<(), String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    let mut linked = metadata.file_type().is_symlink();
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        linked |= metadata.file_attributes() & 0x400 != 0;
    }
    if linked { Err(format!("Project copy cannot follow links or junctions: {}",path.display())) } else { Ok(()) }
}

#[derive(serde::Serialize)]
#[serde(rename_all="camelCase")]
pub(crate) struct ProjectCopy { pub project_path: String, pub files_copied: u64 }

fn skip(name: &str) -> bool {
    matches!(name,"build"|"target"|"node_modules"|".git"|"__pycache__")
}

fn inventory(root: &Path) -> Result<Vec<(PathBuf, bool)>, String> {
    let mut result=Vec::new();
    let mut pending=vec![root.to_path_buf()];
    while let Some(dir)=pending.pop() {
        reject_link(&dir)?;
        for entry in std::fs::read_dir(&dir).map_err(|e|e.to_string())? {
            let entry=entry.map_err(|e|e.to_string())?;
            if skip(&entry.file_name().to_string_lossy()) {continue;}
            let path=entry.path(); reject_link(&path)?;
            let metadata=std::fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
            if !metadata.is_dir() && !metadata.is_file() {return Err("Project copy requires regular files and directories.".into());}
            result.push((path.strip_prefix(root).map_err(|e|e.to_string())?.to_path_buf(),metadata.is_dir()));
            if metadata.is_dir() {pending.push(path);}
        }
    }
    result.sort(); Ok(result)
}

fn hash(path: &Path) -> Result<Vec<u8>, String> {
    let mut input=std::fs::File::open(path).map_err(|e|e.to_string())?;
    let mut digest=Sha256::new();let mut buffer=[0u8;65536];
    loop {let n=input.read(&mut buffer).map_err(|e|e.to_string())?;if n==0 {break;} digest.update(&buffer[..n]);}
    Ok(digest.finalize().to_vec())
}

pub(crate) fn copy_project(source: &Path, destination_parent: &Path) -> Result<ProjectCopy,String> {
    let source=dunce::canonicalize(source).map_err(|e|e.to_string())?;
    if !source.is_dir() {return Err("Source project is not a directory.".into());}
    let parent=dunce::canonicalize(destination_parent).map_err(|e|e.to_string())?;
    if parent.starts_with(&source) {
        return Err("Choose a destination outside the source project; it cannot be copied into itself or a child folder.".into());
    }
    let dest=parent.join(source.file_name().ok_or("The filesystem root cannot be copied as a project.")?);
    // Inspect before writing. A failed copy never changes or removes the source.
    let entries=inventory(&source)?;
    std::fs::create_dir(&dest).map_err(|e|format!("Destination must be a new folder: {e}"))?;
    let mut copied=0;
    for (relative, is_dir) in &entries {
        let from=source.join(relative);let to=dest.join(relative);reject_link(&from)?;
        if from.is_dir()!=*is_dir {return Err("Source file type changed during Save As.".into());}
        if *is_dir {std::fs::create_dir_all(&to).map_err(|e|e.to_string())?;continue;}
        std::fs::create_dir_all(to.parent().ok_or("Missing copied file parent.")?).map_err(|e|e.to_string())?;
        let mut input=std::fs::File::open(&from).map_err(|e|e.to_string())?;
        let mut output=std::fs::OpenOptions::new().create_new(true).write(true).open(&to).map_err(|e|e.to_string())?;
        std::io::copy(&mut input,&mut output).map_err(|e|e.to_string())?;
        output.flush().and_then(|_|output.sync_all()).map_err(|e|e.to_string())?;
        std::fs::set_permissions(&to,std::fs::metadata(&from).map_err(|e|e.to_string())?.permissions()).map_err(|e|e.to_string())?;
        if hash(&from)?!=hash(&to)? {return Err("Source changed during Save As. Partial destination was kept; the active project was not switched.".into());}
        copied+=1;
    }
    if inventory(&source)?!=entries {return Err("Source tree changed during Save As. Review the partial destination before retrying.".into());}
    // Verify every file after copying, not just the last file in the tree.
    for (relative, is_dir) in &entries {
        let from=source.join(relative);reject_link(&from)?;
        if !is_dir && hash(&from)?!=hash(&dest.join(relative))? {
            return Err("Source changed during Save As. The active project was not switched.".into());
        }
    }
    crate::ai::compatibility::hide_internal_metadata(&dest).map_err(|e|e.to_string())?;
    Ok(ProjectCopy{project_path:dest.to_string_lossy().into_owned(),files_copied:copied})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_as_rejects_descendant_and_existing_destination_before_writing() {
        let base=tempfile::tempdir().unwrap();let root=base.path().join("work");
        std::fs::create_dir_all(root.join("sub")).unwrap();std::fs::write(root.join("main.py"),"keep").unwrap();
        assert!(copy_project(&root,&root).is_err());assert!(copy_project(&root,&root.join("sub")).is_err());
        assert!(!root.join("sub/work").exists());
        let parent=base.path().join("copies");std::fs::create_dir_all(parent.join("work")).unwrap();
        std::fs::write(parent.join("work/old.py"),"old").unwrap();
        assert!(copy_project(&root,&parent).is_err());
        assert_eq!(std::fs::read_to_string(parent.join("work/old.py")).unwrap(),"old");
        assert_eq!(std::fs::read_to_string(root.join("main.py")).unwrap(),"keep");
    }

    #[test]
    fn save_as_copies_helpers_metadata_and_empty_directories_without_build_artifacts() {
        let base=tempfile::tempdir().unwrap();let root=base.path().join("work");let parent=base.path().join("copies");
        std::fs::create_dir_all(&parent).unwrap();
        for dir in ["main/empty",".vibekidbright","build","main/__pycache__"] {std::fs::create_dir_all(root.join(dir)).unwrap();}
        for name in ["main/main.py","main/helper.py",".vibekidbright/project.json","build/cache","main/__pycache__/cache"] {
            std::fs::write(root.join(name),name).unwrap();
        }
        let result=copy_project(&root,&parent).unwrap();let dest=PathBuf::from(result.project_path);
        assert_eq!(result.files_copied,3);assert!(dest.join("main/empty").is_dir());
        assert_eq!(hash(&root.join("main/helper.py")).unwrap(),hash(&dest.join("main/helper.py")).unwrap());
        assert!(dest.join(".vibekidbright/project.json").is_file());assert!(!dest.join("build").exists());
        assert!(!dest.join("main/__pycache__").exists());
    }

    #[test]
    fn scoped_file_rejects_sibling_project_traversal_and_internal_metadata() {
        let base=tempfile::tempdir().unwrap();let root=base.path().join("project");let other=base.path().join("project_other");
        std::fs::create_dir_all(root.join("main")).unwrap();std::fs::create_dir_all(other.join("main")).unwrap();
        std::fs::write(root.join("main/main.py"),"correct").unwrap();std::fs::write(other.join("main/main.py"),"other").unwrap();
        let key=root.to_str().unwrap();
        assert!(scoped_file(Some(key),other.join("main/main.py").to_str().unwrap()).is_err());
        assert!(scoped_file(Some(key),"main/../main/main.py").is_err());
        assert!(scoped_file(Some(key),".vibekidbright/project.json").is_err());
        assert!(scoped_file(Some("."),"main/main.py").is_err());
        assert_eq!(std::fs::read_to_string(scoped_file(Some(key),"main/main.py").unwrap()).unwrap(),"correct");
        assert!(scoped_file(Some(key),"main/helper.py").unwrap().starts_with(dunce::canonicalize(&root).unwrap()));
    }

    #[cfg(windows)]
    #[test]
    fn scoped_file_accepts_real_windows_case_and_separator_aliases() {
        let base=tempfile::tempdir().unwrap();let root=base.path().join("MixedCase");
        std::fs::create_dir_all(root.join("main")).unwrap();
        let file=root.join("main/program.py");std::fs::write(&file,"selected workspace").unwrap();
        let alias=file.to_string_lossy().to_uppercase().replace('\\',"/");
        let resolved=scoped_file(Some(root.to_str().unwrap()),&alias).unwrap();
        assert_eq!(std::fs::read_to_string(resolved).unwrap(),"selected workspace");
    }

    #[test]
    fn manual_classic_creation_keeps_existing_code_and_rejects_invalid_names() {
        let base=tempfile::tempdir().unwrap();
        let existing=base.path().join("existing");std::fs::create_dir_all(existing.join("main")).unwrap();
        std::fs::write(existing.join("main/main.c"),"user program").unwrap();
        assert!(crate::ai::tools::initialize_project_workspace(base.path(),"existing").is_err());
        assert_eq!(std::fs::read_to_string(existing.join("main/main.c")).unwrap(),"user program");
        for name in ["", "..", "../escape", "with space", "/absolute", "C:\\outside"] {
            assert!(crate::ai::tools::initialize_project_workspace(base.path(),name).is_err());
        }
        let fresh=crate::ai::tools::initialize_project_workspace(&base.path().join("."),"fresh").unwrap();
        assert_eq!(fresh,dunce::canonicalize(&fresh).unwrap());
        for file in ["CMakeLists.txt","main/CMakeLists.txt","main/main.c"] {assert!(fresh.join(file).is_file());}
    }

    #[test]
    fn save_as_preserves_plus_identity_and_helpers_in_unicode_parent_paths() {
        let base=tempfile::tempdir().unwrap();let parent=base.path().join("งาน ที่มีช่องว่าง");
        std::fs::create_dir(&parent).unwrap();
        let source=crate::ai::uai_plus::initialize(&parent,"plus",&serde_json::json!({"project_type":"kidbright_uai_plus","python_example":"capabilities"})).unwrap();
        let copies=base.path().join("copies");std::fs::create_dir(&copies).unwrap();
        let result=copy_project(&source,&copies).unwrap();let dest=PathBuf::from(result.project_path);
        assert!(crate::ai::uai_plus::is_project(&dest));
        assert!(dest.join("main/plus_metadata.py").is_file());
        assert_eq!(hash(&source.join("main/main.py")).unwrap(),hash(&dest.join("main/main.py")).unwrap());
        assert!(scoped_file(Some(dest.to_str().unwrap()),source.join("main/main.py").to_str().unwrap()).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn save_as_and_scoped_files_reject_windows_junctions() {
        let base=tempfile::tempdir().unwrap();let source=base.path().join("source");let target=base.path().join("target");let copies=base.path().join("copies");
        for dir in [&source,&target,&copies] {std::fs::create_dir(dir).unwrap();}
        std::fs::write(target.join("keep.txt"),"other workspace").unwrap();
        let quote=|p:&Path|p.to_string_lossy().replace('\'',"''");
        let script=format!("New-Item -ItemType Junction -Path '{}' -Target '{}' -ErrorAction Stop | Out-Null",quote(&source.join("linked")),quote(&target));
        let status=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command",&script])
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().unwrap();
        assert!(status.success());
        assert!(copy_project(&source,&copies).is_err());assert!(!copies.join("source").exists());
        assert!(scoped_file(Some(source.to_str().unwrap()),"linked/keep.txt").is_err());
        assert_eq!(std::fs::read_to_string(target.join("keep.txt")).unwrap(),"other workspace");
    }

    #[cfg(windows)]
    #[test]
    fn scoped_files_reject_windows_aliases_and_alternate_streams() {
        let root=tempfile::tempdir().unwrap();std::fs::create_dir(root.path().join("main")).unwrap();
        for name in ["main/file.py:stream","main/CON.py","main/file.py.","main/file.py "] {
            assert!(scoped_file(Some(root.path().to_str().unwrap()),name).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn save_as_and_file_scope_reject_external_symlinks() {
        let base=tempfile::tempdir().unwrap();let root=base.path().join("work");let other=base.path().join("other");let copies=base.path().join("copies");
        for dir in [&root,&other,&copies] {std::fs::create_dir(dir).unwrap();}
        std::fs::write(other.join("private"),"keep").unwrap();std::os::unix::fs::symlink(&other,root.join("linked")).unwrap();
        assert!(copy_project(&root,&copies).is_err());assert!(!copies.join("work").exists());
        assert!(scoped_file(Some(root.to_str().unwrap()),"linked/private").is_err());
    }
}
