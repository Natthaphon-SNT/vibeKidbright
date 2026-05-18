    /**
 * @file main2.c
 * @brief Formula Kid — RECEIVER (ฝั่งรถ)
 *
 * รับค่า ESP-NOW จาก Controller และสั่ง DRV8833 motor driver
 * ตรงตาม Logic บล็อก KidBright:
 *
 *   ESPNOW_VALUE == 999          → หยุด  (LED "--")
 *   -100 <= value <= -10         → ถอยหลัง (speed = |value|%, LED "D")
 *     10 <= value <= 100         → เดินหน้า (speed = value%,  LED "U")
 *   300 <= value <= 500          → เลี้ยว   (value -= 400)
 *       value ∈ [-100..-10]      → เลี้ยวซ้าย  (LED "L")
 *       value ∈ [10..100]        → เลี้ยวขวา   (LED "R")
 *
 * Hardware: KidBright32 (ESP32) + DRV8833 + HT16K33 LED 16x8
 *
 * GPIO Map (verified by hardware test):
 *   GPIO18 = ขวา-เดินหน้า  (J07 A-forward)
 *   GPIO26 = ขวา-ถอยหลัง  (J07 A-backward)
 *   GPIO19 = ซ้าย-เดินหน้า (J06 B-forward)
 *   GPIO27 = ซ้าย-ถอยหลัง (J06 B-backward)
 */

#include <stdio.h>
#include <string.h>
#include <math.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "driver/i2c.h"
#include "esp_wifi.h"
#include "esp_now.h"
#include "nvs_flash.h"

#define TAG "FORMULA_KID_RECEIVER"

// ── I2C / LED Matrix ──────────────────────────────────────────────────────────
#define I2C_SDA_GPIO    GPIO_NUM_21
#define I2C_SCL_GPIO    GPIO_NUM_22
#define HT16K33_ADDR    0x70

// ── DRV8833 GPIO Mapping (verified) ──────────────────────────────────────────
#define DRV_NSLEEP      GPIO_NUM_23   // HIGH = chip active

#define MOTOR_R_FWD     GPIO_NUM_18   // ขวา เดินหน้า
#define MOTOR_R_BWD     GPIO_NUM_26   // ขวา ถอยหลัง
#define MOTOR_L_FWD     GPIO_NUM_19   // ซ้าย เดินหน้า
#define MOTOR_L_BWD     GPIO_NUM_27   // ซ้าย ถอยหลัง

// ── LEDC PWM Config ───────────────────────────────────────────────────────────
#define LEDC_TIMER      LEDC_TIMER_0
#define LEDC_MODE       LEDC_LOW_SPEED_MODE
#define LEDC_FREQ_HZ    5000
#define LEDC_RES        LEDC_TIMER_8_BIT   // 0-255

#define CH_R_FWD        LEDC_CHANNEL_0
#define CH_R_BWD        LEDC_CHANNEL_1
#define CH_L_FWD        LEDC_CHANNEL_2
#define CH_L_BWD        LEDC_CHANNEL_3

// ── ESP-NOW ───────────────────────────────────────────────────────────────────
#define ESPNOW_CHANNEL  1

// ── LED Matrix patterns ───────────────────────────────────────────────────────
static const uint8_t img_up[16]    = {0x00,0x00,0xFF,0xFF,0x01,0x01,0x01,0x01,0x01,0x01,0x01,0x01,0xFF,0xFF,0x00,0x00};
static const uint8_t img_down[16]  = {0x00,0x00,0x00,0xFF,0xFF,0x81,0x81,0x81,0x81,0x81,0x81,0x7E,0x3C,0x00,0x00,0x00};
static const uint8_t img_left[16]  = {0x00,0x00,0x00,0xFF,0xFF,0x01,0x01,0x01,0x01,0x01,0x01,0x01,0x01,0x00,0x00,0x00};
static const uint8_t img_right[16] = {0x00,0x00,0x00,0x00,0xFF,0xFF,0x90,0x90,0x98,0x94,0x62,0x01,0x00,0x00,0x00,0x00};
static const uint8_t img_stop[16]  = {0x00,0x00,0x18,0x18,0x18,0x18,0x00,0x00,0x00,0x00,0x18,0x18,0x18,0x18,0x00,0x00};

// ── Shared state ──────────────────────────────────────────────────────────────
static volatile int32_t g_espnow_value = 999;
static volatile bool    g_new_data     = false;

// ─────────────────────────────────────────────────────────────────────────────
// I2C + LED Matrix
// ─────────────────────────────────────────────────────────────────────────────
static void i2c_init(void) {
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = I2C_SDA_GPIO,
        .scl_io_num       = I2C_SCL_GPIO,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = 100000,
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_NUM_0, &conf));
    ESP_ERROR_CHECK(i2c_driver_install(I2C_NUM_0, conf.mode, 0, 0, 0));
}

static void matrix_init(void) {
    uint8_t osc_on[]     = {0x21};
    uint8_t disp_on[]    = {0x81};
    uint8_t brightness[] = {0xEF};
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, osc_on,     1, pdMS_TO_TICKS(100));
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, disp_on,    1, pdMS_TO_TICKS(100));
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, brightness,  1, pdMS_TO_TICKS(100));
}

