/**
 * @file all_sensors_demo.c
 * @brief All-sensors demo for KidBright32 iA — ESP-IDF v5.x
 *
 * Combines: LED Matrix (HT16K33) · LM73 Temp · KXTJ3 Accel · LDR ADC
 *           Buzzer (LEDC) · Buttons SW1/SW2 (ISR + Queue)
 *
 * ── BUS INIT ORDER (mandatory) ──────────────────────────────────────────────
 *  1. i2c_bus0_init()  → I2C_NUM_0: HT16K33 (0x70) + KXTJ3 (0x0E)
 *  2. temp_sensor_init() → I2C_NUM_1: LM73 (0x4D)
 *  3. adc_init_all()   → ADC1: LDR (CH0) + IN1 (CH4) + IN2 (CH5)
 *  4. buzzer_init()    → LEDC on GPIO13
 *  5. gpio_interrupt_init() → SW1 (GPIO16) + SW2 (GPIO14) via ISR queue
 *
 * ── CRITICAL RULES (DO NOT REMOVE) ─────────────────────────────────────────
 *  • ESP-IDF v5.x ONLY — NEVER use driver/adc.h, esp_adc_cal.h, ADC_ATTEN_DB_11
 *  • i2c_driver_install() called ONCE per bus number
 *  • HT16K33 init commands MUST be separate 1-byte I2C writes
 *  • rows_to_columns_16x8 MUST use (7 - row) for Y-axis inversion
 *  • Two-digit display MUST use display_two_digits() — not display_pattern() twice
 *  • ledc_stop() requires 3 args: ledc_stop(mode, ch, idle_level)
 *  • ISR functions MUST be IRAM_ATTR — NEVER call blocking code from ISR
 *  • Every FreeRTOS task MUST call vTaskDelay inside its while(1) loop
 */

/* ─── Includes ──────────────────────────────────────────────────────────────── */
#include <stdio.h>
#include <string.h>
#include <math.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/queue.h"
#include "freertos/semphr.h"
#include "driver/i2c.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "esp_random.h"
#include "esp_adc/adc_oneshot.h"
#include "esp_adc/adc_cali.h"
#include "esp_adc/adc_cali_scheme.h"

static const char *TAG = "KB_DEMO";

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 1 — DEFINES & GLOBALS
   ══════════════════════════════════════════════════════════════════════════════ */

/* ── I2C Bus 0 (Matrix + Accel) ─────────────────────────────────────────── */
#define I2C_BUS0_NUM     I2C_NUM_0
#define I2C_BUS0_SDA     GPIO_NUM_21
#define I2C_BUS0_SCL     GPIO_NUM_22
#define I2C_BUS0_FREQ    100000

/* ── I2C Bus 1 (Temperature) ────────────────────────────────────────────── */
#define I2C_BUS1_NUM     I2C_NUM_1
#define I2C_BUS1_SDA     GPIO_NUM_4   // ⚠️ GPIO4 conflict with BT LED
#define I2C_BUS1_SCL     GPIO_NUM_5
#define I2C_BUS1_FREQ    100000

/* ── HT16K33 LED Matrix ─────────────────────────────────────────────────── */
#define HT16K33_ADDR     0x70

/* ── KXTJ3 Accelerometer ────────────────────────────────────────────────── */
#define KXTJ3_ADDR           0x0E
#define KXTJ3_REG_XOUT_L     0x06
#define KXTJ3_REG_WHO_AM_I   0x0F
#define KXTJ3_REG_CTRL_REG1  0x1B
#define KXTJ3_REG_DATA_CTRL  0x21
#define KXTJ3_EXPECTED_ID    0x35

/* ── LM73 Temperature ───────────────────────────────────────────────────── */
#define LM73_ADDR        0x4D
#define LM73_REG_TEMP    0x00

/* ── ADC Channels ───────────────────────────────────────────────────────── */
#define LDR_ADC_CHAN     ADC_CHANNEL_0   // GPIO36
#define IN1_ADC_CHAN     ADC_CHANNEL_4   // GPIO32

