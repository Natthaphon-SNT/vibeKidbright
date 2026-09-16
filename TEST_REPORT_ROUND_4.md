# รายงานผลการทดสอบระบบ vibeKidbright — รอบที่ 4

วันที่ทดสอบ: 16 กันยายน 2026  
เวอร์ชันใน working tree: 3.6.12  
Local/remote HEAD: `1669a5da10360bc6d623f5093422d59b50d7f4f3`  
Branch: `main`  
ระบบปฏิบัติการ: Windows 10.0.26200 x64  
ผู้ทดสอบ: Codex  
เส้นทางที่เลือก: **Minimal-footprint round**  
ผลรวม: **Process Gate Failed / Conditional Fail**

> Working tree มี uncommitted changes ตั้งแต่ก่อนรอบทดสอบ จึงต้องระบุผลนี้ว่าเป็น “version 3.6.12 บน dirty working tree ที่มี HEAD ตามข้างต้น” ไม่ใช่ release candidate ที่ reproduce ได้จาก commit SHA เพียงตัวเดียว

## 1. Process Gate — ตรวจเป็นขั้นตอนแรก

| Gate | ผล | หลักฐาน |
|---|---:|---|
| Dev acknowledgment | **ไม่พบ** | ไม่พบ issue, PR, changelog หรือ commit message ที่อ้างรายงาน/รหัส defect |
| Owner / priority / target version | **ไม่พบ** | GitHub repository มี issues 0 และ PRs 0 จึงไม่มี assignee, label, milestone หรือ target version สำหรับ R1/R3 |
| Source delta ที่อ้าง defect ID | **ไม่พบ** | local และ remote HEAD ยังเป็น SHA เดิมจาก 9 กันยายน 2026; ไม่พบ `R1-xx`/`R3-xx` นอกรายงาน และไฟล์เกี่ยวข้องไม่มี timestamp หลังรายงานรอบ 3 |

ตรวจ remote โดยตรงผ่าน GitHub API แล้ว ไม่ได้อาศัย local clone เพียงอย่างเดียว:

