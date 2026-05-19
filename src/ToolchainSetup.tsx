// ToolchainSetup.tsx
// "Happy Meal" First-Launch Toolchain Setup UI
//
// แสดงผลตอนเปิดแอปครั้งแรก หรือเมื่อ toolchain หายไป
// หน้าจอจะหายไปอัตโนมัติเมื่อ download + extract เสร็จสมบูรณ์
// ─────────────────────────────────────────────────────────────────────────────

import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ── Types ─────────────────────────────────────────────────────────────────────

interface ToolchainProgress {
  stage: "downloading" | "extracting" | "done" | "error" | "cancelled";
  percent: number;
  message: string;
}

interface ToolchainStatus {
  status: "ready" | "not_installed";
  version: string | null;
  path: string | null;
}

interface Props {
  /** เรียกเมื่อ toolchain พร้อมใช้งาน (ติดตั้งใหม่หรือมีอยู่แล้ว) */
  onReady: () => void;
  /** URL ที่จะใช้ดาวน์โหลด ZIP (optional — ถ้าว่างจะใช้ default ใน Rust) */
  toolchainUrl?: string;
}

// ── Main Component ─────────────────────────────────────────────────────────────

export default function ToolchainSetup({ onReady, toolchainUrl }: Props) {
  const [checkDone, setCheckDone] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [progress, setProgress] = useState<ToolchainProgress>({
    stage: "downloading",
    percent: 0,
    message: "Checking environment...",
  });
  const [errorMsg, setErrorMsg] = useState("");
  const [customUrl, setCustomUrl] = useState(toolchainUrl ?? "");
  const [showUrlInput, setShowUrlInput] = useState(false);
  const [logs, setLogs] = useState<string[]>([]);
  const logsRef = useRef<HTMLDivElement>(null);

  // ── Scroll logs to bottom ────────────────────────────────────────────────
  useEffect(() => {
    if (logsRef.current) {
      logsRef.current.scrollTop = logsRef.current.scrollHeight;
    }
  }, [logs]);

  // ── Listen for progress events from Rust ────────────────────────────────
  useEffect(() => {
    const unlisten = listen<ToolchainProgress>("toolchain-progress", (event) => {
      const p = event.payload;
      setProgress(p);
      setLogs((prev) => [...prev, `[${p.stage.toUpperCase()}] ${p.message}`]);

      if (p.stage === "done") {
        setTimeout(() => onReady(), 800); // brief pause to show 100%
      }
      if (p.stage === "error" || p.stage === "cancelled") {
        setIsDownloading(false);
        setErrorMsg(p.message);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [onReady]);

  // ── Check toolchain status on mount ─────────────────────────────────────
  useEffect(() => {
    checkToolchain();
  }, []);

  const checkToolchain = async () => {
    try {
      const status = (await invoke("check_toolchain")) as ToolchainStatus;
      setCheckDone(true);

      if (status.status === "ready") {
        // Toolchain มีอยู่แล้ว → ผ่านไปเลย
        setProgress({ stage: "done", percent: 100, message: `Toolchain v${status.version} ready.` });
        setTimeout(() => onReady(), 500);
      }
      // ถ้า not_installed → รอให้ผู้ใช้กดปุ่ม
    } catch (err) {
      setCheckDone(true);
      setErrorMsg(String(err));
    }
  };

  const startDownload = async () => {
    if (isDownloading) return;
    setErrorMsg("");
    setIsDownloading(true);
    setLogs([]);
    setProgress({ stage: "downloading", percent: 0, message: "Starting download..." });

    try {
      await invoke("download_toolchain", {
        url: customUrl.trim() || null,
      });
      // onReady จะถูกเรียกจาก event listener ตอน stage === "done"
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      setProgress({ stage: "error", percent: 0, message: msg });
      setIsDownloading(false);
    }
  };

  const cancelDownload = async () => {
    await invoke("cancel_toolchain_download").catch(() => {});
    setIsDownloading(false);
  };

  // ── Derived state ────────────────────────────────────────────────────────
  const isReady = progress.stage === "done";
  const isError = progress.stage === "error" || !!errorMsg;
  const isCancelled = progress.stage === "cancelled";

  const stageLabel: Record<string, string> = {
    downloading: "📥 Downloading toolchain...",
    extracting: "📦 Extracting files...",
    done: "✅ Ready!",
    error: "❌ Error",
    cancelled: "⛔ Cancelled",
  };

  const progressBarColor =
    isError || isCancelled
      ? "linear-gradient(90deg, #ef4444, #dc2626)"
      : isReady
      ? "linear-gradient(90deg, #22c55e, #16a34a)"
      : "linear-gradient(90deg, #3b82f6, #1d4ed8, #6366f1)";

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "linear-gradient(135deg, #0a0a12 0%, #0d1117 50%, #0a0f1a 100%)",
        fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
      }}
    >
      {/* Animated background dots */}
      <div style={{ position: "absolute", inset: 0, overflow: "hidden", pointerEvents: "none" }}>
        {[...Array(20)].map((_, i) => (
          <div
            key={i}
            style={{
              position: "absolute",
              borderRadius: "50%",
              background: `rgba(59,130,246,${0.03 + Math.random() * 0.06})`,
              width: `${60 + Math.random() * 200}px`,
              height: `${60 + Math.random() * 200}px`,
              left: `${Math.random() * 100}%`,
              top: `${Math.random() * 100}%`,
              animation: `pulse ${3 + Math.random() * 4}s ease-in-out infinite`,
              animationDelay: `${Math.random() * 4}s`,
            }}
          />
        ))}
      </div>

      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap');
        @keyframes pulse { 0%,100% { transform: scale(1); opacity: 0.5; } 50% { transform: scale(1.1); opacity: 1; } }
        @keyframes shimmer { 0% { background-position: -200% center; } 100% { background-position: 200% center; } }
        @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(16px); } to { opacity: 1; transform: translateY(0); } }
        @keyframes progress-shimmer {
          0% { background-position: 200% center; }
          100% { background-position: -200% center; }
        }
        .progress-bar-inner {
          background-size: 200% auto;
          animation: progress-shimmer 2s linear infinite;
        }
        .log-entry { animation: fadeIn 0.2s ease forwards; }
        .setup-card { animation: fadeIn 0.4s ease forwards; }
        .spinner { animation: spin 1s linear infinite; }
      `}</style>

      {/* Main card */}
      <div
        className="setup-card"
        style={{
          width: "min(520px, 92vw)",
          background: "rgba(15,20,30,0.95)",
          border: "1px solid rgba(59,130,246,0.2)",
          borderRadius: "20px",
          padding: "40px",
          boxShadow: "0 0 80px rgba(59,130,246,0.15), 0 32px 64px rgba(0,0,0,0.5)",
          backdropFilter: "blur(20px)",
          position: "relative",
        }}
      >
        {/* Header */}
        <div style={{ textAlign: "center", marginBottom: "32px" }}>
          {/* Logo / Icon */}
          <div
            style={{
              width: "72px",
              height: "72px",
              margin: "0 auto 20px",
              borderRadius: "20px",
              background: "linear-gradient(135deg, #1e40af, #3b82f6, #6366f1)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              boxShadow: "0 8px 32px rgba(59,130,246,0.4)",
              fontSize: "36px",
            }}
          >
            🤖
          </div>

          <h1
            style={{
              fontSize: "22px",
              fontWeight: 700,
              color: "#f0f6ff",
              margin: "0 0 8px",
              letterSpacing: "-0.5px",
            }}
          >
            VibeKidbright
          </h1>
          <p style={{ fontSize: "14px", color: "rgba(148,163,184,0.8)", margin: 0, lineHeight: 1.5 }}>
            {isReady
              ? "🎉 Compiler toolchain is ready to use!"
              : isError
              ? "Setup encountered an error. Please try again."
              : isDownloading
              ? "Setting up the C compiler toolchain for ESP32..."
              : "The ESP32 compiler toolchain needs to be installed.\nThis only happens once on first launch."}
          </p>
        </div>

        {/* Progress Section (shown during download or when done) */}
        {(isDownloading || isReady || isError || isCancelled) && (
          <div style={{ marginBottom: "24px" }}>
            {/* Stage label */}
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: "10px",
              }}
            >
              <span style={{ fontSize: "13px", color: "#94a3b8", display: "flex", alignItems: "center", gap: "8px" }}>
                {isDownloading && (
                  <svg className="spinner" width="14" height="14" viewBox="0 0 24 24" fill="none">
                    <circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.3)" strokeWidth="3" />
                    <path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round" />
                  </svg>
                )}
                {stageLabel[progress.stage] || progress.stage}
              </span>
              <span
                style={{
                  fontSize: "13px",
                  fontWeight: 600,
                  color: isError ? "#ef4444" : isReady ? "#22c55e" : "#60a5fa",
                  fontFamily: "'JetBrains Mono', monospace",
                }}
              >
                {progress.percent}%
              </span>
            </div>

            {/* Progress bar track */}
            <div
              style={{
                height: "8px",
                borderRadius: "100px",
                background: "rgba(30,40,60,0.8)",
                border: "1px solid rgba(59,130,246,0.1)",
                overflow: "hidden",
              }}
            >
              <div
                className="progress-bar-inner"
                style={{
                  height: "100%",
                  width: `${progress.percent}%`,
                  borderRadius: "100px",
                  background: progressBarColor,
                  transition: "width 0.3s ease",
                }}
              />
            </div>

            {/* Status message */}
            <p
              style={{
                fontSize: "12px",
                color: isError ? "#f87171" : "rgba(148,163,184,0.7)",
                margin: "8px 0 0",
                fontFamily: "'JetBrains Mono', monospace",
                lineHeight: 1.4,
                wordBreak: "break-all",
              }}
            >
              {progress.message}
            </p>
          </div>
        )}

        {/* Custom URL input (collapsed by default) */}
        {!isDownloading && !isReady && (
          <div style={{ marginBottom: "20px" }}>
            <button
              onClick={() => setShowUrlInput((v) => !v)}
              style={{
                background: "none",
                border: "none",
                color: "rgba(96,165,250,0.7)",
                fontSize: "12px",
                cursor: "pointer",
                padding: "4px 0",
                display: "flex",
                alignItems: "center",
                gap: "6px",
              }}
            >
              <span style={{ transform: showUrlInput ? "rotate(90deg)" : "none", display: "inline-block", transition: "transform 0.2s" }}>▶</span>
              Use custom toolchain URL
            </button>

            {showUrlInput && (
              <div style={{ marginTop: "10px" }}>
                <label style={{ fontSize: "12px", color: "#64748b", display: "block", marginBottom: "6px" }}>
                  Toolchain ZIP URL
                </label>
                <input
                  id="toolchain-url-input"
                  type="url"
                  placeholder="https://your-server.com/kb_compiler_v1.zip"
                  value={customUrl}
                  onChange={(e) => setCustomUrl(e.target.value)}
                  style={{
                    width: "100%",
                    boxSizing: "border-box",
                    background: "rgba(10,15,25,0.8)",
                    border: "1px solid rgba(59,130,246,0.25)",
                    borderRadius: "10px",
                    padding: "10px 14px",
                    fontSize: "12px",
                    color: "#e2e8f0",
                    outline: "none",
                    fontFamily: "'JetBrains Mono', monospace",
                  }}
                />
              </div>
            )}
          </div>
        )}

        {/* Error details */}
        {isError && errorMsg && (
          <div
            style={{
              background: "rgba(239,68,68,0.08)",
              border: "1px solid rgba(239,68,68,0.25)",
              borderRadius: "10px",
              padding: "12px 16px",
              marginBottom: "20px",
            }}
          >
            <p style={{ fontSize: "12px", color: "#f87171", margin: 0, wordBreak: "break-all", lineHeight: 1.5, fontFamily: "'JetBrains Mono', monospace" }}>
              {errorMsg}
            </p>
          </div>
        )}

        {/* Live log (shown during download) */}
        {(isDownloading || (logs.length > 0 && !isReady)) && (
          <div
            ref={logsRef}
            style={{
              background: "rgba(5,10,20,0.6)",
              border: "1px solid rgba(59,130,246,0.12)",
              borderRadius: "10px",
              padding: "12px",
              maxHeight: "120px",
              overflowY: "auto",
              marginBottom: "20px",
              scrollbarWidth: "thin",
              scrollbarColor: "rgba(59,130,246,0.3) transparent",
            }}
          >
            {logs.map((log, i) => (
              <p
                key={i}
                className="log-entry"
                style={{
                  margin: "2px 0",
                  fontSize: "11px",
                  color: log.includes("ERROR") ? "#f87171" : "rgba(148,163,184,0.7)",
                  fontFamily: "'JetBrains Mono', monospace",
                  lineHeight: 1.4,
                }}
              >
                {log}
              </p>
            ))}
          </div>
        )}

        {/* Action buttons */}
        <div style={{ display: "flex", gap: "12px" }}>
          {/* Primary button */}
          {!isReady && (
            <>
              {isDownloading ? (
                <button
                  id="btn-cancel-download"
                  onClick={cancelDownload}
                  style={{
                    flex: 1,
                    padding: "14px",
                    borderRadius: "12px",
                    border: "1px solid rgba(239,68,68,0.3)",
                    background: "rgba(239,68,68,0.08)",
                    color: "#f87171",
                    fontSize: "14px",
                    fontWeight: 600,
                    cursor: "pointer",
                    transition: "all 0.2s",
                  }}
                >
                  ⛔ Cancel Download
                </button>
              ) : (
                <button
                  id="btn-start-download"
                  onClick={startDownload}
                  disabled={!checkDone}
                  style={{
                    flex: 1,
                    padding: "14px",
                    borderRadius: "12px",
                    border: "none",
                    background:
                      !checkDone
                        ? "rgba(30,40,60,0.5)"
                        : "linear-gradient(135deg, #1d4ed8, #3b82f6, #6366f1)",
                    color: !checkDone ? "#64748b" : "white",
                    fontSize: "14px",
                    fontWeight: 700,
                    cursor: !checkDone ? "not-allowed" : "pointer",
                    boxShadow: checkDone ? "0 4px 24px rgba(59,130,246,0.4)" : "none",
                    transition: "all 0.2s",
                    letterSpacing: "0.2px",
                  }}
                >
                  {isError || isCancelled ? "🔄 Try Again" : "⬇️ Install Toolchain (1-time)"}
                </button>
              )}
            </>
          )}

          {isReady && (
            <button
              id="btn-enter-app"
              onClick={onReady}
              style={{
                flex: 1,
                padding: "14px",
                borderRadius: "12px",
                border: "none",
                background: "linear-gradient(135deg, #15803d, #22c55e)",
                color: "white",
                fontSize: "14px",
                fontWeight: 700,
                cursor: "pointer",
                boxShadow: "0 4px 24px rgba(34,197,94,0.4)",
                transition: "all 0.2s",
              }}
            >
              🚀 Enter VibeKidbright
            </button>
          )}
        </div>

        {/* Skip / Already installed note */}
        {!isDownloading && !isReady && (
          <p style={{ textAlign: "center", marginTop: "16px", fontSize: "12px", color: "#334155" }}>
            Download size: ~1 GB · Extracted to AppData · One-time only
          </p>
        )}
      </div>
    </div>
  );
}