/* ── Buzzer ─────────────────────────────────────────────────────────────── */
#define BUZZER_GPIO      GPIO_NUM_13
#define BUZZER_TIMER     LEDC_TIMER_0
#define BUZZER_CHANNEL   LEDC_CHANNEL_0
#define BUZZER_FREQ_HZ   1000
#define BUZZER_DUTY_RES  LEDC_TIMER_10_BIT
#define BUZZER_DUTY_50   512            // 50% duty for 10-bit

/* ── Buttons ────────────────────────────────────────────────────────────── */
#define SW1_GPIO         GPIO_NUM_16    // Active LOW — internal pull-up
#define SW2_GPIO         GPIO_NUM_14    // Active LOW — internal pull-up
#define ESP_INTR_FLAG_DEFAULT 0

/* ── Global handles ─────────────────────────────────────────────────────── */
static adc_oneshot_unit_handle_t adc1_handle   = NULL;
static adc_cali_handle_t         cali_ldr      = NULL;
static adc_cali_handle_t         cali_in1      = NULL;
static bool cali_ldr_ok = false;
static bool cali_in1_ok = false;
static QueueHandle_t gpio_evt_queue = NULL;

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 2 — VERIFIED DIGIT & ICON PATTERNS (hardware-tested hex values)
   ⚠️ DO NOT invent or modify these hex values — garbled output on hardware.
   ══════════════════════════════════════════════════════════════════════════════ */

/* Each uint16_t = 1 row (top → bottom). Bit15 = leftmost pixel. */
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

static const uint16_t PATTERN_HEART[8] = {
    0x0000, 0x0660, 0x0FF0, 0x1FF8, 0x0FF0, 0x07E0, 0x03C0, 0x0180
};
static const uint16_t PATTERN_SMILEY[8] = {
    0x0000, 0x0C30, 0x0C30, 0x0000, 0x0000, 0x1008, 0x07E0, 0x0000
};
static const uint16_t PATTERN_CHECK[8] = {
    0x0000, 0x0018, 0x0030, 0x0060, 0x1CC0, 0x0F80, 0x0700, 0x0200
};

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 3 — ISR HANDLER (must be first — IRAM_ATTR)
   ══════════════════════════════════════════════════════════════════════════════ */

static void IRAM_ATTR gpio_isr_handler(void *arg)
{
    uint32_t gpio_num = (uint32_t)arg;
    /* ISR-safe queue send — NEVER call printf/vTaskDelay/I2C from here */
    xQueueSendFromISR(gpio_evt_queue, &gpio_num, NULL);
}

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 4 — LOW-LEVEL HELPERS
   ══════════════════════════════════════════════════════════════════════════════ */

/* ── Matrix: row-major → column-major conversion with Y-axis inversion ──── */
/**
 * @brief Convert human-readable row-major uint16_t[8] bitmap to
 *        hardware-ready column-major uint8_t[16] array.
 * ⚠️ CRITICAL: Uses (7 - row) for Y-axis inversion (hardware wired upside-down).
 *    NEVER replace with just (row).
 */
static void rows_to_columns_16x8(const uint16_t row_data[8], uint8_t out_cols[16])
{
    memset(out_cols, 0, 16);
    for (int row = 0; row < 8; row++) {
        for (int col = 0; col < 16; col++) {
            if (row_data[row] & (1u << (15 - col))) {
                /* Y-axis inversion: use (7 - row), NOT (row) */
                out_cols[col] |= (1u << (7 - row));
            }
        }
    }
}

/* ── Matrix: send column-major frame to HT16K33 ────────────────────────── */
/**
 * @brief Write 16 column bytes to the HT16K33 using interleaved mapping.
 *        buf layout: [0x00 addr] [col0_L col0_R col1_L col1_R ... col7_L col7_R]
 * ⚠️ I2C error is logged but NOT checked with ESP_ERROR_CHECK to avoid reboot on glitch.
 */
static void matrix_draw(const uint8_t cols[16])
{
    uint8_t buf[17] = {0};
    buf[0] = 0x00;  /* RAM start address */
    for (int c = 0; c < 8; c++) {
        buf[1 + (c * 2)] = cols[c];       /* Left half — even RAM addresses */
        buf[2 + (c * 2)] = cols[c + 8];   /* Right half — odd RAM addresses */
    }
    esp_err_t ret = i2c_master_write_to_device(I2C_BUS0_NUM, HT16K33_ADDR,
                                               buf, sizeof(buf),
                                               pdMS_TO_TICKS(100));
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "matrix_draw I2C error: %s", esp_err_to_name(ret));
    }
}

