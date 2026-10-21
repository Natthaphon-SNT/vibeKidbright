//! Debug protocol fault fixture, not a language server or semantic-quality test.
#[cfg(debug_assertions)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::{json, Value};
    use std::io::{BufRead, Read, Write};
    let exe = std::env::current_exe()?;
    let mode = exe
        .parent()
        .and_then(|p| p.file_name())
        .unwrap()
        .to_string_lossy();
    let mut input = std::io::BufReader::new(std::io::stdin());
    let mut output = std::io::stdout();
    let mut documents = std::collections::BTreeMap::<String, String>::new();
    loop {
        let mut size = None;
        loop {
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                return Ok(());
            }
            if line.trim().is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                size = Some(value.trim().parse::<usize>()?);
            }
        }
        let size = size
            .filter(|s| *s <= 4 * 1024 * 1024)
            .ok_or("Invalid fixture frame")?;
        let mut data = vec![0; size];
        input.read_exact(&mut data)?;
        let message: Value = serde_json::from_slice(&data)?;
        let response = match message["method"].as_str() {
            Some("initialize") => Some(
                json!({"jsonrpc":"2.0","id":message["id"],"result":{"capabilities":{"positionEncoding":"utf-16"}}}),
            ),
            Some("textDocument/didOpen") => {
                let doc = &message["params"]["textDocument"];
                documents.insert(
                    doc["uri"].as_str().ok_or("Missing URI")?.into(),
                    doc["text"].as_str().ok_or("Missing text")?.into(),
                );
                Some(
                    json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":doc["uri"],"diagnostics":[]}}),
                )
            }
            Some("textDocument/definition" | "textDocument/references") => {
                let uri = message["params"]["textDocument"]["uri"]
                    .as_str()
                    .ok_or("Missing selected URI")?;
                let text = documents.get(uri).ok_or("Missing open document")?;
                let range = if mode.ends_with("surrogate") {
                    let byte = text.find('😀').ok_or("Fixture needs emoji")?;
                    let prefix = &text[..byte];
                    let line = prefix.bytes().filter(|b| *b == b'\n').count();
                    let column = prefix.rsplit('\n').next().unwrap().encode_utf16().count() + 1;
                    json!({"start":{"line":line,"character":column},"end":{"line":line,"character":column}})
                } else if mode.ends_with("reversed") {
                    json!({"start":{"line":0,"character":1},"end":{"line":0,"character":0}})
                } else {
                    let line = text.bytes().filter(|b| *b == b'\n').count() + 1;
                    json!({"start":{"line":line,"character":0},"end":{"line":line,"character":0}})
                };
                Some(
                    json!({"jsonrpc":"2.0","id":message["id"],"result":[{"uri":uri,"range":range}]}),
                )
            }
            _ => None,
        };
        if let Some(response) = response {
            let body = serde_json::to_vec(&response)?;
            write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
            output.write_all(&body)?;
            output.flush()?;
        }
    }
}
#[cfg(not(debug_assertions))]
fn main() {
    panic!("Protocol fault fixtures are debug-only");
}
