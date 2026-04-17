/**
 * @file adc_ldr_external.c
 * @brief LDR (GPIO36/ADC1_CH0) + External JST IN1 (GPIO32/ADC1_CH4) + IN2 (GPIO33/ADC1_CH5)
 *        KidBright32 iA — ESP-IDF v5.x ONLY
 *
 * ✅ Uses esp_adc/adc_oneshot.h  (ESP-IDF v5.x)
 * ❌ NEVER uses driver/adc.h or esp_adc_cal.h (REMOVED in v5.x)
 * ❌ NEVER uses ADC_ATTEN_DB_11 (renamed to ADC_ATTEN_DB_12)
 */

#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"

static const char *TAG = "ADC_SENSORS";

/* ─── ADC Channel Definitions ─────────────────────────────────────────────── */
#define LDR_ADC_CHAN      ADC_CHANNEL_0   // GPIO36 — on-board LDR (input-only)
#define IN1_ADC_CHAN      ADC_CHANNEL_4   // GPIO32 — JST IN1
#define IN2_ADC_CHAN      ADC_CHANNEL_5   // GPIO33 — JST IN2

/* LDR brightness mapping (tune to your environment) */
#define LDR_ADC_MIN_VAL   0     // raw when fully bright
#define LDR_ADC_MAX_VAL   900   // raw when fully dark

/* ─── Global Handles ───────────────────────────────────────────────────────── */
static adc_oneshot_unit_handle_t adc1_handle = NULL;
static adc_cali_handle_t         cali_ldr    = NULL;
static adc_cali_handle_t         cali_in1    = NULL;
static adc_cali_handle_t         cali_in2    = NULL;
static bool cali_ldr_ok = false;
static bool cali_in1_ok = false;
static bool cali_in2_ok = false;

/* ─── Internal: Create calibration scheme ─────────────────────────────────── */
static bool adc_calibration_init(adc_unit_t unit,
                                 adc_channel_t channel,
                                 adc_atten_t atten,
                                 adc_cali_handle_t *out_handle)
{
    adc_cali_handle_t handle = NULL;
    esp_err_t ret = ESP_FAIL;
    bool calibrated = false;

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
    if (!calibrated) {
        ESP_LOGW(TAG, "Calibration not supported on this chip for ch=%d", channel);
    }
    return calibrated;
}

/* ─── Public: Init all ADC channels ──────────────────────────────────────── */
esp_err_t adc_init_all(void)
{
    /* 1. Create ADC unit */
    adc_oneshot_unit_init_cfg_t unit_cfg = { .unit_id = ADC_UNIT_1 };
    ESP_ERROR_CHECK(adc_oneshot_new_unit(&unit_cfg, &adc1_handle));

    /* 2. Channel configuration — ADC_ATTEN_DB_12 for full 0–3.3 V range */
    adc_oneshot_chan_cfg_t ch_cfg = {
        .atten    = ADC_ATTEN_DB_12,      // ✅ v5.x — was DB_11 (RENAMED)
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, LDR_ADC_CHAN, &ch_cfg));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN1_ADC_CHAN, &ch_cfg));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN2_ADC_CHAN, &ch_cfg));

    /* 3. Calibration per channel */
    cali_ldr_ok = adc_calibration_init(ADC_UNIT_1, LDR_ADC_CHAN, ADC_ATTEN_DB_12, &cali_ldr);
    cali_in1_ok = adc_calibration_init(ADC_UNIT_1, IN1_ADC_CHAN, ADC_ATTEN_DB_12, &cali_in1);
    cali_in2_ok = adc_calibration_init(ADC_UNIT_1, IN2_ADC_CHAN, ADC_ATTEN_DB_12, &cali_in2);

    ESP_LOGI(TAG, "ADC init OK — LDR:cali=%d  IN1:cali=%d  IN2:cali=%d",
             cali_ldr_ok, cali_in1_ok, cali_in2_ok);
    return ESP_OK;
}

/* ─── Public: Read raw ADC value ─────────────────────────────────────────── */
int adc_read_raw(adc_channel_t channel)
{
    int raw = 0;
    /* EMA state: keeps noise-smoothed value between calls */
    static int ema_ldr = -1, ema_in1 = -1, ema_in2 = -1;

    ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, channel, &raw));

    /* Apply Exponential Moving Average to reduce ADC noise */
    if (channel == LDR_ADC_CHAN) {
        if (ema_ldr < 0) ema_ldr = raw;
        ema_ldr = (ema_ldr * 9 + raw) / 10;
        return ema_ldr;
    } else if (channel == IN1_ADC_CHAN) {
        if (ema_in1 < 0) ema_in1 = raw;
        ema_in1 = (ema_in1 * 9 + raw) / 10;
        return ema_in1;
    } else if (channel == IN2_ADC_CHAN) {
        if (ema_in2 < 0) ema_in2 = raw;
        ema_in2 = (ema_in2 * 9 + raw) / 10;
        return ema_in2;
    }
    return raw;
}

