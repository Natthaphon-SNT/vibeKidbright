import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";

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
vi.mock("../AiChat", () => ({ default: ({ onApplyToFile, onInjectCode, hasUnsavedChanges }: {
  onApplyToFile: (path: string, content: string) => Promise<void>; onInjectCode: (code: string) => void; hasUnsavedChanges: boolean;
}) => <div>
  <span data-testid="unsaved-program-state">{String(hasUnsavedChanges)}</span>
  <button onClick={() => { void onApplyToFile("CMakeLists.txt", "AI saved content"); }}>Apply supporting proposal</button>
  <button onClick={() => onInjectCode("AI injected program")}>Inject program</button>
</div> }));
vi.mock("../ToolchainSetup", () => ({ default: () => null }));
vi.mock("../WikiView", () => ({ default: () => null }));
vi.mock("../CodeEditor", () => ({
  default: ({ value, onChange, readOnly }: { value: string; onChange: (value: string) => void; readOnly: boolean }) => (
    <textarea aria-label="Program editor" value={value} readOnly={readOnly} onChange={event => onChange(event.target.value)} />
  ),
}));

const root = "D:\\projects\\s1_sound";
const mainFile = `${root}\\main\\main.c`;
const supportFile = `${root}\\CMakeLists.txt`;
const file = (path: string) => ({ name: path.split("\\").pop()!, path, is_dir: false });
const tree = [
  { name: "components", path: `${root}\\components`, is_dir: true, children: [file(`${root}\\components\\kb_core.c`)] },
  { name: "main", path: `${root}\\main`, is_dir: true, children: [file(mainFile), file(`${root}\\main\\kb_board.h`), file(`${root}\\main\\CMakeLists.txt`)] },
  file(supportFile), file(`${root}\\sdkconfig`), file(`${root}\\KIDBRIGHT_COMPATIBILITY.md`),
];

