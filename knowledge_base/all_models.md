# 📋 KidBright — ประวัติรุ่น, GPIO Pinout & Sensor ทุกรุ่น (ตั้งแต่ V2016 ถึงรุ่นล่าสุด)
> **จัดทำโดย:** รวบรวมจากเอกสารทางการ NECTEC/NSTDA · อัปเดต 2026
> ครอบคลุมทุกรุ่น: **KidBright V2016 (ESP8266)** · **V1.0 (ESP8266)** · **V1.1–V1.6 (ESP32)** · **KidBright μAI (AllWinner V831/ESP32-S3)**

---

## 🏛️ ประวัติรุ่น KidBright ทุกรุ่น (Timeline)

| รุ่น | ปี | MCU | USB | หมายเหตุสำคัญ |
|------|-----|-----|-----|---------------|
| **KidBright V2016** | 2016 | ESP8266 | Micro-USB | Prototype ทดสอบในคลองหลวง — ใช้ Android App บน WiFi ไม่มี USB-Serial programming |
| **KidBright V1.0** | 2017 | ESP8266 | Micro-USB | รุ่นแรกที่แจกจ่ายจริง รูปร่างบอร์ดต่างจาก V1.1+ ควบคุมผ่าน KidBright IDE/Android App |
| **V1.1** | 2018 | ESP32-WROOM-32 | Micro-USB (Cypress CY7C65213) | รุ่นแรกที่ใช้ ESP32, มี LED status 4 ดวง |
| **V1.2** | 2018 | ESP32-WROOM-32 | Micro-USB (Cypress CY7C65213) | เหมือน V1.1 แก้ไข PCB เล็กน้อย |
| **V1.3** | 2019 | ESP32-WROOM-32 | Micro-USB (FTDI FT232RL) | เปลี่ยน USB bridge เป็น FTDI |
| **V1.4** | 2019–2020 | ESP32-WROOM-32 | Micro-USB (FTDI) | LED status ลดเหลือ 2 ดวง (WiFi+BT) |
| **V1.5 Rev 3.1** | 2020 | ESP32-WROOM-32 | Micro-USB (CP2102) | NECTEC Standard, SW2=GPIO14 |
| **V1.5 Rev 3.1G** | 2020 | ESP32-WROOM-32 | Micro-USB (CP2102) | Gravitech OEM, SW2=GPIO14 |
| **V1.5 iA** | 2021–2022 | ESP32-WROOM-32 | **USB-C** (CP2102) | INEX, เพิ่ม KXTJ3-1057 Accelerometer, SW2=GPIO14 |
| **KidBright32i** | 2021–2022 | ESP32-WROOM-32 | **USB-C** (CP2102) | INEX บอร์ดสีเขียว, ต้นแบบ i-series — Phototransistor แทน LDR, ADC บน IN1–IN4, **SW2=GPIO14** |
| **KidBright32iA** | 2022 | ESP32-WROOM-32 | **USB-C** (CP2102) | INEX — เพิ่ม KXTJ3-1057 Accelerometer บน I2C0, **SW2=GPIO14**, ฐานเดียวกับ 32i |
| **V1.6** | 2022+ | ESP32-WROOM-32 | **USB-C** (CP2102) | Gravitech, เพิ่ม MPU-6050 + RGB LED ×6, SW2=GPIO17 |
| **KidBright32iP** | 2023–2024 | ESP32-WROOM-32 | **USB-C** (CP2102) | INEX บอร์ดสีชมพู — ปรับปรุงจาก 32i, Phototransistor ดีขึ้น, LED สถานะไฟเลี้ยง, รองรับ Servo, **SW2=GPIO14** |
| **KidBright μAI** | 2024 | AllWinner V831 + ESP32-S3 | USB-C (OTG+UART) | รุ่นล่าสุด — Edge AI, มีกล้อง 2MP, ไมโครโฟน, จอ IPS 1.3 นิ้ว, Tina Linux |

---

## 📌 GPIO Pinout สรุปทุกรุ่น (ESP32 Series)

### Generation 1 — ESP8266 (V2016 / V1.0) ⛔ ไม่รองรับ ESP-IDF

> ใช้ได้เฉพาะ **KidBright IDE** หรือ **Arduino IDE** เท่านั้น ไม่มี native GPIO header เหมือน ESP32

