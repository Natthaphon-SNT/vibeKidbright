//! Deterministic test export only: refuses an existing directory.
use std::path::Path;
use tauri_app_lib::intelligence::block_to_c::{project_files, Program};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Use export_block_fixture <fixture.json> <new-project-directory>".into());
    }
    let program: Program = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let files = project_files(&program)?;
    let directory = Path::new(&args[2]);
    std::fs::create_dir(directory)?;
    std::fs::write(directory.join("CMakeLists.txt"),"cmake_minimum_required(VERSION 3.16)\ninclude($ENV{IDF_PATH}/tools/cmake/project.cmake)\nproject(kidbright_block_pilot)\n")?;
    std::fs::write(
        directory.join("sdkconfig.defaults"),
        "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n",
    )?;
    for (relative, content) in files {
        let path = directory.join(relative);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)?;
    }
    println!(
        "Exported {} (physical behavior unverified)",
        directory.display()
    );
    Ok(())
}
