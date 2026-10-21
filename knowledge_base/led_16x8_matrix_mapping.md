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

### 6.2 — Convert Row-Major → Column-Major (Y-axis inversion)

> ⚠️ **CRITICAL HARDWARE QUIRK:** Matrix บอร์ดต่อสายกลับด้าน (Y-axis inverted)
> ต้องใช้ `(7 - row)` เสมอ — ห้ามใช้แค่ `row`

```c
static void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16]) {
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1 << (15 - col))) {
                // HARDWARE QUIRK: Y-axis inversion required -> (7 - row)
                out_cols[col] |= (1 << (7 - row));
            }
        }
    }
}
```

### 6.2.1 — Alternative: Column-Major 5×7 Font with `reverse_bits()` (HARDWARE-VERIFIED CORRECT ORIENTATION)

> ⚠️ **ใช้วิธีนี้เมื่อแสดงตัวอักษร/ข้อความ** จะได้ตัวอักษรหันหน้าถูกต้อง ไม่กลับซ้าย-ขวา ไม่กลับหัว
> วิธีนี้ไม่ต้องผ่าน `rows_to_columns_16x8()` เพราะ font data เป็น column-major อยู่แล้ว

```c
// reverse_bits() — จำเป็นเพื่อแก้ bit order ให้ตรงกับ hardware wiring
static uint8_t reverse_bits(uint8_t value) {
    value = (uint8_t)(((value & 0xF0U) >> 4) | ((value & 0x0FU) << 4));
    value = (uint8_t)(((value & 0xCCU) >> 2) | ((value & 0x33U) << 2));
    value = (uint8_t)(((value & 0xAAU) >> 1) | ((value & 0x55U) << 1));
    return value;
}

// ตัวอย่างประกาศ font — แต่ละตัวอักษรเป็น 5 bytes (5 columns × 7 rows)
// แต่ละ byte = 1 column, bit 0 = row 0 (top), bit 6 = row 6 (bottom)
// ค่า hex ออกแบบให้ตัวอักษรหันหน้าปกติ (อ่านจากซ้ายไปขวา)
static const uint8_t FONT_A[5] = {0x7E, 0x09, 0x09, 0x09, 0x7E};

// สร้าง cols[16] จาก font data — reverse_bits ทุก column
static void build_text_columns(uint8_t cols[16]) {
    memset(cols, 0, 16);
    // วาง font ที่ตำแหน่ง column ที่ต้องการใน cols[0..15]
    for (int c = 0; c < 5; c++) {
        cols[c] = reverse_bits(FONT_X[c]);  // reverse_bits สำคัญมาก!
    }
    // ตัวอักษรหลายตัว: เลื่อน column offset
    // cols[0..4] = ตัวแรก, cols[5..9] = ตัวที่สอง, cols[10..14] = ตัวที่สาม
}
```

**หลักการสำคัญ:**
- Font data เป็น **column-major**: แต่ละ byte = 1 column ของตัวอักษร (ไม่ใช่ row)
- **MUST** ใช้ `reverse_bits()` กับทุก column byte ก่อนใส่ `cols[]` เพื่อแก้ bit order ที่ hardware ต่อสายกลับ
- เขียนลง `cols[0..15]` ตรงๆ ได้เลย **ไม่ต้องผ่าน `rows_to_columns_16x8()`**
- ❌ ห้ามลืม `reverse_bits()` — ถ้าไม่ reverse จะได้ตัวอักษรกลับหัว
- ❌ ห้ามใช้ `rows_to_columns_16x8()` กับวิธีนี้ — เพราะ data เป็น column-major อยู่แล้ว

---

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
    for (int row = 0; row < 8; row++)
        for (int col = 0; col < 16; col++)
            if (row_data[row] & (1 << (15 - col)))
                out_cols[col] |= (1 << (7 - row));
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
| 2 | ข้าม `rows_to_columns_16x8` | Pixel แสดงผลกลับหัว/ผิดตำแหน่ง |
| 3 | ใช้ `(row)` แทน `(7 - row)` | ภาพกลับหัว |
| 4 | `display_pattern(DIGIT_x)` เดี่ยวๆ | ฝั่งขวาดับ |
| 5 | ประดิษฐ์ค่า hex pattern เอง | Pixel garbled |
| 6 | `ESP_ERROR_CHECK` บน data transfer | Board รีบูทถ้า I2C glitch |
| 7 | ใช้ `FONT_4x7` แทน `DIGIT_0–9` | ตัวเลขผิดตำแหน่ง (ไม่ผ่าน hardware test) |