| ฟังก์ชัน | หมายเหตุ |
|----------|----------|
| MCU | ESP8266 (ESP-12F module) |
| WiFi | 802.11 b/g/n 2.4 GHz (built-in) |
| ADC | 1× 10-bit (A0) เท่านั้น |
| I2C | SW I2C (GPIO4=SDA, GPIO5=SCL) |
| LED Matrix | 16×8 Red LED (HT16K33 via I2C) |
| Sensor | LDR (A0), LM73 Temperature (I2C), RTC (I2C) |
| Buzzer | Passive Piezo (GPIO) |
| Button | SW1, SW2 |
| Input Port | IN1–IN4 (Digital เท่านั้น) |
| Output Port | OUT1–OUT2 (Digital) |
| USB | Micro-USB (สำหรับ power + programming) |
| Control | Android App ผ่าน WiFi / KidBright IDE |

---

### Generation 2 — ESP32 V1.1 / V1.2 (Cypress USB, LED 4 ดวง)

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO2 | LED WiFi (Active HIGH) | ⚠️ Boot strapping pin |
| GPIO4 | I2C_NUM_1 SDA (LM73/RTC) | — |
| GPIO5 | I2C_NUM_1 SCL + LED NTP | ⚠️ แชร์กับ VSPI CLK |
| GPIO12 | LED IoT (Active HIGH) | ⚠️ Boot strapping — ห้าม pull-up ไปยัง 3.3V |
| GPIO13 | Passive Buzzer (LEDC/PWM) | — |
| GPIO14 | SW2 Button (Active LOW) | — |
| GPIO16 | SW1 Button (Active LOW) | — |
| GPIO21 | I2C_NUM_0 SDA (HT16K33 Matrix) | — |
| GPIO22 | I2C_NUM_0 SCL (HT16K33 Matrix) | — |
| GPIO23 | LED BT (Active HIGH) | ⚠️ บางล็อตแชร์กับ I2C_NUM_0 SCL |
| GPIO25 | USB Host Type-A Control (Active LOW) | — |
| GPIO26 | OUT1 (Active LOW) | — |
| GPIO27 | OUT2 (Active LOW) | — |
| GPIO32 | IN1 (Digital Input) | — |
| GPIO33 | IN2 (Digital Input) | — |
| GPIO34 | IN3 (Digital Input-only) | ไม่มี internal pull |
| GPIO35 | IN4 (Digital Input-only) | ไม่มี internal pull |
| GPIO36 | LDR Light Sensor (ADC1_CH0) | Input-only |

**เซนเซอร์ on-board:** LDR (GPIO36), LM73 Temp (I2C 0x4D), RTC MCP794xx (I2C 0x6F), HT16K33 LED Matrix (I2C 0x70)

---

### Generation 2 — ESP32 V1.3 (FTDI USB)

> GPIO เหมือน V1.1/V1.2 ทุกอย่าง เปลี่ยนเพียง USB bridge chip เป็น FTDI FT232RL

---

### Generation 2 — ESP32 V1.4 (LED Status ลดเหลือ 2 ดวง)

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO2 | LED WiFi (Active HIGH) | ⚠️ Boot strapping |
| GPIO4 | LED BT (Active HIGH) + I2C_NUM_1 SDA | ⚠️ แชร์ — เลือกได้แค่อย่างเดียว |
| GPIO5 | I2C_NUM_1 SCL | ว่างจาก NTP LED แล้ว |
| GPIO12 | GPIO ทั่วไป | ว่างจาก IoT LED แล้ว (แต่ยังเป็น boot pin) |
| GPIO13 | Passive Buzzer (LEDC/PWM) | — |
| GPIO14 | SW2 Button (Active LOW) | — |
| GPIO16 | SW1 Button (Active LOW) | — |
| GPIO21 | I2C_NUM_0 SDA (HT16K33) | — |
| GPIO22 | I2C_NUM_0 SCL (HT16K33) | — |
| GPIO25 | USB Host Control (Active LOW) | — |
| GPIO26 | OUT1 (Active LOW) | — |
| GPIO27 | OUT2 (Active LOW) | — |
| GPIO32 | IN1 (Digital Input) | — |
| GPIO33 | IN2 (Digital Input) | — |
| GPIO34 | IN3 (Digital Input-only) | — |
| GPIO35 | IN4 (Digital Input-only) | — |
| GPIO36 | LDR ADC (ADC1_CH0) | Input-only |

