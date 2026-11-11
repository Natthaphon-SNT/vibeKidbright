import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ToolchainSetup from "../ToolchainSetup";
const { invokeMock, listenMock } = vi.hoisted(() => ({ invokeMock: vi.fn(), listenMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...args: unknown[]) => listenMock(...args) }));

describe("First launch without a toolchain", () => {
  beforeEach(() => { listenMock.mockResolvedValue(() => {}); });
  afterEach(() => { cleanup(); vi.useRealTimers(); vi.restoreAllMocks(); invokeMock.mockReset(); listenMock.mockReset(); });
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
  it("shows the full installation error in the expanded mini widget and allows retry", async () => {
    const message = "Cannot save SDK file frameworks/esp-idf/components/very-long-path/tool.txt: The system cannot find the path specified. (os error 3)";
    invokeMock.mockImplementation((command: string) => command === "check_toolchain"
      ? Promise.resolve({ status: "not_installed", version: null }) : Promise.reject(message));
    await act(async () => { render(<ToolchainSetup mini onReady={vi.fn()} />); });
    fireEvent.click(screen.getByText("Toolchain not installed"));
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: /Install Toolchain/ })); });
    expect(screen.getByRole("alert")).toHaveTextContent(message);
    expect(screen.getByText(`[ERROR] ${message}`)).toBeVisible();
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: /Retry Installation/ })); });
    expect(invokeMock.mock.calls.filter(([command]) => command === "download_toolchain")).toHaveLength(2);
  });
  it("shows integrity rejection without marking ready or invoking another installation route", async () => {
    const onReady = vi.fn();
    const message = "Toolchain integrity: kidbright-tools is unverified. Installation is disabled. The existing toolchain was retained.";
    invokeMock.mockImplementation((command: string) => command === "check_toolchain"
      ? Promise.resolve({ status: "not_installed", version: null }) : Promise.reject(message));
    await act(async () => { render(<ToolchainSetup mini onReady={onReady} />); });
    fireEvent.click(screen.getByText("Toolchain not installed"));
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: /Install Toolchain/ })); });
    expect(screen.getByRole("alert")).toHaveTextContent(message);
    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual(["check_toolchain", "download_toolchain"]);
    expect(onReady).not.toHaveBeenCalled();
  });
  it("renders verification progress without announcing installation success", async () => {
    const onReady = vi.fn();
    invokeMock.mockImplementation((command: string) => command === "check_toolchain"
      ? Promise.resolve({ status: "not_installed", version: null }) : new Promise(() => {}));
    await act(async () => { render(<ToolchainSetup onReady={onReady} />); });
    fireEvent.click(screen.getByRole("button", { name: /Install Toolchain/ }));
    const callback = listenMock.mock.calls[0][1];
    act(() => callback({ payload: { stage: "verifying", percent: 99, message: "Rechecking archive SHA-256 before activation..." } }));
    expect(screen.getByText("Checking toolchain SHA-256...")).toBeVisible();
    expect(screen.getByText("Rechecking archive SHA-256 before activation...")).toBeVisible();
    expect(onReady).not.toHaveBeenCalled();
  });

  it("guides both Mac architectures to native SDK setup and rechecks instead of downloading the Windows ZIP", async () => {
    vi.useFakeTimers();
    const onReady = vi.fn();
    const error = "ESP-IDF Python environment is missing or broken. Install the native tools.";
    invokeMock.mockResolvedValueOnce({ status: "not_installed", platform: "macos", error })
      .mockResolvedValueOnce({ status: "ready", platform: "macos", version: "5.5.1" })
      .mockResolvedValueOnce({ status: "ready", platform: "macos", version: "5.5.1" });
    await act(async () => { render(<ToolchainSetup onReady={onReady} />); });
    expect(screen.getByRole("region", { name: "macOS ESP-IDF setup" })).toBeVisible();
    expect(screen.getByText(/Apple Silicon และ Intel/)).toBeVisible();
    expect(screen.queryByRole("button", { name: /Install Toolchain/ })).toBeNull();
    expect(onReady).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Check Native SDK" }));
    await act(async () => { await vi.advanceTimersByTimeAsync(600); });
    expect(onReady).toHaveBeenCalledTimes(1);
    expect(invokeMock.mock.calls.map(([command]) => command)).toEqual(["check_toolchain", "select_native_macos_sdk", "check_toolchain"]);
  });
});
