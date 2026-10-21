//! Fault injection only, not clangd or semantic correctness evidence.
fn main() {
    let exe=std::env::current_exe().unwrap();
    std::fs::write(exe.parent().unwrap().join("stall-pid.txt"),std::process::id().to_string()).unwrap();
    loop {std::thread::park_timeout(std::time::Duration::from_secs(60));}
}
