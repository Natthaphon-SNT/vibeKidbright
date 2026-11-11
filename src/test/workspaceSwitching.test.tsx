import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { normPath } from "../utils";

const { invokeMock, handlers } = vi.hoisted(() => ({ invokeMock: vi.fn(), handlers: new Map<string, (event: { payload: string }) => unknown>() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn((name: string, handler: (event: { payload: string }) => unknown) => {
  handlers.set(name, handler); return Promise.resolve(() => handlers.delete(name));
}) }));
vi.mock("../AiChat", () => ({ default: ({ onApplyToFile }: { onApplyToFile: (path: string, value: string) => Promise<void> }) => <>
  <button onClick={() => { void onApplyToFile("CMakeLists.txt", "accepted proposal"); }}>Apply supporting file</button>
  <button onClick={() => { void onApplyToFile("D:/other/main/main.c", "wrong project"); }}>Apply foreign file</button>
</> }));
vi.mock("../ToolchainSetup", () => ({ default: () => null }));
vi.mock("../WikiView", () => ({ default: () => null }));
vi.mock("../CodeEditor", () => ({ default: ({ value, onChange }: { value: string; onChange: (s: string) => void }) =>
  <textarea aria-label="Program editor" value={value} onChange={event => onChange(event.target.value)} /> }));

const ROOT = "D:/Learning projects/งาน KidBright";
const OTHER = "D:/projects/other";
const COPY = "D:/copies/งาน KidBright";
const tree = (root: string, names = ["main.c", "helper.c"]) => [{ name: "main", path: `${root}/main`, is_dir: true,
  children: names.map(name => ({ name, path: `${root}/main/${name}`, is_dir: false })) }];
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(r => { resolve = r; }); return { promise, resolve }; }
async function event(name: string, payload: unknown) {
  await act(async () => { await handlers.get(name)?.({ payload: typeof payload === "string" ? payload : JSON.stringify(payload) }); });
}
async function openOther() {
  fireEvent.click(screen.getByRole("button", { name: "OPEN" }));
  await waitFor(() => expect(localStorage.getItem("vibe-project-dir")).toBe(OTHER));
}
async function openMain() { fireEvent.click(await screen.findByRole("button", { name: "main.c" })); return screen.findByRole("textbox", { name: "Program editor" }); }

