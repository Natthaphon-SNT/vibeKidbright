# KidBright32 iA — LED Matrix Display Reference
> ส่วนที่แยกออกมาจาก ESP-IDF Developer Reference
> ครอบคลุม: HT16K33 Driver · 16×8 Matrix · Two-Digit Display · Pattern Table

---

## 1. ข้อมูลพื้นฐาน LED Dot Matrix (16×8)

| Property | Detail |
|---|---|
| Driver IC | HT16K33 (Single Chip) |
| I2C Address | `0x70` |
| Display resolution | 16 columns × 8 rows |
| I2C Bus | `I2C_NUM_0` (SDA=GPIO21, SCL=GPIO22) |

---

## 2. HT16K33 Register Map

| Command | Value | Description |
|---|---|---|
| Oscillator ON | `0x21` | Turn on system oscillator |
| Display ON | `0x81` | Display ON, no blink |
| Brightness MAX | `0xEF` | Maximum brightness (16/16 duty) |
| RAM Start | `0x00` | Start address for display RAM write |

---

## 3. HT16K33 Display RAM Layout (INTERLEAVED MAPPING)

The 16×8 LED matrix on the KidBright32 iA is wired in an **interleaved** fashion to the single HT16K33 chip.

- **Left 8×8 Matrix (Columns 0–7):** Mapped to **EVEN** RAM addresses (0x00, 0x02, 0x04…)
- **Right 8×8 Matrix (Columns 8–15):** Mapped to **ODD** RAM addresses (0x01, 0x03, 0x05…)

**Buffer layout for a full 16×8 frame (17 bytes total: 1 address + 16 data):**
```
buffer[0]  = 0x00 (RAM start address)
buffer[1]  = column_0_rows  (Left Matrix Col 0)
buffer[2]  = column_8_rows  (Right Matrix Col 0)
buffer[3]  = column_1_rows  (Left Matrix Col 1)
buffer[4]  = column_9_rows  (Right Matrix Col 1)
...
buffer[15] = column_7_rows  (Left Matrix Col 7)
buffer[16] = column_15_rows (Right Matrix Col 7)
```

---

## 4. ⚠️ Anti-Pattern — Init ผิดทำ Display ดับ (BLANK DISPLAY)

> **CRITICAL:** คำสั่ง HT16K33 แต่ละตัว (`0x21`, `0x81`, `0xEF`) ต้องส่งเป็น **I2C transaction แยกกัน** ทีละ 1 byte เท่านั้น ถ้าส่งรวมกันใน write เดียว chip จะเข้าใจผิดและ display จะดับสนิท

**❌ WRONG — Display stays completely blank:**
```c
// ANTI-PATTERN: DO NOT DO THIS!
uint8_t init_cmds[] = {0x21, 0x81, 0xEF};
i2c_master_write_to_device(I2C_NUM_0, 0x70, init_cmds, sizeof(init_cmds), pdMS_TO_TICKS(100));
```

**✅ CORRECT — Each command as a separate I2C transaction:**
```c
uint8_t cmd;
cmd = 0x21; // Oscillator ON
i2c_master_write_to_device(I2C_NUM_0, 0x70, &cmd, 1, pdMS_TO_TICKS(100));
cmd = 0x81; // Display ON
i2c_master_write_to_device(I2C_NUM_0, 0x70, &cmd, 1, pdMS_TO_TICKS(100));
cmd = 0xEF; // Max Brightness
i2c_master_write_to_device(I2C_NUM_0, 0x70, &cmd, 1, pdMS_TO_TICKS(100));
```

---

## 5. ⚠️ I2C Error Handling — อย่าใช้ ESP_ERROR_CHECK กับ Data Transfers

**❌ WRONG — board รีบูทถ้า I2C glitch:**
```c
ESP_ERROR_CHECK(i2c_master_write_to_device(I2C_NUM_0, 0x70, buf, sizeof(buf), pdMS_TO_TICKS(100)));
```