/* ── ADC calibration helper ─────────────────────────────────────────────── */
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
            .unit_id  = unit, .chan = channel,
            .atten    = atten, .bitwidth = ADC_BITWIDTH_DEFAULT,
        };
        ret = adc_cali_create_scheme_curve_fitting(&cfg, &handle);
        if (ret == ESP_OK) calibrated = true;
    }
#endif
#if ADC_CALI_SCHEME_LINE_FITTING_SUPPORTED
    if (!calibrated) {
        adc_cali_line_fitting_config_t cfg = {
            .unit_id  = unit,
            .atten    = atten, .bitwidth = ADC_BITWIDTH_DEFAULT,
        };
        ret = adc_cali_create_scheme_line_fitting(&cfg, &handle);
        if (ret == ESP_OK) calibrated = true;
    }
#endif
    *out_handle = handle;
    return calibrated;
}

/* ── KXTJ3 low-level ────────────────────────────────────────────────────── */
static esp_err_t kxtj3_write_reg(uint8_t reg, uint8_t val)
{
    uint8_t buf[2] = { reg, val };
    return i2c_master_write_to_device(I2C_BUS0_NUM, KXTJ3_ADDR,
                                      buf, 2, pdMS_TO_TICKS(100));
}
static esp_err_t kxtj3_read_reg(uint8_t reg, uint8_t *out)
{
    return i2c_master_write_read_device(I2C_BUS0_NUM, KXTJ3_ADDR,
                                        &reg, 1, out, 1, pdMS_TO_TICKS(100));
}

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 5 — MID-LEVEL DISPLAY & PATTERN HELPERS
   ══════════════════════════════════════════════════════════════════════════════ */

/** Display any row-major pattern on the full 16×8 matrix. */
static void display_pattern(const uint16_t pattern[8])
{
    uint8_t cols[16];
    rows_to_columns_16x8(pattern, cols);
    matrix_draw(cols);
}

/**
 * @brief Display two decimal digits — tens on LEFT panel (cols 3–7),
 *        units on RIGHT panel (cols 11–15).
 * ⚠️ MANDATORY for any two-digit number.
 *    display_pattern(DIGITS[x]) alone leaves the right side DARK.
 */
static void display_two_digits(int tens, int units)
{
    if (tens  < 0) tens  = 0;  if (tens  > 9) tens  = 9;
    if (units < 0) units = 0;  if (units > 9) units = 9;

    uint16_t combined[8];
    for (int i = 0; i < 8; i++) {
        /* DIGITS[units] >> 8 moves the left-panel pattern to the right panel */
        combined[i] = DIGITS[tens][i] | (DIGITS[units][i] >> 8);
    }
    uint8_t cols[16];
    rows_to_columns_16x8(combined, cols);
    matrix_draw(cols);
}

/** Display integer 0–99 as two digits. */
static void display_number(int value)
{
    if (value < 0)  value = 0;
    if (value > 99) value = 99;
    display_two_digits((value / 10) % 10, value % 10);
}

/* ── Buzzer helpers ─────────────────────────────────────────────────────── */
static void buzzer_play(uint32_t freq_hz)
{
    ledc_set_freq(LEDC_LOW_SPEED_MODE, BUZZER_TIMER, freq_hz);
    ledc_set_duty(LEDC_LOW_SPEED_MODE, BUZZER_CHANNEL, BUZZER_DUTY_50);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, BUZZER_CHANNEL);
}

static void buzzer_stop(void)
{
    /* Set duty = 0 (mute without stopping timer) */
    ledc_set_duty(LEDC_LOW_SPEED_MODE, BUZZER_CHANNEL, 0);
    ledc_update_duty(LEDC_LOW_SPEED_MODE, BUZZER_CHANNEL);
    /* OR: ledc_stop with 3 args (ESP-IDF v5.x) — idle_level=0 → GPIO LOW */
    /* ledc_stop(LEDC_LOW_SPEED_MODE, BUZZER_CHANNEL, 0); */
}

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 6 — FREERTOS TASK FUNCTIONS
   ══════════════════════════════════════════════════════════════════════════════ */

