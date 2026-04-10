/**
 * @file all_sensors_demo.c
 * @brief KidBright32 iA — Complete All-Sensor Demo (ESP-IDF v5.x)
 *
 * ══ SENSOR MAP ═══════════════════════════════════════════════════════
 *
 *  ON-BOARD SENSORS:
 *  ┌──────────────────┬──────────────────────────────────────────────┐
 *  │ Sensor           │ Interface                                    │
 *  ├──────────────────┼──────────────────────────────────────────────┤
 *  │ LDR (Light)      │ ADC1_CH0 → GPIO36 (input-only, no pull)     │
 *  │ LM73 (Temp)      │ I2C_NUM_1, addr=0x4D, SDA=GPIO4, SCL=GPIO5  │
 *  │ KXTJ3 (Accel)    │ I2C_NUM_0, addr=0x0E, SDA=GPIO21, SCL=GPIO22│
 *  │ LED Matrix       │ I2C_NUM_0, addr=0x70 (HT16K33)              │
 *  └──────────────────┴──────────────────────────────────────────────┘
 *
 *  EXTERNAL (JST) SENSORS:
 *  ┌──────────────────┬──────────────────────────────────────────────┐
 *  │ IN1 GPIO32       │ ADC1_CH4 — Digital I/O, ADC, touch capable  │
 *  │ IN2 GPIO33       │ ADC1_CH5 — Digital I/O, ADC, touch capable  │
 *  │ IN3 GPIO34       │ ADC1_CH6 — Input-only, no pull              │
 *  │ IN4 GPIO35       │ ADC1_CH7 — Input-only, no pull              │
 *  │ OUT1 GPIO26      │ Digital I/O, DAC2                           │
 *  │ OUT2 GPIO27      │ Digital I/O                                 │
 *  └──────────────────┴──────────────────────────────────────────────┘
 *
 * ══ CRITICAL RULES ═══════════════════════════════════════════════════
 *  [ADC] Use esp_adc/adc_oneshot.h ONLY. NEVER use driver/adc.h.
 *  [ADC] ADC_ATTEN_DB_12 for 0–3.3V range. NEVER ADC_ATTEN_DB_11.
 *  [I2C] Init I2C_NUM_0 once (shared: matrix + accel). Init I2C_NUM_1 separately.
 *  [GPIO4] BT LED and LM73 SDA share GPIO4 — MUTUALLY EXCLUSIVE.
 *          In this file we use LM73, so do NOT call gpio_set_level(GPIO_NUM_4, ...)
 * ═════════════════════════════════════════════════════════════════════
 */

#include <stdio.h>
#include <string.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "esp_log.h"
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"

static const char *TAG = "KB_SENSORS";

/* ══ I2C Configuration ════════════════════════════════════════════ */
/* Bus 0: LED Matrix (0x70) + Accelerometer (0x0E) */
#define I2C0_SDA        GPIO_NUM_21
#define I2C0_SCL        GPIO_NUM_22
#define I2C0_FREQ       400000
/* Bus 1: LM73 Temperature (0x4D) */
#define I2C1_SDA        GPIO_NUM_4    // ⚠️ Conflicts with BT LED
#define I2C1_SCL        GPIO_NUM_5
#define I2C1_FREQ       100000

/* ══ Device addresses ═════════════════════════════════════════════ */
#define LM73_ADDR       0x4D
#define KXTJ3_ADDR      0x0E
#define KXTJ3_WHO_AM_I  0x0F
#define KXTJ3_CTRL_REG1 0x1B
#define KXTJ3_XOUT_L    0x06

/* ══ ADC channel mapping ══════════════════════════════════════════ */
#define LDR_CH      ADC_CHANNEL_0   // GPIO36 — On-board LDR
#define IN1_CH      ADC_CHANNEL_4   // GPIO32 — JST IN1 external
#define IN2_CH      ADC_CHANNEL_5   // GPIO33 — JST IN2 external
#define IN3_CH      ADC_CHANNEL_6   // GPIO34 — JST IN3 external (input-only)
#define IN4_CH      ADC_CHANNEL_7   // GPIO35 — JST IN4 external (input-only)

/* ══ Module handles ═══════════════════════════════════════════════ */
static adc_oneshot_unit_handle_t adc1_handle;
static adc_cali_handle_t cali_ldr, cali_in1, cali_in2;