**✅ CORRECT — graceful error handling:**
```c
esp_err_t ret = i2c_master_write_to_device(I2C_NUM_0, 0x70, buf, sizeof(buf), pdMS_TO_TICKS(100));
if (ret != ESP_OK) {
    ESP_LOGE(TAG, "I2C write failed: %s", esp_err_to_name(ret));
}
```

---

## 6. Core Functions

### 6.1 — Init I2C + HT16K33

```c
#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

#define I2C_MASTER_NUM    I2C_NUM_0
#define I2C_MASTER_SDA_IO GPIO_NUM_21
#define I2C_MASTER_SCL_IO GPIO_NUM_22
#define I2C_MASTER_FREQ   100000  // ⚠️ Use 100 kHz (safe default)
#define HT16K33_ADDR      0x70

static void i2c_and_matrix_init(void) {
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C_MASTER_SDA_IO,
        .scl_io_num = I2C_MASTER_SCL_IO,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master = {
            .clk_speed = I2C_MASTER_FREQ,
        },
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_MASTER_NUM, &conf));
    ESP_ERROR_CHECK(i2c_driver_install(I2C_MASTER_NUM, conf.mode, 0, 0, 0));

    // HT16K33 init — EACH command MUST be a SEPARATE 1-byte I2C write
    uint8_t cmd;
    cmd = 0x21; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));
    cmd = 0x81; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));
    cmd = 0xEF; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));
}
```

### 6.2 — Convert Row-Major → Column-Major

```c
static void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                out_cols[col] |= (1 << (7 - row));
            }
        }
    }
}
```

### 6.3 — Draw Frame (matrix_draw)

```c
static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00; // RAM start address
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];       // Left half (Even addresses)
        buf[2 + (c * 2)] = cols[c + 8];   // Right half (Odd addresses)
    }
    esp_err_t ret = i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, buf, sizeof(buf), pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE("MATRIX", "I2C write failed: %s", esp_err_to_name(ret));
    }
}
```

### 6.4 — Helper: Display Pattern (รวม convert + draw)

```c
static void display_pattern(const uint16_t pattern[8]) {
    uint8_t cols[16];
    rows_to_columns_16x8(pattern, cols);
    matrix_draw(cols);
}
```

---

## 7. ⚠️ TWO-DIGIT DISPLAY — เทคนิคบังคับสำหรับตัวเลข 2 หลัก

> **CRITICAL:** DIGIT patterns (DIGIT_0–DIGIT_9) มาตรฐานจะส่องเฉพาะ **columns 3–7** (ฝั่ง LEFT เท่านั้น)
> ถ้าแสดง DIGIT_x เดี่ยวๆ ฝั่งขวาจะ **ดับสนิท**
> ต้องใช้ `display_two_digits()` เสมอเมื่อแสดงตัวเลข 2 หลัก

**❌ WRONG — ฝั่งขวาดับ (common AI mistake):**
```c
display_pattern(DIGITS[tens]);   // only cols 3–7 light up, right is dark
display_pattern(DIGITS[units]);  // same problem
```

**✅ CORRECT — ทั้งสองฝั่งติดพร้อมกัน:**
```c
display_two_digits(tens, units); // tens on LEFT cols 3-7, units on RIGHT cols 11-15
```

**หลักการทำงาน:** DIGIT patterns ใช้ bit positions 12–8 (→ cols 3–7, left panel)
Shift right 8 bit → positions 4–0 (→ cols 11–15, right panel)
OR สองค่า → frame เดียวที่ติดทั้งสองฝั่ง

### ✅ VERIFIED Functions (copy-paste ready):

