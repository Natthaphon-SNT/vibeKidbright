# KidBright: รุ่นบอร์ดและขอบเขตที่รองรับ

ตรวจข้อมูลวันที่ 17 ตุลาคม 2026

ใช้ [catalog พร้อมแหล่งอ้างอิง](compatibility/board_catalog.json) และ [ขอบเขตการรองรับ](support_scope.md) ก่อนตารางเก่าหรือข้อความว่า compatible การมีชื่อรุ่นไม่ใช่หลักฐานว่ามี driver ครบหรือผ่านบอร์ดจริงแล้ว

| รุ่นหรือชื่อที่ใช้เปรียบเทียบ | Target ที่ยืนยันในงานนี้ | สถานะในแอป |
|---|---|---|
| KidBright V2016 | ESP8266 | ประวัติศาสตร์; ไม่มี ESP32 recipe |
| KidBright V1.0 | ยังไม่ยืนยันตัวตน PCB แยก | ยังไม่มี profile |
| KidBright32 V1.1 / V1.2 | ยังไม่ยืนยันวงจรเฉพาะ revision | มีหลักฐานชื่อรุ่น/USB driver; ยังไม่มี profile |
| KidBright32 V1.3 | ESP32 | `kidbright32_v13` เฉพาะ recipe ที่ระบุ |
| KidBright32 V1.4 | ยังไม่ยืนยันวงจรเฉพาะ revision | ยังไม่มี profile |
| KidBright32 V1.5 ไม่ระบุ revision | ESP32 ตามแบบที่ให้มา | ต้องแยก PCB/revision ก่อนเลือก profile |
| KidBright32 V1.5 Rev3.1 | ESP32 ใน profile เดิม | `kidbright32_rev31` เฉพาะส่วนเดิม; ตัวตน PCB ยังต้องยืนยัน |
| KidBright32 V1.5 Rev3.1G | ยังไม่ยืนยันวงจรเฉพาะ revision | ยังไม่มี profile |
| INEX KidBright32i / V1.5i | ยังไม่ยืนยันวงจรเฉพาะ revision | ยังไม่มี profile |
| INEX KidBright32iA | ESP32 | ผู้ใช้รายงาน Hello/Heartbeat รันได้; ยังไม่มี peripheral recipe ของ iA |
| INEX KidBright32iP | ESP32 | ผู้ใช้ยืนยัน A–Z LED Matrix ผ่านเมื่อ 19 ต.ค. 2026; มี source ใน `inex_ia_ip.md`; ยังไม่มี deterministic profile/ผลทดสอบที่ agent รันเอง |
| Gravitech KidBright32 V1.6 | ESP32 | ยังไม่มี profile |
| Gravitech KidBright Lite V1.1 | ESP32 | ยังไม่มี profile; คนละรุ่นกับ KidBright32 V1.1 |
| INEX OpenKB | ESP32 | บอร์ดอนุพันธ์; ยังไม่มี profile |
| Maker Asia KB32-FT | ESP32 | บอร์ดอนุพันธ์ใช้ TFT จำลอง matrix; ยังไม่มี profile |

ตารางย่อรวม V1.1/V1.2 หนึ่งแถว ส่วน catalog แยก 16 รายการ รวมชื่อคลุมเครือและบอร์ดอนุพันธ์ ไม่ใช่ 16 PCB ที่ทดสอบครบ

## ขอบเขตของ recipe

`button_buzzer`, `temperature_serial`, `music_scale` ใช้กับสอง profile เดิม ส่วน `matrix_counter` จำกัด v1.3 ตาม schematic ที่ระบุรุ่นชัด ใช้ฟอนต์ตัวเลข 5×7 ที่เพิ่มใหม่ ยังไม่ได้ทำ font/Unicode/scroll/scheduler ต้นฉบับครบ

เอกสาร v1.5 ชื่อไฟล์ Rev3_1 แต่ title block ระบุ REV3 จึงไม่เปิด matrix หรือพินใหม่ให้ rev31 อัตโนมัติ ไม่ใช้ความเหมือนของ MCU เลือก profile แทนกัน

## ความต่างที่ต้องแยกรายรุ่น

- v1.3 และแบบ v1.5 ที่ให้มา: display SDA21/SCL22; LM73/RTC SDA4/SCL5; USB-UART ในวงจรเป็น FT231XQ ไม่ใช่ pin map ของทุก revision
- INEX iA ในข้อมูลผู้ใช้และหน้าผู้ผลิตปัจจุบัน: LDR และ MC3479 ผ่าน iA-MC3479 ไม่เลือก KXTJ3 หรือโฟโต้ทรานซิสเตอร์จากตารางเก่า อ่าน [กฎ iA/iP](inex_ia_ip.md)
- INEX iP: โฟโต้ทรานซิสเตอร์; วงจร Rev.2021.04.01 ระบุ S2=GPIO14 และ servo2=GPIO17 คนละสัญญาณกัน ข้อความ LM75 ในบทความกับ LM73-1 ในวงจรต้องเทียบ BOM จริง
- V1.6: หน้าผลิตภัณฑ์หลักระบุ Micro USB ไม่เหมารวม USB-C ส่วน Lite V1.1 ระบุ LSM6DS3TR-C/RGB 4 ดวง ต่างจาก V1.6 ที่ระบุ MPU-6050/RGB 6 ดวง
- KB32-FT ใช้ ST7735 TFT จึงเขียน RAM ของ HT16K33 แทนไม่ได้

อ่าน `source_ids`, `unknowns`, `conflicts` ของรุ่นนั้นก่อนสร้างโค้ด เมื่อไม่มีข้อมูลที่ยืนยันให้ระบุช่องว่าง ไม่เติม GPIO จากรุ่นใกล้เคียง

ผล Hello/Heartbeat บน iA ยังไม่มี serial trace, build log หรือผลทดสอบอุปกรณ์ต่อพ่วง จึงไม่ขยายเป็นการรับรองทุกฟังก์ชันหรือทุกภาษา

KidBright μAI และ μAI Plus เป็นแผนอนาคตหลังทำ KidBright ให้สมบูรณ์ ไม่อยู่ใน implementation/deployment รอบนี้
