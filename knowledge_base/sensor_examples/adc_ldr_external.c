/*
 * @file adc_ldr_external.c
 * @brief ESP-IDF v5.x ADC Example for KidBright32 iA
 *
 * ── SENSORS COVERED ──────────────────────────────────────────────
 *  LDR  : GPIO36 / ADC1_CHANNEL_0 (On-board light sensor)
 *  IN1  : GPIO32 / ADC1_CHANNEL_4 (External: e.g. LM35, potentiometer)
 *  IN2  : GPIO33 / ADC1_CHANNEL_5 (External: e.g. moisture sensor)
 *
 * ── CRITICAL: ESP-IDF v5.x ONLY ──────────────────────────────────
 *  NEVER use: #include "driver/adc.h", #include "esp_adc_cal.h"
 *  NEVER use: adc1_config_width(), adc1_get_raw(), esp_adc_cal_characterize()
 *  NEVER use: ADC_ATTEN_DB_11 (deprecated — use ADC_ATTEN_DB_12)
 *  All of the above were REMOVED in ESP-IDF v5.x.
 * ─────────────────────────────────────────────────────────────────
 */

#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"

static const char *TAG = "ADC_SENSOR";

/* ── Pin / Channel definitions ─────────────────────────────────── */
#define LDR_ADC_CHANNEL     ADC_CHANNEL_0   // GPIO36  — On-board LDR
#define IN1_ADC_CHANNEL     ADC_CHANNEL_4   // GPIO32  — JST IN1 (external)
#define IN2_ADC_CHANNEL     ADC_CHANNEL_5   // GPIO33  — JST IN2 (external)
/* Note: IN3=GPIO34(ADC_CH6), IN4=GPIO35(ADC_CH7) are also ADC-capable
 * but are input-only pins with no pull-up/pull-down. */

/* ── Module-level handles ──────────────────────────────────────── */
static adc_oneshot_unit_handle_t adc1_handle = NULL;
static adc_cali_handle_t cali_ldr = NULL;
static adc_cali_handle_t cali_in1 = NULL;
static adc_cali_handle_t cali_in2 = NULL;

/* ── Calibration helper ────────────────────────────────────────── */
static bool adc_calibration_init(adc_unit_t unit,
                                  adc_channel_t channel,
                                  adc_atten_t atten,
                                  adc_cali_handle_t *out_handle)
{
    bool calibrated = false;
    esp_err_t ret = ESP_FAIL;
    adc_cali_handle_t handle = NULL;

#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
    if (!calibrated) {
        adc_cali_curve_fitting_config_t cfg = {
            .unit_id  = unit,
            .chan     = channel,
            .atten    = atten,
            .bitwidth = ADC_BITWIDTH_DEFAULT,
        };
        ret = adc_cali_create_scheme_curve_fitting(&cfg, &handle);
        if (ret == ESP_OK) calibrated = true;
    }
#endif

#if ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
    if (!calibrated) {
        adc_cali_line_fitting_config_t cfg = {
            .unit_id  = unit,
            .atten    = atten,
            .bitwidth = ADC_BITWIDTH_DEFAULT,
        };
        ret = adc_cali_create_scheme_line_fitting(&cfg, &handle);
        if (ret == ESP_OK) calibrated = true;
    }
#endif

    *out_handle = handle;
    if (calibrated) {
        ESP_LOGI(TAG, "ADC calibration OK for unit=%d chan=%d", unit, channel);
    } else {
        ESP_LOGW(TAG, "ADC calibration FAILED (no eFuse data). Raw values only.");
    }
    return calibrated;
}

