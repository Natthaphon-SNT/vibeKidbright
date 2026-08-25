import { useRef, useEffect } from "react";
import Editor, { OnMount, BeforeMount } from "@monaco-editor/react";
import type { editor } from "monaco-editor";

// ── Language detection from file extension ──────────────────────────────────
function getLanguageFromPath(filePath: string): string {
    const ext = filePath.split(".").pop()?.toLowerCase() || "";
    const map: Record<string, string> = {
        c: "c",
        h: "c",
        cpp: "cpp",
        cxx: "cpp",
        cc: "cpp",
        hpp: "cpp",
        py: "python",
        rs: "rust",
        js: "javascript",
        ts: "typescript",
        tsx: "typescript",
        jsx: "javascript",
        json: "json",
        md: "markdown",
        txt: "plaintext",
        cmake: "plaintext",
        yml: "yaml",
        yaml: "yaml",
        toml: "plaintext",
        cfg: "ini",
        ini: "ini",
        sh: "shell",
        bat: "bat",
        ps1: "powershell",
        html: "html",
        css: "css",
        xml: "xml",
        svg: "xml",
    };
    return map[ext] || "plaintext";
}

// ── Custom "Vibe Dark" theme definition ─────────────────────────────────────
const defineVibeDarkTheme: BeforeMount = (monaco) => {
    monaco.editor.defineTheme("vibe-dark", {
        base: "vs-dark",
        inherit: true,
        rules: [
            { token: "comment", foreground: "5c6370", fontStyle: "italic" },
            { token: "comment.block", foreground: "5c6370", fontStyle: "italic" },
            { token: "keyword", foreground: "c678dd" },
            { token: "keyword.control", foreground: "c678dd" },
            { token: "keyword.operator", foreground: "56b6c2" },
            { token: "type", foreground: "e5c07b" },
            { token: "type.identifier", foreground: "e5c07b" },
            { token: "storage.type", foreground: "c678dd" },
            { token: "entity.name.function", foreground: "61afef" },
            { token: "support.function", foreground: "61afef" },
            { token: "string", foreground: "98c379" },
            { token: "string.escape", foreground: "56b6c2" },
            { token: "number", foreground: "d19a66" },
            { token: "constant.numeric", foreground: "d19a66" },
            { token: "keyword.directive", foreground: "e06c75" },
            { token: "keyword.other", foreground: "e06c75" },
            { token: "meta.preprocessor", foreground: "e06c75" },
            { token: "variable", foreground: "e06c75" },
            { token: "variable.predefined", foreground: "e5c07b" },
            { token: "identifier", foreground: "abb2bf" },
            { token: "delimiter", foreground: "abb2bf" },
            { token: "delimiter.bracket", foreground: "abb2bf" },
            { token: "operator", foreground: "56b6c2" },
            { token: "constant", foreground: "d19a66" },
            { token: "constant.language", foreground: "d19a66" },
        ],
        colors: {
            "editor.background": "#0d1220",
            "editor.foreground": "#abb2bf",
            "editor.selectionBackground": "#1e2d52",
            "editor.selectionHighlightBackground": "#1e2d5280",
            "editor.inactiveSelectionBackground": "#1e2d5260",
            "editor.lineHighlightBackground": "#111e36",
            "editor.lineHighlightBorder": "#1e2d52",
            "editorLineNumber.foreground": "#2a3a5a",
            "editorLineNumber.activeForeground": "#4a6090",
            "editorGutter.background": "#0d1220",
            "editorIndentGuide.background": "#1e2d52",
            "editorIndentGuide.activeBackground": "#2a3f6a",
            "editorCursor.foreground": "#4a72c8",
            "editorBracketMatch.background": "#1e2d5280",
            "editorBracketMatch.border": "#4a72c860",
            "minimap.background": "#0d1220",
            "minimapSlider.background": "#1e2d5240",
            "minimapSlider.hoverBackground": "#2a3f6a60",
            "minimapSlider.activeBackground": "#254796a0",
            "scrollbar.shadow": "#00000000",
            "scrollbarSlider.background": "#1e2d5280",
            "scrollbarSlider.hoverBackground": "#2a3f6aa0",
            "scrollbarSlider.activeBackground": "#254796c0",
            "editorWidget.background": "#111e36",
            "editorWidget.border": "#1e2d52",
            "editorHoverWidget.background": "#111e36",
            "editorHoverWidget.border": "#1e2d52",
            "editorSuggestWidget.background": "#111e36",
            "editorSuggestWidget.border": "#1e2d52",
            "editorSuggestWidget.selectedBackground": "#1e2d52",
            "editor.findMatchBackground": "#d19a6640",
            "editor.findMatchHighlightBackground": "#d19a6620",
            "editor.wordHighlightBackground": "#61afef20",
            "editor.wordHighlightStrongBackground": "#61afef30",
            "editor.overviewRulerBorder": "#1e2d52",
        },
    });

    // ── Vibe Light theme (PMS 293) ────────────────────────────────────────────
    monaco.editor.defineTheme("vibe-light", {
        base: "vs",
        inherit: true,
        rules: [
            { token: "comment", foreground: "7a8fb8", fontStyle: "italic" },
            { token: "comment.block", foreground: "7a8fb8", fontStyle: "italic" },
            { token: "keyword", foreground: "7c3aed" },
            { token: "keyword.control", foreground: "7c3aed" },
            { token: "keyword.operator", foreground: "0e6eb8" },
            { token: "type", foreground: "b45309" },
            { token: "type.identifier", foreground: "b45309" },
            { token: "storage.type", foreground: "7c3aed" },
            { token: "entity.name.function", foreground: "254796" },
            { token: "support.function", foreground: "254796" },
            { token: "string", foreground: "0d7f45" },
            { token: "string.escape", foreground: "0e6eb8" },
            { token: "number", foreground: "c2410c" },
            { token: "constant.numeric", foreground: "c2410c" },
            { token: "keyword.directive", foreground: "b91c1c" },
            { token: "keyword.other", foreground: "b91c1c" },
            { token: "meta.preprocessor", foreground: "b91c1c" },
            { token: "variable", foreground: "b91c1c" },
            { token: "variable.predefined", foreground: "b45309" },
            { token: "identifier", foreground: "0d1a3a" },
            { token: "delimiter", foreground: "3d5080" },
            { token: "delimiter.bracket", foreground: "3d5080" },
            { token: "operator", foreground: "0e6eb8" },
            { token: "constant", foreground: "c2410c" },
            { token: "constant.language", foreground: "c2410c" },
        ],
        colors: {
            "editor.background": "#ffffff",
            "editor.foreground": "#0d1a3a",
            "editor.selectionBackground": "#dde6f7",
            "editor.selectionHighlightBackground": "#dde6f780",
            "editor.inactiveSelectionBackground": "#eef2fb",
            "editor.lineHighlightBackground": "#f5f7fc",
            "editor.lineHighlightBorder": "#eef2fb",
            "editorLineNumber.foreground": "#b0bcdb",
            "editorLineNumber.activeForeground": "#7a8fb8",
            "editorGutter.background": "#f8f9fc",
            "editorIndentGuide.background": "#d0daf0",
            "editorIndentGuide.activeBackground": "#aabfe0",
            "editorCursor.foreground": "#254796",
            "editorBracketMatch.background": "#dde6f780",
            "editorBracketMatch.border": "#25479660",
            "minimap.background": "#f5f7fc",
            "minimapSlider.background": "#c0ceea40",
            "minimapSlider.hoverBackground": "#a0b4d860",
            "minimapSlider.activeBackground": "#25479680",
            "scrollbar.shadow": "#00000000",
            "scrollbarSlider.background": "#c0ceea80",
            "scrollbarSlider.hoverBackground": "#a0b4d8a0",
            "scrollbarSlider.activeBackground": "#254796c0",
            "editorWidget.background": "#ffffff",
            "editorWidget.border": "#d0daf0",
            "editorHoverWidget.background": "#ffffff",
            "editorHoverWidget.border": "#d0daf0",
            "editorSuggestWidget.background": "#ffffff",
            "editorSuggestWidget.border": "#d0daf0",
            "editorSuggestWidget.selectedBackground": "#eef2fb",
            "editor.findMatchBackground": "#fcd34d40",
            "editor.findMatchHighlightBackground": "#fcd34d20",
            "editor.wordHighlightBackground": "#dde6f760",
            "editor.wordHighlightStrongBackground": "#dde6f7a0",
            "editor.overviewRulerBorder": "#d0daf0",
        },
    });
};

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "./Toast";
import { DiffEditor } from "@monaco-editor/react";

