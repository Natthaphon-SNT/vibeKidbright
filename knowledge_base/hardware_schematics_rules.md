# KidBright Hardware Schematics & Pinout Rules
> **ปรับ V1.3/V1.5 วันที่ 22 ตุลาคม 2026:** ใช้ [ข้อมูลรายรุ่น](kidbright32_versions.md) แทนการรวม revision ตาราง minibike ด้านล่างเป็นชุดมอเตอร์แบบเฉพาะ ไม่ใช่ pinout กลางของปลั๊กอิน BDC/BLDC ดู `compatibility/README.md`
> เอกสารอ้างอิงฮาร์ดแวร์และการเชื่อมต่อพินจาก Schematic PDFs

เอกสารนี้รวบรวมกฎการเชื่อมต่อและใช้งานพินที่ได้จาก Schematic PDF ทั้งหมดเพื่อลดข้อผิดพลาดในการพัฒนา Firmware บน Vibe KidBright IDE

---

## 1. KidBright32 V1.3 และ V1.5 ตาม design ที่ส่ง

อ้างอิง [V1.3](kidbright32_v13.md) และ [V1.5](kidbright32_v15.md) จาก Markdown ที่ผู้ใช้ส่ง ไม่รวม Rev3.1/3.1G เป็นวงจรเดียวกันโดยอัตโนมัติ PDF V1.5 ชื่อไฟล์ Rev3_1 แต่ title block ระบุ Rev.3

| สัญญาณ | GPIO | ขอบเขต |
|---|---|---|
| SDA0/SCL0 | 21/22 | HT16K33 matrix และ expansion ตาม design; ชื่อเน็ตไม่บังคับ controller ID |
| SDA1/SCL1 | 4/5 | LM73, MCP79411 RTC และ KB Chain แชร์บัสนี้; address ใช้หลักฐาน part/software แยก |
| LDR | 36 | ADC1_CH0; input-only ไม่มี internal pull |
| Buzzer | 13 | PWM ผ่านวงจรขับ |
| SW1/SW2 | 16/14 | Active-low; ใช้ pull-up/debounce ให้เหมาะสม |
| IN1–IN4 | 32/33/34/35 | V1.3 digital buffer ไม่รองรับ analog; V1.5 ภาพ S1 มี bypass 0Ω และ U7/U8=N/A; ตรวจการประกอบจริง |
| OUT1/OUT2 control | 26/27 | แยก GPIO/gate, JST gate ใน V1.5 และ banana MOSFET drain |
| GPIO15/GPIO17 | 15/17 | V1.3 GPIO15=NTP LED ไม่มีหลักฐานช่อง Servo เฉพาะ; V1.5 Servo1=15, Servo2=17 |
| USB_SW | 25 | USB-A สำหรับสั่งไฟโหลด ไม่ยืนยัน USB host data |

Gate HIGH ทำให้ drain sink ลง GND; gate LOW ปิด MOSFET ไม่ได้ขับ drain เป็น HIGH ไม่เหมารวมกับข้อความ OUT LOW=เปิดในตัวอย่างคู่มือหรือคำสั่ง API ขาไฟ banana ของ V1.3 กับ V1.5 ต่างกัน และ Servo +5VExt แยกจาก USB +5V ดูตาราง connector รายรุ่นก่อนต่อสาย

---

## 2. Formula Kid / Bike Controller V1
อ้างอิงจากไฟล์: `Bike_Controller_V1_20240627.pdf`

Controller เป็นโมดูล ESP32 เปล่าที่ต่อกับ Joystick และ OLED