**เซนเซอร์ on-board:** LDR, LM73 Temp, RTC, HT16K33 Matrix — **ไม่มี ADC บน IN1–IN4**

---

### Generation 2 — ESP32 V1.5 Rev 3.1 (NECTEC Standard)

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO2 | LED WiFi (Active HIGH) | — |
| GPIO4 | LED BT (Active HIGH) + I2C_NUM_1 SDA | ⚠️ แชร์ |
| GPIO5 | I2C_NUM_1 SCL (LM73/RTC) | — |
| GPIO13 | Passive Buzzer (LEDC/PWM) | — |
| **GPIO14** | **SW2 Button (Active LOW)** | ⚠️ ต่างจาก Rev 3.1G/iA/V1.6 |
| GPIO16 | SW1 Button (Active LOW) | — |
| GPIO21 | I2C_NUM_0 SDA (HT16K33) | — |
| GPIO22 | I2C_NUM_0 SCL (HT16K33) | — |
| GPIO25 | USB Host Control (Active LOW) | — |
| GPIO26 | OUT1 (Active LOW) | — |
| GPIO27 | OUT2 (Active LOW) | — |
| GPIO32 | IN1 (Digital เท่านั้น) | ไม่รองรับ ADC |
| GPIO33 | IN2 (Digital เท่านั้น) | ไม่รองรับ ADC |
| GPIO34 | IN3 (Digital Input-only) | ไม่รองรับ ADC |
| GPIO35 | IN4 (Digital Input-only) | ไม่รองรับ ADC |
| GPIO36 | LDR (ADC1_CH0) | — |

**เซนเซอร์ on-board:** LDR, LM73 (I2C 0x4D), RTC MCP794xx (I2C 0x6F), HT16K33 (I2C 0x70) — **ไม่มี Accelerometer**

---

### Generation 2 — ESP32 V1.5 Rev 3.1G (Gravitech OEM)

> GPIO เหมือน V1.5 Rev 3.1 ทุกอย่าง **SW2 = GPIO14** เหมือนกัน
> ฮาร์ดแวร์และเซนเซอร์เหมือนกันทุกประการ

---

### Generation 2 — ESP32 V1.5 iA (INEX) — เพิ่ม Accelerometer

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO2 | LED WiFi (Active HIGH) | — |
| GPIO4 | LED BT (Active HIGH) + I2C_NUM_1 SDA | ⚠️ แชร์ |
| GPIO5 | I2C_NUM_1 SCL | — |
| GPIO13 | Passive Buzzer (LEDC/PWM) | — |
| GPIO16 | SW1 Button (Active LOW) | — |
| **GPIO14** | **SW2 Button (Active LOW)** | ✅ เหมือน Rev 3.1/3.1G |
| GPIO18 | I/O Port ขา 18 (Active HIGH) | — |
| GPIO19 | I/O Port ขา 19 (Active HIGH) | — |
| GPIO21 | I2C_NUM_0 SDA (HT16K33 + KXTJ3) | — |
| GPIO22 | I2C_NUM_0 SCL (HT16K33 + KXTJ3) | — |
| GPIO23 | I/O Port ขา 23 (Active HIGH) | — |
| GPIO25 | USB Host Control (Active LOW) | — |
| GPIO26 | OUT1 (Active LOW) | — |
| GPIO27 | OUT2 (Active LOW) | — |
| GPIO32 | IN1 (Digital + **ADC** รองรับ) | ✅ รองรับ ADC |
| GPIO33 | IN2 (Digital + **ADC** รองรับ) | ✅ รองรับ ADC |
| GPIO34 | IN3 (Digital Input-only + ADC) | ✅ รองรับ ADC, ไม่มี pull |
| GPIO35 | IN4 (Digital Input-only + ADC) | ✅ รองรับ ADC, ไม่มี pull |
| GPIO36 | LDR (ADC1_CH0) | — |

