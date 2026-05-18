# Formula Kid Controller + CAR — Plugin Rules & Hardware Reference
> **Plugin สำหรับ KidBrightIDE / KBIDE** · บอร์ด **KB1.3 (V1.5 Rev 3.1)** และ **KB1.5G (V1.5 Rev 3.1G)**
> ใช้โปรโตคอล **ESP-NOW** สื่อสารแบบ Unicast ระหว่าง Controller (บอร์ดถือ) และ Receiver (บอร์ดรถ)
> ยืนยันจากการทดสอบจริง 100% บนฮาร์ดแวร์จริง
> บอร์ดรับ: KidBright32 V1.4 + Formula Kid CAR rev 1.0
> บอร์ดส่ง: KidBright32 V1.5 Rev 3.1G + Formula Kid rev 1.1

---


> 🔬 **ข้อมูลนี้ยืนยันจากการทดสอบจริง 100% บนฮาร์ดแวร์จริง**
> บอร์ดรับ: KidBright32 V1.4 + Formula Kid CAR rev 1.0
> บอร์ดส่ง: KidBright32 V1.5 Rev 3.1G + Formula Kid rev 1.1

### 20.1 GPIO Pinout — Formula Kid CAR rev 1.0

| สัญญาณ | GPIO | หมายเหตุ |
|---|---|---|
| DRV_NSLEEP | GPIO_NUM_23 | ต้องตั้งเป็น HIGH เสมอ เพื่อ Enable DRV8833 |
| DRV_AIN1 | GPIO_NUM_18 | Motor A — phase 1 |
| DRV_AIN2 | GPIO_NUM_26 | Motor A — phase 2 |
| DRV_BIN1 | GPIO_NUM_19 | Motor B — phase 1 |
| DRV_BIN2 | GPIO_NUM_27 | Motor B — phase 2 |

> ⚠️ **AI RULE:** ห้ามใช้ GPIO25/14 สำหรับมอเตอร์บน Formula Kid CAR rev 1.0 — GPIO ที่ถูกต้องคือ 18/26/19/27 พร้อม nSLEEP=23

### 20.2 DRV8833 Motor Truth Table (ยืนยันจากฮาร์ดแวร์จริง)

> ⚠️ **CRITICAL:** DRV8833 บน Formula Kid CAR rev 1.0 มีพฤติกรรม **กลับขั้ว** จาก datasheet ทั่วไป
> ต้องใช้ค่าต่อไปนี้เท่านั้น ห้ามคำนวณเองจาก datasheet

```
motor_raw(a1, a2, b1, b2)  ←  ลำดับ: AIN1, AIN2, BIN1, BIN2

motor_raw(0,   0,   0,   0  ) = วิ่งหน้า (Forward)   ✅
motor_raw(255, 255, 255, 255) = ถอยหลัง (Backward)   ✅
motor_raw(0,   255, 0,   255) = หยุด (Stop/Brake)    ✅
motor_raw(D,   0,   0,   0  ) = เลี้ยวขวา 90°       ✅
motor_raw(0,   0,   D,   0  ) = เลี้ยวซ้าย 90°      ✅
(โดยที่ D = pct_to_duty ค่า 0-255)
```

### 20.3 Verified Receiver Code Reference

> ℹ️ **AI IMPLEMENTATION GUIDE:** โค้ดด้านล่างคือ implementation ที่ได้รับการทดสอบบนฮาร์ดแวร์จริงแล้วว่าทำงานได้สมบูรณ์ (100% Verified) เมื่อผู้ใช้ขอให้สร้างโค้ด Receiver สำหรับ Formula Kid CAR ให้ใช้โครงสร้างและ Logic ตามนี้เป็นมาตรฐานหลัก เพื่อให้ได้โค้ดที่ใช้งานได้ทันทีโดยไม่ติด error หรือปัญหาทางฮาร์ดแวร์
>
> ⚠️ **AI READING RULE:** เมื่อต้องสร้างโค้ด Receiver ให้อ่านโค้ดทั้งหมดในบล็อก `code block` นี้ **ทุก character** โดยไม่ตัดทอน แล้วใช้ verbatim — ห้ามสร้างโค้ดจากความจำหรือ semantic search เด็ดขาด

