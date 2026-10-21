import { describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import PrivateContextConsent from "../PrivateContextConsent";
describe("Private context export consent", () => {
    it("requires an explicit user selection and explains the remote/local scope", () => {
        const change = vi.fn();
        render(<PrivateContextConsent allowed={false} onChange={change} />);
        const box = screen.getByRole("checkbox");
        expect(box).not.toBeChecked();
        expect(change).not.toHaveBeenCalled();
        expect(screen.getByText(/configured fallback providers/)).toBeInTheDocument();
        fireEvent.click(box);
        expect(change).toHaveBeenCalledWith(true);
    });
});