---

## 12. VERIFIED HARDWARE RECIPES — KidBright32 iA (USER-CONFIRMED CORRECT ON REAL HARDWARE)

> Reuse these patterns EXACTLY. Change only what the user asks (values, text, timing).
> Remember the PATTERN, not a specific project name.
> If the user asks to create a NEW project → create the project workspace first, then write code.
> If the user does NOT ask for a new project → edit the CURRENT project directly (read_file → write_file).

**Shared helpers in every LED-matrix recipe (same as Section 6.2.1 / Method 3):**
`i2c_init_bus0()` (I2C_NUM_0, SDA=GPIO21, SCL=GPIO22, 100 kHz, pull-ups enabled, `clk_flags = 0`),
`matrix_command()`, `matrix_init()` (0x21 → delay 10 ms → 0x81 → 0xEF; failures only `ESP_LOGW`, never `ESP_ERROR_CHECK`),
`matrix_draw(cols[16])` (interleaved 17-byte buffer), `reverse_bits()`.
All I2C helpers use `i2c_master_write_to_device(..., pdMS_TO_TICKS(100))`.

### 12.1 Temperature (LM73) → 2 digits on LED Matrix every 2 s

- Matrix on `I2C_NUM_0` (21/22). LM73 on a **second bus** `I2C_NUM_1` (SDA=GPIO4, SCL=GPIO5, 100 kHz, address `0x4D`, register `0x00`).
- Write a separate `i2c_init_bus1()` and call `i2c_init_bus0(); i2c_init_bus1(); matrix_init();`.
- Read with ONE combined transaction (`i2c_master_write_read_device`), then convert `raw / 128.0f`:

```c
#define TEMP_I2C_PORT I2C_NUM_1
#define TEMP_I2C_SDA_GPIO GPIO_NUM_4
#define TEMP_I2C_SCL_GPIO GPIO_NUM_5
#define LM73_ADDR 0x4D
#define LM73_TEMP_REG 0x00

static esp_err_t lm73_read_temperature(float *temperature)
{
    uint8_t reg = LM73_TEMP_REG;
    uint8_t raw[2] = {0};
    esp_err_t ret = i2c_master_write_read_device(
        TEMP_I2C_PORT, LM73_ADDR, &reg, 1, raw, sizeof(raw), pdMS_TO_TICKS(100));
    if (ret != ESP_OK) return ret;
    int16_t raw_temp = (int16_t)(((uint16_t)raw[0] << 8) | (uint16_t)raw[1]);
    *temperature = (float)raw_temp / 128.0f;
    return ESP_OK;
}
```

- Round `(int)(t + (t >= 0.0f ? 0.5f : -0.5f))`, clamp 0..99, then `display_two_digits(v / 10, v % 10)`
  (DIGIT_0..9 `uint16_t[8]` arrays + `rows_to_columns_16x8()` + `matrix_draw()` — Section 7).
- Log `ESP_LOGI(TAG, "Temperature: %.2f C", (double)temperature);` on error `ESP_LOGW(TAG, "LM73 read failed: %s", esp_err_to_name(ret));`
- Loop delay `vTaskDelay(pdMS_TO_TICKS(2000))`.

### 12.2 LDR light level 0–100 % on LED Matrix (3 digits, `"%03d"`) every 2 s

- Includes: `"esp_adc/adc_oneshot.h"`, `"esp_rom_sys.h"`.
- `LDR_ADC_UNIT ADC_UNIT_1`, `LDR_ADC_CHANNEL ADC_CHANNEL_0`, `LDR_SAMPLE_COUNT 16`, `LDR_EMA_ALPHA 0.25f`, `LDR_ADC_MIN_VAL 100`, `LDR_ADC_MAX_VAL 900`.
- Init: `adc_oneshot_new_unit()` (`ADC_ULP_MODE_DISABLE`) + `adc_oneshot_config_channel()` with `.atten = ADC_ATTEN_DB_12`, `.bitwidth = ADC_BITWIDTH_DEFAULT`.
- Read: 16 × `adc_oneshot_read()` with `esp_rom_delay_us(500)` between samples → average → EMA (first sample seeds it) → `(int)(ema + 0.5f)`.
- Percent (inverted LDR), clamp **0..100**: `(int)(((float)(MAX - raw) / (float)(MAX - MIN)) * 100.0f)`.
- 4×7 font at column offsets 0, 5, 10 (each row = 4-bit nibble, bit 3 = leftmost):

