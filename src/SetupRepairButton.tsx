interface SetupRepairButtonProps {
  isInstalling: boolean;
  onClick: () => void;
}

export default function SetupRepairButton({
  isInstalling,
  onClick,
}: SetupRepairButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={isInstalling}
      className="w-full text-left p-2 rounded flex items-center gap-2 text-sm transition-colors group"
      style={isInstalling
        ? { backgroundColor: "rgba(245,158,11,0.1)", color: "#fcd34d", cursor: "not-allowed" }
        : { color: "var(--text-secondary)" }}
      onMouseEnter={(event) => {
        if (!isInstalling) event.currentTarget.style.backgroundColor = "var(--bg-hover)";
      }}
      onMouseLeave={(event) => {
        if (!isInstalling) event.currentTarget.style.backgroundColor = "";
      }}
    >
      <span
        className="w-4 h-4 flex items-center justify-center rounded text-[10px] font-bold"
        style={isInstalling
          ? { backgroundColor: "rgba(245,158,11,0.2)", color: "var(--warning)" }
          : { backgroundColor: "var(--bg-hover)", color: "var(--text-muted)" }}
      >
        {isInstalling ? "…" : "⚙"}
      </span>
      {isInstalling ? "Installing ESP-IDF..." : "Setup / Repair ESP-IDF"}
    </button>
  );
}
