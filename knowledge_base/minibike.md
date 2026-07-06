# KidBright Minibike — Hardware Rules & Firmware Reference
> ระบบรถจักรยานทรงตัว (Self-Balancing Bike) ควบคุมผ่านจอย KidBright Controller V1  
> สื่อสารด้วย **ESP-NOW** (ไม่ใช้ WiFi AP)  
> ESP-IDF v5.x เท่านั้น — ห้ามใช้ Arduino Framework

---

## ส่วนที่ 1: ภาพรวมระบบ

```
┌─────────────────────────────────┐   ESP-NOW (int32_t)   ┌────────────────────────────────┐
│  SENDER (Controller)            │ ────────────────────► │  RECEIVER (รถ)                 │
│  ESP32 bare module              │                        │  KidBright32iP + L298N Ext     │
│  Joystick ADC (2 แกน)          │                        │  MPU6050 + Servo + 2 Motors    │
│  OLED SH1106 (สถานะ+ทิศทาง)   │                        │                                │
└─────────────────────────────────┘                        └────────────────────────────────┘
```

| ด้าน | ไฟล์โค้ด | บอร์ด |
|------|---------|-------|
| **Sender** (Controller) | `minibike_sender.c` | ESP32 bare module (KidBright Controller V1) |
| **Receiver** (รถ) | `minibike_receiver.c` | KidBright32iP + L298N Extension Board |

---

## ส่วนที่ 2: SENDER — KidBright Controller V1

### 2.1 ฮาร์ดแวร์ฝั่ง Sender

| ชิ้นส่วน | รายละเอียด |
|---------|-----------|
| MCU | ESP32 bare module |
| Joystick | 2 แกน (X/Y), อ่านผ่าน ADC1 Legacy API |
| สวิตช์ | SW1 (GPIO16), SW2 (GPIO14) — Active LOW |
| จอแสดงผล | **OLED SH1106** 128×64 px, I2C `0x3C` |
| การสื่อสาร | ESP-NOW channel 1, WiFi STA (ไม่ต่อ AP) |

### 2.2 GPIO Pinout — SENDER

| สัญญาณ | GPIO | หมายเหตุ |
|--------|------|---------|
| Joystick X (ซ้าย-ขวา) | **GPIO34** | ADC1_CHANNEL_6, input-only |
| Joystick Y (ขึ้น-ลง) | **GPIO35** | ADC1_CHANNEL_7, input-only |
| SW1 | **GPIO16** | Active LOW, `GPIO_PULLUP_ENABLE` |
| SW2 | **GPIO14** | Active LOW, `GPIO_PULLUP_ENABLE` |
| OLED SDA | **GPIO21** | I2C_NUM_0, 400kHz |
| OLED SCL | **GPIO22** | I2C_NUM_0, 400kHz |

### 2.3 OLED SH1106 — กฎการใช้งาน

| พารามิเตอร์ | ค่า |
|-----------|-----|
| IC | **SH1106** (ไม่ใช่ SSD1306 — protocol ต่างกัน) |
| I2C Address | `0x3C` |
| ความละเอียด | 128 × 64 px (8 pages × 128 columns) |
| Internal RAM | 132 columns → **ต้องชดเชย column offset = 2** |
| I2C Port | `I2C_NUM_0` (SDA=GPIO21, SCL=GPIO22, 400kHz) |
| Framebuffer | `uint8_t s_oled_fb[1024]` (128×64/8) |

**Init Sequence (SH1106-specific):**
```c
oled_cmd(0xAE);             // display off
oled_cmd(0xD5); oled_cmd(0x80); // clock div
oled_cmd(0xA8); oled_cmd(0x3F); // mux ratio 64
oled_cmd(0xD3); oled_cmd(0x00); // display offset
oled_cmd(0x40);                  // start line 0
oled_cmd(0x8D); oled_cmd(0x14); // charge pump on  ← SH1106 ใช้ 0x8D (SSD1306=0xAD)
oled_cmd(0x20); oled_cmd(0x00); // horizontal addressing
oled_cmd(0xA1);                  // seg remap
oled_cmd(0xC8);                  // com scan direction
oled_cmd(0xDA); oled_cmd(0x12); // com pins
oled_cmd(0x81); oled_cmd(0xCF); // contrast
oled_cmd(0xD9); oled_cmd(0xF1); // pre-charge
oled_cmd(0xDB); oled_cmd(0x40); // vcomh
oled_cmd(0xA4);                  // all pixels off (RAM)
oled_cmd(0xA6);                  // normal display
oled_cmd(0xAF);                  // display on
```