/** Button task — receives GPIO events from ISR queue */
static void button_task(void *pvParameters)
{
    uint32_t io_num;
    while (1) {
        if (xQueueReceive(gpio_evt_queue, &io_num, portMAX_DELAY)) {
            if (io_num == SW1_GPIO) {
                ESP_LOGI(TAG, "SW1 pressed");
                buzzer_play(880);
                vTaskDelay(pdMS_TO_TICKS(100));
                buzzer_stop();
                display_pattern(PATTERN_HEART);
            } else if (io_num == SW2_GPIO) {
                ESP_LOGI(TAG, "SW2 pressed");
                buzzer_play(440);
                vTaskDelay(pdMS_TO_TICKS(100));
                buzzer_stop();
                display_pattern(PATTERN_SMILEY);
            }
        }
        /* Note: portMAX_DELAY blocks here; vTaskDelay not needed */
    }
}

/** Sensor polling task — reads all sensors and updates display */
static void sensor_task(void *pvParameters)
{
    int display_counter = 0;

    while (1) {
        /* ── LDR (ADC) ────────────────────────────────────────────────────── */
        int ldr_raw = 0;
        ESP_ERROR_CHECK(adc_oneshot_read(adc1_handle, LDR_ADC_CHAN, &ldr_raw));
        int ldr_mv = 0;
        if (cali_ldr_ok) {
            adc_cali_raw_to_voltage(cali_ldr, ldr_raw, &ldr_mv);
        }
        ESP_LOGI(TAG, "LDR  raw=%4d  mv=%4d", ldr_raw, ldr_mv);

        /* ── LM73 Temperature ────────────────────────────────────────────── */
        uint8_t t_raw[2];
        uint8_t t_reg = LM73_REG_TEMP;
        float temp_c = -999.0f;
        esp_err_t t_ret = i2c_master_write_read_device(
            I2C_BUS1_NUM, LM73_ADDR, &t_reg, 1, t_raw, 2, pdMS_TO_TICKS(100));
        if (t_ret == ESP_OK) {
            /* 11-bit default mode: shift right 5, divide by 32 */
            int16_t t_val = (int16_t)((t_raw[0] << 8) | t_raw[1]);
            temp_c = (float)(t_val >> 5) / 32.0f;
            ESP_LOGI(TAG, "LM73 temp=%.2f °C", temp_c);
        } else {
            ESP_LOGE(TAG, "LM73 read error: %s", esp_err_to_name(t_ret));
        }

        /* ── KXTJ3 Accelerometer ─────────────────────────────────────────── */
        uint8_t xyz[6];
        uint8_t xyz_reg = KXTJ3_REG_XOUT_L;
        esp_err_t a_ret = i2c_master_write_read_device(
            I2C_BUS0_NUM, KXTJ3_ADDR, &xyz_reg, 1, xyz, 6, pdMS_TO_TICKS(100));
        if (a_ret == ESP_OK) {
            int16_t ax = (int16_t)((xyz[1] << 8) | xyz[0]) >> 4;
            int16_t ay = (int16_t)((xyz[3] << 8) | xyz[2]) >> 4;
            int16_t az = (int16_t)((xyz[5] << 8) | xyz[4]) >> 4;
            float x_g = (float)ax / 1024.0f;
            float y_g = (float)ay / 1024.0f;
            float z_g = (float)az / 1024.0f;
            ESP_LOGI(TAG, "KXTJ3 X=%.3fg Y=%.3fg Z=%.3fg", x_g, y_g, z_g);
        }

        /* ── Update matrix display (cycle every 3 s) ─────────────────────── */
        display_counter++;
        if (display_counter % 3 == 0 && temp_c > -998.0f) {
            /* Show temperature as two-digit integer (e.g. 28 °C → "28") */
            int t_int = (int)temp_c;
            if (t_int < 0)  t_int = 0;
            if (t_int > 99) t_int = 99;
            display_number(t_int);
        }

        /* MANDATORY yield — prevents watchdog reset */
        vTaskDelay(pdMS_TO_TICKS(1000));
    }
}

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 7 — INIT FUNCTIONS
   ══════════════════════════════════════════════════════════════════════════════ */

