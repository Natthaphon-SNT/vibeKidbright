fn main() {
    tauri_build::build();
    // Tauri's main binary already receives its application manifest. GUI-linked
    // development examples also require Common Controls v6 (TaskDialogIndirect).
    #[cfg(windows)] {
        let manifest=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/desktop-evaluation.manifest");
        println!("cargo:rerun-if-changed={}",manifest.display());
        println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-examples=/MANIFESTINPUT:{}",manifest.display());
    }
}
