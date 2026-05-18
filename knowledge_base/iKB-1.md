# iKB-1 / iKB-1Z — INEX Expansion Board · Hardware Reference & ESP-IDF Rules
> **ผู้ผลิต:** Innovative Experiment Co., Ltd. (INEX), Thailand
> **Plugin KidBright IDE:** V2.2.0 · [store.kidbright.info/plugin/7/iKB-1](https://store.kidbright.info/plugin/7/iKB-1)
> **MicroPython library:** [github.com/inexglobal/uPython_iKB](https://github.com/inexglobal/uPython_iKB)

---

## 1. iKB-1 vs iKB-1Z

| Feature | iKB-1 | iKB-1Z |
|---------|-------|--------|
| I²C Address | ตายตัว `0x20` | กำหนดได้ (hardware jumper) |
| ใช้หลายตัวพร้อมกัน | ❌ | ✅ เปลี่ยน address แต่ละตัว |

> **AI RULE:** เมื่อ user ใช้ iKB-1Z ต้องถาม I²C address ที่ตั้งไว้ก่อน — ค่า default คือ `0x20`

## 2. Hardware Specs

| รายการ | ค่า |
|--------|-----|
| I²C Address (default) | `0x20` |
| Logic level | +3.3V (รองรับ host 3.3V และ 5V) |
| I²C Speed | 100 kHz / 400 kHz |
| I/O Ports | 8 ช่อง (0–7) — JST 2mm 3-pin |
| ADC Resolution | 8-bit หรือ 10-bit |
| Motor Driver (On-board) | 2 ช่อง (CH1, CH2) — 1A/ch สูงสุด +9V |
| Servo Outputs | 6 ช่อง (CH10–15) — regulated ≤+5V |
| External Motor | 2 ช่อง (CH3, CH4) |
| I²C Expansion | KB-CHAIN 5-pin + Grove 4-pin |
| แหล่งจ่ายไฟ Motor/Servo | External 6–9V DC barrel jack |

## 3. MCP23017-Compatible Register Map

| Register | Address | คำอธิบาย |
|----------|---------|----------|
| `IODIRA` | `0x00` | Port A direction: `1`=input, `0`=output |
| `IODIRB` | `0x01` | Port B direction |
| `GPPUA` | `0x0C` | Port A pull-up enable |
| `GPPUB` | `0x0D` | Port B pull-up enable |
| `GPIOA` | `0x12` | Port A — อ่านสถานะ pin |
| `GPIOB` | `0x13` | Port B — อ่านสถานะ pin |
| `OLATA` | `0x14` | Port A — เขียน output |
| `OLATB` | `0x15` | Port B — เขียน output |

> GPIO 0–7 ของ iKB-1 → **Port A (GPIOA/OLATA)** — Port B ใช้สำหรับ motor/extended features

## 4. ESP-IDF C — I²C Setup (Legacy API)

```c
#include "driver/i2c.h"

#define IKB_I2C_PORT    I2C_NUM_0   // ใช้ bus เดียวกับ HT16K33 Matrix
#define IKB_I2C_SDA     21
#define IKB_I2C_SCL     22
#define IKB_I2C_FREQ    400000
#define IKB_ADDR        0x20        // iKB-1Z default

// ⚠️ ถ้า HT16K33 init bus แล้ว ห้ามเรียก i2c_driver_install() อีกครั้ง
void ikb_i2c_init(void) {
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = IKB_I2C_SDA,
        .scl_io_num       = IKB_I2C_SCL,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = IKB_I2C_FREQ,
    };
    i2c_param_config(IKB_I2C_PORT, &conf);
    i2c_driver_install(IKB_I2C_PORT, conf.mode, 0, 0, 0);
}

esp_err_t ikb_write_reg(uint8_t reg, uint8_t value) {
    uint8_t buf[2] = { reg, value };
    return i2c_master_write_to_device(IKB_I2C_PORT, IKB_ADDR,
        buf, sizeof(buf), pdMS_TO_TICKS(10));
}

esp_err_t ikb_read_reg(uint8_t reg, uint8_t *out) {
    return i2c_master_write_read_device(IKB_I2C_PORT, IKB_ADDR,
        &reg, 1, out, 1, pdMS_TO_TICKS(10));
}
```

## 5. Digital I/O

```c
// Init: port 0 = output, port 1-7 = input with pull-up
void ikb_digital_init(void) {
    ikb_write_reg(0x00, 0b11111110);   // IODIRA
    ikb_write_reg(0x0C, 0b11111110);   // GPPUA — pull-up on inputs
}

// Write output (port 0)
void ikb_output(uint8_t port, uint8_t val) {
    uint8_t mask = (1 << port);
    uint8_t cur = 0;
    ikb_read_reg(0x14, &cur);   // OLATA
    if (val) cur |= mask; else cur &= ~mask;
    ikb_write_reg(0x14, cur);
}

// Read input (returns 0 or 1)
uint8_t ikb_input(uint8_t port) {
    uint8_t val = 0;
    ikb_read_reg(0x12, &val);   // GPIOA
    return (val >> port) & 0x01;
}
```

## 6. Motor Control (CH1/CH2)

> ⚠️ ต้องต่อ External 6–9V DC ที่ barrel jack — ถ้าไม่ต่อ CH1/CH2 จะไม่ทำงาน

```c
// speed: -100 to 100 (ลบ = ถอยหลัง)
esp_err_t ikb_motor(int8_t motor1_speed, int8_t motor2_speed) {
    uint8_t cmd[3] = { 0x70, (uint8_t)motor1_speed, (uint8_t)motor2_speed };
    return i2c_master_write_to_device(IKB_I2C_PORT, IKB_ADDR,
        cmd, sizeof(cmd), pdMS_TO_TICKS(10));
}

void ikb_fd(uint8_t s)          { ikb_motor(s, s); }        // เดินหน้า
void ikb_bk(uint8_t s)          { ikb_motor(-s, -s); }      // ถอยหลัง
void ikb_sl(uint8_t s)          { ikb_motor(0, s); }         // เลี้ยวซ้าย
void ikb_sr(uint8_t s)          { ikb_motor(s, 0); }         // เลี้ยวขวา
void ikb_spin_left(uint8_t s)   { ikb_motor(-s, s); }       // หมุนซ้าย
void ikb_spin_right(uint8_t s)  { ikb_motor(s, -s); }       // หมุนขวา
void ikb_stop(void)              { ikb_motor(0, 0); }        // หยุด
```

## 7. Servo Control (CH10–15)

```c
// channel: 10-15 (KidBright IDE แสดงเป็น 1-6)
// angle: 0-200 degrees
esp_err_t ikb_servo(uint8_t channel, uint8_t angle) {
    if (channel < 10 || channel > 15) return ESP_ERR_INVALID_ARG;
    if (angle > 200) angle = 200;
    uint8_t cmd[3] = { 0x50, channel, angle };
    return i2c_master_write_to_device(IKB_I2C_PORT, IKB_ADDR,
        cmd, sizeof(cmd), pdMS_TO_TICKS(10));
}
// ตัวอย่าง: servo ช่อง 1 (channel 10) หมุนไป 90°
// ikb_servo(10, 90);
```

## 8. กฎการใช้งานร่วมกับ KidBright32 (MANDATORY)

### I²C Bus Sharing

| สถานการณ์ | วิธี |
|-----------|------|
| ใช้ iKB-1 + HT16K33 บน I2C_NUM_0 | ใช้ bus เดียวกัน — เรียก `i2c_driver_install()` **ครั้งเดียว** แล้ว share (HT16K33=0x70, iKB-1=0x20) |
| iKB-1Z หลายตัว | เปลี่ยน address แต่ละตัว (0x20, 0x21, 0x22...) |

### MANDATORY Rules

1. **ห้าม `i2c_driver_install()` สองครั้ง** — ถ้า HT16K33 init แล้ว ให้ใช้ bus นั้นเลย
2. **iKB-1Z default address = `0x20`** — ไม่ชนกับ HT16K33 (0x70) หรือ LM73 (0x4D)
3. **Motor CH1/CH2 = External Power** — ถ้า motor ไม่หมุน ตรวจ adapter ก่อน
4. **Logic 3.3V** — ไม่ต้อง level shifter ระหว่าง ESP32 และ iKB-1Z
5. **iKB-1Z block "I²C Address"** ใน KidBright IDE ต้องเป็น block แรกเสมอ
6. **ต้องมี pull-up บน SDA/SCL** — ใน code (`GPIO_PULLUP_ENABLE`) หรือ external 4.7kΩ

## 9. Power Supply Summary

| Channel | จ่ายจาก | External Adapter? |
|---------|---------|------------------|
| GPIO 0–7, Serial, CH3/CH4 | KB-CHAIN (3.3V KidBright) | ❌ ไม่ต้อง |
| **Motor CH1/CH2, Servo CH10–15** | **External 6–9V barrel jack** | ✅ **จำเป็น** |

> ⚠️ ห้ามต่ออุปกรณ์ 3.3V เข้าพอร์ต 5V (Servo/Motor CH1-2)

## 10. KidBright IDE Blocks

| Block | คำอธิบาย |
|-------|---------|
| `I²C Address (iKB-1Z)` | กำหนด address — **ต้องเป็น block แรกเสมอ** |
| `Digital Read/Write CH[0-7]` | อ่าน/เขียนค่า digital |
| `Analog Read CH[0-7]` | อ่านค่า 10-bit (0–1023) |
| `Motor CH[1-4] speed [0-100%]` | ควบคุมมอเตอร์ |
| `Servo 180° CH[1-6] [0-200°]` | servo 180° |
| `Servo 360° CH[1-6] CW/CCW [0-100%]` | servo 360° |
| `Forward/Backward/Turn left/Turn right/Spin/Stop` | Robot car blocks |

## 11. อ้างอิง

- Product: https://inex.co.th/home/product/ikb-1z/
- Plugin (V2.2.0): https://store.kidbright.info/plugin/7/iKB-1
- Plugin source: https://github.com/inexglobal/ikb_1_plugin
- MicroPython: https://github.com/inexglobal/uPython_iKB
- micro:bit: https://github.com/inexglobal/pxt-iKB1z
- Verified: May 14, 2026
