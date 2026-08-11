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

---

### ⚠️ Hardware Quirk 1 — LED Matrix Interleaved Display Mapping (CRITICAL)

> **กฎเหล็กจอ 16×8:** จอภาพ KidBright32 คือจอ 8×8 สองจอต่อร่วมกันบนชิป HT16K33 ตัวเดียว
> - **จอฝั่งซ้าย (Columns 0–7):** แมปอยู่ที่ **แอดเดรสคู่** (Even bytes) → `buf[1 + c*2]`
> - **จอฝั่งขวา (Columns 8–15):** แมปอยู่ที่ **แอดเดรสคี่** (Odd bytes) → `buf[2 + c*2]`
>
> ❌ **ห้ามส่งคอลัมน์ 0..15 เรียงกันตรงๆ** เด็ดขาด เพราะข้อมูลตัวอักษรซีกซ้ายและขวาจะถูกแยกสลับกัน ทำให้ตัวอักษรซ้อนทับกันเละ หรือเห็นแค่ฝั่งเดียว

```c
// ✅ ฟังก์ชันวาดจอ 16x8 ที่ถูกต้อง (สลับคู่-คี่ Interleaved Mapping)
static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00; // RAM Start Pointer 0x00
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];     // จอฝั่งซ้าย (Cols 0–7)  -> แอดเดรสคู่
        buf[2 + (c * 2)] = cols[c + 8]; // จอฝั่งขวา (Cols 8–15) -> แอดเดรสคี่
    }
    i2c_master_write_to_device(I2C_NUM_0, 0x70, buf, sizeof(buf), pdMS_TO_TICKS(100));
}
```

---

### ⚠️ Hardware Quirk 2 — LED Matrix Y-axis Inversion

```c
// ❌ ผิด — จะแสดงผลกลับหัว
out_cols[col] |= (1 << row);

// ✅ ถูก — ต้อง invert Y-axis เสมอ (hardware wired upside-down)
out_cols[col] |= (1 << (7 - row));
```

```c
// ฟังก์ชันแปลงผัง 16-bit Bitmap 8 แถว -> 16 Columns พร้อม Invert Y-axis
static void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                out_cols[col] |= (1 << (7 - row)); // กลับแกน Y
            }
        }
    }
}
```

---

### ⚠️ Hardware Quirk 3 — HT16K33 Init ต้องส่งทีละคำสั่ง (Single-Byte Writes)

```c
// ❌ ผิด — หน้าจอจะดับสนิท (Blank Display)
uint8_t cmd_on[] = {0x21, 0x81, 0xEF};
i2c_master_write_to_device(I2C_NUM_0, 0x70, cmd_on, 3, ...);

// ✅ ถูก — ส่งแยกทีละ 1 ไบต์
uint8_t cmd;
cmd = 0x21; i2c_master_write_to_device(I2C_NUM_0, 0x70, &cmd, 1, pdMS_TO_TICKS(100)); // Osc ON
cmd = 0x81; i2c_master_write_to_device(I2C_NUM_0, 0x70, &cmd, 1, pdMS_TO_TICKS(100)); // Display ON
cmd = 0xEF; i2c_master_write_to_device(I2C_NUM_0, 0x70, &cmd, 1, pdMS_TO_TICKS(100)); // Brightness Max
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
```---

## 📖 สรุปโครงสร้างและเชิงความหมาย (Semantics) ของไฟล์ใน sensor_examples/

### 1. `accel_kxtj3.c` — KXTJ3-1057 Accelerometer
- **Protocol:** I2C_NUM_0 (`SDA=GPIO21`, `SCL=GPIO22`, `Addr=0x0E`)
- **การแชร์ Bus:** ใช้ I2C_NUM_0 ร่วมกับ HT16K33 LED Matrix (`0x70`) — *ห้ามเรียก `i2c_driver_install()` ซ้ำ*
- **Registers:** `WHO_AM_I` (`0x0F` ต้องตอบกลับ `0x35`), `CTRL_REG1` (`0x1B`, ตั้งค่า `0xC0` สำหรับ 12-bit ±2g operating mode), `DATA_CTRL` (`0x21`, `0x06` สำหรับ 50 Hz ODR)
- **ฟังก์ชัน:** อ่านค่าความเร่ง 3 แกน ($X, Y, Z$) แปลงเป็น g-force และตรวจวัดมุมเอียง (Tilt State)

