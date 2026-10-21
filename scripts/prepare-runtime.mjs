// Build-time preparation only. Installed applications never execute this script.
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";

const download = process.argv.includes("--download");
const check = process.argv.includes("--check");
const modelRoot = resolve("resources/embedding");
const manifest = JSON.parse(readFileSync(join(modelRoot, "manifest.json"), "utf8"));
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
for (const file of manifest.files) {
  const target = join(modelRoot, file.file);
  if (!existsSync(target) && download) {
    const url = `https://huggingface.co/${manifest.model}/resolve/${manifest.revision}/${file.source}`;
    const response = await fetch(url, { signal: AbortSignal.timeout(180_000) });
    if (!response.ok) throw new Error(`Cannot prepare ${file.file}: HTTP ${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (digest(bytes) !== file.sha256) throw new Error(`Downloaded resource hash mismatch: ${file.file}`);
    writeFileSync(target, bytes);
  }
  if (!existsSync(target) || digest(readFileSync(target)) !== file.sha256) {
    throw new Error(`Missing or changed runtime resource: ${file.file}. Run npm run prepare:runtime -- --download on the build machine.`);
  }
}

if (process.platform === "win32") {
  const target = resolve("resources/runtime/windows");
  mkdirSync(target, { recursive: true });
  if (!check) {
    const vswhere = join(process.env["ProgramFiles(x86)"] || "C:/Program Files (x86)", "Microsoft Visual Studio/Installer/vswhere.exe");
    const installation = execFileSync(vswhere, ["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], { encoding: "utf8" }).trim();
    const redist = join(installation, "VC/Redist/MSVC");
    const versions = readdirSync(redist).filter(name => /^\d/.test(name)).sort((a, b) => a.localeCompare(b, undefined, { numeric: true })).reverse();
    const source = versions.map(version => join(redist, version, "x64/Microsoft.VC143.CRT")).find(existsSync);
    if (!source) throw new Error("MSVC x64 redistributable runtime not found on the build machine.");
    const files = readdirSync(source).filter(name => name.endsWith(".dll"));
    for (const name of files) copyFileSync(join(source, name), join(target, name));
    writeFileSync(join(target, "manifest.json"), JSON.stringify({ architecture: "x64", files: files.map(name => ({ file: basename(name), sha256: digest(readFileSync(join(target, name))) })) }, null, 2) + "\n");
  }
  const runtime = JSON.parse(readFileSync(join(target, "manifest.json"), "utf8"));
  for (const file of runtime.files) {
    if (digest(readFileSync(join(target, file.file))) !== file.sha256) throw new Error(`MSVC runtime hash mismatch: ${file.file}`);
  }
  for (const name of ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"]) {
    if (!runtime.files.some(file => file.file === name)) throw new Error(`Required runtime missing: ${name}`);
  }
}
console.log("Offline model and platform runtime assets verified. No user settings or toolchain files bundled.");