```c
static const uint8_t FONT_4X7[10][7] = {
    {0x6,0x9,0x9,0x9,0x9,0x9,0x6}, {0x2,0x6,0x2,0x2,0x2,0x2,0x7},
    {0x6,0x9,0x1,0x2,0x4,0x8,0xF}, {0xE,0x1,0x1,0x6,0x1,0x1,0xE},
    {0x2,0x6,0xA,0xA,0xF,0x2,0x2}, {0xF,0x8,0xE,0x1,0x1,0x9,0x6},
    {0x6,0x8,0x8,0xE,0x9,0x9,0x6}, {0xF,0x1,0x2,0x2,0x4,0x4,0x4},
    {0x6,0x9,0x9,0x6,0x9,0x9,0x6}, {0x6,0x9,0x9,0x7,0x1,0x1,0x6},
};

static void draw_digit(int digit, int col_offset, uint8_t cols[16])
{
    for (int row = 0; row < 7; row++) {
        for (int col = 0; col < 4; col++) {
            if ((FONT_4X7[digit][row] & (1U << (3 - col))) != 0) {
                cols[col_offset + col] |= (uint8_t)(1U << (7 - row));
            }
        }
    }
}

static void display_light_percent(int percent)
{
    char text[16];
    uint8_t cols[16] = {0};
    if (percent < 0) percent = 0;
    if (percent > 100) percent = 100;
    snprintf(text, sizeof(text), "%03d", percent);
    draw_digit(text[0] - '0', 0, cols);
    draw_digit(text[1] - '0', 5, cols);
    draw_digit(text[2] - '0', 10, cols);
    matrix_draw(cols);
}
```

- Log `ESP_LOGI(TAG, "LDR raw ADC: %d, brightness: %d%%", raw_value, light_percent);`

### 12.3 Buzzer: 3 beeps (200 ms each), pause 1 s, repeat (GPIO13, LEDC)

- `BUZZER_GPIO GPIO_NUM_13`, `BUZZER_FREQUENCY_HZ 2000`, `BUZZER_DUTY 512`, `BEEP_DURATION_MS 200`, `BEEP_GAP_MS 200`, `CYCLE_PAUSE_MS 1000`.
- LEDC: `LEDC_LOW_SPEED_MODE`, `LEDC_TIMER_10_BIT`, `LEDC_TIMER_0`, `LEDC_AUTO_CLK`, channel `LEDC_CHANNEL_0`, `LEDC_INTR_DISABLE`, duty 0, hpoint 0.
- `buzzer_on()` = `ledc_set_duty(BUZZER_DUTY)` + `ledc_update_duty()`; `buzzer_off()` = duty 0 + update.

```c
while (1) {
    for (int beep = 0; beep < 3; beep++) {
        buzzer_on();
        vTaskDelay(pdMS_TO_TICKS(BEEP_DURATION_MS));
        buzzer_off();
        if (beep < 2) vTaskDelay(pdMS_TO_TICKS(BEEP_GAP_MS));   // no gap after the 3rd beep
    }
    vTaskDelay(pdMS_TO_TICKS(CYCLE_PAUSE_MS));
}
```

### 12.4 Button SW1 / SW2 → `ESP_LOGI` "SW1 Pressed" + scrolling text on LED Matrix

- SW1 = `GPIO_NUM_16`, SW2 = `GPIO_NUM_14` (Rev 3.1 / 3.1G / iA). Input, `GPIO_PULLUP_ENABLE`, `GPIO_INTR_DISABLE`; pressed = level 0.
- Edge detection with `bool was_pressed`; `POLL_INTERVAL_MS 20`, `DEBOUNCE_MS 30`; `SCROLL_DELAY_MS 60`.
- On new press: delay 30 ms, re-check level, then `ESP_LOGI(TAG, "SW1 Pressed")` and `matrix_scroll_text("SW1 PRESSED")` (SW2 → `"SW2 Pressed"` / `"SW2 PRESSED"`).
- Draw a blank `uint8_t blank[16] = {0}` once after `matrix_init()`.
- Needs BOTH fonts from 12.5 (digits + uppercase) because the text contains digits.

