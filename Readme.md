# vibeKidbright

<p align="center">
  <img src="src-tauri/icons/icon.png" alt="vibeKidbright Logo" width="120"/>
</p>

<p align="center">
  <strong>Desktop IDE สำหรับเรียนรู้และพัฒนา KidBright ด้วย ESP-IDF และผู้ช่วย AI</strong>
</p>

พัฒนาด้วย **Tauri v2, React 19, TypeScript และ Rust** รวมตัวแก้ไขโค้ด Vibe Coder, Wiki, ฐานความรู้ภายในเครื่อง และเครื่องมือ Build & Flash ไว้ในโปรแกรมเดียว เป้าหมายคือรองรับ KidBright32 หลายรุ่น และต่อยอดไปยัง KidBright μAI / μAI Plus หลังงาน KidBright32 สมบูรณ์

**สถานะ ณ 18 ตุลาคม 2026:** เหมาะสำหรับตรวจทานและทดสอบแบบ Draft/Beta ยังไม่รับรองว่าแทน KidBright IDE ต้นฉบับได้ครบทุกบอร์ด ทุกบล็อก หรือทุกฟังก์ชัน ตัวติดตั้ง Windows x64 ที่สร้างและตรวจล่าสุดคือ **3.7.3** ตรวจไฟล์ภายในและรัน binary ด้วย AppData ใหม่โดยตัด path เครื่องพัฒนาแล้ว ยังไม่ได้ทดสอบ GUI บน clean VM ดู [ผลตรวจชุดติดตั้ง](docs/ai-engineering/installer-validation-3.7.3.json)

อัปเดต 18 ตุลาคม 2026: จัดเวอร์ชัน Tauri, npm, Cargo และ lockfile ให้ตรงกันที่ **3.7.3** แล้ว ผลทดสอบตัวติดตั้ง 3.7.2 เป็นหลักฐานย้อนหลัง ดูผลสร้างและตรวจรุ่นใหม่ใน [verification.json](docs/ai-engineering/verification.json)

ซอร์สที่พัฒนาต่อจาก NSIS 3.7.3 มี optional clangd สำหรับค้นหา definition/references ด้วย compile_commands ของ ESP-IDF และข้อความที่ยังไม่ Save, Block-to-C เพิ่มเป็น 9 ID และ tests ด้านการกู้คืน/ข้อมูลแทรกคำสั่ง งานใหม่นี้ **ยังไม่อยู่ใน installer เดิม** และยังไม่ publish ดู [รายงานรอบพัฒนาล่าสุด](docs/ai-engineering/14-continuation-report.md) โมเดลค้นหาเริ่มต้นยังเป็นตัวเดิม

## ความสามารถปัจจุบัน

- **Vibe Coder:** ใช้ Google Gemini, OpenAI, OpenRouter หรือ local provider เช่น Ollama / LM Studio ตามการตั้งค่าของผู้ใช้ ช่วยอ่านไฟล์ สร้างโปรเจกต์ และเสนอการแก้โค้ดผ่านเครื่องมือ
- **ตรวจทานงาน AI:** มี diff และ Keep/Undo สำหรับงานที่เสนอ รวม Project Review เมื่อสร้างโปรเจกต์ รอการยืนยันก่อน Build & Flash เมื่อมีรายการตรวจทานค้าง
- **พื้นที่โค้ดสำหรับผู้เรียน:** แถบ Project แสดงโฟลเดอร์ `main` และไฟล์ที่ผู้เรียนแก้ได้ ซ่อนไฟล์ระบบ เอกสารภายใน และ `kb_board.h` จากรายการแก้ไข ไฟล์ที่ใช้ build ยังคงอยู่บนดิสก์
- **Build & Flash:** บันทึกไฟล์ที่แก้ก่อน build เรียก ESP-IDF สำหรับ classic ESP32 และอัปโหลดผ่านพอร์ต serial พร้อม Serial Monitor และ Terminal
- **Toolchain Manager:** ติดตั้งหรือซ่อม ESP-IDF, Python และ compiler บนเครื่องผู้ใช้ โดยผู้ใช้เป็นคนเริ่มดาวน์โหลด เปิดแอปและแก้โค้ดได้ก่อนติดตั้ง toolchain
- **ฐานความรู้ภายในเครื่อง:** รวมเอกสาร โมเดล `all-MiniLM-L6-v2` และ tokenizer ใช้ fastembed/ONNX กับ SQLite สำหรับค้นหาความรู้โดยไม่ต้องดาวน์โหลดโมเดลเมื่อเปิดแอปครั้งแรก
- **Wiki:** อ่านเอกสารและใช้ผู้ช่วย AI ผ่าน native backend เนื้อหา HTML ผ่าน sanitizer; ตัวแก้ไข Monaco และฟอนต์ไม่พึ่ง CDN
- **การตั้งค่า:** เก็บข้อมูลของผู้ใช้แยกตามเครื่อง API key ใหม่ใช้ Windows Credential Manager / macOS Keychain การย้าย key เก่าจะลบ plaintext เดิมหลังยืนยันอ่านจาก credential store กลับได้
- **ภาษา:** ผู้ช่วย AI เลือกภาษาจากคำขอล่าสุดหรือภาษาที่ผู้ใช้ระบุ ส่วนหน้าพื้นที่เรียนรู้มี TH/EN ยังไม่ได้ทดสอบคุณภาพคำตอบครบทุกภาษา และไม่ได้แปลทั้งแอปครบทุกภาษา

