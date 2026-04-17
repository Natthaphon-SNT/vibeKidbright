/**
 * @file temp_lm73.c
 * @brief LM73 Temperature Sensor — I2C_NUM_1 (SDA=GPIO4, SCL=GPIO5, addr=0x4D)
 *        KidBright32 iA — ESP-IDF v5.x ONLY
 *
 * ⚠️  GPIO4 CONFLICT: GPIO4 is shared with the BT LED.
 *     NEVER call gpio_set_level(GPIO_NUM_4, ...) while using this driver.
 *
 * Resolution modes supported:
 *   • 11-bit default (0.25 °C/LSB)  — no config write needed after power-on
 *   • 14-bit max    (0.03125 °C/LSB) — call lm73_set_resolution_14bit() once
 */

#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

static const char *TAG = "LM73";

/* ─── Bus & Device Configuration ──────────────────────────────────────────── */
#define I2C_TEMP_NUM      I2C_NUM_1
#define I2C_TEMP_SDA_IO   GPIO_NUM_4    // ⚠️ shared with BT LED — do NOT gpio_set_level
#define I2C_TEMP_SCL_IO   GPIO_NUM_5
#define I2C_TEMP_FREQ_HZ  100000        // 100 kHz (LM73 max 400 kHz)
#define LM73_ADDR         0x4D
#define LM73_REG_TEMP     0x00          // Temperature register pointer
#define LM73_REG_CFG      0x01          // Configuration register pointer
#define LM73_REG_ID       0x07          // ID register — always returns 0x09
#define LM73_EXPECTED_ID  0x09

/* ─── Init I2C_NUM_1 ─────────────────────────────────────────────────────── */
esp_err_t temp_sensor_init(void)
{
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = I2C_TEMP_SDA_IO,
        .scl_io_num       = I2C_TEMP_SCL_IO,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C_TEMP_FREQ_HZ,
    };
    esp_err_t ret = i2c_param_config(I2C_TEMP_NUM, &conf);
    if (ret != ESP_OK) return ret;

    ret = i2c_driver_install(I2C_TEMP_NUM, conf.mode, 0, 0, 0);
    if (ret != ESP_OK) return ret;

    /* Wait at least 14 ms for first conversion after power-on */
    vTaskDelay(pdMS_TO_TICKS(20));

    /* Verify device identity */
    uint8_t id_reg = LM73_REG_ID;
    uint8_t id_val = 0;
    ret = i2c_master_write_read_device(I2C_TEMP_NUM, LM73_ADDR,
                                       &id_reg, 1, &id_val, 1,
                                       pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "I2C read failed during ID check: %s", esp_err_to_name(ret));
        return ret;
    }
    if (id_val != LM73_EXPECTED_ID) {
        ESP_LOGW(TAG, "ID mismatch: got 0x%02X, expected 0x%02X", id_val, LM73_EXPECTED_ID);
        /* Non-fatal — some boards may differ; continue */
    } else {
        ESP_LOGI(TAG, "LM73 found, ID=0x%02X", id_val);
    }
    return ESP_OK;
}

/* ─── Set 14-bit resolution (optional; call once after init) ─────────────── */
esp_err_t lm73_set_resolution_14bit(void)
{
    /* Configuration register: pointer(0x01) + 2 config bytes
       Bits 6:5 = RES[1:0] = 11b → 14-bit resolution → 0x60 in high byte */
    uint8_t buf[3] = { LM73_REG_CFG, 0x60, 0x00 };
    esp_err_t ret = i2c_master_write_to_device(I2C_TEMP_NUM, LM73_ADDR,
                                               buf, sizeof(buf),
                                               pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Set 14-bit resolution failed: %s", esp_err_to_name(ret));
    } else {
        ESP_LOGI(TAG, "LM73 set to 14-bit resolution");
        vTaskDelay(pdMS_TO_TICKS(120)); /* Wait for 14-bit conversion (max 112 ms) */
    }
    return ret;
}

/* ─── Read temperature — 11-bit default mode (0.25 °C/LSB) ─────────────── */
/* Formula: left-justified 16-bit → shift right 5 → divide by 32.0          */
float lm73_read_11bit(void)
{
    uint8_t raw[2];
    uint8_t reg = LM73_REG_TEMP;
    esp_err_t ret = i2c_master_write_read_device(I2C_TEMP_NUM, LM73_ADDR,
                                                  &reg, 1, raw, 2,
                                                  pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Read (11-bit) failed: %s", esp_err_to_name(ret));
        return -999.0f;  /* Error sentinel */
    }
    int16_t temp_raw = (int16_t)((raw[0] << 8) | raw[1]);
    return (float)(temp_raw >> 5) / 32.0f;
}

/* ─── Read temperature — 14-bit max mode (0.03125 °C/LSB) ──────────────── */
/* Formula: left-justified 16-bit → shift right 2 → divide by 128.0         */
float lm73_read_14bit(void)
{
    uint8_t raw[2];
    uint8_t reg = LM73_REG_TEMP;
    esp_err_t ret = i2c_master_write_read_device(I2C_TEMP_NUM, LM73_ADDR,
                                                  &reg, 1, raw, 2,
                                                  pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Read (14-bit) failed: %s", esp_err_to_name(ret));
        return -999.0f;
    }
    int16_t temp_raw = (int16_t)((raw[0] << 8) | raw[1]);
    return (float)(temp_raw >> 2) / 128.0f;
}

/* ─── Demo Task ────────────────────────────────────────────────────────────
   Uses 11-bit default mode (no extra config needed).
   Replace lm73_read_11bit() with lm73_read_14bit() if higher precision needed.
   ─────────────────────────────────────────────────────────────────────────── */
void temp_sensor_task(void *pvParameters)
{
    while (1) {
        float temp = lm73_read_11bit();

        if (temp > -998.0f) {
            ESP_LOGI(TAG, "Temperature: %.2f °C", temp);
        } else {
            ESP_LOGE(TAG, "Temperature read error");
        }

        /* MANDATORY yield — prevents watchdog reset */
        vTaskDelay(pdMS_TO_TICKS(2000));
    }
}

/* ─── app_main entry ────────────────────────────────────────────────────────
   Uncomment to run as standalone program.
   ─────────────────────────────────────────────────────────────────────────── */
/*
void app_main(void)
{
    ESP_ERROR_CHECK(temp_sensor_init());
    // Optional: switch to 14-bit for higher precision
    // lm73_set_resolution_14bit();
    xTaskCreate(temp_sensor_task, "temp_task", 4096, NULL, 5, NULL);
}
*/
