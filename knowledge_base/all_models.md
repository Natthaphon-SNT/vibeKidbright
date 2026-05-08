# KidBright32 — All Models Reference

> **Framework:** ESP-IDF v5.x เท่านั้น · ห้ามใช้ Arduino Framework
> **MCU:** ESP32-WROOM-32 · Dual-core Xtensa LX6 240MHz · 4MB Flash · 520KB SRAM · 3.3V logic

---

## สรุปรุ่นทั้งหมด

| รุ่น | ผู้ผลิต | Accel | ADC on IN | SW2 GPIO | สถานะ |
|------|---------|-------|-----------|----------|-------|
| **V1.5 Rev 3.1** | NECTEC Standard | ไม่มี | ❌ | GPIO14 | Official |
| **V1.5 Rev 3.1G** | Gravitech OEM | ไม่มี | ❌ | GPIO14 | Official |
| **V1.5 iA** | INEX | KXTJ3-1057 | ✅ | GPIO17 | Official |
| **V1.6** | Gravitech | MPU-6050 | ✅ | — | Official (ล่าสุด) |

> ⚠️ **V1.5 Rev 3.1 และ Rev 3.1G**: SW2 = GPIO14 ทั้งคู่ — ยืนยันจาก hardware scan Apr 17 2026

---

## Core MCU — ESP32-WROOM-32

| Parameter | Value |
|-----------|-------|
| CPU | Dual-core Xtensa LX6, up to 240 MHz |
| Flash | 4 MB |
| RAM | 520 KB SRAM |
| Logic voltage | **3.3 V** (GPIO ไม่ทน 5V) |
| ADC | ADC1: CH0–CH7 (ADC2 ห้ามใช้เมื่อเปิด WiFi) |
| I2C | 2 buses (I2C_NUM_0 และ I2C_NUM_1) |

---

## On-Board Sensor Map — V1.5 Rev 3.1 (NECTEC Standard)

> ⚠️ ไม่มี Accelerometer · ไม่รองรับ ADC บน IN1–IN4 · SW2 = GPIO14

| Sensor | Protocol | Bus / Pin | Address |
|--------|----------|-----------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x6F |
| HT16K33 (Matrix) | I2C | I2C_NUM_0 · SDA=GPIO21 · SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 | GPIO | GPIO16 | — |
| **SW2** | GPIO | **GPIO14** | — |
| USB Host | GPIO | GPIO25 (Active LOW) | — |

```
I2C Init Order:
1. I2C_NUM_0 (GPIO21/22): HT16K33 (0x70)
2. I2C_NUM_1 (GPIO4/5):   LM73 (0x4D) + RTC (0x6F)
3. ADC1: LDR GPIO36 เท่านั้น
SW1=GPIO16, SW2=GPIO14
```

---

## On-Board Sensor Map — V1.5 Rev 3.1G (Gravitech OEM)

> ⚠️ ไม่มี Accelerometer · ไม่รองรับ ADC บน IN1–IN4 · SW2 = GPIO14 (เหมือน Rev 3.1)

| Sensor | Protocol | Bus / Pin | Address |
|--------|----------|-----------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x6F |
| HT16K33 (Matrix) | I2C | I2C_NUM_0 · SDA=GPIO21 · SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 | GPIO | GPIO16 | — |
| **SW2** | GPIO | **GPIO14** | — |
| USB Host | GPIO | GPIO25 (Active LOW) | — |

> 📋 I2C Scan (confirmed Apr 17 2026): NUM_0=`0x70` · NUM_1=`0x4D`+`0x6F`

```
I2C Init Order:
1. I2C_NUM_0 (GPIO21/22): HT16K33 (0x70)
2. I2C_NUM_1 (GPIO4/5):   LM73 (0x4D) + RTC (0x6F)
3. ADC1: LDR GPIO36 เท่านั้น
SW1=GPIO16, SW2=GPIO14 ← ยืนยัน Apr 17 2026
```

---

## On-Board Sensor Map — V1.5 iA (INEX)

> ✅ มี KXTJ3 Accelerometer · รองรับ ADC บน IN1–IN4 · SW2 = GPIO17

| Sensor | Protocol | Bus / Pin | Address |
|--------|----------|-----------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x6F |
| **KXTJ3-1057** | I2C | I2C_NUM_0 · SDA=GPIO21 · SCL=GPIO22 | **0x0E** |
| HT16K33 (Matrix) | I2C | I2C_NUM_0 · SDA=GPIO21 · SCL=GPIO22 | 0x70 |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 | GPIO | GPIO16 | — |
| **SW2** | GPIO | **GPIO17** | — |

