# รายงานการตรวจสอบ vibeKidbright แบบลงมือทำจริง

วันที่จัดทำ: 16 กันยายน 2026 (Asia/Bangkok)  
ประเภทเอกสาร: Hands-on investigation และ post-change verification  
สถานะผลิตภัณฑ์: **Technical Conditional Fail**  
ขอบเขต: ตรวจ source และ Git history, รัน automated tests, production build และจำลองพฤติกรรม component; ไม่มีการทดสอบ Flash/Boot/Serial/ADB กับบอร์ดจริง

> เอกสารนี้แยก “snapshot ตอนเริ่มตรวจ” ออกจาก “สถานะล่าสุดหลังมี commit ใหม่” เพราะ repository เปลี่ยนแปลงระหว่างการสืบสวน หากอ้างผล defect ต้องระบุ commit กำกับเสมอ

## 1. Executive summary

ผลตรวจพบประเด็นสำคัญ 3 เรื่อง:

1. ตอนเริ่มตรวจ `main` และ tag `v3.6.12` ชี้ไปที่ `1669a5d` ซึ่งไม่มีงานจำนวนมากที่เคยถูกทดสอบใน local dirty working tree และ `package.json` ยังเป็น `3.6.10` ต่อมามี commit `5e7b0c6` push งานดังกล่าวขึ้น `main` และย้าย tag `v3.6.12` มาที่ commit ใหม่นี้แล้ว ทำให้ source บน branch, tag และ version metadata ตรงกันใน snapshot ปัจจุบัน
2. defect R1-01 ยังเกิดได้ใน source ล่าสุด: backend คืน `Ok(())` ทันทีหลัง spawn process ขณะที่ UI ตีความการ resolve นั้นเป็น Build & Flash สำเร็จ จึงสามารถแสดง success ก่อน process จบ และคง success แม้ output ต่อมามี compiler error
3. พบ defect ใหม่ RH-01: มี flow ติดตั้ง toolchain สองชุดที่ใช้ state คนละชุด และ backend ไม่มี mutual-exclusion lock ทำให้ `download_toolchain` ถูกเรียกซ้อนและเขียนลง directory เดียวกันได้

ผลตรวจอัตโนมัติของ baseline ล่าสุดผ่านทั้งหมด: frontend 85 tests, Rust 50 tests และ release build แต่ test suite ปัจจุบันยังไม่มี regression test ที่ป้องกัน R1-01 หรือ RH-01

## 2. Baseline และความเปลี่ยนแปลงระหว่างการตรวจ

### 2.1 Snapshot ตอนเริ่ม hands-on investigation

| รายการ | ค่า |
|---|---|
| Branch | `main` |
| HEAD | `1669a5da10360bc6d623f5093422d59b50d7f4f3` |
| Tag `v3.6.12` | ชี้ไปที่ `1669a5d` |
| `package.json` ที่ commit นี้ | `3.6.10` |
| ลักษณะ source | ไม่มี board selector/MiuAiPlus implementation ที่เคยถูกทดสอบในรายงานรอบก่อน |

การ clone repository แบบเต็มและค้นหา history ใน snapshot นี้ไม่พบ `selectedBoard`, `MiuAiPlus` หรือ implementation ของ `kidbright32` นอก system prompt ทั้งที่รายงานรอบ 5 บันทึกว่า local working tree มีไฟล์สำคัญถูกแก้ไขค้างอยู่หลายไฟล์ จึงสรุปได้ว่า baseline ที่ใช้ทดสอบรอบก่อนหน้าไม่สามารถสร้างซ้ำได้จาก remote commit เดียว ณ เวลานั้น

ผลกระทบที่เกิดขึ้นในช่วงดังกล่าว:

- project owner ไม่สามารถตรวจ defect บางรายการจาก source บน remote ได้
- งานที่อยู่เฉพาะ local working tree มีความเสี่ยงสูญหาย
- tag `v3.6.12` และ version ใน `package.json` ให้ข้อมูลไม่ตรงกัน
- R1-02 และ R1-03 ซึ่งเกี่ยวกับ board selection ยังตรวจซ้ำจาก remote snapshot นั้นไม่ได้

### 2.2 สถานะล่าสุดที่ตรวจซ้ำก่อนจัดทำรายงาน

| รายการ | ค่าล่าสุด |
|---|---|
| Local HEAD | `5e7b0c6b61d85a8bcfeb4a760b2534690c4f2e6c` |
| Local tracking ref `origin/main` | `5e7b0c6` |
| Tag `v3.6.12` | ชี้ไปที่ `5e7b0c6` |
| Commit time | `2026-09-16T15:24:24+07:00` |
| Commit message | `feat: update version and improve board detection for KidBright IDE` |
| Working tree ก่อนเพิ่มรายงานนี้ | clean |
| Version metadata | `package.json`, `Cargo.toml`, `tauri.conf.json` = `3.6.12` |
| Board selection | พบ `KidBright32`/`MiuAiPlus`, serial/ADB flow ใน `src/App.tsx` |

