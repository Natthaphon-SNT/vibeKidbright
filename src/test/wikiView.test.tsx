import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import WikiView, { renderMarkdown } from "../WikiView";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));

describe("Wiki on an installed desktop app", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    localStorage.setItem("vibe_wiki_articles", JSON.stringify([{ id: "example", title: "My board", content: "Temperature lesson", tags: [], backlinks: [], createdAt: 1, updatedAt: 1 }]));
    invokeMock.mockImplementation(async (command: string) => command === "wiki_ai_request" ? "Native AI summary" : "local");
  });
  afterEach(() => { cleanup(); vi.restoreAllMocks(); });

  it("calls the native AI bridge without browser fetch or reading API keys into Wiki", async () => {
    const fetch = vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("Browser network disabled"));
    render(<WikiView />);
    fireEvent.click(await screen.findByText("My board"));
    fireEvent.click(screen.getByRole("button", { name: /Summarize Article/ }));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("wiki_ai_request", expect.objectContaining({ userPrompt: expect.stringContaining("Temperature lesson") })));
    expect(await screen.findByText("Native AI summary")).toBeVisible();
    expect(fetch).not.toHaveBeenCalled();
    expect(invokeMock.mock.calls.some(([command]) => /api_key/.test(command))).toBe(false);
  });

  it("removes active HTML, external image requests and unsafe links from Wiki content", () => {
    const html = renderMarkdown('<img src="https://example.org/tracker" onerror="alert(1)"><script>alert(1)</script>\n[bad](javascript:alert)\n**Safe text**');
    const node = document.createElement("div");
    node.innerHTML = html;
    expect(node.querySelector("img,script,iframe")).toBeNull();
    expect(node.querySelector("a")?.getAttribute("href")).toBeNull();
    expect(node.textContent).toContain("Safe text");
  });
});