**เซนเซอร์ on-board:**
| เซนเซอร์ | Protocol | I2C Address | หมายเหตุ |
|----------|----------|-------------|----------|
| LDR (แสง) | ADC | GPIO36 | — |
| LM73 (อุณหภูมิ) | I2C_NUM_1 | 0x4D | SDA=GPIO4, SCL=GPIO5 |
| RTC MCP794xx | I2C_NUM_1 | 0x6F | + CR1220 battery |
| HT16K33 (LED Matrix 16×8) | I2C_NUM_0 | 0x70 | SDA=GPIO21, SCL=GPIO22 |
| **KXTJ3-1057 (Accelerometer 3-axis)** | I2C_NUM_0 | **0x0E** | เพิ่มใหม่ใน iA |
| Passive Buzzer | PWM/LEDC | GPIO13 | — |

---

### Generation 2 — ESP32 KidBright32i (INEX บอร์ดสีเขียว) — i-series ต้นฉบับ

> **ผลิตโดย INEX** ใช้เป็น base ของ i-series ทั้งหมด GPIO layout เหมือน V1.5 iA แต่ **ไม่มี KXTJ3 Accelerometer**

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO2 | LED WiFi (Active HIGH) | — |
| GPIO4 | LED BT (Active HIGH) + I2C_NUM_1 SDA | ⚠️ แชร์ |
| GPIO5 | I2C_NUM_1 SCL | — |
| GPIO13 | Passive Buzzer (LEDC/PWM) | — |
| GPIO16 | SW1 Button (Active LOW) | — |
| **GPIO14** | **SW2 Button (Active LOW)** | ✅ เหมือน Rev 3.1/3.1G/iA |
| GPIO18 | I/O Port ขา 18 | จุดบัดกรีอิสระ |
| GPIO19 | I/O Port ขา 19 | จุดบัดกรีอิสระ |
| GPIO21 | I2C_NUM_0 SDA (HT16K33) | — |
| GPIO22 | I2C_NUM_0 SCL (HT16K33) | — |
| GPIO23 | I/O Port ขา 23 | จุดบัดกรีอิสระ |
| GPIO25 | USB Host Control (Active LOW) | — |
| GPIO26 | OUT1 (Active LOW) | — |
| GPIO27 | OUT2 (Active LOW) | — |
| GPIO32 | IN1 (Digital + ADC) | ✅ รองรับ ADC |
| GPIO33 | IN2 (Digital + ADC) | ✅ รองรับ ADC |
| GPIO34 | IN3 (Digital Input-only + ADC) | ✅ ไม่มี pull |
| GPIO35 | IN4 (Digital Input-only + ADC) | ✅ ไม่มี pull |
| GPIO36 | **Phototransistor** (ADC1_CH0) | ⚡ เปลี่ยนจาก LDR เป็น Phototransistor |
| VN (GPIO39) | จุดบัดกรีอิสระ (ADC Input-only) | — |

**เซนเซอร์ on-board:**
| เซนเซอร์ | Protocol | I2C Address | หมายเหตุ |
|----------|----------|-------------|----------|
| **Phototransistor (แสง)** | ADC | GPIO36 | ⚡ Phototransistor ไม่ใช่ LDR |
| LM73 (อุณหภูมิ -40~150°C) | I2C_NUM_1 | 0x4D | — |
| RTC | I2C_NUM_1 | 0x6F | EEPROM ใหญ่ขึ้น |
| HT16K33 (LED Matrix 16×8) | I2C_NUM_0 | 0x70 | — |
| Passive Buzzer | PWM/LEDC | GPIO13 | — |

**จุดต่างจาก V1.5 Rev 3.1 หลักๆ:** USB-C, Phototransistor แทน LDR, ADC บน IN1–IN4, 3.3V Regulator จาก USB, GPIO18/19/23/VN breakout, LED USB status (สีฟ้า), **SW2=GPIO14** (เหมือน Rev 3.1/3.1G/iA)

---

### Generation 2 — ESP32 KidBright32iA (INEX) — เพิ่ม Accelerometer บน 32i

