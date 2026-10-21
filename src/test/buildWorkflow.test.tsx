import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { MIUAI_DEPLOYMENT_UNAVAILABLE } from "../buildFlash";

const { invokeMock, handlers } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  handlers: new Map<string, (event: { payload: string }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((name: string, handler: (event: { payload: string }) => void) => {
    handlers.set(name, handler);
    return Promise.resolve(() => handlers.delete(name));
  }),
}));
vi.mock("../AiChat", () => ({ default: ({ hasUnsavedChanges }: { hasUnsavedChanges: boolean }) => (
  <span data-testid="unsaved-program-state">{String(hasUnsavedChanges)}</span>
) }));
vi.mock("../ToolchainSetup", () => ({ default: () => null }));
vi.mock("../WikiView", () => ({ default: () => null }));
vi.mock("../CodeEditor", () => ({
  default: ({ value, onChange, readOnly }: { value: string; onChange: (value: string) => void; readOnly: boolean }) => (
    <textarea aria-label="Program editor" value={value} readOnly={readOnly} onChange={event => onChange(event.target.value)} />
  ),
}));

const projectDir = "D:\\Learning projects\\KidBright & demos";
const mainFile = `${projectDir}\\main\\main.c`;
const supportFile = `${projectDir}\\CMakeLists.txt`;

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>(finish => { resolve = finish; });
  return { promise, resolve };
}

async function readyApp() {
  render(<App />);
  await screen.findByRole("button", { name: "main.c" });
  fireEvent.click(screen.getByRole("button", { name: "เขียนโปรแกรม" }));
  fireEvent.keyDown(window, { key: "F5" });
  return screen.findByRole("button", { name: /Build & Flash/ });
}

async function editMain(content = "latest program") {
  fireEvent.click(screen.getByRole("button", { name: "main.c" }));
  const editor = await screen.findByRole("textbox", { name: "Program editor" });
  fireEvent.change(editor, { target: { value: content } });
  return editor;
}