ดังนั้นปัญหา “งานอยู่เฉพาะ local และ version metadata ไม่ตรงกัน” ได้รับการแก้ใน snapshot ปัจจุบัน อย่างไรก็ตาม การย้าย release tag ที่เคยเผยแพร่แล้วทำให้ tag ไม่คงที่ (immutable) ผู้ใช้งานที่เคย fetch tag เดิมอาจยังอ้าง object เก่าอยู่ จึงควรบันทึกเหตุผลการย้าย tag และหลีกเลี่ยงการย้าย release tag ในรุ่นถัดไป

## 3. Findings

### 3.1 R1-01 — UI รายงาน Build & Flash สำเร็จก่อน process จบ

Severity: **High**  
สถานะล่าสุด: **Confirmed / Not Fixed**

หลักฐานจาก source ล่าสุด:

- `run_shell_command` ใน `src-tauri/src/esp_idf.rs` spawn process และแยก thread อ่าน stdout/stderr
- backend เรียก `child.wait()` ภายใน detached thread แต่ทิ้ง exit status แล้วคืน `Ok(())` ให้ frontend ทันที
- `handleBuildFlash` ใน `src/App.tsx` await คำสั่งดังกล่าว แล้ว `finally` เปลี่ยน `buildResult` จาก `building` เป็น `success` หากยังไม่ได้รับ rejection
- terminal output ที่เข้ามาภายหลังไม่มีเส้นทางเปลี่ยนผล build จาก success เป็น failed ตาม exit code จริง

การจำลองบน component tree จริงด้วย jsdom, Testing Library และ `userEvent` ให้ผลดังนี้:

1. เปิด project และคลิก `Build & Flash`
2. mock `run_shell_command` ให้ resolve หลัง spawn ตาม contract ของ backend จริง
3. UI แสดง success ทันที
4. ส่ง terminal output ตามหลัง รวมบรรทัด `main.c:42:5: error: 'foo' undeclared...`
5. UI ยังแสดง success อยู่

ความเสี่ยง: ผู้ใช้อาจเข้าใจว่า firmware build/flash สำเร็จทั้งที่ compiler หรือ flashing process ล้มเหลว

แนวทางแก้:

- ให้ backend await process completion และคืน exit code/status จริง
- คืน `Err(...)` เมื่อ exit code ไม่เป็นศูนย์
- emit completion event ที่มี command ID เพื่อป้องกัน output ของหลาย process ปะปนกัน
- ให้ UI แสดง success เฉพาะหลังได้รับ completion status ที่สำเร็จ
- เพิ่ม integration test ครอบคลุม success, compiler failure, flash failure และ process spawn failure

### 3.2 RH-01 — Toolchain install race condition

Severity: **Medium-High**  
สถานะล่าสุด: **Confirmed / Not Fixed**

เงื่อนไขที่ทำให้เกิดปัญหา:

- `AppShell` mount `<App>` และ `<ToolchainSetup mini={true}>` พร้อมกันเมื่อ toolchain ยังไม่พร้อม
- mini widget เริ่ม auto-install หลังนับถอยหลัง 5 วินาที และไม่มี cancel ก่อนเริ่ม
- หน้า IDE มี `Setup / Repair ESP-IDF` และ modal ที่เรียก `download_toolchain` ผ่าน state `isSettingUpEspIdf`
- mini widget ใช้ state `isDownloading` ของตัวเอง ทั้งสอง flow ไม่แชร์ installation state หรือ lock
- backend มีเพียง global `CANCEL_FLAG`; ไม่พบ mutex, semaphore หรือ atomic compare-and-set ที่กัน `download_toolchain` หลาย invocation
- `is_toolchain_ready()` ถูกตรวจครั้งเดียวก่อนสร้าง directory และเริ่มดาวน์โหลด จึงมีช่องว่างแบบ TOCTOU

สถานการณ์จำลอง: ผู้ใช้เปิดแอปใหม่ ระหว่าง countdown กด `Setup / Repair ESP-IDF` และเริ่มติดตั้งจาก modal จากนั้น mini widget เริ่มอัตโนมัติ ทำให้มีสองงานดาวน์โหลด/แตกไฟล์ลง toolchain directory เดียวกัน

ความเสี่ยง: archive หรือไฟล์ที่แตกอาจเสียหาย, progress state สับสน, cancel งานหนึ่งกระทบอีกงาน และ sentinel ถูกสร้างในสถานะที่ไม่สมบูรณ์