### 2. `accel_mc3479.c` — MC3479 Accelerometer
- **Protocol:** I2C_NUM_0 (`SDA=GPIO21`, `SCL=GPIO22`, `Addr=0x4C` หรือ `0x6C`)
- **คุณลักษณะ:** ไดรเวอร์สำหรับบอร์ดที่มี MC3479 3-axis accelerometer รองรับการตั้งค่าการสุ่มวัดความเร่ง และอ่านค่าดิจิทัล 3 แกน

### 3. `temp_lm73.c` — LM73 Temperature Sensor
- **Protocol:** I2C_NUM_1 (`SDA=GPIO4`, `SCL=GPIO5`, `Addr=0x4D`)
- **⚠️ GPIO Conflict Warning:** `GPIO4` ถูกแชร์ร่วมกับ BT LED — *ห้ามใช้ `gpio_set_level(GPIO_NUM_4, ...)` ขณะรันไดรเวอร์นี้*
- **Modes:** 11-bit default (`0.25 °C/LSB`) และ 14-bit high-resolution (`0.03125 °C/LSB`)
- **Registers:** `LM73_REG_TEMP` (`0x00`), `LM73_REG_CFG` (`0x01`), `LM73_REG_ID` (`0x07` ตอบกลับ `0x09`)

### 4. `adc_ldr_external.c` — LDR & JST Analog Inputs
- **API:** ESP-IDF v5.x `esp_adc/adc_oneshot.h` และ `esp_adc/adc_cali.h` (ห้ามใช้ `driver/adc.h` หรือ `esp_adc_cal.h` ซึ่งถูกลบออกใน v5)
- **Channels:**
  - `LDR` = GPIO36 (`ADC1_CHANNEL_0`) — LDR บนบอร์ด
  - `IN1` = GPIO32 (`ADC1_CHANNEL_4`) — พอร์ต JST External Analog Input 1
  - `IN2` = GPIO33 (`ADC1_CHANNEL_5`) — พอร์ต JST External Analog Input 2
- **Calibration:** รองรับ Curve Fitting หรือ Line Fitting Calibration Scheme แปลงค่า Raw ADC เป็นแรงดัน millivolts (mV)

### 5. `formulakid_sender.c` — FormulaKid Controller (Sender)
- **Protocol:** ESP-NOW (Wi-Fi Channel 1 Broadcast)
- **การทำงาน:** อ่านค่า Joystick RC Timing (Trigger & Capacitor discharge loop บน `GPIO26`/`GPIO32` และ `GPIO27`/`GPIO33`)
- **LED Display:** ขับ HT16K33 Matrix แสดงทิศทางคันโยก (ลูกศร Up/Down/Left/Right)
- **การส่งข้อมูล:** ส่งแพ็กเกจควบคุมความเร็วและทิศทางไปยังตัวรถผ่าน ESP-NOW

### 6. `fomulakid_receiver.c` — FormulaKid Vehicle (Receiver)
- **Driver:** ควบคุม DRV8833 Motor Driver ผ่าน LEDC PWM (5 kHz, 8-bit resolution)
- **Pinout:** `NSLEEP=GPIO23`, มอเตอร์ขวาเดินหน้า/ถอยหลัง (`GPIO18`/`GPIO26`), มอเตอร์ซ้ายเดินหน้า/ถอยหลัง (`GPIO19`/`GPIO27`)
- **Logic สั่งงาน:**
  - `999` = หยุด (`LED "--"`)
  - `-100..-10` = ถอยหลัง (`LED "D"`)
  - `10..100` = เดินหน้า (`LED "U"`)
  - `300..500` = เลี้ยวซ้าย/เลี้ยวขวา (`LED "L"` / `"R"`)