```c
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/queue.h"
#include "esp_log.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "driver/i2c.h"
#include "esp_wifi.h"
#include "esp_now.h"
#include "esp_netif.h"
#include "esp_event.h"
#include "nvs_flash.h"
#include "soc/soc.h"
#include "soc/rtc_cntl_reg.h"

#define TAG "CAR_RX"

// ── I2C / LED Matrix ──────────────────────────────────────
#define I2C_SDA_GPIO  GPIO_NUM_21
#define I2C_SCL_GPIO  GPIO_NUM_22
#define HT16K33_ADDR  0x70

// ── DRV8833 GPIO ──────────────────────────────────────────
#define DRV_NSLEEP    GPIO_NUM_23
#define DRV_AIN1      GPIO_NUM_18
#define DRV_AIN2      GPIO_NUM_26
#define DRV_BIN1      GPIO_NUM_19
#define DRV_BIN2      GPIO_NUM_27

// ── LEDC ──────────────────────────────────────────────────
#define LEDC_MODE     LEDC_LOW_SPEED_MODE
#define LEDC_RES      LEDC_TIMER_8_BIT
#define CH_AIN1       LEDC_CHANNEL_0
#define CH_AIN2       LEDC_CHANNEL_1
#define CH_BIN1       LEDC_CHANNEL_2
#define CH_BIN2       LEDC_CHANNEL_3

// ── ESP-NOW ───────────────────────────────────────────────
#define ESPNOW_CHANNEL  1

// ════════════════════════════════════════════════════════
//  Truth table ยืนยันจากการทดสอบจริง 100%:
//
//  motor_raw(0,   0,   0,   0  ) = วิ่งหน้า        ✅
//  motor_raw(255, 255, 255, 255) = ถอยหลัง         ✅
//  motor_raw(0,   255, 0,   255) = หยุด            ✅
//  motor_raw(D,   0,   0,   0  ) = เลี้ยวขวา 90°  ✅
//  motor_raw(0,   0,   D,   0  ) = เลี้ยวซ้าย 90° ✅
//
//  Protocol จาก TX:
//    999        = STOP
//    10 ~ 100   = เดินหน้า  (ค่า = ความเร็ว %)
//    -10 ~ -100 = ถอยหลัง  (ค่า = ความเร็ว %)
//    300 ~ 500  = เลี้ยว   (offset 400, <400=ซ้าย, >400=ขวา)
// ════════════════════════════════════════════════════════

static QueueHandle_t g_cmd_queue;

// ── LED Matrix Images ─────────────────────────────────────
static const uint8_t img_up[16]    = {0x00, 0x00, 0xFF, 0xFF, 0x01, 0x01, 0x01, 0x01,
                                       0x01, 0x01, 0x01, 0x01, 0xFF, 0xFF, 0x00, 0x00}; // U
static const uint8_t img_down[16]  = {0x00, 0x00, 0x00, 0xFF, 0xFF, 0x81, 0x81, 0x81,
                                       0x81, 0x81, 0x81, 0x7E, 0x3C, 0x00, 0x00, 0x00}; // D
static const uint8_t img_left[16]  = {0x00, 0x00, 0x00, 0xFF, 0xFF, 0x01, 0x01, 0x01,
                                       0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00}; // L
static const uint8_t img_right[16] = {0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x90, 0x90,
                                       0x98, 0x94, 0x62, 0x01, 0x00, 0x00, 0x00, 0x00}; // R
static const uint8_t img_stop[16]  = {0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00,
                                       0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00}; // --

// ── I2C ───────────────────────────────────────────────────
static void i2c_init(void) {
    i2c_config_t c = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = I2C_SDA_GPIO,
        .scl_io_num       = I2C_SCL_GPIO,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = 100000,
    };
    i2c_param_config(I2C_NUM_0, &c);
    i2c_driver_install(I2C_NUM_0, I2C_MODE_MASTER, 0, 0, 0);
}

static void matrix_cmd(uint8_t cmd) {
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR,
                               &cmd, 1, pdMS_TO_TICKS(50));
}

static void matrix_init(void) {
    matrix_cmd(0x21);
    matrix_cmd(0x81);
    matrix_cmd(0xEF);
}

static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0x00};
    for (int c = 0; c < 8; c++) {
        buf[1 + c*2] = cols[c];
        buf[2 + c*2] = cols[c + 8];
    }
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR,
                               buf, 17, pdMS_TO_TICKS(50));
}

// ── DRV8833 Init ──────────────────────────────────────────
static void drv8833_init(void) {
    gpio_config_t io = {
        .pin_bit_mask = (1ULL << DRV_NSLEEP),
        .mode         = GPIO_MODE_OUTPUT,
        .pull_up_en   = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_DISABLE,
    };
    gpio_config(&io);
    gpio_set_level(DRV_NSLEEP, 1);

    ledc_timer_config_t t = {
        .speed_mode      = LEDC_MODE,
        .duty_resolution = LEDC_RES,
        .timer_num       = LEDC_TIMER_0,
        .freq_hz         = 5000,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&t);

    gpio_num_t     pins[] = {DRV_AIN1, DRV_AIN2, DRV_BIN1, DRV_BIN2};
    ledc_channel_t chs[]  = {CH_AIN1,  CH_AIN2,  CH_BIN1,  CH_BIN2};
    for (int i = 0; i < 4; i++) {
        ledc_channel_config_t ch = {
            .gpio_num   = pins[i],
            .channel    = chs[i],
            .speed_mode = LEDC_MODE,
            .timer_sel  = LEDC_TIMER_0,
            .duty       = 0,
            .hpoint     = 0,
        };
        ledc_channel_config(&ch);
    }
    ESP_LOGI(TAG, "DRV8833 init OK");
}

// ── Motor Low-level ───────────────────────────────────────
static void motor_raw(uint32_t a1, uint32_t a2,
                      uint32_t b1, uint32_t b2) {
    ledc_set_duty(LEDC_MODE, CH_AIN1, a1);
    ledc_update_duty(LEDC_MODE, CH_AIN1);
    ledc_set_duty(LEDC_MODE, CH_AIN2, a2);
    ledc_update_duty(LEDC_MODE, CH_AIN2);
    ledc_set_duty(LEDC_MODE, CH_BIN1, b1);
    ledc_update_duty(LEDC_MODE, CH_BIN1);
    ledc_set_duty(LEDC_MODE, CH_BIN2, b2);
    ledc_update_duty(LEDC_MODE, CH_BIN2);
}

static uint32_t pct_to_duty(int pct) {
    if (pct < 0)   pct = -pct;
    if (pct > 100) pct = 100;
    return (uint32_t)(pct * 255 / 100);
}

// ── คำสั่งพื้นฐาน ─────────────────────────────────────────

static void cmd_stop(void) {
    motor_raw(0, 255, 0, 255);
}

static void cmd_forward(int pct) {
    uint32_t brake = 255 - pct_to_duty(pct);
    motor_raw(0, brake, 0, brake);
}

static void cmd_backward(int pct) {
    uint32_t d = pct_to_duty(pct);
    motor_raw(d, 255, d, 255);
}

static void cmd_turn_left(int pct) {
    uint32_t d = pct_to_duty(pct);
    motor_raw(0, 0, d, 0);
}

static void cmd_turn_right(int pct) {
    uint32_t d = pct_to_duty(pct);
    motor_raw(d, 0, 0, 0);
}

// ── ESP-NOW Callback ──────────────────────────────────────
static void recv_cb(const esp_now_recv_info_t *info,
                    const uint8_t *data, int len) {
    if (len != sizeof(int32_t)) return;
    int32_t val;
    memcpy(&val, data, sizeof(int32_t));
    xQueueOverwrite(g_cmd_queue, &val);
}

// ── WiFi + ESP-NOW Init ───────────────────────────────────
static void espnow_init(void) {
    esp_err_t r = nvs_flash_init();
    if (r == ESP_ERR_NVS_NO_FREE_PAGES ||
        r == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        nvs_flash_erase();
        nvs_flash_init();
    }
    esp_netif_init();
    esp_event_loop_create_default();
    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    esp_wifi_init(&cfg);
    esp_wifi_set_storage(WIFI_STORAGE_RAM);
    esp_wifi_set_mode(WIFI_MODE_STA);
    esp_wifi_start();
    esp_wifi_set_max_tx_power(40);
    esp_wifi_set_channel(ESPNOW_CHANNEL, WIFI_SECOND_CHAN_NONE);
    esp_now_init();
    esp_now_register_recv_cb(recv_cb);

    uint8_t mac[6];
    esp_wifi_get_mac(WIFI_IF_STA, mac);
    ESP_LOGI(TAG, "========================================");
    ESP_LOGI(TAG, "Receiver MAC: %02X:%02X:%02X:%02X:%02X:%02X",
             mac[0],mac[1],mac[2],mac[3],mac[4],mac[5]);
    ESP_LOGI(TAG, "========================================");
}

// ── app_main ──────────────────────────────────────────────
void app_main(void) {
    WRITE_PERI_REG(RTC_CNTL_BROWN_OUT_REG, 0);

    g_cmd_queue = xQueueCreate(1, sizeof(int32_t));

    i2c_init();
    matrix_init();
    matrix_draw(img_stop);

    drv8833_init();
    cmd_stop();
    ESP_LOGI(TAG, "Initial STOP");

    espnow_init();
    ESP_LOGI(TAG, "รอรับคำสั่ง...");

    const uint8_t *cur_img = img_stop;
    int32_t val = 999;

    while (1) {
        // รอรับคำสั่งโดยไม่มี timeout (portMAX_DELAY)
        xQueueReceive(g_cmd_queue, &val, portMAX_DELAY);

        const uint8_t *new_img = img_stop;

        if (val == 999) {
            cmd_stop();
            new_img = img_stop;
            ESP_LOGI(TAG, "STOP");

        } else if (val >= 10 && val <= 100) {
            cmd_forward((int)val);
            new_img = img_up;
            ESP_LOGI(TAG, "FORWARD %ld%%", (long)val);

        } else if (val >= -100 && val <= -10) {
            cmd_backward((int)(-val));
            new_img = img_down;
            ESP_LOGI(TAG, "BACKWARD %ld%%", (long)(-val));

        } else if (val >= 300 && val <= 500) {
            int32_t js = val - 400;
            if (js <= -10) {
                cmd_turn_left((int)(-js));
                new_img = img_left;
                ESP_LOGI(TAG, "LEFT %ld%%", (long)(-js));
            } else if (js >= 10) {
                cmd_turn_right((int)js);
                new_img = img_right;
                ESP_LOGI(TAG, "RIGHT %ld%%", (long)js);
            } else {
                cmd_stop();
                new_img = img_stop;
            }

        } else {
            cmd_stop();
            new_img = img_stop;
            ESP_LOGW(TAG, "Unknown val: %ld", (long)val);
        }

        if (new_img != cur_img) {
            matrix_draw(new_img);
            cur_img = new_img;
        }
    }
}
```