```c
void app_main(void)
{
    bool was_pressed = false;
    uint8_t blank[16] = {0};
    i2c_init_bus0();
    matrix_init();
    matrix_draw(blank);
    sw1_init();
    while (1) {
        bool is_pressed = gpio_get_level(SW1_GPIO) == 0;
        if (is_pressed && !was_pressed) {
            vTaskDelay(pdMS_TO_TICKS(DEBOUNCE_MS));
            if (gpio_get_level(SW1_GPIO) == 0) {
                ESP_LOGI(TAG, "SW1 Pressed");
                matrix_scroll_text("SW1 PRESSED");
            }
        }
        was_pressed = is_pressed;
        vTaskDelay(pdMS_TO_TICKS(POLL_INTERVAL_MS));
    }
}
```

### 12.5 Scroll text right → left (A–Z) — column-major 5×7 fonts

```c
static const uint8_t FONT_5X7_DIGITS[10][5] = {
    {0x3E,0x51,0x49,0x45,0x3E},{0x00,0x42,0x7F,0x40,0x00},{0x42,0x61,0x51,0x49,0x46},{0x21,0x41,0x45,0x4B,0x31},
    {0x18,0x14,0x12,0x7F,0x10},{0x27,0x45,0x45,0x45,0x39},{0x3C,0x4A,0x49,0x49,0x30},{0x01,0x71,0x09,0x05,0x03},
    {0x36,0x49,0x49,0x49,0x36},{0x06,0x49,0x49,0x29,0x1E},
};

static const uint8_t FONT_5X7_UPPERCASE[26][5] = {  // A..Z
    {0x7E,0x11,0x11,0x11,0x7E},{0x7F,0x49,0x49,0x49,0x36},{0x3E,0x41,0x41,0x41,0x22},{0x7F,0x41,0x41,0x22,0x1C},
    {0x7F,0x49,0x49,0x49,0x41},{0x7F,0x09,0x09,0x09,0x01},{0x3E,0x41,0x49,0x49,0x7A},{0x7F,0x08,0x08,0x08,0x7F},
    {0x00,0x41,0x7F,0x41,0x00},{0x20,0x40,0x41,0x3F,0x01},{0x7F,0x08,0x14,0x22,0x41},{0x7F,0x40,0x40,0x40,0x40},
    {0x7F,0x02,0x0C,0x02,0x7F},{0x7F,0x04,0x08,0x10,0x7F},{0x3E,0x41,0x41,0x41,0x3E},{0x7F,0x09,0x09,0x09,0x06},
    {0x3E,0x41,0x51,0x21,0x5E},{0x7F,0x09,0x19,0x29,0x46},{0x46,0x49,0x49,0x49,0x31},{0x01,0x01,0x7F,0x01,0x01},
    {0x3F,0x40,0x40,0x40,0x3F},{0x1F,0x20,0x40,0x20,0x1F},{0x3F,0x40,0x38,0x40,0x3F},{0x63,0x14,0x08,0x14,0x63},
    {0x07,0x08,0x70,0x08,0x07},{0x61,0x51,0x49,0x45,0x43},
};

static uint8_t get_font_column(char character, int column)
{
    if (column < 0 || column >= 5) return 0x00;                  // column 5 = 1-column gap
    if (character >= 'A' && character <= 'Z') return reverse_bits(FONT_5X7_UPPERCASE[character - 'A'][column]);
    if (character >= '0' && character <= '9') return reverse_bits(FONT_5X7_DIGITS[character - '0'][column]);
    return 0x00;                                                 // space / unknown
}

static void matrix_scroll_text(const char *text)
{
    int total_width = (int)strlen(text) * 6;                     // 5 cols + 1 gap per character
    for (int x = 16; x > -total_width; x--) {                    // starts off-screen RIGHT, moves LEFT
        uint8_t cols[16] = {0};
        for (int col = 0; col < 16; col++) {
            int text_x = col - x;
            if (text_x >= 0 && text_x < total_width) {
                cols[col] = get_font_column(text[text_x / 6], text_x % 6);
            }
        }
        matrix_draw(cols);
        vTaskDelay(pdMS_TO_TICKS(SCROLL_DELAY_MS));              // 60–80 ms
    }
}
```

