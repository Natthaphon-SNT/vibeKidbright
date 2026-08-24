# vibeKidbright

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="vibeKidbright Logo" width="120"/>
</p>

<p align="center">
  <strong>vibeKidbright — AI-Powered Desktop IDE สำหรับพัฒนา ESP-IDF และ KidBright</strong>
</p>

<p align="center">
  <a href="https://github.com/Natthaphon-SNT/vibeKidbright/releases">
    <img src="https://img.shields.io/badge/version-v3.6.4-blue.svg" alt="Version"/>
  </a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey.svg" alt="Platform"/>
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"/>
  <img src="https://img.shields.io/badge/built%20with-Tauri%20v2%20%2B%20React%2019-orange.svg" alt="Tech Stack"/>
  <img src="https://img.shields.io/badge/winget-available-blueviolet.svg" alt="WinGet"/>
</p>

---

## 📖 ภาพรวม (Overview)

**vibeKidbright** คือ Next-Generation IDE สำหรับการพัฒนา Firmware บนไมโครคอนโทรลเลอร์ ESP32 และ KidBright ด้วย **ESP-IDF (Espressif IoT Development Framework)** 

สร้างขึ้นบนสถาปัตยกรรม **Tauri v2 + React 19 + TypeScript + Rust** มอบประสิทธิภาพที่เบา ทำงานรวดเร็ว ใช้ทรัพยากรเครื่องน้อย พร้อมระบบ **AI Copilot** ที่รองรับ Multi-Provider, **Local RAG (Vector Database ในตัวแบบ Offline)** และระบบ **Happy Meal Toolchain Manager** ที่ช่วยดาวน์โหลดและติดตั้ง ESP-IDF อัตโนมัติในคลิกเดียวโดยไม่ต้องตั้งค่าระบบด้วยตนเองให้ยุ่งยาก

---

## ✨ ฟีเจอร์หลัก (Key Features)

### 🍱 Happy Meal Toolchain Manager (ติดตั้งง่ายในคลิกเดียว)
- **Zero-Configuration Setup** — ดาวน์โหลดและติดตั้ง Toolchain (ESP-IDF + Python + Compiler) แบบ Pre-packaged ลงในเครื่องให้อัตโนมัติ
- **Auto Environment Repair** — มีระบบตรวจจับและ Auto-repair path ของ `pyvenv.cfg` และ Toolchain environment เมื่อย้ายเครื่องหรือเปลี่ยน Directory
- **Progress Tracking** — แสดงสถานะและเปอร์เซ็นต์การดาวน์โหลด/แตกไฟล์แบบ Real-time พร้อมโหมด Mini Widget

### 🤖 Vibe Coder — Multi-Provider AI Assistant
- **Multi-Provider Support** — รองรับโมเดลภาษาหลากหลาย:
  - **Google Gemini** (Gemini 2.5 Flash, Pro พร้อมรองรับ Thinking Mode และ Thought Signature)
  - **OpenAI** (GPT-4o, GPT-4.1)
  - **OpenRouter** (เข้าถึงโมเดลชั้นนำทั่วโลก)
  - **Local LLM** (Ollama, LM Studio สำหรับการใช้งานแบบ Offline 100% และความเป็นส่วนตัว)
- **Agentic Function Calling / Tools** — AI สามารถเรียกเครื่องมือเพื่อช่วยงานได้จริง:
  - อ่านโค้ดและโครงสร้างไฟล์ในโปรเจกต์
  - เขียน/แก้ไขโค้ดพร้อมแสดง **Unified Diff Preview** ให้ผู้ใช้ตรวจสอบและกดยอมรับ (Accept) หรือยกเลิก (Reject)
  - รันคำสั่ง Terminal และค้นหาไฟล์อย่างปลอดภัย
- **ความปลอดภัยระดับ OS Keychain** — จัดเก็บ API Keys ใน Windows Credential Manager หรือ macOS Keychain โดยอัตโนมัติ ไม่มีการบันทึก Plaintext ลงบน Disk