### 20.4 ESP-NOW Protocol — Formula Kid CAR

| ค่า (int32_t) | ความหมาย | Action |
|---|---|---|
| `999` | STOP ฉุกเฉิน | `cmd_stop()` |
| `10` ถึง `100` | เดินหน้า | `cmd_forward(val)` |
| `-10` ถึง `-100` | ถอยหลัง | `cmd_backward(-val)` |
| `300` ถึง `500` | เลี้ยว (offset 400) | decode: js=val-400 |
| → js `-10` ถึง `-100` | เลี้ยวซ้าย | `cmd_turn_left(-js)` |
| → js `10` ถึง `100` | เลี้ยวขวา | `cmd_turn_right(js)` |

### 20.5 กฎบังคับสำหรับ AI

1. **ห้าม** ใช้ `motor_raw(d, 0, d, 0)` สำหรับ Forward — เป็นผลลัพธ์ที่ผิดบนฮาร์ดแวร์นี้
2. **ต้องใช้** `cmd_forward(pct)` ด้วยสูตร `brake = 255 - pct_to_duty(pct)` เสมอ
3. **nSLEEP (GPIO23) ต้อง HIGH** ก่อนใช้งาน — ถ้าไม่ set DRV8833 จะไม่ทำงาน
4. **ใช้ FreeRTOS Queue** (`xQueueCreate(1, sizeof(int32_t))` + `xQueueOverwrite`) แทน `volatile bool` เพื่อความ thread-safe
5. **ปิด Brownout** `WRITE_PERI_REG(RTC_CNTL_BROWN_OUT_REG, 0)` เสมอ เพราะ WiFi+Motor ดึงกระแสสูง
6. **ลด WiFi TX Power** `esp_wifi_set_max_tx_power(40)` เพื่อลดการดึงกระแสสูงสุด

