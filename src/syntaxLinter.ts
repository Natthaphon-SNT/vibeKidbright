/**
 * syntaxLinter.ts — Lightweight C/C++ syntax linter for real-time error checking.
 *
 * Runs entirely client-side (no compiler needed). Detects common syntax issues
 * that beginners hit frequently, such as missing semicolons, unmatched brackets,
 * unterminated strings/comments, bad #include syntax, and non-ASCII characters.
 *
 * Designed to work with Monaco editor's `setModelMarkers()` for wavy underlines
 * and hover tooltips — exactly like VS Code's diagnostic experience.
 */

// ── Public Types ────────────────────────────────────────────────────────────

export interface SyntaxDiagnostic {
    /** 1-indexed line number */
    line: number;
    /** 1-indexed start column */
    startCol: number;
    /** 1-indexed end column (exclusive) */
    endCol: number;
    /** Short English message (shown as first line of tooltip) */
    message: string;
    /** Kid-friendly hint in Thai + English (shown as detail in tooltip) */
    messageHint: string;
    /** Severity level */
    severity: "error" | "warning";
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/** Check if a file path is a C/C++ family file that should be linted. */
export function isCFamilyFile(filePath: string): boolean {
    const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
    return ["c", "h", "cpp", "cxx", "cc", "hpp", "hxx", "ino"].includes(ext);
}

/**
 * Strip single-line comments (//) but preserve strings.
 * Block comments are handled separately since they can span lines.
 */
function stripLineComments(line: string): string {
    let inString = false;
    let strChar = "";
    for (let i = 0; i < line.length; i++) {
        const ch = line[i];
        if (inString) {
            if (ch === "\\" && i + 1 < line.length) { i++; continue; }
            if (ch === strChar) inString = false;
        } else {
            if (ch === '"' || ch === "'") { inString = true; strChar = ch; }
            else if (ch === "/" && i + 1 < line.length && line[i + 1] === "/") {
                return line.slice(0, i);
            }
        }
    }
    return line;
}

// ── Main Linter ─────────────────────────────────────────────────────────────

/**
 * Lint C/C++ source code and return a list of diagnostics.
 * This is intentionally conservative — it may miss some errors but should
 * rarely produce false positives for valid C code.
 */
export function lintCCode(source: string): SyntaxDiagnostic[] {
    const diagnostics: SyntaxDiagnostic[] = [];
    const lines = source.split(/\r?\n/);

    // ── State for multi-line tracking ────────────────────────────────────
    let inBlockComment = false;
    let blockCommentStartLine = 0;
    let blockCommentStartCol = 0;

    // Bracket matching stack: { char, line, col }
    const bracketStack: { char: string; line: number; col: number }[] = [];
    const matchingClose: Record<string, string> = { "(": ")", "[": "]", "{": "}" };
    const matchingOpen: Record<string, string> = { ")": "(", "]": "[", "}": "{" };

    // Duplicate #define tracking
    const definedMacros = new Map<string, number>();

    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
        const rawLine = lines[lineIdx];
        const lineNum = lineIdx + 1;

        // ── 1. Block comment tracking ────────────────────────────────────
        if (inBlockComment) {
            const closeIdx = rawLine.indexOf("*/");
            if (closeIdx !== -1) {
                inBlockComment = false;
                const rest = rawLine.slice(closeIdx + 2);
                processCodeLine(rest, lineNum, closeIdx + 3);
            }
            continue;
        }

        // Strip single-line comments first
        const noLineComment = stripLineComments(rawLine);

        // Process block comments within the line
        let codePart = "";
        let i = 0;
        let inStr = false;
        let strCh = "";
        while (i < noLineComment.length) {
            const ch = noLineComment[i];
            if (inStr) {
                codePart += ch;
                if (ch === "\\" && i + 1 < noLineComment.length) {
                    codePart += noLineComment[i + 1];
                    i += 2;
                    continue;
                }
                if (ch === strCh) inStr = false;
                i++;
                continue;
            }
            if (ch === '"' || ch === "'") {
                inStr = true;
                strCh = ch;
                codePart += ch;
                i++;
                continue;
            }
            if (ch === "/" && i + 1 < noLineComment.length && noLineComment[i + 1] === "*") {
                const closeIdx = noLineComment.indexOf("*/", i + 2);
                if (closeIdx !== -1) {
                    // Block comment opens and closes on the same line
                    i = closeIdx + 2;
                    continue;
                } else {
                    // Unterminated on this line — enters multi-line mode
                    inBlockComment = true;
                    blockCommentStartLine = lineNum;
                    blockCommentStartCol = i + 1;
                    break;
                }
            }
            codePart += ch;
            i++;
        }

        if (inBlockComment && i >= noLineComment.length) {
            processCodeLine(codePart, lineNum, 1);
            continue;
        }

        processCodeLine(codePart, lineNum, 1);
    }