```c
// Lookup table — declare globally after DIGIT_0..DIGIT_9 definitions
static const uint16_t *DIGITS[10] = {
    DIGIT_0, DIGIT_1, DIGIT_2, DIGIT_3, DIGIT_4,
    DIGIT_5, DIGIT_6, DIGIT_7, DIGIT_8, DIGIT_9
};

// Display tens on LEFT panel, units on RIGHT panel — full 16x8 display
void display_two_digits(int tens, int units) {
    if (tens  < 0) tens  = 0; if (tens  > 9) tens  = 9;
    if (units < 0) units = 0; if (units > 9) units = 9;
    uint16_t combined[8];
    for (int i = 0; i < 8; i++) {
        combined[i] = DIGITS[tens][i] | (DIGITS[units][i] >> 8);
    }
    uint8_t cols[16];
    rows_to_columns_16x8(combined, cols);
    matrix_draw(cols);
}

// Convenience function: float temperature → 2-digit display
void display_temperature_on_matrix(float temperature_c) {
    int t = (int)temperature_c;
    if (t < 0)  t = 0;
    if (t > 99) t = 99;
    display_two_digits((t / 10) % 10, t % 10);
}

// Convenience function: integer 0-99 → 2-digit display
void display_number(int value) {
    if (value < 0)  value = 0;
    if (value > 99) value = 99;
    display_two_digits((value / 10) % 10, value % 10);
}
```

**Usage ใน app_main:**
```c
while (1) {
    if (lm73_read_temp(&temperature) == ESP_OK) {
        ESP_LOGI(TAG, "Temperature: %.2f C", temperature);
        display_temperature_on_matrix(temperature); // shows e.g. "28" on full display
    }
    vTaskDelay(pdMS_TO_TICKS(2000));
}
```

---

## 8. Verified Patterns (ห้ามประดิษฐ์ค่า hex เอง!)

> ⚠️ **CRITICAL:** ห้าม invent ค่า `uint16_t` hex สำหรับ digit/icon เองเด็ดขาด
> ค่าที่ AI คิดเองจะแสดงผล garbled บน hardware เสมอ
> ใช้เฉพาะ verified patterns ด้านล่างเท่านั้น

**Pattern format:** แต่ละ `uint16_t` = 1 แถว (บนลงล่าง)
Bit 15 = pixel ซ้ายสุด, Bit 0 = pixel ขวาสุด

```c
// --- Icons (verified) ---
const uint16_t PATTERN_HEART[8] = {
    0x0000, 0x0660, 0x0FF0, 0x1FF8, 0x0FF0, 0x07E0, 0x03C0, 0x0180
};
const uint16_t PATTERN_SMILEY[8] = {
    0x0000, 0x0C30, 0x0C30, 0x0000, 0x0000, 0x1008, 0x07E0, 0x0000
};
const uint16_t PATTERN_CHECK[8] = {
    0x0000, 0x0018, 0x0030, 0x0060, 0x1CC0, 0x0F80, 0x0700, 0x0200
};
const uint16_t PATTERN_CROSS[8] = {
    0x0000, 0x1818, 0x0C30, 0x0660, 0x03C0, 0x0660, 0x0C30, 0x1818
};

// --- Digits 0–9 (verified hardware-tested, 5-pixel wide, left-panel positioned) ---
// Each DIGIT_x pattern occupies bit positions 12–8 on the left half (cols 3–7)
// To display on RIGHT panel: shift right 8 bits → bit positions 4–0 (cols 11–15)
const uint16_t DIGIT_0[8] = {0x0E00,0x1100,0x1100,0x1100,0x1100,0x1100,0x1100,0x0E00};
const uint16_t DIGIT_1[8] = {0x0200,0x0600,0x0A00,0x0200,0x0200,0x0200,0x0200,0x1F00};
const uint16_t DIGIT_2[8] = {0x0E00,0x1100,0x0100,0x0200,0x0400,0x0800,0x1000,0x1F00};
const uint16_t DIGIT_3[8] = {0x0E00,0x1100,0x0100,0x0600,0x0100,0x0100,0x1100,0x0E00};
const uint16_t DIGIT_4[8] = {0x0200,0x0600,0x0A00,0x1200,0x1F00,0x0200,0x0200,0x0200};
const uint16_t DIGIT_5[8] = {0x1F00,0x1000,0x1E00,0x0100,0x0100,0x0100,0x1100,0x0E00};
const uint16_t DIGIT_6[8] = {0x0E00,0x1100,0x1000,0x1E00,0x1100,0x1100,0x1100,0x0E00};
const uint16_t DIGIT_7[8] = {0x1F00,0x0100,0x0200,0x0400,0x0400,0x0400,0x0400,0x0400};
const uint16_t DIGIT_8[8] = {0x0E00,0x1100,0x1100,0x0E00,0x1100,0x1100,0x1100,0x0E00};
const uint16_t DIGIT_9[8] = {0x0E00,0x1100,0x1100,0x0F00,0x0100,0x0100,0x1100,0x0E00};

// Helper: get digit pattern by integer value
const uint16_t* get_digit_pattern(int digit) {
    static const uint16_t* digits[10] = {
        DIGIT_0, DIGIT_1, DIGIT_2, DIGIT_3, DIGIT_4,
        DIGIT_5, DIGIT_6, DIGIT_7, DIGIT_8, DIGIT_9
    };
    if (digit < 0 || digit > 9) return DIGIT_0;
    return digits[digit];
}
```