**Page Write (ต้องชดเชย column offset 2):**
```c
oled_cmd(0xB0 | page);  // page address (0–7)
oled_cmd(0x02);          // lower column = 2  ← CRITICAL สำหรับ SH1106
oled_cmd(0x10);          // higher column = 0
// ส่ง 0x40 (data mode) + 128 bytes framebuffer
```

**OLED Display Layout (Sender):**
| Page (บรรทัด) | เนื้อหา |
|--------------|--------|
| Page 0 | สถานะ ESP-NOW: `"ESP-NOW READY"` หรือ `"ESP-NOW FAIL"` |
| Page 1 | (ว่าง) |
| Page 2 | ทิศทาง: `"DIR: FORWARD"` / `"DIR: BACKWARD"` / `"DIR: LEFT"` / `"DIR: RIGHT"` / `"DIR: STOP"` |

> OLED อัปเดต **เฉพาะเมื่อค่าเปลี่ยน** (ประหยัด I2C bandwidth)

> ❌ **ห้าม** ใช้ lower column = `0x00` เหมือน SSD1306 — จะทำให้ภาพเลื่อน 2 pixel

### 2.4 ADC API — Legacy (driver/adc.h)

> ⚠️ **CRITICAL:** Sender ใช้ **Legacy ADC API** (`driver/adc.h` + `esp_adc_cal.h`)  
> **ไม่ใช่** `esp_adc/adc_oneshot.h`

```c
// Headers
#include "driver/adc.h"
#include "esp_adc_cal.h"

// Init (เรียกครั้งเดียว)
adc1_config_width(ADC_WIDTH_BIT_12);
adc1_config_channel_atten(ADC1_CHANNEL_6, ADC_ATTEN_DB_11);  // GPIO34 — X
adc1_config_channel_atten(ADC1_CHANNEL_7, ADC_ATTEN_DB_11);  // GPIO35 — Y

// อ่านค่าใน task loop
int rx = adc1_get_raw(ADC1_CHANNEL_6);   // Joystick X
int ry = adc1_get_raw(ADC1_CHANNEL_7);   // Joystick Y
```

| พารามิเตอร์ | ค่า |
|-----------|-----|
| Bit Width | `ADC_WIDTH_BIT_12` (0–4095) |
| Attenuation | `ADC_ATTEN_DB_11` (0–3.9V range) |

### 2.5 ADC Calibration — Joystick

```c
#define X_MIN       0
#define X_CENTER    1680    // ค่า center จริงของ GPIO34
#define X_MAX       4095
#define Y_MIN       0
#define Y_CENTER    1794    // ค่า center จริงของ GPIO35
#define Y_MAX       4095
#define DEAD_ZONE   10      // % dead zone รอบ center
```

**การแปลง ADC → -100…+100 (กลับทิศด้วย):**
```c
int joy_x = (rx >= X_CENTER)
    ? (int)map_val(rx, X_CENTER, X_MAX,    0,    100)
    : (int)map_val(rx, X_MIN,    X_CENTER, -100, 0);
// negate เพราะ joystick ติดตั้งกลับทิศ
joy_x = (int)constrain_val(-joy_x, -100, 100);
```

### 2.6 โครงสร้างโค้ด Sender

```c
void app_main(void) {
    gpio_init_pins();   // SW1/SW2
    adc_init();         // ADC1 legacy
    oled_init();        // I2C + SH1106 init  ← ต้องก่อน espnow
    espnow_init();      // WiFi STA + ESP-NOW
    xTaskCreate(controller_task, "controller_task", 4096, NULL, 5, NULL);
}
```

**`s_espnow_ready` flag:**
```c
static volatile bool s_espnow_ready = false;
// set จาก send callback
static void on_sent(const uint8_t *mac_addr, esp_now_send_status_t status) {
    (void)mac_addr;
    s_espnow_ready = (status == ESP_NOW_SEND_SUCCESS);
}
```

---

## ส่วนที่ 3: RECEIVER — KidBright32iP + L298N Extension

### 3.1 ฮาร์ดแวร์ฝั่ง Receiver