```
I2C Init Order:
1. I2C_NUM_0 (GPIO21/22): HT16K33 (0x70) + KXTJ3 (0x0E)
2. I2C_NUM_1 (GPIO4/5):   LM73 (0x4D) + RTC (0x6F)
3. ADC1: LDR (GPIO36) + IN1(CH4) + IN2(CH5) + IN3(CH6) + IN4(CH7)
SW1=GPIO16, SW2=GPIO17
```

---

## On-Board Sensor Map — V1.6 (Gravitech)

> ✅ มี MPU-6050 (Accel+Gyro) · Gerora RGB LED 6 ดวง · รองรับ ADC บน IN1–IN4

| Sensor | Protocol | Bus / Pin | Address |
|--------|----------|-----------|---------|
| LDR | ADC | GPIO36 / ADC1_CH0 | — |
| LM73 (Temp) | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x4D |
| RTC MCP794xx | I2C | I2C_NUM_1 · SDA=GPIO4 · SCL=GPIO5 | 0x6F |
| **MPU-6050** | I2C | I2C_NUM_0 · SDA=GPIO21 · SCL=GPIO22 | **0x68** |
| HT16K33 (Matrix) | I2C | I2C_NUM_0 · SDA=GPIO21 · SCL=GPIO22 | 0x70 |
| Gerora RGB LED × 6 | WS2812B | GPIO (addressable) | — |
| Passive Buzzer | GPIO/PWM | GPIO13 (LEDC) | — |
| SW1 | GPIO | GPIO16 (shared with SERVO1) | — |

```
I2C Init Order:
1. I2C_NUM_0 (GPIO21/22): HT16K33 (0x70) + MPU-6050 (0x68)
2. I2C_NUM_1 (GPIO4/5):   LM73 (0x4D) + RTC (0x6F)
3. ADC1: LDR (GPIO36) + IN1(CH4) + IN2(CH5) + IN3(CH6) + IN4(CH7)
SW1=GPIO16 (shared SERVO1 — เลือกอย่างใดอย่างหนึ่ง)
```

---

## External JST Ports (ทุกรุ่น V1.5+)

| Port | GPIO | ADC Channel | หมายเหตุ |
|------|------|-------------|---------|
| IN1 | GPIO32 | ADC1_CH4 | ADC รองรับเฉพาะ iA และ V1.6 |
| IN2 | GPIO33 | ADC1_CH5 | ADC รองรับเฉพาะ iA และ V1.6 |
| IN3 | GPIO34 | ADC1_CH6 | Input-only · ไม่มี pull |
| IN4 | GPIO35 | ADC1_CH7 | Input-only · ไม่มี pull |
| OUT1 | GPIO26 | DAC2 | — |
| OUT2 | GPIO27 | — | — |

---

## GPIO Conflict Table

### V1.5 Rev 3.1 / Rev 3.1G

| GPIO | Conflict |
|------|---------|
| GPIO2 | Wi-Fi LED — อย่าใช้งานอื่น |
| GPIO4 | BT LED หรือ LM73 SDA — เลือกได้แค่อย่างเดียว |
| GPIO13 | Passive Buzzer — ต้องใช้ LEDC/PWM เสมอ |
| GPIO14 | SW2 — ห้ามใช้งานอื่น |
| GPIO16 | SW1 — ห้ามใช้งานอื่น |
| GPIO25 | USB Host (Active LOW) — อย่าใช้งานอื่น |
| GPIO36 | LDR ADC — Input-only, ไม่มี pull |

### V1.5 iA

| GPIO | Conflict |
|------|---------|
| GPIO2 | Wi-Fi LED |
| GPIO4 | BT LED หรือ LM73 SDA |
| GPIO13 | Passive Buzzer — LEDC/PWM เท่านั้น |
| GPIO16 | SW1 |
| GPIO17 | SW2 |
| GPIO36 | LDR ADC — Input-only |

### V1.6

| GPIO | Conflict |
|------|---------|
| GPIO2 | Wi-Fi LED |
| GPIO4 | BT LED หรือ LM73 SDA |
| GPIO13 | Passive Buzzer — LEDC/PWM เท่านั้น |
| GPIO16 | SW1 หรือ SERVO1 — เลือกได้แค่อย่างเดียว |
| GPIO36 | LDR ADC — Input-only |

