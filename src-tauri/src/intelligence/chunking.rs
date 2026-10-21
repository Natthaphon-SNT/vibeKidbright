//! C syntax units with an explicit lexical fallback; neither establishes semantics.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceChunk {
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub kind: String,
    pub symbols: Vec<String>,
    pub includes: Vec<String>,
    pub parent_start_line: usize,
    pub parent_end_line: usize,
    #[serde(default)] pub start_byte: usize,
    #[serde(default)] pub end_byte: usize,
    #[serde(default)] pub parent_start_byte: usize,
    #[serde(default)] pub parent_end_byte: usize,
    #[serde(default)] pub signature: Option<String>,
    #[serde(default)] pub parser: String,
    #[serde(default)] pub syntax_error: bool,
}

pub fn chunk_source(path: &str, source: &str, max_chars: usize) -> Vec<SourceChunk> {
    if path.ends_with(".c") || path.ends_with(".h") {
        if let Some(chunks) = super::c_parser::chunks(source, max_chars) { return chunks; }
    }
    let mut chunks = lexical_chunks(path, source, max_chars);
    let offsets: Vec<_> = std::iter::once(0).chain(source.match_indices('\n').map(|(i,_)|i+1)).collect();
    for c in &mut chunks {
        c.start_byte = offsets[c.start_line-1];
        c.end_byte = offsets.get(c.end_line).copied().unwrap_or(source.len());
        c.parent_start_byte = offsets[c.parent_start_line-1];
        c.parent_end_byte = offsets.get(c.parent_end_line).copied().unwrap_or(source.len());
        c.content = source[c.start_byte..c.end_byte].into();
    }
    chunks
}

fn lexical_chunks(path: &str, source: &str, max_chars: usize) -> Vec<SourceChunk> {
    let lines: Vec<_> = source.lines().collect();
    let c = path.ends_with(".c") || path.ends_with(".h");
    let includes: Vec<_> = if c {
        lines
            .iter()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("#include")
                    .map(|s| s.trim().to_owned())
            })
            .take(64).collect()
    } else {
        vec![]
    };
    let mut ranges = Vec::new();
    let mut start = 0;
    let (mut braces, mut parentheses, mut block_comment, mut quote, mut escaped) =
        (0usize, 0usize, false, None, false);
    let mut macro_continuation = false;
    for (index, line) in lines.iter().enumerate() {
        if c {
            let bytes = line.as_bytes();
            let mut i = 0;
            let directive =
                braces == 0 && (macro_continuation || line.trim_start().starts_with('#'));
            macro_continuation = directive && line.trim_end().ends_with('\\');
            if !directive {
                while i < bytes.len() {
                    let b = bytes[i];
                    let next = bytes.get(i + 1).copied();
                    if block_comment {
                        if b == b'*' && next == Some(b'/') {
                            block_comment = false;
                            i += 2;
                            continue;
                        }
                    } else if let Some(q) = quote {
                        if escaped {
                            escaped = false;
                        } else if b == b'\\' {
                            escaped = true;
                        } else if b == q {
                            quote = None;
                        }
                    } else {
                        match (b, next) {
                            (b'/', Some(b'/')) => break,
                            (b'/', Some(b'*')) => {
                                block_comment = true;
                                i += 2;
                                continue;
                            }
                            (b'"' | b'\'', _) => quote = Some(b),
                            (b'{', _) => braces += 1,
                            (b'}', _) => braces = braces.saturating_sub(1),
                            (b'(', _) => parentheses += 1,
                            (b')', _) => parentheses = parentheses.saturating_sub(1),
                            _ => (),
                        }
                    }
                    i += 1;
                }
            }
            let trimmed = line.trim_end();
            if braces == 0
                && parentheses == 0
                && !block_comment
                && quote.is_none()
                && ((directive && !trimmed.ends_with('\\'))
                    || trimmed.ends_with(';')
                    || trimmed.ends_with('}'))
            {
                ranges.push((start, index + 1));
                start = index + 1;
            }
        } else if line.trim().is_empty() && index > start {
            ranges.push((start, index + 1));
            start = index + 1;
        }
    }
    if start < lines.len() {
        ranges.push((start, lines.len()));
    }
    let symbol_re = regex::Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap();
    let mut chunks = Vec::new();
    for (begin, end) in ranges {
        let body = lines[begin..end].join("\n");
        let kind = if !c {
            "text"
        } else if body.contains("typedef") {
            "typedef"
        } else if body.contains("struct ") {
            "struct_or_use"
        } else if body.contains("enum ") {
            "enum_or_use"
        } else if body.contains("#define") {
            "macro"
        } else if body.contains('(') && body.contains('{') {
            "function_candidate"
        } else if body.contains('(') {
            "declaration_or_use"
        } else {
            "declaration"
        };
        let mut symbols: Vec<String> = symbol_re
            .captures_iter(&body)
            .map(|cap| cap[1].to_owned())
            .filter(|s| !matches!(s.as_str(), "if" | "while" | "for" | "switch" | "sizeof"))
            .take(128)
            .collect();
        symbols.sort();
        symbols.dedup();
        // Keep complete units when possible. Large units have line subchunks with
        // explicit parent ranges; a single long line is retained, never byte-sliced.
        let mut sub = begin;
        let mut length = 0;
        for index in begin..end {
            if length > 0 && length + lines[index].chars().count() > max_chars.max(128) {
                push(
                    &mut chunks,
                    &lines,
                    sub,
                    index,
                    begin,
                    end,
                    kind,
                    &symbols,
                    &includes,
                );
                sub = index;
                length = 0;
            }
            length += lines[index].chars().count() + 1;
        }
        push(
            &mut chunks,
            &lines,
            sub,
            end,
            begin,
            end,
            kind,
            &symbols,
            &includes,
        );
        if chunks.len()>4096{break;}
    }
    chunks
}

