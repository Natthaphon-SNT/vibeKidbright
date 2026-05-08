# KidBright32 iA — Sensor Code Examples (ESP-IDF v5.x)

> ⚠️ **ไฟล์ทั้งหมดในโฟลเดอร์นี้เขียนด้วย ESP-IDF v5.x เท่านั้น**
> ห้ามใช้ API Legacy (`driver/adc.h`, `esp_adc_cal.h`) เด็ดขาด
>
> **Board Coverage:** V1.5 Rev 3.1 (NECTEC Standard) · V1.5 Rev 3.1G (Gravitech OEM) · V1.5 iA (INEX) · V1.6 (Gravitech)
>
> ⚠️ **SW2 = GPIO14 ทั้ง Rev 3.1 และ Rev 3.1G** — ยืนยันจาก hardware scan Apr 17 2026
> ตรวจสอบ PCB silkscreen ก่อนเสมอ

---

## ไฟล์ในโฟลเดอร์นี้

| ไฟล์ | เซ็นเซอร์ | อธิบาย |
|------|----------|--------|
| `adc_ldr_external.c` | LDR, IN1, IN2 | ADC oneshot + calibration สำหรับ LDR (GPIO36) และพอร์ต JST |
| `temp_lm73.c` | LM73 (I2C) | อ่านอุณหภูมิจาก LM73 บน I2C_NUM_1 |
| `accel_kxtj3.c` | KXTJ3-1057 (I2C) | อ่านความเร่งจาก KXTJ3 บน I2C_NUM_0 |
| `all_sensors_demo.c` | ทุกเซ็นเซอร์ | Demo รวมทุกเซ็นเซอร์ใน task เดียว |
| `all_models.md` | — | Reference ครบทุกรุ่น: sensor map, GPIO, ports, KB Chain |

---

## กฎที่ต้องรู้ก่อน (Vaccine)

### ❌ Framework ที่ห้ามใช้

```c
// ❌ BANNED: Arduino Framework — ห้ามใช้โดยเด็ดขาด
#include <Wire.h>          // ❌
#include <Arduino.h>       // ❌
void setup() { }           // ❌
void loop()  { }           // ❌
digitalWrite(...)          // ❌
analogRead(...)            // ❌
```

> **กฎเหล็ก:** โค้ดทุกไฟล์ต้องเป็น C/C++ บน ESP-IDF components เท่านั้น ไม่มีข้อยกเว้น

---

### ❌ ADC Legacy API (ถูกลบใน ESP-IDF v5)

```c
// ห้ามใช้โดยเด็ดขาด — ทุกอย่างด้านล่างถูกลบแล้ว
#include "driver/adc.h"           // ❌ BANNED
#include "esp_adc_cal.h"          // ❌ BANNED
adc1_config_width(...)            // ❌ ถูกลบ
adc1_config_channel_atten(...)    // ❌ ถูกลบ
adc1_get_raw(...)                 // ❌ ถูกลบ
esp_adc_cal_characterize(...)     // ❌ ถูกลบ
ADC_ATTEN_DB_11                   // ❌ Deprecated → ใช้ ADC_ATTEN_DB_12 แทน
```

### ✅ ADC Oneshot API ที่ถูกต้อง (ESP-IDF v5.x)

```c
#include "esp_adc/adc_oneshot.h"     // ✅
#include "esp_adc/adc_cali.h"        // ✅
#include "esp_adc/adc_cali_scheme.h" // ✅

// ขั้นตอน 4 ขั้น:
adc_oneshot_new_unit(...)            // 1. Create unit
adc_oneshot_config_channel(...)      // 2. Config channel (ใช้ ADC_ATTEN_DB_12)
adc_oneshot_read(...)                // 3. Read raw
adc_cali_raw_to_voltage(...)         // 4. Convert to mV (optional calibration)
```

---

### ❌ I2C Golden Rule — `i2c_driver_install()` เรียกได้แค่ครั้งเดียวต่อ port

```c
// ❌ ห้ามเรียก i2c_driver_install() ซ้ำบน port เดิม
// จะเกิด ESP_ERR_INVALID_STATE

// ✅ เรียกครั้งเดียว แล้วแชร์ bus ให้ทุก device บน port นั้น
i2c_driver_install(I2C_NUM_0, I2C_MODE_MASTER, 0, 0, 0);  // ครั้งเดียวเท่านั้น
// จากนั้น HT16K33 (0x70) และ KXTJ3 (0x0E) ใช้ I2C_NUM_0 ร่วมกันได้เลย
```

