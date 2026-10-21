# การพัฒนา KidBright และผลตรวจรอบ 17 ตุลาคม 2026

ลำดับงานที่ผู้ใช้กำหนดคือพัฒนา KidBright ให้สมบูรณ์ก่อน ส่วน μAI และ μAI Plus เป็นแผนอนาคต
รอบนี้แก้เส้นทางใช้งานจริง เพิ่มแกนจอและเพลง และจัดทำการเทียบรุ่นพร้อมหลักฐาน ไม่ได้รับรองว่าเทียบเท่า KidBright IDE ครบแล้ว

## สิ่งที่แก้และเพิ่ม

| ส่วน | การเปลี่ยนแปลง | ขอบเขต |
|---|---|---|
| Build & Flash | บันทึกโค้ดที่แก้ก่อน build, หยุดเมื่อบันทึกไม่สำเร็จ, กันการกดซ้ำและการสลับโปรเจกต์ระหว่างบันทึก | บันทึกเฉพาะไฟล์ผู้เรียนที่แก้จริง ไม่เขียนทับไฟล์ระบบที่เปิดอ่าน |
| เรียก SDK | ส่ง argument แยกให้ backend และ Python/idf.py โดยตรง พร้อม stream stdout/stderr จน process จบ | เลิกประกอบคำสั่ง export.bat ที่เสียเมื่อ path มีช่องว่าง |
| แคช build เก่า | ตรวจ project directory ที่ใช้จริง รวม `-C`; จัดการเฉพาะ `project/build` ที่ตรวจขอบเขตแล้ว | ไม่ลบ custom `-B` หรือโฟลเดอร์ข้างเคียง |
| เพิ่ม/เปลี่ยนชื่อ/ลบไฟล์ | ปรับ SRCS ภายใน managed block ของ CMake และรักษา include/dependency เดิม | รองรับ C/C++/assembly ที่กำหนดไว้; custom/dynamic CMake แจ้งให้ตรวจเอง |
| AI เพิ่ม source | เสนอการลงทะเบียน CMake ผ่าน diff, รักษา dependency ที่เสนอค้างไว้ และรอ Keep | มี pending changes แล้ว build หรือแก้รายการไฟล์ไม่ได้จนกว่าจะตรวจทาน |
| ตรวจบอร์ดก่อน build | ปฏิเสธ profile เสียหรือไม่รู้จัก, recipe ไม่ตรงรุ่น และ target ที่ไม่ใช่ classic ESP32 | ไม่อนุมานว่าบอร์ดที่อยู่ใน catalog มี firmware พร้อมแล้ว |
| จอ | เพิ่ม framebuffer 16×8, pixel, shift, numeric glyph 5×7, HT16K33 packing และ new-driver I2C adapter | ฟอนต์ตัวเลขทำใหม่; ยังไม่ใช่ฟอนต์/Unicode/scroll/scheduler ต้นฉบับ |
| เพลง | เพิ่ม MIDI pitch และการคำนวณระยะเวลาจาก BPM/quarter-note beats | ปัด Hz/ms ตามสัญญาที่เขียนไว้; ยังไม่ใช่ SOUND lifecycle เดิมครบ |
| กฎและรุ่นบอร์ด | catalog พร้อมแหล่งอ้างอิง, unknowns/conflicts และการแยก alias | μAI/Plus อยู่ใน deferred scope เท่านั้น |

## ตัวอย่าง firmware

| Recipe | โปรไฟล์ที่อนุญาต | พฤติกรรม |
|---|---|---|
| `button_buzzer` | v1.3 / rev31 เดิม | กด S1 เล่น A4 440 Hz 500 ms แล้วรอปล่อย |
| `temperature_serial` | v1.3 / rev31 เดิม | อ่าน LM73 ทุก 500 ms และรายงาน I2C error แยกจากอุณหภูมิ |
| `music_scale` | v1.3 / rev31 เดิม | กด S1 เล่น C4..C5 ที่ 120 BPM แล้วรอปล่อย |
| `matrix_counter` | v1.3 เท่านั้น | จอแสดง 00..99 ทุก 500 ms ด้วยตัวเลข 5×7 |

วงจร v1.3 ยืนยัน matrix SDA21/SCL22 ส่วน LM73/RTC ใช้ SDA4/SCL5
วงจร v1.5 ที่มีชื่อไฟล์ Rev3_1 มี title block เป็น REV3 จึงยังไม่ขยาย matrix recipe ให้ profile rev31 เดิมโดยอัตโนมัติ

โปรเจกต์ผู้เรียนยังแสดงเฉพาะ `main` ตามคำขอ ไฟล์ภายในและไลบรารียังคงอยู่บนดิสก์เพื่อ build
ตัวอย่างจอและเพลงติดตั้งโมดูลเพิ่มเฉพาะที่จำเป็น ไม่คัดลอกเอกสารกฎหรือ catalog ลงโฟลเดอร์โปรเจกต์

## การเปรียบเทียบรุ่น