> ฐานเดียวกับ KidBright32i (บอร์ดสีเขียว) แต่เพิ่ม **KXTJ3-1057** บน I2C_NUM_0
> GPIO layout เหมือน KidBright32i ทุกอย่าง ยกเว้น I2C_NUM_0 มี KXTJ3 ด้วย

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO21 | I2C_NUM_0 SDA (HT16K33 + **KXTJ3**) | เพิ่ม KXTJ3 |
| GPIO22 | I2C_NUM_0 SCL (HT16K33 + **KXTJ3**) | เพิ่ม KXTJ3 |
| **GPIO14** | **SW2 Button (Active LOW)** | ✅ เหมือน Rev 3.1/3.1G/iA/32i |
| GPIO36 | **Phototransistor** (ADC1_CH0) | ⚡ Phototransistor |
| (ที่เหลือ) | เหมือน KidBright32i ทุกอย่าง | — |

**เซนเซอร์ on-board:**
| เซนเซอร์ | Protocol | I2C Address | หมายเหตุ |
|----------|----------|-------------|----------|
| Phototransistor (แสง) | ADC | GPIO36 | Phototransistor แทน LDR |
| LM73 (อุณหภูมิ) | I2C_NUM_1 | 0x4D | — |
| RTC | I2C_NUM_1 | 0x6F | EEPROM ใหญ่ขึ้น |
| HT16K33 (LED Matrix 16×8) | I2C_NUM_0 | 0x70 | — |
| **KXTJ3-1057 (Accelerometer 3-axis)** | I2C_NUM_0 | **0x0E** | เพิ่มจาก 32i |
| Passive Buzzer | PWM/LEDC | GPIO13 | — |

---

### Generation 2 — ESP32 KidBright32iP (INEX บอร์ดสีชมพู) — ปรับปรุงจาก 32i

> **ผลิตโดย INEX** เข้ากันได้กับ V1.5 มาตรฐาน สวทช. รับคุณสมบัติของ 32i มาทั้งหมด + ปรับปรุงเพิ่มเติม

| GPIO | ฟังก์ชัน | หมายเหตุ |
|------|----------|----------|
| GPIO2 | LED WiFi (Active HIGH) | — |
| GPIO4 | LED BT (Active HIGH) + I2C_NUM_1 SDA | ⚠️ แชร์ |
| GPIO5 | I2C_NUM_1 SCL | — |
| GPIO13 | Passive Buzzer (LEDC/PWM) | — |
| **GPIO15** | **SERVO1** (PWM/LEDC) | — |
| GPIO16 | SW1 Button (Active LOW) | — |
| **GPIO17** | **SW2 Button (Active LOW) / SERVO2** | ⚠️ แชร์กัน |
| GPIO18 | I/O Port ขา 18 | จุดบัดกรีอิสระ |
| GPIO19 | I/O Port ขา 19 | จุดบัดกรีอิสระ |
| GPIO21 | I2C_NUM_0 SDA (HT16K33) | — |
| GPIO22 | I2C_NUM_0 SCL (HT16K33) | — |
| GPIO23 | I/O Port ขา 23 | จุดบัดกรีอิสระ |
| GPIO25 | USB Host Control (Active LOW) | — |
| GPIO26 | OUT1 (Active LOW) | — |
| GPIO27 | OUT2 (Active LOW) | — |
| GPIO32 | IN1 (Digital + ADC) | ✅ |
| GPIO33 | IN2 (Digital + ADC) | ✅ |
| GPIO34 | IN3 (Digital Input-only + ADC) | ✅ |
| GPIO35 | IN4 (Digital Input-only + ADC) | ✅ |
| GPIO36 | **Phototransistor** ปรับปรุง (ADC1_CH0) | ⚡ ตอบสนองแสงขาวดีกว่า 32i |
| VN (GPIO39) | จุดบัดกรีอิสระ (ADC Input-only) | — |

**เซนเซอร์ on-board:**
| เซนเซอร์ | Protocol | I2C Address | หมายเหตุ |
|----------|----------|-------------|----------|
| **Phototransistor ปรับปรุง (แสง)** | ADC | GPIO36 | ดีกว่า 32i — เชิงเส้นมากขึ้น |
| LM73 (อุณหภูมิ -40~150°C) | I2C_NUM_1 | 0x4D | — |
| RTC | I2C_NUM_1 | 0x6F | EEPROM ใหญ่ขึ้น |
| HT16K33 (LED Matrix 16×8) | I2C_NUM_0 | 0x70 | — |
| Passive Buzzer | PWM/LEDC | GPIO13 | — |
| SERVO1 | PWM/LEDC | GPIO15 | — |
| SERVO2 | PWM/LEDC | GPIO17 | แชร์กับ SW2 |

