# ติดตั้ง vibeKidbright 3.7.2 บนเครื่องผู้ใช้

ตัวติดตั้ง Windows x64 รุ่นนี้จัดให้ใช้ไฟล์ของตัวเองได้บนเครื่องผู้ใช้ ไม่ต้องมีโฟลเดอร์โปรเจกต์จากเครื่องพัฒนา ไม่ต้องติดตั้ง Node.js, npm, Rust หรือ Git เพื่อเปิด IDE และไม่ดึงไฟล์แอปจาก GitHub ขณะใช้งาน

## สิ่งที่รวมในตัวติดตั้ง

- หน้าแอป, Monaco Editor, web workers, ไอคอน และไฟล์ความรู้/ตัวอย่างทั้งหมด
- โมเดลค้นหาความรู้ all-MiniLM-L6-v2 ONNX พร้อม tokenizer, revision และ SHA-256 ที่ตรึงไว้ รวมใบอนุญาต Apache-2.0 และข้อมูลผู้ดูแล โมเดลมีขนาดประมาณ 90 MB และโหลดจากไฟล์ในชุดติดตั้ง ไม่มี first-run model download
- Windows WebView2 offline installer เพื่อเตรียม WebView2 เมื่อเครื่องยังไม่มี และ Visual C++ runtime DLL แบบ app-local
- การเก็บ settings และฐานข้อมูลค้นหาความรู้ใน AppData ของผู้ใช้คนนั้น ไม่เขียนลงโฟลเดอร์ที่ติดตั้งโปรแกรม

การติดตั้งเป็น current-user ไม่ต้องมีสิทธิ์เขียนลงเครื่องของผู้พัฒนา สิทธิ์และนโยบายติดตั้ง WebView2 ยังขึ้นกับ Windows/องค์กรของเครื่องปลายทาง

## วิธีใช้งานบนเครื่องใหม่

1. คัดลอกไฟล์ `VibeKidbright IDE_3.7.2_x64-setup.exe` ไปยังเครื่อง Windows x64 แล้วติดตั้งตามหน้าจอ
2. เปิด vibeKidbright จาก shortcut ใช้พื้นที่เรียนรู้ แก้โค้ด และอ่าน Wiki ได้ก่อนติดตั้ง toolchain
3. เมื่อต้องการสร้าง firmware ให้เลือกติดตั้ง toolchain ใน Setup / Repair ESP-IDF โปรแกรมจะดาวน์โหลดและเตรียมบนเครื่องผู้ใช้เอง ส่วนนี้ยังใช้ GitHub ได้ตามขอบเขตที่กำหนด และไม่เริ่มดาวน์โหลดเองเพียงเพราะเปิดแอป
4. ตั้งค่า AI provider และ API key ของผู้ใช้ใน Vibe Coder Settings บริการ AI ผ่านอินเทอร์เน็ตยังต้องเชื่อมต่อ provider ที่เลือก หรือเลือก local provider ที่ผู้ใช้เตรียมเอง ไม่ใช้ API key หรือเซิร์ฟเวอร์ของเครื่องพัฒนา
5. เปิดหรือสร้างโปรเจกต์ในโฟลเดอร์ที่ผู้ใช้เขียนได้ เลือกบอร์ด/พอร์ตและยืนยัน Keep ก่อน Build & Flash

## การแยกข้อมูลแต่ละเครื่อง

Settings ใช้ AppData ของ Tauri application ID `th.nectec.vibekidbright` บน Windows อยู่ใต้ `%APPDATA%\th.nectec.vibekidbright` ขณะที่ toolchain อยู่ใต้ AppData ของ application ID เดียวกันตาม Tauri API รายชื่อโปรเจกต์และบทสนทนาใช้ storage ของ WebView ของผู้ใช้นั้น ไม่บรรจุข้อมูลเหล่านี้ลง installer

API key ใหม่ใช้ native Windows Credential Manager โดยเปิด native backend ของ keyring จริง ไม่สร้าง plaintext fallback ใหม่ การย้าย settings เก่าจาก `.vibekidbright` จะนำ key เข้า credential store และลบ field เดิมหลังยืนยันอ่านกลับได้ หาก OS store หรือการย้ายข้อมูลไม่สำเร็จ จะเก็บข้อมูลเก่าไว้เพื่อกู้คืน ไม่ลบทิ้งเงียบ ๆ

AI Wiki ใช้ native backend เช่นเดียวกับการตั้งค่า provider ของผู้ใช้ จึงไม่ติด browser CORS/CSP ส่วน HTML ในบทความผ่าน sanitizer และไม่โหลดรูปติดตามจากภายนอก ฟอนต์ใช้ system font stack ไม่มีการร้องขอ Google Fonts

## การสร้างตัวติดตั้งจากซอร์ส

เครื่อง build ใช้ Node/Rust/MSVC ตามปกติ รัน `npm run prepare:runtime -- --download` เพื่อเตรียมโมเดลที่ตรึง revision/hash และคัดลอก MSVC redistributable จากเครื่อง build ขั้นนี้ทำตอนสร้างโปรแกรมเท่านั้น ไฟล์โมเดลขนาดใหญ่และ runtime DLL เป็น build assets ที่ไม่เก็บใน Git แต่ถูกแนบใน installer

`npm run build:release` ตรวจ runtime assets, packaging policy, knowledge bundle, board/rule inventories และ TypeScript/Vite ก่อน Tauri สร้าง NSIS ขั้นเตรียม Windows WebView2 offline installer อาจดาวน์โหลดจาก Microsoft บนเครื่อง build เพื่อแนบในชุดติดตั้ง ผู้ใช้ปลายทางไม่ต้องดาวน์โหลด bootstrapper จากอินเทอร์เน็ต

ตรวจชุดติดตั้งที่แตกไฟล์แล้วด้วย `python scripts/test-standalone-install.py --installed-directory <directory> --report <report.json>` ตัวตรวจเรียก `--self-check` ของ binary จาก working directory แยก ใช้ AppData ใหม่ ลบพาธ compiler/cache ของเครื่องพัฒนาออกจาก environment และตั้ง HTTP proxy ไปยังปลายทางที่ปิดอยู่ ตรวจโมเดลจริง, SQLite search, การเขียน settings และ Credential Manager ด้วย credential ทดสอบชั่วคราวแล้วลบทันที

หลักฐานผลจริงและ hash ตัวติดตั้งอยู่ใน [standalone validation](research/standalone-installation-2026-10-17-validation.json) การย้ายโฟลเดอร์และ environment isolation ไม่ใช่การทดสอบบน clean VM และไม่รับรอง USB driver/hardware ทุกเครื่อง

## ขอบเขตที่ยังคงเดิม

ความพร้อมติดตั้งแยกจากความครอบคลุมบอร์ด: μAI/Plus ยังเป็น learning preview ตามแผน ไม่เพิ่ม native deployment/training หรือรับรองทุกบล็อกจากการแก้ installer รายการ Undo และการกู้คืน session ที่ยังคงค้างใน [ผล audit](PROJECT_AUDIT_2026-10-17.md) ไม่ได้กลายเป็นฟังก์ชันสมบูรณ์เพียงเพราะย้ายไปเครื่องอื่นได้
