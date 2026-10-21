#[cfg(debug_assertions)]
fn main() -> Result<(),String> {
    let args:Vec<_>=std::env::args().collect();
    if args.len()!=5 {return Err("desktop_evaluation <new-run-directory> <idf-path> <tools-path> <loopback-debug-port>".into());}
    tauri_app_lib::run_isolated_desktop_evaluation(std::path::Path::new(&args[1]),std::path::Path::new(&args[2]),std::path::Path::new(&args[3]),args[4].parse().map_err(|_|"Invalid port")?)
}
#[cfg(not(debug_assertions))]
fn main() { panic!("Desktop evaluation is available only in debug builds"); }