---

## 9. Complete Display Pipeline (3-Step Mandatory)

> **AI INSTRUCTION:** ต้องทำครบ 3 ขั้นตอนเสมอ ห้ามข้าม `rows_to_columns_16x8`

```c
// Step 1: Define pattern in row-major format
const uint16_t PATTERN_HEART[8] = {
    0x0000, 0x0660, 0x0FF0, 0x1FF8, 0x0FF0, 0x07E0, 0x03C0, 0x0180
};

// Step 2 & 3: Convert and draw
void show_heart(void) {
    uint8_t cols[16];
    rows_to_columns_16x8(PATTERN_HEART, cols); // Step 2: convert (applies Y-inversion)
    matrix_draw(cols);                          // Step 3: send to HT16K33 over I2C
}
```

---

## 10. Complete Minimal Working Program (COPY-PASTE READY)

```c
#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

static const char *TAG = "MATRIX_DEMO";

#define I2C_MASTER_NUM    I2C_NUM_0
#define I2C_MASTER_SDA_IO GPIO_NUM_21
#define I2C_MASTER_SCL_IO GPIO_NUM_22
#define I2C_MASTER_FREQ   100000
#define HT16K33_ADDR      0x70

// --- Verified Patterns ---
static const uint16_t PATTERN_HEART[8] = {
    0x0000, 0x0660, 0x0FF0, 0x1FF8, 0x0FF0, 0x07E0, 0x03C0, 0x0180
};

// --- Verified Digits (hardware-tested) ---
static const uint16_t DIGIT_0[8] = {0x0E00,0x1100,0x1100,0x1100,0x1100,0x1100,0x1100,0x0E00};
static const uint16_t DIGIT_1[8] = {0x0200,0x0600,0x0A00,0x0200,0x0200,0x0200,0x0200,0x1F00};
static const uint16_t DIGIT_2[8] = {0x0E00,0x1100,0x0100,0x0200,0x0400,0x0800,0x1000,0x1F00};
static const uint16_t DIGIT_3[8] = {0x0E00,0x1100,0x0100,0x0600,0x0100,0x0100,0x1100,0x0E00};
static const uint16_t DIGIT_4[8] = {0x0200,0x0600,0x0A00,0x1200,0x1F00,0x0200,0x0200,0x0200};
static const uint16_t DIGIT_5[8] = {0x1F00,0x1000,0x1E00,0x0100,0x0100,0x0100,0x1100,0x0E00};
static const uint16_t DIGIT_6[8] = {0x0E00,0x1100,0x1000,0x1E00,0x1100,0x1100,0x1100,0x0E00};
static const uint16_t DIGIT_7[8] = {0x1F00,0x0100,0x0200,0x0400,0x0400,0x0400,0x0400,0x0400};
static const uint16_t DIGIT_8[8] = {0x0E00,0x1100,0x1100,0x0E00,0x1100,0x1100,0x1100,0x0E00};
static const uint16_t DIGIT_9[8] = {0x0E00,0x1100,0x1100,0x0F00,0x0100,0x0100,0x1100,0x0E00};

static const uint16_t *DIGITS[10] = {
    DIGIT_0, DIGIT_1, DIGIT_2, DIGIT_3, DIGIT_4,
    DIGIT_5, DIGIT_6, DIGIT_7, DIGIT_8, DIGIT_9
};

// --- Init ---
static void i2c_and_matrix_init(void) {
    i2c_config_t conf = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C_MASTER_SDA_IO,
        .scl_io_num = I2C_MASTER_SCL_IO,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master = { .clk_speed = I2C_MASTER_FREQ },
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_MASTER_NUM, &conf));
    ESP_ERROR_CHECK(i2c_driver_install(I2C_MASTER_NUM, conf.mode, 0, 0, 0));

    uint8_t cmd;
    cmd = 0x21; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));
    cmd = 0x81; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));
    cmd = 0xEF; i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));
}

// --- Convert & Draw ---
static void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                out_cols[col] |= (1 << (7 - row));
            }
        }
    }
}

static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00;
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];
        buf[2 + (c * 2)] = cols[c + 8];
    }
    esp_err_t ret = i2c_master_write_to_device(I2C_MASTER_NUM, HT16K33_ADDR, buf, sizeof(buf), pdMS_TO_TICKS(100));
    if (ret != ESP_OK) ESP_LOGE(TAG, "I2C write failed: %s", esp_err_to_name(ret));
}

static void display_pattern(const uint16_t pattern[8]) {
    uint8_t cols[16];
    rows_to_columns_16x8(pattern, cols);
    matrix_draw(cols);
}

// --- Two-Digit Display ---
static void display_two_digits(int tens, int units) {
    if (tens  < 0) tens  = 0; if (tens  > 9) tens  = 9;
    if (units < 0) units = 0; if (units > 9) units = 9;
    uint16_t combined[8];
    for (int i = 0; i < 8; i++)
        combined[i] = DIGITS[tens][i] | (DIGITS[units][i] >> 8);
    uint8_t cols[16];
    rows_to_columns_16x8(combined, cols);
    matrix_draw(cols);
}

static void display_number(int value) {
    if (value < 0)  value = 0;
    if (value > 99) value = 99;
    display_two_digits((value / 10) % 10, value % 10);
}

void app_main(void) {
    i2c_and_matrix_init();

    // Show heart icon
    display_pattern(PATTERN_HEART);
    vTaskDelay(pdMS_TO_TICKS(2000));

    // Show two-digit number: "42"
    display_two_digits(4, 2);
    vTaskDelay(pdMS_TO_TICKS(2000));

    // Show number 28
    display_number(28);

    ESP_LOGI(TAG, "Display ready!");
    while (1) {
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}
```

---

## 11. สรุป Pitfalls ที่ต้องระวัง

| # | Anti-Pattern | ผลที่เกิด |
|---|---|---|
| 1 | ส่ง init commands รวมใน write เดียว | Display ดับสนิท |
| 2 | ข้าม `rows_to_columns_16x8` | Pixel แสดงผลผิดตำแหน่ง |
| 4 | `display_pattern(DIGIT_x)` เดี่ยวๆ | ฝั่งขวาดับ |
| 5 | ประดิษฐ์ค่า hex pattern เอง | Pixel garbled |
| 6 | `ESP_ERROR_CHECK` บน data transfer | Board รีบูทถ้า I2C glitch |
| 7 | ใช้ `FONT_4x7` แทน `DIGIT_0–9` | ตัวเลขผิดตำแหน่ง (ไม่ผ่าน hardware test) |