---

### ⚠️ Hardware Quirk — LED Matrix Y-axis Inversion

```c
// ❌ ผิด — จะแสดงผลกลับหัว
out_cols[col] |= (1 << row);

// ✅ ถูก — ต้อง invert Y-axis เสมอ (hardware wired upside-down)
out_cols[col] |= (1 << (7 - row));
```

---

## Sensor Map สรุป

### On-board Sensors — V1.5 Rev 3.1 (NECTEC Standard)

> ⚠️ **ไม่มี Accelerometer** · **ไม่รองรับ ADC บน IN1–IN4** · **SW2 = GPIO14**

| Sensor | Protocol | Bus/Pin | Address |
|--------|----------|---------|---------|
| LDR (Light) | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x6F |
| HT16K33 (Matrix) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 Button | GPIO | GPIO16 | — |
| **SW2 Button** | GPIO | **GPIO14** | — |
| USB Host Control | GPIO | GPIO25 (Active LOW) | — |

---

### On-board Sensors — V1.5 Rev 3.1G (Gravitech OEM)

> ⚠️ **ไม่มี Accelerometer** · **ไม่รองรับ ADC บน IN1–IN4** · **SW2 = GPIO14** (เหมือน Rev 3.1)

| Sensor | Protocol | Bus/Pin | Address |
|--------|----------|---------|---------|
| LDR (Light) | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x6F |
| HT16K33 (Matrix) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 Button | GPIO | GPIO16 | — |
| **SW2 Button** | GPIO | **GPIO14** | — |
| USB Host Control | GPIO | GPIO25 (Active LOW) | — |

> 📋 **I2C Scan Result (V1.5 Rev 3.1G — confirmed Apr 17 2026)**
> I2C_NUM_1: `0x4D` (LM73) + `0x6F` (RTC MCP794xx) · I2C_NUM_0: `0x70` (HT16K33)

---

### On-board Sensors — V1.5 iA (INEX)

> ✅ **เพิ่ม KXTJ3 Accelerometer** · **รองรับ ADC บน IN1–IN4** · SW2 = GPIO17

| Sensor | Protocol | Bus/Pin | Address |
|--------|----------|---------|---------|
| LDR (Light) | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x6F |
| **KXTJ3 (Accel)** | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | **0x0E** |
| HT16K33 (Matrix) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 Button | GPIO | GPIO16 | — |
| SW2 Button | GPIO | GPIO17 | — |

---

### External JST Ports (ทุกรุ่น V1.5+)

| Port | GPIO | Mode | หมายเหตุ |
|------|------|------|---------|
| IN1 | GPIO32 | Digital / ADC1_CH4 / Touch | ADC รองรับเฉพาะ iA และ V1.6 |
| IN2 | GPIO33 | Digital / ADC1_CH5 / Touch | ADC รองรับเฉพาะ iA และ V1.6 |
| IN3 | GPIO34 | Input-only / ADC1_CH6 | ไม่มี pull resistor |
| IN4 | GPIO35 | Input-only / ADC1_CH7 | ไม่มี pull resistor |
| OUT1 | GPIO26 | Digital / DAC2 | — |
| OUT2 | GPIO27 | Digital | — |

---

## ⚠️ GPIO Conflict Table

### V1.5 Rev 3.1 (NECTEC Standard)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO2 | **Wi-Fi LED** — อย่าใช้งานอื่น |
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO13 | **Passive Buzzer** — ต้องใช้ LEDC/PWM เสมอ |
| GPIO14 | **SW2 Button** — ห้ามใช้งานอื่น |
| GPIO16 | **SW1 Button** — ห้ามใช้งานอื่น |
| GPIO25 | **USB Host (Active LOW)** — อย่าใช้งานอื่น |
| GPIO36 | **LDR ADC** — Input-only, ไม่มี pull resistor |

