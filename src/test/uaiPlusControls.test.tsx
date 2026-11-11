import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import UaiPlusControls from "../UaiPlusControls";

const { invokeMock, handlers } = vi.hoisted(() => ({ invokeMock: vi.fn(), handlers: new Map<string, (event: { payload: any }) => void>() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async (name: string, handler: (event: { payload: any }) => void) => {
  handlers.set(name, handler); return () => handlers.delete(name);
} }));
const fingerprint = "a".repeat(64);
let connected = false;
let runtimeLoaded = false;

beforeEach(() => {
  vi.clearAllMocks(); handlers.clear(); localStorage.clear();
  connected = false;
  runtimeLoaded = false;
  invokeMock.mockImplementation(async (command: string) => {
    if (command === "get_uai_plus_status") return { connected, running: false, runtimeInfo: runtimeLoaded ? {
      maixpy: "4.12.5", runtime: "1.24.0", api_checked: false, missing_apis: [],
    } : null };
    if (command === "probe_uai_plus") return fingerprint;
    if (command === "connect_uai_plus") { connected = true; return "Connected Plus"; }
    if (command === "run_uai_plus_project") { runtimeLoaded = true; return "Upload sent"; }
    return "OK";
  });
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

it("does not probe, upload or execute until the user connects and runs", async () => {
  const log = vi.fn(); const save = vi.fn(async () => {});
  vi.spyOn(window, "confirm").mockReturnValue(true);
  render(<UaiPlusControls projectDir="D:/plus" saveFiles={save} onLog={log} />);
  expect(screen.getByRole("button", { name: "Run Python" })).toBeDisabled();
  expect(invokeMock.mock.calls.map(([name]) => name)).not.toContain("probe_uai_plus");
  fireEvent.click(screen.getByRole("button", { name: "Connect Plus" }));
  await screen.findByRole("button", { name: "Disconnect Plus" });
  expect(window.confirm).toHaveBeenCalledWith(expect.stringContaining(fingerprint));
  expect(invokeMock).toHaveBeenCalledWith("connect_uai_plus", { fingerprint });
  expect(localStorage.getItem("vibe-plus-usb-certificate")).toBe(fingerprint);
  expect(screen.queryByLabelText("Plus runtime versions")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Run Python" })).toBeEnabled();
  expect(save).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Run Python" }));
  await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_uai_plus_project", { projectDir: "D:/plus" }));
  expect(await screen.findByLabelText("Plus runtime versions")).toHaveTextContent("MaixPy 4.12.5 · Runtime 1.24.0");
  expect(save).toHaveBeenCalledTimes(1);
  expect(save.mock.invocationCallOrder[0]).toBeLessThan(invokeMock.mock.invocationCallOrder[invokeMock.mock.calls.findIndex(([name]) => name === "run_uai_plus_project")]);
  expect(invokeMock.mock.calls.some(([name]) => ["run_idf_command", "start_adb_monitor", "list_adb_devices"].includes(name))).toBe(false);
});

it("does not send files when certificate trust is declined or a save fails", async () => {
  const log = vi.fn();
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  render(<UaiPlusControls projectDir="D:/plus" saveFiles={async () => { throw new Error("disk full"); }} onLog={log} />);
  fireEvent.click(screen.getByRole("button", { name: "Connect Plus" }));
  await waitFor(() => expect(confirm).toHaveBeenCalled());
  expect(invokeMock.mock.calls.map(([name]) => name)).not.toContain("connect_uai_plus");
  confirm.mockReturnValue(true);
  fireEvent.click(screen.getByRole("button", { name: "Connect Plus" }));
  await screen.findByRole("button", { name: "Disconnect Plus" });
  fireEvent.click(screen.getByRole("button", { name: "Run Python" }));
  await waitFor(() => expect(log).toHaveBeenCalledWith(expect.stringContaining("disk full")));
  expect(invokeMock.mock.calls.map(([name]) => name)).not.toContain("run_uai_plus_project");
});

it("does not treat a Stop request as a confirmed successful exit", async () => {
  const log = vi.fn();
  render(<UaiPlusControls projectDir="D:/plus" saveFiles={async () => {}} onLog={log} />);
  await act(async () => { handlers.get("uai-plus-state")?.({ payload: { connected: true, running: true } }); });
  fireEvent.click(screen.getByRole("button", { name: "Stop Python" }));
  await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("stop_uai_plus"));
  expect(screen.getByRole("button", { name: "Run Python" })).toBeDisabled();
  await act(async () => { handlers.get("uai-plus-state")?.({ payload: { connected: true, running: false, exitCode: 1 } }); });
  expect(log).toHaveBeenCalledWith("Plus Python exited with code 1.");
  expect(screen.getByRole("button", { name: "Run Python" })).toBeEnabled();
});

it("cancels launch when the workspace changes while saving", async () => {
  let finish!: () => void;
  const save = () => new Promise<void>(resolve => { finish = resolve; });
  const log = vi.fn();
  const { rerender } = render(<UaiPlusControls projectDir="D:/old" saveFiles={save} onLog={log} />);
  await act(async () => { handlers.get("uai-plus-state")?.({ payload: { connected: true, running: false } }); });
  fireEvent.click(screen.getByRole("button", { name: "Run Python" }));
  rerender(<UaiPlusControls projectDir="D:/new" saveFiles={save} onLog={log} />);
  await act(async () => { finish(); });
  expect(invokeMock.mock.calls.map(([name]) => name)).not.toContain("run_uai_plus_project");
  expect(log).toHaveBeenCalledWith(expect.stringContaining("Project changed"));
});

it("shows newer detected versions and preserves unknown runtime information", async () => {
  render(<UaiPlusControls projectDir="D:/plus" saveFiles={async () => {}} onLog={vi.fn()} />);
  await act(async () => { handlers.get("uai-plus-state")?.({ payload: { connected: true, running: true,
    runtimeInfo: { maixpy: "4.99.0", runtime: null, api_checked: true, missing_apis: ["display.Display.show"] },
  } }); });
  const versions = screen.getByLabelText("Plus runtime versions");
  expect(versions).toHaveTextContent("MaixPy 4.99.0 · Runtime unknown");
  expect(versions).toHaveAttribute("title", "Missing APIs: display.Display.show");
});

it("cancels an old Run after switching away and back while its save is pending", async () => {
  let finish!: () => void;
  const save = () => new Promise<void>(resolve => { finish = resolve; });
  const log = vi.fn();
  const { rerender } = render(<UaiPlusControls projectDir="D:/old" workspaceRevision={0} saveFiles={save} onLog={log} />);
  await act(async () => { handlers.get("uai-plus-state")?.({ payload: { connected: true, running: false } }); });
  fireEvent.click(screen.getByRole("button", { name: "Run Python" }));
  rerender(<UaiPlusControls projectDir="D:/other" workspaceRevision={1} saveFiles={save} onLog={log} />);
  rerender(<UaiPlusControls projectDir="D:/old" workspaceRevision={2} saveFiles={save} onLog={log} />);
  await act(async () => { finish(); });
  expect(invokeMock).not.toHaveBeenCalledWith("run_uai_plus_project", expect.anything());
  expect(log).toHaveBeenCalledWith(expect.stringContaining("Project changed"));
});