/* ══ ADC INIT ═════════════════════════════════════════════════════ */
static bool adc_cali_init(adc_channel_t ch, adc_cali_handle_t *out)
{
    bool ok = false;
    adc_cali_handle_t h = NULL;
#if ADC_CALI_SCHEME_CURVE_FITTING_SUPPORTED
    adc_cali_curve_fitting_config_t c = {
        .unit_id = ADC_UNIT_1, .chan = ch,
        .atten = ADC_ATTEN_DB_12, .bitwidth = ADC_BITWIDTH_DEFAULT
    };
    ok = (adc_cali_create_scheme_curve_fitting(&c, &h) == ESP_OK);
#endif
#if ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
    if (!ok) {
        adc_cali_line_fitting_config_t c = {
            .unit_id = ADC_UNIT_1, .atten = ADC_ATTEN_DB_12,
            .bitwidth = ADC_BITWIDTH_DEFAULT
        };
        ok = (adc_cali_create_scheme_line_fitting(&c, &h) == ESP_OK);
    }
#endif
    *out = h;
    return ok;
}

static void adc_init_all(void)
{
    adc_oneshot_unit_init_cfg_t u = { .unit_id = ADC_UNIT_1 };
    ESP_ERROR_CHECK(adc_oneshot_new_unit(&u, &adc1_handle));

    adc_oneshot_chan_cfg_t c = { .bitwidth = ADC_BITWIDTH_DEFAULT, .atten = ADC_ATTEN_DB_12 };
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, LDR_CH, &c));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN1_CH, &c));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN2_CH, &c));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN3_CH, &c));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN4_CH, &c));

    adc_cali_init(LDR_CH, &cali_ldr);
    adc_cali_init(IN1_CH, &cali_in1);
    adc_cali_init(IN2_CH, &cali_in2);
    ESP_LOGI(TAG, "ADC initialized");
}

static int adc_read_mv(adc_channel_t ch, adc_cali_handle_t cali)
{
    int raw = 0, mv = 0;
    ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, ch, &raw));
    if (cali) {
        adc_cali_raw_to_voltage(cali, raw, &mv);
    } else {
        mv = (raw * 3300) / 4095;
    }
    return mv;
}

/* ══ I2C INIT ═════════════════════════════════════════════════════ */
static void i2c_init_bus0(void)
{
    /* Shared bus: LED Matrix + KXTJ3 Accelerometer */
    i2c_config_t cfg = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C0_SDA, .scl_io_num = I2C0_SCL,
        .sda_pullup_en = GPIO_PULLUP_ENABLE, .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C0_FREQ,
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_NUM_0, &cfg));
    ESP_ERROR_CHECK(i2c_driver_install(I2C_NUM_0, I2C_MODE_MASTER, 0, 0, 0));
    ESP_LOGI(TAG, "I2C_NUM_0 ready (SDA=21, SCL=22)");
}

static void i2c_init_bus1(void)
{
    /* Exclusive bus: LM73 Temperature Sensor */
    i2c_config_t cfg = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = I2C1_SDA, .scl_io_num = I2C1_SCL,
        .sda_pullup_en = GPIO_PULLUP_ENABLE, .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C1_FREQ,
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_NUM_1, &cfg));
    ESP_ERROR_CHECK(i2c_driver_install(I2C_NUM_1, I2C_MODE_MASTER, 0, 0, 0));
    ESP_LOGI(TAG, "I2C_NUM_1 ready (SDA=GPIO4, SCL=GPIO5) — LM73");
}

/* ══ LM73 Temperature ═════════════════════════════════════════════ */
static float lm73_read(void)
{
    uint8_t reg = 0x00, data[2] = {0};
    if (i2c_master_write_read_device(I2C_NUM_1, LM73_ADDR,
                                     &reg, 1, data, 2,
                                     pdMS_TO_TICKS(100)) != ESP_OK) {
        ESP_LOGE(TAG, "LM73 read failed");
        return -999.0f;
    }
    int16_t raw = (int16_t)((data[0] << 8) | data[1]);
    return raw / 128.0f;
}

/* ══ KXTJ3 Accelerometer ══════════════════════════════════════════ */
static uint8_t kxtj3_read_reg(uint8_t reg)
{
    uint8_t val = 0;
    i2c_master_write_read_device(I2C_NUM_0, KXTJ3_ADDR,
                                 &reg, 1, &val, 1, pdMS_TO_TICKS(50));
    return val;
}

