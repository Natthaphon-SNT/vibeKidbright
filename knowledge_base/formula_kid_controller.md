# Formula Kid Controller — Plugin Rules & Hardware Reference
> Plugin สำหรับ **KidBrightIDE / KBIDE** · บอร์ด **KB1.3 (V1.5 Rev 3.1)** และ **KB1.5G (V1.5 Rev 3.1G)**
> ใช้โปรโตคอล **ESP-NOW** สื่อสารแบบ Unicast ระหว่าง Controller (บอร์ดถือ) และ Receiver (บอร์ดรถ)

---

## ส่วนที่ 1: ฮาร์ดแวร์ — สวิตช์ S1, S2 บนบอร์ด KidBright32 (KB1.3/KB1.5G)

### ข้อมูล GPIO ของ S1 และ S2

> ⚠️ **CRITICAL — Formula Kid Controller ใช้ S1=GPIO36, S2=GPIO39 ไม่ใช่ SW1/SW2 ปุ่มบนบอร์ด**

| สวิตช์ | GPIO (ESP32) | อ้างอิง | ข้อจำกัดสำคัญ |
|--------|-------------|---------|--------------|
| **S1** | **GPIO36 (VP)** | ADC1_CH0 | Input-only · ไม่มี internal pull-up/pull-down |
| **S2** | **GPIO39 (VN)** | ADC1_CH3 | Input-only · ไม่มี internal pull-up/pull-down |

> ⚠️ **GPIO36 และ GPIO39 เป็น input-only pins** — ห้ามกำหนดเป็น output เด็ดขาด
> ⚠️ บอร์ด KidBright32 มีวงจร **pull-up ภายนอก** ให้แล้ว — ห้ามใช้ `INPUT_PULLUP` หรือ `GPIO_PULLUP_ENABLE` ใน code

### พฤติกรรมทางไฟฟ้า (Logic Level)

| สถานะ | ระดับสัญญาณ | ค่า `gpio_get_level()` |
|-------|------------|----------------------|
| ปล่อยปุ่ม (Released) | HIGH | 1 |
| กดปุ่ม (Pressed) | LOW | 0 |

### กฎการใช้งาน S1, S2 (MANDATORY)

1. **Input-only**: GPIO36/39 ห้ามกำหนดเป็น output เด็ดขาด
2. **ห้าม pull-up ใน code**: บอร์ดมี external pull-up แล้ว ห้ามใช้ `GPIO_PULLUP_ENABLE` หรือ `INPUT_PULLUP`
3. **ห้ามใช้ interrupt**: เมื่อใช้ ESP-NOW ร่วมกัน ห้ามใช้ ISR interrupt บน GPIO36/39 เพราะจะเกิด glitch จาก ADC/WiFi — ให้ใช้ **polling** เท่านั้น
4. **Active LOW**: กด = LOW (0), ปล่อย = HIGH (1)
5. **Config ด้วย `GPIO_MODE_INPUT`** พร้อม `GPIO_PULLUP_DISABLE` และ `GPIO_PULLDOWN_DISABLE`

### ตัวอย่าง GPIO Config (ESP-IDF v5.x) — ถูกต้อง

```c
#include "driver/gpio.h"

void s1_s2_init(void) {
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << GPIO_NUM_36) | (1ULL << GPIO_NUM_39),
        .mode         = GPIO_MODE_INPUT,
        .pull_up_en   = GPIO_PULLUP_DISABLE,   // External pull-up on board
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_DISABLE,     // NEVER use interrupt on GPIO36/39 with ESP-NOW
    };
    gpio_config(&io_conf);
}

// Read (0 = pressed, 1 = released)
int s1 = gpio_get_level(GPIO_NUM_36);
int s2 = gpio_get_level(GPIO_NUM_39);
```

---

## ส่วนที่ 2: Plugin Joystick (ฝั่ง Controller) — RC Timing

> 🔑 **CRITICAL DISCOVERY:** Joystick **ไม่ได้ใช้ ADC** บน GPIO36/39 — ใช้วงจร **RC Timing** ผ่าน GPIO คนละชุด!

### GPIO Pins จริง (จาก Plugin generators.js)

| Joystick | แกน | Trigger GPIO (Output) | Capture GPIO (Input+ISR) |
|----------|-----|-----------------------|--------------------------|
| **JS1** | ขึ้น/ลง (Y) | **GPIO26** (OUT1) | **GPIO32** (IN1) |
| **JS2** | ซ้าย/ขวา (X) | **GPIO27** (OUT2) | **GPIO33** (IN2) |