---

## ส่วนที่ 1: ฮาร์ดแวร์ Controller — สวิตช์ S1, S2 (KB1.3/KB1.5G)

### GPIO ของ S1 และ S2 (Formula Kid Controller)

> ⚠️ **CRITICAL — Formula Kid Controller ใช้ S1=GPIO36, S2=GPIO39 ไม่ใช่ SW1/SW2 ปุ่มบนบอร์ด**

| สวิตช์ | GPIO (ESP32) | อ้างอิง | ข้อจำกัด |
|--------|-------------|---------|----------|
| **S1** | **GPIO36 (VP)** | ADC1_CH0 | Input-only · ไม่มี internal pull-up/pull-down |
| **S2** | **GPIO39 (VN)** | ADC1_CH3 | Input-only · ไม่มี internal pull-up/pull-down |

| สถานะ | สัญญาณ | ค่า `gpio_get_level()` |
|-------|--------|----------------------|
| ปล่อยปุ่ม | HIGH | 1 |
| กดปุ่ม | LOW | 0 |

### กฎการใช้งาน S1, S2 (MANDATORY)

1. **Input-only**: GPIO36/39 ห้ามกำหนดเป็น output เด็ดขาด
2. **ห้าม pull-up ใน code**: บอร์ดมี external pull-up แล้ว ห้ามใช้ `GPIO_PULLUP_ENABLE`
3. **ห้ามใช้ interrupt**: เมื่อใช้ ESP-NOW ร่วมกัน ให้ใช้ **polling** เท่านั้น
4. **Active LOW**: กด = LOW (0), ปล่อย = HIGH (1)

