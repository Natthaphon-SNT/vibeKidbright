/**
 * @file accel_kxtj3.c
 * @brief KXTJ3-1057 Accelerometer — I2C_NUM_0 (SDA=GPIO21, SCL=GPIO22, addr=0x0E)
 *        KidBright32 iA — ESP-IDF v5.x ONLY
 *
 * ⚠️  I2C_NUM_0 is SHARED with the LED Matrix (HT16K33 @ 0x70).
 *     Call matrix_init() / i2c_bus0_init() FIRST — NEVER re-install I2C_NUM_0 here.
 *
 * Default config: High-res 12-bit, ±2g, 50 Hz ODR (CTRL_REG1 = 0xC0)
 * WHO_AM_I (0x0F) must return 0x35 — checked on init.
 */

#include <stdio.h>
#include <string.h>
#include <math.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"

static const char *TAG = "KXTJ3";

/* ─── Device Registers ────────────────────────────────────────────────────── */
#define I2C_BUS_NUM          I2C_NUM_0
#define KXTJ3_ADDR           0x0E
#define KXTJ3_REG_XOUT_L     0x06
#define KXTJ3_REG_XOUT_H     0x07
#define KXTJ3_REG_YOUT_L     0x08
#define KXTJ3_REG_YOUT_H     0x09
#define KXTJ3_REG_ZOUT_L     0x0A
#define KXTJ3_REG_ZOUT_H     0x0B
#define KXTJ3_REG_WHO_AM_I   0x0F
#define KXTJ3_REG_CTRL_REG1  0x1B
#define KXTJ3_REG_DATA_CTRL  0x21
#define KXTJ3_EXPECTED_ID    0x35

/* CTRL_REG1 values
   0x00 = Stand-by (must set before changing config)
   0xC0 = PC1=1, RES=1 → High-res 12-bit, ±2g, operating mode */
#define KXTJ3_STANDBY        0x00
#define KXTJ3_OPERATING_HRES 0xC0

/* DATA_CTRL_REG: 0x06 = 50 Hz ODR */
#define KXTJ3_ODR_50HZ       0x06

/* ─── Data type ───────────────────────────────────────────────────────────── */
typedef struct {
    float x_g;
    float y_g;
    float z_g;
} kxtj3_data_t;

typedef enum {
    TILT_FLAT,
    TILT_UPSIDE,
    TILT_SIDEWAYS,
} tilt_state_t;

/* ─── Low-level I2C helpers ───────────────────────────────────────────────── */
static esp_err_t kxtj3_write_reg(uint8_t reg, uint8_t value)
{
    uint8_t buf[2] = { reg, value };
    esp_err_t ret = i2c_master_write_to_device(I2C_BUS_NUM, KXTJ3_ADDR,
                                               buf, sizeof(buf),
                                               pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Write reg 0x%02X failed: %s", reg, esp_err_to_name(ret));
    }
    return ret;
}

static esp_err_t kxtj3_read_reg(uint8_t reg, uint8_t *out)
{
    esp_err_t ret = i2c_master_write_read_device(I2C_BUS_NUM, KXTJ3_ADDR,
                                                  &reg, 1, out, 1,
                                                  pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Read reg 0x%02X failed: %s", reg, esp_err_to_name(ret));
    }
    return ret;
}

/* ─── Init ────────────────────────────────────────────────────────────────── */
/**
 * @brief Initialize KXTJ3-1057.
 * @note  I2C_NUM_0 MUST already be installed (e.g. by matrix_init()) before calling this.
 *        Do NOT call i2c_driver_install() for I2C_NUM_0 here — it causes ESP_ERR_INVALID_STATE.
 */
