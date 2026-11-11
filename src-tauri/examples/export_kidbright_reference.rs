// Export the same deterministic files used by the AI tool for build checks.
use tauri_app_lib::reference_export;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        return Err(
            "Usage: export_kidbright_reference <new-directory> <board-profile> <example>".into(),
        );
    }
    let directory = std::path::Path::new(&args[1]);
    reference_export::export(directory, &args[2], &args[3])?;
    println!("Exported {}", directory.display());
    Ok(())
}