```c
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
// Read: 0 = pressed, 1 = released
int s1 = gpio_get_level(GPIO_NUM_36);
int s2 = gpio_get_level(GPIO_NUM_39);
```

---

## ส่วนที่ 2: Joystick — RC Timing (ฝั่ง Controller)

> 🔑 **CRITICAL:** Joystick **ไม่ได้ใช้ ADC** — ใช้วงจร **RC Timing** ผ่าน GPIO คนละชุด

### GPIO Pins จริง (จาก Plugin generators.js)

| Joystick | แกน | Trigger GPIO (Output) | Capture GPIO (Input+ISR) |
|----------|-----|-----------------------|--------------------------|
| **JS1** | ขึ้น/ลง (Y) | **GPIO26** (OUT1) | **GPIO32** (IN1) |
| **JS2** | ซ้าย/ขวา (X) | **GPIO27** (OUT2) | **GPIO33** (IN2) |

> ⚠️ GPIO36/39 คือ **S1/S2 switches** เท่านั้น — ไม่เกี่ยวกับ Joystick

### ค่าคงที่ RC Timing

```c
#define R_SERIE        1000.0f
#define RC_FACTOR_5V   9.788075945f
#define CAP_TIMEOUT_US 500000    // 500ms
#define DISCHARGE_MS   10

// JS1: release=-3, cal_min=-100, cal_max=89
// JS2: release=-3, cal_min=-100, cal_max=90
#define JS1_DEAD_ZONE  10
#define JS2_DEAD_ZONE  20
```

### Dead Zone และการ Encode ค่าส่งผ่าน ESP-NOW

| สถานการณ์ | ESPNOW_VALUE | ความหมาย |
|-----------|-------------|---------|
| JS1 ≥ 10 | `10` ถึง `100` | เดินหน้า — แสดง "U" |
| JS1 ≤ -10 | `-100` ถึง `-10` | ถอยหลัง — แสดง "D" |
| JS2 ≥ 10 | `410` ถึง `500` (JS2+400) | เลี้ยวขวา — แสดง "R" |
| JS2 ≤ -10 | `300` ถึง `390` (JS2+400) | เลี้ยวซ้าย — แสดง "L" |
| ทั้งคู่ dead zone | `999` | หยุด — แสดง "--" |

> **Priority: JS1 > JS2 > Stop** — ถ้า JS1 เคลื่อน ค่า JS2 จะถูกละเว้น

---

## ส่วนที่ 3: ความแตกต่าง KB1.3 (Rev 3.1) vs KB1.5G (Rev 3.1G)

> ⚠️ **CRITICAL: ทั้งสองรุ่นนี้ไม่มี KXTJ3 Accelerometer — ห้าม init KXTJ3**

| Feature | KB1.3 (Rev 3.1) | KB1.5G (Rev 3.1G) |
|---------|----------------|-------------------|
| S1 (Formula Kid) | **GPIO36** | **GPIO36** |
| S2 (Formula Kid) | **GPIO39** | **GPIO39** |
| SW1 ปุ่มบนบอร์ด | GPIO16 | GPIO16 |
| **SW2 ปุ่มบนบอร์ด** | **GPIO14** | **GPIO14** |
| KXTJ3 Accelerometer | ❌ ไม่มี | ❌ ไม่มี |
| USB Connector | Micro-USB | Micro-USB |

> ⚠️ S1/S2 (GPIO36/39) ไม่เกี่ยวกับ SW1/SW2 ปุ่มบนบอร์ด (GPIO16/14) — เป็นคนละวงจรกัน

---

## ส่วนที่ 4: กฎ ESP-NOW (Controller + Receiver)

### กฎสำคัญ (MANDATORY)

