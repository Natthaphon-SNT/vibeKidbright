import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Message {
    id: string;
    role: "user" | "assistant";
    content: string;
    toolCalls?: { name: string; result?: string }[];
}

interface ChatSession {
    id: string;
    title: string;
    messages: Message[];
    updatedAt: number;
}

function ApplyButton({ onApply, targetFile }: { onApply: () => void, targetFile?: string | null }) {
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
            {targetFile ? `APPLY TO ${targetFile}` : "APPLY"}
        </button>
    );
}


function AiChat({ projectDir, onInjectCode, onApplyToFile }: { projectDir: string, onInjectCode: (code: string) => void, onApplyToFile?: (filePath: string, code: string) => void }) {
    const [messages, setMessages] = useState<Message[]>([]);
    const [input, setInput] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const [streamingText, setStreamingText] = useState("");
    const [activeTools, setActiveTools] = useState<string[]>([]);
    const [showSettings, setShowSettings] = useState(false);
    const [showHistory, setShowHistory] = useState(false);
    const [activeModelBadge, setActiveModelBadge] = useState<string | null>(null);

    // Chat Sessions
    const [sessions, setSessions] = useState<ChatSession[]>([]);
    const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);

    const [api_key, setApiKey] = useState("");
    const [apiKeyInput, setApiKeyInput] = useState("");
    const [baseUrl, setBaseUrl] = useState("https://api.openai.com/v1");
    const [baseUrlInput, setBaseUrlInput] = useState("https://api.openai.com/v1");
    const [provider, setProvider] = useState<"openai" | "local" | "openrouter" | "google">("openai");
    const [providerInput, setProviderInput] = useState<"openai" | "local" | "openrouter" | "google">("openai");
    const [modelInput, setModelInput] = useState("gpt-4o");
    const [openrouterApiKey, setOpenrouterApiKey] = useState("");
    const [openrouterApiKeyInput, setOpenrouterApiKeyInput] = useState("");
    const [_openrouterModel, setOpenrouterModel] = useState("qwen/qwen3-coder:free");
    const [openrouterModelInput, setOpenrouterModelInput] = useState("qwen/qwen3-coder:free");
    const [googleApiKey, setGoogleApiKey] = useState("");
    const [googleApiKeyInput, setGoogleApiKeyInput] = useState("");
    const [_googleModel, setGoogleModel] = useState("gemini-2.5-flash");
    const [googleModelInput, setGoogleModelInput] = useState("gemini-2.5-flash");
    const [knowledgeFiles, setKnowledgeFiles] = useState<string[]>([]);
    const [isIndexing, setIsIndexing] = useState(false);
    const scrollRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLTextAreaElement>(null);

    // Auto-resize prompt textarea
    useEffect(() => {
        if (inputRef.current) {
            inputRef.current.style.height = 'auto';
            inputRef.current.style.height = `${inputRef.current.scrollHeight}px`;
        }
    }, [input]);

    useEffect(() => {
        // Load sessions from localStorage
        try {
            const saved = localStorage.getItem("vibe_chat_sessions");
            if (saved) {
                const parsed = JSON.parse(saved);
                setSessions(parsed);
                if (parsed.length > 0) {
                    const latest = parsed.sort((a: ChatSession, b: ChatSession) => b.updatedAt - a.updatedAt)[0];
                    setCurrentSessionId(latest.id);
                    setMessages(latest.messages);
                }
            }
        } catch (e) {
            console.error("Failed to load chat history:", e);
        }

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
            const pr = p as "openai" | "local" | "openrouter" | "google";
            setProvider(pr);
            setProviderInput(pr);
        });
        invoke("get_openrouter_api_key").then((key) => {
            const k = key as string;
            setOpenrouterApiKey(k);
            setOpenrouterApiKeyInput(k);
        });
        invoke("get_openrouter_model").then((m) => {
            const mod = m as string;
            setOpenrouterModel(mod);
            setOpenrouterModelInput(mod);
        });
        invoke("get_google_api_key").then((key) => {
            const k = key as string;
            setGoogleApiKey(k);
            setGoogleApiKeyInput(k);
        });
        invoke("get_google_model").then((m) => {
            const mod = m as string;
            setGoogleModel(mod);
            setGoogleModelInput(mod);
        });
        // Listen for streaming events
        const unlistenActiveModel = listen("ai-active-model", (event) => {
            setActiveModelBadge(event.payload as string);
        });

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
                setMessages((msgs) => {
                    const last = msgs[msgs.length - 1];
                    if (last && last.role === "assistant") {
                        // อัปเดตข้อความสุดท้ายให้สมบูรณ์
                        return [
                            ...msgs.slice(0, -1),
                            { ...last, content: prev },
                        ];
                    }
                    // ถ้าไม่มีข้อความ assistant รองรับ ให้สร้างใหม่
                    return [...msgs, { id: crypto.randomUUID(), role: "assistant", content: prev }];
                });
                return ""; // เคลียร์ streaming text
            });
            // บังคับปิดสถานะ Loading และ Tool เสมอ
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
                    id: crypto.randomUUID(),
                    role: "assistant",
                    content: `❌ Error: ${event.payload as string}`,
                },
            ]);
        });

        return () => {
            unlistenActiveModel.then((f) => f());
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
    }, [projectDir, showSettings, isIndexing]);

    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [messages, streamingText, activeTools]);

    useEffect(() => {
        if (messages.length === 0 && !currentSessionId) return;

        let sid = currentSessionId;
        if (!sid) {
            sid = crypto.randomUUID();
            setCurrentSessionId(sid);
        }

        setSessions(prev => {
            const existing = prev.find(s => s.id === sid);
            const title = existing?.title || (messages.length > 0 ? messages[0].content.substring(0, 30) + "..." : "New Chat");

            const updated = prev.filter(s => s.id !== sid);
            if (messages.length === 0 && !existing) return prev; // don't save empty without user action

            const newSession = {
                id: sid,
                title,
                messages,
                updatedAt: Date.now()
            };

            const newSessions = [newSession, ...updated];
            localStorage.setItem("vibe_chat_sessions", JSON.stringify(newSessions));
            return newSessions;
        });
    }, [messages, currentSessionId]);

    const createNewChat = () => {
        setCurrentSessionId(crypto.randomUUID());
        setMessages([]);
        setShowHistory(false);
    };

    const loadSession = (id: string) => {
        const session = sessions.find(s => s.id === id);
        if (session) {
            setCurrentSessionId(id);
            setMessages(session.messages);
            setShowHistory(false);
        }
    };

    const clearChat = () => {
        if (currentSessionId) {
            setSessions(prev => {
                const updated = prev.filter(s => s.id !== currentSessionId);
                localStorage.setItem("vibe_chat_sessions", JSON.stringify(updated));
                return updated;
            });
        }
        setCurrentSessionId(crypto.randomUUID());
        setMessages([]);
    };

    const stopGeneration = () => {
        invoke("stop_ai_generation").catch(e => console.error(e));
    };

    const undoChanges = async (userMsgIndex: number) => {
        // Find the assistant message that follows this user message
        const assistantMsg = messages[userMsgIndex + 1];
        const messageId = assistantMsg?.id;
        try {
            if (messageId) {
                await invoke("undo_ai_changes", { messageId });
            }
        } catch (e) {
            // Graceful fallback: if no file backups exist, just log it.
            // We still revert the chat history below.
            console.warn("Undo file revert skipped (no backups):", e);
        }
        // Always truncate messages: remove this user message and everything after it
        setMessages(prev => prev.slice(0, userMsgIndex));
    };

    // Check if the AI response following a user message performed any write_file operations
    const assistantDidWriteFile = (userMsgIndex: number): boolean => {
        const assistantMsg = messages[userMsgIndex + 1];
        if (!assistantMsg || assistantMsg.role !== "assistant") return false;
        return assistantMsg.toolCalls?.some(tc => tc.name === "write_file") ?? false;
    };

    const reusePrompt = (content: string) => {
        setInput(content);
        // Focus the input field so the user can edit immediately
        setTimeout(() => {
            inputRef.current?.focus();
        }, 50);
    };

    const sendMessage = async (overrideInput?: string) => {
        const textToSend = overrideInput || input.trim();
        if (!textToSend || isLoading) return;

        // Guard: show settings if the active provider has no key configured
        const missingKey =
            provider === "openrouter"
                ? !openrouterApiKey
                : provider === "google"
                    ? !googleApiKey
                    : !api_key && !baseUrl.includes("localhost") && !baseUrl.includes("127.0.0.1");

        if (missingKey) {
            setShowSettings(true);
            return;
        }

        const messageId = crypto.randomUUID();
        const userMessage: Message = { id: crypto.randomUUID(), role: "user", content: textToSend };
        const newMessages = [...messages, userMessage];
        setMessages(newMessages);
        setInput("");
        setIsLoading(true);
        setStreamingText("");

        // Add empty assistant message placeholder
        setMessages((prev) => [...prev, { id: messageId, role: "assistant", content: "" }]);

        // Convert messages to the format the backend expects
        const apiMessages = newMessages.map((m) => {
            let content = m.content;

            // Inject hidden system warning only for the newly sent message
            if (m.id === userMessage.id) {
                if (projectDir === ".") {
                    content = `[CRITICAL SYSTEM ENFORCEMENT: NO WORKSPACE IS CURRENTLY OPEN! If the user asks to create a project from scratch and there is no active workspace, you MUST call 'create_project_workspace' FIRST. You are FORBIDDEN from using 'run_command' (e.g., mkdir) or 'write_file' to create initial folders. Wait for the tool to return the selected path before writing files.]\n\n${content}`;
                } else {
                    // แอบแนบสถานะโปรเจกต์ไปกับคำถามเสมอ
                    let systemContext = `[CURRENT PROJECT STATE: You are working in '${projectDir}'. `;
                    systemContext += `Always rely on explicitly declared variables. DO NOT invent macros.]\n\n`;

                    // 🛠 [เพิ่มส่วนนี้] กระตุ้น AI บังคับให้ใช้ Tool ทุกครั้งที่มีโปรเจกต์อยู่แล้ว
                    content = `${systemContext}${content}\n\n[CRITICAL REMINDER: If the user asks you to fix, check, or write code, you MUST use the \`read_file\` or \`write_file\` tool IMMEDIATELY. DO NOT just apologize or explain what you will do. Execute the tool NOW.]`;
                }
            }
            return {
                role: m.role,
                content: content,
            };
        });

        try {
            await invoke("send_ai_message", {
                messages: apiMessages,
                projectDir,
                messageId
            });
        } catch (err) {
            setIsLoading(false);
            setMessages((prev) => [
                ...prev.slice(0, -1), // Remove placeholder
                {
                    id: crypto.randomUUID(),
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
            await invoke("set_openrouter_api_key", { key: openrouterApiKeyInput });
            await invoke("set_openrouter_model", { model: openrouterModelInput });
            await invoke("set_google_api_key", { key: googleApiKeyInput });
            await invoke("set_google_model", { model: googleModelInput });

            setApiKey(apiKeyInput);
            setBaseUrl(baseUrlInput);
            setProvider(providerInput);
            setOpenrouterApiKey(openrouterApiKeyInput);
            setOpenrouterModel(openrouterModelInput);
            setGoogleApiKey(googleApiKeyInput);
            setGoogleModel(googleModelInput);

            setShowSettings(false);
        } catch (err) {
            console.error("Failed to save AI settings:", err);
        }
    };

    const handleProviderChange = (newProvider: "openai" | "local" | "openrouter" | "google") => {
        setProviderInput(newProvider);
        if (newProvider === "openai") {
            setBaseUrlInput("https://api.openai.com/v1");
            setModelInput("gpt-4o");
        } else if (newProvider === "local") {
            setBaseUrlInput("http://localhost:1234/v1");
            setModelInput("qwen2.5-coder-7b-instruct");
        } else {
            // openrouter — baseUrl is fixed, model comes from openrouterModelInput
            setBaseUrlInput("https://openrouter.ai/api/v1");
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
        let prevLine = ""; // track the line just before the ``` fence

        for (const line of lines) {
            if (line.startsWith("```")) {
                if (inCodeBlock) {
                    let targetFile: string | null = null;
                    const langMatch = codeLanguage.replace(/\[FILE:\s*.+?\]/i, '').trim();

                    // 1. Check language part e.g. ```c [FILE: main/main.c]
                    const fileMatchLang = codeLanguage.match(/\[FILE:\s*(.+?)\]/i);
                    // 2. Check the line BEFORE the fence, e.g. [FILE: main/main.c]
                    const fileMatchPrev = prevLine.match(/\[FILE:\s*(.+?)\]/i);
                    // 3. Check first line of code content e.g. // [FILE: main/main.c]
                    const fileMatchContent = codeContent.length > 0
                        ? codeContent[0].match(/\[FILE:\s*(.+?)\]/i)
                        : null;

                    if (fileMatchLang?.[1]) {
                        targetFile = fileMatchLang[1].trim();
                    } else if (fileMatchPrev?.[1]) {
                        targetFile = fileMatchPrev[1].trim();
                    } else if (fileMatchContent?.[1]) {
                        targetFile = fileMatchContent[1].trim();
                        // Remove the [FILE:...] line from actual code content
                        codeContent = codeContent.slice(1);
                    }

                    const currentTargetFile = targetFile;
                    const currentCodeContent = [...codeContent];

                    elements.push(
                        <div key={key++} className="my-2 rounded-lg border border-neutral-700/50 overflow-hidden relative group/code">
                            <div className="bg-neutral-800 px-3 py-1.5 text-[10px] text-neutral-400 font-mono flex justify-between items-center border-b border-neutral-700/50">
                                <span>{currentTargetFile ? <><span className="text-red-400 font-bold">{currentTargetFile}</span> <span className="uppercase opacity-50 ml-2">{langMatch}</span></> : <span className="uppercase">{langMatch || "code"}</span>}</span>
                                <ApplyButton
                                    targetFile={currentTargetFile ? currentTargetFile.split('/').pop() : null}
                                    onApply={() => {
                                        if (currentTargetFile && onApplyToFile) {
                                            onApplyToFile(currentTargetFile, currentCodeContent.join("\n"));
                                        } else {
                                            onInjectCode(currentCodeContent.join("\n"));
                                        }
                                    }}
                                />
                            </div>
                            <pre className="bg-neutral-900/80 p-3 overflow-x-auto text-xs">
                                <code>{currentCodeContent.join("\n")}</code>
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
                prevLine = line;
            } else if (inCodeBlock) {
                codeContent.push(line);
            } else if (line.startsWith("### ")) {
                prevLine = line;
                elements.push(
                    <h4 key={key++} className="font-bold text-sm mt-3 mb-1 text-red-300">
                        {line.slice(4)}
                    </h4>
                );
            } else if (line.startsWith("## ")) {
                prevLine = line;
                elements.push(
                    <h3 key={key++} className="font-bold text-base mt-3 mb-1 text-red-300">
                        {line.slice(3)}
                    </h3>
                );
            } else if (line.startsWith("# ")) {
                prevLine = line;
                elements.push(
                    <h2 key={key++} className="font-bold text-lg mt-3 mb-1 text-red-300">
                        {line.slice(2)}
                    </h2>
                );
            } else if (line.startsWith("- ") || line.startsWith("* ")) {
                prevLine = line;
                elements.push(
                    <div key={key++} className="flex gap-2 ml-2">
                        <span className="text-red-500">•</span>
                        <span>{renderInlineCode(line.slice(2))}</span>
                    </div>
                );
            } else if (line.match(/^\d+\. /)) {
                prevLine = line;
                const num = line.match(/^(\d+)\. /)?.[1];
                elements.push(
                    <div key={key++} className="flex gap-2 ml-2">
                        <span className="text-red-500 font-mono text-xs min-w-[1.2em]">{num}.</span>
                        <span>{renderInlineCode(line.replace(/^\d+\. /, ""))}</span>
                    </div>
                );
            } else if (line.trim() === "") {
                prevLine = "";
                elements.push(<div key={key++} className="h-2" />);
            } else if (line.match(/^\[FILE:\s*.+?\]/i)) {
                // [FILE:] tag on its own line — render as a subtle label and track as prevLine
                prevLine = line;
                elements.push(
                    <div key={key++} className="text-[10px] text-red-500/70 font-mono mt-2 flex items-center gap-1">
                        <span>📄</span>{line}
                    </div>
                );
            } else {
                prevLine = line;
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
                        <div className="bg-neutral-700 px-3 py-1 text-[10px] text-neutral-400 font-mono uppercase">
                            {codeLanguage}
                        </div>
                    )}
                    <pre className="bg-neutral-800 p-3 overflow-x-auto text-xs">
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
                    <code key={i} className="bg-neutral-700 px-1.5 py-0.5 rounded text-red-300 text-xs font-mono">
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
        <div className="flex flex-col h-full bg-neutral-950">
            {/* Header */}
            <div className="h-10 border-b border-neutral-800 flex items-center justify-between px-4 bg-neutral-900/80 backdrop-blur-sm shrink-0">
                <div className="flex items-center gap-2">
                    <div className="w-5 h-5 rounded bg-gradient-to-br from-violet-500 to-rose-600 flex items-center justify-center text-[10px] font-bold text-white shadow-lg shadow-violet-500/20">
                        AI
                    </div>
                    <span className="text-xs font-bold text-neutral-300 tracking-wide">
                        Vibe Coder
                    </span>
                    {activeModelBadge ? (
                        <span className="text-[9px] px-1.5 py-0.5 rounded font-bold uppercase bg-red-500/10 text-red-400 border border-red-500/20">
                            {activeModelBadge}
                        </span>
                    ) : (
                        <span className={`text-[8px] px-1.5 py-0.5 rounded font-bold uppercase ${provider === "openai" ? "bg-violet-500/10 text-violet-400" :
                            provider === "openrouter" ? "bg-orange-500/10 text-orange-400" :
                                provider === "google" ? "bg-red-500/10 text-red-400" :
                                    "bg-emerald-500/10 text-emerald-400"
                            }`}>
                            {provider === "openai" ? "Cloud" : provider === "openrouter" ? "OpenRouter" : provider === "google" ? "Google AI" : "Local"}
                        </span>
                    )}
                </div>
                <div className="flex items-center gap-2">
                    <button
                        onClick={createNewChat}
                        className="text-neutral-500 hover:text-emerald-400 transition-colors"
                        title="➕ แชทใหม่"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                        </svg>
                    </button>
                    <button
                        onClick={() => setShowHistory(true)}
                        className="text-neutral-500 hover:text-red-400 transition-colors"
                        title="🕒 ประวัติแชท"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                    </button>
                    <button
                        onClick={clearChat}
                        className="text-neutral-500 hover:text-rose-400 transition-colors"
                        title="🗑️ ล้างข้อความ"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                    </button>
                    <div className="w-px h-4 bg-neutral-700 mx-1"></div>
                    <button
                        onClick={() => setShowSettings(true)}
                        className="text-neutral-500 hover:text-violet-400 transition-colors"
                        title="Settings"
                    >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                        </svg>
                    </button>
                </div>
            </div>

            {/* History Modal */}
            {showHistory && (
                <div className="absolute inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50" onClick={(e) => { if (e.target === e.currentTarget) setShowHistory(false) }}>
                    <div className="bg-neutral-800 border border-neutral-700 rounded-xl p-6 w-96 shadow-2xl overflow-y-auto max-h-[90vh]">
                        <div className="flex items-center justify-between mb-4">
                            <h3 className="text-sm font-bold text-neutral-200">ประวัติแชท (Chat History)</h3>
                            <button onClick={() => setShowHistory(false)} className="text-neutral-500 hover:text-neutral-300">
                                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
                            </button>
                        </div>
                        <div className="space-y-2">
                            {sessions.length === 0 ? (
                                <p className="text-xs text-neutral-500 text-center py-4">ไม่มีประวัติการแชท</p>
                            ) : (
                                sessions.sort((a, b) => b.updatedAt - a.updatedAt).map(session => (
                                    <button
                                        key={session.id}
                                        onClick={() => loadSession(session.id)}
                                        className={`w-full text-left p-3 rounded-lg border transition-colors ${currentSessionId === session.id
                                                ? "bg-violet-600/20 border-violet-500/50 text-violet-300"
                                                : "bg-neutral-900/50 border-neutral-700 text-neutral-400 hover:bg-neutral-700 hover:text-neutral-200"
                                            }`}
                                    >
                                        <div className="font-medium text-sm truncate">{session.title}</div>
                                        <div className="text-[10px] opacity-60 mt-1">
                                            {new Date(session.updatedAt).toLocaleString("th-TH")} • {session.messages.length} messages
                                        </div>
                                    </button>
                                ))
                            )}
                        </div>
                    </div>
                </div>
            )}

            {/* Settings Modal */}
            {showSettings && (
                <div className="absolute inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
                    <div className="bg-neutral-800 border border-neutral-700 rounded-xl p-6 w-96 shadow-2xl overflow-y-auto max-h-[90vh]">
                        <h3 className="text-sm font-bold text-neutral-200 mb-4">
                            AI Provider Settings
                        </h3>

                        {/* Provider Switcher Tabs */}
                        <div className="flex bg-neutral-900 p-1 rounded-lg mb-6 border border-neutral-700 gap-1">
                            <button
                                onClick={() => handleProviderChange("openai")}
                                className={`flex-1 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${providerInput === "openai" ? "bg-violet-600 text-white shadow-lg" : "text-neutral-500 hover:text-neutral-300"}`}
                            >
                                Cloud (OpenAI)
                            </button>
                            <button
                                onClick={() => handleProviderChange("openrouter")}
                                className={`flex-1 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${providerInput === "openrouter" ? "bg-orange-500 text-white shadow-lg shadow-orange-500/20" : "text-neutral-500 hover:text-neutral-300"}`}
                            >
                                OpenRouter
                            </button>
                            <button
                                onClick={() => handleProviderChange("local")}
                                className={`flex-1 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${providerInput === "local" ? "bg-emerald-600 text-white shadow-lg" : "text-neutral-500 hover:text-neutral-300"}`}
                            >
                                Local
                            </button>
                            <button
                                onClick={() => handleProviderChange("google")}
                                className={`flex-1 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${providerInput === "google" ? "bg-red-600 text-white shadow-lg shadow-red-500/20" : "text-neutral-500 hover:text-neutral-300"}`}
                            >
                                Google
                            </button>
                        </div>

                        <div className="space-y-4 mb-6">
                            {providerInput === "openai" && (
                                <div className="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                                    <div>
                                        <label className="text-xs text-neutral-400 mb-1 block">
                                            OpenAI API Key
                                        </label>
                                        <input
                                            type="password"
                                            value={apiKeyInput}
                                            onChange={(e) => setApiKeyInput(e.target.value)}
                                            placeholder="sk-..."
                                            className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-violet-500 transition-colors"
                                        />
                                    </div>
                                    <div>
                                        <label className="text-xs text-neutral-400 mb-2 block font-bold uppercase tracking-wider">
                                            Cloud Model
                                        </label>
                                        <select
                                            value={modelInput}
                                            onChange={(e) => setModelInput(e.target.value)}
                                            className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-violet-500 transition-colors appearance-none cursor-pointer"
                                            style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E")`, backgroundRepeat: "no-repeat", backgroundPosition: "right 12px center" }}
                                        >
                                            <option value="gpt-4o">GPT-4o (Standard)</option>
                                            <option value="gpt-4o-mini">GPT-4o Mini (Fast)</option>
                                            <option value="o1-preview">o1-preview (Reasoning)</option>
                                            <option value="o3-mini">o3-mini (Advanced Reasoning Fast)</option>
                                        </select>
                                        <p className="text-[10px] text-neutral-500 mt-1.5 font-mono truncate">
                                            ID: {modelInput}
                                        </p>
                                    </div>
                                </div>
                            )}

                            {providerInput === "local" && (
                                <div className="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                                    <div>
                                        <label className="text-xs text-neutral-400 mb-1 block">
                                            LM Studio Server URL
                                        </label>
                                        <input
                                            type="text"
                                            value={baseUrlInput}
                                            onChange={(e) => setBaseUrlInput(e.target.value)}
                                            placeholder="http://localhost:1234/v1"
                                            className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-emerald-500 transition-colors"
                                        />
                                        <p className="text-[10px] text-neutral-500 mt-1">Note: /v1 is required in LM Studio</p>
                                    </div>
                                    <div>
                                        <label className="text-xs text-neutral-400 mb-1 block">
                                            Local Model ID
                                        </label>
                                        <input
                                            type="text"
                                            value={modelInput}
                                            onChange={(e) => setModelInput(e.target.value)}
                                            placeholder="qwen2.5-coder-7b-instruct"
                                            className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-emerald-500 transition-colors"
                                        />
                                        <p className="text-[10px] text-amber-500 mt-1 font-bold">⚠️ Must match the "Model ID" in LM Studio</p>
                                    </div>
                                    <div>
                                        <label className="text-xs text-neutral-400 mb-1 block">
                                            Local API Key (Keep empty if not needed)
                                        </label>
                                        <input
                                            type="password"
                                            value={apiKeyInput}
                                            onChange={(e) => setApiKeyInput(e.target.value)}
                                            placeholder="not required for local"
                                            className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-emerald-500 transition-colors opacity-50"
                                        />
                                    </div>
                                </div>
                            )}
                        </div>

                        {providerInput === "openrouter" && (
                            <div className="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                                <div className="flex items-center gap-2 p-2 bg-orange-500/5 border border-orange-500/20 rounded-lg">
                                    <span className="text-orange-400 text-[10px]">🔀</span>
                                    <p className="text-[10px] text-orange-300/80">
                                        OpenRouter lets you access hundreds of AI models via a single API.
                                    </p>
                                </div>
                                <div>
                                    <label className="text-xs text-neutral-400 mb-1 block">
                                        OpenRouter API Key
                                    </label>
                                    <input
                                        type="password"
                                        value={openrouterApiKeyInput}
                                        onChange={(e) => setOpenrouterApiKeyInput(e.target.value)}
                                        placeholder="sk-or-..."
                                        className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-orange-500 transition-colors"
                                    />
                                    <p className="text-[10px] text-neutral-500 mt-1">
                                        Get your key at <span className="text-orange-400">openrouter.ai/keys</span>
                                    </p>
                                </div>
                                <div>
                                    <label className="text-xs text-neutral-400 mb-1.5 block font-bold uppercase tracking-wider">
                                        OpenRouter Model
                                    </label>
                                    <select
                                        value={openrouterModelInput}
                                        onChange={(e) => setOpenrouterModelInput(e.target.value)}
                                        className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-orange-500 transition-colors appearance-none cursor-pointer"
                                        style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E")`, backgroundRepeat: "no-repeat", backgroundPosition: "right 12px center" }}
                                    >
                                        <optgroup label="🆓 Free — Best for Coding">
                                            <option value="qwen/qwen3-coder:free">⭐ Qwen 3 Coder 480B (Best Free Coder)</option>
                                            <option value="meta-llama/llama-3.3-70b-instruct:free">Llama 3.3 70B Instruct (Free)</option>
                                            <option value="nousresearch/hermes-3-llama-3.1-405b:free">Hermes 3 405B Instruct (Free)</option>
                                            <option value="qwen/qwen3-next-80b-a3b-instruct:free">Qwen 3 Next 80B Instruct (Free)</option>
                                            <option value="nvidia/nemotron-3-super-120b-a12b:free">Nemotron 3 Super 120B (Free)</option>
                                            <option value="openai/gpt-oss-120b:free">GPT-OSS 120B (Free)</option>
                                            <option value="google/gemma-3-27b-it:free">Gemma 3 27B (Free)</option>
                                            <option value="deepseek/deepseek-chat:free">DeepSeek V3 Chat (Free)</option>
                                            <option value="deepseek/deepseek-r1:free">DeepSeek R1 (Free)</option>
                                        </optgroup>
                                        <optgroup label="🆓 Free — Auto">
                                            <option value="openrouter/free">Auto Free (Smart Fallback)</option>
                                        </optgroup>
                                        <optgroup label="✦ Premium — Top Models">
                                            <option value="anthropic/claude-4.6-sonnet-20260217">Claude Sonnet 4.6 (🏆 #1 Week)</option>
                                            <option value="anthropic/claude-4.6-opus-20260205">Claude Opus 4.6 (Most Powerful)</option>
                                            <option value="deepseek/deepseek-v3.2-20251201">DeepSeek V3.2 (Fast &amp; Cheap)</option>
                                            <option value="google/gemini-3-flash-preview-20251217">Gemini 3 Flash Preview</option>
                                            <option value="moonshotai/kimi-k2.5-0127">Kimi K2.5 (Long Context)</option>
                                            <option value="openai/gpt-4o">GPT-4o</option>
                                            <option value="openai/gpt-4o-mini">GPT-4o Mini</option>
                                            <option value="deepseek/deepseek-r1">DeepSeek R1 (Full)</option>
                                        </optgroup>
                                    </select>
                                    <p className="text-[10px] text-neutral-500 mt-1.5 font-mono truncate">
                                        ID: {openrouterModelInput}
                                    </p>
                                </div>
                            </div>
                        )}

                        {providerInput === "google" && (
                            <div className="space-y-4 animate-in fade-in slide-in-from-top-1 duration-200">
                                <div className="flex items-center gap-2 p-2 bg-red-500/5 border border-red-500/20 rounded-lg">
                                    <span className="text-red-400 text-[10px]">💡</span>
                                    <p className="text-[10px] text-red-300/80">
                                        We link directly to Google AI Studio.
                                        Rate limits apply to free-tier accounts.
                                    </p>
                                </div>
                                <div>
                                    <label className="text-xs text-neutral-400 mb-1 block flex justify-between">
                                        <span>Google AI API Key</span>
                                        <a href="https://aistudio.google.com/app/apikey" target="_blank" rel="noreferrer" className="text-[10px] text-red-400 hover:underline">
                                            Get Free Key ↗
                                        </a>
                                    </label>
                                    <input
                                        type="password"
                                        value={googleApiKeyInput}
                                        onChange={(e) => setGoogleApiKeyInput(e.target.value)}
                                        placeholder="AIzaSy..."
                                        className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-red-500 transition-colors"
                                    />
                                </div>
                                <div>
                                    <label className="text-xs text-neutral-400 mb-1.5 block font-bold uppercase tracking-wider">
                                        Gemini Model
                                    </label>
                                    <select
                                        value={googleModelInput}
                                        onChange={(e) => setGoogleModelInput(e.target.value)}
                                        className="w-full bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-sm text-neutral-200 focus:outline-none focus:border-red-500 transition-colors appearance-none cursor-pointer"
                                        style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M6 9l6 6 6-6'/%3E%3C/svg%3E")`, backgroundRepeat: "no-repeat", backgroundPosition: "right 12px center" }}
                                    >
                                        <optgroup label="🔥 Gemini 3.1 Series (Latest Preview)">
                                            <option value="gemini-3.1-pro-preview">Gemini 3.1 Pro (Best Reasoning &amp; Coding)</option>
                                            <option value="gemini-3.1-flash-lite-preview">Gemini 3.1 Flash-Lite (Fastest &amp; Cheapest)</option>
                                            <option value="gemini-3.1-flash-live-preview">Gemini 3.1 Flash Live (Real-time A2A)</option>
                                        </optgroup>
                                        <optgroup label="⚡ Gemini 3 Flash (Preview)">
                                            <option value="gemini-3-flash-preview-20251217">Gemini 3 Flash Preview (Frontier-class)</option>
                                        </optgroup>
                                        <optgroup label="✦ Gemini 2.5 Series (Stable — Recommended)">
                                            <option value="gemini-2.5-pro">Gemini 2.5 Pro (Best Stable)</option>
                                            <option value="gemini-2.5-flash">⭐ Gemini 2.5 Flash (Best Price/Perf)</option>
                                            <option value="gemini-2.5-flash-lite">Gemini 2.5 Flash-Lite (Budget)</option>
                                        </optgroup>
                                        <optgroup label="⚠️ Gemini 2.0 (Deprecated)">
                                            <option value="gemini-2.0-flash">Gemini 2.0 Flash (Deprecated)</option>
                                        </optgroup>
                                    </select>
                                    <p className="text-[10px] text-neutral-500 mt-1.5 font-mono truncate">
                                        ID: {googleModelInput}
                                    </p>
                                </div>
                            </div>
                        )}

                        <div className="border-t border-neutral-700 pt-4 mb-6">
                            <label className="text-[10px] font-bold text-neutral-500 uppercase tracking-widest mb-3 block">
                                Extra Capabilities
                            </label>
                            <div className="bg-neutral-900/50 border border-neutral-700 rounded-lg p-3">
                                <div className="flex items-center gap-2 mb-1">
                                    <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
                                    <span className="text-xs font-bold text-neutral-300">Web Search Enabled</span>
                                </div>
                                <p className="text-[10px] text-neutral-500">
                                    AI will automatically search DuckDuckGo for documentation and technical info. No API key required.
                                </p>
                            </div>

                            <div className="mt-4">
                                <label className="text-[10px] font-bold text-neutral-500 uppercase tracking-widest mb-2 block flex justify-between items-center">
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
                                            className={`text-[10px] flex items-center gap-1 ${isIndexing ? 'text-neutral-500' : 'text-emerald-400 hover:underline'}`}
                                        >
                                            <svg className={`w-2.5 h-2.5 ${isIndexing ? 'animate-spin' : ''}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                            </svg>
                                            {isIndexing ? 'Indexing...' : 'Re-index'}
                                        </button>
                                        <button
                                            onClick={() => invoke("add_knowledge_base_files", { projectDir }).then(() => invoke("get_knowledge_base_files", { projectDir }).then(f => setKnowledgeFiles(f as string[]))).catch(err => console.error("Error adding file:", err))}
                                            className="text-[10px] text-red-400 hover:underline flex items-center gap-1"
                                        >
                                            <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                                            </svg>
                                            Add Files
                                        </button>
                                        <button
                                            onClick={() => invoke("open_knowledge_base_folder", { projectDir }).then(() => invoke("get_knowledge_base_files", { projectDir }).then(f => setKnowledgeFiles(f as string[])))}
                                            className="text-[10px] text-violet-400 hover:underline flex items-center gap-1"
                                        >
                                            <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                                            </svg>
                                            Open
                                        </button>
                                    </div>
                                </label>
                                <div className="space-y-2">
                                    <div className="flex flex-wrap gap-1.5 min-h-[40px] p-2 bg-neutral-900/50 border border-neutral-700 rounded-lg">
                                        {knowledgeFiles.length === 0 ? (
                                            <span className="text-[10px] text-neutral-600 italic">No custom docs added...</span>
                                        ) : (
                                            knowledgeFiles.map(file => {
                                                const isEnabled = !file.endsWith('.disabled');
                                                const displayFileName = file.replace('.disabled', '');
                                                return (
                                                    <div key={file} className={`flex items-center gap-1.5 px-2 py-0.5 border rounded text-[10px] transition-colors ${isEnabled ? 'bg-neutral-800 border-neutral-600 text-neutral-300' : 'bg-neutral-900 border-neutral-800 text-neutral-600'}`}>
                                                        {isEnabled ? (
                                                            <svg className="w-2.5 h-2.5 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                                            </svg>
                                                        ) : (
                                                            <svg className="w-2.5 h-2.5 text-neutral-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                                                            </svg>
                                                        )}
                                                        <span className={`truncate max-w-[150px] ${!isEnabled ? 'line-through opacity-70' : ''}`} title={file}>{displayFileName}</span>
                                                        <button
                                                            onClick={(e) => {
                                                                e.stopPropagation();
                                                                invoke("toggle_knowledge_base_file", { projectDir, fileName: file })
                                                                    .then(() => invoke("get_knowledge_base_files", { projectDir }))
                                                                    .then(f => setKnowledgeFiles(f as string[]))
                                                                    .catch(err => console.error("Failed to toggle file:", err));
                                                            }}
                                                            className={`p-0.5 rounded transition-colors ml-1 ${isEnabled ? 'text-amber-400/50 hover:text-amber-400' : 'text-emerald-400/50 hover:text-emerald-400'}`}
                                                            title={isEnabled ? "ซ่อนไฟล์จากแชท (Disable)" : "เปิดใช้งานในแชท (Enable)"}
                                                        >
                                                            {isEnabled ? (
                                                                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" /></svg>
                                                            ) : (
                                                                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" /></svg>
                                                            )}
                                                        </button>
                                                    </div>
                                                );
                                            })
                                        )}
                                    </div>
                                    <p className="text-[10px] text-neutral-500">Add .txt or .md files to `/knowledge_base` to give the AI project-specific context.</p>
                                </div>
                            </div>
                        </div>

                        <div className="flex gap-2">
                            <button
                                onClick={() => setShowSettings(false)}
                                className="flex-1 py-2 bg-neutral-700 hover:bg-neutral-600 text-sm text-neutral-300 rounded-lg transition-colors"
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
                        <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-violet-500/20 to-rose-600/20 border border-violet-500/20 flex items-center justify-center mb-3">
                            <span className="text-lg">✨</span>
                        </div>
                        <p className="text-sm text-neutral-500 font-medium">
                            Ask me anything about your ESP-IDF project
                        </p>
                        <p className="text-xs text-neutral-600 mt-1">
                            I can read, write files, and run commands
                        </p>
                    </div>
                )}

                {messages.map((msg, i) => (
                    <div
                        key={i}
                        className={`flex flex-col mb-2 group/msg ${msg.role === "user" ? "items-end" : "items-start"}`}
                    >
                        <div
                            className={`max-w-[90%] rounded-xl px-4 py-3 text-sm leading-relaxed ${msg.role === "user"
                                ? "bg-violet-600/80 text-white rounded-br-sm"
                                : "bg-neutral-800/80 text-neutral-200 rounded-bl-sm border border-neutral-700/50"
                                }`}
                        >
                            {msg.role === "assistant" ? (
                                <>
                                    {/* Tool call indicators */}
                                    {msg.toolCalls?.map((tc, j) => (
                                        <div
                                            key={j}
                                            className="flex items-center gap-2 text-xs text-neutral-400 mb-2 bg-neutral-700/50 rounded-lg px-2 py-1.5"
                                        >
                                            <span className="text-emerald-400">⚡</span>
                                            <span className="font-mono">{tc.name}</span>
                                            <span className="text-neutral-600">✓</span>
                                        </div>
                                    ))}
                                    <div className="prose-sm">{renderMarkdown(msg.content)}</div>
                                </>
                            ) : (
                                <div className="whitespace-pre-wrap">{msg.content}</div>
                            )}
                        </div>

                        {/* User Message Action Buttons — Reuse Prompt & Undo */}
                        {msg.role === "user" && !isLoading && (
                            <div className="flex items-center gap-1 mt-1.5 mr-1 opacity-0 group-hover/msg:opacity-100 transition-opacity duration-200">
                                <button
                                    onClick={() => reusePrompt(msg.content)}
                                    className="px-2 py-1 rounded-md hover:bg-violet-500/15 text-[10px] text-violet-300/70 hover:text-violet-300 transition-colors flex items-center gap-1.5"
                                    title="ใช้พรอมต์นี้อีกครั้ง"
                                >
                                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>
                                    <span>Reuse</span>
                                </button>
                                {/* Only show Undo when the AI response actually wrote files */}
                                {assistantDidWriteFile(i) && (
                                    <button
                                        onClick={() => undoChanges(i)}
                                        className="px-2 py-1 rounded-md hover:bg-rose-500/15 text-[10px] text-rose-300/70 hover:text-rose-300 transition-colors flex items-center gap-1.5"
                                        title="ย้อนคืนไฟล์ — กลับไปก่อนส่งข้อความนี้"
                                    >
                                        <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" /></svg>
                                        <span>Undo</span>
                                    </button>
                                )}
                            </div>
                        )}
                    </div>
                ))}

                {/* Streaming text */}
                {isLoading && streamingText && (
                    <div className="flex justify-start">
                        <div className="max-w-[90%] rounded-xl rounded-bl-sm px-4 py-3 bg-neutral-800/80 text-neutral-200 text-sm leading-relaxed border border-neutral-700/50">
                            <div className="prose-sm">{renderMarkdown(streamingText)}</div>
                            <span className="inline-block w-1.5 h-4 bg-violet-400 animate-pulse ml-0.5 align-middle" />
                        </div>
                    </div>
                )}

                {/* Active tool indicators */}
                {activeTools.length > 0 && (
                    <div className="flex justify-start">
                        <div className="rounded-xl px-4 py-2 bg-neutral-800/50 border border-neutral-700/50 text-xs text-neutral-400 flex items-center gap-2">
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
                        <div className="rounded-xl px-4 py-3 bg-neutral-800/50 border border-neutral-700/50">
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
            <div className="p-3 border-t border-neutral-800 bg-neutral-900/60 shrink-0">
                <div className="flex gap-2 items-end">
                    <textarea
                        ref={inputRef}
                        value={input}
                        onChange={(e) => setInput(e.target.value)}
                        onKeyDown={handleKeyDown}
                        placeholder="Ask about your code..."
                        rows={1}
                        className="flex-1 bg-neutral-800 border border-neutral-700 rounded-lg px-3 py-2 text-sm text-neutral-200 resize-none focus:outline-none focus:border-violet-500 transition-colors placeholder:text-neutral-600 max-h-32 overflow-y-auto"
                        style={{ minHeight: "36px" }}
                    />
                    {isLoading ? (
                        <button
                            onClick={stopGeneration}
                            className="p-2 rounded-lg transition-all duration-200 bg-rose-600 hover:bg-rose-500 text-white shadow-lg shadow-rose-500/20 active:scale-95"
                            title="⏹️ หยุดการทำงาน"
                        >
                            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                                <rect x="6" y="6" width="12" height="12" rx="2" />
                            </svg>
                        </button>
                    ) : (
                        <button
                            onClick={() => sendMessage()}
                            disabled={!input.trim()}
                            className={`p-2 rounded-lg transition-all duration-200 ${!input.trim()
                                ? "bg-neutral-700 text-neutral-500 cursor-not-allowed"
                                : "bg-violet-600 hover:bg-violet-500 text-white shadow-lg shadow-violet-500/20 active:scale-95"
                                }`}
                        >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
                            </svg>
                        </button>
                    )}
                </div>
                <p className="text-[10px] text-neutral-600 mt-1 ml-1">
                    Enter to send · Shift+Enter for new line
                </p>
            </div>
        </div>
    );
}

export default AiChat;
