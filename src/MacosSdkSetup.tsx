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
root="$HOME/.vibekidbright/macos-sdk"
case "$root" in
  *[[:space:]]*) echo "ESP-IDF needs a home/SDK path without spaces."; exit 1 ;;
esac
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
git --version >/dev/null
mkdir -p "$root"
# Install at its final location: Python venvs must not be moved after creation.
# Every run is clean. Older installs remain available until this one succeeds.
install="$(mktemp -d "$root/install-XXXXXXXX")"
sdk="$install/esp-idf"
export IDF_TOOLS_PATH="$install/.espressif"
git clone --branch v5.5.1 --depth 1 --recursive --shallow-submodules https://github.com/espressif/esp-idf.git "$sdk"
cd "$sdk"
git submodule update --init --recursive --depth 1
/usr/bin/arch "-$native" "$python" tools/idf_tools.py install --targets esp32
/usr/bin/arch "-$native" "$python" tools/idf_tools.py install cmake ninja
/usr/bin/arch "-$native" "$python" tools/idf_tools.py install-python-env --reinstall
/usr/bin/arch "-$native" "$python" - "$root" "$install" <<'PY'
import json, os, pathlib, subprocess, sys
root, install = map(pathlib.Path, sys.argv[1:])
assert install.parent.resolve() == root.resolve()
environments = sorted((install / '.espressif/python_env').glob('idf5.5_py*_env'))
assert environments, 'The native ESP-IDF Python environment was not created'
venv = environments[-1]
env = dict(os.environ, IDF_PATH=str(install / 'esp-idf'), IDF_PYTHON_ENV_PATH=str(venv), VIRTUAL_ENV=str(venv))
subprocess.run([str(venv / 'bin/python'), str(install / 'esp-idf/tools/idf.py'), '--version'], env=env, check=True)
# Store only a relative directory name; derive the user's home on this Mac.
pending = root / (install.name + '.json')
pending.write_text(json.dumps({'directory': install.name}), encoding='utf-8')
os.replace(pending, root / 'current.json')
PY
echo "ESP-IDF setup complete. Return to vibeKidbright and click Check Native SDK."
)`;

export default function MacosSdkSetup({ onRecheck }: { onRecheck: () => void | Promise<void> }) {
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState("");
  const [checking, setChecking] = useState(false);
  return (
    <section aria-label="macOS ESP-IDF setup" style={{ fontSize: 12, lineHeight: 1.5 }}>
      <p>macOS ต้องใช้ Python และ compiler สำหรับ Mac โดยตรง รองรับทั้ง Apple Silicon และ Intel</p>
      <p>เปิด Terminal บน Mac แล้วรันคำสั่งด้านล่างเพื่อ Clean install SDK และ Python ใหม่ในโฟลเดอร์ของผู้ใช้เครื่องนี้ จากนั้นกด Check Native SDK เพื่อแทนค่า path เก่า</p>
      <p>แต่ละครั้งจะสร้าง SDK ชุดใหม่แยกจากชุดเก่า โดยเก็บชุดเก่าไว้สำรองและเก็บโปรเจกต์/การตั้งค่าอื่นไว้</p>
      <p>ต้องมี Git และ Python 3.9 ขึ้นไป หากยังไม่มี ให้รัน <code>xcode-select --install</code> แล้วติดตั้งให้เสร็จก่อน</p>
      <details>
        <summary style={{ cursor: "pointer" }}>คำสั่ง Clean install ESP-IDF สำหรับ Mac</summary>
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
