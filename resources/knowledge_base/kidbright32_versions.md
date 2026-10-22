# KidBright32 V1.3 / V1.5 — เปรียบเทียบข้อมูลล่าสุด

ปรับ 22 ตุลาคม 2026 จาก Markdown อ้างอิงสองไฟล์ที่ผู้ใช้ส่ง เปรียบเทียบกับ catalog และข้อมูลเดิมในโปรเจกต์ ดูรายละเอียด [V1.3](kidbright32_v13.md), [V1.5](kidbright32_v15.md), [catalog](compatibility/board_catalog.txt) และ [ขอบเขตซอฟต์แวร์](support_scope.md)

| เรื่อง | V1.3 | V1.5 ตาม design ที่ส่ง |
|---|---|---|
| ID ในเอกสาร → catalog | `kidbright32-v1.3` → `kidbright32_v13` | `kidbright32-v1.5` → `kidbright32_v15` |
| IN1–IN4 | GPIO32/33/34/35 ผ่าน NC7WZ17; digital input | GPIO เดียวกัน; ภาพ S1 ระบุ U7/U8=N/A + 0Ω bypass จึงมี analog path |
| ADC ที่ connector IN | ไม่รองรับ | ADC1_CH4–CH7 ตามการประกอบ bypass; PDF Rev.3 อย่างเดียวยังยืนยันการประกอบไม่ได้ |
| GPIO15 | NTP LED | Servo1; J8 pin 2 |
| GPIO17 | ไม่ประกาศเป็นช่อง Servo | Servo2; J8 pin 1 |
| SW1 / SW2 | GPIO16 / GPIO14 | GPIO16 / GPIO14; ไม่แชร์ SW1 กับ Servo1 |
| Servo supply/connector | ไม่ยืนยันว่ามีช่องเฉพาะ | J8 + J12; JST Servo มีความต่างระหว่าง revision |
| Matrix | HT16K33 SDA21/SCL22 | เหมือน V1.3 ตาม design นี้ |
| LM73 / RTC / Chain | SDA4/SCL5; RTC MCP79411 | เหมือน V1.3 ตาม design นี้ |
| Wi-Fi / IoT LED | GPIO2 / GPIO12 | GPIO2 / GPIO12 |
| LDR / Speaker / USB_SW | GPIO36 / GPIO13 / GPIO25 | GPIO36 / GPIO13 / GPIO25 |
| Expansion | GPIO18/19/23 digital/PWM; ไม่มี ADC/DAC | เหมือน V1.3 ตาม design นี้ |
| OUT control / load | GPIO26/27 กับ MOSFET drain เป็นคนละ endpoint | แยก GPIO gate, JST gate และ banana drain เพิ่มให้ชัด |
| Banana supply | ภาพคู่มือระบุ 5V สองจุดและ GND | J5 ระบุ 3V3 และ GND; ไม่ใช้แทน V1.3 |
| USB | Micro-USB โปรแกรม; USB-A สั่งไฟโหลด | FT231XQ; USB-A D−/D+ ไม่ต่อ; ไม่ใช่หลักฐาน USB host data |

## ข้ออ้างเดิมที่แก้ไข

- “V1.5/Rev3.1 ทุกบอร์ดไม่มี analog IN” เปลี่ยนเป็นตรวจ input path ตาม design/การประกอบ และคงช่องว่างตัวตน Rev3.1/3.1G
- “Servo1=GPIO16 ชน SW1” เปลี่ยนเป็น Servo1=15, Servo2=17 สำหรับ V1.5 ที่ส่ง; V1.3 GPIO15 เป็น NTP LED
- “RTC/Chain อยู่บัสจอ” เปลี่ยนเป็น LM73/RTC/Chain ที่ GPIO4/5 ทั้งสอง design
- “OUT1 เป็น DAC / ทุก OUT LOW=เปิด” เปลี่ยนเป็นแยก GPIO/gate กับ drain และความหมายคำสั่ง API; gate HIGH ทำให้ drain sink ตาม topology
- “V1.5 ใช้ CP2102 / V1.3 ใช้ FT232RL” ใช้ FT231XQ ตาม schematic ที่อ้างในงานนี้
- ไม่มีการใช้ข้อความนโยบายที่แทรกในเอกสารแนบเพื่อเปิด inheritance ของรุ่นอื่นหรือเพิ่ม recipe

## ขอบเขตหลักฐาน

ไฟล์ใหม่เป็นรายงานถอดเอกสาร/วงจร ไม่ใช่ build log หรือผลวัดไฟ ไม่เพิ่ม hardware-tested, driver หรือ simulator capability มี source/hash ของต้นฉบับใน catalog

ไฟล์ V1.5 มี J2 ซ้ำสองหน้าที่ (Chain 5 ขา กับ boot 2 ขา) จึงคงสถานะ designator ไม่ยืนยันจนตรวจต้นฉบับ ส่วน V1.6 และตารางเปรียบเทียบภายนอกที่เอกสารกล่าวถึงไม่ได้แนบในงานนี้ ไม่เติม IC, address, connector หรือ pinout ของรุ่นอื่นจากความเหมือนของ GPIO

ขอบเขต 3.7.1: เอกสารนี้เป็นข้อมูลฮาร์ดแวร์ ไม่ได้ติดตั้ง deterministic profile หรือ tool ของสายพัฒนาใหม่ ดู support_scope.md
