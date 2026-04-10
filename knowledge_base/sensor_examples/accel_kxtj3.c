/**
 * @file accel_kxtj3.c
 * @brief KXTJ3-1057 Accelerometer — ESP-IDF v5.x
 *
 * ── HARDWARE ────────────────────────────────────────────────────
 *  IC      : KXTJ3-1057 (Rohm/Kionix)
 *  Bus     : I2C_NUM_0  (SDA=GPIO21, SCL=GPIO22) — SHARED with LED matrix
 *  Address : 0x0E
 *  WHO_AM_I register (0x0F) must return 0x35
 *
 * ⚠️ RULE: ALWAYS call matrix_init() FIRST to initialize I2C_NUM_0.
 *   NEVER re-initialize I2C_NUM_0 separately for the accelerometer.
 *   If no matrix is used in the project, initialize I2C_NUM_0 once here.
 *
 * ── REGISTERS ───────────────────────────────────────────────────
 *  0x0F  WHO_AM_I      (must return 0x35)
 *  0x1A  CTRL_REG1     (0x00=standby, 0xC0=operating, high-res 12-bit ±2g)
 *  0x06  XOUT_L, 0x07  XOUT_H
 *  0x08  YOUT_L, 0x09  YOUT_H
 *  0x0A  ZOUT_L, 0x0B  ZOUT_H
 *
 * ── SENSITIVITY ─────────────────────────────────────────────────
 *  12-bit high-res mode, ±2g: 1024 LSB/g
 *  Formula: raw >> 4, then / 1024.0f = g
 * ─────────────────────────────────────────────────────────────────
 */

#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

static const char *TAG = "KXTJ3";

#define ACCEL_I2C_PORT   I2C_NUM_0
#define ACCEL_SDA_PIN    GPIO_NUM_21
#define ACCEL_SCL_PIN    GPIO_NUM_22
#define ACCEL_I2C_FREQ   400000       // 400kHz (Fast mode)
#define KXTJ3_ADDR       0x0E
#define KXTJ3_WHO_AM_I   0x0F
#define KXTJ3_CTRL_REG1  0x1A
#define KXTJ3_XOUT_L     0x06

/* ── Initialize I2C_NUM_0 ──────────────────────────────────────── */
static void accel_i2c_init(void)
{
    /* IMPORTANT: If LED matrix is used in the same project, call
     * matrix_init() instead of this function — do NOT init I2C_NUM_0 twice! */
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = ACCEL_SDA_PIN,
        .scl_io_num       = ACCEL_SCL_PIN,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = ACCEL_I2C_FREQ,
    };
    ESP_ERROR_CHECK(i2c_param_config(ACCEL_I2C_PORT, &conf));
    ESP_ERROR_CHECK(i2c_driver_install(ACCEL_I2C_PORT, I2C_MODE_MASTER, 0, 0, 0));
    ESP_LOGI(TAG, "I2C_NUM_0 initialized (SDA=GPIO21, SCL=GPIO22)");
}

/* ── Read one byte register ────────────────────────────────────── */
static esp_err_t kxtj3_read_reg(uint8_t reg, uint8_t *value)
{
    return i2c_master_write_read_device(
        ACCEL_I2C_PORT, KXTJ3_ADDR,
        &reg, 1, value, 1,
        pdMS_TO_TICKS(50)
    );
}

/* ── Write one byte register ───────────────────────────────────── */
static esp_err_t kxtj3_write_reg(uint8_t reg, uint8_t value)
{
    uint8_t buf[2] = {reg, value};
    return i2c_master_write_to_device(
        ACCEL_I2C_PORT, KXTJ3_ADDR,
        buf, sizeof(buf),
        pdMS_TO_TICKS(50)
    );
}

/* ── Initialize KXTJ3-1057 ─────────────────────────────────────── */
static esp_err_t kxtj3_init(void)
{
    /* Verify WHO_AM_I */
    uint8_t who_am_i = 0;
    if (kxtj3_read_reg(KXTJ3_WHO_AM_I, &who_am_i) != ESP_OK || who_am_i != 0x35) {
        ESP_LOGE(TAG, "WHO_AM_I mismatch! Got 0x%02X (expected 0x35)", who_am_i);
        return ESP_ERR_NOT_FOUND;
    }
    ESP_LOGI(TAG, "KXTJ3-1057 found (WHO_AM_I=0x%02X)", who_am_i);

    /* CTRL_REG1: PC1=1 (operating), RES=1 (high-res 12-bit), GSEL=00 (±2g) */
    return kxtj3_write_reg(KXTJ3_CTRL_REG1, 0xC0);
}

/* ── Read 3-axis acceleration ──────────────────────────────────── */
typedef struct {
    float x_g, y_g, z_g;
} accel_data_t;

static esp_err_t kxtj3_read_accel(accel_data_t *out)
{
    uint8_t buf[6] = {0};
    uint8_t reg = KXTJ3_XOUT_L;

    esp_err_t ret = i2c_master_write_read_device(
        ACCEL_I2C_PORT, KXTJ3_ADDR,
        &reg, 1, buf, 6,
        pdMS_TO_TICKS(50)
    );
    if (ret != ESP_OK) return ret;

    /* 12-bit left-justified: shift raw right by 4, then divide by 1024 LSB/g */
    int16_t raw_x = (int16_t)((buf[1] << 8) | buf[0]);
    int16_t raw_y = (int16_t)((buf[3] << 8) | buf[2]);
    int16_t raw_z = (int16_t)((buf[5] << 8) | buf[4]);

    out->x_g = (raw_x >> 4) / 1024.0f;
    out->y_g = (raw_y >> 4) / 1024.0f;
    out->z_g = (raw_z >> 4) / 1024.0f;
    return ESP_OK;
}

/* ── Task ──────────────────────────────────────────────────────── */
static void accel_task(void *pvParam)
{
    accel_i2c_init();

    if (kxtj3_init() != ESP_OK) {
        ESP_LOGE(TAG, "Accelerometer init failed. Halting task.");
        vTaskDelete(NULL);
        return;
    }

    accel_data_t accel = {0};
    while (1) {
        if (kxtj3_read_accel(&accel) == ESP_OK) {
            ESP_LOGI(TAG, "X: %.3fg  Y: %.3fg  Z: %.3fg",
                     accel.x_g, accel.y_g, accel.z_g);
        }
        vTaskDelay(pdMS_TO_TICKS(100));
    }
}

void app_main(void)
{
    ESP_LOGI(TAG, "KidBright32 iA — KXTJ3 Accelerometer Demo");
    xTaskCreate(accel_task, "accel_task", 4096, NULL, 5, NULL);
}