### 🧠 Local RAG & Hybrid Knowledge Base (ฐานข้อมูลความรู้ในตัว)
- **Local Embedding Engine (fastembed)** — สร้าง Vector Embedding ด้วยโมเดล `all-MiniLM-L6-v2` แบบ Offline 100% โดยไม่ต้องใช้ API Key
- **SQLite Vector Store** — จัดเก็บและค้นหา Knowledge Chunks ผ่าน SQLite ภายในเครื่องอย่างรวดเร็ว
- **Hybrid Search** — ค้นหาข้อมูลเชิงความหมาย (Semantic Search) ผสานกับการค้นหาคำสำคัญ (Keyword Search) สำหรับวงจร บอร์ด KidBright, ไดรเวอร์ และ API ของ ESP-IDF
- **Built-in Wiki View** — มีหน้าสำหรับเปิดอ่านคู่มือ บทเรียน และ Schematic ของบอร์ดได้โดยตรงจากโปรแกรม

### 💻 Monaco Code Editor & Project Management
- **Monaco Editor Integration** — ระบบ Code Editor ระดับเดียวกับ VS Code รองรับ Syntax Highlighting สำหรับ C/C++, CMakeLists, Kconfig, JSON, Python ฯลฯ
- **Project Tree Explorer** — จัดการไฟล์ สร้าง โฟลเดอร์ เปลี่ยนชื่อ ลบไฟล์ได้ง่ายดาย
- **Project Template Generator** — สร้างโครงสร้างโปรเจกต์ ESP-IDF เริ่มต้นได้ทันที

### ⚡ One-Click Build, Flash & Serial Monitor
- **Build Firmware** — คอมไพล์โปรเจกต์ด้วยปุ่มเดียวผ่าน Toolchain ภายในแอพ
- **Flash to Board** — อัปโหลด Firmware เข้า ESP32/KidBright ผ่านพอร์ต Serial
- **Real-time Serial Monitor** — เปิดหน้าต่างรับ-ส่งข้อความ Serial Log แบบสดจากบอร์ด พร้อม auto-scroll และ clear log
- **Interactive Terminal** — หน้าต่าง Terminal ในตัวที่ส่งคำสั่ง shell หรือ idf.py ได้อย่างอิสระ

---

## 🎯 รองรับบอร์ดและฮาร์ดแวร์

- **KidBright32** (ทุกเวอร์ชัน: V1.3, V1.5, V1.6)
- **KidBright μAI Plus**
- **ESP32** (ESP32-WROOM, ESP32-WROVER)
- **ESP32-S2 / ESP32-S3**
- **ESP32-C3 / ESP32-C6**

---

## 🚀 การติดตั้ง (Installation)

### วิธีที่ 1: ติดตั้งผ่าน WinGet (Windows - แนะนำ)
เปิด Command Prompt หรือ PowerShell แล้วพิมพ์:
```powershell
winget install Natthaphon-SNT.vibeKidbright
```

### วิธีที่ 2: ดาวน์โหลดตัวติดตั้ง (Windows / macOS)
1. ไปที่ [GitHub Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases)
2. เลือกดาวน์โหลดไฟล์ติดตั้งตามระบบปฏิบัติการของคุณ:
   - **Windows:** ดาวน์โหลดไฟล์ `.exe` (NSIS Installer) หรือ `.msi`
   - **macOS:** ดาวน์โหลดไฟล์ `.dmg` (Universal Binary รองรับทั้ง Apple Silicon M1/M2/M3/M4 และ Intel)
3. เปิดไฟล์และทำตามขั้นตอนการติดตั้ง

---

## 🛠️ การรันและพัฒนาจาก Source Code (Developer Guide)

