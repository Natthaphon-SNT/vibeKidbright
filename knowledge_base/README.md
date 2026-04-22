# KidBright32 iA — Sensor Code Examples (ESP-IDF v5.x)

> ⚠️ **ไฟล์ทั้งหมดในโฟลเดอร์นี้เขียนด้วย ESP-IDF v5.x เท่านั้น**
> ห้ามใช้ API Legacy (`driver/adc.h`, `esp_adc_cal.h`) เด็ดขาด
>
> **Board Coverage:** V1.5 Rev 3.1 (NECTEC Standard) · V1.5 Rev 3.1G (Gravitech OEM) · V1.5 iA (INEX) · V1.6 (Gravitech)
>
> ⚠️ **SW2 แตกต่างกันระหว่าง Rev 3.1 (GPIO14) และ Rev 3.1G (GPIO17)** — ตรวจสอบ PCB silkscreen ก่อนเสมอ

---

## ไฟล์ในโฟลเดอร์นี้

| ไฟล์ | เซ็นเซอร์ | อธิบาย |
|------|----------|--------|
| `adc_ldr_external.c` | LDR, IN1, IN2 | ADC oneshot + calibration สำหรับ LDR (GPIO36) และพอร์ต JST |
| `temp_lm73.c` | LM73 (I2C) | อ่านอุณหภูมิจาก LM73 บน I2C_NUM_1 |
| `accel_kxtj3.c` | KXTJ3-1057 (I2C) | อ่านความเร่งจาก KXTJ3 บน I2C_NUM_0 |
| `all_sensors_demo.c` | ทุกเซ็นเซอร์ | Demo รวมทุกเซ็นเซอร์ใน task เดียว |

---

## กฎที่ต้องรู้ก่อน (Vaccine)

### ❌ ADC Legacy API (ถูกลบใน ESP-IDF v5)
```c
// ห้ามใช้โดยเด็ดขาด — ทุกอย่างด้านล่างถูกลบแล้ว
#include "driver/adc.h"           // ❌ BANNED
#include "esp_adc_cal.h"          // ❌ BANNED
adc1_config_width(...)            // ❌ ถูกลบ
adc1_config_channel_atten(...)    // ❌ ถูกลบ
adc1_get_raw(...)                 // ❌ ถูกลบ
esp_adc_cal_characterize(...)     // ❌ ถูกลบ
ADC_ATTEN_DB_11                   // ❌ Deprecated → ใช้ DB_12 แทน
```

### ✅ ADC Oneshot API ที่ถูกต้อง (ESP-IDF v5.x)
```c
#include "esp_adc/adc_oneshot.h"     // ✅
#include "esp_adc/adc_cali.h"        // ✅
#include "esp_adc/adc_cali_scheme.h" // ✅

// ขั้นตอน 3 ขั้น:
adc_oneshot_new_unit(...)            // 1. Create unit
adc_oneshot_config_channel(...)      // 2. Config channel (ใช้ ADC_ATTEN_DB_12)
adc_oneshot_read(...)                // 3. Read
adc_cali_raw_to_voltage(...)         // 4. Convert to mV (optional calibration)
```

---

## Sensor Map สรุป

### On-board Sensors — V1.5 Rev 3.1 (NECTEC Standard)
> **ไม่มี Accelerometer** และ **ไม่รองรับ ADC บน IN1–IN4**
> ⚠️ **SW2 = GPIO14** — ต่างจาก Rev 3.1G ที่ใช้ GPIO17

| Sensor | Protocol | Bus/Pin | Address |
|--------|----------|---------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x6F |
| HT16K33 (Matrix) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 Button | GPIO | GPIO16 | — |
| **SW2 Button** | GPIO | **GPIO14** | — |
| USB Host | GPIO | GPIO25 (Active LOW) | — |

---

### On-board Sensors — V1.5 Rev 3.1**G** (Gravitech OEM)
> **ไม่มี Accelerometer** และ **ไม่รองรับ ADC บน IN1–IN4** (เหมือน Rev 3.1)
> ⚠️ **SW2 = GPIO17** — ต่างจาก Rev 3.1 ที่ใช้ GPIO14

| Sensor | Protocol | Bus/Pin | Address |
|--------|----------|---------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x6F |
| HT16K33 (Matrix) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 Button | GPIO | GPIO16 | — |
| **SW2 Button** | GPIO | **GPIO17** | — |
| USB Host | GPIO | GPIO25 (Active LOW) | — |

> 📋 **I2C Scan Result (V1.5 Rev 3.1G — confirmed Apr 17 2026)**
> I2C_NUM_1: `0x4D` (LM73) + `0x6F` (RTC MCP794xx) · I2C_NUM_0: `0x70` (HT16K33)

### On-board Sensors — V1.5 iA (INEX)
> **เพิ่ม KXTJ3 Accelerometer** และ **รองรับ ADC บน IN1–IN4**