**จุดเพิ่มจาก 32i:** LED สถานะไฟเลี้ยงบอร์ด, LED สถานะไฟเลี้ยง Servo, Phototransistor ปรับปรุง, SERVO connector (GPIO15/17)

---

### Generation 2 — ESP32 V1.6 (Gravitech) — เพิ่ม MPU-6050 + RGB LED

| GPIO | ฟังก์ชั�| LED ไฟเลี้ยงบอร์ด | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | — |
| GPIO18/19/23 breakout | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ✅ | — |
| กล้อง (Camera) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 2MP |
| ไมโครโฟน | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| จอสี IPS | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 1.3" |
| Edge AI | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

---

## ⚡ เซนเซอร์ภายนอก (External) ที่รองรับผ่าน JST Port ทุกรุ่น ESP32

เชื่อมต่อผ่านพอร์ต IN1–IN4 (JST 3-pin), I2C KB Chain, หรือ OUT1–OUT2:

| เซนเซอร์ | ประเภท | พอร์ตที่ใช้ |
|----------|---------|------------|
| PIR Motion Sensor | Digital | IN1–IN4 |
| Reed Switch (Magnetic) | Digital | IN1–IN4 |
| Soil Moisture | Digital / Analog (iA/V1.6/32i/32iA/32iP) | IN1–IN4 |
| DHT11/DHT22 (Temp+Humidity) | Digital 1-wire | IN1–IN4 |
| Ultrasonic HC-SR04 | Digital | IN1–IN4 |
| IR Sensor | Digital | IN1–IN4 |
| เซนเซอร์ I2C อื่นๆ | I2C | KB Chain port |
| พัดลม / หลอดไฟ (5V) | Digital | OUT1–OUT2 / USB Host |

---

## 🔑 สรุปความแตกต่างหลัก INEX i-series vs V1.5 มาตรฐาน

| คุณสมบัติ | V1.5 Rev 3.1 (NECTEC) | V1.5 iA (INEX) | **32i** (INEX สีเขียว) | **32iA** (INEX) | **32iP** (INEX สีชมพู) |
|-----------|----------------------|----------------|----------------------|----------------|----------------------|
| SW2 GPIO | GPIO14 | **GPIO14** | **GPIO14** | **GPIO14** | GPIO17 (SERVO2 shared) |
| USB | Micro-USB | USB-C | USB-C | USB-C | USB-C |
| เซนเซอร์แสง | LDR | LDR | **Phototransistor** | **Phototransistor** | **Phototransistor ปรับปรุง** |
| ADC บน IN1-IN4 | ❌ | ✅ | ✅ | ✅ | ✅ |
| Accelerometer | ❌ | KXTJ3 0x0E (I2C0) | ❌ | KXTJ3 0x0E (I2C0) | ❌ |
| 3.3V Regulator USB | ❌ | ❌ | ✅ | ✅ | ✅ |
| GPIO18/19/23 breakout | ❌ | ✅ | ✅ | ✅ | ✅ |
| LED USB status | ❌ | ❌ | ✅ | ✅ | ✅ |
| LED ไฟเลี้ยงบอร์ด | ❌ | ❌ | ❌ | ❌ | ✅ |
| SERVO connector | ❌ | ❌ | ❌ | ❌ | ✅ (GPIO15/17) |
| RTC EEPROM | มาตรฐาน | มาตรฐาน | **ใหญ่ขึ้น** | **ใหญ่ขึ้น** | **ใหญ่ขึ้น** |

---

## 🛠️ มาตรฐานและข้อกำหนดทางเทคนิคสำหรับ ESP-IDF Firmware