---

## HT16K33 — LED Matrix Driver (ทุกรุ่น)

| Property | Detail |
|----------|--------|
| I2C Address | `0x70` |
| I2C Bus | I2C_NUM_0 (SDA=GPIO21, SCL=GPIO22) |
| Display | 16 columns × 8 rows |
| Oscillator ON | `0x21` |
| Display ON | `0x81` |
| Brightness MAX | `0xEF` |

### ⚠️ Y-axis Inversion (CRITICAL)

```c
// ❌ ผิด
out_cols[col] |= (1 << row);

// ✅ ถูก — hardware wired upside-down
out_cols[col] |= (1 << (7 - row));
```

---

## ADC Rules

### ❌ Legacy API (ถูกลบใน v5)

```c
#include "driver/adc.h"        // ❌ BANNED
#include "esp_adc_cal.h"       // ❌ BANNED
ADC_ATTEN_DB_11                // ❌ → ใช้ ADC_ATTEN_DB_12
```

### ✅ Oneshot API (v5.x)

```c
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"

adc_oneshot_new_unit(...)           // 1. Create unit
adc_oneshot_config_channel(...)     // 2. Config (ADC_ATTEN_DB_12)
adc_oneshot_read(...)               // 3. Read raw
adc_cali_raw_to_voltage(...)        // 4. Convert to mV
```

---

## Code Examples

### LDR (GPIO36) — ทุกรุ่น

```c
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"

adc_oneshot_unit_handle_t adc1_handle;
adc_cali_handle_t cali_handle;

void ldr_init(void) {
    adc_oneshot_unit_init_cfg_t unit_cfg = { .unit_id = ADC_UNIT_1 };
    adc_oneshot_new_unit(&unit_cfg, &adc1_handle);
    adc_oneshot_chan_cfg_t chan_cfg = {
        .atten = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    adc_oneshot_config_channel(adc1_handle, ADC_CHANNEL_0, &chan_cfg);
    adc_cali_curve_fitting_config_t cali_cfg = {
        .unit_id = ADC_UNIT_1,
        .atten = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    adc_cali_create_scheme_curve_fitting(&cali_cfg, &cali_handle);
}

int ldr_read_mv(void) {
    int raw = 0, mv = 0;
    adc_oneshot_read(adc1_handle, ADC_CHANNEL_0, &raw);
    adc_cali_raw_to_voltage(cali_handle, raw, &mv);
    return mv;
}
```

### LM73 Temperature (I2C_NUM_1) — ทุกรุ่น

```c
#include "driver/i2c.h"
#define I2C1_SDA GPIO_NUM_4
#define I2C1_SCL GPIO_NUM_5
#define LM73_ADDR 0x4D

void i2c1_init(void) {
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C1_SDA,
        .scl_io_num = I2C1_SCL,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = 100000,
    };
    i2c_param_config(I2C_NUM_1, &conf);
    i2c_driver_install(I2C_NUM_1, I2C_MODE_MASTER, 0, 0, 0); // เรียกครั้งเดียวเท่านั้น
}

float lm73_read_celsius(void) {
    uint8_t buf[2] = {0};
    i2c_master_read_from_device(I2C_NUM_1, LM73_ADDR, buf, 2, pdMS_TO_TICKS(100));
    int16_t raw = ((int16_t)buf[0] << 8) | buf[1];
    return (raw >> 5) * 0.25f;
}
```

### KXTJ3 Accelerometer — V1.5 iA เท่านั้น

```c
#include "driver/i2c.h"
#define KXTJ3_ADDR 0x0E
#define KXTJ3_CTRL 0x1B
#define KXTJ3_XOUT 0x06

void kxtj3_init(void) {
    uint8_t cmd[2] = { KXTJ3_CTRL, 0xC0 };
    i2c_master_write_to_device(I2C_NUM_0, KXTJ3_ADDR, cmd, 2, pdMS_TO_TICKS(100));
}

void kxtj3_read(float *ax, float *ay, float *az) {
    uint8_t reg = KXTJ3_XOUT, buf[6];
    i2c_master_write_read_device(I2C_NUM_0, KXTJ3_ADDR, &reg, 1, buf, 6, pdMS_TO_TICKS(100));
    int16_t rx = (int16_t)((buf[1] << 8) | buf[0]) >> 4;
    int16_t ry = (int16_t)((buf[3] << 8) | buf[2]) >> 4;
    int16_t rz = (int16_t)((buf[5] << 8) | buf[4]) >> 4;
    const float scale = 2.0f / 2048.0f;
    *ax = rx * scale; *ay = ry * scale; *az = rz * scale;
}
```

