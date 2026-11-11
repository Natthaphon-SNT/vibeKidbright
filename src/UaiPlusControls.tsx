import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { normPath } from "./utils";

type DeviceState = { connected: boolean; running: boolean; exitCode?: number; executionUnknown?: boolean;
  runtimeInfo?: { maixpy: string | null; runtime: string | null; api_checked: boolean; missing_apis: string[] } | null };

export default function UaiPlusControls({ projectDir, saveFiles, onLog, disabled = false, workspaceRevision = 0 }: {
  projectDir: string; saveFiles: () => Promise<void>; onLog: (text: string) => void; disabled?: boolean; workspaceRevision?: number;
}) {
  const [device, setDevice] = useState<DeviceState>({ connected: false, running: false });
  const [busy, setBusy] = useState(false);
  const active = useRef(true);
  const inFlight = useRef(false);
  const project = useRef(projectDir);
  project.current = projectDir;
  const workspace = useRef({ path: normPath(projectDir), revision: workspaceRevision, generation: 0 });
  if (workspace.current.path !== normPath(projectDir) || workspace.current.revision !== workspaceRevision) {
    workspace.current = { path: normPath(projectDir), revision: workspaceRevision, generation: workspace.current.generation + 1 };
  }
  const log = useRef(onLog);
  log.current = onLog;

  useEffect(() => {
    active.current = true;
    let disposed = false;
    let receivedEvent = false;
    const unlisten = listen<DeviceState>("uai-plus-state", event => {
      if (disposed) return;
      receivedEvent = true;
      setDevice(event.payload);
      if (event.payload.exitCode !== undefined) log.current(`Plus Python exited with code ${event.payload.exitCode}.`);
      if (event.payload.executionUnknown) log.current("Plus disconnected before an exit status was received; physical execution status is unknown.");
    }).catch(error => {
      if (!disposed) log.current(`Plus event listener: ${error}`);
      return () => {};
    });
    void invoke<DeviceState>("get_uai_plus_status").then(state => {
      if (!disposed && !receivedEvent) setDevice(state);
    }).catch(error => log.current(`Plus status: ${error}`));
    return () => {
      disposed = true;
      active.current = false;
      void unlisten.then(stop => stop()).catch(() => {});
      // Keep the device session when navigating between code and learning.
      // An explicit board switch or Disconnect owns device disconnection.
    };
  }, []);

  const perform = async (action: () => Promise<void>) => {
    if (inFlight.current) return;
    inFlight.current = true;
    setBusy(true);
    try { await action(); }
    catch (error) { log.current(`Plus: ${error}`); }
    finally { inFlight.current = false; if (active.current) setBusy(false); }
  };

  const connect = () => perform(async () => {
    if (device.connected) {
      await invoke("disconnect_uai_plus");
      if (active.current) setDevice({ connected: false, running: false });
      return;
    }
    const fingerprint = await invoke<string>("probe_uai_plus");
    if (!active.current) return;
    const trusted = localStorage.getItem("vibe-plus-usb-certificate");
    if (trusted !== fingerprint && !window.confirm(
      `Connect to your KidBright uAI Plus at 10.155.55.1 over USB RNDIS?\n\nThe board uses a local TLS certificate. Trust this device certificate only if this is your connected board.\nSHA-256: ${fingerprint}`,
    )) return;
    const result = await invoke<string>("connect_uai_plus", { fingerprint });
    if (!active.current) { await invoke("disconnect_uai_plus"); return; }
    localStorage.setItem("vibe-plus-usb-certificate", fingerprint);
    setDevice(await invoke<DeviceState>("get_uai_plus_status"));
    log.current(result);
  });

  const run = () => perform(async () => {
    const root = projectDir;
    const generation = workspace.current.generation;
    await saveFiles();
    if (!active.current || workspace.current.generation !== generation || normPath(project.current) !== normPath(root)) throw new Error("Project changed while saving; run again in the selected project.");
    log.current(await invoke<string>("run_uai_plus_project", { projectDir: root }));
    const state = await invoke<DeviceState>("get_uai_plus_status");
    if (active.current) setDevice(state);
  });

  return <div className="flex flex-wrap items-center gap-2" aria-label="KidBright uAI Plus Python controls">
    <span className="text-[10px]" title="USB-OTG → RNDIS → 10.155.55.1:5050">USB RNDIS</span>
    <button className="text-[10px] px-2 py-1 rounded" onClick={() => void connect()} disabled={busy}
      style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)' }}>
      {busy ? "Please wait..." : device.connected ? "Disconnect Plus" : "Connect Plus"}
    </button>
    <button className="text-[10px] px-2 py-1 rounded font-bold" onClick={() => void run()}
      disabled={disabled || busy || !device.connected || device.running || projectDir === "."}
      style={{ backgroundColor: 'var(--success)', color: '#fff' }}>Run Python</button>
    <button className="text-[10px] px-2 py-1 rounded" disabled={busy || !device.connected || !device.running}
      onClick={() => void perform(async () => { log.current(await invoke<string>("stop_uai_plus")); })}
      style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)' }}>Stop Python</button>
    {device.connected && device.runtimeInfo && <span className="text-[9px]" aria-label="Plus runtime versions"
      title={device.runtimeInfo.api_checked ? (device.runtimeInfo.missing_apis.length ? `Missing APIs: ${device.runtimeInfo.missing_apis.join(", ")}` : "Required MaixPy APIs checked during Run") : "Native APIs will be checked during Run"}>
      MaixPy {device.runtimeInfo.maixpy ?? "unknown"} · Runtime {device.runtimeInfo.runtime ?? "unknown"}
    </span>}
  </div>;
}