1. **ห้ามใช้ IoT (WiFi) พร้อมกับ ESP-NOW**: SSID/Password ต้องว่างเปล่า
2. **ชนิดข้อมูล: `int32_t` เท่านั้น** — ห้ามใช้ `float` เด็ดขาด (ทำให้ decode ผิด: `-nan`, `0.00`)
3. **ส่งทุก 500ms** หรือเฉพาะเมื่อมีการเปลี่ยนแปลงค่า (Smart Send: delta > 5)
4. **MAC Address ต้อง hardcode** ให้ถูกต้องตรงกับบอร์ดรถ
5. **CMakeLists.txt**: ห้ามใส่ `esp_now` ใน `PRIV_REQUIRES` — ใช้แค่ `PRIV_REQUIRES driver esp_timer esp_wifi nvs_flash`

### ESP-NOW Callback Signature (ตาม ESP-IDF version)

```c
// ✅ ESP-IDF v5.4.x — CORRECT
static void espnow_send_cb(const uint8_t *mac_addr, esp_now_send_status_t status) {
    (void)mac_addr;
    if (status != ESP_NOW_SEND_SUCCESS) {
        ESP_LOGW(TAG, "ESP-NOW send failed");
    }
}

// ✅ ESP-IDF v5.5+ — CORRECT (Breaking Change)
static void espnow_send_cb(const wifi_tx_info_t *tx_info, esp_now_send_status_t status) {
    (void)tx_info;  // wifi_tx_info_t has NO dst_mac field — NEVER access tx_info->dst_mac
    if (status != ESP_NOW_SEND_SUCCESS) {
        ESP_LOGW(TAG, "ESP-NOW send failed");
    }
}

// ✅ Receive callback (ESP-IDF v5.x)
static void espnow_recv_cb(const esp_now_recv_info_t *recv_info, const uint8_t *data, int len) {
    // recv_info->src_addr, recv_info->des_addr
}
```

---

## ส่วนที่ 5: Verified Controller Code Patterns (ESP-IDF v5.4.3)

> ⚠️ **AI INSTRUCTION:** ก่อนเขียนโค้ด Controller Sender ทุกครั้ง ต้องใช้โครงสร้างด้านล่างเป็นแม่แบบ

### 1. RC Timing ISR — Queue Pattern

```c
typedef struct {
    int     gpio_num;
    int64_t duration;   // stop_time จาก esp_timer_get_time()
} rc_timing_event_t;

static QueueHandle_t s_rc_timing_queue;  // xQueueCreate(10, sizeof(rc_timing_event_t))

static IRAM_ATTR void rc_timing_isr_handler(void *arg) {
    int gpio_num = (int)arg;
    int64_t stop_time = esp_timer_get_time();
    rc_timing_event_t event = { .gpio_num = gpio_num, .duration = stop_time };
    xQueueSendFromISR(s_rc_timing_queue, &event, NULL);
}
```

### 2. Discharge + Measure Sequence

```c
// Step 1: Discharge capacitor
gpio_intr_disable(cap_gpio);
gpio_set_level(trig_gpio, 1);
esp_rom_delay_us(DISCHARGE_MS * 1000);  // ใช้ us เสมอ (10ms = 10000us)

// Step 2: Start charge + timestamp
gpio_set_level(trig_gpio, 0);
int64_t js_start_time = esp_timer_get_time();
gpio_intr_enable(cap_gpio);
```

### 3. Flush Queue ก่อนวัดแต่ละแกน (ป้องกัน stale event)

```c
// Flush stale events ก่อนวัด JS1
while (xQueueReceive(s_rc_timing_queue, &event, 0) == pdTRUE) { }
// ... trigger JS1 ...
if (xQueueReceive(s_rc_timing_queue, &event, pdMS_TO_TICKS(CAP_TIMEOUT_US / 1000)) == pdTRUE
    && event.gpio_num == JS1_CAP_GPIO) {
    js1_pos = calculate_joystick_position(event.duration - js1_start_time, -3, -100, 89);
}
// Flush อีกครั้งก่อนวัด JS2
while (xQueueReceive(s_rc_timing_queue, &event, 0) == pdTRUE) { }
```

### 4. Calibration Function

```c
int calculate_joystick_position(int64_t duration, int release, int min_cal, int max_cal) {
    float resistance = (float)duration * RC_FACTOR_5V - R_SERIE;
    int raw_pos = (int)(resistance * 200.0f / 10000.0f) - 100;
    int pos = raw_pos - release;
    if (pos < 0) pos = (int)((float)pos * 100.0f / (float)abs(min_cal - release));
    else         pos = (int)((float)pos * 100.0f / (float)abs(max_cal - release));
    if (pos >  100) pos =  100;
    if (pos < -100) pos = -100;
    return pos;
}
```

### 5. Priority Logic + Smart ESP-NOW Send

