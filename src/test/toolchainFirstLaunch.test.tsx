import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ToolchainSetup from "../ToolchainSetup";
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));

describe("First launch without a toolchain", () => {
  afterEach(() => { cleanup(); vi.useRealTimers(); vi.restoreAllMocks(); });
  it("keeps the IDE usable without auto-downloading until the user chooses installation", async () => {
    vi.useFakeTimers();
    invokeMock.mockImplementation((command: string) => command === "check_toolchain"
      ? Promise.resolve({ status: "not_installed", version: null }) : new Promise(() => {}));
    await act(async () => { render(<ToolchainSetup onReady={vi.fn()} />); });
    await act(async () => { await vi.advanceTimersByTimeAsync(7000); });
    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual(["check_toolchain"]);
    fireEvent.click(screen.getByRole("button", { name: /Install Toolchain/ }));
    expect(invokeMock).toHaveBeenCalledWith("download_toolchain", { url: null });
  });
});
