# iKB-1Z / KidBright / MAI Plus — implementation และการทดสอบ

วันที่ 28 ตุลาคม 2026 (Asia/Bangkok)

## ผลที่ได้

ระบบเลือกกฎตาม host และเพิ่มไดรเวอร์ iKB-1Z ให้โปรเจกต์ใหม่ได้ โดยไม่เปลี่ยน KidBright ESP-IDF C เป็น MAI Plus Python หรือสลับกลับเอง ไดรเวอร์ C/Python ใช้ command/parameter และการตรวจช่วงเดียวกัน แต่ใช้ transport ตาม host ไม่มีผลทดสอบกับบอร์ดจริงหรือการเรียกโมเดล AI จริงในรอบนี้

## การประกอบ prompt และการคง target

- `src-tauri/src/ai/system_prompt.txt`: กฎกลางเรื่องเครื่องมือ, Keep/Undo, ความรู้, private context และ source precedence
- `src-tauri/src/ai/prompts/classic_esp32.md`: กฎและตัวอย่าง C/ESP-IDF ของ classic เท่านั้น พร้อม compatibility contract เฉพาะ target นี้
- `src-tauri/src/ai/prompts/mai_plus.md`: กฎ MaixPy Python และข้อแตกต่าง original μAI/Plus
- `src-tauri/src/ai/prompts/ikb_1z.md`: กฎบอร์ดเสริมร่วม โหลดเมื่อเลือก iKB-1Z ในคำขอหรือ metadata ที่ตรวจแล้ว
- `src-tauri/src/ai/prompts.rs`: ประกอบเฉพาะกฎ target ทุกครั้งที่ provider เรียกโมเดล รวมถึงหลัง tool สร้าง workspace; Gemini compaction ทำหลังประกอบ target ด้วย

คำแก้ target ล่าสุดของผู้ใช้มาก่อน UI การเปลี่ยน UI จริงส่งเป็น boolean แยกจากข้อความแชท ข้อความตอบสั้นใช้ host ที่ยืนยันไว้หรือ project metadata ที่ได้รับสิทธิ์อ่าน การสร้าง workspace เก็บเฉพาะ target/expansion identifier จากผล native โดยตรง จึงเปลี่ยนบริบทได้แม้ปิด private sharing ไม่อ่าน private files และไม่ดึงคำสั่งจาก tool history มาเป็น system prompt

คำเปรียบเทียบหลาย host ยังไม่เลือกภาษา/profile ส่วน original V831 ยังคง deferred เอกสารแนบ, fenced code, tagged reference, blockquote และ logs ไม่ได้ใช้เป็นคำขอเลือก hardware คำสั่งที่ระบุชื่อบอร์ดใน inline quote/backtick ใช้เลือก target ได้ การแก้ target ไม่อนุญาตให้เขียนทับโปรเจกต์คนละ target

## ไดรเวอร์และการสร้าง workspace

อ่าน [คู่มือไดรเวอร์](../knowledge_base/drivers/README.md) และ [ข้อมูลโปรโตคอล](../knowledge_base/ikb_1z.md)

| ส่วน | Implementation |
|---|---|
| Portable C | `ikb1z.h/.c`: validate/encode/decode และ operations ผ่าน callback |
| ESP-IDF | `ikb1z_esp_idf.h/.c`: master หรือ legacy ที่เลือกทั้งแอป; ยืม bus/port และ shared mutex เดิม |
| Python | `ikb1z.py`: IKB1Z กับ MaixI2CTransport ที่รับ existing external bus; ไม่มี import-time I/O |
| Workspace | `expansion_boards=["ikb_1z"]` เพิ่ม Python helper สำหรับ Plus หรือ C component พร้อม CMake dependency สำหรับ classic |
| Driver family | classic ต้องระบุ `ikb_i2c_driver=master/legacy`; recipe เดิมใช้ master และปฏิเสธ legacy ที่จะทำให้ผสม driver family |
| Metadata | เก็บ target และ expansion; custom ESP32 ไม่ได้รับ profile หรือ GPIO ของบอร์ดอื่นโดยอัตโนมัติ |
| KB/runtime | บรรจุ Python ด้วย sync:kb, seeding, file listing, ingestion และ public-source hash; ทดสอบค้น Python แบบ bundled-only โดยไม่เปิด private sharing |