/** Init I2C_NUM_0 (Matrix + Accelerometer) — call ONCE */
static esp_err_t i2c_bus0_init(void)
{
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = I2C_BUS0_SDA,
        .scl_io_num       = I2C_BUS0_SCL,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C_BUS0_FREQ,
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_BUS0_NUM, &conf));
    return i2c_driver_install(I2C_BUS0_NUM, conf.mode, 0, 0, 0);
}

/** Init HT16K33 — MUST be called after i2c_bus0_init()
 *  ⚠️ Each command MUST be a SEPARATE 1-byte I2C write.
 *     Sending them combined in one write leaves display BLANK.
 */
static esp_err_t matrix_init(void)
{
    uint8_t cmd;

    /* Oscillator ON */
    cmd = 0x21;
    i2c_master_write_to_device(I2C_BUS0_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));

    /* Display ON, no blink */
    cmd = 0x81;
    i2c_master_write_to_device(I2C_BUS0_NUM, HT16K33_ADDR, &cmd, 1, pdMS_TO_TICKS(100));

    /* Maximum brightness */
    cmd = 0xEF;
    esp_err_t ret = i2c_master_write_to_device(I2C_BUS0_NUM, HT16K33_ADDR,
                                               &cmd, 1, pdMS_TO_TICKS(100));
    if (ret == ESP_OK) {
        ESP_LOGI(TAG, "HT16K33 matrix init OK");
    } else {
        ESP_LOGE(TAG, "HT16K33 init failed: %s", esp_err_to_name(ret));
    }
    return ret;
}

/** Init KXTJ3 — MUST be called after i2c_bus0_init() (shares I2C_NUM_0) */
static esp_err_t kxtj3_init(void)
{
    uint8_t who = 0;
    esp_err_t ret = kxtj3_read_reg(KXTJ3_REG_WHO_AM_I, &who);
    if (ret != ESP_OK || who != KXTJ3_EXPECTED_ID) {
        ESP_LOGE(TAG, "KXTJ3 WHO_AM_I=0x%02X (expected 0x%02X)", who, KXTJ3_EXPECTED_ID);
        return ESP_ERR_NOT_FOUND;
    }
    kxtj3_write_reg(KXTJ3_REG_CTRL_REG1, 0x00);   // Stand-by before config
    kxtj3_write_reg(KXTJ3_REG_DATA_CTRL, 0x06);   // 50 Hz ODR
    ret = kxtj3_write_reg(KXTJ3_REG_CTRL_REG1, 0xC0); // High-res 12-bit ±2g
    if (ret == ESP_OK) ESP_LOGI(TAG, "KXTJ3 init OK");
    return ret;
}

/** Init I2C_NUM_1 (LM73 Temperature) — separate bus, call ONCE */
static esp_err_t temp_sensor_init(void)
{
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = I2C_BUS1_SDA,
        .scl_io_num       = I2C_BUS1_SCL,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C_BUS1_FREQ,
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_BUS1_NUM, &conf));
    esp_err_t ret = i2c_driver_install(I2C_BUS1_NUM, conf.mode, 0, 0, 0);
    if (ret == ESP_OK) {
        vTaskDelay(pdMS_TO_TICKS(20)); /* Wait for first conversion */
        ESP_LOGI(TAG, "LM73 I2C_NUM_1 init OK");
    }
    return ret;
}

/** Init ADC — ✅ ESP-IDF v5.x oneshot API only */
static esp_err_t adc_init_all(void)
{
    adc_oneshot_unit_init_cfg_t unit_cfg = { .unit_id = ADC_UNIT_1 };
    ESP_ERROR_CHECK(adc_oneshot_new_unit(&unit_cfg, &adc1_handle));

    /* ADC_ATTEN_DB_12 = full 0–3.3 V range (renamed from DB_11 in v5.x) */
    adc_oneshot_chan_cfg_t ch_cfg = {
        .atten    = ADC_ATTEN_DB_12,
        .bitwidth = ADC_BITWIDTH_DEFAULT,
    };
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, LDR_ADC_CHAN, &ch_cfg));
    ESP_ERROR_CHECK(adc_oneshot_config_channel(adc1_handle, IN1_ADC_CHAN, &ch_cfg));

    cali_ldr_ok = adc_calibration_init(ADC_UNIT_1, LDR_ADC_CHAN, ADC_ATTEN_DB_12, &cali_ldr);
    cali_in1_ok = adc_calibration_init(ADC_UNIT_1, IN1_ADC_CHAN, ADC_ATTEN_DB_12, &cali_in1);

    ESP_LOGI(TAG, "ADC init OK — LDR cali=%d  IN1 cali=%d", cali_ldr_ok, cali_in1_ok);
    return ESP_OK;
}