export interface GotoLineRequest {
    path: string;
    line: number;
    column?: number;
    token: number;
}

interface CodeEditorProps {
    value: string;
    onChange: (value: string) => void;
    filePath: string;
    onSave?: () => void;
    isDarkMode?: boolean;
    gotoLineRequest?: GotoLineRequest | null;
}

export default function CodeEditor({
    value,
    onChange,
    filePath,
    onSave,
    isDarkMode = true,
    gotoLineRequest = null,
}: CodeEditorProps) {
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
    const language = getLanguageFromPath(filePath);
    const monacoTheme = isDarkMode ? "vibe-dark" : "vibe-light";
    
    // State to hold pending diff content
    const [pendingContent, setPendingContent] = useState<string | null>(null);

    // Latest goto request, applied on mount (Monaco loads async) and on change
    const gotoRef = useRef<GotoLineRequest | null>(gotoLineRequest);
    useEffect(() => {
        gotoRef.current = gotoLineRequest;
        const ed = editorRef.current;
        if (ed && gotoLineRequest) {
            ed.revealLineInCenter(gotoLineRequest.line);
            ed.setPosition({ lineNumber: gotoLineRequest.line, column: gotoLineRequest.column ?? 1 });
            ed.focus();
        }
    }, [gotoLineRequest]);

    // Keep onSave callback fresh without recreating keybinding
    const onSaveRef = useRef(onSave);
    useEffect(() => {
        onSaveRef.current = onSave;
    }, [onSave]);

    // Check for pending diffs when the file changes or when AI proposes one
    useEffect(() => {
        if (!filePath) {
            setPendingContent(null);
            return;
        }
        
        let isMounted = true;
        
        const checkDiff = () => {
            invoke<string | null>("check_pending_diff", { path: filePath })
                .then(res => {
                    if (isMounted) {
                        setPendingContent(res);
                    }
                })
                .catch(err => {
                    console.error("Failed to check pending diff:", err);
                    toast("Could not check for pending AI changes.", "error");
                    if (isMounted) setPendingContent(null);
                });
        };
        
        checkDiff(); // Initial check

        let unlisten: (() => void) | null = null;
        
        import("@tauri-apps/api/event").then(({ listen }) => {
            if (!isMounted) return;
            listen("ai-diff-pending", (event) => {
                if (!isMounted) return;
                try {
                    const data = typeof event.payload === 'string' ? JSON.parse(event.payload) : event.payload as any;
                    const eventPath = String(data.fullPath || "");
                    const currentPath = String(filePath || "");
                    
                    if (eventPath && currentPath) {
                        const normalize = (p: string) => p.replace(/\\/g, '/').toLowerCase().replace(/\/+/g, '/');
                        const normEvent = normalize(eventPath);
                        const normCurrent = normalize(currentPath);
                        // Also match by last 2 path segments (e.g. "main/main.c")
                        const tailSegments = (p: string) => p.split('/').slice(-2).join('/');
                        if (normEvent === normCurrent || 
                            normCurrent.endsWith(normEvent) ||
                            normEvent.endsWith(normCurrent) ||
                            tailSegments(normEvent) === tailSegments(normCurrent)) {
                            checkDiff();
                        }
                    }
                } catch (e) {
                    console.error("Error handling ai-diff-pending:", e);
                }            }).then(fn => {
                if (!isMounted) fn();
                else unlisten = fn;
            });
        });

        return () => { 
            isMounted = false; 
            if (unlisten) unlisten();
        };
    }, [filePath]);

    const handleAcceptDiff = async () => {
        try {
            await invoke("accept_diff", { path: filePath });
            setPendingContent(null);
            // The file-modified event will trigger a reload of the content from disk automatically
        } catch (err) {
            console.error("Failed to accept diff:", err);
            toast("Failed to apply AI changes. Please try again.", "error");
        }
    };

    const handleRejectDiff = async () => {
        try {
            await invoke("reject_diff", { path: filePath });
            setPendingContent(null);
        } catch (err) {
            console.error("Failed to reject diff:", err);
            toast("Failed to discard AI changes. Please try again.", "error");
        }
    };

    const handleEditorMount: OnMount = (editor, monaco) => {
        editorRef.current = editor;

        // Register Ctrl+S keybinding for save
        editor.addAction({
            id: "vibe-save-file",
            label: "Save File",
            keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
            run: () => {
                onSaveRef.current?.();
            },
        });

        // Apply a pending jump-to-line request (e.g. from a build error)
        const pendingGoto = gotoRef.current;
        if (pendingGoto) {
            editor.revealLineInCenter(pendingGoto.line);
            editor.setPosition({ lineNumber: pendingGoto.line, column: pendingGoto.column ?? 1 });
        }

        // Focus the editor
        editor.focus();
    };

    const editorOptions = {
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace",
        fontSize: 13,
        fontWeight: "400" as const,
        fontLigatures: true,
        lineHeight: 22,
        letterSpacing: 0.3,
        minimap: { enabled: true, maxColumn: 80, renderCharacters: false, scale: 1 },
        smoothScrolling: true,
        scrollBeyondLastLine: false,
        wordWrap: "off" as const,
        autoIndent: "full" as const,
        formatOnPaste: true,
        tabSize: 4,
        insertSpaces: true,
        bracketPairColorization: { enabled: true },
        autoClosingBrackets: "always" as const,
        autoClosingQuotes: "always" as const,
        matchBrackets: "always" as const,
        cursorBlinking: "smooth" as const,
        cursorSmoothCaretAnimation: "on" as const,
        cursorStyle: "line" as const,
        cursorWidth: 2,
        renderWhitespace: "selection" as const,
        renderLineHighlight: "all" as const,
        guides: { indentation: true, bracketPairs: true },
        padding: { top: 12, bottom: 12 },
        scrollbar: { verticalScrollbarSize: 10, horizontalScrollbarSize: 10, useShadows: false },
        quickSuggestions: false,
        suggestOnTriggerCharacters: false,
        parameterHints: { enabled: false },
        hover: { enabled: true, delay: 600 },
    };

    return (
        <div className="relative w-full h-full" style={{ borderTop: '1px solid var(--border-color)' }}>
            {pendingContent !== null && (
                <div className="absolute top-4 right-8 z-10 flex gap-2 p-2 rounded-lg shadow-xl backdrop-blur-sm" style={{ backgroundColor: 'var(--bg-modal)', border: '1px solid var(--border-color)', boxShadow: 'var(--shadow-lg)' }}>
                    <div className="px-3 py-1 text-xs font-bold rounded flex items-center mr-2" style={{ backgroundColor: 'var(--pms-293-pale)', color: 'var(--accent)' }}>
                        Review AI Changes
                    </div>
                    <button
                        onClick={handleRejectDiff}
                        className="px-4 py-1.5 bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border border-rose-500/30 rounded-md text-xs font-medium flex items-center gap-1.5 transition-colors shadow-sm"
                        title="Discard AI proposed changes"
                    >
                        <span className="text-sm">❌</span> Undo
                    </button>
                    <button
                        onClick={handleAcceptDiff}
                        className="px-4 py-1.5 bg-emerald-500/10 hover:bg-emerald-500/30 text-emerald-400 border border-emerald-500/30 rounded-md text-xs font-medium flex items-center gap-1.5 transition-colors shadow-sm"
                        title="Accept AI proposed changes"
                    >
                        <span className="text-sm">✅</span> Keep
                    </button>
                </div>
            )}
            
            {pendingContent !== null ? (
                <DiffEditor
                    height="100%"
                    language={language}
                    original={value}
                    modified={pendingContent}
                    theme={monacoTheme}
                    beforeMount={defineVibeDarkTheme}
                    options={{
                        ...editorOptions,
                        readOnly: false,
                        originalEditable: false,
                        renderSideBySide: true,
                        diffWordWrap: "off",
                    }}
                />
            ) : (
                <Editor
                    height="100%"
                    language={language}
                    value={value}
                    theme={monacoTheme}
                    beforeMount={defineVibeDarkTheme}
                    onMount={handleEditorMount}
                    onChange={(val) => onChange(val ?? "")}
                    options={{
                        ...editorOptions,
                        lineNumbers: "on",
                        glyphMargin: false,
                        folding: true,
                        foldingHighlight: true,
                        lineDecorationsWidth: 8,
                        lineNumbersMinChars: 4,
                        overviewRulerLanes: 0,
                    }}
                />
            )}
        </div>
    );
}
