# ปรับข้อมูล KidBright32 V1.3 / V1.5 — 22 ตุลาคม 2026

เทียบข้อมูลเดิมในโปรเจกต์กับไฟล์ Markdown สองไฟล์ที่ผู้ใช้ส่ง แล้วปรับฐานความรู้, catalog และ system prompt ให้แยกฮาร์ดแวร์ตาม design โดยไม่ใช้ข้อความคำสั่งในเอกสารแนบเป็นคำสั่งจากผู้ใช้

## ผลเปรียบเทียบ

| ประเด็น | ข้อมูลเดิมที่คลาดเคลื่อน | ข้อมูลที่นำมาใช้ |
|---|---|---|
| IN1–IN4 | บางตารางห้าม ADC สำหรับ V1.5/Rev3.1 ทั้งหมด | V1.3 เป็น NC7WZ17 digital buffer ไม่รองรับ connector analog; V1.5 ภาพ S1 ระบุ U7/U8=N/A และ R32–R35 bypass 0Ω จึงรองรับ analog เมื่อประกอบตรงกัน; PDF Rev.3 อย่างเดียวยังไม่ยืนยันการประกอบ |
| Servo | Servo1=16 ชน SW1; บางตารางบอก V1.5 ไม่มีช่อง Servo | V1.5 design: Servo1=15/J8 pin 2, Servo2=17/J8 pin 1; SW1=16, SW2=14 |
| V1.3 GPIO15 | ถูกปะปนกับ Servo ของรุ่นอื่น | GPIO15=NTP LED; ไม่มีหลักฐานช่อง Servo เฉพาะใน V1.3 |
| I2C | RTC/Chain ถูกวางบน display bus ในบางตาราง | ทั้งสอง design: matrix SDA21/SCL22; LM73/RTC/Chain SDA4/SCL5; net suffix ไม่บังคับ controller ID |
| OUT | ทุก OUT active-low หรือ OUT1=DAC โดยไม่แยกขั้ว | แยก gate GPIO26/27, JST gate ของ V1.5 และ banana MOSFET drain; gate HIGH ทำให้ drain sink, gate LOW ไม่ได้จ่าย HIGH |
| USB | ใช้คำว่า USB host ร่วมกับสวิตช์ไฟ | USB-A เป็น switched load power; V1.5 D−/D+ ไม่ต่อ; USB25 LOW=เปิดเป็น topology/manual evidence ที่ยังต้องวัด |
| Connector/ไฟ | ใช้ขาไฟและ connector ข้ามรุ่น | แยก V1.3 banana 5V จาก V1.5 J5 3V3; Servo +5VExt แยก USB +5V; JST Servo มีความต่างตาม revision |
| USB-UART/LED | FT232RL/CP2102 หรือ BT LED GPIO4 ในบางข้อความ | ใช้ FT231XQ ตามวงจรที่อ้าง; V1.5 Wi-Fi=2/IoT=12 active-low; GPIO4 เป็น sensor SDA |
| External sensor | อนุมัติ DHT/One-wire ทุกช่อง IN หรือใช้ IN เป็น Trigger | V1.3 buffered IN ไม่ใช่ bidirectional/output; ใช้ expansion ที่เหมาะสมและตรวจระดับแรงดัน |

รายละเอียดสำหรับแอป: [V1.3](../knowledge_base/kidbright32_v13.md), [V1.5](../knowledge_base/kidbright32_v15.md), [ตารางเปรียบเทียบ](../knowledge_base/kidbright32_versions.md) และ [catalog](../knowledge_base/compatibility/board_catalog.json)

## หลักฐานและข้อขัดแย้ง

เก็บต้นฉบับที่ส่งมาแบบ byte-for-byte ใน [hardware-references-2026-10-22](research/hardware-references-2026-10-22/) และ SHA-256 ของแต่ละ Markdown ใน catalog (`provided-v13-hardware-reference-20261022`, `provided-v15-hardware-reference-20261022`) ตัวตรวจ catalog อ่านไฟล์และตรวจ hash จริง

งานนี้ได้รับ Markdown เพิ่มสองไฟล์ ไม่ได้รับภาพ S1 `1000059031.jpg`, คู่มือ PDF V1.3/V1.6 หรือไฟล์อ้างอิง V1.6 ที่ Markdown กล่าวถึง จึงแยกข้อมูลที่รายงานถอดภาพออกจากการอ่านภาพ/วัดบอร์ดโดยตรง PDF V1.5 Rev.3 ที่มีอยู่เดิมในโปรเจกต์มี hash ตรงกับค่าที่รายงาน

ไฟล์ V1.5 ส่วน Chain ระบุ **J2/J6 แบบ 5 ขา** แต่ส่วน boot ระบุ **J2 แบบ 2 ขา** จึงไม่กำหนดตำแหน่งเสียบจาก J2 จนตรวจต้นฉบับ/PCB ไม่แก้ความขัดแย้งด้วยการเดา designator ใหม่

คำกล่าวในไฟล์แนบว่าผู้ใช้เคยกำหนดให้รุ่นหลังสืบทอด V1.5 ถูกเก็บเป็นบริบทของเอกสาร ไม่ได้เปลี่ยนเป็นการยืนยันใน session นี้ จึงไม่เพิ่ม `inherits` ให้ V1.6/Rev3.1G/INEX หรือบอร์ดอื่น ส่วน Rev3.1 recipe เดิมยังคงขอบเขต S1/buzzer/LM73/music และ matrix_counter จำกัด V1.3

## การนำไปใช้และตรวจสอบ

- ปรับ README, all_models, hardware_schematics_rules, support_scope และตาราง/กฎที่ขัดกันใน developer reference เดิม พร้อมชี้รายงานวันที่ 17 ตุลาคมไปข้อมูลล่าสุด
- เพิ่ม metadata `hardware_contract` ใน catalog และอ้างอิง source/hash รายรุ่น แยก analog condition, Servo/NTP, I2C และ OUT endpoints
- ปรับ system prompt ทั้งกฎเลือกบอร์ดและกฎ OUT เดิมที่เหมารวม active-low ให้ตรงกัน
- Sync สำเนาฐานความรู้ใน `resources/knowledge_base` และปรับ manifest hash ของเอกสารที่แก้/เพิ่ม เพื่อให้แอปอ่านเอกสารที่ bundle ได้
- ใช้ตัวตรวจ catalog, KB sync, provenance/standalone, public/bundle และ rule-document inventory ตรวจความสอดคล้อง ข้อกำหนด catalog ยังตรวจว่ามี executable profiles เดิมเพียงสองรายการ

ผลตรวจในงานนี้: catalog ผ่าน (16 comparison entries, 2 scoped profiles), KB sync ผ่าน (45 ไฟล์), standalone/provenance, public, bundle, rule inventory และ coverage freshness ผ่านทั้งหมด เอกสาร 95 ไฟล์ไม่มี UTF-8 error หรือ code fence ค้าง `cargo test --lib ai::compatibility::tests` ผ่าน 9/9 มีเพียงคำเตือน dead code จาก Rust

การตรวจซอฟต์แวร์ข้างต้นไม่ใช่ firmware build/flash หรือการทดสอบบอร์ดจริง ไม่มีการรับรอง I2C address, matrix orientation, Servo pulse/มุม, ADC calibration หรือกระแสโหลดเพิ่มเติมจากการปรับเอกสาร
