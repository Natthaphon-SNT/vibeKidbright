// Export the same deterministic files used by the AI tool for build checks.
#[path = "../src/ai/compatibility.rs"]
#[allow(dead_code)]
mod compatibility;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Usage: export_kidbright_reference <new-directory> <board-profile> <example>".into());
    }
    let files = compatibility::files(&args[2], &args[3])?;
    let directory = std::path::Path::new(&args[1]);
    std::fs::create_dir(directory)?; // Existing projects must never be overwritten.
    std::fs::write(directory.join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.16)\ninclude($ENV{IDF_PATH}/tools/cmake/project.cmake)\nproject(kidbright_reference)\n")?;
    std::fs::write(directory.join("sdkconfig.defaults"), "CONFIG_IDF_TARGET=\"esp32\"\nCONFIG_FREERTOS_HZ=1000\n")?;
    for (relative, content) in files {
        let path = directory.join(relative);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, content)?;
    }
    compatibility::hide_internal_metadata(directory)?;
    println!("Exported {}", directory.display());
    Ok(())
}