| ชิ้นส่วน | รายละเอียด |
|---------|-----------|
| MCU | KidBright32iP (ESP32-WROOM-32) |
| IMU | MPU6050, I2C address `0x68` |
| Motor Driver | L298N (บน Extension Board) — 2 channel |
| มอเตอร์ A | Reaction Wheel (ล้อทรงตัว) |
| มอเตอร์ B | Rear Wheel (ล้อหลัง ขับเคลื่อน) |
| Servo | ควบคุมมุมเลี้ยว (Steering) GPIO15 |
| การสื่อสาร | ESP-NOW channel 1 |

### 3.2 GPIO Pinout — RECEIVER

#### MPU6050
| สัญญาณ | GPIO | หมายเหตุ |
|--------|------|---------|
| SDA | **GPIO4** | I2C_NUM_0, 400kHz |
| SCL | **GPIO5** | I2C_NUM_0, 400kHz |
| I2C Address | `0x68` | — |

#### L298N — Motor A (Reaction Wheel)
| สัญญาณ | GPIO | ฟังก์ชัน |
|--------|------|---------|
| IN1 | **GPIO12** | ทิศทาง phase 1 |
| IN2 | **GPIO23** | ทิศทาง phase 2 |
| ENA | **GPIO26** | PWM ความเร็ว — LEDC_CHANNEL_0 |

#### L298N — Motor B (Rear Wheel)
| สัญญาณ | GPIO | ฟังก์ชัน |
|--------|------|---------|
| IN3 | **GPIO18** | ทิศทาง phase 1 |
| IN4 | **GPIO19** | ทิศทาง phase 2 |
| ENB | **GPIO27** | PWM ความเร็ว — LEDC_CHANNEL_1 |

#### Servo
| สัญญาณ | GPIO | ฟังก์ชัน |
|--------|------|---------|
| Signal | **GPIO15** | LEDC_CHANNEL_2, 50Hz, 16-bit |

### 3.3 LEDC PWM Configuration — RECEIVER

| Channel | GPIO | Timer | Resolution | Freq | ใช้กับ |
|---------|------|-------|-----------|------|--------|
| LEDC_CHANNEL_0 | GPIO26 | LEDC_TIMER_0 | 8-bit (0–255) | 5kHz | Motor A ENA |
| LEDC_CHANNEL_1 | GPIO27 | LEDC_TIMER_0 | 8-bit (0–255) | 5kHz | Motor B ENB |
| LEDC_CHANNEL_2 | GPIO15 | LEDC_TIMER_1 | 16-bit | 50Hz | Servo |

### 3.4 L298N Truth Table

**Motor A (Reaction Wheel):**
| IN1 | IN2 | ENA | ผลลัพธ์ |
|-----|-----|-----|--------|
| 1 | 0 | duty | หมุน CW (pid > 0) |
| 0 | 1 | duty | หมุน CCW (pid < 0) |
| 0 | 0 | 0 | หยุด |

**Motor B (Rear Wheel):**
| IN3 | IN4 | ENB | ผลลัพธ์ |
|-----|-----|-----|--------|
| 0 | 1 | 150 | เดินหน้า |
| 1 | 0 | 150 | ถอยหลัง |
| 0 | 0 | 0 | หยุด |

### 3.5 Servo — มุมเลี้ยว

| turn | มุม | ความหมาย |
|------|-----|---------|
| 0 | 90° | ตรง (center) |
| +1 | 60° | เลี้ยวขวา |
| -1 | 120° | เลี้ยวซ้าย |

```c
float pulse_us = 500.0f + (float)angle * (1900.0f / 180.0f);
uint32_t duty  = (uint32_t)(pulse_us / 20000.0f * 65536.0f);
```

### 3.6 MPU6050 — การอ่านมุม

```c
// Wake up: เขียน 0x00 ไปที่ register 0x6B
// อ่าน 6 bytes จาก register 0x3B (AX H/L, AY H/L, AZ H/L)
float angle = atan2f((float)accY, (float)accZ) * 180.0f / M_PI;
```

---

## ส่วนที่ 4: ESP-NOW Protocol

### 4.1 รูปแบบข้อมูล

```
ชนิด: int32_t (4 bytes) — ห้ามใช้ float
```

