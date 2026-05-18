# OLED SSD1306 — ESP-IDF LCD Panel API (ESP-IDF v5.x)
> **Hardware-verified:** ทดสอบบน ESP32 (KidBright32 / โมดูล ESP32 ทั่วไป) — May 2026
> ใช้ I2C 0.96" SSD1306 OLED 128×64 พิกเซล

---

## ⚠️ ข้อควรระวังบน KidBright

| Bus | Pin | ใช้โดย (On-board) | ใช้ OLED ได้? |
|-----|-----|-------------------|--------------|
| I2C_NUM_0 | SDA=GPIO21, SCL=GPIO22 | HT16K33 LED Matrix (0x70) | ⚠️ ระวัง conflict — ถาม user ก่อน |
| I2C_NUM_1 | SDA=GPIO4, SCL=GPIO5 | LM73 (0x4D) + RTC (0x6F) | ⚠️ ระวัง conflict — ถาม user ก่อน |
| GPIO อิสระ | GPIO อื่นๆ | — | ✅ แนะนำ (ต้องต่อ pull-up 4.7kΩ) |

> **กฎ:** ถ้าโปรเจกต์ใช้ HT16K33 อยู่แล้ว ให้ใช้ GPIO อิสระสำหรับ OLED (เช่น GPIO25/26 ถ้าไม่ใช้ OUT) หรือถาม user ก่อนเสมอ

---

## ❌ BANNED — วิธีที่ห้ามใช้

```c
// ❌ BANNED: Legacy direct I2C command — ส่ง SSD1306 init sequence มือ
// ❌ BANNED: ผสม driver/i2c.h legacy กับ driver/i2c_master.h ใน project เดียว
// ❌ BANNED: เรียก esp_lcd_new_panel_ssd1306() ก่อน esp_lcd_new_panel_io_i2c()
// ❌ BANNED: ใช้ i2c_driver_install() ถ้าใช้ LCD Panel API แล้ว
```

---

## ✅ Correct API — ESP-IDF v5.x LCD Panel (New Driver)

### Required Headers
```c
#include <stdio.h>
#include <stdlib.h>
#include "esp_log.h"
#include "driver/i2c_master.h"       // ✅ NEW I2C driver (ESP-IDF v5.x)
#include "esp_lcd_panel_io.h"         // ✅
#include "esp_lcd_panel_ops.h"        // ✅
#include "esp_lcd_panel_vendor.h"     // ✅ for esp_lcd_new_panel_ssd1306
```

### CMakeLists.txt
```cmake
idf_component_register(
    SRCS "main.c"
    INCLUDE_DIRS "."
    REQUIRES driver esp_lcd       # ← ต้องเพิ่ม esp_lcd
)
```

---

## การเริ่มต้น (Initialization) — ลำดับขั้นตอนสำคัญ

```c
static const char *TAG = "OLED";

#define OLED_SCL_IO    22    // ปรับตาม hardware ที่ใช้
#define OLED_SDA_IO    21
#define OLED_I2C_PORT  I2C_NUM_0
#define OLED_I2C_ADDR  0x3C  // บางรุ่นใช้ 0x3D (SA0=HIGH)

void app_main(void)
{
    // ── STEP 1: สร้าง I2C Master Bus ──────────────────────────────────────
    ESP_LOGI(TAG, "Initialize I2C bus");
    i2c_master_bus_config_t i2c_bus_config = {
        .i2c_port = OLED_I2C_PORT,
        .sda_io_num = OLED_SDA_IO,
        .scl_io_num = OLED_SCL_IO,
        .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7,
        .flags.enable_internal_pullup = true,
    };
    i2c_master_bus_handle_t bus_handle;
    ESP_ERROR_CHECK(i2c_new_master_bus(&i2c_bus_config, &bus_handle));

    // ── STEP 2: สร้าง Panel IO บน bus ─────────────────────────────────────
    ESP_LOGI(TAG, "Install panel IO");
    esp_lcd_panel_io_handle_t io_handle = NULL;
    esp_lcd_panel_io_i2c_config_t io_config = {
        .dev_addr = OLED_I2C_ADDR,
        .control_phase_bytes = 1,
        .lcd_cmd_bits = 8,
        .lcd_param_bits = 8,
        .dc_bit_offset = 6,
        .scl_speed_hz = 400 * 1000,    // 400kHz Fast Mode
    };
    ESP_ERROR_CHECK(esp_lcd_new_panel_io_i2c(bus_handle, &io_config, &io_handle));

    // ── STEP 3: สร้าง SSD1306 Panel Handle ────────────────────────────────
    // ⚠️ ต้องมีขั้นตอนนี้เสมอก่อนสั่งการจอ!
    ESP_LOGI(TAG, "Install SSD1306 panel driver");
    esp_lcd_panel_handle_t panel_handle = NULL;
    esp_lcd_panel_dev_config_t panel_config = {
        .bits_per_pixel = 1,
        .reset_gpio_num = -1,           // ไม่ใช้ขา Reset
    };
    ESP_ERROR_CHECK(esp_lcd_new_panel_ssd1306(io_handle, &panel_config, &panel_handle));

    // ── STEP 4: Reset → Init → Display ON ────────────────────────────────
    ESP_LOGI(TAG, "Initialize display");
    ESP_ERROR_CHECK(esp_lcd_panel_reset(panel_handle));
    ESP_ERROR_CHECK(esp_lcd_panel_init(panel_handle));
    ESP_ERROR_CHECK(esp_lcd_panel_disp_on_off(panel_handle, true));

    // ── (ทางเลือก) Invert สี ─────────────────────────────────────────────
    // ESP_ERROR_CHECK(esp_lcd_panel_invert_color(panel_handle, true));
}
```