**"Clear all code and show only A–Z":** keep only `i2c_init_bus0`, `matrix_command`, `matrix_init`, `matrix_draw`,
`reverse_bits`, the uppercase font, `get_font_column`, `matrix_scroll_text`.
`app_main` = `i2c_init_bus0(); matrix_init(); while (1) { matrix_scroll_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ"); }` with `SCROLL_DELAY_MS 80`.
Remove every unused `static const` (ESP-IDF treats it as a build error).

### 12.6 Scrolling text + melody on the buzzer at the same time (FreeRTOS task)

- The melody runs in its own task: `xTaskCreate(melody_task, "abc_melody", 2048, NULL, 5, NULL)` (check `pdPASS`, else `ESP_LOGE`), while `app_main` loops `matrix_scroll_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ")`.
- Buzzer: GPIO13, LEDC 10-bit, TIMER_0 / CHANNEL_0, `BUZZER_DUTY 512`, initial timer frequency `NOTE_C4_HZ`.
  Change pitch with `ledc_set_freq(LEDC_LOW_SPEED_MODE, LEDC_TIMER_0, hz)` then set/update duty. Stop = duty 0 + update.
- Notes: `C4 262, D4 294, E4 330, F4 349, G4 392, A4 440, B4 494, C5 523`, `NOTE_HOLD_HZ 0` (= "-" sustain).
  `NOTE_DURATION_MS 400` (1 beat), `NOTE_GAP_MS 40`, `MELODY_PAUSE_MS 500`.

**CORRECT MELODY** (32 beats, 1 note per beat, `-` = hold the previous note, `C5` = high C):

```
| C  D  E  G  | G  E  D  C  |
| E  F  G  A  | G  F  E  D  |
| C  E  G  C5 | B  A  G  E  |
| F  E  D  B  | C  -  -  -  |
```

```c
static const uint16_t ABC_MELODY_HZ[32] = {
    NOTE_C4_HZ, NOTE_D4_HZ, NOTE_E4_HZ, NOTE_G4_HZ,  NOTE_G4_HZ, NOTE_E4_HZ, NOTE_D4_HZ, NOTE_C4_HZ,
    NOTE_E4_HZ, NOTE_F4_HZ, NOTE_G4_HZ, NOTE_A4_HZ,  NOTE_G4_HZ, NOTE_F4_HZ, NOTE_E4_HZ, NOTE_D4_HZ,
    NOTE_C4_HZ, NOTE_E4_HZ, NOTE_G4_HZ, NOTE_C5_HZ,  NOTE_B4_HZ, NOTE_A4_HZ, NOTE_G4_HZ, NOTE_E4_HZ,
    NOTE_F4_HZ, NOTE_E4_HZ, NOTE_D4_HZ, NOTE_B4_HZ,  NOTE_C4_HZ, NOTE_HOLD_HZ, NOTE_HOLD_HZ, NOTE_HOLD_HZ,
};

static void play_abc_melody(void)
{
    for (int note = 0; note < 32; note++) {
        uint16_t frequency_hz = ABC_MELODY_HZ[note];
        if (frequency_hz == NOTE_HOLD_HZ) {                      // keep the previous tone sounding
            vTaskDelay(pdMS_TO_TICKS(NOTE_DURATION_MS));
            continue;
        }
        buzzer_start_tone(frequency_hz);
        vTaskDelay(pdMS_TO_TICKS(NOTE_DURATION_MS));
        bool next_is_hold = (note + 1 < 32) && (ABC_MELODY_HZ[note + 1] == NOTE_HOLD_HZ);
        if (!next_is_hold) {                                     // gap only between separate notes
            buzzer_stop_tone();
            vTaskDelay(pdMS_TO_TICKS(NOTE_GAP_MS));
        }
    }
    buzzer_stop_tone();
}
```

- NEVER invent notes or reorder them. A `-` lengthens the previous note (no gap, tone keeps playing); it is NOT a silent rest.

### 12.7 Flashing tips (Windows)

- `PermissionError(13, 'Access is denied.')` on COM port while flashing = the serial monitor is still connected. Disconnect the serial monitor, then Build & Flash again.
- Do not run `idf.py set-target`, `fullclean` or delete `build/` between normal edits — incremental builds only recompile the changed file.
