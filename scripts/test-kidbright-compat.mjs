import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const directory = mkdtempSync(join(tmpdir(), "kidbright-core-"));
const include = resolve("knowledge_base/compatibility");
const source = join(include, "kb_core.c");
const test = resolve("src-tauri/tests/kidbright_core_test.c");
const executable = join(directory, process.platform === "win32" ? "core-test.exe" : "core-test");
if (process.platform === "win32") {
  const vswhere = join(process.env["ProgramFiles(x86)"] || "C:/Program Files (x86)", "Microsoft Visual Studio/Installer/vswhere.exe");
  if (!existsSync(vswhere)) throw new Error("MSVC Build Tools are required for the C host tests.");
  const install = execFileSync(vswhere, ["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], { encoding: "utf8" }).trim();
  if (!install) throw new Error("No MSVC C compiler installation found.");
  const batch = join(directory, "compile.cmd");
  writeFileSync(batch, [
    "@echo off",
    `call "${join(install, "Common7/Tools/VsDevCmd.bat")}" -no_logo -arch=x64`,
    "if errorlevel 1 exit /b 1",
    `cl /nologo /W4 /WX /std:c11 /D_CRT_SECURE_NO_WARNINGS /I"${include}" "${source}" "${test}" /Fe:"${executable}"`,
    "exit /b %errorlevel%",
  ].join("\r\n"));
  execFileSync(process.env.ComSpec || "cmd.exe", ["/d", "/c", batch], { cwd: directory, stdio: "inherit" });
} else {
  execFileSync(process.env.CC || "cc", ["-std=c11", "-Wall", "-Wextra", "-Werror", "-I", include, source, test, "-lm", "-o", executable], { cwd: directory, stdio: "inherit" });
}
execFileSync(executable, [], { stdio: "inherit" });

const basicSource = join(include, "kb_basic.c");
const basicTest = resolve("src-tauri/tests/kidbright_basic_test.c");
const basicExecutable = join(directory, process.platform === "win32" ? "basic-test.exe" : "basic-test");
if (process.platform === "win32") {
  const vswhere = join(process.env["ProgramFiles(x86)"] || "C:/Program Files (x86)", "Microsoft Visual Studio/Installer/vswhere.exe");
  const install = execFileSync(vswhere, ["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], { encoding: "utf8" }).trim();
  const batch = join(directory, "compile-basic.cmd");
  writeFileSync(batch, ["@echo off", `call "${join(install, "Common7/Tools/VsDevCmd.bat")}" -no_logo -arch=x64`, "if errorlevel 1 exit /b 1",
    `cl /nologo /W4 /WX /std:c11 /D_CRT_SECURE_NO_WARNINGS /I"${include}" "${basicSource}" "${basicTest}" /Fe:"${basicExecutable}"`, "exit /b %errorlevel%"].join("\r\n"));
  execFileSync(process.env.ComSpec || "cmd.exe", ["/d", "/c", batch], { cwd: directory, stdio: "inherit" });
} else {
  execFileSync(process.env.CC || "cc", ["-std=c11", "-Wall", "-Wextra", "-Werror", "-I", include, basicSource, basicTest, "-lm", "-o", basicExecutable], { cwd: directory, stdio: "inherit" });
}
execFileSync(basicExecutable, [], { stdio: "inherit" });