describe("Build and flash from the learner editor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    handlers.clear();
    localStorage.clear();
    localStorage.setItem("vibe-project-dir", projectDir);
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "list_project_files") return [{ name: "main", path: `${projectDir}\\main`, is_dir: true, children: [
        { name: "main.c", path: mainFile, is_dir: false },
      ] }];
      if (command === "validate_idf_project") return true;
      if (command === "check_toolchain") return { status: "ready", version: "test" };
      if (command === "get_idf_custom_paths") return { idf_path: "C:\\Toolchains & SDKs\\ESP IDF", tools_path: "C:\\SDK tools" };
      if (command === "list_serial_ports") return ["COM7", "COM8"];
      if (command === "list_adb_devices") return [];
      if (command === "read_project_file") return "on disk program";
      return "Ready";
    });
  });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it("saves current main.c before running IDF with the selected port as separate arguments", async () => {
    const build = await readyApp();
    await editMain();
    fireEvent.change(screen.getByTitle("Auto-refreshing Serial Ports"), { target: { value: "COM8" } });
    fireEvent.click(build);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_idf_command", {
      command: "-p", args: ["COM8", "build", "flash"], projectDir,
    }));
    expect(invokeMock).toHaveBeenCalledWith("write_project_file", { path: mainFile, content: "latest program" });
    const commands = invokeMock.mock.calls.map(([command]) => command);
    expect(commands.indexOf("write_project_file")).toBeLessThan(commands.indexOf("run_idf_command"));
    expect(commands).not.toContain("run_shell_command");
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("false");
  });

  it("aborts before starting IDF on a save failure and allows a later retry", async () => {
    const original = invokeMock.getMockImplementation()!;
    let failSave = true;
    invokeMock.mockImplementation((command: string, args: unknown) => {
      if (command === "write_project_file" && failSave) return Promise.reject(new Error("Disk is full"));
      return original(command, args);
    });
    const build = await readyApp();
    await editMain();
    fireEvent.click(build);
    await screen.findByText(/Build stopped:.*Disk is full/);
    expect(invokeMock).not.toHaveBeenCalledWith("run_idf_command", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("run_shell_command", expect.anything());
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("true");
    failSave = false;
    fireEvent.click(screen.getByRole("button", { name: /Build & Flash/ }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_idf_command", expect.anything()));
  });

  it("locks repeated build shortcuts while saving and waits for IDF to finish", async () => {
    const save = deferred();
    const run = deferred();
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => {
      if (command === "write_project_file") return save.promise;
      if (command === "run_idf_command") return run.promise;
      return original(command, args);
    });
    await readyApp();
    await editMain();
    act(() => {
      fireEvent.keyDown(window, { key: "B", ctrlKey: true, shiftKey: true });
      fireEvent.keyDown(window, { key: "b", ctrlKey: true, shiftKey: true });
    });
    expect(invokeMock.mock.calls.filter(([command]) => command === "write_project_file")).toHaveLength(1);
    expect(invokeMock).not.toHaveBeenCalledWith("run_idf_command", expect.anything());
    expect(screen.getByRole("button", { name: "Building..." })).toBeDisabled();
    await act(async () => { save.resolve(); });
    expect(invokeMock.mock.calls.filter(([command]) => command === "run_idf_command")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Building..." })).toBeDisabled();
    await act(async () => { run.resolve(); });
    expect(screen.getByRole("button", { name: /Build & Flash/ })).toBeEnabled();
  });

  it("saves the dirty learner source while an opened supporting file stays read only", async () => {
    const build = await readyApp();
    await editMain();
    await act(async () => { handlers.get("ai-diff-pending")?.({ payload: JSON.stringify({ fullPath: supportFile }) }); });
    const supportEditor = screen.getByRole("textbox", { name: "Program editor" });
    expect(supportEditor).toHaveAttribute("readonly");
    fireEvent.change(supportEditor, { target: { value: "must not write this" } });
    fireEvent.click(build);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_idf_command", expect.anything()));
    expect(invokeMock.mock.calls.filter(([command]) => command === "write_project_file")).toEqual([
      ["write_project_file", { path: mainFile, content: "latest program" }],
    ]);
  });

  it("leaves edits made during an outstanding save dirty instead of falsely marking them saved", async () => {
    const save = deferred();
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "write_project_file" ? save.promise : original(command, args));
    const build = await readyApp();
    const editor = await editMain("program at build click");
    fireEvent.click(build);
    fireEvent.change(editor, { target: { value: "newer edit during save" } });
    await act(async () => { save.resolve(); });
    expect(invokeMock).toHaveBeenCalledWith("write_project_file", { path: mainFile, content: "program at build click" });
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("true");
    expect(editor).toHaveValue("newer edit during save");
  });

  it("uses the IDF default port without adding a blank port argument", async () => {
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "list_serial_ports" ? Promise.resolve([]) : original(command, args));
    fireEvent.click(await readyApp());
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_idf_command", { command: "build", args: ["flash"], projectDir }));
  });

  it("cancels the owned build workspace after the visible project changes",async()=>{
    const run=deferred();const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command:string,args:unknown)=>command==="run_idf_command"?run.promise:original(command,args));
    fireEvent.click(await readyApp());
    await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("run_idf_command",expect.anything()));
    await act(async()=>{handlers.get("force-project-dir")?.({payload:"D:\\projects\\another"});});
    fireEvent.click(screen.getByRole("button",{name:"Stop build"}));
    await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("cancel_sdk_command",{projectDir}));
    await act(async()=>run.resolve());
  });
  it("honors Stop during a pending save without starting the SDK afterward",async()=>{
    const save=deferred();const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command:string,args:unknown)=>command==="write_project_file"?save.promise:original(command,args));
    const build=await readyApp();await editMain();fireEvent.click(build);
    fireEvent.click(await screen.findByRole("button",{name:"Stop build"}));
    await act(async()=>save.resolve());
    await screen.findByText(/Build stopped:.*cancelled before starting the SDK/);
    expect(invokeMock).not.toHaveBeenCalledWith("run_idf_command",expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("cancel_sdk_command",expect.anything());
  });

  it("preserves a backend Keep requirement without retrying through a shell", async () => {
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "run_idf_command"
      ? Promise.reject(new Error("Choose Keep before building this project")) : original(command, args));
    fireEvent.click(await readyApp());
    await screen.findByText(/Build stopped:.*Choose Keep/);
    expect(invokeMock.mock.calls.filter(([command]) => command === "run_idf_command")).toHaveLength(1);
    expect(invokeMock).not.toHaveBeenCalledWith("run_shell_command", expect.anything());
  });

  it("keeps μAI deployment unavailable in both the button and shortcut", async () => {
    localStorage.setItem("vibe-selected-board", "miuaiplus");
    const build = await readyApp();
    expect(build).toBeDisabled();
    expect(build).toHaveAttribute("title", MIUAI_DEPLOYMENT_UNAVAILABLE);
    expect(MIUAI_DEPLOYMENT_UNAVAILABLE).toContain("after KidBright support is complete");
    fireEvent.keyDown(window, { key: "b", ctrlKey: true, shiftKey: true });
    expect(invokeMock).not.toHaveBeenCalledWith("run_idf_command", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("run_shell_command", expect.anything());
  });

  it("routes typed idf.py commands through backend argv without an export.bat shell string", async () => {
    await readyApp();
    const input = screen.getByPlaceholderText("Type command (e.g. idf.py) and press Enter...");
    fireEvent.change(input, { target: { value: "idf.py -p COM8 build" } });
    fireEvent.submit(input.closest("form")!);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_idf_command", { command: "-p", args: ["COM8", "build"], projectDir }));
    expect(invokeMock).not.toHaveBeenCalledWith("run_shell_command", expect.anything());
  });

  it("preserves quoted project paths and Windows backslashes in terminal IDF arguments", async () => {
    await readyApp();
    const input = screen.getByPlaceholderText("Type command (e.g. idf.py) and press Enter...");
    fireEvent.change(input, { target: { value: `idf.py   -C "${projectDir}" -p COM8 build` } });
    fireEvent.submit(input.closest("form")!);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_idf_command", {
      command: "-C", args: [projectDir, "-p", "COM8", "build"], projectDir,
    }));
    expect(invokeMock).not.toHaveBeenCalledWith("run_shell_command", expect.anything());
  });

  it("passes other IDF tools as argv and leaves an unfinished quoted command unexecuted", async () => {
    await readyApp();
    const input = screen.getByPlaceholderText("Type command (e.g. idf.py) and press Enter...");
    fireEvent.change(input, { target: { value: `cmake -S "${projectDir}" -B build` } });
    fireEvent.submit(input.closest("form")!);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("run_shell_command", {
      cmd: "cmake", args: ["-S", projectDir, "-B", "build"], cwd: projectDir,
    }));
    fireEvent.change(input, { target: { value: 'idf.py -C "unfinished path' } });
    fireEvent.submit(input.closest("form")!);
    expect(invokeMock).not.toHaveBeenCalledWith("run_idf_command", expect.anything());
    expect(input).toHaveValue('idf.py -C "unfinished path');
    expect(screen.getByText(/Close the quote before running/)).toBeInTheDocument();
  });
});