ดู [ตารางรายรุ่น](KIDBRIGHT_BOARD_MATRIX_2026-10-17.md) และ [catalog](../knowledge_base/compatibility/board_catalog.json)
มี 16 รายการสำหรับเปรียบเทียบ รวมชื่อรุ่นกว้าง ชื่อประวัติศาสตร์ และบอร์ดอนุพันธ์ ไม่ใช่ 16 PCB ที่ทดสอบครบ
ชื่อที่ยังไม่ชัด เช่น V1.0, iA/iP แบบย่อ และ revision ย่อย ยังคงมีช่องว่างให้ยืนยันแทนการใช้ GPIO ของรุ่นใกล้เคียง

เพิ่ม [ข้อมูล iA/iP](../knowledge_base/inex_ia_ip.md) จากข้อความและภาพผู้ใช้ พร้อมตรวจหน้าผู้ผลิตอีกครั้ง แยก iA=LDR/MC3479 จาก iP=โฟโต้ทรานซิสเตอร์ ผู้ใช้รายงานว่าโปรแกรมพื้นฐานรันบน iA ผ่านแล้ว สำเนา [main.c](research/kidbright32ia-user-tested-main-2026-10-17.c) มีเพียง Hello/Heartbeat ทุก 1 วินาที จึงบันทึกผลเฉพาะ basic runtime/serial ไม่เพิ่มสถานะผ่านให้จอ เสียง ปุ่ม หรือเซนเซอร์ ดู [หลักฐานพร้อม SHA256](research/kidbright-inex-user-evidence-2026-10-17.json)

## หลักฐานและขอบเขตการทดสอบ

ผลรันจริง จำนวน tests แฮชซอร์สและ firmware อยู่ใน [validation JSON](research/kidbright-expansion-2026-10-17-validation.json)
ผ่าน frontend 139 tests, Rust 119 tests, host C 490 checks (122 เดิม + 368 ใหม่), firmware ESP-IDF 5.5.4 ครบ 7 คู่ profile/recipe ที่อนุญาต และ `npm run build:release` แล้ว การ build นี้ยังไม่ได้สร้างหรือเปลี่ยนตัวติดตั้งแอปบนเครื่องผู้ใช้
ชุด C แยกแกนเดิมออกจาก primitive จอ/เพลงใหม่ จึงไม่เพิ่มจำนวนบล็อกที่รองรับจากการนับ tests
ผล SDK build เป็นหลักฐานว่าคอมไพล์และลิงก์ได้ ไม่ใช่ผลการวัดเสียง ภาพหรือค่าเซนเซอร์จริง
ไม่มีการเรียก API โมเดลจริงหรือ flash บอร์ดโดย agent ในรอบนี้ ผู้ใช้มี schematic v1.3/v1.5 และรายงานผล basic runtime บน iA แยกต่างหาก ยังไม่มีผลทดสอบ peripheral recipes หรือการเทียบพฤติกรรมต้นฉบับบนบอร์ดจริง

## งานที่ยังต้องทำก่อนประกาศว่าเทียบเท่าต้นฉบับ

1. ยืนยัน PCB/revision ของแต่ละรุ่นและทำโปรไฟล์อุปกรณ์/ขา/ทรัพยากรที่มีหลักฐานตรงรุ่น รวมข้อมูลที่ catalog ยังเว้น unknown
2. ทำไลบรารีพื้นฐานที่เหลือ เช่น S2, แสงพร้อมหน่วยและการสอบเทียบ, RTC, I/O/USB control รวม error paths และ resource conflicts
3. พอร์ตฟอนต์/ข้อความ/scroll และ scheduling ของจอและเสียงให้ตรงสัญญาต้นฉบับ รวม variables/functions/multiple tasks และ callbacks
4. ทำชุดทดสอบทั้งเส้นทางผ่าน AI จริง: คำสั่ง → เครื่องมือ → source → UI build → board รวมภาษา การยกเลิก การแก้ไข และการเปิดโปรเจกต์เดิม
5. เปรียบเทียบต้นฉบับกับ firmware ใหม่บนบอร์ดจริง: pixel mapping, waveform/เวลา, press-hold-release, sensor traces และ packet bytes พร้อมค่าความคลาดเคลื่อนที่ตกลง
6. เพิ่ม runtime รายปลั๊กอินและคู่ส่งรับที่ต้องใช้จริง เช่น IMU/encoder/servo/motor/radio/IoT โดยไม่ใช้ logging-only stubs แทน driver
7. หากต้องการใช้แทน IDE เดิมทั้งระบบ เพิ่ม project/Blockly import ที่รักษา variables/mutations/root stacks และแจ้ง unknown blocks รวม simulator adapter แยกต่างหาก
8. เก็บสถานะ pending review ให้กู้คืนได้หลังปิดแอป และทดสอบตัวติดตั้งกับเครื่องสะอาด

μAI และ μAI Plus เริ่มหลังงาน KidBright ผ่านเกณฑ์ที่ตกลงแล้วเท่านั้น เอกสารอ้างอิงเดิมยังเก็บไว้สำหรับระยะนั้น
