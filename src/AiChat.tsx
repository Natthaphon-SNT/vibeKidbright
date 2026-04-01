import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Message {
    role: "user" | "assistant";
    content: string;
    toolCalls?: { name: string; result?: string }[];
}

function ApplyButton({ onApply }: { onApply: () => void }) {
    const [applied, setApplied] = useState(false);

    const handleClick = () => {
        onApply();
        setApplied(true);
        setTimeout(() => setApplied(false), 2000);
    };

    if (applied) {
        return (
            <div className="bg-emerald-600 text-white px-2 py-0.5 rounded text-[8px] transition-all font-bold flex items-center gap-1">
                <svg className="w-2 h-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                </svg>
                APPLIED
            </div>
        );
    }

    return (
        <button
            onClick={handleClick}
            className="opacity-0 group-hover/code:opacity-100 bg-violet-600 hover:bg-violet-500 text-white px-2 py-0.5 rounded text-[8px] transition-all font-bold"
        >
            APPLY
        </button>
    );
}

function AiChat({ projectDir, onInjectCode }: { projectDir: string, onInjectCode: (code: string) => void }) {
    const [messages, setMessages] = useState<Message[]>([]);
    const [input, setInput] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const [streamingText, setStreamingText] = useState("");
    const [activeTools, setActiveTools] = useState<string[]>([]);
    const [showSettings, setShowSettings] = useState(false);
    const [api_key, setApiKey] = useState("");
    const [apiKeyInput, setApiKeyInput] = useState("");
    const [baseUrl, setBaseUrl] = useState("https://api.openai.com/v1");
    const [baseUrlInput, setBaseUrlInput] = useState("https://api.openai.com/v1");
    const [provider, setProvider] = useState<"openai" | "local">("openai");
    const [providerInput, setProviderInput] = useState<"openai" | "local">("openai");
    const [modelInput, setModelInput] = useState("gpt-4o");
    const [knowledgeFiles, setKnowledgeFiles] = useState<string[]>([]);
    const [isIndexing, setIsIndexing] = useState(false);
    const scrollRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLTextAreaElement>(null);

    useEffect(() => {
        // Load API key and model on mount
        invoke("get_api_key").then((key) => {
            const k = key as string;
            setApiKey(k);
            setApiKeyInput(k);
        });
        invoke("get_model").then((m) => {
            const mod = m as string;
            setModelInput(mod);
        });
        invoke("get_base_url").then((url) => {
            const u = url as string;
            setBaseUrl(u);
            setBaseUrlInput(u);
        });
        invoke("get_provider").then((p) => {
            const pr = p as "openai" | "local";
            setProvider(pr);
            setProviderInput(pr);
        });
        // Listen for streaming events
        const unlistenDelta = listen("ai-chat-delta", (event) => {
            setStreamingText((prev) => prev + (event.payload as string));
        });

        const unlistenToolStart = listen("ai-chat-tool-start", (event) => {
            const data = JSON.parse(event.payload as string);
            setActiveTools((prev) => [...prev, data.name]);
        });

        const unlistenToolResult = listen("ai-chat-tool-result", (event) => {
            const data = JSON.parse(event.payload as string);
            setActiveTools((prev) => prev.filter((t) => t !== data.name));
            // Add tool info to current message
            setMessages((prev) => {
                const last = prev[prev.length - 1];
                if (last && last.role === "assistant") {
                    const toolResult = typeof data.result === 'string'
                        ? data.result
                        : JSON.stringify(data.result);

                    const updated = { ...last };
                    updated.toolCalls = [
                        ...(updated.toolCalls || []),
                        { name: data.name, result: toolResult.substring(0, 500) },
                    ];
                    return [...prev.slice(0, -1), updated];
                }
                return prev;
            });
        });

        const unlistenDone = listen("ai-chat-done", () => {
            setStreamingText((prev) => {
                if (prev) {
                    setMessages((msgs) => {
                        const last = msgs[msgs.length - 1];
                        if (last && last.role === "assistant") {
                            return [
                                ...msgs.slice(0, -1),
                                { ...last, content: last.content + prev },
                            ];
                        }
                        return [...msgs, { role: "assistant", content: prev }];
                    });
                }
                return "";
            });
            setIsLoading(false);
            setActiveTools([]);
        });

        const unlistenError = listen("ai-chat-error", (event) => {
            setIsLoading(false);
            setStreamingText("");
            setActiveTools([]);
            setMessages((prev) => [
                ...prev,
                {
                    role: "assistant",
                    content: `❌ Error: ${event.payload as string}`,
                },
            ]);
        });

        return () => {
            unlistenDelta.then((f) => f());
            unlistenToolStart.then((f) => f());
            unlistenToolResult.then((f) => f());
            unlistenDone.then((f) => f());
            unlistenError.then((f) => f());
        };
    }, []);

    useEffect(() => {
        invoke("get_knowledge_base_files", { projectDir }).then((files) => {
            setKnowledgeFiles(files as string[]);
        });
    }, [projectDir]);

    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [messages, streamingText, activeTools]);

    const sendMessage = async () => {
        if (!input.trim() || isLoading) return;

        if (!api_key && !baseUrl.includes("localhost") && !baseUrl.includes("127.0.0.1")) {
            setShowSettings(true);
            return;
        }

        const userMessage: Message = { role: "user", content: input.trim() };
        const newMessages = [...messages, userMessage];
        setMessages(newMessages);
        setInput("");
        setIsLoading(true);
        setStreamingText("");

        // Add empty assistant message placeholder
        setMessages((prev) => [...prev, { role: "assistant", content: "" }]);

        // Convert messages to the format the backend expects
        const apiMessages = newMessages.map((m) => ({
            role: m.role,
            content: m.content,
        }));

        try {
            await invoke("send_ai_message", {
                messages: apiMessages,
                projectDir,
            });
        } catch (err) {
            setIsLoading(false);
            setMessages((prev) => [
                ...prev.slice(0, -1), // Remove placeholder
                {
                    role: "assistant" as const,
                    content: `❌ Error: ${err}`,
                },
            ]);
        }
    };

    const saveSettings = async () => {
        try {
            await invoke("set_api_key", { key: apiKeyInput });
            await invoke("set_model", { model: modelInput });
            await invoke("set_base_url", { url: baseUrlInput });
            await invoke("set_provider", { provider: providerInput });
            setApiKey(apiKeyInput);
            setBaseUrl(baseUrlInput);
            setProvider(providerInput);
            setShowSettings(false);
        } catch (err) {
            console.error("Failed to save AI settings:", err);
        }
    };

    const handleProviderChange = (newProvider: "openai" | "local") => {
        setProviderInput(newProvider);
        if (newProvider === "openai") {
            setBaseUrlInput("https://api.openai.com/v1");
            setModelInput("gpt-4o");
        } else {
            setBaseUrlInput("http://localhost:1234/v1");
            setModelInput("qwen2.5-coder-7b-instruct");
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            sendMessage();
        }
    };

    const renderMarkdown = (text: string) => {
        // Simple markdown rendering
        const lines = text.split("\n");
        const elements: React.ReactNode[] = [];
        let inCodeBlock = false;
        let codeLanguage = "";
        let codeContent: string[] = [];
        let key = 0;

        for (const line of lines) {
            if (line.startsWith("```")) {
                if (inCodeBlock) {
                    // End code block
                    elements.push(
                        <div key={key++} className="my-2 rounded-lg overflow-hidden relative group/code">
                            <div className="bg-slate-700 px-3 py-1 text-[10px] text-slate-400 font-mono uppercase flex justify-between items-center h-7">
                                <span>{codeLanguage || "code"}</span>
                                <ApplyButton onApply={() => onInjectCode(codeContent.join("\n"))} />
                            </div>
                            <pre className="bg-slate-800 p-3 overflow-x-auto text-xs">
                                <code>{codeContent.join("\n")}</code>
                            </pre>
                        </div>
                    );

                    inCodeBlock = false;
                    codeContent = [];
                    codeLanguage = "";
                } else {
                    inCodeBlock = true;
                    codeLanguage = line.slice(3).trim();
                }
            } else if (inCodeBlock) {
                codeContent.push(line);
            } else if (line.startsWith("### ")) {
                elements.push(
                    <h4 key={key++} className="font-bold text-sm mt-3 mb-1 text-sky-300">
                        {line.slice(4)}
                    </h4>
                );
            } else if (line.startsWith("## ")) {
                elements.push(
                    <h3 key={key++} className="font-bold text-base mt-3 mb-1 text-sky-300">
                        {line.slice(3)}
                    </h3>
                );
            } else if (line.startsWith("# ")) {
                elements.push(
                    <h2 key={key++} className="font-bold text-lg mt-3 mb-1 text-sky-300">
                        {line.slice(2)}
                    </h2>
                );
            } else if (line.startsWith("- ") || line.startsWith("* ")) {
                elements.push(
                    <div key={key++} className="flex gap-2 ml-2">
                        <span className="text-sky-500">•</span>
                        <span>{renderInlineCode(line.slice(2))}</span>
                    </div>
                );
            } else if (line.match(/^\d+\. /)) {
                const num = line.match(/^(\d+)\. /)?.[1];
                elements.push(
                    <div key={key++} className="flex gap-2 ml-2">
                        <span className="text-sky-500 font-mono text-xs min-w-[1.2em]">{num}.</span>
                        <span>{renderInlineCode(line.replace(/^\d+\. /, ""))}</span>
                    </div>
                );
            } else if (line.trim() === "") {
                elements.push(<div key={key++} className="h-2" />);
            } else {
                elements.push(
                    <p key={key++}>{renderInlineCode(line)}</p>
                );
            }
        }

        // Handle unclosed code block
        if (inCodeBlock) {
            elements.push(
                <div key={key++} className="my-2 rounded-lg overflow-hidden">
                    {codeLanguage && (
                        <div className="bg-slate-700 px-3 py-1 text-[10px] text-slate-400 font-mono uppercase">
                            {codeLanguage}
                        </div>
                    )}
                    <pre className="bg-slate-800 p-3 overflow-x-auto text-xs">
                        <code>{codeContent.join("\n")}</code>
                    </pre>
                </div>
            );
        }

        return elements;
    };

    const renderInlineCode = (text: string) => {
        const parts = text.split(/(`[^`]+`)/g);
        return parts.map((part, i) => {
            if (part.startsWith("`") && part.endsWith("`")) {
                return (
                    <code key={i} className="bg-slate-700 px-1.5 py-0.5 rounded text-sky-300 text-xs font-mono">
                        {part.slice(1, -1)}
                    </code>
                );
            }
            // Bold
            const boldParts = part.split(/(\*\*[^*]+\*\*)/g);
            return boldParts.map((bp, j) => {
                if (bp.startsWith("**") && bp.endsWith("**")) {
                    return <strong key={`${i}-${j}`}>{bp.slice(2, -2)}</strong>;
                }
                return <span key={`${i}-${j}`}>{bp}</span>;
            });
        });
    };

    return (
        <div className="flex flex-col h-full bg-slate-950">
            {/* Header */}
            <div className="h-10 border-b border-slate-800 flex items-center justify-between px-4 bg-slate-900/80 backdrop-blur-sm shrink-0">
                <div className="flex items-center gap-2">
                    <div className="w-5 h-5 rounded bg-gradient-to-br from-violet-500 to-indigo-600 flex items-center justify-center text-[10px] font-bold text-white shadow-lg shadow-violet-500/20">
                        AI
                    </div>
                    <span className="text-xs font-bold text-slate-300 tracking-wide">
                        Vibe Coder
                    </span>
                    <span className={`text-[8px] px-1.5 py-0.5 rounded font-bold uppercase ${provider === "openai" ? "bg-violet-500/10 text-violet-400" : "bg-emerald-500/10 text-emerald-400"}`}>
                        {provider === "openai" ? "Cloud" : "Local"}
                    </span>
                </div>
                <button
                    onClick={() => setShowSettings(true)}
                    className="text-slate-500 hover:text-slate-300 transition-colors"
                    title="Settings"
                >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                </button>
            </div>

            {/* Settings Modal */}
            {showSettings && (
                <div className="absolute inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
                    <div className="bg-slate-800 border border-slate-700 rounded-xl p-6 w-96 shadow-2xl overflow-y-auto max-h-[90vh]">
                        <h3 className="text-sm font-bold text-slate-200 mb-4">
                            AI Provider Settings
                        </h3>

                        {/* Provider Switcher Tabs */}
                        <div className="flex bg-slate-900 p-1 rounded-lg mb-6 border border-slate-700">
                            <button
                                onClick={() => handleProviderChange("openai")}
                                className={`flex-1 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${providerInput === "openai" ? "bg-violet-600 text-white shadow-lg" : "text-slate-500 hover:text-slate-300"}`}
                            >
                                Cloud (OpenAI)
                            </button>
                            <button
                                onClick={() => handleProviderChange("local")}
                                className={`flex-1 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${providerInput === "local" ? "bg-emerald-600 text-white shadow-lg" : "text-slate-500 hover:text-slate-300"}`}
                            >
                                Local (LM Studio)
                            </button>
                        </div>

                        <div className="space-y-4 mb-6">
                            {providerInput === "openai" && (
                                <div className="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                                    <div>
                                        <label className="text-xs text-slate-400 mb-1 block">
                                            OpenAI API Key
                                        </label>
                                        <input
                                            type="password"
                                            value={apiKeyInput}
                                            onChange={(e) => setApiKeyInput(e.target.value)}
                                            placeholder="sk-..."
                                            className="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-violet-500 transition-colors"
                                        />
                                    </div>
                                    <div>
                                        <label className="text-xs text-slate-400 mb-2 block font-bold uppercase tracking-wider">
                                            Cloud Model
                                        </label>
                                        <div className="grid grid-cols-1 gap-2">
                                            {[
                                                { name: "GPT-4o (Standard)", id: "gpt-4o" },
                                                { name: "GPT-4o Mini (Fast)", id: "gpt-4o-mini" },
                                                { name: "o1-preview (Reasoning)", id: "o1-preview" },
                                            ].map((m) => (
                                                <button
                                                    key={m.id}
                                                    onClick={() => setModelInput(m.id)}
                                                    className={`text-left px-3 py-2 rounded-lg text-xs transition-colors border ${modelInput === m.id
                                                        ? "bg-violet-600/20 border-violet-500 text-violet-300"
                                                        : "bg-slate-900/50 border-slate-700 text-slate-400 hover:bg-slate-800"
                                                        }`}
                                                >
                                                    {m.name}
                                                </button>
                                            ))}
                                        </div>
                                    </div>
                                </div>
                            )}

                            {providerInput === "local" && (
                                <div className="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                                    <div>
                                        <label className="text-xs text-slate-400 mb-1 block">
                                            LM Studio Server URL
                                        </label>
                                        <input
                                            type="text"
                                            value={baseUrlInput}
                                            onChange={(e) => setBaseUrlInput(e.target.value)}
                                            placeholder="http://localhost:1234/v1"
                                            className="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-emerald-500 transition-colors"
                                        />
                                        <p className="text-[10px] text-slate-500 mt-1">Note: /v1 is required in LM Studio</p>
                                    </div>
                                    <div>
                                        <label className="text-xs text-slate-400 mb-1 block">
                                            Local Model ID
                                        </label>
                                        <input
                                            type="text"
                                            value={modelInput}
                                            onChange={(e) => setModelInput(e.target.value)}
                                            placeholder="qwen2.5-coder-7b-instruct"
                                            className="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-emerald-500 transition-colors"
                                        />
                                        <p className="text-[10px] text-amber-500 mt-1 font-bold">⚠️ Must match the "Model ID" in LM Studio</p>
                                    </div>
                                    <div>
                                        <label className="text-xs text-slate-400 mb-1 block">
                                            Local API Key (Keep empty if not needed)
                                        </label>
                                        <input
                                            type="password"
                                            value={apiKeyInput}
                                            onChange={(e) => setApiKeyInput(e.target.value)}
                                            placeholder="not required for local"
                                            className="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-emerald-500 transition-colors opacity-50"
                                        />
                                    </div>
                                </div>
                            )}
                        </div>

                        <div className="border-t border-slate-700 pt-4 mb-6">
                            <label className="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-3 block">
                                Extra Capabilities
                            </label>
                            <div className="bg-slate-900/50 border border-slate-700 rounded-lg p-3">
                                <div className="flex items-center gap-2 mb-1">
                                    <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
                                    <span className="text-xs font-bold text-slate-300">Web Search Enabled</span>
                                </div>
                                <p className="text-[10px] text-slate-500">
                                    AI will automatically search DuckDuckGo for documentation and technical info. No API key required.
                                </p>
                            </div>

                            <div className="mt-4">
                                <label className="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-2 block flex justify-between items-center">
                                    Local Knowledge Base
                                    <div className="flex items-center gap-2">
                                        <button
                                            onClick={() => {
                                                setIsIndexing(true);
                                                invoke("refresh_knowledge_base", { projectDir })
                                                    .then(() => invoke("get_knowledge_base_files", { projectDir }))
                                                    .then(f => setKnowledgeFiles(f as string[]))
                                                    .finally(() => setIsIndexing(false));
                                            }}
                                            disabled={isIndexing}
                                            className={`text-[10px] flex items-center gap-1 ${isIndexing ? 'text-slate-500' : 'text-emerald-400 hover:underline'}`}
                                        >
                                            <svg className={`w-2.5 h-2.5 ${isIndexing ? 'animate-spin' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                            </svg>
                                            {isIndexing ? 'Indexing...' : 'Re-index'}
                                        </button>
                                        <button
                                            onClick={() => invoke("open_knowledge_base_folder", { projectDir }).then(() => invoke("get_knowledge_base_files", { projectDir }).then(f => setKnowledgeFiles(f as string[])))}
                                            className="text-[10px] text-violet-400 hover:underline flex items-center gap-1"
                                        >
                                            <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                                            </svg>
                                            Manage Folder
                                        </button>
                                    </div>
                                </label>
                                <div className="space-y-2">
                                    <div className="flex flex-wrap gap-1.5 min-h-[40px] p-2 bg-slate-900/50 border border-slate-700 rounded-lg">
                                        {knowledgeFiles.length === 0 ? (
                                            <span className="text-[10px] text-slate-600 italic">No custom docs added...</span>
                                        ) : (
                                            knowledgeFiles.map(file => (
                                                <div key={file} className="flex items-center gap-1.5 px-2 py-0.5 bg-slate-800 border border-slate-600 rounded text-[10px] text-slate-300">
                                                    <svg className="w-2.5 h-2.5 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                                    </svg>
                                                    {file}
                                                </div>
                                            ))
                                        )}
                                    </div>
                                    <p className="text-[10px] text-slate-500">Add .txt or .md files to `/knowledge_base` to give the AI project-specific context.</p>
                                </div>
                            </div>
                        </div>

                        <div className="flex gap-2">
                            <button
                                onClick={() => setShowSettings(false)}
                                className="flex-1 py-2 bg-slate-700 hover:bg-slate-600 text-sm text-slate-300 rounded-lg transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={saveSettings}
                                className="flex-1 py-2 bg-violet-600 hover:bg-violet-500 text-sm text-white rounded-lg transition-colors font-medium"
                            >
                                Save
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {/* Messages */}
            <div
                ref={scrollRef}
                className="flex-1 overflow-y-auto p-4 space-y-4"
            >
                {messages.length === 0 && !isLoading && (
                    <div className="flex flex-col items-center justify-center h-full text-center opacity-50">
                        <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-violet-500/20 to-indigo-600/20 border border-violet-500/20 flex items-center justify-center mb-3">
                            <span className="text-lg">✨</span>
                        </div>
                        <p className="text-sm text-slate-500 font-medium">
                            Ask me anything about your ESP-IDF project
                        </p>
                        <p className="text-xs text-slate-600 mt-1">
                            I can read, write files, and run commands
                        </p>
                    </div>
                )}

                {messages.map((msg, i) => (
                    <div
                        key={i}
                        className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
                    >
                        <div
                            className={`max-w-[90%] rounded-xl px-4 py-3 text-sm leading-relaxed ${msg.role === "user"
                                ? "bg-violet-600/80 text-white rounded-br-sm"
                                : "bg-slate-800/80 text-slate-200 rounded-bl-sm border border-slate-700/50"
                                }`}
                        >
                            {msg.role === "assistant" ? (
                                <>
                                    {/* Tool call indicators */}
                                    {msg.toolCalls?.map((tc, j) => (
                                        <div
                                            key={j}
                                            className="flex items-center gap-2 text-xs text-slate-400 mb-2 bg-slate-700/50 rounded-lg px-2 py-1.5"
                                        >
                                            <span className="text-emerald-400">⚡</span>
                                            <span className="font-mono">{tc.name}</span>
                                            <span className="text-slate-600">✓</span>
                                        </div>
                                    ))}
                                    {/* Content with markdown */}
                                    <div className="prose-sm">{renderMarkdown(msg.content)}</div>
                                </>
                            ) : (
                                <div className="whitespace-pre-wrap">{msg.content}</div>
                            )}
                        </div>
                    </div>
                ))}

                {/* Streaming text */}
                {isLoading && streamingText && (
                    <div className="flex justify-start">
                        <div className="max-w-[90%] rounded-xl rounded-bl-sm px-4 py-3 bg-slate-800/80 text-slate-200 text-sm leading-relaxed border border-slate-700/50">
                            <div className="prose-sm">{renderMarkdown(streamingText)}</div>
                            <span className="inline-block w-1.5 h-4 bg-violet-400 animate-pulse ml-0.5 align-middle" />
                        </div>
                    </div>
                )}

                {/* Active tool indicators */}
                {activeTools.length > 0 && (
                    <div className="flex justify-start">
                        <div className="rounded-xl px-4 py-2 bg-slate-800/50 border border-slate-700/50 text-xs text-slate-400 flex items-center gap-2">
                            <svg className="w-3 h-3 animate-spin text-violet-400" fill="none" viewBox="0 0 24 24">
                                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                            </svg>
                            <span className="font-mono">{activeTools[activeTools.length - 1]}</span>
                        </div>
                    </div>
                )}

                {/* Loading indicator when no text yet */}
                {isLoading && !streamingText && activeTools.length === 0 && (
                    <div className="flex justify-start">
                        <div className="rounded-xl px-4 py-3 bg-slate-800/50 border border-slate-700/50">
                            <div className="flex gap-1">
                                <div className="w-2 h-2 bg-violet-400 rounded-full animate-bounce" style={{ animationDelay: "0ms" }} />
                                <div className="w-2 h-2 bg-violet-400 rounded-full animate-bounce" style={{ animationDelay: "150ms" }} />
                                <div className="w-2 h-2 bg-violet-400 rounded-full animate-bounce" style={{ animationDelay: "300ms" }} />
                            </div>
                        </div>
                    </div>
                )}
            </div>

            {/* Input */}
            <div className="p-3 border-t border-slate-800 bg-slate-900/60 shrink-0">
                <div className="flex gap-2 items-end">
                    <textarea
                        ref={inputRef}
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder="Ask about your code..."
                        rows={1}
                        className="flex-1 bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 resize-none focus:outline-none focus:border-violet-500 transition-colors placeholder:text-slate-600 max-h-32 overflow-y-auto"
                        style={{ minHeight: "36px" }}
                    />
                    <button
                        onClick={sendMessage}
                        disabled={isLoading || !input.trim()}
                        className={`p-2 rounded-lg transition-all duration-200 ${isLoading || !input.trim()
                            ? "bg-slate-700 text-slate-500 cursor-not-allowed"
                            : "bg-violet-600 hover:bg-violet-500 text-white shadow-lg shadow-violet-500/20 active:scale-95"
                            }`}
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                        </svg>
                    </button>
                </div>
                <p className="text-[10px] text-slate-600 mt-1 ml-1">
                    Enter to send · Shift+Enter for new line
                </p>
            </div>
        </div>
    );
}

export default AiChat;