### 7. `balanced_robot.c` — Self-Balancing Robot PID
- **คุณลักษณะ:** โค้ดหุ่นยนต์สองล้อทรงตัวสมดุล ใช้ MPU6050 6-DOF IMU, PID Controller Loop (`PID_v1`), Encoder (`GPIO32`, `GPIO33`) และมอเตอร์ไดรเวอร์ (`GPIO18`, `GPIO19`, `GPIO26`, `GPIO27`)
- **การสื่อสาร:** รองรับ ESP-NOW Remote Control + MQTT Monitoring ผ่าน Wi-Fi Client

### 8. `kidbright_full_system_demo.c` & `all_sensors_demo.c` — System Integration Demos
- **คุณลักษณะ:** โค้ดตัวอย่างการบูรณาการระบบรวม อ่านค่าเซ็นเซอร์ทั้งหมดบนบอร์ด (LDR, LM73, KXTJ3, SW1/SW2, Buzzer) รันแบบ Multi-tasking บน FreeRTOS แสดงผลบน LED Matrix 16x8 พร้อมควบคุม Passive Buzzer บน GPIO13

---

## 🛠️ ข้อมูลโครงสร้างฮาร์ดแวร์และผังลายวงจร (Hardware & Schematics)

### 1. KidBright32 V1.5 Rev 3.1 PCB Specs (`PCB_KIDBRIGHT32_V1_5_Rev3_1.txt` / `Sch_KidBright32_updated.txt`)
- **Microcontroller:** ESP32-WROOM-32
- **Power Supply:** 5V USB / DC Terminal 5V, LDO Regulator 3.3V (AMS1117-3.3)
- **Matrix LED:** HT16K33 16x8 Dual-color LED Matrix (`I2C_NUM_0`, `SDA=GPIO21`, `SCL=GPIO22`)
- **Buzzer:** Passive Piezo Buzzer (`BZ1` บน `GPIO13` ขับด้วยทรานซิสเตอร์ BC847 / LEDC PWM)
- **Buttons:** SW1 (`GPIO16`), SW2 (`GPIO14` ยืนยันฮาร์ดแวร์ Rev 3.1 & 3.1G)
- **I2C Bus 1:** LM73 (`0x4D`) + MCP794xx RTC (`0x6F`) บน `SDA=GPIO4`, `SCL=GPIO5`

### 2. KB MiniBike Extension V0.3 Board Specs (`KBminibike_Ext_V0_3.txt`)
- **Connectors:** JST ZH 1.5mm 8-pin Connector (`J2`)
- **Motor Control:** NIDEC DC Motor, PWM Speed Control (`PWM`), Direction (`DIR`), Start Signal (`START`)
- **Encoder Feedback:** Encoder Signals `ENC_A` / `ENC_B` และแรงดันเลี้ยง `ENC_V+`
- **Power System:** DC Jack 12V (`J8`), Battery Terminal 2P (`BATT`), Toggle Switch (`MTS-102`)

### 3. Bike Controller V1 (`Bike_Controller_V1_20240627.txt`)
- **Inputs:** Dual RC Joystick Inputs (`JS1`, `JS2`), Status LEDs (`NLWIFI`, `NLIOT`), Direction Control Switches
- **Wireless:** ESP-NOW 2.4GHz Direct Wireless Protocol

---

## อ้างอิงเพิ่มเติม

- `kidbright32iA.md` — Full developer reference: MCU specs, HT16K33 register map, code examples, peripheral wiring
- `all_models.md` — ข้อมูลครบทุกรุ่น (Gen 1 ถึง V1.6) รวม sensor map, KB Chain, schematic links