### สิ่งที่ต้องเตรียม (Prerequisites)
- [Node.js](https://nodejs.org/) (v20+ หรือ v22 แนะนำ)
- [Rust](https://www.rust-lang.org/) (เวอร์ชัน Stable ล่าสุด)
- [Tauri CLI v2](https://v2.tauri.app/) (`cargo install tauri-cli --version "^2"`)
- (สำหรับ macOS) Xcode Command Line Tools

### ขั้นตอนการรัน

```bash
# 1. Clone repository
git clone https://github.com/Natthaphon-SNT/vibeKidbright.git
cd vibeKidbright

# 2. ติดตั้ง Node Dependencies
npm install

# 3. รัน Development Server (Vite + Tauri)
npm run tauri dev
```

### การทดสอบ (Testing)

```bash
# รัน Frontend Unit Tests (Vitest)
npm test

# รัน Backend Tests (Rust)
cargo test --manifest-path src-tauri/Cargo.toml
```

### การ Build สำหรับ Production

```bash
# Build ทั้ง Frontend และ Desktop Package
npm run build
npm run tauri build
```

ไฟล์ Installer จะถูกสร้างไว้ที่: `src-tauri/target/release/bundle/`

---

## 📂 โครงสร้างโปรเจกต์ (Project Structure)

```
vibeKidbright/
├── .github/
│   └── workflows/         # CI/CD Workflows (Windows, macOS, WinGet releases)
├── knowledge_base/        # เนื้อหาเอกสารและฐานความรู้สำหรับ AI (Markdown, Schematics)
├── resources/             # ทรัพยากรของแอป (Knowledge base bundle, Assets)
├── src/                   # Frontend Application (React 19 + TypeScript)
│   ├── AiChat.tsx         # ระบบ AI Assistant UI, Tool execution & Diff review
│   ├── CodeEditor.tsx     # Monaco Editor Wrapper พร้อม Syntax Config
│   ├── ToolchainSetup.tsx # หน้าต่าง Happy Meal Toolchain Manager
│   ├── WikiView.tsx       # ตัวอ่านเอกสาร Knowledge Base Viewer
│   ├── App.tsx            # Main Application Layout & State Management
│   ├── utils.ts           # Pure utility functions
│   └── test/              # Frontend Unit Test Suite (Vitest)
├── src-tauri/             # Backend Application (Tauri v2 + Rust)
│   ├── src/
│   │   ├── main.rs        # Application Entry Point
│   │   ├── lib.rs         # Tauri App Builder, Plugins, Commands Registry
│   │   ├── ai_chat.rs     # AI Core, Stream processing, Tools & Keychain
│   │   ├── esp_idf.rs     # ESP-IDF CLI runner, Serial Monitor & Project Ops
│   │   ├── toolchain.rs   # Happy Meal Toolchain downloader & Auto-repair
│   │   ├── kb_store.rs    # SQLite-backed Vector Store
│   │   ├── kb_embed.rs    # Local fastembed (ONNX) embedding pipeline
│   │   └── ai/            # Modular AI Subsystem Architecture
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration (Window, Security, Bundle)
├── winget-manifests/      # WinGet Package Repository Manifests
├── package.json           # Frontend dependencies & scripts
└── vite.config.ts         # Vite bundler configuration
```

---

## 🔧 เทคโนโลยีที่ใช้งาน (Tech Stack)

| Layer | เทคโนโลยี |
|---|---|
| **Desktop Framework** | [Tauri v2](https://v2.tauri.app/) (Rust) |
| **Frontend UI** | [React 19](https://react.dev/), [TypeScript](https://www.typescriptlang.org/), [Tailwind CSS v4](https://tailwindcss.com/) |
| **Code Editor** | [Monaco Editor](https://microsoft.github.io/monaco-editor/) (`@monaco-editor/react`) |
| **Local AI & RAG** | [fastembed-rs](https://github.com/Anush008/fastembed-rs) (all-MiniLM-L6-v2 ONNX) + [rusqlite](https://github.com/rusqlite/rusqlite) (SQLite WAL) |
| **AI Providers** | Google Gemini API, OpenAI API, OpenRouter, Local Ollama / LM Studio |
| **Security** | OS Keychain (`keyring-rs`) สำหรับเก็บ API Keys |
| **Embedded Framework** | ESP-IDF (FreeRTOS, CMake, Ninja, esptool) |
| **CI/CD** | GitHub Actions (Auto-build Windows EXE/MSI, macOS Universal DMG, WinGet automation) |

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

## 📬 ติดต่อ / ช่องทางติดต่อผู้พัฒนา

- **Issues & Discussions:** [GitHub Issues](https://github.com/Natthaphon-SNT/vibeKidbright/issues)
- **Releases:** [GitHub Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases)
- **Maintainer:** Natthaphon-SNT
- **LINE ID:** `dragon3541` | **LINE Name:** Peter B. Parker

---

<p align="center">
  Made with ❤️ for the KidBright, ESP32 & Embedded Developers Community
</p>
