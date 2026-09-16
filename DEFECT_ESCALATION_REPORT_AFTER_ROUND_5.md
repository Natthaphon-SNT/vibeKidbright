# รายงานการ Escalate defect หลังการทดสอบ vibeKidbright รอบที่ 5

วันที่จัดทำ: 16 กันยายน 2026 (Asia/Bangkok)  
สถานะเอกสาร: **Prepared / Project-owner acknowledgment pending**  
สถานะผลิตภัณฑ์จากการทดสอบ: **Technical Conditional Fail**  
Process Gate: **Failed 3 รอบติดต่อกัน**  
ขอบเขตข้อมูล: `TEST_REPORT_ROUND_1.md` ถึง `TEST_REPORT_ROUND_5.md`

> เอกสารนี้เป็น escalation และข้อเสนอจัดลำดับงาน ไม่ใช่ผลการทดสอบรอบที่ 6 และไม่ถือว่า project owner ได้ acknowledge, accept priority, assign owner หรืออนุมัติ target version แล้ว จนกว่าจะมีการตอบรับเป็นลายลักษณ์อักษรใน issue/PR/project tracker

## 1. Executive summary

การทดสอบ 5 รอบระหว่างวันที่ 15–16 กันยายน 2026 ยืนยัน defect เดิมรวม 14 รายการ โดยผลล่าสุดคือ:

| สถานะ | จำนวน |
|---|---:|
| Fixed | 0 |
| Partially Fixed | 0 |
| Not Fixed | 14 |
| Regressed | 0 |

แบ่งตาม severity เป็น **High 3 / Medium 6 / Low 5** นอกจากนี้มี R5-01 และ R5-02 เป็น investigation item เกี่ยวกับ environment/build reproducibility ซึ่งยังไม่จัดเป็น blocking product defect

การตรวจสถานะล่าสุดเมื่อ `2026-09-16T15:12:43+07:00` พบว่า:

- local และ remote `main` ยังอยู่ที่ `1669a5da10360bc6d623f5093422d59b50d7f4f3`
- [GitHub Issues](https://github.com/Natthaphon-SNT/vibeKidbright/issues) = 0
- [GitHub Pull Requests](https://github.com/Natthaphon-SNT/vibeKidbright/pulls) = 0
- ไม่มี commit ใหม่ที่อ้าง `R1-xx` หรือ `R3-xx`
- รายงานรอบ 1–5 ยังเป็น untracked files ใน working tree จึงยังไม่มี traceability บน remote repository
- working tree ยัง dirty และ version ใน committed HEAD ไม่สอดคล้องกัน (`package.json` 3.6.10, `Cargo.toml` 0.1.0, `tauri.conf.json` 3.6.12) ขณะที่ working tree ใช้ 3.6.12

**ข้อสรุปสำหรับการบริหารงาน:** ไม่ควรสั่ง full regression รอบใหม่โดยไม่มี defect-referenced source delta การทดสอบซ้ำในสถานะเดิมพิสูจน์แล้ว 5 รอบว่าไม่ทำให้ defect ถูกแก้ สิ่งที่ต้องเกิดถัดไปคือ acknowledgment, assignment, prioritization และ implementation จากฝ่ายพัฒนา/เจ้าของโครงการ

## 2. สิ่งที่ขอให้ project owner ตอบรับ

1. ยืนยันว่าได้รับรายงานรอบ 1–5 และรับทราบ defect 14 รายการ
2. ยืนยันหรือแก้ไข priority, owner role และ target version ที่เสนอในหัวข้อ 4
3. ระบุชื่อผู้รับผิดชอบจริงหรือ team alias สำหรับแต่ละ workstream
4. กำหนด clean release-candidate SHA ที่ต้องการให้ QA ใช้เป็น baseline
5. บังคับให้ commit/PR อ้าง defect ID และแนบ automated test/acceptance evidence
6. แจ้ง QA เมื่อ defect ใดพร้อม targeted verification โดยไม่จำเป็นต้องรอแก้ครบทั้ง 14 รายการ
7. จัดหา disposable Windows VM และบอร์ดจริงหากต้องการปิด hardware/installer acceptance gaps

## 3. Release recommendation

### 3.1 Current recommendation

**No-Go สำหรับ full hardware acceptance และไม่ควรประกาศว่า 3.6.12 ผ่าน QA เต็มรูปแบบ**

เหตุผลหลัก:

- High severity 3 รายการกระทบ core Build/Flash workflow โดยตรง
- UI อาจแสดงผลสำเร็จก่อน process จบ และ MiuAiPlus ใช้ผิด deployment path
- application target support ไม่ตรงกับ README
- component หลักที่ควบคุม workflow มี coverage 0%
- KB source/bundle ไม่ parity และ sample compile ไม่ครบ
- fmt/Clippy quality gates ไม่ผ่าน
- MSI ยังมี ICE warnings 4 รายการและยังไม่ผ่าน install/upgrade matrix บน disposable VM
- ยังไม่มี Flash/Boot/Serial/ADB evidence จากอุปกรณ์จริง

### 3.2 เงื่อนไขขั้นต่ำก่อน targeted QA

Targeted verification เริ่มได้ทันทีเมื่อมีครบทุกข้อสำหรับ defect ที่ส่งมา:

- issue หรือ PR อ้าง defect ID ชัดเจน
- owner และ acceptance criteria
- commit SHA ที่ต้องการทดสอบ
- automated test ใหม่หรือคำอธิบายว่าทำไมทดสอบอัตโนมัติไม่ได้
- working tree/release candidate ที่ reproduce ได้

### 3.3 เงื่อนไขขั้นต่ำก่อน full regression รอบใหม่

- High severity อย่างน้อย R1-01 และ R1-02 มี source delta พร้อม tests
- version metadata ใน committed source ตรงกัน
- CI quality gates ที่เกี่ยวข้องผ่าน หรือมี documented waiver จาก owner
- มี release-candidate SHA ที่ clean และ immutable

## 4. Proposed ownership and prioritization matrix

> ตารางนี้เป็น **ข้อเสนอ** เพื่อให้ owner ตอบรับหรือแก้ไข ค่า `Owner` ทั้งหมดยังเป็น `TBD` และ target version ยังไม่ถือว่าได้รับอนุมัติ

| ID | Severity | Proposed priority | Proposed owner role | Proposed target | Acceptance summary |
|---|---:|---:|---|---|---|
| R1-01 | High | **P0** | Desktop Backend + Frontend | 3.6.x hotfix ถัดไป | UI รอ process completion จริง; exit nonzero ต้องเป็น failed; ป้องกัน concurrent/stale jobs; มี exit 0/2 tests |
| R1-02 | High | **P0** | Hardware Integration + Frontend | 3.6.x hotfix ถัดไป | MiuAiPlus ต้องใช้ deployment contract ที่ถูกต้อง หรือ disable Build & Flash จนรองรับ; KidBright ยังใช้ ESP-IDF |
| R1-03 | High | **P0/P1** | ESP-IDF Integration + Product | เอกสาร hotfix ทันที / feature ใน 3.7.0 | README ต้องตรง capability จริง หรือเพิ่ม target selector/allowlist/cache isolation ครบ 5 targets |
| R1-04 | Medium | **P1** | AI Integration | 3.6.x maintenance | แยก typed catalog; dynamic OpenRouter refresh/cache; deprecated/invalid model handling; catalog tests |
| R1-05 | Medium | **P1** | Firmware Examples / KB | 3.6.x maintenance | ESP-IDF samples 12/12 compile;แก้ misleading indentation และ warnings ที่บันทึกไว้ |
| R1-06 | Medium | **P1** | Frontend + QA Automation | เริ่มพร้อม P0 fixes | เพิ่ม tests สำหรับ Build/Flash state machine, board routing, toolchain, AI provider และ KB failure paths |
| R1-07 | Medium | **P1** | Knowledge Base + CI | 3.6.x maintenance | source/bundle file list และ SHA-256 parity ผ่าน deterministic gate; CI fail เมื่อ mismatch |
| R3-01 | Medium | **P1** | Windows Packaging | ก่อน release ถัดไป | ไม่มี ICE03;ทดสอบเครื่องไม่มี WebView2 และ fallback bootstrapper สำเร็จ |
| R3-02 | Medium | **P1** | Release Engineering | ก่อน release ถัดไป | กำหนด upgrade range ถูกต้อง; downgrade policy ผ่าน install/upgrade/downgrade matrix |
| R1-08 | Low | **P2** | Rust Maintainers + CI | 3.7.0 | `cargo fmt --check` และ Clippy `-D warnings` ผ่านใน CI |
| R1-09 | Low | **P2** | Frontend Performance | 3.7.0 | lazy-load/code-split Monaco;กำหนด bundle budget และ CI gate |
| R1-10 | Low | **P1** | Release Engineering | ก่อน release/tag ถัดไป | package/Cargo/Tauri/Winget version ตรงกัน; manifest URL/hash สร้างจาก release artifact |
| R3-03 | Low | **P2** | Windows Packaging | 3.7.0 หรือก่อน MSI GA | ไม่มี ICE57; multi-user install/repair/uninstall ผ่าน |
| R3-04 | Low | **P2** | Windows Packaging | 3.7.0 หรือก่อน MSI GA | ไม่มี ICE40 หรือมี documented/validated reinstall policy |

## 5. Recommended workstream order

### Workstream A — Core Build/Flash safety

ลำดับ: **R1-01 → R1-02 → R1-03**

นี่คือเส้นทาง critical path เพราะเกี่ยวข้องกับการรายงานผลผิด, การ deploy ผิด board และความไม่ตรงกันของ supported target ควรแก้พร้อม automated tests ก่อนเพิ่ม feature อื่น

### Workstream B — Release and installer integrity

ลำดับ: **R1-10 → R3-01 → R3-02 → R3-03 → R3-04**

หลังแก้ source ให้สร้าง MSI/NSIS บน pinned Windows CI image แล้วทดสอบ clean install, repair, upgrade, downgrade, uninstall, multi-user และ no-WebView2 บน disposable VM

### Workstream C — Knowledge and firmware correctness

ลำดับ: **R1-05 → R1-07**

ทำ sample compile matrix และ deterministic KB parity gate เป็น CI checks เพื่อป้องกันการย้อนกลับ

### Workstream D — Quality and maintainability

ลำดับ: **R1-06 → R1-08 → R1-09**

เพิ่ม regression tests ของ critical workflow ก่อน cleanup lint และ bundle optimization เพื่อให้ refactor มี safety net

### Workstream E — External AI catalog lifecycle

ลำดับ: **R1-04**

แยก static UI labels ออกจาก model availability และให้ provider catalog/cache/fallback จัดการ lifecycle โดยมี offline behavior ชัดเจน

## 6. Defect register และ evidence pointer

| ID | เรื่องย่อ | Evidence หลัก |
|---|---|---|
| R1-01 | UI success ก่อน child process จบ | `src-tauri/src/esp_idf.rs:1028-1056`, `src/App.tsx:1278-1314`, report รอบ 1–5 |
| R1-02 | Build/Flash ไม่แยก board flow | `src/App.tsx` `handleBuildFlash`, report รอบ 1–5 |
| R1-03 | UI target support ไม่ตรง README | `esp_idf.rs:662,1013`, compile matrix 5 targets |
| R1-04 | Static/stale AI catalog | live catalog evidence ใน report รอบ 2, 4, 5 |
| R1-05 | KB sample compile 11/12 | `all_sensors_demo.c:258-259` |
| R1-06 | Critical component coverage 0% | coverage reports รอบ 1, 2, 5 |
| R1-07 | KB source/bundle mismatch | parity POC รอบ 4–5: source 28 / bundle 18 / missing 10 / hash mismatch 2 |
| R1-08 | Rust fmt/Clippy fail | full regression รอบ 1, 2, 5 |
| R1-09 | Frontend bundle ใหญ่ | Vite output รอบ 1–3, 5 |
| R1-10 | Release version/Winget drift | version tables รอบ 1–5 และ Winget Actions failure |
| R3-01 | ICE03 | host `light.exe -v` รอบ 3–5 |
| R3-02 | ICE61 | host `light.exe -v` รอบ 3–5 |
| R3-03 | ICE57 | host `light.exe -v` รอบ 3–5 |
| R3-04 | ICE40 | host `light.exe -v` รอบ 3–5 |

รายงานฉบับเต็ม:

- `TEST_REPORT_ROUND_1.md`
- `TEST_REPORT_ROUND_2.md`
- `TEST_REPORT_ROUND_3.md`
- `TEST_REPORT_ROUND_4.md`
- `TEST_REPORT_ROUND_5.md`

## 7. Non-blocking investigation items

| ID | สถานะ | สิ่งที่ต้องทำ |
|---|---|---|
| R5-01 | Open investigation | pin/hash ESP-IDF, compiler, ccache, sdkconfig และ fixture; เปรียบเทียบ clean output เพื่ออธิบาย firmware-size drift |
| R5-02 | Open investigation | ทำ reproducibility build MSI/NSIS อย่างน้อย 2 ครั้งบน image เดียวกัน; normalize timestamp และ diff archive/CAB contents |

R5-01/R5-02 ยังไม่ควรถูกนับรวมใน blocking defect 14 รายการจนกว่าจะยืนยันว่า drift มาจาก product source ไม่ใช่ environment/toolchain/metadata

## 8. Acceptance and response template

Project owner สามารถตอบกลับด้วยรูปแบบนี้เพื่อเปิด dev-fix loop:

```text
Acknowledged: YES / NO
Acknowledged by: <name or team>
Date: <ISO-8601>
Release-candidate baseline SHA: <commit>

ID: R1-01
Owner: <name/team>
Priority: P0/P1/P2
Target version: <version>
Tracking issue: <URL>
Acceptance criteria approved: YES / changes below
Notes: <text>

Ready for targeted QA IDs: <list or none>
Hardware/VM available: <details or none>
```

## 9. Escalation verdict

**Escalation is justified and remains unresolved.**

หลักฐาน 5 รอบเพียงพอที่จะยืนยันปัญหาเดิม การทดสอบเพิ่มโดยไม่มี source delta จะไม่แก้ defect และแทบไม่เพิ่มข้อมูลด้าน product behavior งานถัดไปต้องเปลี่ยนจาก “QA rerun” เป็น “owner acknowledgment → tracked implementation → defect-referenced commit/PR → targeted verification”

จนกว่าจะมีการตอบรับดังกล่าว สถานะที่ถูกต้องคือ:

- **Acknowledgment:** Pending
- **Owner assignment:** Pending
- **Approved priority/target:** Pending
- **Defect fixes:** 0/14
- **Ready for another full regression:** No
- **Ready for targeted verification:** เมื่อมี defect-referenced commit แรก

## 10. Verification commands for this escalation report

```powershell
git rev-parse HEAD
git branch --show-current
git status --short
git log -1 --format='%H%n%cI%n%s'
Get-Item TEST_REPORT_ROUND_1.md, TEST_REPORT_ROUND_2.md, TEST_REPORT_ROUND_3.md, TEST_REPORT_ROUND_4.md, TEST_REPORT_ROUND_5.md

GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/commits/main
GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/issues?state=all
GET https://api.github.com/repos/Natthaphon-SNT/vibeKidbright/pulls?state=all
```

ไม่มีการรัน product regression ซ้ำและไม่มีการแก้ source code ในการจัดทำเอกสารนี้ เพราะ escalation brief ขอการดำเนินการด้าน ownership/process จาก project owner ไม่ใช่ test plan รอบใหม่
