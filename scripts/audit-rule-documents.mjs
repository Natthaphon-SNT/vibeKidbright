// Inventory documents as data. Never execute a fenced snippet or reference file.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
let output = resolve(root, "docs/research/rule-documents-2026-10-16.json");
let check = false;
for (let index = 2; index < process.argv.length; index++) {
  const argument = process.argv[index];
  if (argument === "--check") check = true;
  else if (argument === "--output" && process.argv[index + 1] && !process.argv[index + 1].startsWith("--")) output = resolve(root, process.argv[++index]);
  else throw new Error(`Unknown or incomplete argument: ${argument}; usage: node scripts/audit-rule-documents.mjs [--output <json>] [--check]`);
}
const digest = (value) => createHash("sha256").update(value).digest("hex");
const fatalDecoder = new TextDecoder("utf-8", { fatal: true });
const replacementDecoder = new TextDecoder("utf-8");

// TextDecoder reports an invalid stream without its offset. Retain the schema's
// byte_offset by locating the start of the first malformed UTF-8 sequence.
function invalidUtf8Offset(bytes) {
  for (let index = 0; index < bytes.length;) {
    const lead = bytes[index];
    if (lead <= 0x7f) { index++; continue; }
    const length = lead >= 0xc2 && lead <= 0xdf ? 2 : lead >= 0xe0 && lead <= 0xef ? 3 : lead >= 0xf0 && lead <= 0xf4 ? 4 : 0;
    if (!length || index + length > bytes.length) return index;
    for (let next = 1; next < length; next++) {
      const byte = bytes[index + next];
      if (byte < 0x80 || byte > 0xbf) return index;
      if (next === 1 && ((lead === 0xe0 && byte < 0xa0) || (lead === 0xed && byte > 0x9f) || (lead === 0xf0 && byte < 0x90) || (lead === 0xf4 && byte > 0x8f))) return index;
    }
    index += length;
  }
  return null;
}

function inspectDocument(path) {
  const raw = readFileSync(resolve(root, path));
  let source;
  let encodingError = null;
  try {
    source = fatalDecoder.decode(raw);
  } catch (error) {
    encodingError = { byte_offset: invalidUtf8Offset(raw), reason: error.message };
    source = replacementDecoder.decode(raw);
  }
  // Match Python splitlines(), including CRLF and a final newline without a
  // spurious empty line; the decoder strips a UTF-8 BOM as utf-8-sig did.
  const lines = source ? source.split(/\r\n|[\n\v\f\r\u001c-\u001e\u0085\u2028\u2029]/u) : [];
  if (lines.at(-1) === "") lines.pop();
  const fences = [];
  let opened = null;
  for (const [index, line] of lines.entries()) {
    const match = /^\s*(`{3,}|~{3,})(.*)$/.exec(line);
    if (!match) continue;
    const [, marker, info] = match;
    const number = index + 1;
    if (!opened) {
      opened = { start_line: number, marker, language: info.trim().split(/\s+/)[0] || "unspecified" };
    } else if (marker[0] === opened.marker[0] && marker.length >= opened.marker.length && !info.trim()) {
      const content = lines.slice(opened.start_line, number - 1).join("\n");
      fences.push({ start_line: opened.start_line, end_line: number, language: opened.language, content_sha256: digest(content), lines: number - opened.start_line - 1, validation: "document_snippet_not_independently_compiled" });
      opened = null;
    }
  }
  const category = path.startsWith("resources/knowledge_base/") ? "bundled_knowledge" : path.startsWith("knowledge_base/") ? "source_knowledge" : path.endsWith("/system_prompt.txt") ? "system_prompt" : "repository_document";
  return { path, category, bytes: raw.length, sha256: digest(raw), line_count: lines.length, utf8_error: encodingError, replacement_character_lines: lines.flatMap((line, index) => line.includes("\ufffd") ? [index + 1] : []), unclosed_fence_line: opened?.start_line ?? null, code_fences: fences };
}

const gitFiles = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], { cwd: root, encoding: "utf8" }).split("\0");
const paths = [...new Set([
  ...gitFiles.filter((path) => path.endsWith(".md") && existsSync(resolve(root, path)) && statSync(resolve(root, path)).isFile()),
  "src-tauri/src/ai/system_prompt.txt",
])].sort();
const documents = paths.map(inspectDocument);
const languages = new Map();
for (const document of documents) for (const fence of document.code_fences) languages.set(fence.language, (languages.get(fence.language) ?? 0) + 1);
const source = new Map(documents.filter((row) => row.category === "source_knowledge").map((row) => [row.path, row]));
const bundle = new Map(documents.filter((row) => row.category === "bundled_knowledge").map((row) => [row.path.slice("resources/".length), row]));
const mismatches = [...new Set([...source.keys(), ...bundle.keys()])].sort().filter((path) => !source.has(path) || !bundle.has(path) || source.get(path).sha256 !== bundle.get(path).sha256);
const record = {
  schema_version: 1,
  audit_date: "2026-10-16",
  method: "Read-only UTF-8 and fenced-code inventory over Git tracked/unignored Markdown plus system_prompt; no snippets executed.",
  limits: ["Code fences are examples; compilation and behavioral equivalence require separate evidence.", "Historical reports and bundled copies are counted separately, not as additional implemented features."],
  summary: { documents: documents.length, source_knowledge_documents: source.size, bundled_knowledge_documents: bundle.size, code_fences: [...languages.values()].reduce((total, count) => total + count, 0), fences_by_language: Object.fromEntries([...languages.entries()].sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)), invalid_utf8_documents: documents.filter((row) => row.utf8_error !== null).length, unclosed_fences: documents.filter((row) => row.unclosed_fence_line !== null).length },
  markdown_bundle_mismatches: mismatches,
  documents,
};
// Keep ASCII-only JSON, matching the previous Python artifact's serialization.
const serialized = `${JSON.stringify(record, null, 2).replace(/[\u007f-\uffff]/g, (character) => `\\u${character.charCodeAt(0).toString(16).padStart(4, "0")}`)}\n`;
const problems = documents.filter((row) => row.utf8_error || row.unclosed_fence_line).map((row) => row.path);
console.log(JSON.stringify(record.summary));
if (problems.length) console.error(JSON.stringify({ documents_to_review: problems }));
if (mismatches.length) console.error(JSON.stringify({ markdown_bundle_mismatches: mismatches }));
if (check) {
  if (!existsSync(output) || readFileSync(output, "utf8") !== serialized) throw new Error("Document inventory is stale; run node scripts/audit-rule-documents.mjs");
  if (problems.length || mismatches.length) throw new Error("Rule documents have invalid encoding, unclosed fences or an out-of-sync Markdown bundle");
} else {
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, serialized, "utf8");
}