| สัญญาณ / โมดูล | GPIO | หมายเหตุ / กฎการใช้งาน |
|--------------|------|----------------------|
| **Joystick X** | 34 | ต่อผ่าน ADC1_CH6 (พอร์ต IN3 เดิม) เป็น Input-only |
| **Joystick Y** | 35 | ต่อผ่าน ADC1_CH7 (พอร์ต IN4 เดิม) เป็น Input-only |
| **S1 (Joystick)**| 36 | สวิตช์ S1 บนจอยสติ๊ก (Input-only) |
| **S2 (Joystick)**| 39 | สวิตช์ S2 บนจอยสติ๊ก (Input-only) |
| **OLED** | SDA=21, SCL=22 | ใช้ไดรเวอร์ SH1106 (`0x3C`) บน I2C_NUM_0 |
| **SW1 (บอร์ด)** | 16 | ปุ่มบนบอร์ด Controller |
| **SW2 (บอร์ด)** | 14 | ปุ่มบนบอร์ด Controller |

> **กฎการแปลง ADC Joystick:**
> จอยสติ๊กถูกติดตั้งแบบกลับทิศทาง ดังนั้นเมื่ออ่านค่า ADC ผ่าน Legacy API จะต้องทำการ Map และ Inverse เครื่องหมายเสมอ

---

## 3. Minibike Extension Board (บอร์ดขับมอเตอร์รถ)
อ้างอิงจากไฟล์: `KBminibike_Ext_V0_3.pdf`

บอร์ดพ่วงสำหรับใส่เข้าไปใน KidBright32iP เพื่อขับมอเตอร์สำหรับรถ Self-Balancing

| สัญญาณ / โมดูล | GPIO | หมายเหตุ / กฎการใช้งาน |
|--------------|------|----------------------|
| **MPU6050 (IMU)**| SDA=4, SCL=5 | I2C_NUM_0 (`0x68`) เพื่ออ่านค่าความเอียงสำหรับการทรงตัว |
| **Motor A ENA** | 26 | พอร์ต OUT1 ขับ PWM ควบคุมความเร็วล้อทรงตัว (Reaction Wheel) |
| **Motor A IN1** | 12 | ควบคุมทิศทาง Phase 1 |
| **Motor A IN2** | 23 | ควบคุมทิศทาง Phase 2 |
| **Motor B ENB** | 27 | พอร์ต OUT2 ขับ PWM ควบคุมความเร็วล้อขับเคลื่อน (Rear Wheel) |
| **Motor B IN3** | 18 | ควบคุมทิศทาง Phase 1 |
| **Motor B IN4** | 19 | ควบคุมทิศทาง Phase 2 |
| **Servo Signal** | 15 | ใช้ LEDC ควบคุมมุมเลี้ยวซ้าย/ขวา ที่ 50Hz |

> **กฎของ DRV8833/L298N บน Minibike:**
> - ล้อ Motor A: ให้ duty-cycle แทนความเร็ว หากต้องการหมุนอีกทางให้สลับ IN1/IN2
> - ต้องทำการ Coast (ตั้ง PWM=0 และ IN=0) เป็นเวลาอย่างน้อย 20ms ก่อนกลับทิศทางมอเตอร์เพื่อป้องกันกระแสกระชาก (Inrush Current)

---

## สรุปข้อระวังสำหรับ AI (System Rules)
1. **ห้ามเดา Pinout:** ตรวจเอกสารผู้ผลิตของ PCB ที่ตรงรุ่น หากข้อมูลในไฟล์นี้ขัดกันให้หยุดยืนยันก่อน ดู `support_scope.md`
2. **การเรียกใช้ ADC:** เลือก API ตาม SDK ที่ติดตั้ง ไม่ผูก legacy ADC กับชื่อ Controller; ตัวอย่าง sender ปัจจุบันใช้ ADC oneshot
3. **พอร์ตจำกัด:** GPIO34, 35, 36, 39 บน ESP32 เป็นพอร์ต Input-only ไม่สามารถสร้างสัญญาณ Output หรือ Pull-up ได้ ต้องกำหนดเป็นระดับอินพุตล้วน
4. **ความถี่ Servo:** ตรวจรุ่น servo และช่วง pulse ที่โปรแกรมต้นฉบับใช้ ตัวอย่าง legacy บางชุดใช้ 0.5–2.5ms ที่50Hz จึงห้ามแทนด้วย 1–2ms โดยไม่แจ้งการเปลี่ยนพฤติกรรม