### MPU-6050 — V1.6 เท่านั้น

```c
#include "driver/i2c.h"
#define MPU6050_ADDR  0x68
#define MPU6050_PWR   0x6B
#define MPU6050_ACCEL 0x3B

void mpu6050_init(void) {
    uint8_t cmd[2] = { MPU6050_PWR, 0x00 };
    i2c_master_write_to_device(I2C_NUM_0, MPU6050_ADDR, cmd, 2, pdMS_TO_TICKS(100));
}

void mpu6050_read_accel(float *ax, float *ay, float *az) {
    uint8_t reg = MPU6050_ACCEL, buf[6];
    i2c_master_write_read_device(I2C_NUM_0, MPU6050_ADDR, &reg, 1, buf, 6, pdMS_TO_TICKS(100));
    int16_t rx = (int16_t)((buf[0] << 8) | buf[1]);
    int16_t ry = (int16_t)((buf[2] << 8) | buf[3]);
    int16_t rz = (int16_t)((buf[4] << 8) | buf[5]);
    const float scale = 2.0f / 32768.0f;
    *ax = rx * scale; *ay = ry * scale; *az = rz * scale;
}
```

---

## KB Chain — ระบบต่อเสริม (I2C_NUM_0)

### เซ็นเซอร์

| บอร์ด | Sensor | หมายเหตุ |
|-------|--------|---------|
| KB Chain VOC | Temp + Humidity + Pressure + Gas | BME680 |
| KB Chain UVA/UVB | UV-A (365nm) + UV-B (330nm) | — |
| BH1750 | ความเข้มแสง (lux) | — |
| ZX-DHT11 | อุณหภูมิ + ความชื้น | ต่อที่พอร์ต IN |
| KidUltra | Ultrasonic วัดระยะ | — |
| เซ็นเซอร์ฝุ่น SPS30 | PM1.0/2.5/4/10 | — |

### I/O Expansion

| บอร์ด | รายละเอียด |
|-------|-----------|
| KB Chain 4-CH ADC | ADC 12-bit 4 ช่อง (ADS1015, 0–5V) |
| KB Chain 5-CH Hub | ต่อบอร์ด KB Chain หลายตัว |
| iKB-1 / MotorKB | DIO 8ch · Servo 6ch · Motor 4ch |
| KB Chain OLED | จอ OLED (0x3C / 0x3D) |

---

## Golden Rules

1. **ESP-IDF v5.x ONLY** — ห้ามใช้ Arduino (`Wire.h`, `setup()`, `loop()`)
2. **ADC Oneshot API ONLY** — ห้ามใช้ `driver/adc.h` / `esp_adc_cal.h`
3. **ADC_ATTEN_DB_12** — ห้ามใช้ `ADC_ATTEN_DB_11`
4. **`i2c_driver_install()` ครั้งเดียวต่อ port** — เรียก 2 ครั้ง → `ESP_ERR_INVALID_STATE`
5. **Y-axis inversion** — ใช้ `(7 - row)` บน LED Matrix เสมอ
6. **3.3V logic** — GPIO ไม่ทน 5V
7. **ADC2 ห้ามใช้เมื่อเปิด WiFi**
8. **IN3/IN4 = Input-only** — ไม่มี pull-up/down
9. **Buzzer ต้องใช้ PWM/LEDC** — ไม่ใช้ `gpio_set_level()`
10. **ตรวจสอบ PCB silkscreen** — SW2: GPIO14 (Rev3.1/3.1G) vs GPIO17 (iA)

---

## อ้างอิงไฟล์

| ไฟล์ | เนื้อหา |
|------|--------|
| `README.md` | Quick start · sensor map สรุป · I2C init order |
| `all_models.md` | **ไฟล์นี้** — Reference ครบทุกรุ่น |
| `kidbright32iA.md` | Full developer reference |
| `adc_ldr_external.c` | ADC oneshot + calibration |
| `temp_lm73.c` | LM73 temperature |
| `accel_kxtj3.c` | KXTJ3 accelerometer (V1.5 iA) |
| `all_sensors_demo.c` | Demo รวมทุกเซ็นเซอร์ |