## บอร์ดและขอบเขตที่รองรับ

การมีชื่อบอร์ดใน catalog หรือเลือกชื่อใน UI ได้ ยังไม่ใช่หลักฐานว่าโปรแกรมรองรับ peripheral และ deployment ของบอร์ดนั้นครบ

| บอร์ด / ส่วนงาน | สถานะปัจจุบัน |
|---|---|
| `kidbright32_v13` | มีโปรไฟล์และตัวอย่าง `button_buzzer`, `temperature_serial`, `music_scale`, `matrix_counter` ผ่านการ build ตามหลักฐาน ยังไม่มีผลเทียบ peripheral กับ IDE ต้นฉบับบนบอร์ดจริงครบ |
| `kidbright32_rev31` | มีโปรไฟล์และตัวอย่างเสียง/อุณหภูมิ/เพลง ยังไม่เปิด `matrix_counter` เพราะหลักฐาน revision และขาจอยังต้องยืนยัน |
| KidBright32 รุ่นอื่น รวม iA / iP | มีตารางเปรียบเทียบและข้อมูลอ้างอิง รวม 16 รายการใน catalog ไม่ใช่ 16 บอร์ดที่ใช้งานได้ครบ ผู้ใช้รายงาน Hello/Heartbeat บน iA ผ่านแล้ว แต่ยังไม่ยืนยันจอ ปุ่ม เสียง และเซนเซอร์ |
| KidBright μAI / μAI Plus | มีหน้าพื้นที่เรียนรู้แบบ preview สำหรับจัดตัวอย่างและป้ายกำกับ ดูความพร้อมข้อมูลและตัวอย่างโค้ด ยังฝึกโมเดลหรือ deploy ลงบอร์ดจากแอปไม่ได้ งานเชื่อมต่อจริงอยู่ในแผนอนาคต |
| ESP32-S2 / S3 / C3 / C6 | ยังไม่มีเส้นทางเลือก target และ Build & Flash ที่รองรับครบใน UI |
| Blockly / Simulator | มีข้อมูลต้นแบบและ inventory บล็อก ยังไม่มีตัวนำเข้า/แปลงโปรเจกต์ Blockly หรือ simulator ที่ทำงานเทียบเท่าต้นฉบับครบ |

ตัวอย่าง `matrix_counter` ใช้ตัวเลข 5×7 ที่ทำขึ้นใหม่ ส่วนจอ/เพลงยังไม่ครอบคลุมฟอนต์ ข้อความเลื่อน และการจัดตารางงานของต้นฉบับทั้งหมด ดู [สถานะการพัฒนา](docs/KIDBRIGHT_IMPLEMENTATION_STATUS_2026-10-17.md), [ตารางบอร์ด](docs/KIDBRIGHT_BOARD_MATRIX_2026-10-17.md) และ [ขอบเขตการรองรับ](knowledge_base/support_scope.md)

## ติดตั้งและเริ่มใช้งาน

