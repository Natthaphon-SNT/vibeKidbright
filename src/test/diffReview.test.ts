import { describe, expect, it } from "vitest";
import { buildDiffRows } from "../DiffReview";

describe("diff review line preservation", () => {
  it("aligns a small insertion without changing equal surrounding lines", () => {
    const rows = buildDiffRows("header\nbody\nend", "header\ninserted\nbody\nend");
    expect(rows.map(row => row.kind)).toEqual(["equal", "added", "equal", "equal"]);
    expect(rows[1].right).toEqual({ n: 2, text: "inserted" });
  });

  it("retains every line and its line number for a large generated replacement", () => {
    const before = ["header", ...Array.from({ length: 4000 }, (_, i) => `old ${i}`), "end"];
    const after = ["header", ...Array.from({ length: 4200 }, (_, i) => `new ${i}`), "end"];
    const rows = buildDiffRows(before.join("\n"), after.join("\n"));
    expect(rows.flatMap(row => row.left ? [row.left.text] : [])).toEqual(before);
    expect(rows.flatMap(row => row.right ? [row.right.text] : [])).toEqual(after);
    expect(rows[0].kind).toBe("equal");
    expect(rows[rows.length - 1]?.kind).toBe("equal");
    expect(rows[rows.length - 1]?.right?.n).toBe(after.length);
  });
});
