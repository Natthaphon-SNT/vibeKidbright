import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

if (process.platform !== "darwin") {
  console.error("A macOS DMG must be built on a Mac. Use the Build macOS Installer GitHub Actions workflow from Windows/Linux.");
  process.exit(1);
}

const root = fileURLToPath(new URL("../", import.meta.url));
process.chdir(root);
try {
  const installed = execFileSync("rustup", ["target", "list", "--installed"], { encoding: "utf8" });
  for (const target of ["aarch64-apple-darwin", "x86_64-apple-darwin"]) {
    if (!installed.split(/\s+/).includes(target)) {
      throw new Error("Install both Rust targets first: rustup target add aarch64-apple-darwin x86_64-apple-darwin");
    }
  }
  execFileSync(process.execPath, ["scripts/prepare-runtime.mjs", "--download"], { stdio: "inherit" });
  execFileSync(process.execPath, ["node_modules/@tauri-apps/cli/tauri.js", "build",
    "--target", "universal-apple-darwin", "--bundles", "dmg", "--", "--locked"], {
    stdio: "inherit",
    env: { ...process.env, NODE_OPTIONS: process.env.NODE_OPTIONS || "--max-old-space-size=4096" },
  });
  console.log("DMG: src-tauri/target/universal-apple-darwin/release/bundle/dmg/");
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
