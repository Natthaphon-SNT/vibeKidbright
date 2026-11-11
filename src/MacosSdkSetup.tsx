import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export const isMacOS = () => /Mac/.test(navigator.platform);

// Explicit Terminal instructions, separate from the managed ZIP installer.
export const MACOS_SDK_SETUP = `(
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
  echo "Run these commands in Terminal on the Mac."
  exit 1
fi
unset IDF_PATH IDF_PYTHON_ENV_PATH VIRTUAL_ENV PYTHONHOME PYTHONPATH ESP_IDF_VERSION
export IDF_TOOLS_PATH="$HOME/.vibekidbright/.espressif"
sdk="$HOME/esp/esp-idf-v5.5.1"
if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = "1" ]; then
  native=arm64
else
  native=x86_64
fi
python=""
for candidate in /opt/homebrew/bin/python3.11 /usr/local/bin/python3.11 python3 /usr/bin/python3; do
  if /usr/bin/arch "-$native" "$candidate" -c 'import sys; assert (3, 9) <= sys.version_info[:2] < (3, 14)' >/dev/null 2>&1; then
    python="$candidate"
    break
  fi
done
if [ -z "$python" ]; then
  echo "Install native Python 3.9-3.13 or Xcode Command Line Tools, then retry."
  exit 1
fi
mkdir -p "$HOME/esp"
if [ ! -d "$sdk" ]; then
  git clone --branch v5.5.1 --depth 1 --recursive --shallow-submodules https://github.com/espressif/esp-idf.git "$sdk"
fi
cd "$sdk"
git submodule update --init --recursive --depth 1
/usr/bin/arch "-$native" "$python" tools/idf_tools.py install --targets esp32
/usr/bin/arch "-$native" "$python" tools/idf_tools.py install cmake ninja
/usr/bin/arch "-$native" "$python" tools/idf_tools.py install-python-env --reinstall
echo "ESP-IDF setup complete. Return to vibeKidbright and click Check Native SDK."
)`;

export default function MacosSdkSetup({ onRecheck }: { onRecheck: () => void | Promise<void> }) {
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState("");
  const [checking, setChecking] = useState(false);
  return (
    <section aria-label="macOS ESP-IDF setup" style={{ fontSize: 12, lineHeight: 1.5 }}>
      <p>macOS ต้องใช้ Python และ compiler สำหรับ Mac โดยตรง รองรับทั้ง Apple Silicon และ Intel</p>
      <p>เปิด Terminal บน Mac แล้วรันคำสั่งด้านล่างเพื่อติดตั้ง/ซ่อม ESP-IDF แยกจาก toolchain เก่า จากนั้นกด Check Native SDK</p>
      <p>ต้องมี Git และ Python 3.9 ขึ้นไป หากยังไม่มี ให้รัน <code>xcode-select --install</code> แล้วติดตั้งให้เสร็จก่อน</p>
      <details>
        <summary style={{ cursor: "pointer" }}>คำสั่งติดตั้ง ESP-IDF สำหรับ Mac</summary>
        <textarea aria-label="macOS SDK setup commands" readOnly value={MACOS_SDK_SETUP}
          style={{ width: "100%", minHeight: 160, marginTop: 8, fontFamily: "monospace", fontSize: 11, color: "#e2e8f0", background: "#0f172a" }} />
      </details>
      <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
        <button type="button" onClick={async () => {
          try { await navigator.clipboard.writeText(MACOS_SDK_SETUP); setCopied(true); setCopyError(""); }
          catch { setCopyError("เลือกและคัดลอกคำสั่งจากช่องข้อความ แล้ววางใน Terminal"); }
        }} className="px-3 py-2 bg-blue-700 text-white rounded">{copied ? "Copied" : "Copy macOS Setup"}</button>
        <button type="button" disabled={checking} onClick={async () => {
          setChecking(true); setCopyError("");
          try { await invoke("select_native_macos_sdk"); await onRecheck(); }
          catch (error) { setCopyError(String(error)); }
          finally { setChecking(false); }
        }} className="px-3 py-2 bg-emerald-700 text-white rounded">{checking ? "Checking…" : "Check Native SDK"}</button>
      </div>
      {copyError && <p role="status">{copyError}</p>}
      <p>ใช้โฟลเดอร์โปรเจกต์ที่ไม่มีช่องว่าง และเลือกพอร์ต USB ของบอร์ด ไม่ใช้ /dev/cu.debug-console</p>
    </section>
  );
}