> ⚠️ GPIO36/39 คือ **S1/S2 switches** เท่านั้น — ไม่เกี่ยวกับ Joystick position

### หลักการ RC Timing (จาก Plugin joystick.cpp)

```
1. ตั้ง trig_gpio = HIGH  → discharge capacitor (รอ 10ms)
2. จับเวลา start_ts = esp_timer_get_time()
3. ตั้ง trig_gpio = LOW   → เริ่มชาร์จ capacitor
4. ISR (rising edge บน cap_gpio) บันทึก stop_ts
5. คำนวณ:
   resistance = (stop_ts - start_ts) × RC_FACTOR_5V − R_SERIE
   raw_pos    = (resistance × 200.0 / 10000.0) − 100
```

ค่าคงที่จาก Plugin:
- `R_SERIE = 1000` Ω
- `RC_FACTOR_5V = 9.788075945`
- `CAP_TIMEOUT_US = 500000` (500ms)
- `DISCHARGE_MS = 10`

### Calibration (ตรงกับบล็อก)

```c
// ขั้นตอน calibrate หลัง raw_pos:
pos -= calibrate_release;           // ลบค่า release (JS1=-3, JS2=-3)
if (pos < 0) pos = pos * 100 / abs(calibrate_min - calibrate_release);
if (pos > 0) pos = pos * 100 / abs(calibrate_max - calibrate_release);
pos = clamp(pos, -100, 100);
```

| Joystick | Release | DownMost / LeftMost | UpMost / RightMost |
|----------|---------|---------------------|--------------------|
| JS1 | -3 | -100 | 89 |
| JS2 | -3 | -100 | 90 |

### Dead Zone และ Encode ค่าส่งผ่าน ESP-NOW

| สถานการณ์ | ESPNOW_VALUE ที่ส่ง | ความหมาย |
|-----------|--------------------|---------| 
| JS1 ≥ 10 (เดินหน้า) | `10` ถึง `100` (ค่าบวก) | เดินหน้า — แสดง "U" |
| JS1 ≤ -10 (ถอยหลัง) | `-100` ถึง `-10` (ค่าลบ) | ถอยหลัง — แสดง "D" |
| JS2 ≥ 10 (เลี้ยวขวา) | `410` ถึง `500` (JS2+400) | เลี้ยวขวา — แสดง "R" |
| JS2 ≤ -10 (เลี้ยวซ้าย) | `300` ถึง `399` (JS2+400) | เลี้ยวซ้าย — แสดง "L" |
| ทั้งคู่ dead zone | `999` | หยุด — แสดง "--" |

> **Encoding rule:** ส่งค่า JS1 ตรงๆ (ไม่ invert) · JS2 บวก offset `+400` ก่อนส่ง เพื่อแยกช่วงค่าจาก JS1

---


## ส่วนที่ 3: Plugin DRV8833 Motor Driver (ฝั่งรถ)

ชิปขับมอเตอร์ DRV8833 ควบคุมผ่าน ESP32 GPIO บนบอร์ดรถ

### GPIO Mapping — DRV8833

| สัญญาณ | GPIO | หมายเหตุ |
|--------|------|---------|
| nSLEEP | GPIO23 | Enable chip (HIGH = active) |
| Motor A1 | GPIO18 | |
| Motor A2 | GPIO26 (OUT1) | Active LOW connector |
| Motor B1 | GPIO19 | |
| Motor B2 | GPIO27 (OUT2) | Active LOW connector |

### กฎทิศทางการเคลื่อนที่

| Direction Code | ทิศทาง | Motor A | Motor B |
|---------------|---------|---------|---------|
| 0 | เดินหน้า (Forward) | +speed | +speed |
| 1 | ถอยหลัง (Backward) | -speed | -speed |
| 2 | เลี้ยวซ้าย (Turn Left) | +speed | -speed |
| 3 | เลี้ยวขวา (Turn Right) | -speed | +speed |

### กฎการรับค่า ESP-NOW และสั่งมอเตอร์

| ค่า ESPNOW_VALUE | การกระทำ | รายละเอียด |
|-----------------|---------|-----------|
| `999` | **หยุด** | `drv8833.stop()` |
| `10` ถึง `100` | **เดินหน้า** (direction=0) | speed = ค่า |
| `-100` ถึง `-10` | **ถอยหลัง** (direction=1) | speed = \|ค่า\| |
| `300` ถึง `399` | **เลี้ยวซ้าย** (direction=2) | แปลง: ค่า - 400 → -100 ถึง 0 |
| `400` ถึง `500` | **เลี้ยวขวา** (direction=3) | แปลง: ค่า - 400 → 0 ถึง 100 |

