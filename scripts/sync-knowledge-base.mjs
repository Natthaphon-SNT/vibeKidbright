import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync } from "node:fs";
import { dirname, extname, join, relative } from "node:path";

const source = "knowledge_base";
const destination = "resources/knowledge_base";
const write = process.argv.includes("--write");
const allowed = new Set([".c", ".h", ".md", ".txt", ".pdf", ".png", ".jpg", ".jpeg", ".webp"]);
function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    if (entry.name.startsWith(".")) return [];
    const path = join(directory, entry.name);
    return entry.isDirectory() ? files(path) : allowed.has(extname(path).toLowerCase()) ? [path] : [];
  });
}
const expected = new Set();
const differences = [];
for (const file of files(source)) {
  const name = relative(source, file);
  expected.add(name);
  const target = join(destination, name);
  if (!existsSync(target) || !readFileSync(file).equals(readFileSync(target))) {
    if (write) {
      mkdirSync(dirname(target), { recursive: true });
      copyFileSync(file, target);
    } else differences.push(name);
  }
}
for (const file of files(destination)) {
  const name = relative(destination, file);
  if (!expected.has(name)) differences.push(`${name} (stale bundled file; remove or restore its source explicitly)`);
}
if (differences.length) {
  console.error(`Knowledge base is out of sync:\n${differences.join("\n")}\nRun npm run sync:kb for missing/changed files.`);
  process.exit(1);
}
console.log(`Knowledge base ${write ? "synchronized" : "verified"}: ${expected.size} source files.`);
