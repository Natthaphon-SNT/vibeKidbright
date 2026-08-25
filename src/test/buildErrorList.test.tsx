/**
 * buildErrorList.test.tsx — component tests for the friendly build error panel.
 * Run: npm test
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import BuildErrorList from "../BuildErrorList";
import type { ParsedBuildError } from "../errorHints";

const makeError = (overrides: Partial<ParsedBuildError> = {}): ParsedBuildError => ({
    file: "src/main.c",
    line: 42,
    column: 5,
    message: "expected ';' before '}' token",
    title: "Missing semicolon",
    thaiHint: "ลืมใส่เครื่องหมาย ;",
    englishHint: "A semicolon is missing at the end of a statement.",
    ...overrides,
});

describe("BuildErrorList", () => {
    it("renders nothing when there are no errors", () => {
        const { container } = render(
            <BuildErrorList errors={[]} onJumpToError={vi.fn()} onAskAiFix={vi.fn()} />
        );
        expect(container).toBeEmptyDOMElement();
    });

    it("shows the problem count with correct pluralization", () => {
        render(
            <BuildErrorList
                errors={[makeError()]}
                onJumpToError={vi.fn()}
                onAskAiFix={vi.fn()}
            />
        );
        expect(screen.getByText(/1 problem found/)).toBeInTheDocument();
    });

    it("pluralizes problems correctly", () => {
        render(
            <BuildErrorList
                errors={[makeError(), makeError({ title: "Undefined variable", line: 7 })]}
                onJumpToError={vi.fn()}
                onAskAiFix={vi.fn()}
            />
        );
        expect(screen.getByText(/2 problems found/)).toBeInTheDocument();
    });

    it("renders error title, Thai hint and English hint", () => {
        render(
            <BuildErrorList errors={[makeError()]} onJumpToError={vi.fn()} onAskAiFix={vi.fn()} />
        );
        expect(screen.getByText("Missing semicolon")).toBeInTheDocument();
        expect(screen.getByText("🇹🇭 ลืมใส่เครื่องหมาย ;")).toBeInTheDocument();
        expect(screen.getByText("A semicolon is missing at the end of a statement.")).toBeInTheDocument();
    });

    it("shows file:line button and calls onJumpToError when clicked", () => {
        const onJump = vi.fn();
        const err = makeError();
        render(<BuildErrorList errors={[err]} onJumpToError={onJump} onAskAiFix={vi.fn()} />);

        const jumpBtn = screen.getByRole("button", { name: /main\.c:42/ });
        fireEvent.click(jumpBtn);
        expect(onJump).toHaveBeenCalledOnce();
        expect(onJump).toHaveBeenCalledWith(err);
    });

    it("omits the jump button when the error has no file", () => {
        render(
            <BuildErrorList
                errors={[makeError({ file: undefined, line: undefined })]}
                onJumpToError={vi.fn()}
                onAskAiFix={vi.fn()}
            />
        );
        expect(screen.queryByRole("button", { name: /:\d+/ })).not.toBeInTheDocument();
    });

    it("calls onAskAiFix when the fix button is clicked", () => {
        const onAsk = vi.fn();
        render(<BuildErrorList errors={[makeError()]} onJumpToError={vi.fn()} onAskAiFix={onAsk} />);

        fireEvent.click(screen.getByText(/Ask Vibe Coder to Fix/));
        expect(onAsk).toHaveBeenCalledOnce();
    });
});