---

## ส่วนที่ 4: กฎการใช้ S1, S2 เพื่อส่งคำสั่งพิเศษผ่าน ESP-NOW

เนื่องจาก JS1 และ JS2 ใช้ช่วงค่า `-100` ถึง `500` (รวม `999`) ไปแล้ว ค่าที่ส่งสำหรับ S1/S2 ต้องไม่ซ้ำกับช่วงเหล่านี้

### ตัวอย่างค่าที่แนะนำสำหรับ S1, S2

| สถานะ | ESPNOW_VALUE ที่แนะนำ | ฟังก์ชัน |
|-------|----------------------|---------|
| กด **S1** เท่านั้น | `-200` | ฟังก์ชัน A (เช่น boost / เร็วพิเศษ) |
| กด **S2** เท่านั้น | `-300` | ฟังก์ชัน B (เช่น honk / เสียงแตร) |
| กด **S1 + S2** พร้อมกัน | `-400` | ฟังก์ชัน C (เช่น reset / สลับ mode) |

### Priority Order การส่งค่า (ตามบล็อก)

```
1. JS1 ≥ 10 หรือ JS1 ≤ -10 → ส่ง JS1 ตรงๆ  (JS1 มีความสำคัญกว่า JS2)
2. JS2 ≥ 10 หรือ JS2 ≤ -10 → ส่ง JS2 + 400
3. ทั้งคู่ dead zone (-10 < JS < 10) → ส่ง 999 (stop)
```

> ⚠️ **JS1 มีความสำคัญสูงกว่า JS2 เสมอ** — ถ้า JS1 เคลื่อน ค่า JS2 จะถูกละเว้น
> ℹ️ S1/S2 (GPIO36/39) เป็นวงจรแยกต่างหาก — ถ้าต้องการใช้ในโปรเจกต์นี้ให้ encode เป็นค่าพิเศษที่ไม่ชนช่วง JS

---

## ส่วนที่ 5: กฎการสื่อสาร ESP-NOW

### พารามิเตอร์การส่ง

| พารามิเตอร์ | ค่า | หมายเหตุ |
|------------|-----|---------|
| โปรโตคอล | ESP-NOW | ไม่ต้องใช้ Router |
| โหมด | Unicast | ส่งไปยัง MAC Address เฉพาะของบอร์ดรถ |
| ชนิดข้อมูล | Integer (int) | ส่งตัวเลขจำนวนเต็มหนึ่งตัวต่อครั้ง |
| อัตราการส่ง | ทุก 500ms | `vTaskDelay(pdMS_TO_TICKS(500))` |

### กฎสำคัญ ESP-NOW (MANDATORY)

1. **ห้ามใช้ IoT (WiFi) พร้อมกับ ESP-NOW**: ถ้า SSID/Password ถูกตั้งไว้ ESP-NOW จะส่งได้แค่ครั้งแรกครั้งเดียว ต้องลบ SSID/Password ออกจาก config ก่อนใช้ ESP-NOW
2. **MAC Address ต้องถูกต้อง**: ต้อง hardcode MAC Address ของบอร์ดรถที่ต้องการส่งถึง
3. **ส่งแบบ polling**: ไม่ใช้ interrupt บน GPIO36/39
4. **ค่าที่ส่งเป็น int เดียว**: ไม่ใช่ struct หรือ array (ตามโปรโตคอล Formula Kid)

### ⚠️ BREAKING API CHANGE — ESP-IDF v5.5+ Send Callback

> **`esp_now_register_send_cb` callback signature เปลี่ยนใน ESP-IDF v5.5+**
> ใช้ signature เก่าจะ compile error: `incompatible pointer type`

```c
// ❌ WRONG — ESP-IDF ≤ v5.4 (compile error บน v5.5+)
static void espnow_send_cb(const uint8_t *mac_addr, esp_now_send_status_t status) { }

// ✅ CORRECT — ESP-IDF v5.5+ (ALWAYS use this)
static void espnow_send_cb(const wifi_tx_info_t *tx_info, esp_now_send_status_t status) {
    if (status == ESP_NOW_SEND_SUCCESS) {
        ESP_LOGI("ESPNOW", "Send OK");
    } else {
        ESP_LOGW("ESPNOW", "Send FAIL");
    }
    // ถ้าต้องการ MAC address ของ peer → tx_info->peer_addr
}

// Registration ยังเหมือนเดิม:
ESP_ERROR_CHECK(esp_now_register_send_cb(espnow_send_cb));
```