    // ── Post-pass: unterminated block comment ────────────────────────────
    if (inBlockComment) {
        diagnostics.push({
            line: blockCommentStartLine,
            startCol: blockCommentStartCol,
            endCol: blockCommentStartCol + 2,
            message: "Unterminated block comment /* */",
            messageHint:
                "🇹🇭 มี /* ที่ยังไม่ปิดด้วย */ — เพิ่ม */ ตรงที่ต้องการจบ comment\n" +
                "🇬🇧 A /* comment was opened but never closed. Add */ where it should end.",
            severity: "error",
        });
    }

    // ── Post-pass: unmatched opening brackets ────────────────────────────
    for (const item of bracketStack) {
        diagnostics.push({
            line: item.line,
            startCol: item.col,
            endCol: item.col + 1,
            message: `Unmatched '${item.char}' — missing closing '${matchingClose[item.char]}'`,
            messageHint:
                `🇹🇭 มี '${item.char}' ที่ยังไม่ปิดด้วย '${matchingClose[item.char]}' — นับวงเล็บให้ครบคู่\n` +
                `🇬🇧 Opening '${item.char}' has no matching '${matchingClose[item.char]}'. Count your bracket pairs.`,
            severity: "error",
        });
    }

    return diagnostics;

    // ── Inner function: process a single line of code ────────────────────
    function processCodeLine(code: string, lineNum: number, colOffset: number) {
        const trimmed = code.trim();
        if (!trimmed) return;

        // ── 2. #include syntax check ─────────────────────────────────────
        if (/^\s*#\s*include\b/.test(code)) {
            if (!/^\s*#\s*include\s+(<[^>]+>|"[^"]+")/.test(code)) {
                const startCol = colOffset + (code.length - code.trimStart().length);
                diagnostics.push({
                    line: lineNum,
                    startCol,
                    endCol: colOffset + code.length,
                    message: "Invalid #include syntax",
                    messageHint:
                        '🇹🇭 #include ต้องเขียนเป็น #include <file.h> หรือ #include "file.h"\n' +
                        '🇬🇧 #include must be followed by <file.h> or "file.h".',
                    severity: "error",
                });
            }
            return;
        }

        // ── 3. Other preprocessor lines ──────────────────────────────────
        if (/^\s*#/.test(code)) {
            const defineMatch = code.match(/^\s*#\s*define\s+([A-Za-z_]\w*)/);
            if (defineMatch) {
                const macroName = defineMatch[1];
                const prevLine = definedMacros.get(macroName);
                if (prevLine !== undefined) {
                    const macroIdx = code.indexOf(macroName);
                    const startCol = colOffset + macroIdx;
                    diagnostics.push({
                        line: lineNum,
                        startCol,
                        endCol: startCol + macroName.length,
                        message: `Macro '${macroName}' is redefined (first defined on line ${prevLine})`,
                        messageHint:
                            `🇹🇭 มาโคร '${macroName}' ถูก #define ซ้ำ (ครั้งแรกที่บรรทัด ${prevLine}) — เปลี่ยนชื่อ หรือใช้ #undef ก่อน\n` +
                            `🇬🇧 Macro '${macroName}' was already defined on line ${prevLine}. Rename it or add #undef first.`,
                        severity: "warning",
                    });
                } else {
                    definedMacros.set(macroName, lineNum);
                }
            }
            return;
        }

        // ── 4. Unterminated string literal ───────────────────────────────
        {
            let inString = false;
            let openCol = 0;
            let strChr = "";
            for (let j = 0; j < code.length; j++) {
                const ch = code[j];
                if (inString) {
                    if (ch === "\\" && j + 1 < code.length) { j++; continue; }
                    if (ch === strChr) inString = false;
                } else {
                    if (ch === '"') {
                        inString = true;
                        strChr = ch;
                        openCol = j;
                    }
                }
            }
            if (inString) {
                if (!code.trimEnd().endsWith("\\")) {
                    diagnostics.push({
                        line: lineNum,
                        startCol: colOffset + openCol,
                        endCol: colOffset + code.length,
                        message: "Unterminated string literal",
                        messageHint:
                            '🇹🇭 เปิด " แล้วไม่ได้ปิด " ในบรรทัดนี้ — เช็กว่าเครื่องหมายคำพูดครบคู่\n' +
                            '🇬🇧 A string was opened with " but never closed on this line. Check your quotes.',
                        severity: "error",
                    });
                }
            }
        }

        // ── 5. Non-ASCII character detection ─────────────────────────────
        {
            const nonAsciiMatch = code.match(/[\u201C\u201D\u2018\u2019\uFF08\uFF09\uFF1B\uFF1A\uFF0C\u3001\u3002\uFF01\uFF1F]/);
            if (nonAsciiMatch && nonAsciiMatch.index !== undefined) {
                const badChar = nonAsciiMatch[0];
                const col = colOffset + nonAsciiMatch.index;
                diagnostics.push({
                    line: lineNum,
                    startCol: col,
                    endCol: col + 1,
                    message: `Non-ASCII character '${badChar}' (U+${badChar.charCodeAt(0).toString(16).toUpperCase().padStart(4, "0")}) detected in code`,
                    messageHint:
                        "🇹🇭 มีอักขระพิเศษที่ไม่ใช่ ASCII ปนในโค้ด (เช่น อัญประกาศไทย \u201C \u201D หรือวงเล็บ full-width) — ต้องใช้ตัวอักษร ASCII ธรรมดาเท่านั้น\n" +
                        "🇬🇧 A non-ASCII character was found in the code (e.g. smart quotes or full-width punctuation). Replace it with a normal ASCII character.",
                    severity: "error",
                });
            }
        }

        // ── 6. Bracket matching ──────────────────────────────────────────
        {
            let inStr2 = false;
            let strCh2 = "";
            for (let j = 0; j < code.length; j++) {
                const ch = code[j];
                if (inStr2) {
                    if (ch === "\\" && j + 1 < code.length) { j++; continue; }
                    if (ch === strCh2) inStr2 = false;
                    continue;
                }
                if (ch === '"' || ch === "'") {
                    inStr2 = true;
                    strCh2 = ch;
                    continue;
                }
                if (ch === "(" || ch === "[" || ch === "{") {
                    bracketStack.push({ char: ch, line: lineNum, col: colOffset + j });
                } else if (ch === ")" || ch === "]" || ch === "}") {
                    const expected = matchingOpen[ch];
                    if (bracketStack.length > 0 && bracketStack[bracketStack.length - 1].char === expected) {
                        bracketStack.pop();
                    } else if (bracketStack.length > 0 && bracketStack[bracketStack.length - 1].char !== expected) {
                        const top = bracketStack[bracketStack.length - 1];
                        diagnostics.push({
                            line: lineNum,
                            startCol: colOffset + j,
                            endCol: colOffset + j + 1,
                            message: `Mismatched '${ch}' — expected '${matchingClose[top.char]}' to close '${top.char}' from line ${top.line}`,
                            messageHint:
                                `🇹🇭 วงเล็บ '${ch}' ไม่ตรงกับ '${top.char}' ที่เปิดไว้ที่บรรทัด ${top.line} — ต้องปิดด้วย '${matchingClose[top.char]}' ก่อน\n` +
                                `🇬🇧 '${ch}' doesn't match the '${top.char}' opened on line ${top.line}. Close it with '${matchingClose[top.char]}' first.`,
                            severity: "error",
                        });
                        bracketStack.pop();
                    } else {
                        diagnostics.push({
                            line: lineNum,
                            startCol: colOffset + j,
                            endCol: colOffset + j + 1,
                            message: `Unexpected '${ch}' — no matching opening bracket`,
                            messageHint:
                                `🇹🇭 มี '${ch}' เกินมา โดยไม่มีวงเล็บเปิดจับคู่ — ลบออก หรือเพิ่มวงเล็บเปิดให้ครบ\n` +
                                `🇬🇧 Extra '${ch}' with no matching opening bracket. Remove it or add the missing opener.`,
                            severity: "error",
                        });
                    }
                }
            }
        }

        // ── 7. Missing semicolon heuristic ───────────────────────────────
        {
            const stripped = trimmed;

            // Skip lines that don't need semicolons
            if (
                stripped.endsWith("{") ||
                stripped.endsWith("}") ||
                stripped.endsWith("},") ||
                stripped.endsWith(";") ||
                stripped.endsWith(",") ||
                stripped.endsWith("\\") ||
                stripped.endsWith(":") ||
                stripped.startsWith("//") ||
                stripped === "{" ||
                stripped === "}" ||
                /^\s*(if|else|for|while|switch|do|case|default)\b/.test(stripped) ||
                /^\s*(typedef\s+)?(struct|union|enum)\s+\w*\s*$/.test(stripped) ||
                /^[a-zA-Z_]\w*\s*:$/.test(stripped) ||
                /^(static\s+|extern\s+|inline\s+|const\s+|volatile\s+|unsigned\s+|signed\s+)*\w+[\s*]+\w+\s*\(/.test(stripped)
            ) {
                return;
            }

            // Heuristic: looks like a statement that needs a semicolon
            const looksLikeStatement =
                /^(return\s|break|continue|goto\s)/.test(stripped) ||
                /^[a-zA-Z_]\w*\s*\(.*\)\s*$/.test(stripped) ||
                /^[a-zA-Z_]\w*(\s*(\[.*\]))*\s*=\s*.+[^;{},\\]\s*$/.test(stripped) ||
                /^[a-zA-Z_]\w*(\s*(\[.*\]))*\s*(\+\+|--)\s*$/.test(stripped);

            if (looksLikeStatement) {
                const endCol = colOffset + code.trimEnd().length;
                diagnostics.push({
                    line: lineNum,
                    startCol: Math.max(1, endCol - 1),
                    endCol: endCol + 1,
                    message: "Missing ';' at end of statement",
                    messageHint:
                        "🇹🇭 ลืมใส่เครื่องหมาย ; ท้ายคำสั่ง — เพิ่ม ; ต่อท้ายบรรทัดนี้\n" +
                        "🇬🇧 A semicolon ; is missing at the end of this statement. Add ; at the end.",
                    severity: "error",
                });
            }
        }
    }
}