### 1. การอ่านเซนเซอร์อุณหภูมิ On-board (Texas Instruments LM73)
- **ตำแหน่ง:** `I2C_NUM_1` (SDA=GPIO4, SCL=GPIO5), I2C Address `0x4D`
- **Register:** `0x00` (Temperature Register, 16-bit Two's complement)
- **สูตรคำนวณ:**
  ```c
  int16_t raw_temp = (int16_t)(((uint16_t)raw[0] << 8) | (uint16_t)raw[1]);
  float temperature = (float)raw_temp / 128.0f; // 1 LSB = 1/128°C
  ```
- **ข้อควรระวัง:** ห้ามสับสนกับ MCP9808 (Reg 0x05, Addr 0x18, /16.0f) หรือ ADT7410

### 2. HT16K33 LED Dot Matrix 16x8
- **ตำแหน่ง:** `I2C_NUM_0` (SDA=GPIO21, SCL=GPIO22), Address `0x70`
- **ลำดับคำสั่ง Init:** ส่งทีละ Transaction: `0x21` (Osc ON) -> `delay 10ms` -> `0x81` (Display ON) -> `0xEF` (Brightness Max)
- **การแมป RAM:** Interleaved mapping — Columns 0–7 (ซีกซ้าย) อยู่ Address คู่ (`buf[1 + c*2]`), Columns 8–15 (ซีกขวา) อยู่ Address คี่ (`buf[2 + c*2]`)
- **การแสดงผลจริง:** ห้ามใช้ placeholder function ต้องแปลง Font Bitmap และส่ง payload 17 ไบต์ไปยัง Register `0x00`

### 3. Driver Safety & Return Code Checks
- ต้องใช้ `ESP_ERROR_CHECK(...)` สำหรับทุกขั้นตอนการ Init (`i2c_param_config`, `i2c_driver_install`, `adc_oneshot_new_unit`, `gpio_config`)
- ตรวจสอบ `if (ret != ESP_OK)` สำหรับ I2C Runtime read/write เพื่อความเสถียร
- บอร์ด KidBright เป็น **ESP32-WROOM-32** (ไม่มี Flash/PSRAM บน GPIO16/17 จึงใช้ GPIO16 เป็น SW1 ได้ปลอดภัย แต่หากรันบน ESP32-WROVER ต้องระวังห้ามใช้ GPIO16/17)
024) — Edge AI Platform

> **⚠️ ไม่ใช่ ESP32 ธรรมดา** — ใช้ SoC AllWinner V831 (ARM Cortex-A7) สำหรับ AI + ESP32-S3 สำหรับ IoT/WiFi
> ทำงานบน **Tina Linux** (fork จาก OpenWrt, Kernel 4.9) — ไม่ใช่ ESP-IDF Framework
> พัฒนาด้วย **KidBright μAI IDE** (online Blockly + Python) หรือ cross-compile C/C++ บน Ubuntu 16.04

| คุณสมบัติ | รายละเอียด |
|-----------|-----------|
| AI Processor | AllWinner V831 (ARM Cortex-A7 @ ~800 MHz) |
| IoT Module | ESP32-S3 (WiFi + BLE) |
| Display | จอ IPS สี 1.3 นิ้ว (TFT) |
| Camera | กล้อง 2 ล้านพิกเซล (built-in) |
| Microphone | ไมโครโฟน built-in |
| WiFi | 802.11 b/g/n 2.4 GHz (via ESP32-S3) |
| USB | USB-C (OTG + UART) |
| Input/Output | รองรับ Digital I/O + ต่ออุปกรณ์ภายนอก |
| Storage | SD Card (Tina Linux boot) |
| OS | Tina Linux (OpenWrt-based) |
| IDE | KidBright μAI IDE (online Blockly/Python) |
| AI Features | Image Classification, Object Detection, Sound Classification |
| Released | 2024 (เปิดตัว KDC24 KidBright Developer Conference) |

**เซนเซอร์ / อินพุตที่รองรับ:**
- กล้อง 2MP (ภาพ AI)
- ไมโครโฟน (เสียง AI)
- จอ IPS 1.3 นิ้ว (แสดงผล)
- WiFi (IoT, Cloud)
- ต่อเซนเซอร์ภายนอกผ่าน I/O ports

---

## 🔍 เปรียบเทียบเซนเซอร์ on-board ทุกรุ่น