Operations ที่มี: digital input/output/pull-up, ADC8/percent/10, motor M1/M2 -100..100, scaled servo 0..200, stop M1/M2 และ explicit reset ทุก operation ตรวจช่วงและความยาวข้อมูลก่อนรายงาน success ข้อผิดพลาดไม่กลายเป็น ADC=0 หรือ stale reading

UART, servo free และ M3/M4 actuation คืน unsupported พร้อมส่งศูนย์คำสั่ง เพราะ firmware contract/resource sharing ยังไม่ยืนยัน Pure motor codec รองรับ mask 1..15 เพื่อทดสอบรูปแบบไบต์ ไม่ใช่สิทธิ์ใช้ภาคขับภายนอก ไม่มี logging-only success stub หรือคำสั่ง RPM/current/chip ID ที่แต่งขึ้น

## ผลทดสอบที่รัน

| คำสั่ง/ชุดทดสอบ | ผลและขอบเขต |
|---|---|
| `cargo test --lib --offline` | 259 ผ่าน ครอบคลุม prompt routing, workspace expansion, public-only Python retrieval และ native regression suite |
| `npm test -- --reporter=dot` | 225 ผ่าน ใน 22 ไฟล์ รวมการส่งสถานะเปลี่ยนบอร์ดจาก UI |
| `python -m unittest discover -s scripts/tests -p test_*.py` | 91 รายการ: 89 ผ่าน, 2 native parity cases ข้ามเฉพาะคำสั่งนี้; ทั้งสองรันผ่านใน test:ikb ด้านล่าง |
| `npm run test:ikb` | C strict compilation ผ่าน, IDF contract fixture ทั้ง master/legacy ผ่าน, Python 12 tests ผ่านโดยไม่มี skip |
| C/Python parity | ตรวจ I/O 48 คู่, motor 3,015 คู่, servo 12,663 คู่ และ ADC10 word ทั้ง 65,536 ค่า; ไบต์ตรงกันและช่วงที่ผิดถูกปฏิเสธ |
| Transport failure/concurrency | short/failed write, bad read, lock timeout, late native result, exception cleanup, no automatic retry และสอง device ไม่แทรก write/read pair ผ่าน |
| `scripts/build-ikb1z-idf.py` กับ installed ESP-IDF 5.5.4 | master และ legacy compile/link ผ่านทั้งสอง firmware; ไม่ flash หรือรันบนบอร์ด |
| `npm run test:compat` | portable core 122 และ matrix/music 368 checks ผ่าน |
| `npm run build` | TypeScript และ Vite production build ผ่าน |
| KB/boards/standalone/public/bundle/coverage/rules checks | ผ่าน; inventory รวม prompt files ที่ย้ายแล้ว ไม่ลดเนื้อหา coverage เพราะแยกไฟล์ |

ผล SDK/source hash อยู่ใน [validation.json](research/ikb1z-2026-10-28/validation.json) Build log อยู่ใน `src-tauri/target/ikb1z-idf/run-evxrnnmk/` สำหรับตรวจใน workspace นี้ ไฟล์ test stubs อยู่เฉพาะ tests ไม่ได้บรรจุเป็น SDK header หรือใช้แทนผล build SDK จริง

Native build มี dead-code warnings ของส่วนเดิม และ Vite แจ้ง chunk size ของ editor bundle; ทั้งสองไม่ทำให้ build/test ล้มเหลว ไม่มีการเปลี่ยนพฤติกรรมเหล่านั้นในงานนี้

## สิ่งที่ต้องตรวจบนชุดจริง

ไม่มีบอร์ด/การต่อ iKB-1Z ที่ยืนยันและเข้าถึงได้สำหรับทดสอบรอบนี้ จึงยังไม่รับรอง timing, ลำดับการเคลื่อนไหว, ไฟเลี้ยง, runtime I2C backend หรือ waveform บน KidBright/Plus จริง ขั้นตอน bench และข้อมูลที่ต้องบันทึกแยกสอง host อยู่ในคู่มือไดรเวอร์

Maix native I2C ไม่มี timeout argument ที่ยืนยัน ตัว adapter จำกัดเวลารอ lock และปฏิเสธผลล่าช้า แต่หยุด native call ที่ค้างไม่ได้ Process Stop ก็ไม่ได้ยืนยันว่ามอเตอร์หยุดเมื่อบัสขาด ไม่มีหลักฐาน hardware watchdog/automatic stop-on-loss จึงไม่เปิดรับรองคุณสมบัตินี้
