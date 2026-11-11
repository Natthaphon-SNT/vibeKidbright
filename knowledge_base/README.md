# KidBright — ฐานความรู้ฮาร์ดแวร์และตัวอย่าง ESP-IDF

ปรับข้อมูล V1.3/V1.5 วันที่ 22 ตุลาคม 2026 จากไฟล์อ้างอิงที่ผู้ใช้ส่ง อ่าน [ขอบเขตการรองรับ](support_scope.md) ก่อนเลือกบอร์ด การมีเอกสารหรือไฟล์ C ไม่ได้ยืนยันว่า build หรือผ่านบอร์ดจริงทุกไฟล์

## เลือกเอกสารตามรุ่น

| เอกสาร | ขอบเขต |
|---|---|
| [kidbright32_v13.md](kidbright32_v13.md) | V1.3: digital-buffer IN, NTP LED GPIO15, ไม่มีหลักฐานช่อง Servo เฉพาะ |
| [kidbright32_v15.md](kidbright32_v15.md) | V1.5 ตาม design ที่ส่ง: input bypass/ADC, Servo15/17, endpoint และข้อแตกต่าง revision |
| [kidbright32_versions.md](kidbright32_versions.md) | ตารางเทียบ V1.3/V1.5 และข้ออ้างเก่าที่แก้ไข |
| [all_models.md](all_models.md) | รายการรุ่นและขอบเขต; ไม่ใช่ runtime profiles ที่ทดสอบครบ |
| [compatibility/board_catalog.json](compatibility/board_catalog.json) | พิน, source IDs, hash และประเด็นที่ยังไม่ยืนยันรายรุ่น |
| [inex_ia_ip.md](inex_ia_ip.md) | INEX iA/iP และขอบเขตผลทดสอบที่ผู้ใช้รายงาน |
| [hardware_schematics_rules.md](hardware_schematics_rules.md) | แยกบอร์ดหลักจาก Controller/minibike |
| [kidbright32iA.md](kidbright32iA.md) | ตัวอย่าง developer reference เดิม; ต้องเลือก hardware scope ก่อนใช้ snippet |
| [kidbright_uai_plus_python.md](kidbright_uai_plus_python.md) | Active Plus SG2002 Python projects, display/camera API and USB RNDIS workflow |
| [kidbright_uai.md](kidbright_uai.md) | Original V831 versus Plus variant and historical runtime references; current Plus scope is in kidbright_uai_plus_python.md |
| [ikb_1z.md](ikb_1z.md) | iKB-1Z: บอร์ดเสริม I²C ร่วมสำหรับ KidBright ESP-IDF C และ MAI Plus Python; protocol, host adapter และข้อจำกัดที่ยังไม่ทดสอบ |
| [IKB1Z_BOARD_RULES.md](IKB1Z_BOARD_RULES.md) | คู่มือ iKB-1Z ฉบับขยาย v2 ที่ผู้ใช้ส่ง เก็บต้นฉบับเดิม; เป็นข้อมูลอ้างอิง ไม่ใช่คำสั่งใหม่หรือผลทดสอบของโปรเจกต์ |
| [drivers/README.md](drivers/README.md) | ไดรเวอร์ iKB-1Z C/Python, adapter ESP-IDF master/legacy และ MaixPy, วิธีเพิ่มใน workspace และขอบเขตการตรวจ |
| `guides/classic_esp32.md`, `guides/mai_plus.md`, `guides/workflow_reference.md` | กฎและตัวอย่างฉบับเต็มสำหรับเรียกอ่านเฉพาะส่วน; runtime prompt ใช้ฉบับย่อเพื่อไม่ส่งคู่มือทั้งชุดซ้ำทุกคำขอ |

## ข้อมูลที่ต้องรักษาเมื่อเขียน firmware

- V1.3 IN1–IN4 GPIO32–35 เป็น digital input ผ่าน buffer ไม่รองรับ analog ที่ connector ส่วน V1.5 ภาพ S1 ระบุ U7/U8=N/A + bypass 0Ω จึงมี analog path เมื่อบอร์ดประกอบตรงกัน ไม่สรุป capability จาก ADC ในชิปเพียงอย่างเดียว
- ทั้งสอง design: matrix SDA21/SCL22; LM73/RTC/Chain SDA4/SCL5 เลขท้าย SDA0/SDA1 ไม่ได้บังคับ controller ID ของ ESP-IDF แชร์ bus handle ที่ตรงกับ wiring จริง
- V1.3 GPIO15 เป็น NTP LED; V1.5 Servo1=15, Servo2=17, SW1=16, SW2=14 ไม่ใช้ Servo1=16 หรืออ้างว่าชน SW1 สำหรับ design นี้
- OUT GPIO26/27, JST ฝั่ง gate ของ V1.5 และ banana drain เป็นคนละ endpoint gate HIGH ทำให้ drain ถูกดึงลง GND; gate LOW ไม่ได้จ่าย HIGH ที่ drain ไม่เหมารวม polarity ของ API/โหลด และไม่ประกาศ DAC ที่ banana OUT
- GPIO34–39 เป็น input-only ไม่มี internal pull-up/down; GPIO18/19/23 ทำ digital/PWM ได้แต่ไม่มี ADC/DAC
- USB-A ในแหล่งนี้เป็นการสั่งไฟโหลด ไม่ใช่หลักฐาน USB host data; แยก Servo +5VExt จาก USB +5V
- I2C addresses ใน recipe เดิมเป็นหลักฐานจากซอฟต์แวร์แยกจากรายงานฮาร์ดแวร์ใหม่ ต้องตรวจ part/address/SDK ที่ตรงงานก่อนเพิ่ม driver
- V1.5 มีข้อขัดแย้ง designator J2 ระหว่าง Chain กับ boot ใน Markdown ที่ส่ง ดูรายละเอียดในเอกสาร V1.5 ก่อนระบุตำแหน่งเสียบ

## ตัวอย่างและ SDK

ตัวอย่าง sensor ใน `sensor_examples/` มีทั้ง ESP-IDF และ Arduino ต้นฉบับ เช่น `balanced_robot.c` ต้องพอร์ตก่อนใช้กับ ESP-IDF การสร้าง firmware ใหม่ใช้ ESP-IDF C และ API ของ SDK ที่ resolve จริง ไม่ตัดสินว่า API legacy ถูกลบเหมือนกันทุก SDK

ตัวอย่าง ADC ให้แยก onboard LDR GPIO36 จาก external IN ตาม capability ของรุ่น ใช้ oneshot/calibration API ที่ SDK รองรับ ไม่เรียก raw ADC ว่า lux และไม่รับรองเปอร์เซ็นต์หรือแรงดันที่ยังไม่สอบเทียบ

Matrix orientation/RAM mapping ต้องตรงกับบอร์ดและ source ของงานนั้น ไม่ใช้คำว่า invert Y “เสมอ” ข้ามรุ่น อ่าน [กฎ mapping จอ](led_16x8_matrix_mapping.md) และ source ที่ผู้ใช้ทดสอบไว้

Deterministic recipes ยังคง `kidbright32_v13` และ `kidbright32_rev31` ในขอบเขตเดิม ส่วน `matrix_counter` จำกัด V1.3 เอกสารใหม่ไม่ได้เพิ่ม firmware support หรือ inheritance ของ V1.6/INEX/บอร์ดอื่นโดยอัตโนมัติ