---

## วาดภาพบนจอ (Drawing Bitmaps)

### Full Screen Pattern (128×64)
```c
// จอง 1024 bytes = 128×64 / 8 (1 bit per pixel)
uint8_t *buf = (uint8_t *)malloc(128 * 64 / 8);
if (buf) {
    // ลวดลาย checkerboard
    for (int i = 0; i < (128 * 64 / 8); i++) {
        buf[i] = (i % 2 == 0) ? 0xAA : 0x55;
    }
    // วาดลงจอ (x_start, y_start, x_end_exclusive, y_end_exclusive, data)
    esp_lcd_panel_draw_bitmap(panel_handle, 0, 0, 128, 64, buf);
    free(buf);  // คืน memory หลังส่งข้อมูล
}
```

### Icon 16×16 พิกเซล (ตรงกลางจอ)
```c
// ขนาด 16×16 = 32 bytes (16 rows × 2 bytes/row)
// X center: 128/2 - 16/2 = 56, Y center: 64/2 - 16/2 = 24
const uint8_t heart_icon[32] = {
    0x00, 0x00, 0x38, 0x1C, 0x7C, 0x3E, 0xFE, 0x7F,
    0xFE, 0x7F, 0xFE, 0x7F, 0xFC, 0x3F, 0xF8, 0x1F,
    0xF0, 0x0F, 0xE0, 0x07, 0xC0, 0x03, 0x80, 0x01,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
};
esp_lcd_panel_draw_bitmap(panel_handle, 56, 24, 56 + 16, 24 + 16, heart_icon);
```

---

## esp_lcd_panel_draw_bitmap() — พิกัดและ Format

| พารามิเตอร์ | ความหมาย |
|-------------|----------|
| `x_start` | คอลัมน์เริ่มต้น (0 = ซ้ายสุด) |
| `y_start` | แถวเริ่มต้น (0 = บนสุด) |
| `x_end` | คอลัมน์สิ้นสุด **ไม่นับ** (x_start + width) |
| `y_end` | แถวสิ้นสุด **ไม่นับ** (y_start + height) |
| `color_data` | Bitmap data — 1 bit per pixel, row-major order |

> **ตัวอย่าง:** วาด 16×16 icon ที่ X=56, Y=24:
> `esp_lcd_panel_draw_bitmap(panel, 56, 24, 72, 40, data)` — ขนาด 32 bytes

---

## Bitmap Format — 1 bit per pixel, MSB first

```
Byte 0: pixels (0,0)...(7,0)  — แถว 0 คอลัมน์ 0-7
Byte 1: pixels (8,0)...(15,0) — แถว 0 คอลัมน์ 8-15
Byte 2: pixels (0,1)...(7,1)  — แถว 1 คอลัมน์ 0-7
...
Bit 7 (MSB) = คอลัมน์ซ้ายสุดของ byte นั้น
Bit 0 (LSB) = คอลัมน์ขวาสุดของ byte นั้น
```

---

## I2C Address

| Config | Address |
|--------|---------|
| SA0 pin = LOW (default) | `0x3C` |
| SA0 pin = HIGH | `0x3D` |

ถ้าไม่ทราบให้ I2C scan หา address ก่อน:
```c
// Scan loop (ใช้ legacy driver หรือ new driver)
for (uint8_t addr = 1; addr < 127; addr++) {
    esp_err_t ret = i2c_master_probe(bus_handle, addr, pdMS_TO_TICKS(10));
    if (ret == ESP_OK) {
        ESP_LOGI(TAG, "Found device at 0x%02X", addr);
    }
}
```

---

## Full Working Example — OLED Test (Hardware-Verified)

