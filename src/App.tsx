import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import AiChat from "./AiChat";
import "./App.css";

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children?: FileEntry[];
}

function FileTreeItem({
  item,
  activeFile,
  openFolders,
  onFileClick,
  onFolderToggle
}: {
  item: FileEntry;
  activeFile: string;
  openFolders: Set<string>;
  onFileClick: (path: string) => void;
  onFolderToggle: (path: string) => void;
}) {
  const isOpen = openFolders.has(item.path);

  if (item.is_dir) {
    return (
      <div className="space-y-1">
        <button
          onClick={() => onFolderToggle(item.path)}
          className="w-full text-left p-1.5 hover:bg-slate-800/50 rounded flex items-center gap-2 text-sm transition-colors text-slate-400 group"
        >
          <span className={`transition-transform duration-200 ${isOpen ? "rotate-90" : ""}`}>
            ▶
          </span>
          <span className="w-4 h-4 text-sky-400 flex items-center justify-center rounded text-[10px]">📁</span>
          {item.name}
        </button>
        {isOpen && item.children && (
          <div className="pl-4 space-y-1 border-l border-slate-800 ml-3">
            {item.children.map((child) => (
              <FileTreeItem
                key={child.path}
                item={child}
                activeFile={activeFile}
                openFolders={openFolders}
                onFileClick={onFileClick}
                onFolderToggle={onFolderToggle}
              />
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <button
      onClick={() => onFileClick(item.path)}
      className={`w-full text-left p-1.5 rounded flex items-center gap-2 text-sm transition-colors group ${activeFile === item.path ? "bg-sky-500/10 text-sky-400" : "hover:bg-slate-800/50 text-slate-300"
        }`}
    >
      <span className="w-4 h-4 bg-sky-500/20 text-sky-400 flex items-center justify-center rounded text-[10px] group-hover:bg-sky-500/30">
        {item.name.endsWith(".c") ? "C" : item.name.endsWith(".h") ? "H" : "F"}
      </span>
      {item.name}
    </button>
  );
}

interface FileTab {
  name: string;
  path: string;
  content: string;
}

function App() {
  const [status, setStatus] = useState("Checking ESP-IDF...");
  const [isSettingUpEspIdf, setIsSettingUpEspIdf] = useState(false);
  const [espIdfSetupNote, setEspIdfSetupNote] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [terminalInput, setTerminalInput] = useState("");
  const [openFiles, setOpenFiles] = useState<FileTab[]>([]);
  const [activeFilePath, setActiveFilePath] = useState<string>("");
  const [showAiPanel, setShowAiPanel] = useState(true);
  const [projectDir, setProjectDir] = useState(".");
  const [isBuilding, setIsBuilding] = useState(false);
  const [serialPorts, setSerialPorts] = useState<string[]>([]);
  const [selectedSerialPort, setSelectedSerialPort] = useState("");
  const [serialBaud, setSerialBaud] = useState("115200");
  const [isSerialConnected, setIsSerialConnected] = useState(false);
  const [showNewProjectModal, setShowNewProjectModal] = useState(false);
  const [newProjectName, setNewProjectName] = useState("my_esp_project");
  const [newProjectPath, setNewProjectPath] = useState("");
  const [projectFiles, setProjectFiles] = useState<FileEntry[]>([]);
  const [openFolders, setOpenFolders] = useState<Set<string>>(new Set());
  const scrollRef = useRef<HTMLDivElement>(null);

  const activeFile = openFiles.find(f => f.path === activeFilePath);

  useEffect(() => {
    checkEnvironment();
    loadSerialPorts();

    const unlistenTerminal = listen("terminal-output", (event) => {
      setLogs((prev) => [...prev, event.payload as string]);
    });

    const unlistenFile = listen("file-modified", async (event) => {
      const { path } = JSON.parse(event.payload as string);
      const isOpen = openFiles.find(f => f.path === path);
      if (isOpen) {
        reloadFile(path);
      }
      loadProjectFiles();
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        saveActiveFile();
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlistenTerminal.then((f) => f());
      unlistenFile.then((f) => f());
      window.removeEventListener("keydown", handleKeyDown);
      invoke("stop_serial_monitor").catch(() => null);
    };
  }, [activeFilePath, openFiles]);

  const reloadFile = async (path: string) => {
    try {
      const content = await invoke("read_project_file", { path });
      setOpenFiles(prev => prev.map(f =>
        f.path === path ? { ...f, content: content as string } : f
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

  const saveActiveFile = useCallback(async () => {
    if (!activeFile) return;
    try {
      await invoke("write_project_file", {
        path: activeFile.path,
        content: activeFile.content
      });
      addLog(`Saved: ${activeFile.name}`);
    } catch (err) {
      addLog(`❌ Failed to save: ${err}`);
    }
  }, [activeFile]);

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
      setSerialPorts(list);
      if (list.length > 0 && !selectedSerialPort) {
        setSelectedSerialPort(list[0]);
      }
    } catch (err) {
      addLog(`❌ Failed to list serial ports: ${err}`);
    }
  };

  const handleFileClick = async (path: string) => {
    // Check if file is already open
    const existing = openFiles.find(f => f.path === path);
    if (existing) {
      setActiveFilePath(path);
      return;
    }

    try {
      const content = await invoke("read_project_file", { path });
      const newTab: FileTab = {
        name: path.split("/").pop() || "unknown",
        path,
        content: content as string
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
    setOpenFiles(prev => {
      const next = prev.filter(f => f.path !== path);
      if (activeFilePath === path) {
        setActiveFilePath(next.length > 0 ? next[next.length - 1].path : "");
      }
      return next;
    });
  };

  const updateActiveFileContent = (newContent: string) => {
    if (!activeFilePath) {
      console.warn("Attempted to update content but no file is active.");
      return;
    }
    setOpenFiles(prev => prev.map(f =>
      f.path === activeFilePath ? { ...f, content: newContent } : f
    ));
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

  const runEspIdfSetup = async () => {
    if (isSettingUpEspIdf) return;

    setIsSettingUpEspIdf(true);
    setEspIdfSetupNote("Installing ESP-IDF toolchain for this OS...");
    setStatus("Setting up ESP-IDF...");
    addLog("Starting first-run ESP-IDF setup...");

    try {
      const result = await invoke("setup_esp_idf", {
        version: "v5.2.2",
        targets: ["esp32", "esp32s2", "esp32s3", "esp32c3", "esp32c6"]
      });
      addLog(`${result}`);
      setEspIdfSetupNote("ESP-IDF installed successfully.");
      await checkEnvironment();
    } catch (err) {
      const message = `ESP-IDF setup failed: ${err}`;
      setStatus("ESP-IDF setup failed");
      setEspIdfSetupNote("Setup failed. Check logs and retry.");
      addLog(`❌ ${message}`);
    } finally {
      setIsSettingUpEspIdf(false);
    }
  };

  const checkEnvironment = async () => {
    try {
      const result = await invoke("check_esp_idf");
      setStatus(result as string);
      setEspIdfSetupNote("");
    } catch (err) {
      setStatus("ESP-IDF not found");
      setEspIdfSetupNote("ESP-IDF is required. Starting bootstrap installer...");
      addLog("ESP-IDF not found. Running bootstrap installer...");
      await runEspIdfSetup();
      console.error(err);
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
      const payload = terminalInput.endsWith("\n") ? terminalInput : `${terminalInput}\n`;
      await invoke("send_serial_input", { input: payload });
      addLog(`[SERIAL TX] ${terminalInput}`);
      setTerminalInput("");
    } catch (err) {
      addLog(`❌ Failed to send serial input: ${err}`);
    }
  };

  const addLog = (msg: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  };

  const handleTerminalSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!terminalInput.trim()) return;

    const parts = terminalInput.trim().split(" ");
    const cmd = parts[0];
    const args = parts.slice(1);

    addLog(`> ${terminalInput}`);
    setTerminalInput("");

    try {
      await invoke("run_shell_command", {
        cmd,
        args,
        cwd: projectDir === "." ? null : projectDir
      });
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
    setIsBuilding(true);
    addLog("--- Starting Build & Flash ---");

    try {
      // Run build/flash in the identified project directory
      await invoke("run_shell_command", {
        cmd: "idf.py",
        args: ["build", "flash"],
        cwd: projectDir === "." ? null : projectDir
      });
    } catch (err) {
      addLog(`Build failed: ${err}`);
    } finally {
      setIsBuilding(false);
    }
  };

  return (
    <div className="flex h-screen w-full overflow-hidden bg-slate-950 text-slate-200">
      {/* Sidebar */}
      <div className="w-64 flex flex-col border-r border-slate-800 bg-slate-900/50 backdrop-blur-md font-sans">
        <div className="p-4 border-b border-slate-800">
          <h1 className="text-xl font-bold bg-gradient-to-r from-sky-400 to-indigo-500 bg-clip-text text-transparent">
            vibeKidbright
          </h1>
          <p className="text-xs text-slate-500 mt-1 uppercase tracking-widest font-semibold">ESP-IDF IDE</p>
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          <div className="flex items-center justify-between p-2">
            <span className="text-sm font-medium text-slate-400">PROJECT</span>
            <div className="flex gap-1">
              <button
                onClick={handleOpenProject}
                className="text-[10px] bg-slate-800 hover:bg-slate-700 text-sky-400 px-1.5 py-0.5 rounded transition-colors border border-sky-400/30"
              >
                OPEN
              </button>
              <button
                onClick={handleNewProject}
                className="text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 px-1.5 py-0.5 rounded transition-colors"
              >
                NEW
              </button>
            </div>
          </div>
          <div className="space-y-1">
            {projectFiles.map((file) => (
              <FileTreeItem
                key={file.path}
                item={file}
                activeFile={activeFilePath}
                openFolders={openFolders}
                onFileClick={handleFileClick}
                onFolderToggle={toggleFolder}
              />
            ))}
            {projectFiles.length === 0 && (
              <div className="text-[10px] text-slate-700 p-2 italic">
                No files found
              </div>
            )}
          </div>
          <div className="px-2 py-1 text-[10px] text-slate-600 truncate italic">
            {projectDir === "." ? "No project selected" : projectDir}
          </div>

          <div className="p-2 text-sm font-medium text-slate-400 mt-4">TOOLS</div>
          <button
            onClick={runEspIdfSetup}
            disabled={isSettingUpEspIdf}
            className={`w-full text-left p-2 rounded flex items-center gap-2 text-sm transition-colors group ${isSettingUpEspIdf
              ? "bg-amber-500/10 text-amber-300 cursor-not-allowed"
              : "hover:bg-slate-800/50 text-slate-300"
              }`}
          >
            <span className={`w-4 h-4 flex items-center justify-center rounded text-[10px] font-bold ${isSettingUpEspIdf ? "bg-amber-400/30 text-amber-200" : "bg-slate-700 text-slate-300"}`}>
              {isSettingUpEspIdf ? "…" : "⚙"}
            </span>
            {isSettingUpEspIdf ? "Installing ESP-IDF..." : "Setup / Repair ESP-IDF"}
          </button>
          <button
            onClick={() => setShowAiPanel(!showAiPanel)}
            className={`w-full text-left p-2 rounded flex items-center gap-2 text-sm transition-colors group ${showAiPanel ? "bg-violet-500/10 text-violet-300" : "hover:bg-slate-800/50 text-slate-400"
              }`}
          >
            <span className={`w-4 h-4 flex items-center justify-center rounded text-[10px] font-bold ${showAiPanel ? "bg-violet-500/30 text-violet-300" : "bg-slate-700 text-slate-400 group-hover:bg-slate-600"
              }`}>✦</span>
            Vibe Coder
          </button>
        </div>

        <div className="p-4 border-t border-slate-800 bg-slate-900/50">
          <div className="flex items-center gap-2 text-xs mb-3">
            <div className={`w-2 h-2 rounded-full ${status.includes("Ready") ? "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.6)]" : "bg-amber-500"}`}></div>
            <span className="text-slate-400 truncate font-medium">{status.split(":")[0]}</span>
          </div>
          {espIdfSetupNote && (
            <div className="text-[10px] leading-relaxed text-slate-500 mb-3 rounded border border-slate-800 bg-slate-900/70 p-2">
              {espIdfSetupNote}
            </div>
          )}
          <button
            onClick={handleBuildFlash}
            disabled={isBuilding || isSettingUpEspIdf}
            className={`w-full justify-center text-sm px-4 py-2 rounded-lg transition-all duration-200 font-bold flex items-center gap-2 shadow-lg ${isBuilding
              ? "bg-slate-700 text-slate-500 cursor-not-allowed"
              : "bg-sky-500 hover:bg-sky-600 active:scale-[0.98] text-white shadow-sky-500/20"
              }`}
          >
            {isBuilding ? (
              <>
                <div className="w-3 h-3 border-2 border-slate-500 border-t-slate-300 rounded-full animate-spin" />
                Building...
              </>
            ) : "Build & Flash"}
          </button>
        </div>
      </div>

      {/* Main Area */}
      <div className="flex-1 flex flex-col min-w-0">
        <div className="flex-1 bg-slate-900 overflow-hidden relative">
          <div className="absolute inset-0 flex flex-col">
            {/* Tab Bar */}
            <div className="h-10 border-b border-slate-800 flex items-center justify-between bg-slate-900/80 backdrop-blur-sm z-10 overflow-hidden">
              <div className="flex items-center overflow-x-auto no-scrollbar flex-1">
                {openFiles.map((file) => (
                  <div
                    key={file.path}
                    onClick={() => setActiveFilePath(file.path)}
                    className={`flex items-center gap-2 px-4 h-full border-r border-slate-800 cursor-pointer transition-colors text-xs font-medium whitespace-nowrap ${activeFilePath === file.path
                      ? "bg-slate-950 text-sky-400 border-b-2 border-b-sky-400"
                      : "text-slate-500 hover:text-slate-300 hover:bg-slate-800/30"
                      }`}
                  >
                    <span>{file.name}</span>
                    <button
                      onClick={(e) => closeFile(file.path, e)}
                      className="hover:text-red-400 transition-colors p-0.5 rounded-sm ml-1"
                    >
                      ×
                    </button>
                  </div>
                ))}
                {openFiles.length === 0 && (
                  <div className="px-4 text-xs text-slate-600 italic">No files open</div>
                )}
              </div>

              {activeFile && (
                <div className="flex items-center gap-2 px-3 border-l border-slate-800 shrink-0 h-full bg-slate-900/40">
                  <button
                    onClick={() => reloadFile(activeFile.path)}
                    className="p-1.5 text-slate-500 hover:text-sky-400 transition-colors rounded hover:bg-slate-800"
                    title="Reload from disk"
                  >
                    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                  </button>
                  <button
                    onClick={saveActiveFile}
                    className="flex items-center gap-1.5 px-3 py-1 bg-sky-600/20 hover:bg-sky-600 text-[10px] font-bold text-sky-400 hover:text-white rounded transition-all active:scale-95 uppercase tracking-wider"
                  >
                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
                    </svg>
                    Save
                  </button>
                </div>
              )}
            </div>
            <textarea
              value={activeFile?.content || ""}
              onChange={(e) => updateActiveFileContent(e.target.value)}
              className="flex-1 w-full bg-slate-950 p-6 font-mono text-sm resize-none focus:outline-none focus:ring-1 focus:ring-sky-500/20 selection:bg-sky-500/30 text-slate-300 leading-relaxed"
              spellCheck={false}
              placeholder={projectDir === "." ? "Open or create a project to start coding..." : "Select a file to edit..."}
            />
          </div>
        </div>

        {/* Console & Terminal */}
        <div className="h-80 border-t border-slate-800 bg-slate-950 flex flex-col shadow-2xl">
          <div className="h-9 border-b border-slate-800 flex items-center justify-between px-4 bg-slate-900/40">
            <span className="text-[10px] font-bold text-slate-500 uppercase tracking-[0.2em]">Interactive Terminal</span>
            <div className="flex items-center gap-2">
              <select
                value={selectedSerialPort}
                onChange={(e) => setSelectedSerialPort(e.target.value)}
                className="bg-slate-900 border border-slate-700 rounded px-2 py-1 text-[10px] text-slate-300 max-w-[180px]"
              >
                {serialPorts.length === 0 ? (
                  <option value="">No serial ports</option>
                ) : (
                  serialPorts.map((port) => (
                    <option key={port} value={port}>{port}</option>
                  ))
                )}
              </select>
              <input
                type="text"
                value={serialBaud}
                onChange={(e) => setSerialBaud(e.target.value)}
                className="w-20 bg-slate-900 border border-slate-700 rounded px-2 py-1 text-[10px] text-slate-300"
              />
              <button
                onClick={loadSerialPorts}
                className="text-[10px] px-2 py-1 rounded bg-slate-800 text-slate-300 hover:bg-slate-700"
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
                className="text-[10px] text-slate-500 hover:text-slate-300 transition-colors uppercase font-bold"
              >
                Clear Logs
              </button>
            </div>
          </div>
          <div
            ref={scrollRef}
            className="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1 selection:bg-sky-500/20"
          >
            {logs.length === 0 ? (
              <div className="text-slate-700 italic opacity-50">vibeKidbright Terminal Ready. Type 'idf.py --version' to test.</div>
            ) : (
              logs.map((log, i) => (
                <div key={i} className="flex gap-2 text-slate-400/90 hover:text-slate-200 transition-colors">
                  <span className="whitespace-pre-wrap break-all">{log}</span>
                </div>
              ))
            )}
          </div>
          {/* Terminal Input */}
          <div className="p-2 bg-slate-900/40 border-t border-slate-800 flex items-center gap-2 group">
            <span className="text-sky-500 font-bold text-sm ml-2">$</span>
            <form onSubmit={handleTerminalSubmit} className="flex-1">
              <input
                type="text"
                value={terminalInput}
                onChange={(e) => setTerminalInput(e.target.value)}
                placeholder="Type command and press Enter..."
                className="w-full bg-transparent border-none focus:outline-none font-mono text-sm text-slate-200 placeholder:text-slate-600"
              />
            </form>
            <button
              onClick={sendSerialText}
              className="text-[10px] px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300"
            >
              Send Serial
            </button>
          </div>
        </div>
      </div>

      {/* AI Chat Panel */}
      {showAiPanel && (
        <div className="w-96 border-l border-slate-800 relative flex flex-col">
          <AiChat
            projectDir={projectDir}
            onInjectCode={(newCode) => updateActiveFileContent(newCode)}
          />
        </div>
      )}

      {/* New Project Modal */}
      {showNewProjectModal && (
        <div className="absolute inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
          <div className="bg-slate-800 border border-slate-700 rounded-xl p-6 w-96 shadow-2xl">
            <h3 className="text-lg font-bold text-slate-200 mb-4 flex items-center gap-2">
              <span className="text-sky-400">📁</span> Create New Project
            </h3>

            <div className="space-y-4">
              <div>
                <label className="text-xs text-slate-400 mb-1 block uppercase font-bold tracking-wider">
                  Project Name
                </label>
                <input
                  type="text"
                  value={newProjectName}
                  onChange={(e) => setNewProjectName(e.target.value)}
                  placeholder="my_esp_project"
                  className="w-full bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-sm text-slate-200 focus:outline-none focus:border-sky-500 transition-colors"
                />
              </div>

              <div>
                <label className="text-xs text-slate-400 mb-1 block uppercase font-bold tracking-wider">
                  Location
                </label>
                <div className="flex gap-2">
                  <div className="flex-1 bg-slate-900 border border-slate-600 rounded-lg px-3 py-2 text-xs text-slate-400 truncate flex items-center">
                    {newProjectPath || "No directory selected"}
                  </div>
                  <button
                    onClick={handlePickDirectory}
                    className="px-3 py-2 bg-slate-700 hover:bg-slate-600 text-xs text-slate-200 rounded-lg transition-colors shrink-0"
                  >
                    Browse
                  </button>
                </div>
              </div>
            </div>

            <div className="flex gap-3 mt-8">
              <button
                onClick={() => setShowNewProjectModal(false)}
                className="flex-1 py-2 bg-slate-700 hover:bg-slate-600 text-sm text-slate-300 rounded-lg transition-colors font-medium"
              >
                Cancel
              </button>
              <button
                onClick={confirmCreateProject}
                disabled={!newProjectName || !newProjectPath}
                className={`flex-1 py-2 rounded-lg transition-all font-bold text-sm ${!newProjectName || !newProjectPath
                  ? "bg-slate-700 text-slate-500 cursor-not-allowed"
                  : "bg-sky-500 hover:bg-sky-400 text-white shadow-lg shadow-sky-500/20 active:scale-95"
                  }`}
              >
                Create Project
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