/** Init Buzzer (LEDC PWM on GPIO13) */
static esp_err_t buzzer_init(void)
{
    ledc_timer_config_t timer = {
        .speed_mode      = LEDC_LOW_SPEED_MODE,
        .timer_num       = BUZZER_TIMER,
        .duty_resolution = BUZZER_DUTY_RES,
        .freq_hz         = BUZZER_FREQ_HZ,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ESP_ERROR_CHECK(ledc_timer_config(&timer));

    ledc_channel_config_t channel = {
        .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel    = BUZZER_CHANNEL,
        .timer_sel  = BUZZER_TIMER,
        .intr_type  = LEDC_INTR_DISABLE,
        .gpio_num   = BUZZER_GPIO,
        .duty       = 0,
        .hpoint     = 0,
    };
    ESP_ERROR_CHECK(ledc_channel_config(&channel));
    ESP_LOGI(TAG, "Buzzer LEDC init OK");
    return ESP_OK;
}

/** Init SW1 (GPIO16) and SW2 (GPIO14) with ISR queue */
static void gpio_interrupt_init(void)
{
    gpio_evt_queue = xQueueCreate(10, sizeof(uint32_t));

    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << SW1_GPIO) | (1ULL << SW2_GPIO),
        .mode         = GPIO_MODE_INPUT,
        .pull_up_en   = GPIO_PULLUP_ENABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_NEGEDGE,   /* Falling edge = button press (active LOW) */
    };
    gpio_config(&io_conf);

    gpio_install_isr_service(ESP_INTR_FLAG_DEFAULT);
    gpio_isr_handler_add(SW1_GPIO, gpio_isr_handler, (void *)SW1_GPIO);
    gpio_isr_handler_add(SW2_GPIO, gpio_isr_handler, (void *)SW2_GPIO);
    ESP_LOGI(TAG, "Button ISR init OK (SW1=GPIO%d, SW2=GPIO%d)", SW1_GPIO, SW2_GPIO);
}

/* ══════════════════════════════════════════════════════════════════════════════
   SECTION 8 — app_main (Entry Point)
   ══════════════════════════════════════════════════════════════════════════════ */
void app_main(void)
{
    ESP_LOGI(TAG, "KidBright32 iA — All Sensors Demo (ESP-IDF v5.x)");

    /* ── 1. I2C Bus 0: HT16K33 + KXTJ3 ──────────────────────────────────── */
    ESP_ERROR_CHECK(i2c_bus0_init());
    ESP_ERROR_CHECK(matrix_init());
    ESP_ERROR_CHECK(kxtj3_init());

    /* ── 2. I2C Bus 1: LM73 Temperature ─────────────────────────────────── */
    ESP_ERROR_CHECK(temp_sensor_init());

    /* ── 3. ADC: LDR + IN1 ───────────────────────────────────────────────── */
    ESP_ERROR_CHECK(adc_init_all());

    /* ── 4. Buzzer ───────────────────────────────────────────────────────── */
    ESP_ERROR_CHECK(buzzer_init());

    /* ── 5. Buttons (ISR) ────────────────────────────────────────────────── */
    gpio_interrupt_init();

    /* ── 6. Startup animation ────────────────────────────────────────────── */
    display_pattern(PATTERN_HEART);
    buzzer_play(1047);                  /* C6 note */
    vTaskDelay(pdMS_TO_TICKS(300));
    buzzer_stop();
    vTaskDelay(pdMS_TO_TICKS(500));
    display_pattern(PATTERN_CHECK);
    vTaskDelay(pdMS_TO_TICKS(1000));

    /* ── 7. Launch FreeRTOS tasks ─────────────────────────────────────────── */
    xTaskCreate(button_task,  "btn_task",    4096, NULL, 10, NULL);
    xTaskCreate(sensor_task,  "sensor_task", 8192, NULL,  5, NULL);

    ESP_LOGI(TAG, "All tasks started.");
}