describe("Workspace operations keep their original project boundary", () => {
  beforeEach(() => {
    vi.clearAllMocks(); handlers.clear(); localStorage.clear();
    localStorage.setItem("vibe-project-dir", ROOT); localStorage.setItem("vibe-active-view", "editor");
    invokeMock.mockImplementation(async (command: string, args?: { path?: string }) => {
      if (command === "get_project_kind") return "esp_idf";
      if (command === "list_project_files") return tree(args?.path ?? ROOT);
      if (command === "read_project_file") return `code:${args?.path}`;
      if (command === "pick_directory") return OTHER;
      if (command === "get_idf_custom_paths") return {};
      if (command === "check_esp_idf") return "ESP-IDF Ready";
      if (command === "list_serial_ports" || command === "get_pending_project_reviews") return [];
      if (command === "get_uai_plus_status") return { connected: false, running: false };
      return null;
    });
  });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it("does not let a late Save As replace a newer Open selection", async () => {
    const copy = deferred<{ projectPath: string; filesCopied: number }>(); const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "save_project_as" ? copy.promise : original(command, args));
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />); await openMain();
    fireEvent.click(screen.getByRole("button", { name: "Save Project" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("save_project_as", { sourceDir: ROOT }));
    await openOther();
    await act(async () => { copy.resolve({ projectPath: COPY, filesCopied: 3 }); });
    expect(localStorage.getItem("vibe-project-dir")).toBe(OTHER);
    expect(confirm).not.toHaveBeenCalled();
  });

  it("opens the path returned by New instead of reconstructing it from the picker", async () => {
    const actual="D:/canonical/new_workspace"; const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command:string,args:unknown)=>command==="create_idf_project"?Promise.resolve(actual):original(command,args));
    render(<App />); await openMain();
    fireEvent.click(screen.getByRole("button",{name:"NEW"}));
    fireEvent.click(screen.getByRole("button",{name:"Browse"}));
    await screen.findByText(OTHER);
    fireEvent.click(screen.getByRole("button",{name:"Create Project"}));
    await waitFor(()=>expect(localStorage.getItem("vibe-project-dir")).toBe(actual));
    expect(screen.getByRole("textbox",{name:"Program editor"})).toHaveValue(`code:${actual}/main/main.c`);
  });

  it("does not activate a late New result after the user opens another project", async () => {
    const created=deferred<string>();const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command:string,args:unknown)=>command==="create_idf_project"?created.promise:original(command,args));
    render(<App />);await openMain();
    fireEvent.click(screen.getByRole("button",{name:"NEW"}));fireEvent.click(screen.getByRole("button",{name:"Browse"}));
    await screen.findByText(OTHER);fireEvent.click(screen.getByRole("button",{name:"Create Project"}));
    await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("create_idf_project",{path:OTHER,name:"my_esp_project"}));
    await openOther();await act(async()=>{created.resolve("D:/created/late");});
    expect(localStorage.getItem("vibe-project-dir")).toBe(OTHER);
    expect(screen.getByRole("textbox",{name:"Program editor"})).toHaveValue(`code:${OTHER}/main/main.c`);
  });

  it("keeps the later Open choice when an earlier validation returns late", async () => {
    const validation=deferred<string|null>();const original=invokeMock.getMockImplementation()!;let choices=0;
    invokeMock.mockImplementation((command:string,args:{path?:string})=>{
      if(command==="pick_directory")return Promise.resolve(++choices===1?"D:/slow":OTHER);
      if(command==="get_project_kind"&&args?.path==="D:/slow")return validation.promise;
      return original(command,args);
    });
    render(<App />);await openMain();fireEvent.click(screen.getByRole("button",{name:"OPEN"}));
    await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("get_project_kind",{path:"D:/slow"}));
    await openOther();await act(async()=>{validation.resolve("esp_idf");});
    expect(localStorage.getItem("vibe-project-dir")).toBe(OTHER);
    expect(screen.getByRole("textbox",{name:"Program editor"})).toHaveValue(`code:${OTHER}/main/main.c`);
  });

  it("discards a previous New dialog's late destination picker result", async () => {
    const picker = deferred<string>(); const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "pick_directory" ? picker.promise : original(command, args));
    render(<App />); await openMain();
    fireEvent.click(screen.getByRole("button", { name: "NEW" }));
    fireEvent.click(screen.getByRole("button", { name: "Browse" }));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    fireEvent.click(screen.getByRole("button", { name: "NEW" }));
    await act(async () => { picker.resolve(OTHER); });
    expect(screen.getByRole("button", { name: "Create Project" })).toBeDisabled();
  });

  it("uses the structured Save As path and loads the copied main program", async () => {
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "save_project_as"
      ? Promise.resolve({ projectPath: COPY, filesCopied: 3 }) : original(command, args));
    vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />); await openMain(); fireEvent.click(screen.getByRole("button", { name: "Save Project" }));
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue(`code:${COPY}/main/main.c`));
    expect(localStorage.getItem("vibe-project-dir")).toBe(COPY);
  });

  it("rejects a late file-tree response after switching away and back", async () => {
    const oldTree = deferred<ReturnType<typeof tree>>(); const original = invokeMock.getMockImplementation()!; let reads = 0;
    invokeMock.mockImplementation((command: string, args: { path?: string }) => {
      if (command === "list_project_files" && args?.path === ROOT) return ++reads === 1 ? oldTree.promise : Promise.resolve(tree(ROOT, ["fresh.c"]));
      return original(command, args);
    });
    render(<App />); await waitFor(() => expect(reads).toBe(1));
    await event("force-project-dir", OTHER); await event("force-project-dir", ROOT);
    await screen.findByRole("button", { name: "fresh.c" });
    await act(async () => { oldTree.resolve(tree(ROOT, ["stale.c"])); });
    expect(screen.getByRole("button", { name: "fresh.c" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "stale.c" })).not.toBeInTheDocument();
  });

  it("keeps the last file clicked when an earlier file read finishes later", async () => {
    const oldRead = deferred<string>(); const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: { path?: string }) => command === "read_project_file" && args?.path?.endsWith("main.c")
      ? oldRead.promise : original(command, args));
    render(<App />); fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    fireEvent.click(screen.getByRole("button", { name: "helper.c" }));
    expect(await screen.findByRole("textbox", { name: "Program editor" })).toHaveValue(`code:${ROOT}/main/helper.c`);
    await act(async () => { oldRead.resolve("late main"); });
    expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue(`code:${ROOT}/main/helper.c`);
  });

  it("rejects applying an absolute file from another project before writing", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />); await openMain(); fireEvent.click(screen.getByRole("button", { name: "Apply foreign file" }));
    expect(invokeMock).not.toHaveBeenCalledWith("safe_write_project_file", expect.anything());
    expect(confirm).not.toHaveBeenCalled();
  });

  it("does not reopen an old supporting file when an accepted write finishes after switching", async () => {
    const save = deferred<void>(); const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "safe_write_project_file" ? save.promise : original(command, args));
    vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />); await openMain(); fireEvent.click(screen.getByRole("button", { name: "Apply supporting file" }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("safe_write_project_file", { path: `${ROOT}/CMakeLists.txt`, content: "accepted proposal", projectDir: ROOT }));
    await openOther();
    await act(async () => { save.resolve(); });
    expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue(`code:${OTHER}/main/main.c`);
    expect(screen.queryByRole("button", { name: "Close CMakeLists.txt" })).not.toBeInTheDocument();
  });

  it("does not add an old project's newly created file to the new workspace", async () => {
    const created = deferred<void>(); const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "create_program_file" ? created.promise : original(command, args));
    render(<App />); await openMain(); fireEvent.click(screen.getByTitle("New File in main"));
    const input = screen.getByRole("region", { name: "Project files" }).querySelector("input")!;
    fireEvent.change(input, { target: { value: "old_project.c" } }); fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("create_program_file", { projectDir: ROOT, path: `${ROOT}/main/old_project.c` }));
    await openOther(); await act(async () => { created.resolve(); });
    expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue(`code:${OTHER}/main/main.c`);
    expect(screen.queryByRole("button", { name: "Close old_project.c" })).not.toBeInTheDocument();
  });

  it("ignores a late AI create or Undo switch belonging to a previous workspace", async () => {
    render(<App />); await openMain(); await openOther();
    await event("force-project-dir", { projectPath: COPY, sourceProjectPath: ROOT, action: "create" });
    await event("force-project-dir", { projectPath: ".", sourceProjectPath: ROOT, action: "undo" });
    expect(localStorage.getItem("vibe-project-dir")).toBe(OTHER);
    expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue(`code:${OTHER}/main/main.c`);
  });

  it("does not invalidate a revisited workspace with a stale startup validation", async () => {
    const startup = deferred<string | null>(); const original = invokeMock.getMockImplementation()!; let reads = 0;
    invokeMock.mockImplementation((command: string, args: { path?: string }) => command === "get_project_kind" && args?.path === ROOT && ++reads === 1
      ? startup.promise : original(command, args));
    render(<App />); await screen.findByRole("button", { name: "main.c" });
    await event("force-project-dir", OTHER); await event("force-project-dir", ROOT);
    await act(async () => { startup.resolve(null); });
    expect(localStorage.getItem("vibe-project-dir")).toBe(ROOT);
  });

  it("does not restore a relative folder into the application's working directory", async () => {
    localStorage.setItem("vibe-project-dir", "old_project/main/..");
    render(<App />);
    await waitFor(() => expect(localStorage.getItem("vibe-project-dir")).toBeNull());
    expect(invokeMock).not.toHaveBeenCalledWith("get_project_kind", { path: "old_project/main/.." });
    expect(invokeMock).not.toHaveBeenCalledWith("list_project_files", { path: "old_project/main/.." });
  });

  it("serializes saves and preserves the newer edit while the first save is pending", async () => {
    const first = deferred<void>(); const original = invokeMock.getMockImplementation()!; let writes = 0;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "write_project_file" && ++writes === 1 ? first.promise : original(command, args));
    render(<App />); const editor = await openMain();
    fireEvent.change(editor, { target: { value: "first save" } }); fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    await waitFor(() => expect(writes).toBe(1));
    fireEvent.change(editor, { target: { value: "second save" } }); fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(writes).toBe(1);
    await act(async () => { first.resolve(); }); await waitFor(() => expect(writes).toBe(2));
    expect(editor).toHaveValue("second save");
    expect(invokeMock.mock.calls.filter(([cmd]) => cmd === "write_project_file").map(([, args]) => args)).toEqual([
      { path: `${ROOT}/main/main.c`, content: "first save", projectDir: ROOT },
      { path: `${ROOT}/main/main.c`, content: "second save", projectDir: ROOT },
    ]);
  });

  it("compares Windows aliases without merging distinct POSIX project paths", () => {
    expect(normPath("D:\\PROJECTS\\one\\")).toBe(normPath("d:/projects/one"));
    expect(normPath("\\\\Server\\Share\\Project\\")).toBe(normPath("//server/share/project"));
    expect(normPath("/projects/One")).not.toBe(normPath("/projects/one"));
  });
});
