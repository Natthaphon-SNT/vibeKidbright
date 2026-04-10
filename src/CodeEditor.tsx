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
// Inspired by One Dark Pro, tuned to match vibeKidbright's neutral-950 background
const defineVibeDarkTheme: BeforeMount = (monaco) => {
    monaco.editor.defineTheme("vibe-dark", {
        base: "vs-dark",
        inherit: true,
        rules: [
            // Comments — muted sage
            { token: "comment", foreground: "5c6370", fontStyle: "italic" },
            { token: "comment.block", foreground: "5c6370", fontStyle: "italic" },

            // Keywords — vivid purple-blue
            { token: "keyword", foreground: "c678dd" },
            { token: "keyword.control", foreground: "c678dd" },
            { token: "keyword.operator", foreground: "56b6c2" },

            // Types — teal-cyan
            { token: "type", foreground: "e5c07b" },
            { token: "type.identifier", foreground: "e5c07b" },
            { token: "storage.type", foreground: "c678dd" },

            // Functions — vivid blue
            { token: "entity.name.function", foreground: "61afef" },
            { token: "support.function", foreground: "61afef" },

            // Strings — warm green
            { token: "string", foreground: "98c379" },
            { token: "string.escape", foreground: "56b6c2" },

            // Numbers — warm orange
            { token: "number", foreground: "d19a66" },
            { token: "constant.numeric", foreground: "d19a66" },

            // Preprocessor directives — desaturated rose
            { token: "keyword.directive", foreground: "e06c75" },
            { token: "keyword.other", foreground: "e06c75" },
            { token: "meta.preprocessor", foreground: "e06c75" },

            // Variables & identifiers
            { token: "variable", foreground: "e06c75" },
            { token: "variable.predefined", foreground: "e5c07b" },
            { token: "identifier", foreground: "abb2bf" },

            // Operators & punctuation
            { token: "delimiter", foreground: "abb2bf" },
            { token: "delimiter.bracket", foreground: "abb2bf" },
            { token: "operator", foreground: "56b6c2" },

            // Constants
            { token: "constant", foreground: "d19a66" },
            { token: "constant.language", foreground: "d19a66" },
        ],
        colors: {
            // Editor background — matches neutral-950
            "editor.background": "#020617",
            "editor.foreground": "#abb2bf",

            // Selection & highlights
            "editor.selectionBackground": "#3e4451",
            "editor.selectionHighlightBackground": "#3e445180",
            "editor.inactiveSelectionBackground": "#3e445160",

            // Current line
            "editor.lineHighlightBackground": "#0f172a",
            "editor.lineHighlightBorder": "#1e293b",

            // Gutter (line numbers)
            "editorLineNumber.foreground": "#3b4252",
            "editorLineNumber.activeForeground": "#7c8598",
            "editorGutter.background": "#020617",

            // Indentation guides
            "editorIndentGuide.background": "#1e293b",
            "editorIndentGuide.activeBackground": "#334155",

            // Cursor
            "editorCursor.foreground": "#528bff",

            // Bracket matching
            "editorBracketMatch.background": "#3e445180",
            "editorBracketMatch.border": "#528bff60",

            // Minimap
            "minimap.background": "#020617",
            "minimapSlider.background": "#1e293b40",
            "minimapSlider.hoverBackground": "#33415560",
            "minimapSlider.activeBackground": "#47556980",

            // Scrollbar
            "scrollbar.shadow": "#00000000",
            "scrollbarSlider.background": "#1e293b80",
            "scrollbarSlider.hoverBackground": "#334155a0",
            "scrollbarSlider.activeBackground": "#475569c0",

            // Widget (autocomplete, hover)
            "editorWidget.background": "#0f172a",
            "editorWidget.border": "#1e293b",
            "editorHoverWidget.background": "#0f172a",
            "editorHoverWidget.border": "#1e293b",
            "editorSuggestWidget.background": "#0f172a",
            "editorSuggestWidget.border": "#1e293b",
            "editorSuggestWidget.selectedBackground": "#1e293b",

            // Search highlight
            "editor.findMatchBackground": "#d19a6640",
            "editor.findMatchHighlightBackground": "#d19a6620",

            // Word highlight
            "editor.wordHighlightBackground": "#61afef20",
            "editor.wordHighlightStrongBackground": "#61afef30",

            // Overscroll
            "editor.overviewRulerBorder": "#1e293b",
        },
    });
};

import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DiffEditor } from "@monaco-editor/react";

interface CodeEditorProps {
    value: string;
    onChange: (value: string) => void;
    filePath: string;
    onSave?: () => void;
}

export default function CodeEditor({
    value,
    onChange,
    filePath,
    onSave,
}: CodeEditorProps) {
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
    const language = getLanguageFromPath(filePath);
    
    // State to hold pending diff content
    const [pendingContent, setPendingContent] = useState<string | null>(null);

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
                }
            }).then(fn => {
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
        }
    };

    const handleRejectDiff = async () => {
        try {
            await invoke("reject_diff", { path: filePath });
            setPendingContent(null);
        } catch (err) {
            console.error("Failed to reject diff:", err);
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
        <div className="relative w-full h-full border-t border-neutral-800">
            {pendingContent !== null && (
                <div className="absolute top-4 right-8 z-10 flex gap-2 p-2 bg-neutral-900 border border-neutral-700 rounded-lg shadow-xl backdrop-blur-sm shadow-black/50">
                    <div className="px-3 py-1 bg-violet-600/20 text-violet-400 text-xs font-bold rounded flex items-center mr-2">
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
                    theme="vibe-dark"
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
                    theme="vibe-dark"
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
