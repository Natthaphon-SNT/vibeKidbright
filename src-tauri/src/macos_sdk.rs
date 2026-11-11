//! Bounded SDK checks used by the 3.7.1 macOS backport and installed-app probe.
use std::{io::Read, process::{Command, Stdio}, sync::{Arc,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
pub(crate) struct Captured {pub success:bool,pub stdout:String,pub stderr:String}
pub(crate) fn execute(mut command:Command, emit:Arc<dyn Fn(String,bool)+Send+Sync>, cancel:Arc<AtomicBool>, timeout:Duration)->Result<Captured,String>{
    let output=tempfile::tempfile().map_err(|e|e.to_string())?;
    let error=tempfile::tempfile().map_err(|e|e.to_string())?;
    command.stdout(output.try_clone().map_err(|e|e.to_string())?).stderr(error.try_clone().map_err(|e|e.to_string())?).stdin(Stdio::null());
    #[cfg(unix)] {use std::os::unix::process::CommandExt;command.process_group(0);}
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let mut child=command.spawn().map_err(|e|e.to_string())?;
    let started=Instant::now();
    let status=loop {
        if cancel.load(Ordering::SeqCst)||started.elapsed()>timeout {
            #[cfg(unix)] {let _=Command::new("/bin/kill").arg("-KILL").arg(format!("-{}",child.id())).status();}
            let _=child.kill();let _=child.wait();return Err("SDK probe timed out or was cancelled".into());
        }
        if let Some(status)=child.try_wait().map_err(|e|e.to_string())?{break status;}
        std::thread::sleep(Duration::from_millis(25));
    };
    let read=|mut file:std::fs::File|->Result<String,String>{use std::io::{Seek,SeekFrom};file.seek(SeekFrom::Start(0)).map_err(|e|e.to_string())?;let mut bytes=Vec::new();file.take(2*1024*1024).read_to_end(&mut bytes).map_err(|e|e.to_string())?;Ok(String::from_utf8_lossy(&bytes).into_owned())};
    let stdout=read(output)?;let stderr=read(error)?;
    for line in stdout.lines(){emit(line.into(),false);}for line in stderr.lines(){emit(line.into(),true);}
    Ok(Captured{success:status.success(),stdout,stderr})
}
pub fn self_check(idf:&std::path::Path,tools:&std::path::Path,scratch:&std::path::Path)->Result<serde_json::Value,String>{
    crate::esp_idf::sdk_build_self_check(idf,tools,scratch)
}