- [Repository issues](https://github.com/Natthaphon-SNT/vibeKidbright/issues)
- [Repository pull requests](https://github.com/Natthaphon-SNT/vibeKidbright/pulls)
- [Repository commits](https://github.com/Natthaphon-SNT/vibeKidbright/commits/main)

ผลทั้งสาม gate ไม่ผ่าน จึงเลือก **หัวข้อ 1 — Minimal-footprint round** ตามพรอมพท์รอบ 4 และไม่รัน full regression, NSIS bundle หรือ ESP-IDF matrix ซ้ำ

## 2. Drift check แบบย่อ

| รายการ | ผลรอบ 4 | เทียบรอบ 3 |
|---|---:|---|
| `npm test -- --reporter=verbose` | **ผ่าน 85/85**, 5/5 files | ตรงกัน ไม่มี drift |
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --nocapture` | **ผ่าน 50/50** | ตรงกัน ไม่มี drift |
| Rust test warnings | 2 รายการ: unused `PathBuf`, unused `temp_store` | ตรงกัน |

เมื่อผลตรงรอบ 3 จึงหยุดตาม gate และไม่รัน `build:release`, coverage, audit, fmt, Clippy, full Tauri/NSIS หรือ firmware matrix ซ้ำ

## 3. MSI reproducibility และ CI identity

### 3.1 Host-context reproduction

เรียก WiX `light.exe -v` โดยตรงจากบัญชี `LAPTOP-UD8ED6TS\Acer` โดย **ไม่ใช้ `-sval`**:

- process ไม่ใช่ Administrator
- ICE validation ผ่าน
- exit code 0
- สร้าง MSI 17,563,648 bytes
- SHA-256 ของ diagnostic artifact: `44A7E71F70B4D49A17141036FADCC80446F3612ACE7D4E11BE52E2A4F0507652`

นี่เป็น host-context success ครั้งที่ 2 ติดต่อกันหลังรอบ 3 จึงไม่ใช่ one-off และยืนยันซ้ำว่า failure ภายใต้ `CodexSandboxOffline` เป็นข้อจำกัด service/identity ของ sandbox ไม่ใช่เพราะ WiX ต้องการ Administrator

Diagnostic MSI/WixPDB ถูกลบหลังเก็บขนาด/hash ไม่ได้เก็บเป็น release artifact

### 3.2 ICE warnings

คำเตือนเดิมปรากฏครบและไม่มี warning ใหม่:

| Defect | ICE | ผลรอบ 4 |
|---|---|---|
| R3-01 | ICE03 — `DownloadAndInvokeBootstrapper` target string overflow | ยังพบ |
| R3-02 | ICE61 — upgrade policy ไม่มี Maximum / อนุญาต downgrade | ยังพบ |
| R3-03 | ICE57 — per-machine component ใช้ HKCU key path | ยังพบ |
| R3-04 | ICE40 — explicit `REINSTALLMODE=amus` | ยังพบ |

### 3.3 GitHub Actions runner

[Build Windows Installer run #34311742528](https://github.com/Natthaphon-SNT/vibeKidbright/actions/runs/34311742528) บน `windows-latest` ที่ HEAD SHA เดียวกันมี conclusion `success`; step `Build Tauri App` ผ่าน และ committed `tauri.conf.json` กำหนด bundle target เป็น `all` จึงเป็นหลักฐานว่า GitHub-hosted Windows identity ไม่พบ sandbox ICE failure แบบ local

ข้อจำกัดของหลักฐาน CI:

- Run นี้ทดสอบ committed HEAD ไม่รวม uncommitted working-tree changes ที่ใช้ในรอบ 4
- Actions API รายงาน retained artifacts 0; จึงไม่ได้ดาวน์โหลด MSI มาตรวจ hash/install
- ยังไม่มี clean install, repair, upgrade, downgrade หรือ uninstall บน VM

## 4. AI catalog update แบบ read-only

ตรวจวันที่ 16 กันยายน 2026 โดยอิง [OpenAI Models](https://developers.openai.com/api/docs/models), [OpenAI All models](https://developers.openai.com/api/docs/models/all), [Google Gemini models](https://ai.google.dev/gemini-api/docs/models), [Google deprecations](https://ai.google.dev/gemini-api/docs/deprecations) และ [OpenRouter Models API](https://openrouter.ai/docs/api/api-reference/models/get-models)

### 4.1 OpenAI direct presets

ไม่พบ preset ที่ “เพิ่ง deprecated เพิ่ม” จากรอบ 3 แต่ปัญหาเดิมยังอยู่:

- `gpt-5.6-astra` ยังไม่ใช่ current model ID; official ID คือ `gpt-6-astra`
- UI ยังไม่มี `gpt-5.6-sol`
- `gpt-4.1-nano`, `o4-mini`, `o3-mini` ยังถูกระบุเป็น deprecated
- `gpt-5.6-terra`, `gpt-5.6-luna`, `gpt-4.1`, `gpt-4.1-mini`, `gpt-4o`, `gpt-4o-mini`, `o3` ยังไม่พบว่า deprecated ใน catalog ที่ตรวจ

### 4.2 Google direct presets

ไม่พบการ deprecated เพิ่มจากรอบ 3:

- `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.5-flash-lite` ยังอยู่ใน current catalog
- `gemini-2.0-flash` ถูก shutdown แล้วตั้งแต่ 1 มิถุนายน 2026
- `gemini-1.5-pro` และ `gemini-1.5-flash` ไม่อยู่ใน current catalog และเป็นรายการปิดใช้งานเดิมที่รายงานไว้แล้ว

### 4.3 OpenRouter live catalog

เรียก public `GET /api/v1/models` ได้ 446 models แล้วเทียบ hard-coded presets 28 รายการ:

- อยู่ใน live list: **15/28**
- ไม่อยู่ใน live list: **13/28**

รายการที่ไม่อยู่ใน live list:

```text
google/gemini-2.5-flash:free
google/gemini-2.5-flash-lite:free
meta-llama/llama-3.3-70b-instruct:free
deepseek/deepseek-chat-v3-0324:free
deepseek/deepseek-r1:free
qwen/qwen3-235b-a22b:free
qwen/qwen-2.5-coder-32b-instruct:free
mistralai/mistral-small-3.2:free
anthropic/claude-opus-4-5
anthropic/claude-sonnet-4-5
anthropic/claude-haiku-3-5
mistralai/devstral-small
mistralai/codestral-2501
```

ข้อสังเกต:

- 8 รายการแรกเป็น `:free` variants ที่หายไป แม้ base model หลายตัวยังมีแบบไม่ฟรี
- Claude IDs ใน UI ใช้ `4-5` ขณะที่ live IDs ใช้รูปแบบ `4.5`; catalog ปัจจุบันมีรุ่นใหม่กว่า และ Haiku ปัจจุบันเป็นตระกูล 4.5
- OpenRouter มี `mistralai/devstral-2512` และ `mistralai/codestral-2508` แทน IDs เดิมที่ UI hard-code
- การไม่อยู่ใน list ไม่ได้เท่ากับประกาศ deprecated เสมอ แต่ไม่ควรนำเสนอเป็น preset ที่รับรองว่าใช้งานได้

นี่เป็นหลักฐานเพิ่มของ **R1-04** ไม่เปิด defect ใหม่ซ้ำ เพราะ root cause เดิมคือ static catalog ที่ไม่มี refresh/validation

## 5. Knowledge Base parity POC

สร้าง proof-of-concept ชั่วคราวนอก repository ด้วย Node.js โดย:

- enumerate ไฟล์แบบ recursive
- normalize relative path เป็น `/`
- exclude `.embeddings.json`, `.backup`, `.db`, `.sqlite`
- คำนวณ SHA-256 ต่อไฟล์
- fail เมื่อ missing, unexpected หรือ hash mismatch

ผลทดสอบ:

| Input | ผล | รายละเอียด |
|---|---:|---|
| fixture ที่ file list/hash ตรงกัน | exit 0 | source 2 / bundle 2, ไม่มี diff |
| KB ของ repository จริง | exit 1 | source 28 / bundle 18, missing 10, hash mismatch 2 |

Missing ใน bundle 10 รายการ:

```text
hardware_schematics_rules.md
minibike.md
sensor_examples/20190924134407-20181127211734-Sch_KidBright32_updated.pdf
sensor_examples/Bike_Controller_V1_20240627.pdf
sensor_examples/calibrate_balancing_bike.c
sensor_examples/calibrate_balancing_joystick.c
sensor_examples/KBminibike_Ext_V0_3.pdf
sensor_examples/minibike_receiver.c
sensor_examples/minibike_sender.c
sensor_examples/PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf
```

Hash mismatch 2 รายการ: `all_models.md`, `README.md`

**ข้อสรุป POC:** แนว parity gate จากรายงานรอบ 3 ทำได้จริงและแยก pass/fail ได้ deterministic ควรย้ายแนวคิดนี้เข้า `scripts/check-kb-parity.mjs`, เพิ่ม fixture tests และเรียกหลัง CI sync แต่ POC รอบนี้ไม่ใช่ source change และถูกลบหลังทดสอบ

## 6. ตาราง defect เทียบ 4 รอบ

| ID | Severity | รอบ 1 | รอบ 2 | รอบ 3 | รอบ 4 |
|---|---:|---|---|---|---|
| R1-01 | High | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |
| R1-02 | High | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |
| R1-03 | High | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |
| R1-04 | Medium | Open | Not Fixed | Not Fixed | **Not Fixed — live evidence expanded** |
| R1-05 | Medium | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |
| R1-06 | Medium | Open | Not Fixed | Not Fixed | **Not Fixed — tests still 85/50** |
| R1-07 | Medium | Open | Not Fixed | Not Fixed | **Not Fixed — POC proves proposed gate** |
| R1-08 | Low | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |
| R1-09 | Low | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |
| R1-10 | Low | Open | Not Fixed | Not Fixed | **Not Fixed — no delta** |

| ID | Severity | รอบ 3 | รอบ 4 |
|---|---:|---|---|
| R3-01 | Medium | Open | **Open — ICE03 reproduced** |
| R3-02 | Medium | Open | **Open — ICE61 reproduced** |
| R3-03 | Low | Open | **Open — ICE57 reproduced** |
| R3-04 | Low | Open | **Open — ICE40 reproduced** |

หมายเหตุด้าน version consistency: working tree มี package/Cargo/Tauri version 3.6.12 แต่ committed HEAD มี `package.json` 3.6.10 ขณะที่ committed `tauri.conf.json` เป็น 3.6.12 และ Winget ล่าสุดเป็น 3.6.0 เป็นหลักฐานเพิ่มของ R1-10 และทำให้ CI result กับ dirty working treeเทียบกันได้ไม่สมบูรณ์

## 7. Defect ใหม่รอบ 4

| ID | ผล |
|---|---|
| — | ไม่พบ defect ใหม่ที่แยกจาก R1/R3 เดิมอย่างถูกต้องได้ |

OpenRouter missing presets จัดอยู่ใน R1-04, KB mismatch อยู่ใน R1-07 และ ICE warnings ยังเป็น R3-01 ถึง R3-04 จึงไม่สร้าง ID ซ้ำเพื่อเพิ่มจำนวน defect

## 8. สิ่งที่ไม่ได้รัน/ยังทดสอบไม่ได้

ตั้งใจไม่รันตามข้อห้ามของ minimal-footprint route:

- `npm run build:release`, coverage, audit, fmt และ Clippy
- ESP-IDF 5-target compile matrix และ KB firmware samples
- full Tauri bundle และ NSIS bundle

ยังขาดทรัพยากร:

- KidBright32 V1.3/V1.5/V1.6 และ MiuAiPlus จริง
- ADB/serial device
- Disposable Windows VM สำหรับ clean install/repair/upgrade/uninstall
- Authorized AI test credentials/quota สำหรับ live request

## 9. คำสั่งหลัก

```powershell
git log -n 10 --date=iso-strict
git status --short
git rev-parse HEAD
rg "R1-(0[1-9]|10)|R3-0[1-4]" --glob "!TEST_REPORT_ROUND_*.md"

npm test -- --reporter=verbose
cargo test --manifest-path src-tauri/Cargo.toml --all-targets -- --nocapture

light.exe -v -ext WixUIExtension.dll -ext WixUtilExtension.dll `
  -o output-round4-host.msi -cultures:en-us -loc locale.wxl main.wixobj

GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/commits
GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/issues
GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/pulls
GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/actions/runs
GET https://openrouter.ai/api/v1/models

node check-kb-parity.mjs <source> <bundle>
```

## 10. Verdict — ควรมีรอบทดสอบถัดไปแบบเดิมหรือไม่

**ไม่ควรมีรอบทดสอบที่ 5 ในรูปแบบเดิม**

Process Gate ไม่ผ่านเป็นรอบที่ 2 ติดต่อกัน และ defect หลักยังมีผล 0 fixed หลัง 4 รอบ การสั่งทดสอบซ้ำโดยไม่มี acknowledgment, owner หรือ defect-referenced commit จะใช้ทรัพยากรโดยไม่เพิ่มความเชื่อมั่นทางเทคนิค

ผู้ตัดสินใจขั้นต่อไปควรเป็น **project owner/repository maintainer ที่มีอำนาจกำหนดงานและอนุมัติ release** ไม่ใช่ผู้ทดสอบ โดยต้องทำอย่างน้อย:

1. เปิด issue สำหรับ R1-01 ถึง R1-10 และ R3-01 ถึง R3-04 หรือจัดกลุ่มโดยยังรักษา traceability
2. กำหนด assignee, priority, milestone/target version และ acceptance criteria
3. ยืนยันว่าได้รับรายงานรอบ 1–4 แล้ว
4. ส่ง commit/PR ที่อ้าง defect ID พร้อม automated tests
5. ระบุ clean release-candidate SHA และทำให้ working tree สะอาดก่อนขอ QA รอบถัดไป

รอบถัดไปควรเริ่มได้เมื่อมีอย่างน้อยหนึ่ง source delta ที่อ้าง defect ID หรือมี project owner ระบุเป็นลายลักษณ์อักษรว่าต้องการทดสอบ environment-only case ใดเพิ่มเติม หากไม่มีเงื่อนไขนี้ ควรหยุดวงจร retest และ escalate ไปยัง project owner