| ค่า (int32_t) | ความหมาย | Action ฝั่งรถ |
|--------------|---------|--------------|
| `999` | STOP | setpoint = base, หยุดทุกอย่าง |
| `1` ถึง `100` | เดินหน้า (%) | rear wheel forward + setpoint offset + |
| `-1` ถึง `-100` | ถอยหลัง (%) | rear wheel backward + setpoint offset - |
| `401` ถึง `500` | เลี้ยวขวา | servo 60° |
| `300` ถึง `399` | เลี้ยวซ้าย | servo 120° |

### 4.2 Priority Logic (Sender)

```c
// Y มี priority สูงกว่า X
if (abs_val(joy_y) >= abs_val(joy_x)) {
    // ส่ง joy_y (เดินหน้า/ถอย)
} else {
    // ส่ง joy_x + 400 (เลี้ยว)
}
// ถ้าทั้งคู่ใน dead zone → ส่ง 999
```

### 4.3 espnow_init() บน Sender

```c
nvs_flash_init();
esp_netif_init();
esp_event_loop_create_default();
esp_wifi_init(&cfg);
esp_wifi_set_mode(WIFI_MODE_STA);
esp_wifi_start();
esp_wifi_disconnect();          // ไม่ต่อ AP
// แสดง MAC ทาง Serial
esp_now_init();
esp_now_register_send_cb(on_sent);
esp_now_add_peer(&peer);        // channel=1, encrypt=false
```

### 4.4 กฎ ESP-NOW (MANDATORY)

1. **WiFi mode: `WIFI_MODE_STA`** — ห้ามต่อ AP ใดๆ
2. **Channel: 1** ทั้ง sender และ receiver
3. **ชนิดข้อมูล `int32_t` เท่านั้น**
4. **MAC Address hardcode** ใน sender:
   ```c
   static uint8_t car_mac[] = {0xF8, 0xB3, 0xB7, 0x2A, 0xF9, 0x28};
   ```
5. ดู MAC ของรถจาก Serial: `Car MAC: XX:XX:XX:XX:XX:XX`
6. ทั้งสองฝั่ง: `esp_wifi_disconnect()` หลัง `esp_wifi_start()`
7. ห้ามเรียก blocking function จาก ESP-NOW callback

### 4.5 Send/Receive Callback Signatures

```c
// Sender (ESP-IDF v5.4.x)
static void on_sent(const uint8_t *mac_addr, esp_now_send_status_t status)
{
    (void)mac_addr;
    s_espnow_ready = (status == ESP_NOW_SEND_SUCCESS);
}

// Receiver (ESP-IDF v5.x)
static void on_recv(const esp_now_recv_info_t *info, const uint8_t *data, int len)
{
    if (len == sizeof(int32_t)) memcpy((void *)&g_cmd, data, sizeof(int32_t));
}
```

---

## ส่วนที่ 5: Self-Balancing PID (Receiver)

### 5.1 PID Parameters

```c
float Kp = 15.0f, Ki = 0.0f, Kd = 0.8f;
#define SETPOINT_BASE  -1.0f   // มุมสมดุล (°)
#define SETPOINT_MAX    8.0f   // offset สูงสุดจากจอย (°)
#define INTEGRAL_LIMIT 50.0f
#define DEADZONE        5      // PWM ต่ำกว่านี้หยุด
```

### 5.2 Setpoint Adjustment

```c
// cmd=50  → sp = -1 + 4.0 = +3.0° (เอียงหน้า = เดินหน้า)
// cmd=-50 → sp = -1 - 4.0 = -5.0° (เอียงหลัง = ถอยหลัง)
float sp = SETPOINT_BASE + ((float)cmd / 100.0f) * SETPOINT_MAX;
```

### 5.3 Coast Before Direction Change

```c
// หยุด 20ms ก่อนสลับทิศ ป้องกัน inrush current
analog_write_motor(LEDC_CH_A, 0);
gpio_set_level(PIN_IN1, 0); gpio_set_level(PIN_IN2, 0);
vTaskDelay(pdMS_TO_TICKS(20));
// สลับทิศ → set PWM
vTaskDelay(pdMS_TO_TICKS(20));
```

---

## ส่วนที่ 6: CMakeLists.txt

### Sender
```cmake
idf_component_register(SRCS "main.c"
    INCLUDE_DIRS "."
    PRIV_REQUIRES driver esp_wifi nvs_flash esp_event esp_netif)
```
> Sender ใช้ legacy ADC (อยู่ใน `driver`) และ I2C (`driver`) — **ไม่ต้องใส่ `esp_adc`**