### V1.5 Rev 3.1G (Gravitech OEM)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO2 | **Wi-Fi LED** — อย่าใช้งานอื่น |
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO13 | **Passive Buzzer** — ต้องใช้ LEDC/PWM เสมอ |
| GPIO14 | **SW2 Button** — ห้ามใช้งานอื่น (ยืนยัน Apr 17 2026) |
| GPIO16 | **SW1 Button** — ห้ามใช้งานอื่น |
| GPIO25 | **USB Host (Active LOW)** — อย่าใช้งานอื่น |
| GPIO36 | **LDR ADC** — Input-only, ไม่มี pull resistor |

### V1.5 iA (INEX)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO2 | **Wi-Fi LED** — อย่าใช้งานอื่น |
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO13 | **Passive Buzzer** — ต้องใช้ LEDC/PWM เสมอ |
| GPIO16 | **SW1 Button** — ห้ามใช้งานอื่น |
| GPIO17 | **SW2 Button** — ห้ามใช้งานอื่น |
| GPIO36 | **LDR ADC** — Input-only, ไม่มี pull resistor |

### V1.6 (Gravitech)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO2 | **Wi-Fi LED** — อย่าใช้งานอื่น |
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO13 | **Passive Buzzer** — ต้องใช้ LEDC/PWM เสมอ |
| GPIO16 | **SW1 Button** หรือ **SERVO1** — เลือกได้แค่อย่างเดียว |
| GPIO36 | **LDR ADC** — Input-only, ไม่มี pull resistor |

---

## ตัวอย่าง: อ่าน LM35 บน IN1 (GPIO32)

```c
// LM35: 10mV per degree Celsius
// Connect: VCC→3.3V, GND→GND, OUT→GPIO32 (IN1)
// รองรับเฉพาะ V1.5 iA และ V1.6 เท่านั้น (Rev 3.1 / 3.1G ไม่รองรับ ADC บน IN1)
int mv = adc_read_mv(ADC_CHANNEL_4, cali_in1);
float temp_c = mv / 10.0f;
ESP_LOGI("SENSOR", "LM35 Temperature: %.2f °C", temp_c);
```

## ตัวอย่าง: I2C init order ตามรุ่น

### V1.5 Rev 3.1 (NECTEC Standard) — SW2=GPIO14

```
I2C init order:
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) เท่านั้น (ไม่มี KXTJ3)
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D) + RTC (0x6F)
3. adc_init_all()  → ADC1: LDR (GPIO36) เท่านั้น (IN1–IN4 ไม่รองรับ ADC)

Button config:
- SW1 = GPIO16
- SW2 = GPIO14
```

### V1.5 Rev 3.1G (Gravitech OEM) — SW2=GPIO14

```
I2C init order:
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) เท่านั้น (ไม่มี KXTJ3)
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D) + RTC (0x6F)
3. adc_init_all()  → ADC1: LDR (GPIO36) เท่านั้น (IN1–IN4 ไม่รองรับ ADC)

Button config:
- SW1 = GPIO16
- SW2 = GPIO14  ← ยืนยัน hardware scan Apr 17 2026 (เหมือน Rev 3.1)
```

### V1.5 iA (INEX) — SW2=GPIO17

```
I2C init order (ต้องทำก่อนเสมอ):
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) + KXTJ3 (0x0E)
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D) + RTC (0x6F)
3. adc_init_all()  → ADC1: LDR (GPIO36) + IN1 (CH4) + IN2 (CH5) + IN3 (CH6) + IN4 (CH7)

Button config:
- SW1 = GPIO16
- SW2 = GPIO17
```

### V1.6 (Gravitech)

```
I2C init order:
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) + MPU-6050 (0x68) + RGB LED
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D) + RTC (0x6F)
3. adc_init_all()  → ADC1: LDR (GPIO36) + IN1–IN4

Button config:
- SW1 = GPIO16 (shared with SERVO1 — เลือกใช้อย่างใดอย่างหนึ่ง)
```

> **กฎทอง:** `i2c_driver_install()` เรียกได้แค่ครั้งเดียวต่อ port number
> หากเรียก 2 ครั้งจะเกิด error `ESP_ERR_INVALID_STATE`

---

## อ้างอิงเพิ่มเติม

- `kidbright32iA.md` — Full developer reference: MCU specs, HT16K33 register map, code examples, peripheral wiring
- `all_models.md` — ข้อมูลครบทุกรุ่น (Gen 1 ถึง V1.6) รวม sensor map, KB Chain, schematic links