ชุดติดตั้ง Windows x64 ที่สร้างล่าสุดชื่อ `VibeKidbright IDE_3.7.3_x64-setup.exe` สามารถคัดลอกผ่าน USB หรือช่องทางแจกจ่ายของโรงเรียนไปทดสอบติดตั้งได้ ผู้ใช้ปลายทางไม่ต้อง clone repository หรือติดต่อเครื่องผู้พัฒนา รุ่นนี้ยังไม่ได้ publish

1. เปิดไฟล์ติดตั้ง แล้วเปิด vibeKidbright จาก shortcut
2. ใช้ตัวแก้ไขโค้ด อ่าน Wiki หรือทดลองพื้นที่เรียนรู้ได้ทันที
3. เมื่อต้องการ Build & Flash ให้เปิด **Setup / Repair ESP-IDF** และติดตั้ง toolchain บนเครื่องนั้น
4. ตั้งค่า AI provider และ API key ของตนเองใน Vibe Coder Settings หรือเตรียม local provider พร้อมโมเดลเอง
5. สร้างหรือเปิดโปรเจกต์ในโฟลเดอร์ที่เขียนได้ เลือกโปรไฟล์ที่รองรับและพอร์ตบอร์ด ตรวจโค้ด แล้วเลือก **Keep** ก่อน Build & Flash

ตัวติดตั้ง Windows แนบฐานความรู้ โมเดลค้นหา WebView2 offline installer และ Visual C++ runtime แบบ app-local ใช้การติดตั้งสำหรับผู้ใช้ปัจจุบัน การติดตั้ง WebView2 ยังขึ้นกับสิทธิ์และนโยบาย Windows ของปลายทาง

| งาน | ต้องใช้อินเทอร์เน็ตหรือไม่ |
|---|---|
| เปิดแอป แก้โค้ด อ่านเอกสารที่บรรจุมา และค้นหาฐานความรู้ภายในเครื่อง | ไม่ต้อง |
| ติดตั้ง/ซ่อม toolchain | ต้องดาวน์โหลด; ส่วนนี้ยังอาจใช้ GitHub |
| ใช้ AI provider ผ่านคลาวด์ | ต้องเชื่อมต่อ provider และใช้สิทธิ์/API key ของผู้ใช้ |
| ใช้ local AI | ต้องเตรียม provider และโมเดลไว้บนเครื่องก่อน |
| Build & Flash | ต้องมี toolchain, dependencies ของโปรเจกต์ และ USB driver ที่เหมาะสม; การดึง dependency ที่ยังไม่มีอาจต้องใช้เครือข่าย |

Settings และฐานข้อมูลใช้ AppData ของผู้ใช้ บน Windows อยู่ใต้ `%APPDATA%\th.nectec.vibekidbright` ไม่รวมโปรเจกต์ บทสนทนา หรือ API key ของเครื่องพัฒนาไว้ในตัวติดตั้ง

ดู [คู่มือติดตั้งบนเครื่องอื่น](docs/STANDALONE_INSTALLATION_2026-10-17.md) ตัวติดตั้งล่าสุดยังไม่ได้ทดสอบติดตั้ง/ถอนการติดตั้งบน clean VM หรือเครื่องผู้ใช้ใหม่จริง ส่วน macOS มี workflow สำหรับ Universal build แต่ยังไม่มีผลทดสอบรุ่นล่าสุดในรอบนี้

