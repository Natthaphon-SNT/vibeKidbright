# vibeKidbright

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="vibeKidbright Logo" width="120"/>
</p>

<p align="center">
  <strong>vibeKidbright IDE สำหรับพัฒนา ESP-IDF และ KidBright บน Desktop</strong>
</p>

<p align="center">
  <a href="https://github.com/Natthaphon-SNT/vibeKidbright/releases/tag/v3.0.0">
    <img src="https://img.shields.io/badge/version-v3.0.0-blue.svg" alt="Version"/>
  </a>
  <img src="https://img.shields.io/badge/platform-Windows-lightgrey.svg" alt="Platform"/>
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License"/>
  <img src="https://img.shields.io/badge/built%20with-Tauri%20%2B%20React-orange.svg" alt="Tech Stack"/>
</p>

---

## 📖 ภาพรวม

**vibeKidbright** คือ IDE สมัยใหม่ที่ขับเคลื่อนด้วย AI ออกแบบมาเพื่อการพัฒนาโปรเจกต์ ESP-IDF (Espressif IoT Development Framework) โดยเฉพาะ รองรับบอร์ด ESP32 ทุกรุ่น รวมถึง **KidBright32** และ **KidBright μAI Plus**

สร้างขึ้นด้วย **Tauri + React + TypeScript** ทำให้ได้แอพพลิเคชั่น Desktop ที่เบา เร็ว และมีประสิทธิภาพสูง พร้อม AI Assistant ในตัวที่ช่วยเขียนและวิเคราะห์โค้ด Firmware ได้โดยตรง

---

## ✨ ฟีเจอร์หลัก

### 📁 Project Management
- **สร้างโปรเจกต์ใหม่** — สร้าง ESP-IDF project structure ได้ทันทีจากหน้า UI
- **เปิดโปรเจกต์ที่มีอยู่** — เปิด folder ของโปรเจกต์ ESP-IDF เดิมได้โดยตรง
- **File Explorer** — แสดงต้นไม้ไฟล์ของโปรเจกต์ทั้งหมด พร้อมเปิด/แก้ไขไฟล์ได้ในหน้าเดียว

### 🤖 Vibe Coder — AI Assistant
- **อ่านโค้ดของคุณ** — AI สามารถอ่านไฟล์ source ปัจจุบันและทำความเข้าใจ context ของโปรเจกต์
- **แนะนำการแก้ไขโค้ด** — ถามปัญหาเกี่ยวกับ firmware, logic, หรือ bug ได้เป็นภาษาธรรมชาติ
- **Inject Code โดยตรง** — AI สามารถเขียนโค้ดแล้วแทรกเข้าไปในไฟล์ของคุณได้ทันทีโดยไม่ต้อง copy-paste
- **Knowledge Base** — มีฐานข้อมูลความรู้เฉพาะทางสำหรับ KidBright และ ESP-IDF เพื่อให้คำตอบที่แม่นยำ

### 🖥️ Interactive Terminal
- **Built-in Terminal** — Terminal แบบ interactive ในตัว ไม่ต้องเปิด Command Prompt แยก
- **Real-time Logs** — แสดง output จาก ESP-IDF build system, `idf.py`, และ CMake แบบ real-time
- **Shell Commands** — รันคำสั่งทั่วไปได้เลยจาก terminal ภายในแอพ

### ⚡ One-Click Build & Flash
- **Build** — คอมไพล์โปรเจกต์ด้วยปุ่มเดียว (รัน `idf.py build` ใต้ฝากระโปรง)
- **Flash** — เขียน firmware ลง ESP32 ผ่านพอร์ต Serial โดยไม่ต้องพิมพ์คำสั่ง
- **Monitor** — เปิด Serial Monitor เพื่อดู log จาก device แบบ real-time
- **Clean** — ล้าง build artifacts ได้จากปุ่มเดียว

### 🎨 Modern UI / UX
- **Dark Theme** — ธีมสีเข้มที่ออกแบบมาสำหรับการโค้ดโดยเฉพาะ
- **Tailwind CSS** — UI สะอาด ทันสมัย ปรับขนาดได้อัตโนมัติ
- **Responsive Layout** — แบ่ง panel ระหว่าง Editor, Terminal, และ AI Chat ได้ยืดหยุ่น

---

## 📦 เวอร์ชั่น

### v3.0.0 — Initial Release (6 กรกฎาคม 2026)
> รองรับ **Windows** เท่านั้น

ฟีเจอร์ที่มาพร้อม v3.0.0:
- Project Management (สร้าง/เปิดโปรเจกต์)
- Vibe Coder AI Assistant (อ่านโค้ด + Inject Code)
- Built-in Interactive Terminal
- One-Click Build, Flash, Monitor, Clean
- Dark Theme UI
- Knowledge Base สำหรับ KidBright + ESP-IDF
- GitHub Actions CI/CD pipeline สำหรับ auto-build release

**ดาวน์โหลด:** ไปที่ [Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases/tag/v3.0.0) แล้วดาวน์โหลดไฟล์ `.msi` หรือ `.exe`

---

## 🚀 การติดตั้ง (สำหรับผู้ใช้ทั่วไป)

### Windows (แนะนำ)
1. ไปที่ [Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases) 
2. ดาวน์โหลดไฟล์ `.msi` หรือ `.exe` จาก release ล่าสุด
3. ดับเบิลคลิกติดตั้ง แล้วเปิดใช้งานได้เลย

> ⚠️ ต้องติดตั้ง **ESP-IDF** ไว้ก่อน และตั้งค่า environment variables ให้ถูกต้อง

---

## 🛠️ การ Build จาก Source (สำหรับนักพัฒนา)

