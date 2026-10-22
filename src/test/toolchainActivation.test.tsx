import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import ToolchainSetup from "../ToolchainSetup";

const { invokeMock, state } = vi.hoisted(() => ({ invokeMock: vi.fn(), state: { handler: null as null | ((event: { payload: { stage: string; percent: number; message: string } }) => void) } }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn((_name, handler) => { state.handler = handler; return Promise.resolve(() => {}); }) }));

afterEach(() => { cleanup(); vi.clearAllMocks(); state.handler = null; });

describe("toolchain activation in the same app session", () => {
    it("verifies readiness instead of trusting a premature completion event", async () => {
        invokeMock.mockResolvedValue({ status: "not_installed", version: null, path: null });
        const onReady = vi.fn();
        render(<ToolchainSetup onReady={onReady} mini />);
        await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("check_toolchain"));
        await act(async () => state.handler?.({ payload: { stage: "done", percent: 100, message: "ready" } }));
        expect(invokeMock.mock.calls.filter(([name]) => name === "check_toolchain")).toHaveLength(2);
        await new Promise(resolve => setTimeout(resolve, 850));
        expect(onReady).not.toHaveBeenCalled();
    });
    it("activates after the download result even when the completion event is missed", async () => {
        let installed = false;
        invokeMock.mockImplementation(async (name) => {
            if (name === "download_toolchain") { installed = true; return "installed"; }
            return { status: installed ? "ready" : "not_installed", version: installed ? "1.0.1" : null, path: installed ? "fixture-toolchain" : null };
        });
        const onReady = vi.fn();
        render(<ToolchainSetup onReady={onReady} mini />);
        fireEvent.click(await screen.findByText(/Auto-installing in/));
        fireEvent.click(await screen.findByRole("button", { name: /Install Now/ }));
        await waitFor(() => expect(onReady).toHaveBeenCalled(), { timeout: 1500 });
        expect(invokeMock).toHaveBeenCalledWith("download_toolchain", { url: null });
        expect(installed).toBe(true);
    });
});