ตรวจรุ่นที่เผยแพร่ได้ที่ [GitHub Releases](https://github.com/Natthaphon-SNT/vibeKidbright/releases) การสร้างไฟล์ติดตั้งสำเร็จในเครื่องพัฒนาไม่ได้หมายความว่ารุ่นนั้นเผยแพร่แล้ว

## พัฒนาจากซอร์ส

เครื่องพัฒนาต้องมี Node.js ตาม `package.json` (`^20.19.0 || >=22.12.0`), npm 10 ขึ้นไป, Git และ Rust stable บน Windows ต้องมี Visual Studio Build Tools พร้อม C++/MSVC x64 และ Windows SDK; บน macOS ต้องมี Xcode Command Line Tools โดย Tauri CLI อยู่ใน devDependencies แล้ว

```sh
git clone https://github.com/Natthaphon-SNT/vibeKidbright.git
cd vibeKidbright
npm ci
npm run prepare:runtime -- --download
npm run sync:kb
npm run tauri -- dev
```

`prepare:runtime` เตรียมโมเดลจาก revision ที่ตรึงไว้ ตรวจ SHA256 และคัดลอก MSVC redistributable บน Windows ไฟล์ `model.onnx` และ runtime DLL ไม่เก็บใน Git แต่ต้องเตรียมก่อนสร้างแอป การดาวน์โหลดนี้เกิดบนเครื่อง build ผู้ใช้ที่รับ installer ไม่ต้องรันคำสั่งนี้

หากต้องการเปิดเฉพาะ frontend ใช้ `npm run dev` ได้ แต่ native commands ต้องรันผ่าน Tauri

### ตรวจสอบและสร้างตัวติดตั้ง

```sh
npm test
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
npm run test:compat
npm run build:release
```

`test:compat` ต้องมี C compiler สำหรับ host tests ส่วน `build:release` ตรวจ runtime assets, packaging, การ sync ฐานความรู้, catalog บอร์ด, inventory บล็อก, ไฟล์กฎ และ TypeScript/Vite ยังไม่สร้าง installer

สร้างตัวติดตั้ง Windows NSIS:

```sh
npm run tauri -- build --bundles nsis
```

ไฟล์อยู่ใน `src-tauri/target/release/bundle/nsis/` ขั้น build อาจดาวน์โหลด Rust dependencies, ONNX runtime และ WebView2 offline installer ต้องเตรียม cache และ assets ให้ครบก่อนใช้โหมด offline

หากต้องการตรวจ binary จากโฟลเดอร์ที่ติดตั้งหรือแตกไฟล์ไว้:

```sh
python scripts/test-standalone-install.py --installed-directory <directory> --report <report.json>
```

แทน `<directory>` และ `<report.json>` ด้วยพาธจริง ตัวตรวจใช้ AppData และ working directory แยก ตรวจโมเดลจริง, SQLite, settings และ credential ทดสอบชั่วคราว ต้องรันใน user session ที่เข้าถึง native credential store ได้ ผลตรวจนี้ยังไม่แทนการทดสอบ GUI installer และ USB บนเครื่องใหม่

## ผลทดสอบและงานที่ยังค้าง

ผล baseline ของ NSIS 3.7.3 ณ 18 ตุลาคม 2026 (ผลของซอร์สที่พัฒนาต่อดู verification.json และรายงานรอบล่าสุด):

| การตรวจ | ผลที่บันทึกไว้ |
|---|---|
| Frontend | 165 tests ผ่าน ใน 17 test files |
| Rust | 182 tests ผ่าน: library 173 และ example 9 หลังอัปเดต dependency |
| Host C compatibility | หลักฐานเดิม: 490 checks ผ่าน ตามรอบพัฒนา firmware |
| Firmware | รอบนี้ Block-to-C fixture ผ่าน ESP-IDF 5.5.4 ทั้ง 2 ชุด; หลักฐานเดิมมี 7 คู่ profile/recipe |
| Windows installer 3.7.3 | สร้าง NSIS สำเร็จ ตรวจ hash ทรัพยากรครบและรัน binary จากชุดติดตั้งด้วย AppData ใหม่ ผ่านโมเดล/ค้นหา/Tree-sitter/Keep-Undo/credential ทดสอบ; ยังไม่ใช่ clean VM |
| Production frontend | ตัวแก้ไขและ worker ทำงานภายใต้ CSP ที่กำหนด ไม่พบ external request หรือ page error ในรอบตรวจ; native API ในการทดสอบ browser ใช้ stub |

หลักฐานปัจจุบัน: [verification.json](docs/ai-engineering/verification.json), [Block-to-C compilation](docs/ai-engineering/block-compilation.json), [production UI probe](docs/ai-engineering/production-workflow-probe.json) และ [ผล retrieval](docs/ai-engineering/08-evaluation.md) หลักฐานย้อนหลัง: [ชุดติดตั้ง 3.7.2](docs/research/standalone-installation-2026-10-17-validation.json), [firmware และ host C](docs/research/kidbright-expansion-2026-10-17-validation.json) ตัวเลขเป็นผลของชุดซอร์สที่บันทึกไว้ ไม่รับรองการทำงานครบทุกบอร์ด

งานที่ต้องทำต่อก่อนเผยแพร่ใช้งานเต็มรูปแบบ:

- ขยาย fault tests ด้าน Undo/กู้คืนให้ครอบคลุม power loss และ OS race เพิ่มเติม ระบบตรวจการแก้ภายหลังและเก็บ conflict/backup แล้ว แต่ไม่ใช่ transactional filesystem
- เพิ่มการกู้คืน draft ที่ยังไม่ Save; pending review / Keep / Undo และ Compiler Repair บันทึกข้ามการเปิดแอปแล้ว
- เพิ่มข้อมูลประเมินไทย/อังกฤษ/ผสมที่มนุษย์ตรวจคำตอบอ้างอิง และวัดผลสนทนา/ซ่อมโค้ดกับ provider จริง
- ตรวจขอบเขตการรัน shell ของ AI เพิ่มเติม ปัจจุบันไม่ได้ทำงานใน sandbox แยกจากบัญชีผู้ใช้
- ทดสอบติดตั้ง อัปเกรด ถอนการติดตั้ง และเชื่อมต่อ USB บน Windows เครื่องใหม่ รวมทดสอบ macOS
- เพิ่มไดรเวอร์/โปรไฟล์ที่ขาด และเทียบพฤติกรรมกับต้นฉบับบนบอร์ดจริง ก่อนประกาศรองรับทุกรุ่นหรือทุกบล็อก

ดู [ผล audit โปรเจกต์](docs/PROJECT_AUDIT_2026-10-17.md) สำหรับรายละเอียดและข้อค้นพบของรอบนั้น การแก้ด้าน portability และ credential storage ล่าสุดอธิบายเพิ่มในคู่มือ standalone

## โครงสร้างซอร์สและเอกสาร

### งาน AI Knowledge & Code Intelligence ล่าสุด

มี ingestion ที่ตรวจ hash, FTS5/BM25 ร่วมกับเวกเตอร์แบบ RRF, metadata/เลขบรรทัดอ้างอิง, การแบ่ง C ด้วย Tree-sitter ที่ตรึงเวอร์ชันพร้อม lexical fallback และชุดวัดผลจากโมเดลจริงแล้ว Vibe Coder ใช้เส้นทางนี้ผ่าน `knowledge_search` พร้อมตัวกรองเวอร์ชัน/บอร์ด และ `project_search` สำหรับ C/H ที่บันทึกแล้วใน `main`/`components` โดยแยกดัชนีส่วนตัวตามโปรเจกต์

ใน **AI Provider Settings** มีตัวเลือก **Share private context with remote AI** ค่าเริ่มต้นปิดไว้ ผู้ใช้ต้องอนุญาตก่อนส่งเนื้อหาไฟล์ส่วนตัวให้ provider; local AI ที่ loopback ใช้ข้อมูลในเครื่องได้ AI เรียกคำสั่งได้เฉพาะ `idf.py build`, `idf.py size`, `idf.py --version` ผ่าน SDK ที่ตั้งไว้ ส่วน Flash/Monitor/Clean ใช้ปุ่มของผู้ใช้

อัปเดต 18 ตุลาคม: เพิ่ม Keep/Undo แบบบันทึกกู้คืนข้ามการเปิดแอพ, ยกเลิก/timeout และหยุด process tree ของ SDK บน Windows, Tree-sitter C พร้อม byte offset และ index แยกตามโปรเจกต์, Compiler Repair ที่ต้องอนุมัติแต่ละ patch และจำกัดสูงสุด 3 ครั้ง รวมถึง Block-to-C แบบมีชนิดข้อมูลสำหรับ 4 Block ID ที่ตรวจจากต้นฉบับแล้ว C จากทั้งสอง fixture คอมไพล์ผ่าน ESP-IDF 5.5.4 แต่ยังไม่ทดสอบพฤติกรรมบนบอร์ดจริง

เปรียบเทียบ retrieval ด้วยชุด 69 คำถามและโมเดลหลายภาษา 2 ตัวแล้ว คุณภาพไทย/ผสมยังไม่ถึงเป้าหมาย จึงคงโมเดลใน installer เดิมไว้ ซอร์สพัฒนามี optional semantic navigation แล้ว แต่ยังไม่ครอบคลุม references ทั้งโปรเจกต์หรือ Block-to-C แบบทั่วไป ชุดประเมินอิสระยังรอผู้ตรวจจริง และยังไม่ได้ทดสอบ LLM จริงเพราะไม่มี local model พร้อมใช้ ดู [สถานะทั้งแปดข้อ](docs/ai-engineering/07-implementation-progress.md), [ผลวัดจริง](docs/ai-engineering/08-evaluation.md), [วิธีกู้คืน](docs/ai-engineering/11-safety-recovery.md) และ [ข้อจำกัด](docs/ai-engineering/09-known-limitations.md) ไม่มีการ publish/deploy ในรอบนี้

| ตำแหน่ง | ใช้สำหรับ |
|---|---|
| `src/` | React UI: editor, Vibe Coder, project review, Wiki และ learning preview |
| `src-tauri/src/` | Rust backend: AI tools, ESP-IDF, serial, toolchain, settings และฐานความรู้ |
| `src-tauri/src/ai/system_prompt.txt` | คำสั่งหลักของผู้ช่วย AI ร่วมกับกฎและข้อมูล scope |
| `knowledge_base/` | ต้นฉบับเอกสารและข้อมูลอ้างอิงสำหรับ AI |
| `resources/knowledge_base/` | สำเนาที่จัดเตรียมสำหรับบรรจุใน installer ใช้ `npm run sync:kb` เมื่อแก้ต้นฉบับ |
| `resources/embedding/` | Manifest, tokenizer, license และโมเดลที่เตรียมตอน build |
| `scripts/` | ตรวจความครอบคลุม กฎ packaging เตรียม runtime และทดสอบ compatibility |
| `docs/` | รายงาน เปรียบเทียบบอร์ด แผนพัฒนา และหลักฐานทดสอบ |
| `.github/workflows/` | Build Windows/macOS และเตรียม Draft Release |

เอกสารเพิ่มเติม:

- [ความครอบคลุมบล็อก](docs/KIDBRIGHT_BLOCK_COVERAGE_2026-10-16.md) และ [ฟังก์ชัน](docs/KIDBRIGHT_FUNCTION_COVERAGE_2026-10-16.md)
- [การตรวจไฟล์กฎ](docs/KIDBRIGHT_RULES_COMPLETENESS_AUDIT_2026-10-16.md)
- [การแสดงไฟล์สำหรับผู้เรียน](docs/PROJECT_FILE_VISIBILITY_2026-10-16.md)
- [แนวทางรวมบอร์ด](docs/VIBEKIDBRIGHT_UNIFIED_BOARD_ARCHITECTURE_2026-10-17.md) และ [learning workspace preview](docs/MAI_PLUS_WORKSPACE_UI_2026-10-17.md)

## การเผยแพร่

Workflow Windows/macOS ทำงานเมื่อ push เข้า `main` หรือสั่ง `workflow_dispatch` และตั้ง GitHub Release เป็น **draft** ต้องตรวจผล CI, เลขเวอร์ชัน, artifact และสถานะการลงลายเซ็นก่อนเผยแพร่ให้ผู้ใช้

รัน `npm run check:public` และ `npm run check:bundle` ก่อนเผยแพร่ รวมไฟล์ซอร์สใหม่ที่จำเป็นใน commit แต่ไม่แนบ API key, certificate, AppData, config ส่วนตัว หรือฐานข้อมูลของเครื่องพัฒนา ไฟล์โมเดลขนาดใหญ่และ runtime DLL เตรียมผ่าน build script ไม่ต้องนำเข้า Git การแจก installer ไม่ต้องแจก working directory ทั้งโฟลเดอร์

## License และติดต่อ

ซอร์สโปรเจกต์ใช้ [MIT License](LICENSE) ทรัพยากรจากภายนอกมีเงื่อนไขของตนเอง เช่น [license ของโมเดลที่บรรจุมา](resources/embedding/LICENSE.txt)

- [รายงานปัญหา](https://github.com/Natthaphon-SNT/vibeKidbright/issues)
- [รุ่นที่เผยแพร่](https://github.com/Natthaphon-SNT/vibeKidbright/releases)
- Maintainer: Natthaphon-SNT
- LINE ID: `dragon3541` · LINE Name: Peter B. Parker