### ตัวอย่าง Pseudocode สำหรับ Controller Loop (ESP-IDF v5.x)

```c
// Main loop (polling every 500ms — matches block Delay 0.5)
while (1) {
    int js1 = read_joystick1();  // range -100 to 100 (Joystick 1 Position)
    int js2 = read_joystick2();  // range -100 to 100 (Joystick 2 Position)

    int espnow_value;

    if (js1 >= 10 || js1 <= -10) {
        // JS1 outside dead zone → send JS1 directly (no invert)
        if (js1 >= 10) {
            led_display("U");          // forward
        } else {
            led_display("D");          // backward
        }
        espnow_value = js1;            // +10~+100 = forward, -10~-100 = backward
    } else if (js2 >= 10 || js2 <= -10) {
        // JS2 outside dead zone → send JS2 + 400
        if (js2 >= 10) {
            led_display("R");          // turn right
        } else {
            led_display("L");          // turn left
        }
        espnow_value = js2 + 400;      // 410~500 = right, 300~399 = left
    } else {
        // Both in dead zone → stop
        led_display("--");
        espnow_value = 999;
    }

    esp_now_send(peer_mac, (uint8_t*)&espnow_value, sizeof(int));
    vTaskDelay(pdMS_TO_TICKS(500));
}
```

---

## ส่วนที่ 6: ความแตกต่างระหว่าง KB1.3 (Rev 3.1) และ KB1.5G (Rev 3.1G)

> ⚠️ **CRITICAL:** Formula Kid Controller รองรับทั้ง KB1.3 และ KB1.5G
> **ทั้งสองรุ่นนี้ไม่มี KXTJ3 Accelerometer** — ห้าม init KXTJ3 สำหรับบอร์ดทั้งสอง

| Feature | KB1.3 (Rev 3.1) | KB1.5G (Rev 3.1G) |
|---------|----------------|-------------------|
| S1 (Formula Kid) | **GPIO36** | **GPIO36** |
| S2 (Formula Kid) | **GPIO39** | **GPIO39** |
| SW1 ปุ่มบนบอร์ด | GPIO16 | GPIO16 |
| **SW2 ปุ่มบนบอร์ด** | **GPIO14** | **GPIO14** |
| KXTJ3 Accelerometer | ❌ ไม่มี | ❌ ไม่มี |
| USB Connector | Micro-USB | Micro-USB |

> ⚠️ S1/S2 ของ Formula Kid Controller (GPIO36/39) ไม่เกี่ยวกับ SW1/SW2 ปุ่มบนบอร์ด (GPIO16/14) — เป็นคนละวงจรกัน

---

## ส่วนที่ 7: สรุปกฎ Quick Reference

### DO ✅

- ใช้ `gpio_get_level(GPIO_NUM_36)` และ `gpio_get_level(GPIO_NUM_39)` สำหรับ S1, S2
- Config GPIO36/39 ด้วย `GPIO_MODE_INPUT`, `GPIO_PULLUP_DISABLE`, `GPIO_PULLDOWN_DISABLE`, `GPIO_INTR_DISABLE`
- ใช้ polling loop + delay ไม่ใช่ interrupt
- ส่ง ESP-NOW ทุก 500ms
- ตรวจสอบ JS1 ก่อน JS2 ใน priority order (JS1 override JS2)
- Encode: JS1 ส่งตรงๆ (บวก/ลบตาม direction), JS2+400 สำหรับเลี้ยว, 999 สำหรับ stop
- แสดง LED: "U"=เดินหน้า, "D"=ถอยหลัง, "R"=ขวา, "L"=ซ้าย, "--"=หยุด

### DON'T ❌

- ❌ ห้ามใช้ `GPIO_PULLUP_ENABLE` บน GPIO36/39
- ❌ ห้ามตั้ง GPIO36/39 เป็น output
- ❌ ห้ามใช้ ISR interrupt บน GPIO36/39 เมื่อใช้ ESP-NOW
- ❌ ห้ามใช้ IoT WiFi (SSID/Password) พร้อมกับ ESP-NOW
- ❌ ห้ามสับสน S1/S2 (GPIO36/39) กับ SW1/SW2 ปุ่มบนบอร์ด (GPIO16/14)
- ❌ ห้าม init KXTJ3 Accelerometer สำหรับ KB1.3 / KB1.5G
- ❌ ห้ามใช้ค่า ESPNOW_VALUE ที่ชนกับช่วง JS1 (-100~100), JS2+400 (300~500), หรือ 999