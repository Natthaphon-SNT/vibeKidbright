/**
 * @file temp_lm73.c
 * @brief LM73 Temperature Sensor — ESP-IDF v5.x
 *
 * ── HARDWARE ────────────────────────────────────────────────────
 *  IC      : LM73 (Texas Instruments)
 *  Bus     : I2C_NUM_1  (SDA=GPIO4, SCL=GPIO5)
 *  Address : 0x4D
 *  Resolution: 13-bit  →  raw / 128.0 = °C
 *
 * ⚠️ GPIO4 CONFLICT: GPIO4 is shared with the BT LED indicator.
 *   If using temp sensor, NEVER call gpio_set_level(GPIO_NUM_4, ...)
 *   as it will corrupt the I2C bus. Choose one or the other.
 *
 * ⚠️ I2C RULE: Initialize I2C_NUM_1 SEPARATELY from I2C_NUM_0
 *   (which is used by LED matrix and accelerometer).
 *   NEVER call i2c_driver_install() twice on the same port.
 * ─────────────────────────────────────────────────────────────────
 */

#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

static const char *TAG = "LM73_TEMP";

#define LM73_I2C_PORT    I2C_NUM_1
#define LM73_SDA_PIN     GPIO_NUM_4
#define LM73_SCL_PIN     GPIO_NUM_5
#define LM73_I2C_FREQ    100000      // 100kHz
#define LM73_ADDR        0x4D
#define LM73_REG_TEMP    0x00        // Temperature register

/* ── Initialize I2C_NUM_1 for LM73 ────────────────────────────── */
static void lm73_i2c_init(void)
{
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = LM73_SDA_PIN,
        .scl_io_num       = LM73_SCL_PIN,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = LM73_I2C_FREQ,
    };
    ESP_ERROR_CHECK(i2c_param_config(LM73_I2C_PORT, &conf));
    ESP_ERROR_CHECK(i2c_driver_install(LM73_I2C_PORT, I2C_MODE_MASTER, 0, 0, 0));
    ESP_LOGI(TAG, "I2C_NUM_1 initialized (SDA=GPIO4, SCL=GPIO5)");
}

/* ── Read temperature from LM73 ────────────────────────────────── */
static esp_err_t lm73_read_temp(float *temp_c)
{
    uint8_t reg = LM73_REG_TEMP;
    uint8_t data[2] = {0};

    /* Write register pointer then read 2 bytes */
    esp_err_t ret = i2c_master_write_read_device(
        LM73_I2C_PORT, LM73_ADDR,
        &reg, 1,
        data, 2,
        pdMS_TO_TICKS(100)
    );

    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "I2C read failed: %s", esp_err_to_name(ret));
        return ret;
    }

    /* LM73: 13-bit two's complement, MSB first.
     * Bits [15:3] are the temperature, LSB [2:0] are flags.
     * Raw signed value / 128.0 = degrees Celsius. */
    int16_t raw = (int16_t)((data[0] << 8) | data[1]);
    *temp_c = raw / 128.0f;
    return ESP_OK;
}

/* ── Task ──────────────────────────────────────────────────────── */
static void temp_task(void *pvParam)
{
    lm73_i2c_init();

    while (1) {
        float temp = 0.0f;
        if (lm73_read_temp(&temp) == ESP_OK) {
            ESP_LOGI(TAG, "Temperature: %.2f °C", temp);
        }
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

void app_main(void)
{
    ESP_LOGI(TAG, "KidBright32 iA — LM73 Temperature Sensor Demo");
    xTaskCreate(temp_task, "temp_task", 4096, NULL, 5, NULL);
}
