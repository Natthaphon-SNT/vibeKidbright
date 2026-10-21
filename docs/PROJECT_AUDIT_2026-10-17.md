# ผลตรวจโปรเจกต์ vibeKidbright — 17 ตุลาคม 2026

ตรวจ build, tests, dependency ของ npm, Rust Clippy/formatting, ความสอดคล้องของกฎและ resource bundle และอ่านเส้นทางสำคัญของ editor, AI tools, settings, Keep/Undo, project creation และ CI พบทั้งข้อผิดพลาดที่แก้แล้วและงานคงค้าง จึงยังไม่รับรองว่าโปรแกรมสมบูรณ์ทุกฟังก์ชันหรือทุกรุ่นบอร์ด

รายงานนี้เป็นการตรวจซอร์สและการทดสอบอัตโนมัติบน Windows ไม่ใช่การรับรองความปลอดภัยทั้งระบบ ไม่ได้เปิดเผยค่า API key ของผู้ใช้ ไม่ได้เรียกโมเดลที่เสียเงิน ไม่ได้ flash บอร์ด และไม่ได้ติดตั้ง installer ทับโปรแกรมเดิม

## ข้อผิดพลาดที่แก้ในรอบนี้

| ประเด็น | ก่อนแก้และผลกระทบ | ผลหลังแก้ |
|---|---|---|
| npm dependencies | audit พบ 7 รายการแพ็กเกจ: Critical 2, High 2, Moderate 2, Low 1 | อัปเดต Vitest/coverage-v8 เป็น 4.1.12, DOMPurify override เป็น 3.4.17 และ dependency ทางอ้อม ผล audit หลังแก้เป็น 0 |
| ปิดแท็บที่ยังไม่บันทึก | ปิดแล้วทิ้ง buffer ทันที | ยืนยันก่อนทิ้งการแก้ไข กดยกเลิกแล้วโค้ดยังอยู่ |
| Reload / file-modified | ข้อมูลบนดิสก์สามารถทับโค้ดที่ยังไม่บันทึก รวมถึงการพิมพ์ระหว่างรอ read | การโหลดอัตโนมัติเก็บโค้ดที่ยังไม่บันทึกไว้; โหลดเองต้องยืนยัน; ผล read เก่าไม่ทับข้อความที่เพิ่งพิมพ์ |
| สลับโปรเจกต์ | เปิดโปรเจกต์หรือรับ force-project-dir แล้วล้างแท็บเดิม | เก็บ draft แยกตามโปรเจกต์ใน session และคืนเมื่อกลับมา ไม่มีการ save draft ของโปรเจกต์เก่าลงโปรเจกต์ใหม่ |
| งานอ่านไฟล์ที่ยังไม่เสร็จ | สลับโปรเจกต์ระหว่าง read แล้วแท็บเก่าอาจตามมา หรือกดซ้ำแล้วได้แท็บซ้ำ | ตรวจโปรเจกต์ต้นทางก่อนเปิดผล read และตรวจแท็บซ้ำ |
| Apply AI code | ตั้ง savedContent ก่อน backend ยืนยันการเขียน หากเขียนล้มเหลวหน้าจอกลับแสดงว่า saved | ตั้งสถานะ saved หลังเขียนสำเร็จ; เขียนไม่ได้ยังแสดง dirty พร้อม error |
| AI file tools | join path ตรง ๆ ทำให้ absolute path, ../ หรือ link ออกนอก workspace เข้าถึงไฟล์นอกขอบเขตได้ | ตรวจ relative path และ ancestor ที่มีอยู่จริงก่อนอ่าน/เขียน/list/diff รวมถึง knowledge file; recursive tree/search ข้าม path ที่ออกนอก root; ปฏิเสธ Windows path aliases/device names |
| AI run_command | คำสั่ง exit ไม่ใช่ 0 หรือไม่มี output ยังรายงานเป็น result สำเร็จ และตัด output กลางอักขระไทยได้จน panic | ส่ง error พร้อม exit code และตัดที่ขอบ UTF-8; ใช้ guard pending review/diff เดียวกับ native build และ kill_on_drop สำหรับ shell โดยตรง |
| Diff ไฟล์ใหญ่ | LCS สร้างตารางขนาดจำนวนบรรทัดเก่า × ใหม่โดยไม่จำกัด | จำกัดไม่เกิน 1,000,000 cells; งานใหญ่แสดง changed middle แบบครบทุกบรรทัด โดยละการจัดแนว LCS |
| CI knowledge base | ดาวน์โหลดและ merge ข้อมูลภายนอกที่ไม่ตรึง revision เข้า resources ขณะที่ release build บังคับ bundle ตรงกับ source ใน repo จึงอาจ fail เมื่อมีไฟล์เพิ่ม/ต่าง | Windows/macOS workflows ใช้ knowledge_base ใน repo ที่ผ่าน audit เป็นแหล่งเดียว |
| เวอร์ชันและ warning | Tauri 3.7.1 แต่ npm/Cargo 3.7.0; test helper/import ไม่ถูกใช้งาน | manifest/lock files สอดคล้องที่ 3.7.1; ลบ helper/import ที่ไม่ใช้ |