แนวทางแก้:

- สร้าง installation coordinator เพียงชุดเดียวและแชร์ state ระหว่าง UI ทั้งสองจุด
- เพิ่ม backend mutex/semaphore ให้มี installer ได้ครั้งละหนึ่ง invocation
- ให้ invocation ที่สองคืนสถานะ `already_in_progress` แทนการเริ่มงานใหม่
- ดาวน์โหลดและ extract ลง temporary directory แล้ว atomic rename เมื่อสำเร็จ
- เขียน sentinel หลัง validation ทุกส่วนผ่านเท่านั้น
- เพิ่ม concurrency test ที่เรียก `download_toolchain` พร้อมกันสองครั้ง

### 3.3 R1-04 — รายการ AI model ที่ต้องทบทวน

Severity: อ้างอิงตาม defect เดิม  
สถานะล่าสุด: **ยังพบใน source**

ใน `src/AiChat.tsx` ยังพบรายการ `gpt-5.6-astra`, `o3-mini`, `o4-mini`, `gemini-1.5-pro` และ `gemini-1.5-flash` ตามที่รายงานก่อนหน้าระบุ ควรตรวจความพร้อมใช้งานจริงกับ provider, account และ API contract ก่อนตัดสินสถานะ defect เพราะ automated tests ปัจจุบันไม่ยืนยัน catalog จากบริการภายนอก

## 4. ผลการตรวจอัตโนมัติบน `5e7b0c6`

| การตรวจ | ผล | หมายเหตุ |
|---|---:|---|
| `npm test -- --reporter=dot` | ผ่าน | 5 files, 85/85 tests |
| `cargo test --manifest-path src-tauri/Cargo.toml` | ผ่าน | 50/50 tests; มี warning unused import/dead code ใน test module |
| `npm run check:public` | ผ่าน | ตรวจ 184 tracked/unignored files |
| `npm run check:bundle` | ผ่าน | ตรวจ 18 bundle resource files |
| `npm run build` | ผ่าน | TypeScript และ Vite production build สำเร็จ |

Build มี non-blocking warnings เรื่อง dynamic/static import ของ Tauri event API และ bundle chunk ใหญ่กว่า 500 kB ข้อเตือนเหล่านี้ไม่ทำให้ build ล้มเหลวและไม่ใช่สาเหตุของ R1-01/RH-01

## 5. ข้อจำกัดของการตรวจ

- ไม่มีบอร์ด KidBright32 หรือ MiuAiPlus สำหรับ Flash/Boot/Serial/ADB/peripheral verification
- live reproduction ของ R1-01 ใช้ component tree จริงใน jsdom และ mock เฉพาะ Tauri boundary ไม่ใช่การคลิก Tauri GUI บนเครื่องผู้ใช้
- ไม่ได้ดาวน์โหลด toolchain ขนาดประมาณ 2.28 GB ซ้ำเพื่อทำ destructive concurrency reproduction; RH-01 ยืนยันจากสอง frontend state machines และ backend synchronization contract
- ค่า `origin/main` ในส่วนสถานะล่าสุดเป็น local tracking ref ณ เวลาตรวจ ไม่ใช่ผลจากการ force-fetch ระหว่างเขียนรายงาน

## 6. ข้อเสนอแนะและลำดับดำเนินการ

1. แก้ R1-01 ก่อน release ถัดไป เพราะ UI รายงานผลสำเร็จผิดจาก exit status จริง
2. แก้ RH-01 ด้วย shared coordinator และ backend lock ก่อนกระจาย onboarding flow ให้ผู้ใช้ใหม่
3. เพิ่ม automated regression tests สำหรับทั้งสอง defect ไม่ใช่อาศัย test suite 85/50 ชุดเดิม
4. สร้าง release tag ใหม่แบบ immutable หลัง CI ผ่าน แทนการย้าย tag เดิมซ้ำ
5. ทำ targeted hardware verification สำหรับ KidBright32 และ MiuAiPlus หลังแก้ source
6. ตรวจ R1-02/R1-03 ใหม่บน baseline `5e7b0c6` เพราะ board-selection source เพิ่งปรากฏบน remote baseline หลัง snapshot แรก

## 7. Release recommendation

**ยังไม่แนะนำให้ถือ `v3.6.12` เป็น release ที่ผ่าน QA สมบูรณ์** แม้ source visibility, version alignment, unit tests และ build gate จะดีขึ้นแล้ว เนื่องจาก R1-01 ยังทำให้ผล Build & Flash ไม่น่าเชื่อถือ และ RH-01 ยังเสี่ยงทำให้ toolchain installation เสียหายได้ การ release ควรรอ source fix, regression tests และ targeted hardware verification