#[allow(clippy::too_many_arguments)]
fn push(
    out: &mut Vec<SourceChunk>,
    lines: &[&str],
    start: usize,
    end: usize,
    parent_start: usize,
    parent_end: usize,
    kind: &str,
    symbols: &[String],
    includes: &[String],
) {
    let content = lines[start..end].join("\n");
    if content.trim().is_empty() {
        return;
    }
    out.push(SourceChunk {
        content,
        start_line: start + 1,
        end_line: end,
        kind: kind.into(),
        symbols: symbols.into(),
        includes: includes.into(),
        parent_start_line: parent_start + 1,
        parent_end_line: parent_end,
        start_byte: 0, end_byte: 0, parent_start_byte: 0, parent_end_byte: 0,
        signature: None, parser: "lexical-fallback".into(), syntax_error: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn braces_in_strings_comments_and_macros_do_not_split_functions() {
        let source = "#include <stdio.h>\n#define OPEN {\n// comment }\nint demo(int n) {\n puts(\"} ไทย\"); /* { */\n if(n) { n--; }\n return n;\n}\nint next(void);";
        let chunks = chunk_source("main.c", source, 2000);
        let function = chunks
            .iter()
            .find(|c| c.content.contains("int demo"))
            .unwrap();
        assert!(function.content.contains("return n;\n}"));
        assert_eq!(function.end_line, 8);
        assert!(function.includes.contains(&"<stdio.h>".into()));
        assert!(function.symbols.contains(&"demo".into()));
    }
    #[test]
    fn large_units_keep_parent_and_original_line_ranges() {
        let source = format!(
            "void large(void) {{\n{}}}\n",
            "  work(); // ไทย\n".repeat(80)
        );
        let chunks = chunk_source("main.c", &source, 150);
        assert!(chunks.len() > 1);
        assert!(chunks
            .iter()
            .all(|c| c.parent_start_line == 1 && c.parent_end_line == 82));
        for c in chunks {
            assert_eq!(c.content, source[c.start_byte..c.end_byte]);
        }
    }
}