### Receiver
```cmake
idf_component_register(SRCS "main.c"
    INCLUDE_DIRS "."
    PRIV_REQUIRES driver esp_wifi nvs_flash esp_event esp_netif)
```
> **ห้าม** ใส่ `esp_now` ใน PRIV_REQUIRES — link อัตโนมัติผ่าน `esp_wifi`

---

## ส่วนที่ 7: สรุป GPIO ทั้งระบบ

### SENDER
| GPIO | ฟังก์ชัน | Direction |
|------|---------|-----------|
| 14 | SW2 | Input, pull-up, active LOW |
| 16 | SW1 | Input, pull-up, active LOW |
| 21 | OLED SDA | I2C_NUM_0 |
| 22 | OLED SCL | I2C_NUM_0 |
| 34 | JS_X (ADC1_CH6) | ADC Input, input-only |
| 35 | JS_Y (ADC1_CH7) | ADC Input, input-only |

### RECEIVER
| GPIO | ฟังก์ชัน | Direction |
|------|---------|-----------|
| 4 | MPU6050 SDA | I2C_NUM_0 |
| 5 | MPU6050 SCL | I2C_NUM_0 |
| 12 | Motor A IN1 | Output |
| 15 | Servo PWM | PWM Out (LEDC, 50Hz, 16-bit) |
| 18 | Motor B IN3 | Output |
| 19 | Motor B IN4 | Output |
| 23 | Motor A IN2 | Output |
| 26 | Motor A ENA | PWM Out (LEDC, 5kHz, 8-bit) |
| 27 | Motor B ENB | PWM Out (LEDC, 5kHz, 8-bit) |

---

## ส่วนที่ 8: กฎบังคับสำหรับ AI (MANDATORY RULES)

### DO ✅
- ใช้ Legacy ADC (`driver/adc.h`, `adc1_get_raw()`) สำหรับ Sender
- ใช้ `ADC1_CHANNEL_6` (GPIO34) และ `ADC1_CHANNEL_7` (GPIO35)
- ใช้ `ADC_ATTEN_DB_11` สำหรับ attenuation
- ใช้ OLED SH1106 ด้วย column offset `0x02` เสมอ
- init OLED (`oled_init()`) **ก่อน** `espnow_init()`
- อัปเดต OLED เฉพาะเมื่อค่าเปลี่ยน (`changed` flag)
- ใช้ `int32_t` สำหรับ ESP-NOW — ห้ามใช้ `float`
- ใส่ `(void)mac_addr;` ใน send callback เพื่อกัน warning
- coast มอเตอร์ 20ms ก่อนสลับทิศ
- hardcode MAC address ของรถใน sender

### DON'T ❌
- ❌ ห้ามใช้ `esp_adc/adc_oneshot.h` สำหรับ Sender — ใช้ legacy API
- ❌ ห้ามใช้ `ADC_CHANNEL_6/7` (oneshot) — ใช้ `ADC1_CHANNEL_6/7` (legacy)
- ❌ ห้ามใช้ column lower = `0x00` กับ SH1106 — ต้อง `0x02`
- ❌ ห้ามใช้ SSD1306 init sequence กับ SH1106 (charge pump command ต่างกัน)
- ❌ ห้ามส่ง `float` ผ่าน ESP-NOW
- ❌ ห้ามใส่ `esp_now` ใน CMakeLists PRIV_REQUIRES
- ❌ ห้ามใช้ GPIO34/35 เป็น output หรือ pull-up/down
- ❌ ห้ามเรียก blocking function จาก ESP-NOW callback

---

## ส่วนที่ 9: ไฟล์อ้างอิง

| ไฟล์ | คำอธิบาย |
|------|---------|
| `minibike_sender.c` | Controller (ADC legacy + OLED SH1106 + ESP-NOW TX) |
| `minibike_receiver.c` | Receiver (PID + L298N + Servo + ESP-NOW RX) |
| `calibrate_balancing_joystick.c` | Calibrate tool — ดู raw ADC บน OLED SH1106 |
| `calibrate_balancing_bike.c` | Tune PID balance (ไม่มี ESP-NOW) |
| `Bike_Controller_V1_20240627.pdf` | Schematic KidBright Controller V1 |
| `KBminibike_Ext_V0_3.pdf` | Extension Board (L298N, Servo connector) |
| `PCB_KIDBRIGHT32_V1_5_Rev3_1.pdf` | PCB layout KidBright32 V1.5 Rev 3.1 |
