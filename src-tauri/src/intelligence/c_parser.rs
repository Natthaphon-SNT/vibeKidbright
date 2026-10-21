//! Statically linked, version-pinned C grammar. Byte offsets refer to the exact
//! UTF-8 input (including CRLF); syntax recovery is reported, never semantic proof.
use super::SourceChunk;
use tree_sitter::{Node, Parser};

fn identifiers(node: Node<'_>, source: &str, out: &mut Vec<String>) {
    let mut cursor = node.walk();
    // Cursor traversal avoids stack overflow on adversarially nested C. Symbol
    // metadata is bounded; the exact source range is still retained separately.
    let mut visited = 0;
    loop {
        visited += 1;
        let current = cursor.node();
        if matches!(
            current.kind(),
            "identifier" | "type_identifier" | "field_identifier"
        ) {
            out.push(source[current.byte_range()].to_owned());
        }
        if out.len() >= 128 || visited >= 100000 {
            break;
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

pub(super) fn chunks(source: &str, max_chars: usize) -> Option<Vec<SourceChunk>> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_c::LANGUAGE.into()).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    let includes: Vec<String> = source
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("#include")
                .map(|s| s.trim().to_owned())
        })
        .take(64)
        .collect();
    let lines: Vec<_> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let mut out = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        let mut symbols = Vec::new();
        identifiers(node, source, &mut symbols);
        symbols.sort();
        symbols.dedup();
        let signature = if node.kind() == "function_definition" {
            node.child_by_field_name("body").map(|body| {
                source[node.start_byte()..body.start_byte()]
                    .trim()
                    .to_owned()
            })
        } else {
            None
        };
        let mut start = node.start_byte();
        let mut end = start;
        let mut count = 0;
        for line in source[node.byte_range()].split_inclusive('\n') {
            let length = line.chars().count();
            if count > 0 && count + length > max_chars.max(128) {
                emit(
                    &mut out,
                    source,
                    &lines,
                    node,
                    start,
                    end,
                    &symbols,
                    &includes,
                    signature.clone(),
                );
                if out.len() > 4096 {
                    return Some(out);
                }
                start = end;
                count = 0;
            }
            end += line.len();
            count += length;
        }
        emit(
            &mut out, source, &lines, node, start, end, &symbols, &includes, signature,
        );
        if out.len() > 4096 {
            return Some(out);
        }
    }
    Some(out)
}

#[allow(clippy::too_many_arguments)]
fn emit(
    out: &mut Vec<SourceChunk>,
    source: &str,
    lines: &[usize],
    node: Node<'_>,
    start: usize,
    end: usize,
    symbols: &[String],
    includes: &[String],
    signature: Option<String>,
) {
    if start >= end || source[start..end].trim().is_empty() {
        return;
    }
    let line = |offset: usize| lines.partition_point(|&start| start <= offset);
    out.push(SourceChunk {
        content: source[start..end].into(),
        start_line: line(start),
        end_line: line(end - 1),
        start_byte: start,
        end_byte: end,
        parent_start_byte: node.start_byte(),
        parent_end_byte: node.end_byte(),
        parent_start_line: node.start_position().row + 1,
        parent_end_line: line(node.end_byte().saturating_sub(1)),
        kind: node.kind().into(),
        symbols: symbols.into(),
        includes: includes.into(),
        signature,
        parser: "tree-sitter-c@0.24.2/tree-sitter@0.27.1".into(),
        syntax_error: node.has_error() || node.is_error() || node.is_missing(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_crlf_utf8_offsets_and_structural_metadata() {
        let source = "#include <stdio.h>\r\ntypedef struct {int x;} Point;\r\nint demo(int n) {\r\n puts(\"ไทย }\");\r\n return n;\r\n}\r\n";
        let chunks = chunks(source, 2000).unwrap();
        let function = chunks
            .iter()
            .find(|c| c.kind == "function_definition")
            .unwrap();
        assert_eq!(function.signature.as_deref(), Some("int demo(int n)"));
        assert_eq!(function.start_line, 3);
        assert_eq!(function.end_line, 6);
        assert!(!function.syntax_error);
        for chunk in chunks {
            assert_eq!(chunk.content, source[chunk.start_byte..chunk.end_byte]);
        }
    }
    #[test]
    fn malformed_source_is_reported_and_never_panics_or_shifts_offsets() {
        let source = "int broken( { ไทย\nvoid okay(void) {}";
        let chunks = chunks(source, 128).unwrap();
        assert!(chunks.iter().any(|c| c.syntax_error));
        for c in chunks {
            assert_eq!(c.content, source[c.start_byte..c.end_byte]);
        }
    }
}
