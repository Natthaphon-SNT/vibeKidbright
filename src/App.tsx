import UaiPlusControls from "./UaiPlusControls";
import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import AiChat from "./AiChat";
import CodeEditor from "./CodeEditor";
import ToolchainSetup from "./ToolchainSetup";
import WikiView from "./WikiView";
import LearningWorkspace from "./LearningWorkspace";
import { parseErrorLine, type ParsedBuildError } from "./errorHints";
import BuildErrorList from "./BuildErrorList";
import { toast, ToastHost } from "./Toast";
import { normPath } from "./utils";
import { learnerProjectTree, isLearnerEditablePath, isProjectFilePath, type ProjectFileEntry as FileEntry } from "./projectFiles";
import SetupRepairButton from "./SetupRepairButton";
import MacosSdkSetup, { isMacOS } from "./MacosSdkSetup";
import {
  executeBuildLifecycle,
  isBuildFlashSupported,
  MIUAI_DEPLOYMENT_UNAVAILABLE,
  runBuildForBoard,
  type BoardType,
} from "./buildFlash";

const PORT_REFRESH_INTERVAL_MS = 2_000;

function terminalArguments(input: string): string[] {
  const args: string[] = [];
  let value = "";
  let quote: string | null = null;
  let started = false;
  for (const character of input) {
    if (quote) {
      if (character === quote) quote = null;
      else value += character;
    } else if (character === '"' || character === "'") {
      quote = character;
      started = true;
    } else if (/\s/.test(character)) {
      if (started) args.push(value);
      value = "";
      started = false;
    } else {
      value += character;
      started = true;
    }
  }
  if (quote) throw new Error("Close the quote before running the command.");
  if (started) args.push(value);
  return args;
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
            aria-label={item.name}
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
      aria-label={item.name}
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
  const [isRefreshing, setIsRefreshing] = React.useState(false);
  const [isInstallingToolchain, setIsInstallingToolchain] = React.useState(false);

  /** Soft-refresh: re-check toolchain โดยที่ state ของ App ไม่หาย */
  const refreshToolchain = React.useCallback(async () => {
    if (isRefreshing) return;
    setIsRefreshing(true);
    toast("🔄 กำลังตรวจสอบ Toolchain...", "info");
    try {
      const result = await invoke("check_toolchain") as { status: string; version: string | null };
      if (result.status === "ready") {
        setToolchainReady(true);
        toast(`✅ Toolchain v${result.version ?? ""} พร้อมใช้งาน`, "success");
      } else {
        // toolchain ไม่มี/ถูกลบ → แสดง ToolchainSetup อีกครั้ง
        setToolchainReady(false);
        toast("⚠️ Toolchain ไม่พบ — กรุณาติดตั้งใหม่", "error");
      }
    } catch (err) {
      setToolchainReady(false);
      toast(`❌ ตรวจสอบ Toolchain ล้มเหลว: ${err}`, "error");
    } finally {
      setIsRefreshing(false);
    }
  }, [isRefreshing]);

  // ── ดักจับ F5 / Ctrl+R / Ctrl+Shift+R → soft-refresh แทน hard reload ──
  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isF5 = e.key === "F5";
      const isCtrlR = (e.ctrlKey || e.metaKey) && e.key === "r";
      const isCtrlShiftR = (e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "R";

      if (isF5 || isCtrlR || isCtrlShiftR) {
        e.preventDefault();
        e.stopPropagation();
        refreshToolchain();
      }
    };

    window.addEventListener("keydown", handleKeyDown, true); // capture phase
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [refreshToolchain]);

  return (
    <>
      <App
        toolchainReady={toolchainReady}
        onRefreshToolchain={refreshToolchain}
        isInstallingToolchain={isInstallingToolchain}
        onInstallingToolchainChange={setIsInstallingToolchain}
      />
      <ToastHost />
      {/* Refresh overlay indicator */}
      {isRefreshing && (
        <div
          style={{
            position: "fixed",
            top: 12,
            right: 16,
            zIndex: 9999,
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "6px 14px",
            borderRadius: 8,
            background: "rgba(30,30,40,0.92)",
            border: "1px solid rgba(99,102,241,0.5)",
            color: "#a5b4fc",
            fontSize: 12,
            fontWeight: 600,
            backdropFilter: "blur(8px)",
            boxShadow: "0 4px 24px rgba(0,0,0,0.4)",
          }}
        >
          <div
            style={{
              width: 12,
              height: 12,
              border: "2px solid rgba(99,102,241,0.4)",
              borderTop: "2px solid #818cf8",
              borderRadius: "50%",
              animation: "spin 0.7s linear infinite",
            }}
          />
          ตรวจสอบ Toolchain...
        </div>
      )}
      {!toolchainReady && (
        <ToolchainSetup
          onReady={() => setToolchainReady(true)}
          mini={true}
          isInstalling={isInstallingToolchain}
          onInstallingChange={setIsInstallingToolchain}
        />
      )}
    </>
  );
}