/* ─── Public: Read voltage in mV (calibrated) ────────────────────────────── */
int adc_read_mv(adc_channel_t channel, adc_cali_handle_t cali_handle)
{
    int raw = adc_read_raw(channel);
    int mv  = 0;
    if (cali_handle) {
        adc_cali_raw_to_voltage(cali_handle, raw, &mv);
    } else {
        /* Approximate without calibration: 3300 mV / 4095 */
        mv = (raw * 3300) / 4095;
    }
    return mv;
}

/* ─── LDR helpers ────────────────────────────────────────────────────────── */
int ldr_get_raw(void)
{
    return adc_read_raw(LDR_ADC_CHAN);
}

/** Returns brightness 0–100 % (100 = brightest, 0 = darkest) */
int ldr_get_brightness_percent(int raw)
{
    if (raw <= LDR_ADC_MIN_VAL) return 100;
    if (raw >= LDR_ADC_MAX_VAL) return 0;
    return 100 - ((raw - LDR_ADC_MIN_VAL) * 100 / (LDR_ADC_MAX_VAL - LDR_ADC_MIN_VAL));
}

const char *ldr_classify(int raw)
{
    if      (raw < 500)  return "Very Bright";
    else if (raw < 1500) return "Bright";
    else if (raw < 2500) return "Medium";
    else if (raw < 3500) return "Dim";
    else                 return "Dark";
}

/* ─── Demo Task ─────────────────────────────────────────────────────────────
   Example: LM35 on IN1 (GPIO32 / ADC_CHANNEL_4)
   LM35 formula: 10 mV per degree Celsius
   Connect: VCC→3.3V, GND→GND, OUT→GPIO32 (IN1)
   ─────────────────────────────────────────────────────────────────────────── */
void adc_demo_task(void *pvParameters)
{
    /* Wait for ADC to stabilize */
    vTaskDelay(pdMS_TO_TICKS(50));

    while (1) {
        /* LDR (on-board) */
        int ldr_raw = ldr_get_raw();
        int ldr_pct = ldr_get_brightness_percent(ldr_raw);
        ESP_LOGI(TAG, "LDR  raw=%4d  brightness=%3d%%  (%s)",
                 ldr_raw, ldr_pct, ldr_classify(ldr_raw));

        /* IN1 — e.g. LM35 temperature sensor */
        int in1_mv = adc_read_mv(IN1_ADC_CHAN, cali_in1);
        float temp_c = in1_mv / 10.0f;   // LM35: 10 mV = 1 °C
        ESP_LOGI(TAG, "IN1  mv=%4d  LM35_temp=%.2f °C", in1_mv, temp_c);

        /* IN2 — raw + mV */
        int in2_raw = adc_read_raw(IN2_ADC_CHAN);
        int in2_mv  = adc_read_mv(IN2_ADC_CHAN, cali_in2);
        ESP_LOGI(TAG, "IN2  raw=%4d  mv=%4d", in2_raw, in2_mv);

        /* MANDATORY yield — prevents watchdog reset */
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

/* ─── Cleanup ────────────────────────────────────────────────────────────── */
void adc_deinit(void)
{
    if (cali_ldr_ok) {
#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
        adc_cali_delete_scheme_curve_fitting(cali_ldr);
#elif ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
        adc_cali_delete_scheme_line_fitting(cali_ldr);
#endif
    }
    if (cali_in1_ok) {
#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
        adc_cali_delete_scheme_curve_fitting(cali_in1);
#elif ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
        adc_cali_delete_scheme_line_fitting(cali_in1);
#endif
    }
    if (cali_in2_ok) {
#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
        adc_cali_delete_scheme_curve_fitting(cali_in2);
#elif ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
        adc_cali_delete_scheme_line_fitting(cali_in2);
#endif
    }
    adc_oneshot_del_unit(adc1_handle);
}

/* ─── app_main entry ────────────────────────────────────────────────────────
   Uncomment to run as standalone program.
   ─────────────────────────────────────────────────────────────────────────── */
/*
void app_main(void)
{
    ESP_ERROR_CHECK(adc_init_all());
    xTaskCreate(adc_demo_task, "adc_demo", 4096, NULL, 5, NULL);
}
*/
