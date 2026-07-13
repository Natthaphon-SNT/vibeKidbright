# KidBright Hardware Schematics & Pinout Rules
> เอกสารอ้างอิงฮาร์ดแวร์และการเชื่อมต่อพินจาก Schematic PDFs

เอกสารนี้รวบรวมกฎการเชื่อมต่อและใช้งานพินที่ได้จาก Schematic PDF ทั้งหมดเพื่อลดข้อผิดพลาดในการพัฒนา Firmware บน Vibe KidBright IDE

---

## 1. KidBright32 V1.5 Rev 3.1 / Rev 3.1G (บอร์ดหลัก)
อ้างอิงจากไฟล์: `Sch_KidBright32_updated.pdf` และ `PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf`

| สัญญาณ / โมดูล | GPIO | หมายเหตุ / กฎการใช้งาน |
|--------------|------|----------------------|
| **I2C Bus 0** | SDA=21, SCL=22 | จอ LED Matrix (HT16K33 `0x70`), RTC (MCP7940N `0x6F`), พอร์ตต่อขยาย |
| **I2C Bus 1** | SDA=4, SCL=5 | เซ็นเซอร์อุณหภูมิ LM73 (`0x4D`) |
| **LDR (แสง)** | 36 (ADC1_CH0) | Input-only ห้าม pull-up |
| **Buzzer** | 13 | ต้องขับด้วย PWM (LEDC) |
| **SW1 (Left)** | 16 | Active LOW (กด=0), ต้อง Pull-up |
| **SW2 (Right)**| 14 | Active LOW (กด=0), ต้อง Pull-up |
| **IN1 .. IN4** | 32, 33, 34, 35 | IN3/IN4 (34/35) เป็น Input-only |
| **OUT1, OUT2** | 26, 27 | Digital I/O |

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
1. **ห้ามเดา Pinout:** ให้ยึดตามเอกสารชุดนี้เป็นที่สิ้นสุด 
2. **การเรียกใช้ ADC:** หากเป็นโค้ดของ Controller ให้ใช้ Legacy API (driver/adc.h)
3. **พอร์ตจำกัด:** GPIO34, 35, 36, 39 บน ESP32 เป็นพอร์ต Input-only ไม่สามารถสร้างสัญญาณ Output หรือ Pull-up ได้ ต้องกำหนดเป็นระดับอินพุตล้วน
4. **ความถี่ Servo:** เซอร์โวบังคับเลี้ยวต้องใช้ PWM ที่ความถี่ 50Hz (คาบ 20ms) โดย Duty cycle มีสัดส่วนตั้งแต่ 1ms ถึง 2ms