รายการ dependency อ้างอิงประกาศจากผู้ดูแล [Vitest](https://github.com/vitest-dev/vitest/security/advisories/GHSA-82fw-gwwq-j7x9), [Tinypool](https://github.com/tinylibs/tinypool/security/advisories/GHSA-85c8-ppgw-ccpr) และ [DOMPurify](https://github.com/cure53/DOMPurify/security/advisories/GHSA-6688-9rhm-gjv2) การจัดระดับของแพ็กเกจจาก audit ไม่ได้แปลว่าทุกช่องโหว่ถูกเรียกใช้จาก production app เช่นเดียวกันทั้งหมด เครื่องมือทดสอบอยู่ใน devDependencies

## งานที่ยังคงค้าง เรียงตามความสำคัญ

### P1 — ข้อมูลสำคัญและการกู้คืน

1. **API key ยังถูกเก็บเป็นข้อความอ่านได้ใน config.json**: `src-tauri/src/ai/config.rs` ฟังก์ชัน `set_secure_key` เขียนลง config เสมอแม้ใช้ OS keyring แล้ว ส่วน `migrate_plaintext_keys_on_startup` เป็น no-op ต้องวางทางเก็บ key ที่เข้ารหัส/OS credential store พร้อมแจ้งเมื่อบันทึกไม่ได้ และย้ายข้อมูลเก่าอย่างระมัดระวัง ไม่ลบ key เดิมในรอบตรวจนี้
2. **Undo รายข้อความยังต่างจาก Undo โปรเจกต์ใหม่**: `src-tauri/src/ai_chat.rs::undo_ai_changes` เอา backup ออกจาก map ก่อนคืนไฟล์ กลืนผล write/remove และไม่ตรวจว่าผู้ใช้เปลี่ยนไฟล์หลัง AI หรือไม่ อาจแจ้งสำเร็จทั้งที่คืนไม่ครบ หรือเขียนทับงานใหม่ ต้องเพิ่มการตรวจ conflict และเก็บ backup ไว้เมื่อมีข้อผิดพลาด ส่วน Project Creation Review มี guard อีกชุดและไม่ได้ใช้วิธีเดียวกัน
3. **Keep/Undo, pending diffs และ draft ยังไม่กู้คืนหลัง restart**: `project_review.rs` และ `ai_chat.rs` เก็บ review/diff ใน static memory; draft ใน `App.tsx` เป็น session memory เท่านั้น เมื่อปิดแอปก่อนตัดสินใจ review จะหาย และ draft ที่ยังไม่บันทึกจะหาย ควรมี journal/draft recovery บนดิสก์พร้อมตรวจความขัดแย้ง หน้าจอที่ reload แล้วอ่าน review คืนได้ใน session เดียวไม่ใช่การกู้คืนหลังปิด process
4. **AI run_command ยังเป็น shell ทั่วไป**: `src-tauri/src/ai/tools.rs` เรียก cmd /C หรือ sh -c ด้วยข้อความคำสั่ง จึงไม่ใช่ sandbox และไม่ถูกจำกัดด้วย guard ของเครื่องมือไฟล์ ควรแยกคำสั่ง build/flash เป็น executable + argv ผ่าน native build path และกำหนดขอบเขตคำสั่งสำหรับโหมดผู้เรียน การเพิ่ม kill_on_drop หยุด shell โดยตรง แต่ยังไม่รับรองการหยุด descendant process ทั้งหมดบน Windows

### P2 — ความครอบคลุมและความพร้อมใช้งาน

- **Tests ผ่านยังไม่ครอบคลุมทั้งแอป**: line coverage frontend 44.03% และ branch coverage 41.86% โมดูล `AiChat`, `ToolchainSetup`, `WikiView`, `syntaxLinter` ยังไม่มี coverage จากชุดนี้ ควรเพิ่มสถานการณ์ provider stream/error/cancel, chat history, settings persistence, install failure/retry และ editor lint โดยทดสอบพฤติกรรมที่ผู้ใช้ได้รับ
- **Settings อาจรายงานสำเร็จแม้เขียนไม่ได้**: `write_config` ไม่ส่ง Result ให้ผู้เรียก ใช้ชื่อ temp เดียวและละผล rename; setter commands จึงคืน Ok ได้แม้ดิสก์หรือสิทธิ์ไม่พร้อม ควร serialize การแก้ config และส่ง error กลับ UI
- **ยังไม่เทียบเท่า KidBright IDE ทุกบล็อกและทุกบอร์ด**: catalog เปรียบเทียบ 16 entries แต่มี executable profiles ที่มีหลักฐานตามขอบเขตเพียง 2 profiles; block inventory 560 IDs มี scoped evidence 20 IDs ตัวเลขนี้ไม่ใช่เปอร์เซ็นต์ความสมบูรณ์หรือผล hardware equivalence; ให้ติดตาม [สถานะ implementation](KIDBRIGHT_IMPLEMENTATION_STATUS_2026-10-17.md)
- **μAI/Plus ยังเป็นพื้นที่ฝึกหัดและ UI preview**: ยังไม่เชื่อม training service/board runtime/upload/run จริง และไม่มี bounding-box annotation editor ข้อมูลในหน้านี้ยังไม่ลงโปรเจกต์ ตาม [รายละเอียดพื้นที่เรียนรู้](MAI_PLUS_WORKSPACE_UI_2026-10-17.md)
- **ภาษา UI ยังไม่ทั้งแอป**: ไทย/อังกฤษในพื้นที่เรียนรู้เปลี่ยนได้ ส่วนเมนูและ dialog เดิมยังมีข้อความตายตัว การตอบตามภาษาข้อความล่าสุดของ AI มี unit tests แต่ยังไม่ได้ทดสอบ live provider ทุกภาษา หรือการแสดงอักขระบนฮาร์ดแวร์ทุกแบบ

### P3 — คุณภาพและประสิทธิภาพ

- Clippy ผ่านโดยมี 13 warnings ใน lib และเพิ่มอีก 1 warning ใน test เช่น doc comment, type complexity และ permissions handling ส่วน `cargo fmt --check` ยังไม่ผ่าน รูปแบบโค้ดหลายไฟล์ยังไม่ตรง rustfmt ไม่ได้ format ทั้ง repo เพื่อหลีกเลี่ยง diff ที่ไม่เกี่ยวกับพฤติกรรม
- Vite build ยังเตือน mixed static/dynamic event import และ JS chunk ใหญ่ประมาณ 4.4 MB ก่อน gzip ส่วนใหญ่เกี่ยวกับ editor ควรวัด startup กับเครื่องห้องเรียนจริงก่อนแยก bundle/โหลด editor เมื่อใช้งาน

## ผลตรวจที่ทำจริง

| การตรวจ | ผล |
|---|---|
| Frontend พร้อม V8 coverage | 152 tests, 13 files ผ่าน; lines 44.03%, branches 41.86% |
| Rust `cargo test --offline --all-targets` | lib 123 + example 9 = 132 tests ผ่าน |
| Host C compatibility | portable core 122 + matrix/music 368 = 490 checks ผ่าน |
| npm audit หลังแก้ | 0 vulnerabilities ที่ registry รายงานใน dependency tree นี้ |
| TypeScript / production frontend build | ผ่าน |
| Knowledge bundle / board catalog / coverage / rule inventory / public resources | ผ่านหลังสร้าง inventory ใหม่ |
| Workflow YAML | parse ได้ทั้ง 3 workflows; ไม่ได้รัน GitHub Actions หรือ macOS build จริง |
| Clippy / rustfmt | Clippy ผ่านพร้อม warnings; rustfmt check ไม่ผ่าน |

ไม่มี cargo-audit ในเครื่อง จึงยังไม่ได้ตรวจ Rust dependency กับ RustSec database ไม่ตีความ npm audit 0 ว่า Rust dependencies หรือแอปทั้งระบบไม่มีช่องโหว่ ส่วน firmware build evidence เดิมเป็นผลในรายงานก่อนหน้า ไม่ได้นับเป็นการ flash/test บอร์ดใหม่ในรอบนี้

หลักฐานทดสอบและ hash ของไฟล์ที่แก้อยู่ใน [project audit validation](research/project-audit-2026-10-17-validation.json) รายงานเก่าที่มีจำนวน tests น้อยกว่าเป็น snapshot ของการตรวจครั้งก่อน