```c
// [FILE: main/main.c]
#include <stdio.h>
#include <stdlib.h>
#include "esp_log.h"
#include "driver/i2c_master.h"
#include "esp_lcd_panel_io.h"
#include "esp_lcd_panel_ops.h"
#include "esp_lcd_panel_vendor.h"

static const char *TAG = "OLED_TEST";

#define I2C_MASTER_SCL_IO 22
#define I2C_MASTER_SDA_IO 21
#define I2C_MASTER_NUM    I2C_NUM_0

void app_main(void)
{
    ESP_LOGI(TAG, "Initialize I2C bus");
    i2c_master_bus_config_t i2c_bus_config = {
        .i2c_port = I2C_MASTER_NUM,
        .sda_io_num = I2C_MASTER_SDA_IO,
        .scl_io_num = I2C_MASTER_SCL_IO,
        .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7,
        .flags.enable_internal_pullup = true,
    };
    i2c_master_bus_handle_t bus_handle;
    ESP_ERROR_CHECK(i2c_new_master_bus(&i2c_bus_config, &bus_handle));

    ESP_LOGI(TAG, "Install panel IO");
    esp_lcd_panel_io_handle_t io_handle = NULL;
    esp_lcd_panel_io_i2c_config_t io_config = {
        .dev_addr = 0x3C,
        .control_phase_bytes = 1,
        .lcd_cmd_bits = 8,
        .lcd_param_bits = 8,
        .dc_bit_offset = 6,
        .scl_speed_hz = 400 * 1000,
    };
    ESP_ERROR_CHECK(esp_lcd_new_panel_io_i2c(bus_handle, &io_config, &io_handle));

    ESP_LOGI(TAG, "Install SSD1306 panel driver");
    esp_lcd_panel_handle_t panel_handle = NULL;
    esp_lcd_panel_dev_config_t panel_config = {
        .bits_per_pixel = 1,
        .reset_gpio_num = -1,
    };
    ESP_ERROR_CHECK(esp_lcd_new_panel_ssd1306(io_handle, &panel_config, &panel_handle));

    ESP_LOGI(TAG, "Initialize display");
    ESP_ERROR_CHECK(esp_lcd_panel_reset(panel_handle));
    ESP_ERROR_CHECK(esp_lcd_panel_init(panel_handle));

    ESP_LOGI(TAG, "Turn on display");
    ESP_ERROR_CHECK(esp_lcd_panel_disp_on_off(panel_handle, true));

    // Invert สีเพื่อทดสอบว่าจอตอบสนอง
    ESP_ERROR_CHECK(esp_lcd_panel_invert_color(panel_handle, true));

    // วาดลวดลาย checkerboard เต็มจอ
    ESP_LOGI(TAG, "Drawing pattern on screen...");
    uint8_t *image_data = (uint8_t *)malloc(128 * 64 / 8);
    if (image_data) {
        for (int i = 0; i < (128 * 64 / 8); i++) {
            image_data[i] = (i % 2 == 0) ? 0xAA : 0x55;
        }
        esp_lcd_panel_draw_bitmap(panel_handle, 0, 0, 128, 64, image_data);
        free(image_data);
    }

    // วาดรูปหัวใจกลางจอ (16×16)
    ESP_LOGI(TAG, "Drawing Heart Icon...");
    const uint8_t heart_icon[32] = {
        0x00, 0x00, 0x38, 0x1C, 0x7C, 0x3E, 0xFE, 0x7F,
        0xFE, 0x7F, 0xFE, 0x7F, 0xFC, 0x3F, 0xF8, 0x1F,
        0xF0, 0x0F, 0xE0, 0x07, 0xC0, 0x03, 0x80, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    };
    esp_lcd_panel_draw_bitmap(panel_handle, 56, 24, 56 + 16, 24 + 16, heart_icon);

    ESP_LOGI(TAG, "Test Completed!");
}
```

---

## CRITICAL RULES สรุป

| กฎ | รายละเอียด |
|----|-----------|
| ✅ ลำดับ init | `i2c_new_master_bus` → `esp_lcd_new_panel_io_i2c` → `esp_lcd_new_panel_ssd1306` → `reset` → `init` → `disp_on_off` |
| ❌ ห้ามสลับลำดับ | เรียก `ssd1306` ก่อน `panel_io` = crash หรือ compile error |
| ❌ ห้ามผสม API | `driver/i2c.h` + `driver/i2c_master.h` ใน project เดียว = conflict |
| ✅ CMakeLists | ต้องเพิ่ม `REQUIRES esp_lcd` |
| ✅ Address | 0x3C (default) หรือ 0x3D (SA0=HIGH) |
| ✅ draw_bitmap | end พิกัดคือ exclusive (start + size) |
| ✅ ขนาด buffer | 128×64 / 8 = **1024 bytes** ต้อง malloc ก่อน |

---

## อ้างอิง
- ESP-IDF v5.x Docs: `esp_lcd_panel_ops.h`, `esp_lcd_panel_io.h`
- Hardware: SSD1306 0.96" I2C OLED 128×64
- Verified: May 14, 2026