```c
// Priority: JS1 > JS2 > Stop
if      (js1_pos >= JS1_DEAD_ZONE)  { espnow_value = js1_pos;        img = img_up;    }
else if (js1_pos <= -JS1_DEAD_ZONE) { espnow_value = js1_pos;        img = img_down;  }
else if (js2_pos >= JS2_DEAD_ZONE)  { espnow_value = js2_pos + 400;  img = img_right; }
else if (js2_pos <= -JS2_DEAD_ZONE) { espnow_value = js2_pos + 400;  img = img_left;  }
else                                 { espnow_value = 999;            img = img_stop;  }

// Update LED เฉพาะเมื่อ pattern เปลี่ยน
int direction_changed = (img != prev_img);
if (direction_changed) { matrix_draw(img); prev_img = img; }

// Send ESP-NOW เฉพาะเมื่อ direction เปลี่ยน หรือค่าเปลี่ยนมากกว่า 5
int value_delta = abs((int)espnow_value - (int)prev_espnow_value);
int moving = (img != img_stop);
if (direction_changed || (moving && value_delta > 5)) {
    esp_now_send(s_broadcast_mac, (uint8_t *)&espnow_value, sizeof(espnow_value));
    prev_espnow_value = espnow_value;
}
```

### 6. GPIO Config — CAP (Input+ISR) และ TRIG (Output)

```c
// CAP pins: Input, no pull, rising edge ISR
gpio_config_t cap_conf = {
    .pin_bit_mask = (1ULL << JS1_CAP_GPIO) | (1ULL << JS2_CAP_GPIO),
    .mode = GPIO_MODE_INPUT,
    .pull_up_en = GPIO_PULLUP_DISABLE,
    .pull_down_en = GPIO_PULLDOWN_DISABLE,
    .intr_type = GPIO_INTR_POSEDGE,
};
gpio_config(&cap_conf);

// TRIG pins: Output
gpio_config_t trig_conf = {
    .pin_bit_mask = (1ULL << JS1_TRIG_GPIO) | (1ULL << JS2_TRIG_GPIO),
    .mode = GPIO_MODE_OUTPUT,
    .intr_type = GPIO_INTR_DISABLE,
};
gpio_config(&trig_conf);

gpio_install_isr_service(0);
gpio_isr_handler_add(JS1_CAP_GPIO, rc_timing_isr_handler, (void *)JS1_CAP_GPIO);
gpio_isr_handler_add(JS2_CAP_GPIO, rc_timing_isr_handler, (void *)JS2_CAP_GPIO);
```

---

## ส่วนที่ 6: LED Matrix — Verified Images (180° Rotated)

> **CRITICAL:** จอ LED Matrix บน Formula Kid หมุน 180° — `cols[0]` = ซ้ายจริง, `Bit 7` = บนจริง

```c
// ❌ ห้ามประกาศ img ที่ไม่ได้ใช้ — ESP-IDF v5.x -Werror=unused-const-variable=
// ประกาศเฉพาะที่โปรเจกต์ใช้จริงเท่านั้น
static const uint8_t img_up[16]    = {0x00,0x00,0xFF,0xFF,0x01,0x01,0x01,0x01,
                                       0x01,0x01,0x01,0x01,0xFF,0xFF,0x00,0x00}; // U
static const uint8_t img_down[16]  = {0x00,0x00,0x00,0xFF,0xFF,0x81,0x81,0x81,
                                       0x81,0x81,0x81,0x7E,0x3C,0x00,0x00,0x00}; // D
static const uint8_t img_left[16]  = {0x00,0x00,0x00,0xFF,0xFF,0x01,0x01,0x01,
                                       0x01,0x01,0x01,0x01,0x01,0x00,0x00,0x00}; // L
static const uint8_t img_right[16] = {0x00,0x00,0x00,0x00,0xFF,0xFF,0x90,0x90,
                                       0x98,0x94,0x62,0x01,0x00,0x00,0x00,0x00}; // R
static const uint8_t img_stop[16]  = {0x00,0x00,0x18,0x18,0x18,0x18,0x00,0x00,
                                       0x00,0x00,0x18,0x18,0x18,0x18,0x00,0x00}; // --

// matrix_draw ถูกต้อง — ห้ามสลับ panel
static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00;
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];
        buf[2 + (c * 2)] = cols[c + 8];
    }
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, buf, sizeof(buf), pdMS_TO_TICKS(100));
}
```

---

## ส่วนที่ 7: LED Matrix Text Scrolling (Verified)

> ห้ามเขียนโค้ดสลับซ้าย-ขวาเอง และ **ต้อง Reverse Bits** เสมอ (Bit 7 คือ Top, Bit 0 คือ Bottom)