### Prerequisites

ต้องติดตั้งสิ่งต่อไปนี้ก่อน:

| เครื่องมือ | เวอร์ชั่น | หมายเหตุ |
|---|---|---|
| [Node.js](https://nodejs.org/) | v18 หรือใหม่กว่า | Runtime สำหรับ Frontend |
| [Rust](https://www.rust-lang.org/) | stable | Backend ของ Tauri |
| Tauri CLI | ล่าสุด | `cargo install tauri-cli` |
| [ESP-IDF](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/) | v5.x | Framework สำหรับ ESP32 |
| Xcode CLI Tools | — | **macOS เท่านั้น** |

### Clone และติดตั้ง

```bash
# 1. Clone repository
git clone https://github.com/Natthaphon-SNT/vibeKidbright.git
cd vibeKidbright

# 2. ติดตั้ง Node dependencies
npm install
```

### Development Mode

```bash
npm run tauri dev
```

คำสั่งนี้จะ:
- เปิด Vite dev server สำหรับ Frontend (React + TypeScript)
- Compile และรัน Tauri-Rust backend
- เปิดหน้าต่างแอพในโหมด development พร้อม hot-reload

### Production Build

```bash
npm run build
npm run tauri build
```

ไฟล์ installer จะอยู่ที่:
```
src-tauri/target/release/bundle/
├── msi/          # Windows MSI installer
├── nsis/         # Windows EXE installer
└── (macOS/Linux bundle หากรันบน platform นั้น)
```

---

## 📂 โครงสร้างโปรเจกต์

```
vibeKidbright/
├── .github/
│   └── workflows/         # GitHub Actions CI/CD (auto-build release)
├── knowledge_base/        # ฐานข้อมูลความรู้สำหรับ AI (ESP-IDF, KidBright)
├── public/                # Static assets (icons, images)
├── resources/             # App resources (icon สำหรับ Tauri)
├── src/                   # Frontend Source (React + TypeScript)
│   ├── components/        # React components (Editor, Terminal, AI Panel, etc.)
│   ├── hooks/             # Custom React hooks
│   ├── lib/               # Utility functions
│   └── App.tsx            # Root component
├── src-tauri/             # Tauri Backend (Rust)
│   ├── src/
│   │   ├── main.rs        # Entry point ของ Rust backend
│   │   └── lib.rs         # Tauri commands (build, flash, terminal, file ops)
│   └── tauri.conf.json    # Tauri configuration
├── index.html             # HTML entry point
├── package.json           # Node dependencies
├── vite.config.ts         # Vite bundler config
├── tsconfig.json          # TypeScript config
└── CMakeLists.txt         # CMake config (ตัวอย่าง ESP-IDF project)
```

---

## 🔧 เทคโนโลยีที่ใช้

| Layer | เทคโนโลยี |
|---|---|
| **Frontend** | React, TypeScript, Vite |
| **Styling** | Tailwind CSS |
| **Desktop Shell** | Tauri v2 |
| **Backend Logic** | Rust |
| **AI Integration** | Anthropic Claude API (ผ่าน Knowledge Base) |
| **Embedded Target** | ESP-IDF (ESP32, KidBright32, KidBright μAI Plus) |
| **Build System** | CMake + Ninja (ผ่าน idf.py) |
| **CI/CD** | GitHub Actions |

---

## 🎯 รองรับบอร์ดอะไรบ้าง

vibeKidbright ออกแบบมาสำหรับ ESP32-based boards โดยเฉพาะ:

- **ESP32** (ทุกรุ่น)
- **ESP32-S2 / S3**
- **ESP32-C3 / C6**
- **KidBright32** (IPST)
- **KidBright μAI Plus** (IPST)

> การ Flash และ Monitor ใช้ `idf.py flash monitor` ผ่าน USB Serial ตามปกติ

---

## 📋 Prerequisites สำหรับใช้งาน Flash

ก่อน Flash firmware จำเป็นต้องมี:

1. ติดตั้ง ESP-IDF และ set `IDF_PATH` ใน environment variables
2. Python 3.8+ (มาพร้อม ESP-IDF)
3. USB Driver สำหรับ CH340/CP2102 (บอร์ด KidBright ใช้ CH340)
4. พอร์ต COM ที่มองเห็นได้ใน Device Manager

---

## 🤝 Contributing

ยินดีรับ Pull Request และ Issue ทุกรูปแบบ:

1. Fork repository นี้
2. สร้าง feature branch: `git checkout -b feature/your-feature`
3. Commit การเปลี่ยนแปลง: `git commit -m 'Add some feature'`
4. Push ขึ้น branch: `git push origin feature/your-feature`
5. เปิด Pull Request

---

## 🐛 Known Issues (v3.0.0)

- รองรับ **Windows เท่านั้น** ในปัจจุบัน (macOS/Linux อยู่ใน roadmap)
- ESP-IDF ต้องติดตั้งแยกและตั้งค่า PATH ด้วยตนเอง
- หากพบปัญหา COM port conflict (เช่น Arduino IDE เปิดอยู่พร้อมกัน) ให้ปิดแอพอื่นก่อน Flash

---

## 📄 License

This project is licensed under the **MIT License** — see [LICENSE](LICENSE) for details.

---

## 📬 ติดต่อ / Support

- **Issues:** [GitHub Issues](https://github.com/Natthaphon-SNT/vibeKidbright/issues)
- **Releases:** [GitHub Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases)
- **LINE ID:** [dragon3541] **LINE Name:** [Peter B. Parker]

---

<p align="center">
  Made with ❤️ for the KidBright & ESP32 Community
</p>