| Sensor | Protocol | Bus/Pin | Address |
|--------|----------|---------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1, SDA=GPIO4, SCL=GPIO5 | 0x4D |
| KXTJ3 (Accel) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x0E |
| HT16K33 (Matrix) | I2C | I2C_NUM_0, SDA=GPIO21, SCL=GPIO22 | 0x70 |

### External JST Ports

| Port | GPIO | Mode |
|------|------|------|
| IN1 | GPIO32 | Digital / ADC1_CH4 / Touch |
| IN2 | GPIO33 | Digital / ADC1_CH5 / Touch |
| IN3 | GPIO34 | Input-only / ADC1_CH6 (no pull) |
| IN4 | GPIO35 | Input-only / ADC1_CH7 (no pull) |
| OUT1 | GPIO26 | Digital / DAC2 |
| OUT2 | GPIO27 | Digital |

---

## ⚠️ GPIO Conflict Table

### V1.5 Rev 3.1 (NECTEC Standard)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO13 | **Passive Buzzer** — ต้องใช้ LEDC/PWM เสมอ |
| GPIO14 | **SW2 Button** — ห้ามใช้งานอื่น บน V1.5 Rev 3.1 |
| GPIO16 | **SW1 Button** — ห้ามใช้งานอื่น บน V1.5 Rev 3.1 |
| GPIO25 | **USB Host (Active LOW)** — อย่าใช้งานอื่น |
| GPIO36 | LDR ADC — Input-only, ไม่มี pull resistor |
| GPIO2 | Wi-Fi LED — อย่าใช้งานอื่น |

### V1.5 Rev 3.1**G** (Gravitech OEM)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO13 | **Passive Buzzer** — ต้องใช้ LEDC/PWM เสมอ |
| GPIO16 | **SW1 Button** — ห้ามใช้งานอื่น บน V1.5 Rev 3.1G |
| GPIO14 | **SW2 Button** — ห้ามใช้งานอื่น บน V1.5 Rev 3.1G |
| GPIO25 | **USB Host (Active LOW)** — อย่าใช้งานอื่น |
| GPIO36 | LDR ADC — Input-only, ไม่มี pull resistor |
| GPIO2 | Wi-Fi LED — อย่าใช้งานอื่น |

### V1.5 iA / V1.6 (Shared)

| GPIO | ใช้ได้เป็น... |
|------|--------------|
| GPIO4 | **BT LED** หรือ **LM73 SDA** — เลือกได้แค่อย่างเดียว |
| GPIO16 | **SW1 Button** หรือ **SERVO1** — เลือกได้แค่อย่างเดียว |
| GPIO36 | LDR ADC — Input-only, ไม่มี pull resistor |
| GPIO2 | Wi-Fi LED — อย่าใช้งานอื่น |

---

## ตัวอย่าง: อ่าน LM35 บน IN1 (GPIO32)

```c
// LM35: 10mV per degree Celsius
// Connect: VCC→3.3V, GND→GND, OUT→GPIO32(IN1)
int mv = adc_read_mv(ADC_CHANNEL_4, cali_in1);
float temp_c = mv / 10.0f;
ESP_LOGI("SENSOR", "LM35 Temperature: %.2f °C", temp_c);
```

## ตัวอย่าง: ใช้หลายเซ็นเซอร์พร้อมกัน

### V1.5 Rev 3.1 (NECTEC Standard) — SW2=GPIO14
```
I2C init order:
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) เท่านั้น (ไม่มี KXTJ3)
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D)
3. adc_init_all()  → ADC1: LDR (GPIO36) เท่านั้น (IN1–IN4 ไม่รองรับ ADC)

Button config:
- SW1 = GPIO16
- SW2 = GPIO14  ← ต่างจาก Rev 3.1G
```

### V1.5 Rev 3.1G (Gravitech OEM) — SW2=GPIO17
```
I2C init order:
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) เท่านั้น (ไม่มี KXTJ3)
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D)
3. adc_init_all()  → ADC1: LDR (GPIO36) เท่านั้น (IN1–IN4 ไม่รองรับ ADC)

Button config:
- SW1 = GPIO16
- SW2 = GPIO17  ← ต่างจาก Rev 3.1
```

### V1.5 iA (INEX)
```
I2C init order (ต้องทำก่อนเสมอ):
1. i2c_init_bus0() → I2C_NUM_0: LED Matrix (0x70) + KXTJ3 (0x0E)
2. i2c_init_bus1() → I2C_NUM_1: LM73 (0x4D)
3. adc_init_all()  → ADC1: LDR + IN1..IN4
```

> **กฎทอง:** `i2c_driver_install()` เรียกได้แค่ครั้งเดียวต่อ port number
> หากเรียก 2 ครั้งจะเกิด error `ESP_ERR_INVALID_STATE`
