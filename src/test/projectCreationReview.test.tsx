import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ProjectCreationReview from "../ProjectCreationReview";

const { invokeMock, handlers, unlistenMock } = vi.hoisted(() => ({
    invokeMock: vi.fn(),
    handlers: new Map<string, (event: { payload: string }) => void>(),
    unlistenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/event", () => ({
    listen: vi.fn((name: string, handler: (event: { payload: string }) => void) => {
        handlers.set(name, handler);
        return Promise.resolve(unlistenMock);
    }),
}));

const project = {
    projectPath: "D:\\projects\\s1_sound",
    projectName: "s1_sound",
    previousProjectPath: "D:\\projects\\previous",
    files: ["main/main.c", "main/CMakeLists.txt", "CMakeLists.txt"],
    messageId: "message-1",
    revision: "project-r1",
};

function review(props: { isBusy?: boolean; hasUnsavedChanges?: boolean; projectDir?: string } = {}) {
    return <ProjectCreationReview isBusy={props.isBusy ?? false} projectDir={props.projectDir ?? project.projectPath} hasUnsavedChanges={props.hasUnsavedChanges} />;
}

function emit(name: string, payload: unknown) {
    act(() => { handlers.get(name)?.({ payload: JSON.stringify(payload) }); });
}

describe("Project creation review", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        handlers.clear();
        invokeMock.mockImplementation((command: string) => Promise.resolve(command === "get_pending_project_reviews" ? [project] : "ok"));
    });

    it("restores a pending project on mount with visible actions and its file list", async () => {
        const { unmount } = render(review());
        expect(await screen.findByText("Review new project: s1_sound")).toBeInTheDocument();
        expect(invokeMock).toHaveBeenCalledWith("get_pending_project_reviews");
        expect(screen.getByText("3 files created")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Keep" })).toBeVisible();
        expect(screen.getByRole("button", { name: "Undo" })).toBeVisible();
        fireEvent.click(screen.getByText("3 files created"));
        expect(screen.getByText("main/main.c")).toBeVisible();
        unmount();
        await waitFor(() => expect(unlistenMock).toHaveBeenCalledTimes(2));
    });

    it("hides internal and legacy root metadata from old payloads while keeping coding files and user documents", async () => {
        const visibleFiles = [
            "main/main.c", "components/kb_reference/kb_core.c", "CMakeLists.txt", "sdkconfig",
            "config/user.json", "README.md", "nested/kidbright-project.json", "examples/KIDBRIGHT_COMPATIBILITY.md",
        ];
        const files = [
            ...visibleFiles, ".vibekidbright/project.json", "cache\\.VIBEKIDBRIGHT\\reference.md",
            "kidbright-project.json", "KIDBRIGHT_COMPATIBILITY.MD", ".\\KidBright-Project.JSON",
        ];
        invokeMock.mockResolvedValue([{ ...project, files }]);
        const { container } = render(review());
        fireEvent.click(await screen.findByText("8 files created"));
        for (const file of visibleFiles) expect(screen.getByText(file)).toBeVisible();
        for (const file of files.filter(file => !visibleFiles.includes(file))) {
            expect(screen.queryByText(file)).not.toBeInTheDocument();
        }
        expect(container.textContent?.toLowerCase()).not.toContain(".vibekidbright");

        emit("ai-project-review-pending", { ...project, files: [...files, "main/extra.h"] });
        expect(screen.getByText("9 files created")).toBeInTheDocument();
        expect(screen.getByText("main/extra.h")).toBeVisible();
        expect(container.textContent?.toLowerCase()).not.toContain(".vibekidbright");
        expect(screen.getByRole("button", { name: "Keep" })).toBeEnabled();
        expect(screen.getByRole("button", { name: "Undo" })).toBeEnabled();
    });

    it.each(["Keep", "Undo"])("%s resolves only the selected project", async action => {
        render(review());
        fireEvent.click(await screen.findByRole("button", { name: action }));
        await waitFor(() => expect(invokeMock).toHaveBeenCalledWith(action === "Keep" ? "keep_ai_project" : "undo_ai_project", { projectPath: project.projectPath, expectedRevision: "project-r1" }));
        await waitFor(() => expect(screen.queryByText("Review new project: s1_sound")).not.toBeInTheDocument());
    });

    it("waits for AI completion and protects unsaved editor changes", async () => {
        const { rerender } = render(review({ isBusy: true }));
        expect(await screen.findByRole("button", { name: "Keep" })).toBeDisabled();
        expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();
        expect(screen.getByText("Available when AI finishes.")).toBeInTheDocument();
        rerender(review({ hasUnsavedChanges: true }));
        expect(screen.getByRole("button", { name: "Keep" })).toBeDisabled();
        expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();
        expect(screen.getByText(/Save or discard your unsaved changes/)).toBeInTheDocument();
        fireEvent.click(screen.getByRole("button", { name: "Undo" }));
        expect(invokeMock).not.toHaveBeenCalledWith("undo_ai_project", expect.anything());
        rerender(review());
        expect(screen.getByRole("button", { name: "Keep" })).toBeEnabled();
    });

    it("keeps the review and displays a backend error so the user can retry", async () => {
        invokeMock.mockImplementation((command: string) => command === "undo_ai_project"
            ? Promise.reject("A generated file has changed. Save it before undoing.")
            : Promise.resolve([project]));
        render(review());
        fireEvent.click(await screen.findByRole("button", { name: "Undo" }));
        expect(await screen.findByRole("alert")).toHaveTextContent("A generated file has changed");
        expect(screen.getByText("Review new project: s1_sound")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: "Undo" })).toBeEnabled();
    });

    it("disables both actions until the selected command finishes", async () => {
        let finish!: () => void;
        invokeMock.mockImplementation((command: string) => command === "keep_ai_project"
            ? new Promise<void>(resolve => { finish = resolve; })
            : Promise.resolve([project]));
        render(review());
        fireEvent.click(await screen.findByRole("button", { name: "Keep" }));
        expect(screen.getByRole("button", { name: "Keep" })).toBeDisabled();
        expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();
        await act(async () => { finish(); });
        expect(screen.queryByText("Review new project: s1_sound")).not.toBeInTheDocument();
    });

    it("receives pending and resolved events without changing another project's review", async () => {
        invokeMock.mockResolvedValue([]);
        const { rerender } = render(review());
        await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_pending_project_reviews"));
        emit("ai-project-review-pending", project);
        expect(await screen.findByText("Review new project: s1_sound")).toBeInTheDocument();
        emit("ai-project-review-pending", { ...project, files: ["main/main.c"] });
        expect(screen.getByText("1 file created")).toBeInTheDocument();
        emit("ai-project-review-resolved", { projectPath: "D:\\another", action: "keep" });
        expect(screen.getByRole("button", { name: "Keep" })).toBeInTheDocument();
        rerender(review({ projectDir: "D:/PROJECTS/s1_sound/" }));
        expect(screen.getByRole("button", { name: "Keep" })).toBeInTheDocument();
        rerender(review({ projectDir: "D:/projects/another" }));
        expect(screen.queryByRole("button", { name: "Keep" })).not.toBeInTheDocument();
        rerender(review());
        emit("ai-project-review-resolved", { projectPath: project.projectPath, action: "undo" });
        expect(screen.queryByRole("button", { name: "Keep" })).not.toBeInTheDocument();
    });

    it("retains an event received while the initial review read is outstanding", async () => {
        let finishRead!: (reviews: unknown[]) => void;
        invokeMock.mockImplementation(() => new Promise(resolve => { finishRead = resolve; }));
        render(review());
        await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_pending_project_reviews"));
        emit("ai-project-review-pending", project);
        await act(async () => { finishRead([]); });
        expect(screen.getByText("Review new project: s1_sound")).toBeInTheDocument();
    });
});