describe("Learner project sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    handlers.clear();
    localStorage.clear();
    localStorage.setItem("vibe-project-dir", root);
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "list_project_files") return tree;
      if (command === "validate_idf_project") return true;
      if (command === "get_idf_custom_paths") return { idf_path: "", tools_path: "" };
      if (command === "list_serial_ports") return [];
      if (command === "read_project_file") return "original";
      return "Ready";
    });
  });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it("keeps a dirty tab when Close is cancelled and discards it only after confirmation", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    const editor = await screen.findByRole("textbox", { name: "Program editor" });
    fireEvent.change(editor, { target: { value: "unsaved code" } });
    fireEvent.click(screen.getByRole("button", { name: "Close main.c" }));
    expect(editor).toHaveValue("unsaved code");
    expect(confirm).toHaveBeenCalledTimes(1);
    confirm.mockReturnValue(true);
    fireEvent.click(screen.getByRole("button", { name: "Close main.c" }));
    expect(screen.queryByRole("textbox", { name: "Program editor" })).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.anything());
  });

  it("keeps dirty code on external changes and cancelled Reload", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    const editor = await screen.findByRole("textbox", { name: "Program editor" });
    fireEvent.change(editor, { target: { value: "unsaved code" } });
    await act(async () => { handlers.get("file-modified")?.({ payload: JSON.stringify({ path: mainFile }) }); });
    expect(editor).toHaveValue("unsaved code");
    fireEvent.click(screen.getByTitle("Reload from disk"));
    expect(editor).toHaveValue("unsaved code");
    expect(invokeMock.mock.calls.filter(([command]) => command === "read_project_file")).toHaveLength(1);
  });

  it("does not overwrite an edit made while a disk reload is outstanding", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    const editor = await screen.findByRole("textbox", { name: "Program editor" });
    let finish!: (content: string) => void;
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "read_project_file"
      ? new Promise<string>(resolve => { finish = resolve; }) : original(command, args));
    fireEvent.click(screen.getByTitle("Reload from disk"));
    fireEvent.change(editor, { target: { value: "typed during read" } });
    await act(async () => { finish("new disk contents"); });
    expect(editor).toHaveValue("typed during read");
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("true");
  });

  it("restores dirty code after an AI workspace switch and never saves it to the new project", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    fireEvent.change(await screen.findByRole("textbox", { name: "Program editor" }), { target: { value: "previous project draft" } });
    await act(async () => { handlers.get("force-project-dir")?.({ payload: "D:\\projects\\new_project" }); });
    expect(screen.queryByRole("textbox", { name: "Program editor" })).not.toBeInTheDocument();
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.anything());
    await act(async () => { handlers.get("force-project-dir")?.({ payload: root }); });
    expect(await screen.findByRole("textbox", { name: "Program editor" })).toHaveValue("previous project draft");
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("true");
  });

  it("leaves injected AI code dirty when the write fails", async () => {
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "write_project_file"
      ? Promise.reject(new Error("Disk full")) : original(command, args));
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    await screen.findByRole("textbox", { name: "Program editor" });
    fireEvent.click(screen.getByRole("button", { name: "Inject program" }));
    await screen.findByText(/Could not save AI code.*Disk full/);
    expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue("AI injected program");
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("true");
  });

  it("preserves dirty learner code across the learning view and keeps Build out of that view", async () => {
    const original = invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command: string, args: unknown) => command === "check_toolchain"
      ? Promise.resolve({ status: "ready", version: "test" }) : original(command, args));
    render(<App />);
    fireEvent.keyDown(window, { key: "F5" });
    const buildButton = await screen.findByRole("button", { name: /Build & Flash/ });
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    fireEvent.change(await screen.findByRole("textbox", { name: "Program editor" }), { target: { value: "my unsaved lesson" } });
    fireEvent.click(screen.getByRole("button", { name: "ห้องเรียน AI" }));
    expect(screen.getByRole("heading", { level: 1, name: "ให้ไอเดียของคุณ เรียนรู้ได้ด้วย AI" })).toBeVisible();
    expect(buildButton).toBeDisabled();
    fireEvent.keyDown(window, { key: "B", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(invokeMock).not.toHaveBeenCalledWith("run_idf_command", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.anything());
    fireEvent.click(screen.getByRole("button", { name: "เขียนโปรแกรม" }));
    expect(await screen.findByRole("textbox", { name: "Program editor" })).toHaveValue("my unsaved lesson");
  });

  it("shows only main program files and protects the main folder from rename and delete", async () => {
    render(<App />);
    const sidebar = within(screen.getByRole("region", { name: "Project files" }));
    const main = await sidebar.findByRole("button", { name: "main" });
    expect(sidebar.getByRole("button", { name: "main.c" })).toBeVisible();
    for (const name of ["components", "kb_core.c", "CMakeLists.txt", "sdkconfig", "kb_board.h", "KIDBRIGHT_COMPATIBILITY.md"]) {
      expect(sidebar.queryByRole("button", { name })).not.toBeInTheDocument();
    }
    fireEvent.contextMenu(main);
    expect(screen.queryByRole("button", { name: /Rename/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Delete/ })).not.toBeInTheDocument();
    fireEvent.click(sidebar.getByRole("button", { name: "main.c" }));
    expect(await screen.findByRole("textbox", { name: "Program editor" })).not.toHaveAttribute("readonly");
    expect(invokeMock).not.toHaveBeenCalledWith("delete_file", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("delete_directory", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("delete_program_item", expect.anything());
  });

  it("creates new program files inside main using the header action", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "main.c" });
    fireEvent.click(screen.getByTitle("New File in main"));
    const input = within(screen.getByRole("region", { name: "Project files" })).getByRole("textbox");
    fireEvent.change(input, { target: { value: "lesson.c" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("create_program_file", {
      projectDir: root, path: `${root}\\main\\lesson.c`,
    }));
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.objectContaining({ path: `${root}\\lesson.c` }));
  });

  it("creates learner folders through the validated program-directory command", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "main.c" });
    fireEvent.click(screen.getByTitle("New Folder in main"));
    const input = within(screen.getByRole("region", { name: "Project files" })).getByRole("textbox");
    fireEvent.change(input, { target: { value: "lessons" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("create_program_directory", {
      projectDir: root, path: `${root}\\main\\lessons`,
    }));
    expect(invokeMock).not.toHaveBeenCalledWith("create_directory", expect.anything());
  });

  it("does not fall back to overwriting a program when New File finds an existing name", async () => {
    invokeMock.mockImplementation(async (command: string) => {
      if (command === "list_project_files") return tree;
      if (command === "validate_idf_project") return true;
      if (command === "get_idf_custom_paths") return {};
      if (command === "list_serial_ports") return [];
      if (command === "create_program_file") throw new Error("Already exists");
      return "Ready";
    });
    render(<App />);
    await screen.findByRole("button", { name: "main.c" });
    fireEvent.click(screen.getByTitle("New File in main"));
    const input = within(screen.getByRole("region", { name: "Project files" })).getByRole("textbox");
    fireEvent.change(input, { target: { value: "main.c" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await screen.findByText(/Operation failed.*Already exists/);
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("safe_write_project_file", expect.anything());
  });

  it("updates open descendant tabs after a managed program folder rename", async () => {
    const original = invokeMock.getMockImplementation()!;
    const oldPath = `${root}\\main\\controls`;
    const newPath = `${root}\\main\\lesson`;
    let folderPath = oldPath;
    invokeMock.mockImplementation((command: string, args: unknown) => {
      if (command === "list_project_files") return Promise.resolve([{ name: "main", path: `${root}\\main`, is_dir: true, children: [
        { name: folderPath.split("\\").pop()!, path: folderPath, is_dir: true, children: [file(`${folderPath}\\controller.c`)] },
      ] }]);
      if (command === "rename_program_item") folderPath = newPath;
      return original(command, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "controls" }));
    fireEvent.click(screen.getByRole("button", { name: "controller.c" }));
    await screen.findByRole("textbox", { name: "Program editor" });
    fireEvent.contextMenu(screen.getByRole("button", { name: "controls" }));
    fireEvent.click(screen.getByRole("button", { name: /Rename/ }));
    const input = within(screen.getByRole("region", { name: "Project files" })).getByRole("textbox");
    fireEvent.change(input, { target: { value: "lesson" } });
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("rename_program_item", { projectDir: root, oldPath, newPath }));
    await screen.findByRole("button", { name: "lesson" });
    fireEvent.change(screen.getByRole("textbox", { name: "Program editor" }), { target: { value: "changed after rename" } });
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("write_project_file", { path: `${newPath}\\controller.c`, content: "changed after rename" }));
    expect(invokeMock).not.toHaveBeenCalledWith("rename_item", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.objectContaining({ path: `${oldPath}\\controller.c` }));
  });

  it("closes descendant tabs when a managed program directory is deleted", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const original = invokeMock.getMockImplementation()!;
    const folderPath = `${root}\\main\\controls`;
    let deleted = false;
    invokeMock.mockImplementation((command: string, args: unknown) => {
      if (command === "list_project_files") return Promise.resolve([{ name: "main", path: `${root}\\main`, is_dir: true, children: deleted ? [] : [
        { name: "controls", path: folderPath, is_dir: true, children: [file(`${folderPath}\\controller.c`)] },
      ] }]);
      if (command === "delete_program_item") deleted = true;
      return original(command, args);
    });
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "controls" }));
    fireEvent.click(screen.getByRole("button", { name: "controller.c" }));
    await screen.findByRole("textbox", { name: "Program editor" });
    fireEvent.contextMenu(screen.getByRole("button", { name: "controls" }));
    fireEvent.click(screen.getByRole("button", { name: /Delete/ }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("delete_program_item", { projectDir: root, path: folderPath }));
    await waitFor(() => expect(screen.queryByRole("textbox", { name: "Program editor" })).not.toBeInTheDocument());
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.anything());
    expect(invokeMock).not.toHaveBeenCalledWith("delete_directory", expect.anything());
  });

  it("keeps an already-open supporting file clean after an explicitly accepted AI apply", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    render(<App />);
    await screen.findByRole("button", { name: "main.c" });
    await act(async () => {
      handlers.get("ai-diff-pending")?.({ payload: JSON.stringify({ fullPath: supportFile }) });
    });
    fireEvent.click(screen.getByRole("button", { name: "Apply supporting proposal" }));
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveValue("AI saved content"));
    expect(screen.getByRole("textbox", { name: "Program editor" })).toHaveAttribute("readonly");
    expect(screen.getByTestId("unsaved-program-state")).toHaveTextContent("false");
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    expect(invokeMock).not.toHaveBeenCalledWith("write_project_file", expect.anything());
  });

  it("opens supporting AI changes read only and saves only the edited main source", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "main.c" }));
    const editor = await screen.findByRole("textbox", { name: "Program editor" });
    fireEvent.change(editor, { target: { value: "new program" } });
    await act(async () => {
      handlers.get("ai-diff-pending")?.({ payload: JSON.stringify({ fullPath: supportFile }) });
    });
    expect(screen.getByText("Supporting file · Read only")).toBeVisible();
    const supportingEditor = screen.getByRole("textbox", { name: "Program editor" });
    expect(supportingEditor).toHaveAttribute("readonly");
    fireEvent.change(supportingEditor, { target: { value: "accidental edit" } });
    expect(supportingEditor).toHaveValue("original");
    fireEvent.keyDown(window, { key: "s", ctrlKey: true });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("write_project_file", { path: mainFile, content: "new program" }));
    const writes = invokeMock.mock.calls.filter(([command]) => command === "write_project_file");
    expect(writes).toEqual([["write_project_file", { path: mainFile, content: "new program" }]]);
  });
});