```c
static const uint8_t font5x8[][5] = {
    {0x3E,0x51,0x49,0x45,0x3E}, // '0'
    {0x00,0x42,0x7F,0x40,0x00}, // '1'
    {0x42,0x61,0x51,0x49,0x46}, // '2'
    {0x21,0x41,0x45,0x4B,0x31}, // '3'
    {0x18,0x14,0x12,0x7F,0x10}, // '4'
    {0x27,0x45,0x45,0x45,0x39}, // '5'
    {0x3C,0x4A,0x49,0x49,0x30}, // '6'
    {0x01,0x71,0x09,0x05,0x03}, // '7'
    {0x36,0x49,0x49,0x49,0x36}, // '8'
    {0x06,0x49,0x49,0x29,0x1E}, // '9'
    {0x00,0x36,0x36,0x00,0x00}, // ':'
    {0x00,0x08,0x08,0x08,0x00}, // '-'
    {0x00,0x00,0x00,0x00,0x00}, // ' '
    {0x20,0x40,0x41,0x3F,0x01}, // 'J'
    {0x46,0x49,0x49,0x49,0x31}, // 'S'
};

static uint8_t reverse_bits(uint8_t b) {
    b = (b & 0xF0) >> 4 | (b & 0x0F) << 4;
    b = (b & 0xCC) >> 2 | (b & 0x33) << 2;
    b = (b & 0xAA) >> 1 | (b & 0x55) << 1;
    return b;
}

static uint8_t get_font_col(char c, int col) {
    int idx = 12; // default space
    if (c >= '0' && c <= '9') idx = c - '0';
    else if (c == ':') idx = 10;
    else if (c == '-') idx = 11;
    else if (c == 'J') idx = 13;
    else if (c == 'S') idx = 14;
    if (col < 5) return reverse_bits(font5x8[idx][col]);
    return 0x00;
}

static void matrix_scroll_text(const char *text) {
    int text_len = strlen(text);
    int total_width = text_len * 6;
    for (int x = 16; x > -total_width; x--) {
        uint8_t buf[17] = {0};
        buf[0] = 0x00;
        for (int col = 0; col < 16; col++) {
            int char_x = col - x;
            uint8_t pixel = 0x00;
            if (char_x >= 0 && char_x < total_width) {
                int char_idx = char_x / 6;
                int char_col = char_x % 6;
                if (char_idx < text_len) pixel = get_font_col(text[char_idx], char_col);
            }
            if (col < 8) buf[1 + col * 2] = pixel;
            else         buf[2 + (col - 8) * 2] = pixel;
        }
        i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, buf, sizeof(buf), pdMS_TO_TICKS(100));
        vTaskDelay(pdMS_TO_TICKS(60));
    }
}
```

---

## ส่วนที่ 8: Quick Reference — DO / DON'T

### DO ✅
- Config GPIO36/39 ด้วย `GPIO_MODE_INPUT`, `GPIO_PULLUP_DISABLE`, `GPIO_INTR_DISABLE`
- ตรวจสอบ JS1 ก่อน JS2 เสมอ (JS1 override JS2)
- Encode: JS1 ส่งตรงๆ, JS2+400, 999=stop
- CMakeLists: `PRIV_REQUIRES driver esp_timer esp_wifi nvs_flash`
- ประกาศ `static const uint8_t img_*[]` เฉพาะที่ใช้จริงในโค้ด

### DON'T ❌
- ❌ ห้ามใช้ `GPIO_PULLUP_ENABLE` บน GPIO36/39
- ❌ ห้ามตั้ง GPIO36/39 เป็น output
- ❌ ห้ามใช้ ISR interrupt บน GPIO36/39 เมื่อใช้ ESP-NOW
- ❌ ห้ามใช้ IoT WiFi (SSID/Password) พร้อมกับ ESP-NOW
- ❌ ห้ามสับสน S1/S2 (GPIO36/39) กับ SW1/SW2 ปุ่มบนบอร์ด (GPIO16/14)
- ❌ ห้าม init KXTJ3 Accelerometer สำหรับ KB1.3 / KB1.5G
- ❌ ห้ามใส่ `esp_now` ใน `PRIV_REQUIRES` ของ CMakeLists.txt
- ❌ ห้ามใช้ `float` รับค่า ESP-NOW — ใช้ `int32_t` เท่านั้น
- ❌ ห้ามประกาศ `static const` array ที่ไม่ได้ใช้ (`-Werror=unused-const-variable=`)