function App({
  toolchainReady = true,
  onRefreshToolchain,
  isInstallingToolchain = false,
  onInstallingToolchainChange = () => {},
}: {
  toolchainReady?: boolean;
  onRefreshToolchain?: () => void;
  isInstallingToolchain?: boolean;
  onInstallingToolchainChange?: (isInstalling: boolean) => void;
}) {
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
  const isSettingUpEspIdf = isInstallingToolchain;
  const setIsSettingUpEspIdf = onInstallingToolchainChange;
  const [espIdfSetupNote, setEspIdfSetupNote] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [terminalInput, setTerminalInput] = useState("");
  const terminalHistoryRef = useRef<string[]>([]);
  const terminalHistoryIndexRef = useRef(-1);
  const [openFiles, setOpenFiles] = useState<FileTab[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string>("");
  const [showAiPanel, setShowAiPanel] = useState(() => {
    const saved = localStorage.getItem("vibe-ai-panel");
    return saved === null ? true : saved === "true"; // default: เปิดอยู่เสมอ
  });
  const [activeView, setActiveView] = useState<"editor" | "wiki" | "learning">(() => {
    const saved = localStorage.getItem("vibe-active-view");
    return saved === "editor" || saved === "wiki" ? saved : "learning";
  });
  useEffect(() => { localStorage.setItem("vibe-active-view", activeView); }, [activeView]);
  const activeViewRef = useRef(activeView);
  activeViewRef.current = activeView;

  // Persist the active project across F5 / webview reloads (restored + validated on startup).
  const [projectDir, setProjectDir] = useState<string>(() => {
    const restored = localStorage.getItem("vibe-project-dir");
    return restored && isProjectFilePath(restored, "main/main.py") ? restored : ".";
  });
  // Always-current project dir for listeners registered once (avoids stale closures).
  const projectDirRef = useRef(projectDir);
  projectDirRef.current = projectDir;
  const [isBuilding, setIsBuilding] = useState(false);
  const [isCancellingBuild,setIsCancellingBuild] = useState(false);
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
  const serialPortRefreshInFlightRef = useRef(false);

  // Board type selector (KidBright32 vs MiuAiPlus)
  const [selectedBoard, setSelectedBoard] = useState<BoardType>(() => {
    return (localStorage.getItem("vibe-selected-board") as BoardType) || "kidbright32";
  });
  const previousBoardRef = useRef(selectedBoard);
  useEffect(() => {
    if (previousBoardRef.current === "miuaiplus" && selectedBoard !== "miuaiplus") {
      void invoke("disconnect_uai_plus").catch(error => addLog(`Plus disconnect: ${error}`));
    }
    previousBoardRef.current = selectedBoard;
  }, [selectedBoard]);
  const [showNewProjectModal, setShowNewProjectModal] = useState(false);
  const newProjectModalRef = useRef(showNewProjectModal);
  const newDirectoryRequestRef = useRef(0);
  newProjectModalRef.current = showNewProjectModal;
  const [newProjectName, setNewProjectName] = useState("my_esp_project");
  const [newProjectPath, setNewProjectPath] = useState("");
  const [projectFiles, setProjectFiles] = useState<FileEntry[]>([]);
  const mainDirectoryPath = projectFiles.find(file => file.is_dir)?.path;
  const sidebarProjectRef = useRef("");
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
  const [chatDraftRequest, setChatDraftRequest] = useState<{ id: number; text: string } | null>(null);
  const chatDraftSequence = useRef(0);
  const saveAllFilesRef = useRef<() => Promise<void>>(async () => {});
  const buildFlashRef = useRef<() => void>(() => {});
  const buildInFlightRef = useRef(false);
  const activeBuildProjectRef = useRef<string | null>(null);
  const activeBuildEpochRef = useRef<number | null>(null);
  const buildCancelRequestedRef = useRef(false);
  const sdkBuildStartedRef = useRef(false);
  openFilesRef.current = openFiles;
  const projectDraftsRef = useRef(new Map<string, FileTab[]>());
  const projectEpochRef = useRef(0);
  const openProjectRequestRef = useRef(0);
  const localNavigationRef = useRef<number | null>(null);
  const fileOpenRequestRef = useRef(0);
  const treeRequestRef = useRef(0);
  const reloadRequestsRef = useRef(new Map<string, number>());
  const projectWritesRef = useRef<Promise<unknown>>(Promise.resolve());
  const captureProject = () => ({ path: projectDirRef.current, epoch: projectEpochRef.current });
  const isCurrentProject = (snapshot: { path: string; epoch: number }) =>
    snapshot.epoch === projectEpochRef.current && normPath(snapshot.path) === normPath(projectDirRef.current);
  const queueProjectWrite = <T,>(snapshot: { path: string; epoch: number }, action: () => Promise<T>): Promise<T> => {
    const operation = projectWritesRef.current.catch(() => {}).then(() => {
      if (!isCurrentProject(snapshot)) throw new Error("Project changed before saving; reopen the intended project and save again.");
      return action();
    });
    projectWritesRef.current = operation;
    return operation;
  };
  const acknowledgeSaved = (snapshot: { path: string; epoch: number }, saved: Map<string, string>) => {
    const update = (files: FileTab[]) => files.map(file => {
      const content = saved.get(normPath(file.path));
      return content === undefined ? file : { ...file, savedContent: content };
    });
    if (isCurrentProject(snapshot)) setOpenFiles(update);
    const key = normPath(snapshot.path);
    const cached = projectDraftsRef.current.get(key);
    if (cached) projectDraftsRef.current.set(key, update(cached));
  };

  // AI creation can change the workspace without a user navigation event.
  // Retain unsaved buffers separately so returning to that project restores them.
  const activateProject = (directory: string) => {
    if (directory !== "." && !isProjectFilePath(directory, "main/main.py")) {
      throw new Error("Choose an absolute project directory without path traversal.");
    }
    const previous = projectDirRef.current;
    if (normPath(previous) === normPath(directory)) return;
    projectEpochRef.current += 1;
    fileOpenRequestRef.current += 1;
    treeRequestRef.current += 1;
    const drafts = openFilesRef.current.filter(file => file.content !== file.savedContent);
    projectDraftsRef.current.set(normPath(previous), drafts);
    const restored = projectDraftsRef.current.get(normPath(directory)) || [];
    projectDirRef.current = directory;
    openFilesRef.current = restored;
    setProjectDir(directory);
    setOpenFiles(restored);
    setActiveFilePath(restored[0]?.path || "");
    setProjectFiles([]);
    setOpenFolders(new Set());
    sidebarProjectRef.current = "";
    setInlineAction(null);
    setContextMenu(null);
    setGotoLineRequest(null);
    setBuildErrors([]);
    if (drafts.length || restored.length) {
      toast("Unsaved code is kept in this session. Reopen its project to continue; save before closing the app.", "info");
    }
  };

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
      onRefreshToolchain?.();
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
    await checkEnvironment();
    onRefreshToolchain?.();
  };

  const handlePickIdfPath = async () => {
    const path = await invoke("pick_directory").catch(() => null);
    if (path) setCustomIdfPath(path as string);
  };

  const handlePickToolsPath = async () => {
    const path = await invoke("pick_directory").catch(() => null);
    if (path) setCustomToolsPath(path as string);
  };

  // Keep the footer in sync after native setup/recheck changes SDK readiness.
  useEffect(() => {
    checkEnvironment();
  }, [toolchainReady]);

  useEffect(() => {
    const unlistenTerminal = listen("terminal-output", (event) => {
      const line = event.payload as string;
      parseBuildProgressRef.current(line);
      setLogs((prev) => [...prev.slice(-1999), line.slice(-8192)]);
    });

    const unlistenFile = listen("file-modified", async (event) => {
      const { path } = JSON.parse(event.payload as string);
      if (!isProjectFilePath(projectDirRef.current, path)) return;
      const normP = normPath(path);
      const isOpen = openFilesRef.current.find(
        f => normPath(f.path) === normP
      );
      if (isOpen) {
        void reloadFile(isOpen.path, true);
      }
      loadProjectFiles();
    });

    const unlistenForceDir = listen("force-project-dir", async (event) => {
      try {
        const payload = String(event.payload);
        const change = payload.trim().startsWith("{") ? JSON.parse(payload) as { projectPath: string; sourceProjectPath?: string; action?: string }
          : { projectPath: payload, sourceProjectPath: undefined, action: undefined };
        const newPath = change.projectPath;
        if (typeof newPath !== "string") return;
        if (localNavigationRef.current !== null || newProjectModalRef.current) {
          addLog(`Project available at ${newPath}; the current folder choice was kept.`);
          return;
        }
        if (change.sourceProjectPath && normPath(change.sourceProjectPath) !== normPath(projectDirRef.current)) {
          addLog(`Project available at ${newPath}; the current project selection was kept.`);
          return;
        }
        const snapshot = captureProject();
        const request = ++openProjectRequestRef.current;
        const kind = newPath === "." ? null : await invoke<string | null>("get_project_kind", { path: newPath }).catch(() => null);
        if (!isCurrentProject(snapshot) || request !== openProjectRequestRef.current) return;
        if (newPath !== "." && !kind) {
          if (change.action === "undo") activateProject(".");
          addLog(`Project could not be opened or is incomplete: ${newPath}`);
          return;
        }
        activateProject(newPath);
        if (kind) {
          const board = kind === "kidbright_uai_plus" ? "miuaiplus" : "kidbright32";
          setSelectedBoard(board); localStorage.setItem("vibe-selected-board", board);
        }
        setActiveView("editor");
        addLog(`Switched project to: ${newPath}`);
      } catch (error) { addLog(`Project switch failed: ${String(error)}`); }
    });

    const unlistenDiffPending = listen("ai-diff-pending", async (event) => {
      try {
        const data = typeof event.payload === 'string' ? JSON.parse(event.payload) : event.payload as any;
        const fullPath = String(data.fullPath || "");
        if (!fullPath) return;
        const requestedEpoch = projectEpochRef.current;
        const requestedRoot = projectDirRef.current;
        if (!isProjectFilePath(requestedRoot, fullPath)) return;
        const focus = ++fileOpenRequestRef.current;
        setActiveView("editor");

        const norm = normPath;
        const existing = openFilesRef.current.find(f => norm(f.path) === norm(fullPath));

        if (!existing) {
          try {
            const pending = await invoke<{before:string|null} | null>("check_pending_diff", { path: fullPath });
            const content = pending && typeof pending === "object" && "before" in pending
              ? (pending.before ?? "") : await invoke("read_project_file", { path: fullPath, projectDir: requestedRoot });
            if (requestedEpoch !== projectEpochRef.current || focus !== fileOpenRequestRef.current) return;
            const fileName = fullPath.split(/[\/\\]/).pop() || "file";
            const newTab: FileTab = { name: fileName, path: fullPath, content: content as string, savedContent: content as string };
            setOpenFiles(prev => prev.some(file => norm(file.path) === norm(fullPath)) ? prev : [...prev, newTab]);
          } catch (e) {
            console.error("Failed to auto-open file for diff review:", e);
            return;
          }
        }
        if (requestedEpoch === projectEpochRef.current && focus === fileOpenRequestRef.current) setActiveFilePath(fullPath);
      } catch (e) {
        console.error("Error handling ai-diff-pending in App:", e);
      }
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        if (activeViewRef.current === "learning") {
          toast(localStorage.getItem("vibe-learning-language") === "en"
            ? "Practice data is temporary and is not saved to a project."
            : "ข้อมูลในพื้นที่ทดลองยังไม่บันทึกลงโปรเจกต์", "info");
          return;
        }
        void saveAllFilesRef.current().catch(() => {});
      }
      // Ctrl+Shift+B → Build & Flash
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "b") {
        e.preventDefault();
        buildFlashRef.current();
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
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const reloadFile = async (path: string, automatic = false) => {
    const snapshot = captureProject();
    if (!isProjectFilePath(snapshot.path, path)) return false;
    const key = normPath(path);
    const request = (reloadRequestsRef.current.get(key) ?? 0) + 1;
    reloadRequestsRef.current.set(key, request);
    const previous = openFilesRef.current.find(file => normPath(file.path) === normPath(path));
    if (previous && previous.content !== previous.savedContent) {
      if (automatic) {
        toast("The file changed on disk. Your unsaved code was kept; use Reload to review the disk version.", "info");
        return true;
      }
      if (!window.confirm("Reload this file and discard its unsaved changes?")) return false;
    }
    try {
      const content = await invoke("read_project_file", { path, projectDir: snapshot.path });
      if (!isCurrentProject(snapshot) || reloadRequestsRef.current.get(key) !== request) return false;
      setOpenFiles(prev => prev.map(f =>
        normPath(f.path) === normPath(path) && f.content === previous?.content
          ? { ...f, content: content as string, savedContent: content as string } : f
      ));
      addLog(`Auto-reloaded: ${path.split("/").pop()}`);
      return true;
    } catch (err) {
      if (isCurrentProject(snapshot)) addLog(`❌ Failed to reload file: ${err}`);
      return false;
    }
  };

  useEffect(() => {
    if (projectDir === ".") {
      localStorage.removeItem("vibe-project-dir");
    } else {
      localStorage.setItem("vibe-project-dir", projectDir);
    }
    void loadProjectFiles();
  }, [projectDir]);

  // On startup: if the restored project no longer exists / is invalid, fall back to "no project".
  useEffect(() => {
    if (projectDir === ".") return;
    const restoredProject = projectDir;
    const snapshot = captureProject();
    invoke<string | null>("get_project_kind", { path: restoredProject })
      .then((ok) => {
        if (!isCurrentProject(snapshot)) return;
        if (!ok) activateProject(".");
        else {
          if (ok === "kidbright_uai_plus") { setSelectedBoard("miuaiplus"); localStorage.setItem("vibe-selected-board", "miuaiplus"); }
          addLog(`Restored project: ${projectDir}`);
        }
      })
      .catch(() => { if (isCurrentProject(snapshot)) activateProject("."); });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs]);

  const saveAllFiles = useCallback(async () => {
    const snapshot = captureProject();
    const pendingFiles = openFilesRef.current
      .filter(file => isLearnerEditablePath(snapshot.path, file.path) && file.content !== file.savedContent)
      .map(file => ({ path: file.path, content: file.content }));
    const savedContents = new Map<string, string>();
    try {
      await queueProjectWrite(snapshot, async () => {
        for (const file of pendingFiles) {
          if (!isCurrentProject(snapshot)) throw new Error("Project changed while saving. Remaining files were kept unsaved.");
          await invoke("write_project_file", { path: file.path, content: file.content, projectDir: snapshot.path });
          savedContents.set(normPath(file.path), file.content);
        }
      });
      if (savedContents.size > 0) addLog(`✨ Saved ${savedContents.size} program files`);
    } catch (err) {
      addLog(`❌ Failed to save files: ${err}`);
      throw err;
    } finally {
      // Mark only bytes acknowledged by the backend as saved. Newer edits stay dirty.
      if (savedContents.size > 0) {
        acknowledgeSaved(snapshot, savedContents);
      }
    }
  }, []);
  // Keep ref in sync so the keyboard shortcut always calls the latest version
  useEffect(() => { saveAllFilesRef.current = saveAllFiles; }, [saveAllFiles]);

  const handleSaveProjectAs = async () => {
    const snapshot = captureProject();
    const request = ++openProjectRequestRef.current;
    if (snapshot.path === ".") {
      addLog("❌ No project open to save.");
      return;
    }
    localNavigationRef.current = request;

    try {
      await saveAllFiles();
      if (!isCurrentProject(snapshot) || request !== openProjectRequestRef.current) throw new Error("Project changed before Save As. Choose Save As again in the intended project.");
      addLog("Opening folder picker to save project as...");
      const result = await invoke<{ projectPath: string; filesCopied: number }>("save_project_as", { sourceDir: snapshot.path });
      const { projectPath: newPath, filesCopied: fileCount } = result;

      addLog(`✨ Project successfully saved to ${newPath} (${fileCount} files copied)`);
      if (!isCurrentProject(snapshot) || request !== openProjectRequestRef.current) return;

      const switchProject = window.confirm(`Project saved successfully!\n\nDo you want to switch to the new project location?\n\n${newPath}`);

      if (switchProject) {
        const kind = await invoke<string | null>("get_project_kind", { path: newPath });
        if (!isCurrentProject(snapshot) || request !== openProjectRequestRef.current) return;
        if (!kind) throw new Error("Copied project could not be validated. The previous project remains open.");
        activateProject(newPath);
        const next = captureProject();
        const board = kind === "kidbright_uai_plus" ? "miuaiplus" : "kidbright32";
        setSelectedBoard(board); localStorage.setItem("vibe-selected-board", board);
        const loaded = await handleFileClick(`${newPath.replace(/\\/g, "/")}/main/${kind === "kidbright_uai_plus" ? "main.py" : "main.c"}`);
        if (!isCurrentProject(next) || !loaded) return;
        addLog(`Switched project to: ${newPath}`);
      }
    } catch (err) {
      addLog(`❌ Failed to save project: ${err}`);
      toast(`Save Project As failed: ${err}`, "error");
    } finally {
      if (localNavigationRef.current === request) localNavigationRef.current = null;
    }
  };

  const handleContextMenu = (e: React.MouseEvent, path: string, isDir: boolean) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, path, isDir });
  };

  const handleDeleteItem = async (path: string, isDir: boolean) => {
    const snapshot = captureProject();
    if (!isLearnerEditablePath(projectDir, path)) return;
    const name = path.split(/[\/\\]/).pop() || "";
    if (name === "CMakeLists.txt" || name === "sdkconfig") {
      const confirmCritical = window.confirm(`⚠️ WARNING: '${name}' is a critical ESP-IDF file.\n\nDeleting it may break your project.\n\nAre you absolutely sure you want to delete '${name}'?`);
      if (!confirmCritical) return;
    } else {
      const confirmDelete = window.confirm(`Are you sure you want to delete '${name}'?`);
      if (!confirmDelete) return;
    }

    try {
      await invoke("delete_program_item", { projectDir, path });
      if (!isCurrentProject(snapshot)) return;
      addLog(`✨ Deleted: ${path}`);
      loadProjectFiles();
      const removedPath = (filePath: string) => normPath(filePath) === normPath(path)
        || (isDir && normPath(filePath).startsWith(`${normPath(path)}/`));
      setOpenFiles(prev => prev.filter(file => !removedPath(file.path)));
      if (removedPath(activeFilePath)) setActiveFilePath("");
    } catch (err) {
      if (!isCurrentProject(snapshot)) return;
      toast(`Failed to delete: ${err}`, "error");
    }
  };

  const handleInlineSubmit = async () => {
    const snapshot = captureProject();
    if (!inlineAction || !inlineInputValue.trim()) {
      setInlineAction(null);
      return;
    }

    const value = inlineInputValue.trim();
    try {
      if (value === "." || value === ".." || /[\\/:]/.test(value)) {
        throw new Error("Enter a file or folder name inside main.");
      }
      if (inlineAction.mode === "rename") {
        const oldName = inlineAction.path.split(/[\/\\]/).pop() || "";
        const newPath = inlineAction.path.substring(0, inlineAction.path.length - oldName.length) + value;
        if (!isLearnerEditablePath(projectDir, inlineAction.path) || !isLearnerEditablePath(projectDir, newPath)) {
          throw new Error("Only program files inside main can be renamed.");
        }
        await invoke("rename_program_item", { projectDir, oldPath: inlineAction.path, newPath });
        if (!isCurrentProject(snapshot)) return;
        addLog(`✨ Renamed to ${value}`);
        const renamedPath = (path: string) => {
          if (normPath(path) === normPath(inlineAction.path)) return newPath;
          if (normPath(path).startsWith(`${normPath(inlineAction.path)}/`)) return newPath + path.slice(inlineAction.path.length);
          return path;
        };
        setOpenFiles(prev => prev.map(file => {
          const path = renamedPath(file.path);
          return path === file.path ? file : { ...file, path, name: path.split(/[\/\\]/).pop() || file.name };
        }));
        setActiveFilePath(renamedPath);
        setOpenFolders(prev => new Set([...prev].map(renamedPath)));
      } else {
        const parentPath = inlineAction.path;
        const sep = parentPath.includes('\\') ? '\\' : '/';
        const newPath = parentPath + sep + value;
        if (!isLearnerEditablePath(projectDir, newPath)) {
          throw new Error("Create program files inside main. Build and board settings are managed separately.");
        }
        
        if (inlineAction.mode === "createFile") {
          await invoke("create_program_file", { projectDir, path: newPath });
          if (!isCurrentProject(snapshot)) return;
          addLog(`✨ Created file ${value}`);
          
          await loadProjectFiles();
          if (!isCurrentProject(snapshot)) return;
          
          const newFile: FileTab = { name: value, path: newPath, content: "", savedContent: "" };
          setOpenFiles(prev => [...prev, newFile]);
          setActiveFilePath(newPath);
        } else {
          await invoke("create_program_directory", { projectDir, path: newPath });
          if (!isCurrentProject(snapshot)) return;
          addLog(`✨ Created directory ${value}`);
        }
        setOpenFolders(prev => new Set(prev).add(parentPath));
      }
    } catch (err) {
      if (!isCurrentProject(snapshot)) return;
      toast(`Operation failed: ${err}`, "error");
    }

    setInlineAction(null);
    loadProjectFiles();
  };

  const handleInlineCancel = () => setInlineAction(null);

  const loadProjectFiles = async () => {
    const snapshot = captureProject();
    const dir = snapshot.path;
    const request = ++treeRequestRef.current;
    if (dir === ".") {
      setProjectFiles([]);
      sidebarProjectRef.current = "";
      return;
    }
    try {
      const files = await invoke("list_project_files", { path: dir });
      if (!isCurrentProject(snapshot) || request !== treeRequestRef.current) return;
      const tree = learnerProjectTree(files as FileEntry[], dir);
      setProjectFiles(tree);
      if (sidebarProjectRef.current !== dir) {
        setOpenFolders(new Set(tree.filter(file => file.is_dir).map(file => file.path)));
        sidebarProjectRef.current = dir;
      }
    } catch (err) {
      if (!isCurrentProject(snapshot) || request !== treeRequestRef.current) return;
      console.error("Failed to load project files:", err);
      toast("Could not load the project file list.", "error");
    }
  };

  const loadSerialPorts = async (reportError = true) => {
    if (serialPortRefreshInFlightRef.current) return;
    serialPortRefreshInFlightRef.current = true;

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
      if (reportError) {
        addLog(`❌ Failed to list serial ports: ${err}`);
      }
    } finally {
      serialPortRefreshInFlightRef.current = false;
    }
  };

  // Refresh the active board's port list automatically without overlapping scans.
  useEffect(() => {
    const refreshVisiblePorts = () => {
      if (selectedBoard === "kidbright32") void loadSerialPorts(false);
    };

    refreshVisiblePorts();
    const intervalId = window.setInterval(refreshVisiblePorts, PORT_REFRESH_INTERVAL_MS);
    return () => window.clearInterval(intervalId);
  }, [selectedBoard]);

  const handleFileClick = async (path: string) => {
    const snapshot = captureProject();
    if (!isProjectFilePath(snapshot.path, path)) return false;
    if (!/^(?:[a-z]:[\\/]|[\\/])/i.test(path)) path = `${snapshot.path.replace(/[\\/]+$/, "")}/${path}`;
    const focus = ++fileOpenRequestRef.current;
    setActiveView("editor");
    const np = normPath(path);
    const existing = openFilesRef.current.find(f => normPath(f.path) === np);
    if (existing) {
      setActiveFilePath(existing.path);
      return true;
    }

    try {
      const content = await invoke("read_project_file", { path, projectDir: snapshot.path });
      if (!isCurrentProject(snapshot) || focus !== fileOpenRequestRef.current) return false;
      const newTab: FileTab = {
        name: path.split(/[\/\\]/).pop() || "unknown",
        path,
        content: content as string,
        savedContent: content as string
      };
      setOpenFiles(prev => prev.some(file => normPath(file.path) === np) ? prev : [...prev, newTab]);
      setActiveFilePath(path);
      addLog(`Opened: ${newTab.name}`);
      return true;
    } catch (err) {
      if (isCurrentProject(snapshot) && focus === fileOpenRequestRef.current) addLog(`❌ Failed to read file: ${err}`);
      return false;
    }
  };

  const closeFile = (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    const np = normPath(path);
    const file = openFilesRef.current.find(item => normPath(item.path) === np);
    if (file && file.content !== file.savedContent
      && !window.confirm(`Close ${file.name} and discard its unsaved changes?`)) return;
    setOpenFiles(prev => {
      const next = prev.filter(f => normPath(f.path) !== np);
      if (normPath(activeFilePath) === np) {
        setActiveFilePath(next.length > 0 ? next[next.length - 1].path : "");
      }
      return next;
    });
  };

  const updateActiveFileContent = async (newContent: string) => {
    const snapshot = captureProject();
    if (!activeFilePath) {
      console.warn("Attempted to update content but no file is active.");
      addLog("❌ Vibe Code: No active file selected. Please click on a file in the sidebar to open it first.");
      return;
    }
    if (!isLearnerEditablePath(projectDir, activeFilePath)) return;
    const np = normPath(activeFilePath);
    setOpenFiles(prev => prev.map(f => normPath(f.path) === np ? { ...f, content: newContent } : f));

    try {
      await queueProjectWrite(snapshot, () => invoke("write_project_file", {
        path: activeFilePath, content: newContent, projectDir: snapshot.path
      }));
      acknowledgeSaved(snapshot, new Map([[np, newContent]]));
      const fileName = activeFilePath.split(/[\/\\]/).pop() || activeFilePath;
      addLog(`✨ Vibe Code Applied & Saved to: ${fileName}`);
    } catch (err) {
      addLog(`❌ Failed to save injected code: ${err}`);
      toast(`Could not save AI code: ${err}. Your changes remain unsaved.`, "error");
    }
  };

  const isFileDirty = (file: FileTab): boolean => {
    return file.content !== file.savedContent;
  };

  const handleEditorChange = (newContent: string) => {
    if (!activeFilePath || !isLearnerEditablePath(projectDir, activeFilePath)) return;
    const np = normPath(activeFilePath);
    setOpenFiles(prev => prev.map(f =>
      normPath(f.path) === np ? { ...f, content: newContent } : f
    ));
  };

  const handleApplyToFile = async (filePath: string, newContent: string) => {
    const snapshot = captureProject();
    const normSlash = (p: string) => p.replace(/\\/g, '/');
    const comparePath = normPath;

    let absolutePath: string;
    if (filePath.startsWith('/') || filePath.match(/^[a-zA-Z]:[\\\/]/)) {
      absolutePath = normSlash(filePath);
    } else {
      absolutePath = normSlash(`${snapshot.path}/${filePath}`);
    }
    if (!isProjectFilePath(snapshot.path, absolutePath)) {
      toast("AI code can only be applied to a file inside the selected project.", "error");
      return;
    }

    const confirmApply = window.confirm(`Apply AI code to: ${absolutePath}?`);
    if (!confirmApply) return;
    const originalContent = openFilesRef.current.find(file => normPath(file.path) === normPath(absolutePath))?.content;

    try {
      await queueProjectWrite(snapshot, () => invoke("safe_write_project_file", { path: absolutePath, content: newContent, projectDir: snapshot.path }));
      if (!isCurrentProject(snapshot)) {
        acknowledgeSaved(snapshot, new Map([[normPath(absolutePath), newContent]]));
        return;
      }
      addLog(`✨ Vibe Code check & overwrite OK: ${filePath}`);

      const existingFile = openFilesRef.current.find(
        (f) => comparePath(f.path) === comparePath(absolutePath)
      );

      if (existingFile) {
        setOpenFiles((prev) =>
          prev.map((f) =>
            comparePath(f.path) === comparePath(absolutePath) ? { ...f, content: f.content === originalContent ? newContent : f.content, savedContent: newContent } : f
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
      toast(`Safety check failed: ${err}`, "error");
    }
  };

  const handleOpenProject = async () => {
    const request = ++openProjectRequestRef.current;
    localNavigationRef.current = request;
    const previousEpoch = projectEpochRef.current;
    try {
      const path = await invoke("pick_directory");
      if (!path) return;

      const isValid = await invoke<string | null>("get_project_kind", { path });
      if (request !== openProjectRequestRef.current || previousEpoch !== projectEpochRef.current) return;
      if (isValid) {
        const board = isValid === "kidbright_uai_plus" ? "miuaiplus" : "kidbright32";
        setSelectedBoard(board); localStorage.setItem("vibe-selected-board", board);
        const reopening = normPath(projectDirRef.current) === normPath(path as string);
        activateProject(path as string);
        const openedEpoch = projectEpochRef.current;
        if (reopening) {
          // Reopening is an explicit refresh. Preserve dirty buffers and reload
          // saved tabs, even though React's projectDir value has not changed.
          const refreshed = await Promise.all(openFilesRef.current.map(file => reloadFile(file.path, true)));
          if (refreshed.some(ok => !ok)) {
            if (openedEpoch === projectEpochRef.current) addLog(`Project refresh failed; the previous editor contents were kept: ${path}`);
            return;
          }
          await loadProjectFiles();
        }
        if (openedEpoch !== projectEpochRef.current || request !== openProjectRequestRef.current) return;
        const entry = `${String(path).replace(/\\/g, "/").replace(/\/$/, "")}/main/${isValid === "kidbright_uai_plus" ? "main.py" : "main.c"}`;
        const loaded = await handleFileClick(entry);
        if (openedEpoch !== projectEpochRef.current || request !== openProjectRequestRef.current) return;
        if (!loaded) {
          addLog(`Project selected, but its main program could not be loaded: ${path}`);
          return;
        }
        setActiveView("editor");
        addLog(`Project opened: ${path}`);
      } else {
        toast("Choose an ESP-IDF project or a KidBright uAI Plus Python project", "error");
      }
    } catch (err) {
      addLog(`Error opening project: ${err}`);
    } finally {
      if (localNavigationRef.current === request) localNavigationRef.current = null;
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
    } catch (err) {
      setStatus("ESP-IDF needs setup");
      setEspIdfSetupNote(String(err));
      addLog(`⚠️ ${err}`);
    }
  };

  const toggleSerialMonitor = async () => {
    if (isSerialConnected) {
      try {
        const result = await invoke("stop_serial_monitor");
        addLog(`${result}`);
        setIsSerialConnected(false);
      } catch (err) {
        addLog(`❌ Failed to stop monitor: ${err}`);
      }
      return;
    }

    if (selectedBoard === "kidbright32") {
      // Serial monitor for KidBright32
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
    }
  };

  const sendSerialText = async () => {
    if (!terminalInput.trim()) return;
    if (!isSerialConnected) {
      addLog("❌ Monitor is not connected");
      return;
    }

    try {
      if (selectedBoard === "kidbright32") {
        // Send via serial port
        let payload = terminalInput;
        if (payload.endsWith("\r\n")) {
        } else if (payload.endsWith("\n")) {
          payload = payload.slice(0, -1) + "\r\n";
        } else {
          payload += "\r\n";
        }

        await invoke("send_serial_input", { input: payload });
        addLog(`[SERIAL TX] ${terminalInput}`);
      }
      setTerminalInput("");
    } catch (err) {
      addLog(`❌ Failed to send input: ${err}`);
    }
  };

  const parseBuildProgress = (msg: string) => {
    // Strip optional timestamp prefix added by addLog: "[3:41:18 PM] ..."
    const raw = msg.replace(/^\[\d{1,2}:\d{2}:\d{2}\s+[AP]M\]\s+/, "").trim();

    // 0. Collect friendly build errors from compiler / linker / esptool output
    const parsedErr = parseErrorLine(raw);
    const currentBuild = activeBuildProjectRef.current === null || (
      activeBuildEpochRef.current === projectEpochRef.current
      && normPath(activeBuildProjectRef.current) === normPath(projectDirRef.current));
    if (parsedErr && currentBuild) {
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
    setLogs((prev) => [...prev.slice(-1999), `[${new Date().toLocaleTimeString()}] ${msg.slice(-8192)}`]);
  };

  // Keep ref in sync so event listeners always have the latest parser
  parseBuildProgressRef.current = parseBuildProgress;

  const handleTerminalSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!terminalInput.trim() || selectedBoard === "miuaiplus") return;

    if (isSerialConnected) {
      await sendSerialText();
      return;
    }

    let parts: string[];
    try {
      parts = terminalArguments(terminalInput);
    } catch (err) {
      addLog(`Error: ${err}`);
      return;
    }
    const cmd = parts[0];
    if (!cmd) return;
    const args = parts.slice(1);

    addLog(`> ${terminalInput}`);
    // Save to command history
    terminalHistoryRef.current.push(terminalInput.trim());
    terminalHistoryIndexRef.current = -1;
    setTerminalInput("");

    try {
      if (cmd === "idf.py") {
        await invoke("run_idf_command", {
          command: args[0] || "--help", args: args.slice(1),
          projectDir: projectDir === "." ? null : projectDir,
        });
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
    newDirectoryRequestRef.current += 1;
    newProjectModalRef.current = true;
    setShowNewProjectModal(true);
    setNewProjectName("my_esp_project");
    setNewProjectPath("");
  };

  const handlePickDirectory = async () => {
    const request = ++newDirectoryRequestRef.current;
    try {
      const path = await invoke("pick_directory");
      if (path && request === newDirectoryRequestRef.current && newProjectModalRef.current) {
        setNewProjectPath(path as string);
      }
    } catch (err) {
      addLog(`Directory picker error: ${err}`);
    }
  };

  const confirmCreateProject = async () => {
    if (!newProjectPath || !newProjectName) return;
    const snapshot = captureProject();
    const request = ++openProjectRequestRef.current;
    localNavigationRef.current = request;
    const board = selectedBoard;

    setShowNewProjectModal(false);
    addLog(`Attempting to create project '${newProjectName}' at ${newProjectPath}...`);

    try {
      const result = await invoke<string>(board === "miuaiplus" ? "create_uai_plus_project" : "create_idf_project", {
        path: newProjectPath,
        name: newProjectName
      });
      addLog(`Success: ${result}`);

      if (!isCurrentProject(snapshot) || request !== openProjectRequestRef.current) return;
      const fullPath = result;
      activateProject(fullPath);
      const next = captureProject();
      setSelectedBoard(board); localStorage.setItem("vibe-selected-board", board);
      const loaded = await handleFileClick(`${fullPath.replace(/\\/g, "/")}/main/${board === "miuaiplus" ? "main.py" : "main.c"}`);
      if (!isCurrentProject(next) || !loaded) return;
      setActiveView("editor");
      addLog(`Active project set to: ${fullPath}`);
    } catch (err) {
      console.error("New project error:", err);
      addLog(`❌ ERROR: ${err}`);
      toast(`Failed to create project: ${err}`, "error");
    } finally {
      if (localNavigationRef.current === request) localNavigationRef.current = null;
    }
  };

  const handleBuildFlash = async () => {
    if (buildInFlightRef.current || isBuilding) return;

    if (!isBuildFlashSupported(selectedBoard)) {
      addLog(`⚠️ ${MIUAI_DEPLOYMENT_UNAVAILABLE}`);
      toast(MIUAI_DEPLOYMENT_UNAVAILABLE, "info");
      return;
    }
    if (activeView === "learning") {
      toast("เปิดพื้นที่เขียนโปรแกรมและตรวจบอร์ดที่เลือกก่อน Build & Flash", "info");
      return;
    }
    
    if (projectDir === ".") {
      addLog("❌ Error: No project selected to build.");
      toast("Please open or create a project first from the sidebar.", "info");
      return;
    }
    if (!toolchainReady || isSettingUpEspIdf) {
      toast("Wait for ESP-IDF setup to finish before building.", "info");
      return;
    }
    // Lock before any save/disconnect await, including repeated shortcut events.
    buildInFlightRef.current = true;
    const buildSnapshot = captureProject();
    const buildProjectDir = buildSnapshot.path;
    activeBuildProjectRef.current = buildProjectDir;
    activeBuildEpochRef.current = buildSnapshot.epoch;
    buildCancelRequestedRef.current = false;
    sdkBuildStartedRef.current = false;
    try {
      setBuildStep(0);
      setBuildTotal(0);
      setBuildErrors([]);
      addLog("--- Starting Build & Flash ---");
      await executeBuildLifecycle({
        run: async () => {
          setBuildCurrentTask("Saving program files...");
          await saveAllFiles();
          if (buildCancelRequestedRef.current) throw new Error("Build cancelled before starting the SDK.");
          if (!isCurrentProject(buildSnapshot)) throw new Error("Project changed while preparing the build. Build again in the selected project.");
          setBuildCurrentTask("Initializing...");
          // Release the COM port before esptool opens it.
          if (isSerialConnected) {
            await invoke("stop_serial_monitor");
            setIsSerialConnected(false);
            addLog("🔌 Serial monitor disconnected automatically to free the COM port for flashing.");
            await new Promise(resolve => setTimeout(resolve, 500));
          }
          if (!isCurrentProject(buildSnapshot)) throw new Error("Project changed while preparing the build. Build again in the selected project.");
          const argv = selectedSerialPort ? ["-p", selectedSerialPort, "build", "flash"] : ["build", "flash"];
          if (buildCancelRequestedRef.current) throw new Error("Build cancelled before starting the SDK.");
          if (selectedSerialPort) addLog(`Using port: ${selectedSerialPort}`);
          else addLog("⚠️ No serial port selected — idf.py will use its default port");
          sdkBuildStartedRef.current = true;
          await runBuildForBoard(selectedBoard, () => invoke("run_idf_command", {
            command: argv[0], args: argv.slice(1), projectDir: buildProjectDir,
          }));
        },
        setResult: setBuildResult,
        setBuilding: setIsBuilding,
        onFailure: err => {
          addLog(`Build failed: ${err}`);
          setBuildCurrentTask("Build failed");
          toast(`Build stopped: ${err}`, "error");
        },
      });
      setBuildCurrentTask(prev => prev === "Initializing..." ? "" : prev);
    } finally {
      buildInFlightRef.current = false;
      activeBuildProjectRef.current = null;
      activeBuildEpochRef.current = null;
      sdkBuildStartedRef.current = false;
    }
  };
  // Keep ref in sync so the keyboard shortcut (Ctrl+Shift+B) always calls the latest version
  useEffect(() => { buildFlashRef.current = handleBuildFlash; });

  // Open the file from a build error (if needed) and jump to the offending line
  const jumpToError = async (err: ParsedBuildError) => {
    if (!err.file) return;
    const snapshot = captureProject();
    if (!isProjectFilePath(snapshot.path, err.file)) return;
    const filePath = /^(?:[a-z]:[\\/]|[\\/])/i.test(err.file) ? err.file : `${snapshot.path.replace(/[\\/]+$/, "")}/${err.file}`;
    const target = err.line ?? 1;
    const np = normPath(filePath);
    const existing = openFiles.find(f => normPath(f.path) === np);
    if (existing) {
      setActiveFilePath(existing.path);
      setGotoLineRequest({ path: existing.path, line: target, column: err.column, token: Date.now() });
      return;
    }
    try {
      const opened = await handleFileClick(filePath);
      if (opened && isCurrentProject(snapshot)) setGotoLineRequest({ path: filePath, line: target, column: err.column, token: Date.now() });
    } catch {
      addLog(`❌ Failed to open ${err.file}`);
    }
  };

  // Send the collected build errors to Vibe Coder and ask for a simple fix
  const askAiToFixErrors = async () => {
    if (buildErrors.length === 0) return;
    try {
      const output=buildErrors.slice(0,20).map(e=>e.file?`${e.file}:${e.line??1}:${e.column??1}: error: ${e.message}`:e.message).join("\n");
      await invoke("begin_compiler_repair",{projectDir,output,maxAttempts:3});
    } catch(error) {toast(`Could not start compiler repair: ${String(error)}`,"error");return;}
    const prompt = "My configured ESP-IDF build failed. Call compiler_diagnostics to read its structured evidence, then inspect the relevant project files and propose a correction with write_file. Explain why it helps. The user must approve each attempt separately in Compiler Repair; do not apply a patch or report success before the configured build passes.";
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
            <h1 className="text-xl font-bold brand-gradient flex items-center gap-1.5"><img src="/app-icon.png" alt="icon" className="w-6 h-6" /> vibeKidbright</h1>
            <button
              onClick={toggleTheme}
              className="theme-toggle"
              title={darkMode ? 'Switch to Light Mode' : 'Switch to Dark Mode'}
            >
              {darkMode ? '☀️' : '🌙'}
            </button>
          </div>
          <p className="text-xs mt-1 uppercase tracking-widest font-semibold" style={{ color: 'var(--text-muted)' }}>KidBright · Code &amp; AI</p>
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          <button onClick={() => setActiveView("learning")} aria-pressed={activeView === "learning"}
            className="w-full flex items-center gap-2 rounded-lg px-3 py-3 text-sm text-left mb-3"
            style={{ color: activeView === "learning" ? 'var(--accent)' : 'var(--text-secondary)', background: activeView === "learning" ? 'var(--pms-293-ultra)' : 'transparent' }}>
            <span aria-hidden="true">◈</span> พื้นที่เรียนรู้
            <span className="ml-auto text-[9px] font-semibold" style={{ color: 'var(--text-muted)' }}>Code + AI</span>
          </button>
          <div className="flex items-center justify-between p-2">
            <span className="text-sm font-medium" style={{ color: 'var(--text-muted)' }}>PROJECT</span>
            <div className="flex gap-1 items-center">
              <button
                onClick={() => { if (mainDirectoryPath) { setInlineAction({ mode: "createFile", path: mainDirectoryPath }); setInlineInputValue(""); } }}
                disabled={!mainDirectoryPath}
                className="transition-colors p-1 rounded" style={{ color: 'var(--text-muted)' }}
                title="New File in main"
                onMouseEnter={e => (e.currentTarget.style.color = 'var(--accent)')}
                onMouseLeave={e => (e.currentTarget.style.color = 'var(--text-muted)')}
              >
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z"/><path d="M14 2v6h6"/><line x1="12" y1="18" x2="12" y2="12"/><line x1="9" y1="15" x2="15" y2="15"/></svg>
              </button>
              <button
                onClick={() => { if (mainDirectoryPath) { setInlineAction({ mode: "createDir", path: mainDirectoryPath }); setInlineInputValue(""); } }}
                disabled={!mainDirectoryPath}
                className="transition-colors p-1 rounded mr-2" style={{ color: 'var(--text-muted)' }}
                title="New Folder in main"
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
          <div className="space-y-1" role="region" aria-label="Project files">
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
              <div className="text-[10px] p-2 italic" style={{ color: 'var(--text-disabled)' }}>
                {projectDir === "." ? "No project selected" : "No main folder found"}
              </div>
            )}
          </div>
          <div className="px-2 py-1 text-[10px] truncate italic" style={{ color: 'var(--text-disabled)' }}>
            {projectDir === "." ? "No project selected" : projectDir}
          </div>

          <div className="p-2 text-sm font-medium mt-4" style={{ color: 'var(--text-muted)' }}>TOOLS</div>
          <SetupRepairButton
            isInstalling={isSettingUpEspIdf}
            onClick={() => setShowSetupModal(true)}
          />
          <button
            onClick={() => {
              const next = !showAiPanel;
              setShowAiPanel(next);
              localStorage.setItem("vibe-ai-panel", String(next));
              // Exclusive selection: opening Vibe Coder leaves the Wiki view
              if (next && activeView !== "editor") {
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
            <span className="text-neutral-400 truncate font-medium flex-1" style={{ color: 'var(--text-muted)' }}>{status.split(":")[0]}</span>
            {/* Refresh toolchain button (F5) */}
            {onRefreshToolchain && (
              <button
                onClick={onRefreshToolchain}
                title="ตรวจสอบ Toolchain อีกครั้ง (F5)"
                className="p-1 rounded transition-all active:scale-90"
                style={{ color: 'var(--text-muted)' }}
                onMouseEnter={e => { e.currentTarget.style.color = '#818cf8'; e.currentTarget.style.backgroundColor = 'rgba(99,102,241,0.12)'; }}
                onMouseLeave={e => { e.currentTarget.style.color = 'var(--text-muted)'; e.currentTarget.style.backgroundColor = ''; }}
              >
                <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
              </button>
            )}
          </div>

          {espIdfSetupNote && (
            <div className="text-[10px] leading-relaxed mb-3 rounded p-2" style={{ color: 'var(--text-muted)', border: '1px solid var(--border-color)', backgroundColor: 'var(--bg-hover)' }}>
              {espIdfSetupNote}
            </div>
          )}
          {selectedBoard === "miuaiplus" ? <div className="text-xs rounded p-3" style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)' }}>
            Python · MaixPy 4<br />Use Connect Plus and Run Python in the terminal bar.
          </div> : <>
          <button
            onClick={handleBuildFlash}
            disabled={activeView === "learning" || isBuilding || isSettingUpEspIdf || !toolchainReady || !isBuildFlashSupported(selectedBoard)}
            title={!isBuildFlashSupported(selectedBoard)
              ? MIUAI_DEPLOYMENT_UNAVAILABLE
              : activeView === "learning"
                ? "Open the code workspace and confirm the target board before Build & Flash"
              : !toolchainReady
                ? "Waiting for toolchain download to complete..."
                : "Build & Flash (Ctrl+Shift+B)"}
            className="w-full justify-center text-sm px-4 py-2 rounded-lg transition-all duration-200 font-bold flex items-center gap-2 shadow-lg active:scale-[0.98]"
            style={activeView === "learning" || isBuilding || !toolchainReady || !isBuildFlashSupported(selectedBoard)
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
            ) : (
              <>
                Build &amp; Flash
                <span style={{ fontSize: 9, opacity: 0.65, fontWeight: 500, marginLeft: 'auto', letterSpacing: '0.02em' }}>
                  ⌃⇧B
                </span>
              </>
            )}
          </button>
          </>}
          {isBuilding && <button className="w-full mt-2 text-sm rounded px-3 py-2" disabled={isCancellingBuild}
            onClick={async()=>{const ownedProject=activeBuildProjectRef.current;if(!ownedProject)return;buildCancelRequestedRef.current=true;setIsCancellingBuild(true);try{if(!sdkBuildStartedRef.current){addLog("Build cancellation requested; the current save will finish, then the SDK will not start.");return;}await invoke("cancel_sdk_command",{projectDir:ownedProject});addLog("Stopping SDK process tree...");}catch(error){toast(String(error),"error");}finally{setIsCancellingBuild(false);}}}>
            {isCancellingBuild ? "Stopping..." : "Stop build"}
          </button>}
        </div>
      </div>

      {/* Main Area */}
      <div className="flex-1 min-w-0" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        <nav className="workspace-view-nav" aria-label="Workspace views">
          <button aria-pressed={activeView === "learning"} onClick={() => setActiveView("learning")}>ห้องเรียน AI</button>
          <button aria-pressed={activeView === "editor"} onClick={() => setActiveView("editor")}>เขียนโปรแกรม</button>
        </nav>
        <div className="workspace-learning-container" hidden={activeView !== "learning"}>
          <LearningWorkspace onOpenEditor={() => setActiveView("editor")} onOpenCoder={(prompt) => {
            if (prompt?.trim()) setChatDraftRequest({ id: ++chatDraftSequence.current, text: prompt });
            setActiveView("editor"); setShowAiPanel(true); localStorage.setItem("vibe-ai-panel", "true");
          }}/>
        </div>

        {/* ── Wiki View ──────────────────────────────────────────────── */}
        {activeView === "wiki" && (
          <div style={{ flex: 1, overflow: 'hidden', display: 'flex' }}>
            <WikiView />
          </div>
        )}

        {/* ── Editor + Terminal (hidden when Wiki is active) ─────────── */}
        {activeView === "editor" && (
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
                        aria-label={`Close ${file.name}`}
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

            </div>

            {activeFile ? (
              <>
              {!isLearnerEditablePath(projectDir, activeFile.path) && (
                <div className="px-3 py-1 text-xs" role="status" style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)' }}>
                  Supporting file · Read only
                </div>
              )}
              <CodeEditor
                key={activeFile.path}
                projectDir={projectDir}
                sourceBuffers={openFiles.filter(file=>/\.[ch]$/i.test(file.path)).map(file=>({path:file.path,text:file.content}))}
                onNavigateSource={(path,line,column)=>{const snapshot=captureProject();void handleFileClick(path).then(ok=>{if(ok&&isCurrentProject(snapshot))setGotoLineRequest({path,line,column,token:Date.now()});});}}
                value={activeFile.content}
                onChange={handleEditorChange}
                filePath={activeFile.path}
                hasUnsavedChanges={activeFile.content !== activeFile.savedContent}
                onSave={() => { void saveAllFiles().catch(() => {}); }}
                readOnly={!isLearnerEditablePath(projectDir, activeFile.path)}
                isDarkMode={darkMode}
                gotoLineRequest={gotoLineRequest && normPath(gotoLineRequest.path) === normPath(activeFile.path) ? gotoLineRequest : null}
                buildErrors={buildErrors}
              />
              </>
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

              {/* Floating Build Error Panel — always on top of editor */}
              {buildResult === "failed" && (
                <BuildErrorList
                  errors={buildErrors}
                  onJumpToError={jumpToError}
                  onAskAiFix={askAiToFixErrors}
                />
              )}
          </div>
        </div>

        {/* Console & Terminal */}
        <div className="h-80 flex flex-col" style={{ borderTop: '1px solid var(--border-color)', backgroundColor: 'var(--bg-terminal)', boxShadow: 'var(--shadow-lg)' }}>
          <div className="min-h-9 flex flex-wrap items-center justify-between gap-1 px-4 py-1" style={{ borderBottom: '1px solid var(--border-color)', backgroundColor: 'var(--bg-terminal)' }}>
            <span className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--text-muted)' }}>Interactive Terminal</span>
            <div className="flex flex-wrap items-center justify-end gap-2">
              {/* Board Type Selector */}
              <select
                value={selectedBoard}
                onChange={(e) => {
                  const board = e.target.value as BoardType;
                  setSelectedBoard(board);
                  localStorage.setItem("vibe-selected-board", board);
                  // Disconnect current monitor when switching boards
                  if (isSerialConnected) {
                    toggleSerialMonitor();
                  }
                }}
                className="rounded px-2 py-1 text-[10px] w-[110px] focus:outline-none cursor-pointer font-bold"
                style={{ backgroundColor: 'var(--bg-input)', border: '1px solid var(--accent)', color: 'var(--accent)' }}
                title="Select Board Type"
              >
                <option value="kidbright32">KidBright32</option>
                <option value="miuaiplus">mAi Plus</option>
              </select>

              {/* Port/Device Controls — conditional based on board type */}
              {selectedBoard === "miuaiplus" ? (
                <UaiPlusControls projectDir={projectDir} workspaceRevision={projectEpochRef.current} saveFiles={async () => { await saveAllFiles(); }} onLog={addLog} />
              ) : (
                /* Serial Port Selector for KidBright32 */
                <>
                  <div className="flex items-center">
                    <select
                      value={selectedSerialPort}
                      onChange={(e) => setSelectedSerialPort(e.target.value)}
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
                    onClick={() => void loadSerialPorts()}
                    className="text-[10px] px-2 py-1 rounded"
                    style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)', border: '1px solid var(--border-color)' }}
                  >
                    Refresh Ports
                  </button>
                </>
              )}
              {selectedBoard === "kidbright32" && <>
              <button
                onClick={toggleSerialMonitor}
                className="text-[10px] px-2 py-1 rounded font-bold"
                style={isSerialConnected
                  ? { backgroundColor: 'var(--warning)', color: '#fff' }
                  : { backgroundColor: 'var(--success)', color: '#fff' }
                }
              >
                {isSerialConnected
                  ? "Disconnect Serial"
                  : "Connect Serial"
                }
              </button>
              </>}
              <button
                onClick={() => setLogs([])}
                className="text-[10px] transition-colors uppercase font-bold"
                style={{ color: 'var(--text-muted)' }}
              >
                Clear Logs
              </button>
            </div>
          </div>
          <div
            ref={scrollRef}
            className="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1 selection:bg-red-500/20"
            onCopy={(e) => {
              // Force plain-text copy — strip formatting/colors
              const selection = window.getSelection();
              if (selection) {
                e.preventDefault();
                e.clipboardData.setData("text/plain", selection.toString());
              }
            }}
          >
            {logs.length === 0 ? (
              <div className="italic opacity-50" style={{ color: 'var(--text-disabled)' }}>vibeKidbright Terminal Ready. Type 'idf.py --version' to test.</div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="flex gap-2 transition-colors" style={{ color: 'var(--text-secondary)' }}>
                  <span className="whitespace-pre-wrap break-all">{log}</span>
                </div>
              ))
            )}
          </div>
          {/* Terminal Input */}
          <div className="p-2 flex items-center gap-2 group" style={{ borderTop: '1px solid var(--border-color)', backgroundColor: 'var(--bg-terminal)' }}>
            <span className="font-bold text-sm ml-2" style={{ color: 'var(--accent)' }}>$</span>
            <form onSubmit={handleTerminalSubmit} className="flex-1">
              <input
                type="text"
                value={terminalInput}
                disabled={selectedBoard === "miuaiplus"}
                onChange={(e) => setTerminalInput(e.target.value)}
                onKeyDown={(e) => {
                  const history = terminalHistoryRef.current;
                  if (history.length === 0) return;
                  if (e.key === "ArrowUp") {
                    e.preventDefault();
                    const newIndex = terminalHistoryIndexRef.current === -1
                      ? history.length - 1
                      : Math.max(0, terminalHistoryIndexRef.current - 1);
                    terminalHistoryIndexRef.current = newIndex;
                    setTerminalInput(history[newIndex]);
                  } else if (e.key === "ArrowDown") {
                    e.preventDefault();
                    if (terminalHistoryIndexRef.current === -1) return;
                    const newIndex = terminalHistoryIndexRef.current + 1;
                    if (newIndex >= history.length) {
                      terminalHistoryIndexRef.current = -1;
                      setTerminalInput("");
                    } else {
                      terminalHistoryIndexRef.current = newIndex;
                      setTerminalInput(history[newIndex]);
                    }
                  }
                }}
                placeholder={selectedBoard === "miuaiplus" ? "Plus output ? use Run Python / Stop Python above" : isSerialConnected ? "Type message and press Enter to send to board..." : "Type command (e.g. idf.py) and press Enter..."}
                className="w-full bg-transparent border-none focus:outline-none font-mono text-sm"
                style={{ color: 'var(--text-primary)' }}
              />
            </form>
            <button
              onClick={sendSerialText}
              disabled={selectedBoard === "miuaiplus"}
              className="text-[10px] px-2 py-1 rounded transition-colors"
              style={{ backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)', border: '1px solid var(--border-color)' }}
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
        <div className="w-96 relative flex flex-col" style={{ borderLeft: '1px solid var(--border-color)', display: activeView === "learning" ? 'none' : undefined }}>
          <AiChat
            hasUnsavedChanges={openFiles.some(file => file.content !== file.savedContent)}
            projectDir={projectDir}
            selectedBoard={selectedBoard}
            onInjectCode={(newCode) => updateActiveFileContent(newCode)}
            onApplyToFile={handleApplyToFile}
            sendApiRef={aiChatSendRef}
            draftRequest={chatDraftRequest}
            onDraftConsumed={id => setChatDraftRequest(current => current?.id === id ? null : current)}
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
                  placeholder={selectedBoard === "miuaiplus" ? "my_plus_project" : "my_esp_project"}
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
                      {customIdfPath || <span className="text-neutral-600 italic">e.g. …/frameworks/esp-idf-v5.x</span>}
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
                      {customToolsPath || <span className="text-neutral-600 italic">e.g. …/.espressif</span>}
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
            {isMacOS() ? <MacosSdkSetup onRecheck={async () => {
              await checkEnvironment();
              onRefreshToolchain?.();
            }} /> : <div className="bg-neutral-900/60 border border-neutral-700 rounded-lg p-4 mb-5">
              <div className="flex items-center gap-2 mb-2">
                <span className="text-red-400 text-sm font-bold">⬇ Auto Install</span>
                <span className="text-[10px] text-neutral-500 bg-neutral-700 px-1.5 py-0.5 rounded">GitHub Release (~2.28 GB)</span>
              </div>
              <p className="text-[11px] text-neutral-500 mb-3">
                การติดตั้งจะตรวจ SHA-256 กับ trusted manifest ก่อนแตกไฟล์และเปิดใช้ ขณะนี้ frameworks.zip และ tools.zip ยัง unverified จึงปิดการติดตั้งไว้ โดยรักษา toolchain เดิม
              </p>
              <button
                onClick={() => { setShowSetupModal(false); handleAutoInstallGithub(); }}
                disabled={isSettingUpEspIdf}
                className="w-full py-2 bg-red-600 hover:bg-red-500 text-sm font-bold text-white rounded-lg transition-all active:scale-95 shadow-lg shadow-red-500/20"
              >
                {isSettingUpEspIdf ? "⏳ Downloading from GitHub..." : "⬇️ Start Auto Install (GitHub)"}
              </button>
            </div>}

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
          {isLearnerEditablePath(projectDir, contextMenu.path) && <button
            onClick={() => { setInlineAction({ mode: "rename", path: contextMenu.path }); setInlineInputValue(contextMenu.path.split(/[\/\\]/).pop() || ""); setContextMenu(null); }}
            className="w-full text-left px-3 py-1.5 hover:bg-red-600/20 hover:text-red-300 transition-colors flex items-center gap-2"
          >
            <span>📝</span> Rename <span className="ml-auto text-[10px] text-neutral-500 font-sans">เปลี่ยนชื่อ</span>
          </button>}
          
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

          {isLearnerEditablePath(projectDir, contextMenu.path) && <div className="my-1 border-t border-neutral-700/80"></div>}
          
          {isLearnerEditablePath(projectDir, contextMenu.path) && <button
            onClick={() => { handleDeleteItem(contextMenu.path, contextMenu.isDir); setContextMenu(null); }}
            className="w-full text-left px-3 py-1.5 hover:bg-red-500/20 text-red-400 hover:text-red-300 transition-colors flex items-center gap-2"
          >
            <span>🗑️</span> Delete <span className="ml-auto text-[10px] text-red-500/80 font-sans">ลบ</span>
          </button>}
        </div>
      )}
    </div>
  );
}

export default AppShell;