| เซนเซอร์ | V1.1–V1.3 | V1.4 | V1.5 Rev3.1 | V1.5 Rev3.1G | V1.5 iA | **32i** | **32iA** | V1.6 | **32iP** | μAI |
|----------|-----------|------|-------------|--------------|---------|---------|----------|------|----------|-----|
| LDR (แสง) | ✅ GPIO36 | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ | — |
| Phototransistor | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ GPIO36 | ✅ GPIO36 | ❌ | ✅ GPIO36 | — |
| LM73 (อุณหภูมิ) | ✅ I2C 0x4D | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| RTC MCP794xx | ✅ I2C 0x6F | ✅ | ✅ | ✅ | ✅ | ✅ (EEPROM ใหญ่) | ✅ | ❓ | ✅ (EEPROM ใหญ่) | — |
| HT16K33 LED Matrix | ✅ I2C 0x70 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| KXTJ3-1057 Accel 3-axis | ❌ | ❌ | ❌ | ❌ | ✅ I2C 0x0E | ❌ | ✅ I2C 0x0E | ❌ | ❌ | — |
| MPU-6050 Accel+Gyro 6-axis | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ I2C 0x68 | ❌ | — |
| RGB LED WS2812B | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ ×6 (RMT) | ❌ | — |
| ADC บน IN1–IN4 | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| SERVO connector | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ ×2 | ✅ ×2 | — |
| 3.3V Regulator จาก USB | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅ | — |
| LED USB status (สีฟ้า) | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅ | — |
| LED ไฟเลี้ยงบอร์ด | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | — |
| GPIO18/19/23 breakout | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ✅ | — |
| กล้อง (Camera) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 2MP |
| ไมโครโฟน | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| จอสี IPS | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 1.3" |
| Edge AI | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |

---

## ⚡ เซนเซอร์ภายนอก (External) ที่รองรับผ่าน JST Port ทุกรุ่น ESP32

เชื่อมต่อผ่านพอร์ต IN1–IN4 (JST 3-pin), I2C KB Chain, หรือ OUT1–OUT2:

| เซนเซอร์ | ประเภท | พอร์ตที่ใช้ |
|----------|---------|------------|
| PIR Motion Sensor | Digital | IN1–IN4 |
| Reed Switch (Magnetic) | Digital | IN1–IN4 |
| Soil Moisture | Digital / Analog (iA/V1.6/32i/32iA/32iP) | IN1–IN4 |
| DHT11/DHT22 (Temp+Humidity) | Digital 1-wire | IN1–IN4 |
| Ultrasonic HC-SR04 | Digital | IN1–IN4 |
| IR Sensor | Digital | IN1–IN4 |
| เซนเซอร์ I2C อื่นๆ | I2C | KB Chain port |
| พัดลม / หลอดไฟ (5V) | Digital | OUT1–OUT2 / USB Host |

---

## 🔑 สรุปความแตกต่างหลัก INEX i-series vs V1.5 มาตรฐาน

| คุณสมบัติ | V1.5 Rev 3.1 (NECTEC) | V1.5 iA (INEX) | **32i** (INEX สีเขียว) | **32iA** (INEX) | **32iP** (INEX สีชมพู) |
|-----------|----------------------|----------------|----------------------|----------------|----------------------|
| SW2 GPIO | GPIO14 | **GPIO14** | **GPIO14** | **GPIO14** | GPIO17 (SERVO2 shared) |
| USB | Micro-USB | USB-C | USB-C | USB-C | USB-C |
| เซนเซอร์แสง | LDR | LDR | **Phototransistor** | **Phototransistor** | **Phototransistor ปรับปรุง** |
| ADC บน IN1-IN4 | ❌ | ✅ | ✅ | ✅ | ✅ |
| Accelerometer | ❌ | KXTJ3 0x0E (I2C0) | ❌ | KXTJ3 0x0E (I2C0) | ❌ |
| 3.3V Regulator USB | ❌ | ❌ | ✅ | ✅ | ✅ |
| GPIO18/19/23 breakout | ❌ | ✅ | ✅ | ✅ | ✅ |
| LED USB status | ❌ | ❌ | ✅ | ✅ | ✅ |
| LED ไฟเลี้ยงบอร์ด | ❌ | ❌ | ❌ | ❌ | ✅ |
| SERVO connector | ❌ | ❌ | ❌ | ❌ | ✅ (GPIO15/17) |
| RTC EEPROM | มาตรฐาน | มาตรฐาน | **ใหญ่ขึ้น** | **ใหญ่ขึ้น** | **ใหญ่ขึ้น** |