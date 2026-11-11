// Compile the actual portable driver and both adapter branches against contract
// fixtures; then compare native/Python codecs across their complete valid ranges.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(".");
const outputRoot = resolve("src-tauri/target/ikb1z-tests");
mkdirSync(outputRoot, { recursive: true });
const directory = mkdtempSync(join(outputRoot, "run-"));
const drivers = resolve("knowledge_base/drivers");
const stubs = resolve("src-tauri/tests/ikb1z_stubs");
const core = join(drivers, "ikb1z.c");
const adapter = join(drivers, "ikb1z_esp_idf.c");
const coreTest = resolve("src-tauri/tests/ikb1z_test.c");
const adapterTest = resolve("src-tauri/tests/ikb1z_idf_test.c");
const win = process.platform === "win32";
let vsInstall;
if (win) {
  const vswhere = join(process.env["ProgramFiles(x86)"] || "C:/Program Files (x86)", "Microsoft Visual Studio/Installer/vswhere.exe");
  if (!existsSync(vswhere)) throw new Error("MSVC C build tools are required.");
  vsInstall = execFileSync(vswhere, ["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], { encoding: "utf8" }).trim();
  if (!vsInstall) throw new Error("No MSVC C compiler installation found.");
}
function compile(name, sources, { legacy = false, shared = false, fixture = false } = {}) {
  const artifact = join(directory, name + (shared ? win ? ".dll" : ".so" : win ? ".exe" : ""));
  if (win) {
    const definition = join(directory, "codec.def");
    if (shared) writeFileSync(definition, "EXPORTS\nikb1z_encode_io\nikb1z_encode_motor\nikb1z_encode_servo\nikb1z_decode_adc10\n");
    const batch = join(directory, name + ".cmd");
    writeFileSync(batch, ["@echo off",
      `call "${join(vsInstall, "Common7/Tools/VsDevCmd.bat")}" -no_logo -arch=x64`,
      "if errorlevel 1 exit /b 1",
      `cl /nologo /W4 /WX /std:c11 /Od ${shared ? "/LD" : ""} ${legacy ? "/DIKB1Z_USE_LEGACY_I2C" : ""} /I"${drivers}" ${fixture ? `/I"${stubs}"` : ""} ${sources.map(path => `"${path}"`).join(" ")} /Fe:"${artifact}" ${shared ? `/link /DEF:"${definition}"` : ""}`,
      "exit /b %errorlevel%"].join("\r\n"));
    execFileSync(process.env.ComSpec || "cmd.exe", ["/d", "/c", batch], { cwd: directory, stdio: "inherit" });
  } else {
    execFileSync(process.env.CC || "cc", ["-std=c11", "-Wall", "-Wextra", "-Werror", "-pedantic",
      ...(shared ? ["-shared", "-fPIC"] : []), ...(legacy ? ["-DIKB1Z_USE_LEGACY_I2C"] : []),
      "-I", drivers, ...(fixture ? ["-I", stubs] : []), ...sources, "-o", artifact], { cwd: directory, stdio: "inherit" });
  }
  if (!shared) execFileSync(artifact, [], { cwd: directory, stdio: "inherit" });
  return artifact;
}
compile("protocol", [core, coreTest]);
compile("idf-modern-contract", [core, adapter, adapterTest], { fixture: true });
compile("idf-legacy-contract", [core, adapter, adapterTest], { fixture: true, legacy: true });
const library = compile("codec", [core], { shared: true });
execFileSync(process.env.PYTHON || "python", ["-m", "unittest", "discover", "-s", "scripts/tests", "-p", "test_ikb1z.py", "-v"], {
  cwd: root, stdio: "inherit", env: { ...process.env, IKB1Z_TEST_LIBRARY: library },
});
console.log("iKB-1Z host/contract checks passed; no physical board I/O performed.");
