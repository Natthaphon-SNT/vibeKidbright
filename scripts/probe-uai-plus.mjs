// Production UI with explicit native IPC fixtures; never contacts a real board.
// Optional dependency: npm install --prefix src-tauri/target/learning-ui-tools --no-save playwright-core
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";
import assert from "node:assert/strict";

const require = createRequire(import.meta.url);
const { chromium } = require(resolve(process.env.VIBE_PLAYWRIGHT_MODULE || "src-tauri/target/learning-ui-tools/node_modules/playwright-core"));
const port = 1437;
const origin = `http://127.0.0.1:${port}`;
const preview = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "preview", "--host", "127.0.0.1", "--port", String(port), "--strictPort"], { windowsHide: true, stdio: "ignore" });
let browser;
try {
  for (let attempt = 0; attempt < 50; attempt++) {
    if (await fetch(origin).then(response => response.ok).catch(() => false)) break;
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({ executablePath: process.env.VIBE_CHROME_PATH || "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: true, args: ["--disable-background-networking"] });
  const context = await browser.newContext({ viewport: { width: 1536, height: 1000 } });
  const externalRequests = [];
  await context.route("**/*", route => {
    if (new URL(route.request().url()).origin === origin) return route.continue();
    externalRequests.push(route.request().url()); return route.abort();
  });
  const source = readFileSync("samples/kidbright-uai-plus/display_text_circle.py", "utf8");
  await context.addInitScript(({ source }) => {
    localStorage.setItem("vibe-ai-panel", "true");
    localStorage.setItem("vibe-active-view", "editor");
    localStorage.setItem("vibe-selected-board", "miuaiplus");
    localStorage.setItem("vibe-project-dir", "D:/public-fixture/plus");
    window.__plusFixture = { calls: [], connected: false, running: false, source };
    const callbacks = new Map(); const listeners = new Map(); let next = 1;
    const emit = (name, payload) => { const callback = listeners.get(name); if (callback) callbacks.get(callback)?.({ event: name, id: callback, payload }); };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
      transformCallback: callback => { const id = next++; callbacks.set(id, callback); return id; },
      unregisterCallback: id => callbacks.delete(id),
      invoke: async (command, args) => {
        const state = window.__plusFixture;
        state.calls.push({ command, args });
        if (command === "plugin:event|listen") { listeners.set(args.event, args.handler); return next++; }
        if (command === "plugin:event|unlisten") { listeners.delete(args.event); return null; }
        if (command === "get_project_kind") return "kidbright_uai_plus";
        if (command === "get_provider") return "local";
        if (command === "get_base_url") return "http://127.0.0.1:11434/v1";
        if (["get_model", "get_openrouter_model", "get_google_model"].includes(command)) return "local-fixture";
        if (["get_api_key", "get_openrouter_api_key", "get_google_api_key"].includes(command)) return "";
        if (command === "get_private_context_consent") return false;
        if (command === "get_knowledge_base_files") return [];
        if (command === "list_project_files") return [{ name: "main", path: `${args.path}/main`, is_dir: true, children: [{ name: "main.py", path: `${args.path}/main/main.py`, is_dir: false }] }];
        if (command === "check_toolchain") return { status: "ready", version: "fixture" };
        if (command === "get_idf_custom_paths") return { idf_path: "", tools_path: "" };
        if (command === "read_project_file") return state.source;
        if (command === "write_project_file") { state.source = args.content; return null; }
        if (command === "check_pending_diff") return null;
        if (command === "get_uai_plus_status") return { connected: state.connected, running: state.running,
          runtimeInfo: state.runtimeInfo ?? null };
        if (command === "probe_uai_plus") return "a".repeat(64);
        if (command === "connect_uai_plus") { state.connected = true; return "Connected to local IPC fixture (no physical board)"; }
        if (command === "run_uai_plus_project") { state.running = true;
          state.runtimeInfo = { maixpy: "4.12.5", runtime: "1.24.0", api_checked: true, missing_apis: [] };
          return "Fixture upload acknowledged; simulated execution requested"; }
        if (command === "stop_uai_plus") { state.running = false; emit("uai-plus-state", { connected: state.connected, running: false, exitCode: 130,
          runtimeInfo: { maixpy: "4.12.5", runtime: "1.24.0", api_checked: true, missing_apis: [] } }); return "Fixture stopped"; }
        if (command === "disconnect_uai_plus") { state.connected = false; state.running = false; return null; }
        if (command === "pick_directory") return "D:/public-fixture";
        if (command === "create_uai_plus_project") return `D:/public-fixture/${args.name}`;
        if (["stop_serial_monitor", "check_esp_idf"].includes(command)) return "OK";
        if (["list_serial_ports", "get_pending_project_reviews"].includes(command)) return [];
        throw new Error(`Unsupported native fixture command: ${command}`);
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  }, { source });
  const page = await context.newPage(); const pageErrors = [];
  page.on("pageerror", error => pageErrors.push(String(error)));
  page.on("dialog", dialog => dialog.accept());
  await page.goto(origin, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "main.py", exact: true }).click();
  await page.locator(".monaco-editor textarea").waitFor();
  await page.locator(".view-lines").filter({ hasText: "from maix" }).first().waitFor();
  const initialCalls = await page.evaluate(() => window.__plusFixture.calls.map(call => call.command));
  assert(!initialCalls.includes("probe_uai_plus"));
  assert(!initialCalls.includes("list_adb_devices"));
  await page.getByRole("button", { name: "Connect Plus", exact: true }).click();
  await page.getByRole("button", { name: "Disconnect Plus", exact: true }).waitFor();
  assert.equal(await page.getByLabel("Plus runtime versions").count(), 0);
  await page.locator(".monaco-editor .view-lines").click({ position: { x: 150, y: 100 } });
  await page.keyboard.press("Control+End");
  await page.keyboard.insertText("\n# local IPC fixture edit\n");
  try {
    await page.waitForFunction(() => document.querySelector(".monaco-editor .view-lines")?.textContent?.replace(/\u00a0/g, " ").includes("local IPC fixture edit"), undefined, { timeout: 8000 });
  } catch (error) {
    console.log(await page.evaluate(() => ({ active: document.activeElement?.outerHTML, editor: document.querySelector(".monaco-editor")?.textContent, calls: window.__plusFixture.calls })));
    throw error;
  }
  await page.getByRole("button", { name: "Run Python", exact: true }).click();
  await page.waitForFunction(() => window.__plusFixture.running);
  assert((await page.getByLabel("Plus runtime versions").innerText()).includes("MaixPy 4.12.5 · Runtime 1.24.0"));
  assert(await page.getByRole("button", { name: "Run Python", exact: true }).isDisabled());
  await page.getByRole("button", { name: "Stop Python", exact: true }).click();
  await page.waitForFunction(() => !window.__plusFixture.running);
  await page.getByRole("button", { name: "Run Python", exact: true }).waitFor({ state: "visible" });
  await page.getByRole("button", { name: "NEW", exact: true }).click();
  await page.getByPlaceholder("my_plus_project").fill("plus_fresh");
  await page.getByRole("button", { name: "Browse", exact: true }).click();
  await page.getByRole("button", { name: "Create Project", exact: true }).click();
  await page.waitForFunction(() => localStorage.getItem("vibe-project-dir") === "D:/public-fixture/plus_fresh");
  await page.getByRole("button", { name: "main.py", exact: true }).click();
  await page.locator(".view-lines").filter({ hasText: "from maix" }).first().waitFor();
  const calls = await page.evaluate(() => window.__plusFixture.calls);
  const saveIndex = calls.findIndex(call => call.command === "write_project_file");
  const runIndex = calls.findIndex(call => call.command === "run_uai_plus_project");
  if (saveIndex < 0 || saveIndex >= runIndex) console.log(JSON.stringify({ calls, editor: await page.locator(".monaco-editor").innerText() }));
  assert(saveIndex >= 0 && saveIndex < runIndex, "Dirty Python must be saved before Run");
  assert(calls[saveIndex].args.content.includes("# local IPC fixture edit"));
  assert(calls.some(call => call.command === "create_uai_plus_project"));
  assert(!calls.some(call => ["create_idf_project", "run_idf_command", "start_adb_monitor", "list_adb_devices"].includes(call.command)));
  assert.deepEqual(pageErrors, []);
  assert.deepEqual(externalRequests, []);
  const viewportChecks = [];
  for (const width of [1536, 1280]) {
    await page.setViewportSize({ width, height: 1000 });
    const bounds = await page.locator('[aria-label="KidBright uAI Plus Python controls"]').evaluate(element => {
      const toolbar = element.parentElement.parentElement.getBoundingClientRect();
      return [...element.querySelectorAll("button")].map(button => {
        const box = button.getBoundingClientRect();
        return { label: button.textContent, insideToolbar: box.left >= toolbar.left && box.right <= toolbar.right && box.bottom <= toolbar.bottom };
      });
    });
    assert(bounds.every(button => button.insideToolbar), `Plus controls must fit with Vibe Coder open at width ${width}`);
    viewportChecks.push({ width, controls_visible: true, vibe_coder_open: true });
  }
  await page.setViewportSize({ width: 1536, height: 1000 });
  const output = "docs/research/uai-plus-runtime-2026-10-26";
  mkdirSync(output, { recursive: true });
  await page.evaluate(() => {
    const banner = document.createElement("div"); banner.textContent = "LOCAL IPC FIXTURE — NO PHYSICAL BOARD";
    banner.style.cssText = "position:fixed;bottom:0;left:0;right:0;background:#17364e;color:white;text-align:center;font:12px sans-serif;z-index:99999;padding:4px";
    document.body.append(banner);
  });
  await page.screenshot({ path: "docs/previews/uai-plus-runtime-workspace.png" });
  writeFileSync(`${output}/ui-probe.json`, JSON.stringify({ kind: "production UI with native IPC fixtures", physical_board: false, live_model: false, page_errors: pageErrors, external_requests: externalRequests, viewport_checks: viewportChecks, checks: ["Python restore", "offline Monaco editing", "certificate trust before connect", "Connect without runtime metadata", "detected versions after Run", "save before run", "Run and Stop states", "fresh Python project", "no ADB or ESP-IDF route"], commands: [...new Set(calls.map(call => call.command))] }, null, 2) + "\n");
  console.log("Plus production UI fixture passed: restore/edit/save/connect/run/stop/new Python project; zero page errors or external requests.");
} finally {
  await browser?.close(); preview.kill();
}
