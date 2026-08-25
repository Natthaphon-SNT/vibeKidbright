import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import AiChat from "./AiChat";
import CodeEditor from "./CodeEditor";
import ToolchainSetup from "./ToolchainSetup";
import WikiView from "./WikiView";
import { parseErrorLine, type ParsedBuildError } from "./errorHints";
import BuildErrorList from "./BuildErrorList";


interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

// ── File Icon SVGs by type ──────────────────────────────────────────────────
function FileIcon({ name }: { name: string }) {
  const lower = name.toLowerCase();

  if (lower.endsWith(".c")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#3b82f6" fillOpacity="0.15" />
        <text x="12" y="16" textAnchor="middle" fill="#60a5fa" fontSize="12" fontWeight="700" fontFamily="monospace">C</text>
      </svg>
    );
  }
  if (lower.endsWith(".h")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#a78bfa" fillOpacity="0.15" />
        <text x="12" y="16" textAnchor="middle" fill="#a78bfa" fontSize="12" fontWeight="700" fontFamily="monospace">H</text>
      </svg>
    );
  }
  if (lower.endsWith(".py")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#fbbf24" fillOpacity="0.15" />
        <text x="12" y="16" textAnchor="middle" fill="#fbbf24" fontSize="11" fontWeight="700" fontFamily="monospace">Py</text>
      </svg>
    );
  }
  if (lower.endsWith(".json")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#fbbf24" fillOpacity="0.12" />
        <text x="12" y="16" textAnchor="middle" fill="#f59e0b" fontSize="8" fontWeight="700" fontFamily="monospace">{'{}'}</text>
      </svg>
    );
  }
  if (lower.endsWith(".md") || lower.endsWith(".markdown")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#38bdf8" fillOpacity="0.12" />
        <text x="12" y="16" textAnchor="middle" fill="#38bdf8" fontSize="10" fontWeight="700" fontFamily="monospace">M</text>
      </svg>
    );
  }
  if (lower.includes("cmakelists") || lower.endsWith(".cmake")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#f43f5e" fillOpacity="0.12" />
        <path d="M8 8l4 4-4 4M13 16h4" stroke="#f43f5e" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }
  if (lower.startsWith("sdkconfig") || lower.endsWith(".cfg") || lower.endsWith(".ini") || lower.endsWith(".conf")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#a3e635" fillOpacity="0.1" />
        <path d="M12 15a3 3 0 100-6 3 3 0 000 6z" stroke="#a3e635" strokeWidth="1.5" />
        <path d="M12 4v2m0 12v2m-8-8h2m12 0h2m-3.5-5.5l-1.4 1.4m-5.2 5.2l-1.4 1.4m0-8l1.4 1.4m5.2 5.2l1.4 1.4" stroke="#a3e635" strokeWidth="1.5" strokeLinecap="round" />
      </svg>
    );
  }
  if (lower.endsWith(".rs")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#fb923c" fillOpacity="0.12" />
        <text x="12" y="16" textAnchor="middle" fill="#fb923c" fontSize="11" fontWeight="700" fontFamily="monospace">Rs</text>
      </svg>
    );
  }
  if (lower.endsWith(".toml") || lower.endsWith(".yml") || lower.endsWith(".yaml")) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <rect x="2" y="2" width="20" height="20" rx="3" fill="#94a3b8" fillOpacity="0.1" />
        <path d="M7 8h10M7 12h7M7 16h10" stroke="#64748b" strokeWidth="1.5" strokeLinecap="round" />
      </svg>
    );
  }
  // Default generic file
  return (
    <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
      <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z" stroke="#475569" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M14 2v6h6" stroke="#475569" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function FolderIcon({ isOpen }: { isOpen: boolean }) {
  if (isOpen) {
    return (
      <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
        <path d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h6a2 2 0 012 2v1" stroke="#38bdf8" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        <path d="M5 19h14a2 2 0 002-2l-2-7H5l-2 7a2 2 0 002 2z" fill="#38bdf8" fillOpacity="0.12" stroke="#38bdf8" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    );
  }
  return (
    <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none">
      <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" fill="#38bdf8" fillOpacity="0.08" stroke="#475569" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function ChevronIcon({ isOpen }: { isOpen: boolean }) {
  return (
    <svg
      className={`w-3 h-3 shrink-0 text-neutral-500 transition-transform duration-200 ${isOpen ? "rotate-90" : ""}`}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M9 6l6 6-6 6" />
    </svg>
  );
}

function FileTreeItem({
  item,
  activeFile,
  openFolders,
  onFileClick,
  onFolderToggle,
  onContextMenu,
  inlineAction,
  inlineInputValue,
  setInlineInputValue,
  onInlineInputSubmit,
  onInlineInputCancel,
  depth = 0
}: {
  item: FileEntry;
  activeFile: string;
  openFolders: Set<string>;
  onFileClick: (path: string) => void;
  onFolderToggle: (path: string) => void;
  onContextMenu: (e: React.MouseEvent, path: string, isDir: boolean) => void;
  inlineAction: { mode: "createFile" | "createDir" | "rename"; path: string } | null;
  inlineInputValue: string;
  setInlineInputValue: (val: string) => void;
  onInlineInputSubmit: () => void;
  onInlineInputCancel: () => void;
  depth?: number;
}) {
  const isOpen = openFolders.has(item.path);
  const isActive = activeFile === item.path;
  const indent = depth * 12;

  const isRenaming = inlineAction?.mode === "rename" && inlineAction.path === item.path;
  const isCreatingInside = (inlineAction?.mode === "createFile" || inlineAction?.mode === "createDir") && inlineAction.path === item.path;

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") onInlineInputSubmit();
    if (e.key === "Escape") onInlineInputCancel();
  };

  const renderInput = (iconName: string, isDir: boolean) => (
    <div className="w-full text-left py-[5px] px-2 flex items-center gap-1.5 rounded-md relative text-neutral-200" style={{ paddingLeft: `${(isDir ? 8 : 20) + indent}px` }}>
      {isDir ? <FolderIcon isOpen={false} /> : <FileIcon name={iconName} />}
      <input
        autoFocus
        value={inlineInputValue}
        onChange={(e) => setInlineInputValue(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={onInlineInputCancel}
        className="flex-1 text-[12px] rounded px-1 outline-none relative z-10 w-full"
        style={{ backgroundColor: 'var(--bg-input)', color: 'var(--text-primary)', border: '1.5px solid var(--accent)' }}
      />
    </div>
  );

  if (item.is_dir) {
    return (
      <div>
        {isRenaming ? renderInput(item.name, true) : (
          <button
            onClick={() => onFolderToggle(item.path)}
            onContextMenu={(e) => onContextMenu(e, item.path, true)}
            className="w-full text-left py-[5px] px-2 flex items-center gap-1.5 text-[12px] transition-all duration-150 rounded-md group relative"
            style={{ color: 'var(--text-muted)', paddingLeft: `${8 + indent}px` }}
            onMouseEnter={e => { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; e.currentTarget.style.color = 'var(--text-primary)'; }}
            onMouseLeave={e => { e.currentTarget.style.backgroundColor = ''; e.currentTarget.style.color = 'var(--text-muted)'; }}
          >
            <ChevronIcon isOpen={isOpen} />
            <FolderIcon isOpen={isOpen} />
            <span className="truncate font-medium">{item.name}</span>
          </button>
        )}
        {(isOpen || isCreatingInside) && (
          <div className="relative">
            <div className="absolute top-0 bottom-0" style={{ left: `${16 + indent}px`, borderLeft: '1px solid var(--border-color)' }} />
            {isCreatingInside && renderInput(inlineAction?.mode === "createDir" ? "folder" : "new.txt", inlineAction?.mode === "createDir")}
            {item.children?.map((child) => (
              <FileTreeItem
                key={child.path}
                item={child}
                activeFile={activeFile}
                openFolders={openFolders}
                onFileClick={onFileClick}
                onFolderToggle={onFolderToggle}
                onContextMenu={onContextMenu}
                inlineAction={inlineAction}
                inlineInputValue={inlineInputValue}
                setInlineInputValue={setInlineInputValue}
                onInlineInputSubmit={onInlineInputSubmit}
                onInlineInputCancel={onInlineInputCancel}
                depth={depth + 1}
              />
            ))}
          </div>
        )}
      </div>
    );
  }

  return isRenaming ? renderInput(item.name, false) : (
    <button
      onClick={() => onFileClick(item.path)}
      onDoubleClick={() => onFileClick(item.path)}
      onContextMenu={(e) => onContextMenu(e, item.path, false)}
      className="w-full text-left py-[5px] px-2 flex items-center gap-1.5 text-[12px] transition-all duration-150 rounded-md group relative"
      style={{
        paddingLeft: `${20 + indent}px`,
        ...(isActive
          ? { backgroundColor: 'var(--bg-active)', color: 'var(--accent)' }
          : { color: 'var(--text-muted)' })
      }}
      onMouseEnter={e => { if (!isActive) { e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; e.currentTarget.style.color = 'var(--text-primary)'; } }}
      onMouseLeave={e => { if (!isActive) { e.currentTarget.style.backgroundColor = ''; e.currentTarget.style.color = 'var(--text-muted)'; } }}
    >
      {isActive && <div className="absolute left-0 top-1 bottom-1 w-[3px] rounded-full" style={{ backgroundColor: 'var(--accent)' }} />}
      <FileIcon name={item.name} />
      <span className="truncate">{item.name}</span>
    </button>
  );
}

interface FileTab {
  name: string;
  path: string;
  content: string;
  savedContent: string;
}

// ── AppShell: Toolchain gate wrapper ─────────────────────────────────
function AppShell() {
  const [toolchainReady, setToolchainReady] = React.useState(false);

  return (
    <>
      <App toolchainReady={toolchainReady} />
      {!toolchainReady && (
        <ToolchainSetup
          onReady={() => setToolchainReady(true)}
          mini={true}
        />
      )}
    </>
  );
}

function App({ toolchainReady = true }: { toolchainReady?: boolean }) {
  const [darkMode, setDarkMode] = React.useState(() => {
    return localStorage.getItem("vibe-theme") === "dark";
  });

  const toggleTheme = () => {
    setDarkMode(prev => {
      const next = !prev;
      localStorage.setItem("vibe-theme", next ? "dark" : "light");
      return next;
    });
  };

  React.useEffect(() => {
    document.documentElement.setAttribute("data-theme", darkMode ? "dark" : "light");
  }, [darkMode]);
  const [status, setStatus] = useState("Checking ESP-IDF...");
  const [isSettingUpEspIdf, setIsSettingUpEspIdf] = useState(false);
  const [espIdfSetupNote, setEspIdfSetupNote] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [terminalInput, setTerminalInput] = useState("");
  const [openFiles, setOpenFiles] = useState<FileTab[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string>("");
  const [showAiPanel, setShowAiPanel] = useState(() => {
    const saved = localStorage.getItem("vibe-ai-panel");
    return saved === null ? true : saved === "true"; // default: เปิดอยู่เสมอ
  });
  const [activeView, setActiveView] = useState<"editor" | "wiki">(() => {
    return (localStorage.getItem("vibe-active-view") as "editor" | "wiki") || "editor";
  });

  const [projectDir, setProjectDir] = useState(".");
  const [isBuilding, setIsBuilding] = useState(false);
  const [buildStep, setBuildStep] = useState(0);
  const [buildTotal, setBuildTotal] = useState(0);
  const [buildCurrentTask, setBuildCurrentTask] = useState("");
  const [buildResult, setBuildResult] = useState<"idle" | "building" | "success" | "failed">("idle");
  const [buildErrors, setBuildErrors] = useState<ParsedBuildError[]>([]);
  const [gotoLineRequest, setGotoLineRequest] = useState<{ path: string; line: number; column?: number; token: number } | null>(null);
  const [serialPorts, setSerialPorts] = useState<string[]>([]);
  const [selectedSerialPort, setSelectedSerialPort] = useState("");
  const [serialBaud, setSerialBaud] = useState("115200");
  const [isSerialConnected, setIsSerialConnected] = useState(false);
  const [showNewProjectModal, setShowNewProjectModal] = useState(false);
  const [newProjectName, setNewProjectName] = useState("my_esp_project");
  const [newProjectPath, setNewProjectPath] = useState("");
  const [projectFiles, setProjectFiles] = useState<FileEntry[]>([]);
  const [openFolders, setOpenFolders] = useState<Set<string>>(new Set());
  const [showSetupModal, setShowSetupModal] = useState(false);
  const [customIdfPath, setCustomIdfPath] = useState("");
  const [customToolsPath, setCustomToolsPath] = useState("");
  const [isSavingPaths, setIsSavingPaths] = useState(false);
  const [setupModalError, setSetupModalError] = useState("");

  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; path: string; isDir: boolean } | null>(null);
  const [inlineAction, setInlineAction] = useState<{ mode: "createFile" | "createDir" | "rename"; path: string } | null>(null);
  const [inlineInputValue, setInlineInputValue] = useState("");

  useEffect(() => {
    const handleClickOutside = () => setContextMenu(null);
    window.addEventListener("click", handleClickOutside);
    return () => window.removeEventListener("click", handleClickOutside);
  }, []);
  const scrollRef = useRef<HTMLDivElement>(null);
  const openFilesRef = useRef<FileTab[]>(openFiles);
  const parseBuildProgressRef = useRef<(msg: string) => void>(() => {});
  const aiChatSendRef = useRef<((text: string) => void) | null>(null);
  useEffect(() => { openFilesRef.current = openFiles; }, [openFiles]);

  const normPath = (p: string) => p.replace(/\\/g, '/').toLowerCase();
  const activeFile = openFiles.find(f => normPath(f.path) === normPath(activeFilePath));

  // Load saved custom paths on mount
  useEffect(() => {
    invoke("get_idf_custom_paths").then((paths: unknown) => {
      const p = paths as { idf_path: string; tools_path: string };
      if (p.idf_path) setCustomIdfPath(p.idf_path);
      if (p.tools_path) setCustomToolsPath(p.tools_path);
    }).catch(() => {});
  }, []);

  const handleSaveCustomPaths = async () => {
    setIsSavingPaths(true);
    setSetupModalError("");
    try {
      const result = await invoke("set_idf_custom_paths", {
        idfPath: customIdfPath,
        toolsPath: customToolsPath,
      });
      addLog(`✅ ${result}`);
      setShowSetupModal(false);
      await checkEnvironment();
    } catch (err) {
      setSetupModalError(String(err));
      addLog(`❌ Path validation failed: ${err}`);
    } finally {
      setIsSavingPaths(false);
    }
  };

  const handleClearCustomPaths = async () => {
    await invoke("clear_idf_custom_paths").catch(() => {});
    setCustomIdfPath("");
    setCustomToolsPath("");
    addLog("Custom paths cleared. Using auto-detection.");
  };

  const handlePickIdfPath = async () => {
    const path = await invoke("pick_directory").catch(() => null);
    if (path) setCustomIdfPath(path as string);
  };

  const handlePickToolsPath = async () => {
    const path = await invoke("pick_directory").catch(() => null);
    if (path) setCustomToolsPath(path as string);
  };

  // --- 🛠 COMMAND WRAPPER HACK FOR WINDOWS 🛠 ---
  const runIdfWrappedCommand = async (baseCmd: string, args: string[], cwd: string | null) => {
    let idfPathToUse = customIdfPath;
    
    if (!idfPathToUse) {
      try {
        const p: any = await invoke("get_idf_custom_paths");
        if (p && p.idf_path) idfPathToUse = p.idf_path;
      } catch (e) {}
    }

    if (idfPathToUse) {
      const cleanPath = idfPathToUse.replace(/\//g, '\\');
      const exportScript = `${cleanPath}\\export.bat`;
      
      // 🛠 แก้ไข: เอาเครื่องหมาย "" รอบๆ ${exportScript} ออก เพื่อไม่ให้ CMD งงกับตัวอักษร \"
      const fullCmd = `call ${exportScript} && ${baseCmd} ${args.join(" ")}`;
      
      addLog(`[IDF Wrapper] Injecting environment from: ${exportScript}`);
      
      return await invoke("run_shell_command", {
        cmd: "cmd.exe",
        args: ["/c", fullCmd],
        cwd
      });
    } else {
      return await invoke("run_shell_command", { cmd: baseCmd, args, cwd });
    }
  };

  // Mount-once: ตรวจสภาพและโหลด serial ports ครั้งเดียวเท่านั้น
  useEffect(() => {
    checkEnvironment();
    loadSerialPorts();
  }, []);

  useEffect(() => {
    const unlistenTerminal = listen("terminal-output", (event) => {
      const line = event.payload as string;
      parseBuildProgressRef.current(line);
      setLogs((prev) => [...prev, line]);
    });

    const unlistenFile = listen("file-modified", async (event) => {
      const { path } = JSON.parse(event.payload as string);
      const normPath = path.replace(/\\/g, '/');
      const isOpen = openFilesRef.current.find(
        f => f.path.replace(/\\/g, '/') === normPath
      );
      if (isOpen) {
        reloadFile(isOpen.path);
      }
      loadProjectFiles();
    });

    const unlistenForceDir = listen("force-project-dir", async (event) => {
      const newPath = event.payload as string;
      setProjectDir(newPath);
      setOpenFiles([]);
      setActiveFilePath("");
      addLog(`Switched project to: ${newPath}`);
    });

    const unlistenDiffPending = listen("ai-diff-pending", async (event) => {
      try {
        const data = typeof event.payload === 'string' ? JSON.parse(event.payload) : event.payload as any;
        const fullPath = String(data.fullPath || "");
        if (!fullPath) return;

        const norm = (p: string) => p.replace(/\\/g, '/').toLowerCase();
        const existing = openFilesRef.current.find(f => norm(f.path) === norm(fullPath));

        if (!existing) {
          try {
            const content = await invoke("read_project_file", { path: fullPath });
            const fileName = fullPath.split(/[\/\\]/).pop() || "file";
            const newTab: FileTab = { name: fileName, path: fullPath, content: content as string, savedContent: content as string };
            setOpenFiles(prev => [...prev, newTab]);
          } catch (e) {
            console.error("Failed to auto-open file for diff review:", e);
            return;
          }
        }
        setActiveFilePath(fullPath);
      } catch (e) {
        console.error("Error handling ai-diff-pending in App:", e);
      }
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        saveAllFiles();
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlistenTerminal.then((f) => f());
      unlistenFile.then((f) => f());
      unlistenForceDir.then((f) => f());
      unlistenDiffPending.then((f) => f());
      window.removeEventListener("keydown", handleKeyDown);
      invoke("stop_serial_monitor").catch(() => null);
    };
  }, [activeFilePath, openFiles]);

  const reloadFile = async (path: string) => {
    try {
      const content = await invoke("read_project_file", { path });
      setOpenFiles(prev => prev.map(f =>
        f.path === path ? { ...f, content: content as string, savedContent: content as string } : f
      ));
      addLog(`Auto-reloaded: ${path.split("/").pop()}`);
    } catch (err) {
      addLog(`❌ Failed to reload file: ${err}`);
    }
  };

  useEffect(() => {
    if (projectDir !== ".") {
      loadProjectFiles();
    }
  }, [projectDir]);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  const saveAllFiles = useCallback(async () => {
    if (openFiles.length === 0) return;
    try {
      let savedCount = 0;
      for (const file of openFiles) {
        await invoke("write_project_file", {
          path: file.path,
          content: file.content
        });
        savedCount++;
      }
      setOpenFiles(prev => prev.map(f => ({ ...f, savedContent: f.content })));
      addLog(`✨ Saved all ${savedCount} open files`);
    } catch (err) {
      addLog(`❌ Failed to save files: ${err}`);
    }
  }, [openFiles]);

  const handleSaveProjectAs = async () => {
    if (projectDir === ".") {
      addLog("❌ No project open to save.");
      return;
    }

    await saveAllFiles();

    try {
      addLog("Opening folder picker to save project as...");
      const result = await invoke("save_project_as", { sourceDir: projectDir });
      const [newPath, fileCount] = (result as string).split("|");

      addLog(`✨ Project successfully saved to ${newPath} (${fileCount} files copied)`);

      const switchProject = window.confirm(`Project saved successfully!\n\nDo you want to switch to the new project location?\n\n${newPath}`);

      if (switchProject) {
        setProjectDir(newPath);
        setOpenFiles([]);
        setActiveFilePath("");
        addLog(`Switched project to: ${newPath}`);
      }
    } catch (err) {
      addLog(`❌ Failed to save project: ${err}`);
      alert(`Save Project As failed:\n${err}`);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, path: string, isDir: boolean) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, path, isDir });
  };

  const handleDeleteItem = async (path: string, isDir: boolean) => {
    const name = path.split(/[\/\\]/).pop() || "";
    if (name === "CMakeLists.txt" || name === "sdkconfig") {
      const confirmCritical = window.confirm(`⚠️ WARNING: '${name}' is a critical ESP-IDF file.\n\nDeleting it may break your project.\n\nAre you absolutely sure you want to delete '${name}'?`);
      if (!confirmCritical) return;
    } else {
      const confirmDelete = window.confirm(`Are you sure you want to delete '${name}'?`);
      if (!confirmDelete) return;
    }

    try {
      if (isDir) {
        await invoke("delete_directory", { path });
      } else {
        await invoke("delete_file", { path });
      }
      addLog(`✨ Deleted: ${path}`);
      loadProjectFiles();
      if (openFiles.some(f => normPath(f.path) === normPath(path))) {
        setOpenFiles(prev => prev.filter(f => normPath(f.path) !== normPath(path)));
      }
      if (normPath(activeFilePath) === normPath(path)) setActiveFilePath("");
    } catch (err) {
      alert(`Failed to delete:\n${err}`);
    }
  };

  const handleInlineSubmit = async () => {
    if (!inlineAction || !inlineInputValue.trim()) {
      setInlineAction(null);
      return;
    }

    const value = inlineInputValue.trim();
    try {
      if (inlineAction.mode === "rename") {
        const oldName = inlineAction.path.split(/[\/\\]/).pop() || "";
        const newPath = inlineAction.path.substring(0, inlineAction.path.length - oldName.length) + value;
        await invoke("rename_item", { old_path: inlineAction.path, new_path: newPath });
        addLog(`✨ Renamed to ${value}`);
        setOpenFiles(prev => prev.map(f => normPath(f.path) === normPath(inlineAction.path) ? { ...f, path: newPath, name: value } : f));
        if (normPath(activeFilePath) === normPath(inlineAction.path)) setActiveFilePath(newPath);
      } else {
        const parentPath = inlineAction.path;
        const sep = parentPath.includes('\\') ? '\\' : '/';
        const newPath = parentPath + sep + value;
        
        if (inlineAction.mode === "createFile") {
          await invoke("safe_write_project_file", { path: newPath, content: "" })
            .catch(async () => await invoke("write_project_file", { path: newPath, content: "" }));
          addLog(`✨ Created file ${value}`);
          
          await loadProjectFiles();
          
          const newFile: FileTab = { name: value, path: newPath, content: "", savedContent: "" };
          setOpenFiles(prev => [...prev, newFile]);
          setActiveFilePath(newPath);
        } else {
          await invoke("create_directory", { path: newPath });
          addLog(`✨ Created directory ${value}`);
        }
        setOpenFolders(prev => new Set(prev).add(parentPath));
      }
    } catch (err) {
      alert(`Operation failed:\n${err}`);
    }

    setInlineAction(null);
    loadProjectFiles();
  };

  const handleInlineCancel = () => setInlineAction(null);

  const loadProjectFiles = async () => {
    try {
      const files = await invoke("list_project_files", { path: projectDir });
      setProjectFiles(files as FileEntry[]);
    } catch (err) {
      console.error("Failed to load project files:", err);
    }
  };

  const loadSerialPorts = async () => {
    try {
      const ports = await invoke("list_serial_ports");
      const list = ports as string[];
      
      setSelectedSerialPort(prev => {
        if (list.length === 0) return "";
        if (prev && list.includes(prev)) return prev;
        return list[0];
      });
      
      setSerialPorts(list);
    } catch (err) {
      addLog(`❌ Failed to list serial ports: ${err}`);
    }
  };

  const handleFileClick = async (path: string) => {
    const np = normPath(path);
    const existing = openFiles.find(f => normPath(f.path) === np);
    if (existing) {
      setActiveFilePath(existing.path);
      return;
    }

    try {
      const content = await invoke("read_project_file", { path });
      const newTab: FileTab = {
        name: path.split(/[\/\\]/).pop() || "unknown",
        path,
        content: content as string,
        savedContent: content as string
      };
      setOpenFiles(prev => [...prev, newTab]);
      setActiveFilePath(path);
      addLog(`Opened: ${newTab.name}`);
    } catch (err) {
      addLog(`❌ Failed to read file: ${err}`);
    }
  };

  const closeFile = (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const np = normPath(path);
    setOpenFiles(prev => {
      const next = prev.filter(f => normPath(f.path) !== np);
      if (normPath(activeFilePath) === np) {
        setActiveFilePath(next.length > 0 ? next[next.length - 1].path : "");
      }
      return next;
    });
  };

  const updateActiveFileContent = async (newContent: string) => {
    if (!activeFilePath) {
      console.warn("Attempted to update content but no file is active.");
      addLog("❌ Vibe Code: No active file selected. Please click on a file in the sidebar to open it first.");
      return;
    }
    const np = normPath(activeFilePath);
    setOpenFiles(prev => prev.map(f =>
      normPath(f.path) === np ? { ...f, content: newContent, savedContent: newContent } : f
    ));

    try {
      await invoke("write_project_file", {
        path: activeFilePath,
        content: newContent
      });
      const fileName = activeFilePath.split(/[\/\\]/).pop() || activeFilePath;
      addLog(`✨ Vibe Code Applied & Saved to: ${fileName}`);
    } catch (err) {
      addLog(`❌ Failed to save injected code: ${err}`);
    }
  };

  const isFileDirty = (file: FileTab): boolean => {
    return file.content !== file.savedContent;
  };

  const handleEditorChange = (newContent: string) => {
    if (!activeFilePath) return;
    const np = normPath(activeFilePath);
    setOpenFiles(prev => prev.map(f =>
      normPath(f.path) === np ? { ...f, content: newContent } : f
    ));
  };

  const handleApplyToFile = async (filePath: string, newContent: string) => {
    const normSlash = (p: string) => p.replace(/\\/g, '/');
    const comparePath = (p: string) => normSlash(p).toLowerCase();

    let absolutePath: string;
    if (filePath.startsWith('/') || filePath.match(/^[a-zA-Z]:[\\\/]/)) {
      absolutePath = normSlash(filePath);
    } else {
      absolutePath = normSlash(`${projectDir}/${filePath}`);
    }

    const confirmApply = window.confirm(`Apply AI code to: ${filePath}?`);
    if (!confirmApply) return;

    try {
      await invoke("safe_write_project_file", { path: absolutePath, content: newContent });
      addLog(`✨ Vibe Code check & overwrite OK: ${filePath}`);

      const existingFile = openFilesRef.current.find(
        (f) => comparePath(f.path) === comparePath(absolutePath)
      );

      if (existingFile) {
        setOpenFiles((prev) =>
          prev.map((f) =>
            comparePath(f.path) === comparePath(absolutePath) ? { ...f, content: newContent } : f
          )
        );
        setActiveFilePath(existingFile.path);
      } else {
        const fileName = filePath.split(/[\/\\]/).pop() || 'file';
        const newTab: FileTab = { name: fileName, path: absolutePath, content: newContent, savedContent: newContent };
        setOpenFiles((prev) => [...prev, newTab]);
        setActiveFilePath(absolutePath);
      }
      loadProjectFiles();
    } catch (err) {
      addLog(`❌ Failed to overwrite file: ${err}`);
      alert(`Safety check failed:\n${err}`);
    }
  };

  const handleOpenProject = async () => {
    try {
      const path = await invoke("pick_directory");
      if (!path) return;

      const isValid = await invoke("validate_idf_project", { path });
      if (isValid) {
        setProjectDir(path as string);
        setOpenFiles([]);
        setActiveFilePath("");
        addLog(`Project opened: ${path}`);
      } else {
        alert("Selected directory is not a valid ESP-IDF project (missing CMakeLists.txt)");
      }
    } catch (err) {
      addLog(`Error opening project: ${err}`);
    }
  };


  const toggleFolder = (path: string) => {
    setOpenFolders((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  /** ดาวน์โหลด toolchain จาก GitHub Release (tags: framework, toolchain) */
  const handleAutoInstallGithub = async () => {
    if (isSettingUpEspIdf) return;

    setIsSettingUpEspIdf(true);
    setEspIdfSetupNote("⬇️ Connecting to GitHub Release...");
    setStatus("Downloading toolchain from GitHub...");
    addLog("🚀 Starting GitHub toolchain download (frameworks + tools)...");

    // Subscribe to toolchain-progress events แสดงใน terminal
    const unlisten = await listen<{ stage: string; percent: number; message: string }>(
      "toolchain-progress",
      (event) => {
        const { stage, percent, message } = event.payload;
        addLog(`[${stage.toUpperCase()}] ${percent}% — ${message}`);
        setEspIdfSetupNote(`${percent}% — ${message}`);

        if (stage === "done") {
          setStatus("✅ Toolchain Ready");
          setEspIdfSetupNote("Toolchain installed successfully!");
          setIsSettingUpEspIdf(false);
        } else if (stage === "error" || stage === "cancelled") {
          setStatus("❌ Installation Failed");
          setIsSettingUpEspIdf(false);
        }
      }
    );

    try {
      const result = await invoke("download_toolchain", { url: null });
      addLog(`✅ ${result}`);
      await checkEnvironment();
    } catch (err) {
      addLog(`❌ GitHub toolchain download failed: ${err}`);
      setStatus("❌ Installation Failed");
      setEspIdfSetupNote(`Failed: ${err}`);
      setIsSettingUpEspIdf(false);
    } finally {
      unlisten();
    }
  };

  const checkEnvironment = async () => {
    try {
      const result = await invoke("check_esp_idf");
      setStatus(result as string);
      setEspIdfSetupNote("");
    } catch (_err) {
      // check_esp_idf ล้มเหลว — ตรวจว่า Happy Meal toolchain มีอยู่แล้วหรือเปล่า
      try {
        const tc = await invoke("check_toolchain") as { status: string; version: string | null };
        if (tc.status === "ready") {
          setStatus(`✅ Toolchain v${tc.version || ""} Ready`);
          setEspIdfSetupNote("");
          return; // มี toolchain แล้ว — ไม่ต้องติดตั้งอีก
        }
      } catch (_tc) { /* ignore */ }
      // ไม่มีทั้ง check_esp_idf และ toolchain — แจ้งเฟย ไม่ auto-run
      setStatus("ESP-IDF not found");
      setEspIdfSetupNote("⚠️ ESP-IDF not found. Use Setup / Repair ESP-IDF to install.");
      addLog("⚠️ ESP-IDF not found. Click \"Setup / Repair ESP-IDF\" to install.");
    }
  };

  const toggleSerialMonitor = async () => {
    if (isSerialConnected) {
      try {
        const result = await invoke("stop_serial_monitor");
        addLog(`${result}`);
        setIsSerialConnected(false);
      } catch (err) {
        addLog(`❌ Failed to stop serial monitor: ${err}`);
      }
      return;
    }

    if (!selectedSerialPort) {
      addLog("❌ No serial port selected");
      return;
    }

    try {
      const result = await invoke("start_serial_monitor", {
        port: selectedSerialPort,
        baudRate: Number(serialBaud) || 115200
      });
      addLog(`${result}`);
      setIsSerialConnected(true);
    } catch (err) {
      addLog(`❌ Failed to start serial monitor: ${err}`);
    }
  };

  const sendSerialText = async () => {
    if (!terminalInput.trim()) return;
    if (!isSerialConnected) {
      addLog("❌ Serial monitor is not connected");
      return;
    }

    try {
      let payload = terminalInput;
      if (payload.endsWith("\r\n")) {
      } else if (payload.endsWith("\n")) {
        payload = payload.slice(0, -1) + "\r\n";
      } else {
        payload += "\r\n";
      }
      
      await invoke("send_serial_input", { input: payload });
      addLog(`[SERIAL TX] ${terminalInput}`);
      setTerminalInput("");
    } catch (err) {
      addLog(`❌ Failed to send serial input: ${err}`);
    }
  };

  const parseBuildProgress = (msg: string) => {
    // Strip optional timestamp prefix added by addLog: "[3:41:18 PM] ..."
    const raw = msg.replace(/^\[\d{1,2}:\d{2}:\d{2}\s+[AP]M\]\s+/, "").trim();

    // 0. Collect friendly build errors from compiler / linker / esptool output
    const parsedErr = parseErrorLine(raw);
    if (parsedErr) {
      setBuildErrors((prev) => {
        const key = `${parsedErr.file ?? ""}:${parsedErr.line ?? 0}:${parsedErr.message}`;
        if (prev.some((e) => `${e.file ?? ""}:${e.line ?? 0}:${e.message}` === key)) return prev;
        if (prev.length >= 20) return prev;
        return [...prev, parsedErr];
      });
    }

    // 1. Parse ninja-style compilation progress: [N/M] Some Task Description
    const ninjaMatch = raw.match(/^\[(\d+)\/(\d+)\]\s+(.+)/);
    if (ninjaMatch) {
      const current = parseInt(ninjaMatch[1], 10);
      const total = parseInt(ninjaMatch[2], 10);
      const taskRaw = ninjaMatch[3].trim();

      let task = taskRaw;
      if (task.includes("Building C object")) task = "Building C object";
      else if (task.includes("Building CXX object")) task = "Building C++ object";
      else if (task.includes("Linking C static library")) task = "Linking static library";
      else if (task.includes("Linking CXX executable")) task = "Linking executable";
      else if (task.includes("Generating")) task = "Generating linker script";
      else if (task.includes("Performing build step")) task = "Building bootloader";
      else if (task.includes("Completed")) task = "Bootloader complete";
      else if (task.includes("No install step")) task = "Skipping install";
      else if (task.length > 40) task = task.substring(0, 40) + "...";

      setBuildStep(current);
      setBuildTotal(total);
      setBuildCurrentTask(task);
      return;
    }

    // 2. Parse esptool flash percentage: Writing at 0x00010000... (45 %)
    const flashPctMatch = raw.match(/Writing at 0x[0-9a-fA-F]+\.\.\.\s*\(([0-9]+)\s*%\)/i) || raw.match(/\(([0-9]+)\s*%\)/);
    if (flashPctMatch && (raw.includes("Writing at") || raw.includes("Wrote") || raw.includes("%"))) {
      const pct = Math.min(100, Math.max(0, parseInt(flashPctMatch[1], 10)));
      setBuildStep(pct);
      setBuildTotal(100);
      setBuildCurrentTask(`Flashing to board (${pct}%)`);
      return;
    }

    // 3. Detect flash / upload phase markers
    if (raw.includes("Connecting...") || raw.includes("Connecting..")) {
      setBuildStep(0);
      setBuildTotal(100);
      setBuildCurrentTask("Connecting to board...");
    } else if (raw.includes("Chip is") || raw.includes("Features:")) {
      setBuildCurrentTask("Chip detected, preparing flash...");
    } else if (raw.includes("Writing at 0x") || raw.startsWith("Compressed")) {
      setBuildCurrentTask("Flashing firmware...");
    } else if (raw.includes("Hash of data verified")) {
      setBuildStep(100);
      setBuildTotal(100);
      setBuildCurrentTask("Flash verified ✓");
    } else if (raw.includes("Hard resetting") || (raw.includes("Done") && raw.includes("resetting"))) {
      setBuildStep(100);
      setBuildTotal(100);
      setBuildCurrentTask("Resetting board ✓");
    } else if (raw.startsWith("FAILED:") || raw.includes("ninja failed with exit code")) {
      setBuildResult("failed");
      setBuildCurrentTask("Build failed ✗");
    } else if (raw.includes("--- Starting Build")) {
      setBuildResult("building");
      setBuildStep(0);
      setBuildTotal(0);
      setBuildErrors([]);
      setBuildCurrentTask("Initializing build...");
    }
  };

  const addLog = (msg: string) => {
    parseBuildProgress(msg);
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  };

  // Keep ref in sync so event listeners always have the latest parser
  parseBuildProgressRef.current = parseBuildProgress;

  const handleTerminalSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!terminalInput.trim()) return;

    if (isSerialConnected) {
      await sendSerialText();
      return;
    }

    const parts = terminalInput.trim().split(" ");
    const cmd = parts[0];
    const args = parts.slice(1);

    addLog(`> ${terminalInput}`);
    setTerminalInput("");

    try {
      // 🛠 ใช้ Wrapper ถ้าเป็นคำสั่งของ IDF
      if (["idf.py", "ninja", "cmake", "esptool.py"].includes(cmd)) {
        await runIdfWrappedCommand(cmd, args, projectDir === "." ? null : projectDir);
      } else {
        await invoke("run_shell_command", {
          cmd,
          args,
          cwd: projectDir === "." ? null : projectDir
        });
      }
    } catch (err) {
      addLog(`Error: ${err}`);
    }
  };


  const handleNewProject = () => {
    setShowNewProjectModal(true);
    setNewProjectName("my_esp_project");
    setNewProjectPath("");
  };

  const handlePickDirectory = async () => {
    try {
      const path = await invoke("pick_directory");
      if (path) {
        setNewProjectPath(path as string);
      }
    } catch (err) {
      addLog(`Directory picker error: ${err}`);
    }
  };

  const confirmCreateProject = async () => {
    if (!newProjectPath || !newProjectName) return;

    setShowNewProjectModal(false);
    addLog(`Attempting to create project '${newProjectName}' at ${newProjectPath}...`);

    try {
      const result = await invoke("create_idf_project", {
        path: newProjectPath,
        name: newProjectName
      });
      addLog(`Success: ${result}`);

      const fullPath = `${newProjectPath}/${newProjectName}`;
      setProjectDir(fullPath);
      addLog(`Active project set to: ${fullPath}`);
    } catch (err) {
      console.error("New project error:", err);
      addLog(`❌ ERROR: ${err}`);
      alert(`Failed to create project: ${err}`);
    }
  };

  const handleBuildFlash = async () => {
    if (isBuilding) return;
    
    if (projectDir === ".") {
      addLog("❌ Error: No project selected to build.");
      alert("Please open or create a project first from the sidebar.");
      return;
    }

    setIsBuilding(true);
    setBuildResult("building");
    setBuildStep(0);
    setBuildTotal(0);
    setBuildErrors([]);
    setBuildCurrentTask("Initializing...");
    addLog("--- Starting Build & Flash ---");

    try {
      // 🛠 ใช้ Wrapper เพื่อให้มันโหลด export.bat ก่อนสั่ง idf.py เสมอ
      const flashArgs = ["build", "flash"];
      if (selectedSerialPort) {
        flashArgs.push("-p", selectedSerialPort);
        addLog(`Using port: ${selectedSerialPort}`);
      } else {
        addLog("⚠️ No serial port selected — idf.py will use its default port");
      }
      await runIdfWrappedCommand("idf.py", flashArgs, projectDir);
    } catch (err) {
      addLog(`Build failed: ${err}`);
      setBuildResult("failed");
      setBuildCurrentTask("Build failed");
    } finally {
      setIsBuilding(false);
      // If we didn't explicitly set failed, mark as success
      setBuildResult(prev => prev === "building" ? "success" : prev);
      setBuildCurrentTask(prev => prev === "Initializing..." ? "" : prev);
    }
  };

  // Open the file from a build error (if needed) and jump to the offending line
  const jumpToError = async (err: ParsedBuildError) => {
    if (!err.file) return;
    const target = err.line ?? 1;
    const np = normPath(err.file);
    const existing = openFiles.find(f => normPath(f.path) === np);
    if (existing) {
      setActiveFilePath(existing.path);
      setGotoLineRequest({ path: existing.path, line: target, column: err.column, token: Date.now() });
      return;
    }
    try {
      await handleFileClick(err.file);
      setGotoLineRequest({ path: err.file, line: target, column: err.column, token: Date.now() });
    } catch {
      addLog(`❌ Failed to open ${err.file}`);
    }
  };

  // Send the collected build errors to Vibe Coder and ask for a simple fix
  const askAiToFixErrors = () => {
    if (buildErrors.length === 0) return;
    const errText = buildErrors.slice(0, 5)
      .map((e, i) => `${i + 1}. [${e.title}] ${e.file ? `${e.file}:${e.line ?? "?"}` : "(flash/hardware)"}\n   ${e.message}`)
      .join("\n");
    const prompt =
      `The Build & Flash of my ESP-IDF project just failed. Here are the detected problems:\n\n${errText}\n\n` +
      `Please explain each error in simple Thai first (one sentence each), then use read_file to look at the code ` +
      `and write_file to fix it. Show me exactly what you changed and why.`;
    setShowAiPanel(true);
    setTimeout(() => {
      aiChatSendRef.current?.(prompt);
    }, 200);
  };

  return (
    <div className="flex h-screen w-full overflow-hidden" style={{ backgroundColor: 'var(--bg-app)', color: 'var(--text-primary)' }}>
      {/* Sidebar */}
      <div className="w-64 flex flex-col font-sans" style={{ backgroundColor: 'var(--bg-sidebar)', borderRight: '1px solid var(--border-color)' }}>
        <div className="p-4" style={{ borderBottom: '1px solid var(--border-color)' }}>
          <div className="flex items-center justify-between">
            <h1 className="text-xl font-bold brand-gradient">vibeKidbright</h1>
            <button
              onClick={toggleTheme}
              className="theme-toggle"
              title={darkMode ? 'Switch to Light Mode' : 'Switch to Dark Mode'}
            >
              {darkMode ? '☀️' : '🌙'}
            </button>
          </div>
          <p className="text-xs mt-1 uppercase tracking-widest font-semibold" style={{ color: 'var(--text-muted)' }}>ESP-IDF IDE</p>
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          <div className="flex items-center justify-between p-2">
            <span className="text-sm font-medium" style={{ color: 'var(--text-muted)' }}>PROJECT</span>
            <div className="flex gap-1 items-center">
              <button
                onClick={() => { setInlineAction({ mode: "createFile", path: projectDir }); setInlineInputValue(""); }}
                className="transition-colors p-1 rounded" style={{ color: 'var(--text-muted)' }}
                title="New File in Root"
                onMouseEnter={e => (e.currentTarget.style.color = 'var(--accent)')}
                onMouseLeave={e => (e.currentTarget.style.color = 'var(--text-muted)')}
              >
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z"/><path d="M14 2v6h6"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/></svg>
              </button>
              <button
                onClick={() => { setInlineAction({ mode: "createDir", path: projectDir }); setInlineInputValue(""); }}
                className="transition-colors p-1 rounded mr-2" style={{ color: 'var(--text-muted)' }}
                title="New Folder in Root"
                onMouseEnter={e => (e.currentTarget.style.color = 'var(--accent)')}
                onMouseLeave={e => (e.currentTarget.style.color = 'var(--text-muted)')}
              >
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h6a2 2 0 012 2v1"/><path d="M5 19h14a2 2 0 002-2l-2-7H5l-2 7a2 2 0 002 2z"/><line x1="12" y1="16" x2="12" y2="10"/><line x1="9" y1="13" x2="15" y2="13"/></svg>
              </button>
              <button
                onClick={handleOpenProject}
                className="text-[10px] px-1.5 py-0.5 rounded transition-colors font-bold"
                style={{ background: 'var(--pms-293-pale)', color: 'var(--accent)', border: '1px solid var(--accent)' }}
              >
                OPEN
              </button>
              <button
                onClick={handleNewProject}
                className="text-[10px] px-1.5 py-0.5 rounded transition-colors"
                style={{ background: 'var(--bg-hover)', color: 'var(--text-secondary)' }}
              >
                NEW
              </button>
            </div>
          </div>
          <div className="space-y-1">
            {(inlineAction?.mode === "createFile" || inlineAction?.mode === "createDir") && inlineAction.path === projectDir && (
              <div className="w-full text-left py-[5px] px-2 flex items-center gap-1.5 rounded-md relative text-neutral-200">
                {inlineAction.mode === "createDir" ? (
                  <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" fill="#38bdf8" fillOpacity="0.08" stroke="#475569" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
                ) : (
                  <svg className="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z" stroke="#475569" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /><path d="M14 2v6h6" stroke="#475569" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
                )}
                <input
                  autoFocus
                  value={inlineInputValue}
                  onChange={(e) => setInlineInputValue(e.target.value)}
                  onKeyDown={e => {
                    if (e.key === "Enter") handleInlineSubmit();
                    if (e.key === "Escape") handleInlineCancel();
                  }}
                  onBlur={handleInlineCancel}
                  className="flex-1 bg-neutral-800 text-[12px] border border-red-500 rounded px-1 outline-none relative z-10 w-full"
                />
              </div>
            )}
            {projectFiles.map((file) => (
              <FileTreeItem
                key={file.path}
                item={file}
                activeFile={activeFilePath}
                openFolders={openFolders}
                onFileClick={handleFileClick}
                onFolderToggle={toggleFolder}
                onContextMenu={handleContextMenu}
                inlineAction={inlineAction}
                inlineInputValue={inlineInputValue}
                setInlineInputValue={setInlineInputValue}
                onInlineInputSubmit={handleInlineSubmit}
                onInlineInputCancel={handleInlineCancel}
              />
            ))}
            {projectFiles.length === 0 && !inlineAction && (
              <div className="text-[10px] text-neutral-700 p-2 italic">
                No files found
              </div>
            )}
          </div>
          <div className="px-2 py-1 text-[10px] text-neutral-600 truncate italic">
            {projectDir === "." ? "No project selected" : projectDir}
          </div>

          <div className="p-2 text-sm font-medium mt-4" style={{ color: 'var(--text-muted)' }}>TOOLS</div>
          <button
            onClick={() => setShowSetupModal(true)}
            disabled={isSettingUpEspIdf}
            className="w-full text-left p-2 rounded flex items-center gap-2 text-sm transition-colors group"
            style={isSettingUpEspIdf
              ? { backgroundColor: 'rgba(245,158,11,0.1)', color: '#fcd34d', cursor: 'not-allowed' }
              : { color: 'var(--text-secondary)' }
            }
            onMouseEnter={e => { if (!isSettingUpEspIdf) e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
            onMouseLeave={e => { if (!isSettingUpEspIdf) e.currentTarget.style.backgroundColor = ''; }}
          >
            <span className={`w-4 h-4 flex items-center justify-center rounded text-[10px] font-bold ${isSettingUpEspIdf ? "bg-amber-400/30 text-amber-200" : "bg-neutral-700 text-neutral-300"}`}>
              {isSettingUpEspIdf ? "…" : "⚙"}
            </span>
            {isSettingUpEspIdf ? "Installing ESP-IDF..." : "Setup / Repair ESP-IDF"}
          </button>
          <button
            onClick={() => {
              const next = !showAiPanel;
              setShowAiPanel(next);
              localStorage.setItem("vibe-ai-panel", String(next));
              // Exclusive selection: opening Vibe Coder leaves the Wiki view
              if (next && activeView === "wiki") {
                setActiveView("editor");
                localStorage.setItem("vibe-active-view", "editor");
              }
            }}
            className="w-full text-left p-2 rounded flex items-center gap-2 text-sm transition-colors group"
            style={showAiPanel && activeView !== "wiki"
              ? { backgroundColor: 'var(--pms-293-pale)', color: 'var(--accent)' }
              : { color: 'var(--text-muted)' }
            }
            onMouseEnter={e => { if (!(showAiPanel && activeView !== "wiki")) e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
            onMouseLeave={e => { if (!(showAiPanel && activeView !== "wiki")) e.currentTarget.style.backgroundColor = ''; }}
          >
            <span className="w-4 h-4 flex items-center justify-center rounded text-[10px] font-bold"
              style={showAiPanel && activeView !== "wiki"
                ? { backgroundColor: 'var(--pms-293-pale)', color: 'var(--accent)' }
                : { backgroundColor: 'var(--bg-hover)', color: 'var(--text-muted)' }
              }
            >✦</span>
            Vibe Coder
          </button>

          {/* AI Wiki button */}
          <button
            onClick={() => {
              const next: "editor" | "wiki" = activeView === "wiki" ? "editor" : "wiki";
              setActiveView(next);
              localStorage.setItem("vibe-active-view", next);
              // Exclusive selection: entering the Wiki closes the Vibe Coder panel
              if (next === "wiki" && showAiPanel) {
                setShowAiPanel(false);
                localStorage.setItem("vibe-ai-panel", "false");
              }
            }}
            className="w-full text-left p-2 rounded flex items-center gap-2 text-sm transition-colors group"
            style={activeView === "wiki"
              ? { backgroundColor: 'rgba(167,139,250,0.12)', color: '#a78bfa' }
              : { color: 'var(--text-muted)' }
            }
            onMouseEnter={e => { if (activeView !== "wiki") e.currentTarget.style.backgroundColor = 'var(--bg-hover)'; }}
            onMouseLeave={e => { if (activeView !== "wiki") e.currentTarget.style.backgroundColor = ''; }}
          >
            <span
              className="w-4 h-4 flex items-center justify-center rounded text-[10px]"
              style={activeView === "wiki"
                ? { backgroundColor: 'rgba(167,139,250,0.2)', color: '#a78bfa' }
                : { backgroundColor: 'var(--bg-hover)', color: 'var(--text-muted)' }
              }
            >📚</span>
            AI Wiki
          </button>
        </div>


        <div className="p-4" style={{ borderTop: '1px solid var(--border-color)', backgroundColor: 'var(--bg-hover)' }}>
          <div className="flex items-center gap-2 text-xs mb-3">
            <div className={`w-2 h-2 rounded-full ${status.includes("Ready") || status.includes("OK") ? "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.6)]" : "bg-amber-500"}`}></div>
            <span className="text-neutral-400 truncate font-medium" style={{ color: 'var(--text-muted)' }}>{status.split(":")[0]}</span>
          </div>
          {espIdfSetupNote && (
            <div className="text-[10px] leading-relaxed mb-3 rounded p-2" style={{ color: 'var(--text-muted)', border: '1px solid var(--border-color)', backgroundColor: 'var(--bg-hover)' }}>
              {espIdfSetupNote}
            </div>
          )}
          <button
            onClick={handleBuildFlash}
            disabled={isBuilding || isSettingUpEspIdf || !toolchainReady}
            title={!toolchainReady ? "Waiting for toolchain download to complete..." : undefined}
            className="w-full justify-center text-sm px-4 py-2 rounded-lg transition-all duration-200 font-bold flex items-center gap-2 shadow-lg active:scale-[0.98]"
            style={isBuilding || !toolchainReady
              ? { backgroundColor: 'var(--bg-hover)', color: 'var(--text-muted)', cursor: 'not-allowed' }
              : { backgroundColor: 'var(--accent)', color: '#fff', boxShadow: '0 4px 16px var(--accent-glow)' }
            }
          >
            {isBuilding ? (
              <>
                <div className="w-3 h-3 border-2 border-neutral-500 border-t-neutral-300 rounded-full animate-spin" />
                Building...
              </>
            ) : !toolchainReady ? (
              <>
                <div className="w-3 h-3 border-2 border-amber-700 border-t-amber-400 rounded-full animate-spin" />
                Toolchain Loading...
              </>
            ) : "Build & Flash"}
          </button>
        </div>
      </div>

      {/* Main Area */}
      <div className="flex-1 min-w-0" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>

        {/* ── Wiki View ──────────────────────────────────────────────── */}
        {activeView === "wiki" && (
          <div style={{ flex: 1, overflow: 'hidden', display: 'flex' }}>
            <WikiView />
          </div>
        )}

        {/* ── Editor + Terminal (hidden when Wiki is active) ─────────── */}
        {activeView !== "wiki" && (
          <>
        <div className="flex-1 overflow-hidden relative" style={{ backgroundColor: 'var(--bg-main)' }}>

          <div className="absolute inset-0 flex flex-col">
            {/* Tab Bar + Build Progress */}
            <div className="flex flex-col z-10" style={{ borderBottom: '1px solid var(--border-color)', backgroundColor: 'var(--bg-panel)' }}>
              {/* Tab Row */}
              <div className="h-10 flex items-center justify-between backdrop-blur-sm overflow-hidden">
                <div className="flex items-center overflow-x-auto no-scrollbar flex-1">
                  {openFiles.map((file) => (
                    <div
                      key={file.path}
                      onClick={() => setActiveFilePath(file.path)}
                      className="flex items-center gap-2 px-4 h-10 cursor-pointer transition-colors text-xs font-medium whitespace-nowrap"
                      style={normPath(activeFilePath) === normPath(file.path)
                        ? { backgroundColor: 'var(--bg-active)', color: 'var(--accent)', borderBottom: '2px solid var(--accent)', borderRight: '1px solid var(--border-color)' }
                        : { color: 'var(--text-muted)', borderRight: '1px solid var(--border-color)' }
                      }
                    >
                      {/* Unsaved changes indicator */}
                      {isFileDirty(file) && (
                        <span className="w-2 h-2 rounded-full bg-amber-400 shadow-[0_0_6px_rgba(251,191,36,0.5)] shrink-0" title="Unsaved changes" />
                      )}
                      <span>{file.name}</span>
                      <button
                        onClick={(e) => closeFile(file.path, e)}
                        className="hover:text-red-400 transition-colors p-0.5 rounded-sm ml-1"
                      >
                        {isFileDirty(file) ? "●" : "×"}
                      </button>
                    </div>
                  ))}
                  {openFiles.length === 0 && (
                    <div className="px-4 text-xs italic" style={{ color: 'var(--text-muted)' }}>No files open</div>
                  )}
                </div>

                {activeFile && (
                  <div className="flex items-center gap-2 px-3 shrink-0 h-full" style={{ borderLeft: '1px solid var(--border-color)', backgroundColor: 'var(--bg-hover)' }}>
                    <button
                      onClick={() => reloadFile(activeFile.path)}
                      className="p-1.5 rounded transition-colors"
                      style={{ color: 'var(--text-muted)' }}
                      onMouseEnter={e => { e.currentTarget.style.color = 'var(--danger)'; e.currentTarget.style.backgroundColor = 'var(--bg-active)'; }}
                      onMouseLeave={e => { e.currentTarget.style.color = 'var(--text-muted)'; e.currentTarget.style.backgroundColor = ''; }}
                      title="Reload from disk"
                    >
                      <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                      </svg>
                    </button>
                    <button
                      onClick={handleSaveProjectAs}
                      className="flex items-center gap-1.5 px-3 py-1 text-[10px] font-bold rounded transition-all active:scale-95 uppercase tracking-wider"
                      style={{ backgroundColor: 'rgba(16,185,129,0.12)', color: '#10b981', border: '1px solid rgba(16,185,129,0.25)' }}
                      onMouseEnter={e => { e.currentTarget.style.backgroundColor = '#059669'; e.currentTarget.style.color = '#fff'; }}
                      onMouseLeave={e => { e.currentTarget.style.backgroundColor = 'rgba(16,185,129,0.12)'; e.currentTarget.style.color = '#10b981'; }}
                      title="Save Project As... (copies the whole folder to a new location)"
                    >
                      <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
                      </svg>
                      Save Project
                    </button>
                  </div>
                )}
              </div>

              {/* Build Progress Bar — shown during/after build */}
              {(isBuilding || buildResult === "success" || buildResult === "failed") && (
                <div
                  className="px-3 py-1.5 flex items-center gap-3 animate-fadein"
                  style={{
                    borderTop: '1px solid var(--border-color)',
                    backgroundColor: buildResult === "failed"
                      ? 'rgba(185,28,28,0.06)'
                      : buildResult === "success"
                      ? 'rgba(13,127,69,0.06)'
                      : 'var(--bg-hover)'
                  }}
                >
                  {/* Status icon */}
                  {isBuilding ? (
                    <div className="w-3 h-3 border-2 rounded-full animate-spin shrink-0"
                      style={{ borderColor: 'var(--accent)', borderTopColor: 'transparent' }} />
                  ) : buildResult === "success" ? (
                    <svg className="w-3 h-3 shrink-0" fill="none" viewBox="0 0 24 24" style={{ color: 'var(--success)' }}>
                      <path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                    </svg>
                  ) : (
                    <svg className="w-3 h-3 shrink-0" fill="none" viewBox="0 0 24 24" style={{ color: 'var(--danger)' }}>
                      <path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M6 18L18 6M6 6l12 12" />
                    </svg>
                  )}

                  {/* Progress bar track */}
                  <div className="flex-1 flex flex-col gap-0.5">
                    <div className="w-full rounded-full overflow-hidden" style={{ height: '5px', backgroundColor: 'var(--border-color)' }}>
                      <div
                        className="h-full rounded-full transition-all duration-300"
                        style={{
                          width: buildTotal > 0
                            ? `${Math.min(100, Math.round((buildStep / buildTotal) * 100))}%`
                            : isBuilding ? '100%' : '100%',
                          backgroundColor: buildResult === "failed"
                            ? 'var(--danger)'
                            : buildResult === "success"
                            ? 'var(--success)'
                            : 'var(--accent)',
                          animation: isBuilding && buildTotal === 0 ? 'progressPulse 1.5s ease-in-out infinite' : undefined
                        }}
                      />
                    </div>
                  </div>

                  {/* Task label */}
                  <span
                    className="text-[10px] font-medium shrink-0 max-w-[280px] truncate"
                    style={{
                      color: buildResult === "failed"
                        ? 'var(--danger)'
                        : buildResult === "success"
                        ? 'var(--success)'
                        : 'var(--text-secondary)'
                    }}
                    title={buildCurrentTask}
                  >
                    {buildCurrentTask || (isBuilding ? 'Building...' : '')}
                  </span>

                  {/* Step counter / Percentage */}
                  {buildTotal > 0 && (
                    <span className="text-[10px] font-bold tabular-nums shrink-0" style={{ color: 'var(--text-muted)' }}>
                      {buildTotal === 100
                        ? `${Math.round((buildStep / buildTotal) * 100)}%`
                        : `${buildStep}/${buildTotal}`}
                    </span>
                  )}

                  {/* Dismiss on success/fail */}
                  {!isBuilding && (
                    <button
                      onClick={() => setBuildResult("idle")}
                      className="text-[10px] shrink-0 px-1.5 py-0.5 rounded transition-colors"
                      style={{ color: 'var(--text-muted)', backgroundColor: 'var(--bg-hover)' }}
                      title="Dismiss"
                    >
                      ×
                    </button>
                  )}
                </div>
              )}

              {/* Friendly Build Error Helper — plain-language list of what went wrong */}
              {buildResult === "failed" && (
                <BuildErrorList
                  errors={buildErrors}
                  onJumpToError={jumpToError}
                  onAskAiFix={askAiToFixErrors}
                />
              )}
            </div>

            {activeFile ? (
              <CodeEditor
                key={activeFile.path}
                value={activeFile.content}
                onChange={handleEditorChange}
                filePath={activeFile.path}
                onSave={saveAllFiles}
                isDarkMode={darkMode}
                gotoLineRequest={gotoLineRequest && normPath(gotoLineRequest.path) === normPath(activeFile.path) ? gotoLineRequest : null}
              />
            ) : (
              <div className="flex-1 flex items-center justify-center" style={{ backgroundColor: 'var(--bg-editor)' }}>
                <div className="text-center space-y-3 opacity-40">
                  <div className="text-4xl">✨</div>
                  <p className="text-sm font-medium" style={{ color: 'var(--text-muted)' }}>
                    {projectDir === "." ? "Open or create a project to start coding" : "Select a file from the sidebar"}
                  </p>
                  <p className="text-xs" style={{ color: 'var(--text-disabled)' }}>
                    Ctrl+S to save • Syntax highlighting for C, Python, JSON & more
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Console & Terminal */}
        <div className="h-80 flex flex-col" style={{ borderTop: '1px solid var(--border-color)', backgroundColor: 'var(--bg-terminal)', boxShadow: 'var(--shadow-lg)' }}>
          <div className="h-9 flex items-center justify-between px-4" style={{ borderBottom: '1px solid var(--border-color)', backgroundColor: 'var(--bg-panel)' }}>
            <span className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--text-muted)' }}>Interactive Terminal</span>
            <div className="flex items-center gap-2">
              <div className="flex items-center">
                <select
                  value={selectedSerialPort}
                  onChange={(e) => setSelectedSerialPort(e.target.value)}
                  onClick={loadSerialPorts}
                  className="rounded px-2 py-1 text-[10px] w-[100px] focus:outline-none cursor-pointer"
                  style={{ backgroundColor: 'var(--bg-input)', border: '1px solid var(--border-color)', color: 'var(--text-primary)' }}
                  title="Auto-refreshing Serial Ports"
                >
                  {serialPorts.length === 0 ? (
                    <option value="" disabled>No Ports</option>
                  ) : (
                    <>
                      {!selectedSerialPort && <option value="" disabled>Select Port</option>}
                      {serialPorts.map((port) => (
                        <option key={port} value={port}>{port}</option>
                      ))}
                    </>
                  )}
                </select>
              </div>
              <input
                type="text"
                value={serialBaud}
                onChange={(e) => setSerialBaud(e.target.value)}
                className="w-20 rounded px-2 py-1 text-[10px]"
                style={{ backgroundColor: 'var(--bg-input)', border: '1px solid var(--border-color)', color: 'var(--text-primary)' }}
              />
              <button
                onClick={loadSerialPorts}
                className="text-[10px] px-2 py-1 rounded"
                style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)', border: '1px solid var(--border-color)' }}
              >
                Refresh Ports
              </button>
              <button
                onClick={toggleSerialMonitor}
                className={`text-[10px] px-2 py-1 rounded font-bold ${isSerialConnected ? "bg-amber-700 text-amber-100" : "bg-emerald-700 text-emerald-100"}`}
              >
                {isSerialConnected ? "Disconnect Serial" : "Connect Serial"}
              </button>
              <button
                onClick={() => setLogs([])}
                className="text-[10px] text-neutral-500 hover:text-neutral-300 transition-colors uppercase font-bold"
              >
                Clear Logs
              </button>
            </div>
          </div>
          <div
            ref={scrollRef}
            className="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1 selection:bg-red-500/20"
          >
            {logs.length === 0 ? (
              <div className="text-neutral-700 italic opacity-50">vibeKidbright Terminal Ready. Type 'idf.py --version' to test.</div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="flex gap-2 transition-colors" style={{ color: 'var(--text-secondary)' }}>
                  <span className="whitespace-pre-wrap break-all">{log}</span>
                </div>
              ))
            )}
          </div>
          {/* Terminal Input */}
          <div className="p-2 flex items-center gap-2 group" style={{ borderTop: '1px solid var(--border-color)', backgroundColor: 'var(--bg-panel)' }}>
            <span className="font-bold text-sm ml-2" style={{ color: 'var(--accent)' }}>$</span>
            <form onSubmit={handleTerminalSubmit} className="flex-1">
              <input
                type="text"
                value={terminalInput}
                onChange={(e) => setTerminalInput(e.target.value)}
                placeholder={isSerialConnected ? "Type message and press Enter to send to board..." : "Type command (e.g. idf.py) and press Enter..."}
                className="w-full bg-transparent border-none focus:outline-none font-mono text-sm placeholder:text-neutral-600"
                style={{ color: 'var(--text-primary)' }}
              />
            </form>
            <button
              onClick={sendSerialText}
              className="text-[10px] px-2 py-1 rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300"
            >
              Send Serial
            </button>
          </div>
        </div>
          </>
        )} {/* end activeView !== "wiki" */}
      </div>

      {/* AI Chat Panel */}

      {showAiPanel && (
        <div className="w-96 relative flex flex-col" style={{ borderLeft: '1px solid var(--border-color)' }}>
          <AiChat
            projectDir={projectDir}
            onInjectCode={(newCode) => updateActiveFileContent(newCode)}
            onApplyToFile={handleApplyToFile}
            sendApiRef={aiChatSendRef}
          />
        </div>
      )}

      {/* New Project Modal */}
      {showNewProjectModal && (
        <div className="absolute inset-0 backdrop-blur-sm flex items-center justify-center z-50" style={{ backgroundColor: 'rgba(0,0,0,0.5)' }}>
          <div className="rounded-xl p-6 w-96 animate-fadein theme-modal" style={{ backgroundColor: 'var(--bg-modal)', border: '1px solid var(--border-color)' }}>
            <h3 className="text-lg font-bold text-neutral-200 mb-4 flex items-center gap-2">
              <span className="text-red-400">📁</span> Create New Project
            </h3>

            <div className="space-y-4">
              <div>
                <label className="text-xs text-neutral-400 mb-1 block uppercase font-bold tracking-wider">
                  Project Name
                </label>
                <input
                  type="text"
                  value={newProjectName}
                  onChange={(e) => setNewProjectName(e.target.value)}
                  placeholder="my_esp_project"
                  className="w-full rounded-lg px-3 py-2 text-sm focus:outline-none transition-colors"
                  style={{ backgroundColor: 'var(--bg-input)', border: '1px solid var(--border-color)', color: 'var(--text-primary)' }}
                />
              </div>

              <div>
                <label className="text-xs text-neutral-400 mb-1 block uppercase font-bold tracking-wider">
                  Location
                </label>
                <div className="flex gap-2">
                  <div className="flex-1 bg-neutral-900 border border-neutral-600 rounded-lg px-3 py-2 text-xs text-neutral-400 truncate flex items-center">
                    {newProjectPath || "No directory selected"}
                  </div>
                  <button
                    onClick={handlePickDirectory}
                    className="px-3 py-2 bg-neutral-700 hover:bg-neutral-600 text-xs text-neutral-200 rounded-lg transition-colors shrink-0"
                  >
                    Browse
                  </button>
                </div>
              </div>
            </div>

            <div className="flex gap-3 mt-8">
              <button
                onClick={() => setShowNewProjectModal(false)}
                className="flex-1 py-2 bg-neutral-700 hover:bg-neutral-600 text-sm text-neutral-300 rounded-lg transition-colors font-medium"
              >
                Cancel
              </button>
              <button
                onClick={confirmCreateProject}
                disabled={!newProjectName || !newProjectPath}
                className="flex-1 py-2 rounded-lg transition-all font-bold text-sm active:scale-95"
                style={!newProjectName || !newProjectPath
                  ? { backgroundColor: 'var(--bg-hover)', color: 'var(--text-muted)', cursor: 'not-allowed' }
                  : { backgroundColor: 'var(--accent)', color: '#fff', boxShadow: '0 4px 12px var(--accent-glow)' }
                }
              >
                Create Project
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Setup / Repair ESP-IDF Modal ── */}
      {showSetupModal && (
        <div className="absolute inset-0 backdrop-blur-sm flex items-center justify-center z-50" style={{ backgroundColor: 'rgba(0,0,0,0.5)' }}>
          <div className="rounded-xl p-6 w-[480px] animate-fadein" style={{ backgroundColor: 'var(--bg-modal)', border: '1px solid var(--border-color)', boxShadow: 'var(--shadow-lg)' }}>
            <h3 className="text-lg font-bold text-neutral-200 mb-1 flex items-center gap-2">
              <span className="text-amber-400">⚙</span> Setup / Repair ESP-IDF
            </h3>
            <p className="text-xs text-neutral-500 mb-5">
              Choose how to configure your ESP-IDF environment
            </p>

            {/* Tab: Manual Path */}
            <div className="bg-neutral-900/60 border border-neutral-700 rounded-lg p-4 mb-4">
              <div className="flex items-center gap-2 mb-3">
                <span className="text-emerald-400 text-sm font-bold">📁 Manual Path</span>
                <span className="text-[10px] text-neutral-500 bg-neutral-700 px-1.5 py-0.5 rounded">Recommended if ESP-IDF already installed</span>
              </div>

              {setupModalError && (
                <div className="mb-3 p-2 bg-red-500/10 border border-red-500/20 rounded text-xs text-red-400">
                  {setupModalError}
                </div>
              )}

              <div className="space-y-3">
                <div>
                  <label className="text-[10px] text-neutral-400 uppercase font-bold tracking-wider mb-1 block">
                    ESP-IDF Framework Path <span className="text-neutral-600">(contains tools/idf.py)</span>
                  </label>
                  <div className="flex gap-2">
                    <div className="flex-1 bg-neutral-800 border border-neutral-600 rounded-lg px-3 py-2 text-xs text-neutral-300 truncate flex items-center font-mono">
                      {customIdfPath || <span className="text-neutral-600 italic">e.g. C:\Espressif\frameworks\esp-idf-v5.4.3</span>}
                    </div>
                    <button
                      onClick={handlePickIdfPath}
                      className="px-3 py-2 bg-neutral-700 hover:bg-neutral-600 text-xs text-emerald-300 rounded-lg transition-colors shrink-0 font-bold border border-emerald-500/20"
                    >
                      Browse
                    </button>
                  </div>
                </div>

                <div>
                  <label className="text-[10px] text-neutral-400 uppercase font-bold tracking-wider mb-1 block">
                    ESP-IDF Tools Path <span className="text-neutral-600">(contains python_env, tools folders)</span>
                  </label>
                  <div className="flex gap-2">
                    <div className="flex-1 bg-neutral-800 border border-neutral-600 rounded-lg px-3 py-2 text-xs text-neutral-300 truncate flex items-center font-mono">
                      {customToolsPath || <span className="text-neutral-600 italic">e.g. D:\Espressif</span>}
                    </div>
                    <button
                      onClick={handlePickToolsPath}
                      className="px-3 py-2 bg-neutral-700 hover:bg-neutral-600 text-xs text-emerald-300 rounded-lg transition-colors shrink-0 font-bold border border-emerald-500/20"
                    >
                      Browse
                    </button>
                  </div>
                </div>

                <div className="flex gap-2 pt-1">
                  <button
                    onClick={handleSaveCustomPaths}
                    disabled={isSavingPaths || !customIdfPath || !customToolsPath}
                    className={`flex-1 py-2 rounded-lg text-sm font-bold transition-all ${
                      isSavingPaths || !customIdfPath || !customToolsPath
                        ? "bg-neutral-700 text-neutral-500 cursor-not-allowed"
                        : "bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-500/20 active:scale-95"
                    }`}
                  >
                    {isSavingPaths ? "Saving..." : "Save & Apply"}
                  </button>
                  {(customIdfPath || customToolsPath) && (
                    <button
                      onClick={handleClearCustomPaths}
                      className="px-4 py-2 bg-neutral-700 hover:bg-red-900/50 text-xs text-neutral-400 hover:text-red-300 rounded-lg transition-colors"
                      title="Clear custom paths and use auto-detection"
                    >
                      Reset
                    </button>
                  )}
                </div>
              </div>
            </div>

            {/* Tab: Auto Install */}
            <div className="bg-neutral-900/60 border border-neutral-700 rounded-lg p-4 mb-5">
              <div className="flex items-center gap-2 mb-2">
                <span className="text-red-400 text-sm font-bold">⬇ Auto Install</span>
                <span className="text-[10px] text-neutral-500 bg-neutral-700 px-1.5 py-0.5 rounded">GitHub Release (~2.28 GB)</span>
              </div>
              <p className="text-[11px] text-neutral-500 mb-3">
                ดาวน์โหลด <code className="text-neutral-400">frameworks.zip</code> + <code className="text-neutral-400">tools.zip</code> จาก GitHub Release แล้วแตกไฟล์ลง AppData โดยอัตโนมัติ (ครั้งเดียว)
              </p>
              <button
                onClick={() => { setShowSetupModal(false); handleAutoInstallGithub(); }}
                disabled={isSettingUpEspIdf}
                className="w-full py-2 bg-red-600 hover:bg-red-500 text-sm font-bold text-white rounded-lg transition-all active:scale-95 shadow-lg shadow-red-500/20"
              >
                {isSettingUpEspIdf ? "⏳ Downloading from GitHub..." : "⬇️ Start Auto Install (GitHub)"}
              </button>
            </div>

            <button
              onClick={() => setShowSetupModal(false)}
              className="w-full py-2 bg-neutral-700 hover:bg-neutral-600 text-sm text-neutral-300 rounded-lg transition-colors"
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Context Menu Portal */}
      {contextMenu && (
        <div
          className="fixed z-50 w-48 rounded-md py-1 transform scale-100 origin-top-left flex flex-col text-[13px] font-medium animate-fadein"
          style={{ top: contextMenu.y, left: contextMenu.x, backgroundColor: 'var(--bg-modal)', border: '1px solid var(--border-color)', boxShadow: 'var(--shadow-lg)', color: 'var(--text-primary)' }}
        >
          <button
            onClick={() => { setInlineAction({ mode: "rename", path: contextMenu.path }); setInlineInputValue(contextMenu.path.split(/[\/\\]/).pop() || ""); setContextMenu(null); }}
            className="w-full text-left px-3 py-1.5 hover:bg-red-600/20 hover:text-red-300 transition-colors flex items-center gap-2"
          >
            <span>📝</span> Rename <span className="ml-auto text-[10px] text-neutral-500 font-sans">เปลี่ยนชื่อ</span>
          </button>
          
          {contextMenu.isDir && (
            <>
              <button
                onClick={() => { setInlineAction({ mode: "createFile", path: contextMenu.path }); setInlineInputValue(""); setContextMenu(null); }}
                className="w-full text-left px-3 py-1.5 hover:bg-red-600/20 hover:text-red-300 transition-colors flex items-center gap-2 mt-1"
              >
                <span>➕</span> New File
              </button>
              <button
                onClick={() => { setInlineAction({ mode: "createDir", path: contextMenu.path }); setInlineInputValue(""); setContextMenu(null); }}
                className="w-full text-left px-3 py-1.5 hover:bg-red-600/20 hover:text-red-300 transition-colors flex items-center gap-2"
              >
                <span>📁</span> New Folder
              </button>
            </>
          )}

          <div className="my-1 border-t border-neutral-700/80"></div>
          
          <button
            onClick={() => { handleDeleteItem(contextMenu.path, contextMenu.isDir); setContextMenu(null); }}
            className="w-full text-left px-3 py-1.5 hover:bg-red-500/20 text-red-400 hover:text-red-300 transition-colors flex items-center gap-2"
          >
            <span>🗑️</span> Delete <span className="ml-auto text-[10px] text-red-500/80 font-sans">ลบ</span>
          </button>
        </div>
      )}
    </div>
  );
}

export default AppShell;