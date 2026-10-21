import { readFileSync, existsSync } from "node:fs";
import { createHash } from "node:crypto";

const read = path => readFileSync(path, "utf8");
const config = JSON.parse(read("src-tauri/tauri.conf.json"));
const windows = JSON.parse(read("src-tauri/tauri.windows.conf.json"));
const cargo = read("src-tauri/Cargo.toml");
const errors = [];
const packageManifest = JSON.parse(read("package.json"));
const packageLock = JSON.parse(read("package-lock.json"));
const rustVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];
const rootCargoLock = read("src-tauri/Cargo.lock").split("[[package]]").find(entry => /name = "vibe-kidbright"/.test(entry));
if (!rootCargoLock || /version = "([^"]+)"/.exec(rootCargoLock)?.[1] !== config.version) errors.push("Cargo.lock app version must match Tauri.");
if ([packageManifest.version, packageLock.version, packageLock.packages[""].version, rustVersion].some(version => version !== config.version)) errors.push("Tauri, npm, Cargo and lockfile app versions must match.");
for (const [file, expected] of Object.entries(JSON.parse(read("docs/ai-engineering/bundled-source-hashes.json")))) {
  const path = `resources/knowledge_base/${file}`;
  if (!existsSync(path) || createHash("sha256").update(read(path).replace(/\r\n/g, "\n")).digest("hex") !== expected) errors.push(`Public knowledge provenance changed: ${file}. Review and update the bundled source manifest before release.`);
}
if (config.bundle.windows.webviewInstallMode.type !== "offlineInstaller") errors.push("Windows installer must include offline WebView2.");
if (config.bundle.windows.nsis.installMode !== "currentUser") errors.push("Installer must support per-user installation.");
// Directory mappings preserve nested names. Globs flatten them and collide on README.md.
for (const resource of ["../resources/knowledge_base", "../resources/embedding"]) {
  if (!config.bundle.resources[resource]) errors.push(`Missing packaged resource mapping: ${resource}`);
}
if (Object.keys(config.bundle.resources).some(path => path.includes("knowledge_base/") && path.includes("*"))) errors.push("Knowledge resources must preserve directories; do not flatten a recursive glob.");
if (!("../resources/runtime/windows/*.dll" in windows.bundle.resources)) errors.push("MSVC runtime must be included for Windows.");
if (!/fastembed\s*=\s*\{[^\n]*default-features\s*=\s*false/.test(cargo)) errors.push("Disable fastembed network/hub default features.");
for (const file of ["src/main.tsx", "src/CodeEditor.tsx", "src/ToolchainSetup.tsx", "src/WikiView.tsx", "src/index.css"]) {
  const source = read(file);
  if (/@import\s+url\(['"]?https?:\/\//.test(source)) errors.push(`Remote stylesheet dependency in ${file}`);
}
if (!read("src/main.tsx").includes("getWorker(") || !read("src/main.tsx").includes("loader.config({ monaco })")) errors.push("Monaco and workers must be bundled locally.");
if (/\bfetch\(/.test(read("src/WikiView.tsx"))) errors.push("Wiki provider requests must use native IPC, not browser fetch.");
for (const file of ["resources/embedding/LICENSE.txt", "resources/embedding/UPSTREAM.txt"]) {
  if (!existsSync(file)) errors.push(`Missing model notice: ${file}`);
}
if (errors.length) throw new Error(errors.join("\n"));
console.log("Standalone packaging policy checked: local assets, local workers, native Wiki bridge and offline Windows prerequisites.");