static void matrix_draw(const uint8_t cols[16]) {
    uint8_t buf[17] = {0};
    buf[0] = 0x00;
    for (int c = 0; c < 8; c++) {
        buf[1 + c * 2] = cols[c];
        buf[2 + c * 2] = cols[c + 8];
    }
    i2c_master_write_to_device(I2C_NUM_0, HT16K33_ADDR, buf, sizeof(buf), pdMS_TO_TICKS(100));
}

// ─────────────────────────────────────────────────────────────────────────────
// Motor Driver
// ─────────────────────────────────────────────────────────────────────────────
static void motor_init(void) {
    // NSLEEP
    gpio_config_t io_conf = {
        .pin_bit_mask = (1ULL << DRV_NSLEEP),
        .mode         = GPIO_MODE_OUTPUT,
    };
    gpio_config(&io_conf);
    gpio_set_level(DRV_NSLEEP, 1);

    // LEDC timer
    ledc_timer_config_t timer = {
        .speed_mode      = LEDC_MODE,
        .duty_resolution = LEDC_RES,
        .timer_num       = LEDC_TIMER,
        .freq_hz         = LEDC_FREQ_HZ,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ESP_ERROR_CHECK(ledc_timer_config(&timer));

    // 4 channels: R_FWD, R_BWD, L_FWD, L_BWD
    ledc_channel_config_t chs[] = {
        {.gpio_num=GPIO_NUM_18, .channel=CH_R_FWD, .speed_mode=LEDC_MODE, .timer_sel=LEDC_TIMER, .duty=0, .hpoint=0},
        {.gpio_num=GPIO_NUM_26, .channel=CH_R_BWD, .speed_mode=LEDC_MODE, .timer_sel=LEDC_TIMER, .duty=0, .hpoint=0},
        {.gpio_num=GPIO_NUM_19, .channel=CH_L_FWD, .speed_mode=LEDC_MODE, .timer_sel=LEDC_TIMER, .duty=0, .hpoint=0},
        {.gpio_num=GPIO_NUM_27, .channel=CH_L_BWD, .speed_mode=LEDC_MODE, .timer_sel=LEDC_TIMER, .duty=0, .hpoint=0},
    };
    for (int i = 0; i < 4; i++) {
        ESP_ERROR_CHECK(ledc_channel_config(&chs[i]));
    }
    ESP_LOGI(TAG, "CH_R_FWD→GPIO%d, CH_R_BWD→GPIO%d, CH_L_FWD→GPIO%d, CH_L_BWD→GPIO%d",
         MOTOR_R_FWD, MOTOR_R_BWD, MOTOR_L_FWD, MOTOR_L_BWD);
}

static uint32_t pct_to_duty(int pct) {
    if (pct <= 0)   return 0;
    if (pct > 100)  pct = 100;
    return (uint32_t)(pct * 255 / 100);
}

// ตั้งค่า duty ทั้ง 4 channel: R_FWD, R_BWD, L_FWD, L_BWD
static void motor_set(uint32_t rf, uint32_t rb, uint32_t lf, uint32_t lb) {
    ledc_set_duty(LEDC_MODE, CH_R_FWD, rf); ledc_update_duty(LEDC_MODE, CH_R_FWD);
    ledc_set_duty(LEDC_MODE, CH_R_BWD, rb); ledc_update_duty(LEDC_MODE, CH_R_BWD);
    ledc_set_duty(LEDC_MODE, CH_L_FWD, lf); ledc_update_duty(LEDC_MODE, CH_L_FWD);
    ledc_set_duty(LEDC_MODE, CH_L_BWD, lb); ledc_update_duty(LEDC_MODE, CH_L_BWD);
}

// หยุด: ทุก channel = 0
static void motor_stop(void) {
    motor_set(0, 255, 0, 255);
}

// เดินหน้า: R_FWD=d, L_FWD=d, ส่วน BWD=0
static void motor_forward(int speed_pct) {
    uint32_t d = pct_to_duty(speed_pct);
    ESP_LOGI(TAG, "forward: RF=%lu RB=%lu LF=%lu LB=%lu", d, 0UL, d, 0UL);
    motor_set(0, 0, 0, 0);
}

// ถอยหลัง: R_BWD=d, L_BWD=d, ส่วน FWD=0
static void motor_backward(int speed_pct) {
    uint32_t d = pct_to_duty(speed_pct);
    motor_set(255, 255, 255, 255);
}

// เลี้ยวซ้าย: ขับเฉพาะล้อขวาไปข้างหน้า (R_FWD=d)
static void motor_turn_left(int speed_pct) {
    uint32_t d = pct_to_duty(speed_pct);
    motor_set(0, 0, d, 0);
}

// เลี้ยวขวา: ขับเฉพาะล้อซ้ายไปข้างหน้า (L_FWD=d)
static void motor_turn_right(int speed_pct) {
    uint32_t d = pct_to_duty(speed_pct);
    motor_set(d, 0, 0, 0);
}

// ─────────────────────────────────────────────────────────────────────────────
// WiFi + ESP-NOW
// ─────────────────────────────────────────────────────────────────────────────
static void espnow_recv_cb(const esp_now_recv_info_t *recv_info,
                           const uint8_t *data, int len) {
    if (len != sizeof(int32_t)) return;
    int32_t val;
    memcpy(&val, data, sizeof(int32_t));
    g_espnow_value = val;
    g_new_data     = true;
}

static void wifi_espnow_init(void) {
    ESP_ERROR_CHECK(nvs_flash_init());
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_start());
    ESP_ERROR_CHECK(esp_wifi_set_channel(ESPNOW_CHANNEL, WIFI_SECOND_CHAN_NONE));
    ESP_ERROR_CHECK(esp_now_init());
    ESP_ERROR_CHECK(esp_now_register_recv_cb(espnow_recv_cb));

    uint8_t mac[6];
    esp_wifi_get_mac(WIFI_IF_STA, mac);
    ESP_LOGI(TAG, "ESP-NOW ready | MAC: %02X:%02X:%02X:%02X:%02X:%02X",
             mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
}

// ─────────────────────────────────────────────────────────────────────────────
// app_main
// ─────────────────────────────────────────────────────────────────────────────
void app_main(void) {
    ESP_LOGI(TAG, "Starting Formula Kid Receiver...");

    i2c_init();
    matrix_init();
    matrix_draw(img_stop);

    motor_init();
    motor_stop();

    wifi_espnow_init();

    // // ── Self-test sequence ────────────────────────────────────────────────────
    // ESP_LOGI(TAG, "SELF-TEST: Forward 2s");
    // motor_forward(50);
    // matrix_draw(img_up);
    // vTaskDelay(pdMS_TO_TICKS(2000));

    // ESP_LOGI(TAG, "SELF-TEST: Stop 1s");
    // motor_stop();
    // matrix_draw(img_stop);
    // vTaskDelay(pdMS_TO_TICKS(1000));

    // ESP_LOGI(TAG, "SELF-TEST: Backward 2s");
    // motor_backward(50);
    // matrix_draw(img_down);
    // vTaskDelay(pdMS_TO_TICKS(2000));

    // ESP_LOGI(TAG, "SELF-TEST: Stop 1s");
    // motor_stop();
    // matrix_draw(img_stop);
    // vTaskDelay(pdMS_TO_TICKS(1000));

    // ESP_LOGI(TAG, "SELF-TEST: Turn Left 1.5s");
    // motor_turn_left(50);
    // matrix_draw(img_left);
    // vTaskDelay(pdMS_TO_TICKS(1500));

    // ESP_LOGI(TAG, "SELF-TEST: Stop 1s");
    // motor_stop();
    // matrix_draw(img_stop);
    // vTaskDelay(pdMS_TO_TICKS(1000));

    // ESP_LOGI(TAG, "SELF-TEST: Turn Right 1.5s");
    // motor_turn_right(50);
    // matrix_draw(img_right);
    // vTaskDelay(pdMS_TO_TICKS(1500));

    // motor_stop();
    // matrix_draw(img_stop);
    // ESP_LOGI(TAG, "SELF-TEST done. Entering main loop.");
    // // ─────────────────────────────────────────────────────────────────────────

    const uint8_t *prev_pattern = img_stop;
    int32_t        prev_value   = 999;

    while (1) {
        if (!g_new_data) {
            vTaskDelay(pdMS_TO_TICKS(10));
            continue;
        }
        g_new_data = false;

        int32_t val = g_espnow_value;
        const uint8_t *pattern = img_stop;
        const char    *label   = "STOP  [-]";

        if (val == 999) {
            motor_stop();
            pattern = img_stop;
            label   = "STOP  [-]";

        } else if (val >= -100 && val <= 100) {
            if (val <= -10) {
                motor_backward((int)abs((int)val));
                pattern = img_down;
                label   = "BACKWARD[D]";
            } else if (val >= 10) {
                motor_forward((int)val);
                pattern = img_up;
                label   = "FORWARD [U]";
            } else {
                motor_stop();
                pattern = img_stop;
                label   = "STOP  [-]";
            }

        } else if (val >= 300 && val <= 500) {
            int32_t js2 = val - 400;
            if (js2 <= -10) {
                motor_turn_left((int)abs((int)js2));
                pattern = img_left;
                label   = "LEFT    [L]";
            } else if (js2 >= 10) {
                motor_turn_right((int)js2);
                pattern = img_right;
                label   = "RIGHT   [R]";
            } else {
                motor_stop();
                pattern = img_stop;
                label   = "STOP  [-]";
            }

        } else {
            ESP_LOGW(TAG, "Unknown value: %ld", (long)val);
            motor_stop();
            pattern = img_stop;
            label   = "STOP  [-]";
        }

        if (pattern != prev_pattern) {
            matrix_draw(pattern);
            prev_pattern = pattern;
        }
        if (val != prev_value) {
            ESP_LOGI(TAG, "Dir: %-11s | val: %4ld", label, (long)val);
            prev_value = val;
        }

        vTaskDelay(pdMS_TO_TICKS(10));
    }
}