static void kxtj3_write_reg(uint8_t reg, uint8_t val)
{
    uint8_t buf[2] = {reg, val};
    i2c_master_write_to_device(I2C_NUM_0, KXTJ3_ADDR,
                               buf, 2, pdMS_TO_TICKS(50));
}

static bool kxtj3_init(void)
{
    uint8_t who = kxtj3_read_reg(KXTJ3_WHO_AM_I);
    if (who != 0x35) {
        ESP_LOGE(TAG, "KXTJ3 WHO_AM_I=0x%02X (expected 0x35)", who);
        return false;
    }
    kxtj3_write_reg(KXTJ3_CTRL_REG1, 0xC0); // High-res 12-bit, ±2g, operating
    ESP_LOGI(TAG, "KXTJ3-1057 ready");
    return true;
}

typedef struct { float x, y, z; } vec3_t;
static vec3_t kxtj3_read(void)
{
    uint8_t buf[6] = {0};
    uint8_t reg = KXTJ3_XOUT_L;
    i2c_master_write_read_device(I2C_NUM_0, KXTJ3_ADDR,
                                 &reg, 1, buf, 6, pdMS_TO_TICKS(50));
    int16_t rx = (int16_t)((buf[1] << 8) | buf[0]);
    int16_t ry = (int16_t)((buf[3] << 8) | buf[2]);
    int16_t rz = (int16_t)((buf[5] << 8) | buf[4]);
    return (vec3_t){(rx >> 4) / 1024.0f, (ry >> 4) / 1024.0f, (rz >> 4) / 1024.0f};
}

/* ══ Main sensor polling loop ═════════════════════════════════════ */
static void sensor_poll_task(void *pvParam)
{
    while (1) {
        /* ── On-board ADC: LDR ── */
        int ldr_mv = adc_read_mv(LDR_CH, cali_ldr);
        ESP_LOGI(TAG, "[LDR  GPIO36] %d mV  (higher=darker)", ldr_mv);

        /* ── External: IN1 (GPIO32) — example: LM35 ── */
        int in1_mv = adc_read_mv(IN1_CH, cali_in1);
        float lm35_temp = in1_mv / 10.0f;   // LM35: 10mV/°C
        ESP_LOGI(TAG, "[IN1  GPIO32] %d mV → LM35: %.1f°C", in1_mv, lm35_temp);

        /* ── External: IN2 (GPIO33) — generic 0-3.3V sensor ── */
        int in2_mv = adc_read_mv(IN2_CH, cali_in2);
        ESP_LOGI(TAG, "[IN2  GPIO33] %d mV", in2_mv);

        /* ── External: IN3 (GPIO34) — input-only, no pull ── */
        int in3_mv = adc_read_mv(IN3_CH, NULL);
        ESP_LOGI(TAG, "[IN3  GPIO34] %d mV (no cal)", in3_mv);

        /* ── External: IN4 (GPIO35) — input-only, no pull ── */
        int in4_mv = adc_read_mv(IN4_CH, NULL);
        ESP_LOGI(TAG, "[IN4  GPIO35] %d mV (no cal)", in4_mv);

        /* ── On-board: I2C LM73 Temperature (I2C_NUM_1) ── */
        float temp = lm73_read();
        if (temp > -990.0f) {
            ESP_LOGI(TAG, "[LM73 I2C1 ] %.2f °C", temp);
        }

        /* ── On-board: I2C KXTJ3 Accelerometer (I2C_NUM_0) ── */
        vec3_t a = kxtj3_read();
        ESP_LOGI(TAG, "[KXTJ3 I2C0] X:%.3fg Y:%.3fg Z:%.3fg", a.x, a.y, a.z);

        ESP_LOGI(TAG, "──────────────────────────────────────────");
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

/* ══ Entry point ══════════════════════════════════════════════════ */
void app_main(void)
{
    ESP_LOGI(TAG, "KidBright32 iA — All Sensors Demo (ESP-IDF v5.x)");

    /* ⚠️  Order matters:
     *  1. I2C buses must be initialized before any device communication
     *  2. ADC unit must be initialized once before any channel read
     *  3. Do NOT initialize the same I2C port number twice */
    i2c_init_bus0();    // I2C_NUM_0: LED Matrix + KXTJ3
    i2c_init_bus1();    // I2C_NUM_1: LM73 Temperature
    adc_init_all();     // ADC1: LDR + IN1..IN4

    kxtj3_init();       // KXTJ3 requires I2C_NUM_0 to be ready

    xTaskCreate(sensor_poll_task, "sensor_poll", 8192, NULL, 5, NULL);
}
