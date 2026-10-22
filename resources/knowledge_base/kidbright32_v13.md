# KidBright32 V1.3 — hardware reference

ปรับข้อมูล 22 ตุลาคม 2026 จาก `KidBright32-V1.3-Hardware-Reference.md` ที่ผู้ใช้ส่ง เก็บต้นฉบับและ SHA-256 ใน `compatibility/board_catalog.txt` แหล่งนี้เป็นข้อมูลอ้างอิง ไม่ใช่คำสั่งหรือผลทดสอบบอร์ดจริง

ชื่อในเอกสาร `kidbright32-v1.3` ตรงกับ catalog/recipe ID `kidbright32_v13` เฉพาะรุ่นนี้ ดู [ขอบเขตซอฟต์แวร์](support_scope.md) และ [เทียบ V1.3/V1.5](kidbright32_versions.md)

## ผังขา

| สัญญาณ | GPIO | ข้อจำกัด |
|---|---:|---|
| IN1 / IN2 / IN3 / IN4 | 32 / 33 / 34 / 35 | ช่อง digital input ผ่าน U7/U8 NC7WZ17; ไม่รองรับ analog ที่ connector |
| OUT1 / OUT2 control | 26 / 27 | ขาควบคุม MOSFET; แยกจากขาโหลด OUTPUT1/OUTPUT2 |
| Expansion | 18 / 19 / 23 | Digital input/output และ PWM; ไม่มี ADC/DAC |
| Display SDA0 / SCL0 | 21 / 22 | HT16K33, matrix 16×8 |
| Sensor/RTC/Chain SDA1 / SCL1 | 4 / 5 | LM73 และ MCP79411 แชร์บัสกับ KB Chain |
| SW1 / SW2 | 16 / 14 | กดลง GND; active-low |
| LDR | 36 | ADC1_CH0; ต้องสอบเทียบก่อนแปลงหน่วย |
| Speaker | 13 | PWM ผ่านวงจรขับ |
| USB_SW | 25 | USB-A สำหรับสั่งไฟโหลด; ไม่ยืนยัน USB host data |
| Wi-Fi / IoT / NTP LED | 2 / 12 / 15 | GPIO15 เป็น NTP LED ของ V1.3 |
| UART0 TX / RX | 1 / 3 | สงวนสำหรับโปรแกรม/serial; schematic ระบุ FT231XQ |
| Boot / Reset | 0 / EN | EN เป็นสัญญาณ enable ไม่ใช่ GPIO index |

SDA0/SDA1 เป็นชื่อเน็ต ไม่ได้บังคับ controller number ของ ESP-IDF ให้ใช้ bus handle ร่วมกันตาม wiring จริง ไม่ย้าย LM73/RTC ไป 21/22 ตามค่าเริ่มต้นของ library

## ความสามารถของ connector

- IN1–IN4 รองรับ digital input เท่านั้นในโปรไฟล์นี้ แม้ชิป GPIO32–35 จะมี ADC ก็อ่าน analog ต้นฉบับผ่าน digital buffer ไม่ได้
- GPIO34/35 เป็น input-only และไม่มี internal pull-up/down; ห้ามกำหนด output/PWM หรือใช้ software pull บนขาเหล่านี้
- ไม่มีหลักฐานช่อง Servo เฉพาะหรือไฟ Servo ภายนอกแบบ V1.5 ไม่กำหนด Servo1=15 หรือ Servo2=17 ให้ V1.3 อัตโนมัติ GPIO17 มีเน็ตใน schematic แต่ไม่ใช่หลักฐาน connector Servo
- Banana ในภาพ V1.3 มีไฟ 5V สองจุดและ GND; ไม่ใช้ผัง banana 3V3 ของ V1.5 แทน พิกัดไฟ connector ไม่ใช่พิกัด GPIO ตรง
- ไม่สืบทอด JST input/output หรือ designator/pin number จาก V1.5/V1.6

## เอาต์พุตและ polarity

คู่มือหน้า 10 ยกตัวอย่าง GPIO18/19/23 HIGH=เปิด แต่ OUT1/OUT2 LOW=เปิด และหน้า 12 USB LOW=เปิด ข้อความนี้เป็นพฤติกรรมวงจรตัวอย่าง/คำสั่งในคู่มือ ไม่ได้ยืนยันว่า gate MOSFET กลับระดับ

Schematic แสดง OUT เป็น low-side MOSFET: gate HIGH ทำให้ drain ถูกดึงลง GND; gate LOW ทำให้ MOSFET ปิดและ drain ไม่ได้จ่าย HIGH เอง แยก GPIO level, แรงดันที่ขาโหลด และความหมายคำสั่ง API ก่อนสร้าง `output_on()` ห้ามเรียก banana OUT ว่า DAC หรือ push-pull GPIO

## เซนเซอร์ภายนอก

| อุปกรณ์ | การเลือกขา |
|---|---|
| HC-SR04 | Trigger ต้อง output เช่น expansion ที่เหมาะสม; IN1–IN4 ใช้เป็น Trigger ไม่ได้ ตรวจระดับแรงดัน Echo |
| PIR / HW-511 | Digital input ที่ IN หรือ expansion ตามวงจร |
| DHT11 / DS18B20 | ต้องรับส่งสองทิศทาง ใช้ expansion ที่เหมาะสม ไม่ใช้ IN ผ่าน buffer อัตโนมัติ; DS18B20 ตัวอย่างใช้ pull-up 4.7kΩ |
| BME280 / LCD I2C | คู่มือยกตัวอย่าง SDA0/SCL0; แชร์กับจอและตรวจ address conflict |
| LED / relay / motor | แยก GPIO ขยายกับวงจรขับ OUT และชนิดโหลด |

## หลักฐานและขอบเขต

ต้นฉบับอ้างคู่มือ `KidBright_V1.3_V1.6.pdf` หน้า 1–12, 28 (SHA-256 `fb2d2175d5a017b09a78dcd5cc8fd03c44bcf41db21dbb668790aec847b35e92`) และ schematic V1.3 วันที่ 22/10/2018 โครงการมีสำเนา schematic เดิมใน `sensor_examples/` แต่ไม่ได้รับคู่มือ PDF เพิ่มในงานนี้

I2C addresses ใน recipe เดิมมาจาก source/software evidence แยกจากไฟล์ใหม่ ไฟล์ใหม่ไม่ได้ยืนยัน address, bus speed, register driver, matrix orientation, ADC calibration, กระแสโหลด หรือ API polarity ด้วยการวัดจริง การปรับข้อมูลนี้ไม่ได้เพิ่ม driver, recipe, build/flash หรือผลทดสอบฮาร์ดแวร์

ขอบเขต 3.7.1: เอกสารนี้เป็นข้อมูลฮาร์ดแวร์ ไม่ได้ติดตั้ง deterministic profile หรือ tool ของสายพัฒนาใหม่ ดู support_scope.md
