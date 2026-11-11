/**
 * codeEditorDiff.test.tsx — tests for the AI diff review flow in CodeEditor.
 * Tauri invoke/listen and Monaco are mocked; the focus is on the accept/reject UX.
 * Run: npm test
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { listen } from "@tauri-apps/api/event";
import type { EditorProps, OnMount } from "@monaco-editor/react";
import CodeEditor from "../CodeEditor";

// ── Mocks ─────────────────────────────────────────────────────────────────────

const invokeMock = vi.fn();
const editorPropsMock = vi.fn<(props: EditorProps) => void>();

vi.mock("@tauri-apps/api/core", () => ({
    invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
    listen: vi.fn().mockResolvedValue(() => {}),
}));

vi.mock("@monaco-editor/react", () => ({
    default: (props: EditorProps) => {
        editorPropsMock(props);
        return <div data-testid="mock-editor" data-value={props.value} data-readonly={String(props.options?.readOnly ?? false)} />;
    },
    DiffEditor: () => <div data-testid="mock-diff-editor" />,
}));

describe("CodeEditor diff flow", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.mocked(listen).mockReset().mockResolvedValue(() => {});
    });

    afterEach(() => vi.restoreAllMocks());

    it("ignores a diff event for the same basename in another project", async () => {
        invokeMock.mockResolvedValue(null);
        render(<CodeEditor value="# current" onChange={() => {}} filePath="/current/main/main.py" />);
        await waitFor(() => expect(listen).toHaveBeenCalledWith("ai-diff-pending", expect.any(Function)));
        const before=invokeMock.mock.calls.length;
        await act(async () => {
            const handler=vi.mocked(listen).mock.calls.find(([name])=>name==="ai-diff-pending")![1];
            handler({event:"ai-diff-pending",id:1,payload:JSON.stringify({fullPath:"/other/main/main.py"})});
        });
        expect(invokeMock.mock.calls.length).toBe(before);
        expect(screen.getByTestId("mock-editor")).toHaveAttribute("data-value","# current");
    });

    it("gives same-named files in different projects separate Monaco model identities", async () => {
        invokeMock.mockResolvedValue(null);
        const { rerender } = render(<CodeEditor value="# MP-021" onChange={() => {}} filePath={"D:\\projects\\mai_test_021\\main\\main.py"} />);
        await screen.findByTestId("mock-editor");
        expect(editorPropsMock.mock.calls[editorPropsMock.mock.calls.length - 1]?.[0].path).toBe("D:/projects/mai_test_021/main/main.py");
        rerender(<CodeEditor value="# MP-022" onChange={() => {}} filePath={"D:\\projects\\mai_test_022\\main\\main.py"} />);
        await waitFor(() => expect(editorPropsMock.mock.calls[editorPropsMock.mock.calls.length - 1]?.[0].path).toBe("D:/projects/mai_test_022/main/main.py"));
        expect(screen.getByTestId("mock-editor")).toHaveAttribute("data-value", "# MP-022");
    });

    it("preserves the latest review when an earlier diff request fails", async () => {
        const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
        let rejectInitial!: (reason?: unknown) => void;
        invokeMock.mockImplementationOnce(() => new Promise((_resolve, reject) => {
            rejectInitial = reject;
        })).mockResolvedValue({ revision: "r2", before: "old", proposed: "latest patch", status: "pending" });
        const path = "/proj/main/main.c";
        render(<CodeEditor value="old" onChange={() => {}} filePath={path} />);
        await waitFor(() => expect(listen).toHaveBeenCalledWith("ai-diff-pending", expect.any(Function)));
        await act(async () => {
            vi.mocked(listen).mock.calls[0][1]({ event: "ai-diff-pending", id: 1, payload: { fullPath: path } });
        });
        await screen.findByText("latest patch");
        await act(async () => rejectInitial(new Error("stale request failed")));
        expect(screen.getByText("latest patch")).toBeInTheDocument();
        expect(screen.getByText("Review AI Changes")).toBeInTheDocument();
        expect(errSpy).not.toHaveBeenCalled();
    });

    it("handles a failed diff event subscription without discarding the initial review", async () => {
        const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
        vi.mocked(listen).mockRejectedValueOnce(new Error("event subscription unavailable"));
        invokeMock.mockResolvedValue({ revision: "r1", before: "old", proposed: "pending patch", status: "pending" });
        render(<CodeEditor value="old" onChange={() => {}} filePath="/proj/main/main.c" />);
        await screen.findByText("pending patch");
        await waitFor(() => expect(errSpy).toHaveBeenCalledWith("Failed to listen for pending AI changes:", expect.any(Error)));
        expect(screen.getByText("Review AI Changes")).toBeInTheDocument();
    });

    it("shows the Review AI Changes banner when a pending diff exists", async () => {
        invokeMock.mockImplementation((cmd: string) => {
            if (cmd === "check_pending_diff") return Promise.resolve({revision: "r1", before: "int x = 0;", proposed: "int fixed = 1;", status: "pending"});
            return Promise.resolve(null);
        });

        render(<CodeEditor value="int x = 0;" onChange={() => {}} filePath="/proj/main/main.c" />);

        await waitFor(() => {
            expect(screen.getByText("Review AI Changes")).toBeInTheDocument();
        });

        expect(await screen.findByText("Old code (removed)")).toBeInTheDocument();
        expect(screen.getByText("New code (added)")).toBeInTheDocument();
        expect(screen.getByText("int x = 0;")).toBeInTheDocument();
        expect(screen.getByText("int fixed = 1;")).toBeInTheDocument();
        expect(screen.queryByTestId("mock-editor")).not.toBeInTheDocument();
    });

    it("does not show the banner when there is no pending diff", async () => {
        invokeMock.mockResolvedValue(null);

        render(<CodeEditor value="int x = 0;" onChange={() => {}} filePath="/proj/main/main.c" />);

        await waitFor(() => {
            expect(invokeMock).toHaveBeenCalledWith("check_pending_diff", {
                path: "/proj/main/main.c",
            });
        });

        expect(screen.queryByText("Review AI Changes")).not.toBeInTheDocument();
        expect(await screen.findByTestId("mock-editor")).toBeInTheDocument();
    });

    it("blocks Keep/Undo while an editor draft is unsaved or recovery needs attention", async () => {
        invokeMock.mockResolvedValue({revision:"r1",before:"old",proposed:"new",status:"pending"});
        const props={value:"draft",onChange:()=>{},filePath:"/proj/main/main.c"};
        const {rerender}=render(<CodeEditor {...props} hasUnsavedChanges />);
        await screen.findByText("Review AI Changes");
        expect(screen.getByText(/Keep/).closest("button")).toBeDisabled();
        expect(screen.getByText(/Undo/).closest("button")).toBeDisabled();
        invokeMock.mockResolvedValue({revision:"r1",before:"old",proposed:"new",status:"conflict"});
        rerender(<CodeEditor {...props} filePath="/proj/main/other.c" />);
        await screen.findByText("Review AI Changes");
        expect(screen.getByText(/Keep/).closest("button")).toBeDisabled();
        expect(invokeMock.mock.calls.some(([cmd])=>cmd==="accept_diff")).toBe(false);
    });

    it("refreshes the current review after a stale authorization is rejected", async () => {
        const errSpy=vi.spyOn(console,"error").mockImplementation(()=>{});
        let revision="r1";
        invokeMock.mockImplementation((cmd:string)=>{
            if(cmd==="check_pending_diff") return Promise.resolve({revision,before:"old",proposed:revision,status:"pending"});
            if(cmd==="accept_diff") {revision="r2";return Promise.reject("Stale review authorization");}
            return Promise.resolve(null);
        });
        render(<CodeEditor value="old" onChange={()=>{}} filePath="/proj/main/main.c" />);
        await screen.findByText("r1"); fireEvent.click(screen.getByText(/Keep/));
        await screen.findByText("r2");
        expect(invokeMock).toHaveBeenCalledWith("accept_diff",{path:"/proj/main/main.c",expectedRevision:"r1"});
        errSpy.mockRestore();
    });

    it("blocks edits and the mounted save action for read-only files, then restores editing when allowed", async () => {
        invokeMock.mockResolvedValue(null);
        const onChange = vi.fn();
        const onSave = vi.fn();
        const props = { value: "protected content", onChange, onSave, filePath: "/proj/CMakeLists.txt" };
        const { rerender } = render(<CodeEditor {...props} readOnly />);
        expect(await screen.findByTestId("mock-editor")).toHaveAttribute("data-readonly", "true");

        const addAction = vi.fn();
        const mountedEditor = { addAction, focus: vi.fn(), getModel: () => null };
        const monaco = { KeyMod: { CtrlCmd: 1 }, KeyCode: { KeyS: 2 } };
        const editorProps = () => editorPropsMock.mock.lastCall![0];
        editorProps().onMount?.(
            mountedEditor as unknown as Parameters<OnMount>[0],
            monaco as unknown as Parameters<OnMount>[1],
        );
        const saveAction = addAction.mock.calls[0][0];
        const editEvent = {} as Parameters<NonNullable<EditorProps["onChange"]>>[1];
        editorProps().onChange?.("attempted edit", editEvent);
        saveAction.run();
        expect(onChange).not.toHaveBeenCalled();
        expect(onSave).not.toHaveBeenCalled();

        rerender(<CodeEditor {...props} />);
        expect(screen.getByTestId("mock-editor")).toHaveAttribute("data-readonly", "false");
        editorProps().onChange?.("allowed edit", editEvent);
        saveAction.run();
        expect(onChange).toHaveBeenCalledExactlyOnceWith("allowed edit");
        expect(onSave).toHaveBeenCalledTimes(1);

        rerender(<CodeEditor {...props} readOnly />);
        editorProps().onChange?.("blocked again", editEvent);
        saveAction.run();
        expect(onChange).toHaveBeenCalledTimes(1);
        expect(onSave).toHaveBeenCalledTimes(1);
        expect(addAction).toHaveBeenCalledTimes(1);
    });

    it("Keep button calls accept_diff and dismisses the banner for a read-only file", async () => {
        invokeMock.mockImplementation((cmd: string) => {
            if (cmd === "check_pending_diff") return Promise.resolve({revision: "r1", before: "old content", proposed: "new content", status: "pending"});
            if (cmd === "accept_diff") return Promise.resolve("ok");
            return Promise.resolve(null);
        });

        render(<CodeEditor value="old content" onChange={() => {}} filePath="/proj/main/main.c" readOnly />);
        await screen.findByText("Review AI Changes");

        fireEvent.click(screen.getByText(/Keep/));

        await waitFor(() => {
            expect(invokeMock).toHaveBeenCalledWith("accept_diff", { path: "/proj/main/main.c", expectedRevision: "r1" });
        });

        await waitFor(() => {
            expect(screen.queryByText("Review AI Changes")).not.toBeInTheDocument();
        });
    });

    it("Undo button calls reject_diff and dismisses the banner for a read-only file", async () => {
        invokeMock.mockImplementation((cmd: string) => {
            if (cmd === "check_pending_diff") return Promise.resolve({revision: "r1", before: "old content", proposed: "new content", status: "pending"});
            if (cmd === "reject_diff") return Promise.resolve("ok");
            return Promise.resolve(null);
        });

        render(<CodeEditor value="old content" onChange={() => {}} filePath="/proj/main/main.c" readOnly />);
        await screen.findByText("Review AI Changes");

        fireEvent.click(screen.getByText(/Undo/));

        await waitFor(() => {
            expect(invokeMock).toHaveBeenCalledWith("reject_diff", { path: "/proj/main/main.c", expectedRevision: "r1" });
        });

        await waitFor(() => {
            expect(screen.queryByText("Review AI Changes")).not.toBeInTheDocument();
        });
    });

    it("keeps the banner if accept_diff fails (user can retry)", async () => {
        const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
        invokeMock.mockImplementation((cmd: string) => {
            if (cmd === "check_pending_diff") return Promise.resolve({revision: "r1", before: "old content", proposed: "new content", status: "pending"});
            if (cmd === "accept_diff") return Promise.reject(new Error("disk full"));
            return Promise.resolve(null);
        });

        render(<CodeEditor value="old content" onChange={() => {}} filePath="/proj/main/main.c" />);
        await screen.findByText("Review AI Changes");

        fireEvent.click(screen.getByText(/Keep/));

        await waitFor(() => {
            expect(errSpy).toHaveBeenCalled();
        });
        expect(screen.getByText("Review AI Changes")).toBeInTheDocument();
        errSpy.mockRestore();
    });
});