esp_err_t kxtj3_init(void)
{
    /* Verify device identity */
    uint8_t who = 0;
    esp_err_t ret = kxtj3_read_reg(KXTJ3_REG_WHO_AM_I, &who);
    if (ret != ESP_OK) return ret;
    if (who != KXTJ3_EXPECTED_ID) {
        ESP_LOGE(TAG, "WHO_AM_I mismatch: got 0x%02X, expected 0x%02X", who, KXTJ3_EXPECTED_ID);
        return ESP_ERR_NOT_FOUND;
    }

    /* Step 1: Stand-by mode before configuring */
    ret = kxtj3_write_reg(KXTJ3_REG_CTRL_REG1, KXTJ3_STANDBY);
    if (ret != ESP_OK) return ret;

    /* Step 2: Set ODR to 50 Hz */
    ret = kxtj3_write_reg(KXTJ3_REG_DATA_CTRL, KXTJ3_ODR_50HZ);
    if (ret != ESP_OK) return ret;

    /* Step 3: Enter operating mode — High-res 12-bit, ±2g */
    ret = kxtj3_write_reg(KXTJ3_REG_CTRL_REG1, KXTJ3_OPERATING_HRES);
    if (ret == ESP_OK) {
        ESP_LOGI(TAG, "KXTJ3 init OK (WHO_AM_I=0x%02X, 12-bit ±2g @ 50Hz)", who);
    }
    return ret;
}

/* ─── Read acceleration ───────────────────────────────────────────────────── */
/**
 * @brief Read X, Y, Z acceleration in g (12-bit high-res, ±2g mode).
 *        Sensitivity: 1024 LSB/g. Data is left-justified → shift right by 4.
 */
esp_err_t kxtj3_read(kxtj3_data_t *out)
{
    uint8_t raw[6];
    uint8_t reg = KXTJ3_REG_XOUT_L;
    esp_err_t ret = i2c_master_write_read_device(I2C_BUS_NUM, KXTJ3_ADDR,
                                                  &reg, 1, raw, 6,
                                                  pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Read XYZ failed: %s", esp_err_to_name(ret));
        return ret;
    }

    /* Combine LSB + MSB — data is left-justified, shift right 4 for 12-bit value */
    int16_t x_raw = (int16_t)((raw[1] << 8) | raw[0]) >> 4;
    int16_t y_raw = (int16_t)((raw[3] << 8) | raw[2]) >> 4;
    int16_t z_raw = (int16_t)((raw[5] << 8) | raw[4]) >> 4;

    /* Convert to g: 1024 LSB/g in 12-bit ±2g mode */
    out->x_g = (float)x_raw / 1024.0f;
    out->y_g = (float)y_raw / 1024.0f;
    out->z_g = (float)z_raw / 1024.0f;
    return ESP_OK;
}

/* ─── Motion helpers ──────────────────────────────────────────────────────── */
/**
 * @brief Detect shaking: total acceleration magnitude deviates from 1g.
 * @param threshold_g  Deviation above which motion counts as a shake (e.g. 0.5f)
 */
bool kxtj3_is_shaking(const kxtj3_data_t *data, float threshold_g)
{
    float mag = sqrtf(data->x_g * data->x_g +
                      data->y_g * data->y_g +
                      data->z_g * data->z_g);
    return fabsf(mag - 1.0f) > threshold_g;
}

/**
 * @brief Simple tilt detection from Z-axis.
 */
tilt_state_t kxtj3_get_tilt(const kxtj3_data_t *data)
{
    if      (data->z_g >  0.7f) return TILT_FLAT;
    else if (data->z_g < -0.7f) return TILT_UPSIDE;
    else                        return TILT_SIDEWAYS;
}

/* ─── Demo Task ────────────────────────────────────────────────────────────── */
void accel_task(void *pvParameters)
{
    kxtj3_data_t accel;

    while (1) {
        if (kxtj3_read(&accel) == ESP_OK) {
            ESP_LOGI(TAG, "X=%.3f g  Y=%.3f g  Z=%.3f g | shake=%s | tilt=%d",
                     accel.x_g, accel.y_g, accel.z_g,
                     kxtj3_is_shaking(&accel, 0.5f) ? "YES" : "no",
                     (int)kxtj3_get_tilt(&accel));
        }
        /* MANDATORY yield — prevents watchdog reset */
        vTaskDelay(pdMS_TO_TICKS(100));
    }
}

/* ─── app_main entry ────────────────────────────────────────────────────────
   Uncomment to run as standalone (matrix_init() must be called first).
   ─────────────────────────────────────────────────────────────────────────── */
/*
void app_main(void)
{
    // I2C_NUM_0 is initialized by matrix_init() — call it first
    // matrix_init();
    ESP_ERROR_CHECK(kxtj3_init());
    xTaskCreate(accel_task, "accel_task", 4096, NULL, 5, NULL);
}
*/
