/**
 * toast.test.tsx — tests for the global toast notification system.
 * Run: npm test
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { toast, ToastHost } from "../Toast";

describe("Toast", () => {
    beforeEach(() => {
        vi.useFakeTimers();
    });

    it("renders nothing until a toast is pushed", () => {
        render(<ToastHost />);
        expect(screen.queryByTestId("toast-host")).not.toBeInTheDocument();
    });

    it("shows an error toast pushed via the global function", () => {
        render(<ToastHost />);
        act(() => {
            toast("Something broke", "error");
        });
        expect(screen.getByRole("status")).toHaveTextContent("Something broke");
    });

    it("stacks multiple toasts and keeps them capped at 5", () => {
        render(<ToastHost />);
        act(() => {
            for (let i = 0; i < 8; i++) toast(`msg ${i}`, "info");
        });
        const host = screen.getByTestId("toast-host");
        expect(host.children.length).toBeLessThanOrEqual(5);
        // The newest message must be present
        expect(screen.getByText("msg 7")).toBeInTheDocument();
    });

    it("dismisses a toast when the close button is clicked", () => {
        render(<ToastHost />);
        act(() => {
            toast("dismissible", "info");
        });
        const close = screen.getByTitle("Dismiss");
        fireEvent.click(close);
        expect(screen.queryByText("dismissible")).not.toBeInTheDocument();
    });

    it("auto-dismisses after the timeout", () => {
        render(<ToastHost />);
        act(() => {
            toast("fleeting", "success");
        });
        expect(screen.getByText("fleeting")).toBeInTheDocument();

        act(() => {
            vi.advanceTimersByTime(7000);
        });
        expect(screen.queryByText("fleeting")).not.toBeInTheDocument();
    });

    it("falls back to console when no host is mounted", () => {
        const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
        act(() => {
            toast("no host here", "error");
        });
        expect(errSpy).toHaveBeenCalledWith("[toast] no host here");
        errSpy.mockRestore();
    });
});
