import { useEffect, useMemo, useRef } from "react";

interface DiffReviewProps {
    /** Old code (left, red = removed) */
    original: string;
    /** New code (right, green = added) */
    modified: string;
    isDarkMode?: boolean;
}

type Cell = { n: number; text: string } | null;
interface Row {
    left: Cell;
    right: Cell;
    kind: "equal" | "changed" | "removed" | "added";
}

const split = (s: string) => s.replace(/\r\n/g, "\n").split("\n");

/** Line-level LCS diff, aligned into side-by-side rows. */
export function buildDiffRows(oldText: string, newText: string): Row[] {
    const a = split(oldText);
    const b = split(newText);
    const n = a.length;
    const m = b.length;

    // Trim common prefix/suffix to keep the LCS table small.
    let start = 0;
    while (start < n && start < m && a[start] === b[start]) start++;
    let endA = n;
    let endB = m;
    while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
        endA--;
        endB--;
    }

    const A = a.slice(start, endA);
    const B = b.slice(start, endB);
    const w = B.length + 1;
    // Bound the quadratic work: a large generated file must not exhaust the webview.
    // For large replacements, show the complete changed middle without LCS alignment.
    const useLcs = (A.length + 1) * w <= 1_000_000;
    const lcs = useLcs ? new Uint32Array((A.length + 1) * w) : new Uint32Array(0);
    for (let i = useLcs ? A.length - 1 : -1; i >= 0; i--) {
        for (let j = B.length - 1; j >= 0; j--) {
            lcs[i * w + j] =
                A[i] === B[j]
                    ? lcs[(i + 1) * w + j + 1] + 1
                    : Math.max(lcs[(i + 1) * w + j], lcs[i * w + j + 1]);
        }
    }

    type Op = { t: "eq" | "del" | "ins"; text: string };
    const ops: Op[] = [];
    for (let i = 0; i < start; i++) ops.push({ t: "eq", text: a[i] });
    let i = 0;
    let j = 0;
    while (useLcs && i < A.length && j < B.length) {
        if (A[i] === B[j]) {
            ops.push({ t: "eq", text: A[i] });
            i++;
            j++;
        } else if (lcs[(i + 1) * w + j] >= lcs[i * w + j + 1]) {
            ops.push({ t: "del", text: A[i++] });
        } else {
            ops.push({ t: "ins", text: B[j++] });
        }
    }
    while (i < A.length) ops.push({ t: "del", text: A[i++] });
    while (j < B.length) ops.push({ t: "ins", text: B[j++] });
    for (let k = endA; k < n; k++) ops.push({ t: "eq", text: a[k] });

    // Align: pair up each run of deletions with the following insertions.
    const rows: Row[] = [];
    let ln = 1;
    let rn = 1;
    let p = 0;
    while (p < ops.length) {
        if (ops[p].t === "eq") {
            rows.push({
                left: { n: ln++, text: ops[p].text },
                right: { n: rn++, text: ops[p].text },
                kind: "equal",
            });
            p++;
            continue;
        }
        const dels: string[] = [];
        const inss: string[] = [];
        while (p < ops.length && ops[p].t !== "eq") {
            (ops[p].t === "del" ? dels : inss).push(ops[p].text);
            p++;
        }
        const len = Math.max(dels.length, inss.length);
        for (let k = 0; k < len; k++) {
            const l = k < dels.length ? { n: ln++, text: dels[k] } : null;
            const r = k < inss.length ? { n: rn++, text: inss[k] } : null;
            rows.push({
                left: l,
                right: r,
                kind: l && r ? "changed" : l ? "removed" : "added",
            });
        }
    }
    return rows;
}

export default function DiffReview({ original, modified, isDarkMode = false }: DiffReviewProps) {
    const rows = useMemo(() => buildDiffRows(original, modified), [original, modified]);
    const firstChangeRef = useRef<HTMLDivElement | null>(null);
    const changeCount = rows.filter((r) => r.kind !== "equal").length;
    const firstChangeIdx = rows.findIndex((r) => r.kind !== "equal");

    useEffect(() => {
        const t = setTimeout(() => {
            firstChangeRef.current?.scrollIntoView?.({ block: "center" });
        }, 50);
        return () => clearTimeout(t);
    }, [original, modified]);

    const red = isDarkMode ? "rgba(248,81,73,0.22)" : "rgba(207,34,46,0.16)";
    const redStrong = isDarkMode ? "rgba(248,81,73,0.45)" : "rgba(207,34,46,0.32)";
    const green = isDarkMode ? "rgba(46,160,67,0.24)" : "rgba(45,164,78,0.18)";
    const greenStrong = isDarkMode ? "rgba(46,160,67,0.5)" : "rgba(45,164,78,0.34)";
    const gutterText = isDarkMode ? "#7d8590" : "#8a94ad";

    const cellBase: React.CSSProperties = {
        display: "flex",
        minWidth: 0,
        fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, monospace",
        fontSize: 13,
        lineHeight: "22px",
        whiteSpace: "pre",
    };

    const renderCell = (cell: Cell, kind: Row["kind"], side: "left" | "right") => {
        const isRemoved = side === "left" && (kind === "changed" || kind === "removed");
        const isAdded = side === "right" && (kind === "changed" || kind === "added");
        const bg = isRemoved ? red : isAdded ? green : cell ? "transparent" : "rgba(128,128,128,0.08)";
        const mark = isRemoved ? "−" : isAdded ? "+" : " ";
        return (
            <div style={{ ...cellBase, background: bg }}>
                <span
                    style={{
                        width: 48,
                        flexShrink: 0,
                        textAlign: "right",
                        paddingRight: 8,
                        color: gutterText,
                        userSelect: "none",
                        background: isRemoved ? redStrong : isAdded ? greenStrong : "transparent",
                    }}
                >
                    {cell ? cell.n : ""}
                </span>
                <span
                    style={{
                        width: 18,
                        flexShrink: 0,
                        textAlign: "center",
                        fontWeight: 700,
                        color: isRemoved ? "#f85149" : "#2ea043",
                        userSelect: "none",
                    }}
                >
                    {mark}
                </span>
                <span style={{ paddingRight: 16 }}>{cell ? cell.text || " " : ""}</span>
            </div>
        );
    };

    return (
        <div className="w-full h-full flex flex-col" style={{ background: "var(--bg-primary, transparent)" }}>
            <div
                className="flex text-xs font-bold"
                style={{ borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}
            >
                <div className="flex-1 px-4 py-1.5" style={{ background: red, color: isDarkMode ? "#ffa198" : "#a40e26" }}>
                    Old code (removed)
                </div>
                <div className="flex-1 px-4 py-1.5" style={{ background: green, color: isDarkMode ? "#7ee787" : "#116329" }}>
                    New code (added)
                </div>
            </div>

            {changeCount === 0 && (
                <div className="px-4 py-2 text-xs" style={{ color: "var(--text-muted)" }}>
                    No differences found between the file on disk and the AI's proposed version.
                </div>
            )}

            <div className="flex-1 overflow-auto">
                <div style={{ minWidth: "fit-content" }}>
                    {rows.map((row, idx) => (
                        <div
                            key={idx}
                            ref={idx === firstChangeIdx ? firstChangeRef : undefined}
                            style={{
                                display: "grid",
                                gridTemplateColumns: "1fr 1fr",
                                borderLeft: "0",
                            }}
                        >
                            {renderCell(row.left, row.kind, "left")}
                            <div style={{ borderLeft: "1px solid var(--border-color)" }}>
                                {renderCell(row.right, row.kind, "right")}
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </div>
    );
}
