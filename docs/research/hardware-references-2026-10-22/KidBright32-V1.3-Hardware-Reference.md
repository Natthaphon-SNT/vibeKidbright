# KidBright32 V1.3 — ข้อมูลฮาร์ดแวร์สำหรับ vibeKidbright IDE

## 1. ขอบเขตและหลักฐาน

- Profile: `kidbright32-v1.3`; แยกจาก `kidbright32-v1.5` และ `kidbright32-v1.6`
- **M13:** `KidBright_V1.3_V1.6.pdf` หน้า 1–12 และตารางหน้า 28 ที่ผู้ใช้แนบ
- **S13:** [Schematic V1.3 จากเว็บไซต์ KidBright](https://www.kid-bright.org/download/1940/), หน้า 1, กรอบแบบ `22/10/2018`, `V 1.3`; เปิดอ่านประกอบ 22 ตุลาคม 2026
- **D1:** [Espressif GPIO/ADC reference](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/peripherals/gpio.html) สำหรับความสามารถขาชิป ไม่ใช่ความสามารถของ connector หลังวงจรบอร์ด
- SHA-256 ของ M13: `fb2d2175d5a017b09a78dcd5cc8fd03c44bcf41db21dbb668790aec847b35e92`
- สถานะ: ตรวจเอกสารและวงจร ยังไม่ได้ต่อบอร์ด วัดไฟ build หรือ flash

เอกสารนี้ไม่ได้ติดตั้ง BSP หรือ driver ใน repository และไม่ถือว่า V1.3 สืบทอดคุณสมบัติ Servo/analog input ของ V1.5

## 2. GPIO และหน้าที่

| หน้าที่ | GPIO | หลักฐาน / หมายเหตุ |
|---|---:|---|
| IN1 | 32 | M13 หน้า 28; ใช้เป็น digital input ที่ช่อง IN |
| IN2 | 33 | M13 หน้า 28; ใช้เป็น digital input ที่ช่อง IN |
| IN3 | 34 | M13 หน้า 28; input-only ที่ชิป |
| IN4 | 35 | M13 หน้า 28; input-only ที่ชิป |
| OUT1 control | 26 | M13 หน้า 28; แยก GPIO จากขาโหลดที่ผ่านวงจร |
| OUT2 control | 27 | M13 หน้า 28; แยก GPIO จากขาโหลดที่ผ่านวงจร |
| Expansion IO18 | 18 | M13 หน้า 2–8, 10–11, 28 |
| Expansion IO19 | 19 | เช่นเดียวกับ IO18 |
| Expansion IO23 | 23 | เช่นเดียวกับ IO18 |
| SDA0 / SCL0 สำหรับจอ | 21 / 22 | S13; HT16K33 |
| SDA1 / SCL1 สำหรับเซนเซอร์/Chain | 4 / 5 | S13; LM73 และ RTC อยู่บัสนี้ |
| S1 / SW1 | 16 | S13; ปุ่มกดลง GND |
| S2 / SW2 | 14 | S13; ปุ่มกดลง GND |
| LDR / ADC | 36 | S13; ADC1_CH0 ตาม D1 |
| Speaker PWM | 13 | S13; ผ่านวงจรขับ |
| USB_SW | 25 | S13; M13 หน้า 12 อธิบายการสั่งไฟ USB |
| Wi-Fi LED | 2 | S13 |
| IoT LED | 12 | S13 |
| NTP LED | 15 | S13; เป็นจุดที่ต่างจาก Servo1 ของ V1.5/V1.6 |
| GPIO17 | 17 | S13 มีเน็ต GPIO17; ไม่ประกาศเป็นช่อง Servo สำเร็จรูปของ V1.3 |
| UART0 TX / RX | 1 / 3 | ขา ESP32; สงวนสำหรับการโปรแกรม/serial ตามวงจร |
| Boot | 0 | IO0; เกี่ยวข้องกับ download mode |
| Enable / Reset | EN | ไม่ใช่หมายเลข GPIO |

ชื่อ SDA0/SDA1 เป็นชื่อเน็ต ไม่บังคับหมายเลข controller ใน ESP-IDF; LM73/RTC/Chain ต้องแบ่งปันบัส GPIO4/5 ตาม wiring ไม่ย้ายไป GPIO21/22 เพียงเพราะตัวอย่าง library ใช้ค่าเริ่มต้นนั้น

## 3. ความสามารถของช่องบนบอร์ด

| Endpoint | Digital input | Digital output | Analog input | วิธีตีความ output |
|---|---|---|---|---|
| IN1–IN4 | ได้ | ไม่ประกาศรองรับ | **ไม่ได้ตาม M13 หน้า 28** | เป็นช่องรับข้อมูล |
| OUT1–OUT2 | ไม่ประกาศรองรับ | ได้ | ไม่ประกาศรองรับ | โหลดผ่านวงจรเอาต์พุต ไม่ใช่ GPIO เปล่า |
| GPIO18/19/23 | ได้ | ได้ | ไม่ได้ | ทำ PWM ได้; ไม่ใช่ DAC |

GPIO32–35 มีวงจร ADC ในตัวชิป แต่ไม่ได้ทำให้ช่อง IN ของ V1.3 เป็น analog input โดยอัตโนมัติ S13 แสดง U7/U8 เป็น NC7WZ17 ซึ่งเป็นเส้นทาง digital buffer ต่างจากภาพ V1.5 ที่ระบุ U7/U8=N/A พร้อมตัวต้านทาน bypass ห้ามนำ `adc_oneshot_read()` มาอ่านช่อง IN1–IN4 แล้วอ้างว่าเป็นค่าต้นฉบับเซนเซอร์แบบ analog ของ V1.3

GPIO34/35 ไม่มี internal pull-up/down ตาม D1 ห้ามตั้ง output/PWM หรือพึ่ง internal pull บนขาเหล่านี้ การใช้ขาของชิปโดยข้ามวงจรบอร์ดต้องจัดเป็นการดัดแปลงฮาร์ดแวร์อีกโปรไฟล์

## 4. อุปกรณ์บนบอร์ดและคอนเนกเตอร์

M13 หน้า 9 ระบุจอ LED 16×8, ปุ่มผู้ใช้ 2 ตัว, LDR, เซนเซอร์อุณหภูมิ, speaker, Reset, LED สถานะ, micro-USB, USB-A, KB Chain, banana jack, จุดบัดกรีขยาย, ESP-WROOM-32 และแบตเตอรี่ CR2032 สำหรับ RTC

| กลุ่ม | ข้อมูลที่ใช้สร้างโปรไฟล์ |
|---|---|
| LED matrix | 16×8 ผ่าน HT16K33; ใช้ SDA0/SCL0 |
| Temperature | LM73; ใช้ SDA1/SCL1 |
| RTC | MCP79411 ตาม S13; ใช้ SDA1/SCL1; มีแบตเตอรี่สำรอง |
| Banana input | IN1–IN4 → GPIO32/33/34/35 ผ่านวงจรอินพุต |
| Banana output | OUT1/OUT2 → วงจรขับจาก GPIO26/27 |
| ไฟที่ banana | ภาพ V1.3 แสดง 5V สองจุดและ GND; ไม่ใช้แผนผัง banana 3V3 ของ V1.5 แทน |
| Expansion | GPIO18/19/23 และ SDA0/SCL0 ตามภาพบอร์ด |
| KB Chain | SDA1/SCL1; มี power/GND; ตำแหน่งขาต้องเทียบ schematic/PCB โดยตรง |
| Micro-USB | อัปโหลดโปรแกรมและจ่ายไฟ |
| USB-A | สั่งจ่ายไฟให้โหลด USB; ไม่ยืนยัน USB host data |
| Servo | คู่มือส่วน V1.3 ไม่แสดงช่อง Servo เฉพาะหรือ terminal ไฟ Servo |
| JST input/output | ไม่ประกาศว่ามีแบบ V1.6 จากเอกสารนี้ |

ไม่เติมเลข designator หรือ pin number ของ connector ที่คู่มือไม่ได้แสดงชัดจากความจำของ V1.5 ให้ใช้ชื่อช่องบนบอร์ดและตรวจ pin 1 จาก schematic/PCB ก่อนกำหนด footprint

## 5. Polarity และข้อขัดแย้งของคำอธิบาย

M13 หน้า 10 อธิบาย GPIO18/19/23 HIGH=เปิด, LOW=ปิดสำหรับวงจรตัวอย่าง ส่วน OUT1/OUT2 ระบุ LOW=เปิด, HIGH=ปิด และหน้า 12 ระบุ USB LOW=เปิด

ต้องเก็บข้อความนี้เป็น **พฤติกรรมตัวอย่างในคู่มือ** ไม่เหมารวมเป็นระดับ gate ของทรานซิสเตอร์ทุกจุด โดย S13 แสดง OUT1/OUT2 ผ่าน MOSFET low-side: gate HIGH ทำให้ drain ถูกดึงลงกราวด์ จึงมีความต่างระหว่าง `GPIO level`, `แรงดันที่ขาโหลด`, `วงจร LED ที่ต่อ` และ `ค่าคำสั่ง API`

ก่อนสร้าง `output_on()` ให้ตรวจ implementation ของ API และวงจรโหลดจริง อย่าตัดสินว่า LOW/HIGH ในคู่มือเป็นหลักฐานว่า gate MOSFET มีวงจรกลับระดับเพิ่ม หากยังไม่ได้ trace เส้นทางนั้น

## 6. แนวทางเลือกขาให้เซนเซอร์ภายนอก

| อุปกรณ์ที่คู่มือยกตัวอย่าง | ความต้องการสัญญาณ | ข้อกำหนดสำหรับ generator |
|---|---|---|
| HC-SR04 | Trigger output + Echo input | ห้ามเลือก IN1–IN4 สำหรับ Trigger; ตรวจระดับ Echo ของโมดูลก่อนต่อ GPIO ตรง |
| HC-SR501 PIR | Digital input | ช่อง IN หรือ GPIO ขยายที่เหมาะสม |
| DHT11 | สายข้อมูลสลับรับ/ส่ง | ไม่อนุมัติ IN1–IN4 แบบ digital-buffer input-only เป็นสายข้อมูลโดยอัตโนมัติ; ใช้ GPIO ขยายที่ตรวจแล้ว |
| DS18B20 | One-wire แบบ bidirectional | เลือก GPIO ขยายที่เหมาะสม; คู่มือหน้า 3 ระบุ pull-up 4.7kΩ สำหรับตัวอย่าง |
| HW-511 | Digital input | ช่อง IN หรือ GPIO ขยาย |
| BME280 / LCD I2C | SDA/SCL | คู่มือหน้า 4/8 ยกตัวอย่าง SDA0/SCL0; แชร์กับจอ ต้องตรวจ address conflict |
| LED / RGB / buzzer / relay / motor | Output ตามชนิดโหลด | GPIO18/19/23 หรือวงจร OUT ที่เหมาะสม ไม่ขับโหลดกำลังตรงจาก GPIO |

ข้อความ “1 pin / 2 pin” ในคู่มือเป็นจำนวนขาสัญญาณ ไม่รวมไฟ/GND และไม่ยืนยันว่าเลือกขา input-only มาใช้แทนขา bidirectional ได้

## 7. กฎ metadata และ AI

```yaml
board_profile: kidbright32-v1.3
input_connector_mode: digital_buffer
input_analog_supported: false
dedicated_servo_connector: false
ntp_led_gpio: 15
matrix_bus: {sda: 21, scl: 22}
sensor_chain_bus: {sda: 4, scl: 5}
input_gpios: [32, 33, 34, 35]
output_control_gpios: [26, 27]
expansion_gpios: [18, 19, 23]
hardware_tested: false
```

เป็นตัวอย่างข้อมูลสำหรับนำไปปรับกับ schema ของโปรเจกต์ ไม่ใช่ config ที่ติดตั้งแล้ว ห้ามเปิด Servo1=15 โดยสืบทอด V1.5; หากจะใช้ GPIO15 กับ Servo ภายนอกต้องระบุการใช้ร่วมกับ NTP LED และ boot strapping เป็น recipe แยก

## 8. ช่องว่างที่ยังต้องยืนยัน

I2C address, driver/register, ความถี่บัส, pixel orientation, ADC calibration ของ LDR, กระแสโหลด, pull-up ที่ต้องใช้ และ polarity ของ API จริง ยังไม่ใช่ผลทดสอบในรอบนี้ อย่าใช้ข้อความหน้า 11 ที่ว่าแรงดัน “ไม่เกิน 5.5V” เป็นพิกัด GPIO ESP32 ตรง ต้องตรวจ datasheet ของ buffer และเส้นทาง connector ก่อน

อ่านคู่กับ `KidBright32-V1.6-Hardware-Reference.md` และ `KidBright32-Version-Comparison.md` เพื่อเลือก capability ตามรุ่น