/* ── Initialize all ADC channels ──────────────────────────────── */
static void sensors_adc_init(void)
{
    /* 1. Create ADC1 unit handle (shared across all channels) */
    adc_oneshot_unit_init_cfg_t unit_cfg = {
        .unit_id = ADC_UNIT_1,
    };
    ESP_ERROR_CHECK(adc_oneshot_new_unit(&unit_cfg, &adc1_handle));

    /* 2. Channel config (same attenuation for all: 0–3.3V range) */
    adc_oneshot_chan_cfg_t chan_cfg = {
        .bitwidth = ADC_BITWIDTH_DEFAULT,
        .atten    = ADC_ATTEN_DB_12,   // Full 3.3V range (NOT DB_11!)
    };
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, LDR_ADC_CHANNEL, &chan_cfg));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN1_ADC_CHANNEL, &chan_cfg));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN2_ADC_CHANNEL, &chan_cfg));

    /* 3. Calibration (best-effort — works if eFuse values are burned) */
    adc_calibration_init(ADC_UNIT_1, LDR_ADC_CHANNEL, ADC_ATTEN_DB_12, &cali_ldr);
    adc_calibration_init(ADC_UNIT_1, IN1_ADC_CHANNEL, ADC_ATTEN_DB_12, &cali_in1);
    adc_calibration_init(ADC_UNIT_1, IN2_ADC_CHANNEL, ADC_ATTEN_DB_12, &cali_in2);
}

/* ── Read one channel and return voltage in mV (or raw if no cal) */
static int read_adc_mv(adc_channel_t channel, adc_cali_handle_t cali)
{
    int raw = 0;
    ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, channel, &raw));

    if (cali) {
        int voltage_mv = 0;
        adc_cali_raw_to_voltage(cali, raw, &voltage_mv);
        return voltage_mv;
    }
    /* Rough linear approximation when no calibration data */
    return (raw * 3300) / 4095;
}

/* ── Optional: average multiple reads for stability ───────────── */
static int read_adc_mv_avg(adc_channel_t channel, adc_cali_handle_t cali, int samples)
{
    int sum = 0;
    int raw = 0;
    for (int i = 0; i < samples; i++) {
        ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, channel, &raw));
        sum += raw;
    }
    int avg_raw = sum / samples;

    if (cali) {
        int voltage_mv = 0;
        adc_cali_raw_to_voltage(cali, avg_raw, &voltage_mv);
        return voltage_mv;
    }
    return (avg_raw * 3300) / 4095;
}

/* ── Sensor reading task ───────────────────────────────────────── */
static void sensor_task(void *pvParam)
{
    while (1) {
        /* — LDR (on-board, GPIO36) — INVERTED circuit:
         * MORE light -> LDR resistance drops -> ADC Raw LOW  (bright = ~0-80)
         * LESS light -> LDR resistance rises -> ADC Raw HIGH (dark  = ~700-900+)
         * Use Raw directly — NEVER classify by Voltage.
         * Inverted thresholds: raw<80=สว่างมาก, raw<300=สว่างปานกลาง,
         *                      raw<500=มืดปานกลาง, else=มืดมาก */
        int ldr_raw = 0;
        ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, LDR_ADC_CHANNEL, &ldr_raw));
        const char *ldr_level = (ldr_raw < 80)  ? "สว่างมาก" :
                                (ldr_raw < 300) ? "สว่างปานกลาง" :
                                (ldr_raw < 500) ? "มืดปานกลาง" : "มืดมาก";
        ESP_LOGI(TAG, "LDR (GPIO36): Raw=%d  [%s]", ldr_raw, ldr_level);


        /* — IN1: External LM35 temperature sensor (GPIO32) —
         * LM35 outputs 10mV per °C. Range: 0–150°C → 0–1500mV
         * Connect: VCC→3.3V, GND→GND, OUT→IN1(GPIO32) */
        int in1_mv = read_adc_mv_avg(IN1_ADC_CHANNEL, cali_in1, 16);
        float temp_lm35 = in1_mv / 10.0f;
        ESP_LOGI(TAG, "IN1 (GPIO32) LM35: %d mV → %.1f °C", in1_mv, temp_lm35);

        /* — IN2: External moisture/generic sensor (GPIO33) —
         * Generic 0–3.3V analog sensor: report raw voltage */
        int in2_mv = read_adc_mv_avg(IN2_ADC_CHANNEL, cali_in2, 16);
        ESP_LOGI(TAG, "IN2 (GPIO33) Sensor: %d mV", in2_mv);

        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

/* ── Entry point ───────────────────────────────────────────────── */
void app_main(void)
{
    ESP_LOGI(TAG, "KidBright32 iA — ADC Sensor Demo (ESP-IDF v5.x)");
    sensors_adc_init();
    xTaskCreate(sensor_task, "sensor_task", 4096, NULL, 5, NULL);
}
