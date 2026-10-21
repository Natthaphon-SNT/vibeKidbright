/*
 * Balance Car - ESP-IDF C port
 * Converted from Arduino sketch
 */

#include <stdio.h>
#include <string.h>
#include <math.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "driver/i2c.h"
#include "esp_wifi.h"
#include "esp_now.h"
#include "esp_event.h"
#include "nvs_flash.h"
#include "esp_log.h"

static const char *TAG = "balance_car";

// ── I2C / MPU6050 ────────────────────────────────────────────────────────
#define MPU_ADDR        0x68
#define MPU_SDA         4
#define MPU_SCL         5
#define I2C_PORT        I2C_NUM_0
#define I2C_FREQ_HZ     400000

// ── Reaction Wheel ────────────────────────────────────────────────────────
#define PIN_IN1         12
#define PIN_IN2         23
#define PIN_ENA         26

// ── Rear Wheel ────────────────────────────────────────────────────────────
#define PIN_IN3         18
#define PIN_IN4         19
#define PIN_ENB         27

#define TURN_SPEED      150
#define SERVO_PIN       15

// ── LEDC channels ────────────────────────────────────────────────────────
// Motor A (reaction wheel)
#define LEDC_CH_A       LEDC_CHANNEL_0
// Motor B (rear wheel)
#define LEDC_CH_B       LEDC_CHANNEL_1
// Servo
#define LEDC_CH_SERVO   LEDC_CHANNEL_2
#define LEDC_TIMER_MOT  LEDC_TIMER_0
#define LEDC_TIMER_SRV  LEDC_TIMER_1
#define LEDC_SPEED      LEDC_LOW_SPEED_MODE

// ── PID ──────────────────────────────────────────────────────────────────
static float Kp = 15.0f, Ki = 0.0f, Kd = 0.8f;
static float error = 0, prev_error = 0;
static float integral = 0, derivative = 0, output = 0;
static const float INTEGRAL_LIMIT = 50.0f;
static const int   DEADZONE       = 5;

#define SETPOINT_BASE   -1.0f
#define SETPOINT_MAX     8.0f

// ── ESP-NOW ───────────────────────────────────────────────────────────────
static volatile int32_t g_cmd = 999;

static void on_recv(const esp_now_recv_info_t *info, const uint8_t *data, int len)
{
    if (len == sizeof(int32_t)) {
        memcpy((void *)&g_cmd, data, sizeof(int32_t));
    }
}

// ── PWM helpers ───────────────────────────────────────────────────────────
static void pwm_motor_init(void)
{
    // Timer for motors (5 kHz, 8-bit → duty 0–255)
    ledc_timer_config_t mot_timer = {
        .speed_mode      = LEDC_SPEED,
        .timer_num       = LEDC_TIMER_MOT,
        .duty_resolution = LEDC_TIMER_8_BIT,
        .freq_hz         = 5000,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&mot_timer);

    // Channel A – reaction wheel
    ledc_channel_config_t ch_a = {
        .speed_mode = LEDC_SPEED,
        .channel    = LEDC_CH_A,
        .timer_sel  = LEDC_TIMER_MOT,
        .intr_type  = LEDC_INTR_DISABLE,
        .gpio_num   = PIN_ENA,
        .duty       = 0,
        .hpoint     = 0,
    };
    ledc_channel_config(&ch_a);

    // Channel B – rear wheel
    ledc_channel_config_t ch_b = {
        .speed_mode = LEDC_SPEED,
        .channel    = LEDC_CH_B,
        .timer_sel  = LEDC_TIMER_MOT,
        .intr_type  = LEDC_INTR_DISABLE,
        .gpio_num   = PIN_ENB,
        .duty       = 0,
        .hpoint     = 0,
    };
    ledc_channel_config(&ch_b);
}

static void pwm_servo_init(void)
{
    // Timer for servo (50 Hz, 16-bit → 0–65535 maps to 0–20ms)
    ledc_timer_config_t srv_timer = {
        .speed_mode      = LEDC_SPEED,
        .timer_num       = LEDC_TIMER_SRV,
        .duty_resolution = LEDC_TIMER_16_BIT,
        .freq_hz         = 50,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ledc_timer_config(&srv_timer);

    ledc_channel_config_t ch_srv = {
        .speed_mode = LEDC_SPEED,
        .channel    = LEDC_CH_SERVO,
        .timer_sel  = LEDC_TIMER_SRV,
        .intr_type  = LEDC_INTR_DISABLE,
        .gpio_num   = SERVO_PIN,
        .duty       = 0,
        .hpoint     = 0,
    };
    ledc_channel_config(&ch_srv);
}

// Convert 0–255 → LEDC 8-bit duty
static void analog_write_motor(ledc_channel_t ch, int value)
{
    if (value < 0) value = 0;
    if (value > 255) value = 255;
    ledc_set_duty(LEDC_SPEED, ch, (uint32_t)value);
    ledc_update_duty(LEDC_SPEED, ch);
}

// Convert servo angle (0–180°) to 16-bit LEDC duty
// 500 µs (0°) – 2400 µs (180°)  at 50 Hz (period = 20 000 µs)
static void servo_write(int angle)
{
    if (angle < 0)   angle = 0;
    if (angle > 180) angle = 180;
    // pulse width µs: 500 + angle*(1900/180)
    float pulse_us = 500.0f + (float)angle * (1900.0f / 180.0f);
    uint32_t duty  = (uint32_t)(pulse_us / 20000.0f * 65536.0f);
    ledc_set_duty(LEDC_SPEED, LEDC_CH_SERVO, duty);
    ledc_update_duty(LEDC_SPEED, LEDC_CH_SERVO);
}

// ── GPIO init ─────────────────────────────────────────────────────────────
static void gpio_motor_init(void)
{
    gpio_config_t io = {
        .pin_bit_mask = (1ULL << PIN_IN1) | (1ULL << PIN_IN2) |
                        (1ULL << PIN_IN3) | (1ULL << PIN_IN4),
        .mode         = GPIO_MODE_OUTPUT,
        .pull_up_en   = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_DISABLE,
    };
    gpio_config(&io);
    gpio_set_level(PIN_IN1, 0);
    gpio_set_level(PIN_IN2, 0);
    gpio_set_level(PIN_IN3, 0);
    gpio_set_level(PIN_IN4, 0);
}

// ── MPU6050 ───────────────────────────────────────────────────────────────
static void mpu_init(void)
{
    i2c_config_t conf = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = MPU_SDA,
        .scl_io_num       = MPU_SCL,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C_FREQ_HZ,
    };
    i2c_param_config(I2C_PORT, &conf);
    i2c_driver_install(I2C_PORT, conf.mode, 0, 0, 0);

    // Wake MPU6050 (write 0x00 to PWR_MGMT_1 = 0x6B)
    uint8_t data[2] = {0x6B, 0x00};
    i2c_master_write_to_device(I2C_PORT, MPU_ADDR, data, sizeof(data), pdMS_TO_TICKS(100));
    ESP_LOGI(TAG, "MPU6050 ready");
}

static float mpu_read_angle(void)
{
    uint8_t reg = 0x3B;
    uint8_t buf[6];
    esp_err_t err = i2c_master_write_read_device(
        I2C_PORT, MPU_ADDR, &reg, 1, buf, 6, pdMS_TO_TICKS(100));
    if (err != ESP_OK) return 0.0f;

    int16_t accX = (int16_t)((buf[0] << 8) | buf[1]);
    int16_t accY = (int16_t)((buf[2] << 8) | buf[3]);
    int16_t accZ = (int16_t)((buf[4] << 8) | buf[5]);
    return atan2f((float)accY, (float)accZ) * 180.0f / (float)M_PI;
}

// ── Motor drive functions ─────────────────────────────────────────────────
static void drive_reaction_wheel(float pid_value)
{
    int speed = (int)fabsf(pid_value);
    if (speed > 255) speed = 255;

    if (speed < DEADZONE) {
        analog_write_motor(LEDC_CH_A, 0);
        gpio_set_level(PIN_IN1, 0);
        gpio_set_level(PIN_IN2, 0);
        return;
    }

    // Brief coast before direction change (mirrors Arduino delay(20) pattern)
    analog_write_motor(LEDC_CH_A, 0);
    gpio_set_level(PIN_IN1, 0);
    gpio_set_level(PIN_IN2, 0);
    vTaskDelay(pdMS_TO_TICKS(20));

    if (pid_value > 0) {
        gpio_set_level(PIN_IN1, 1);
        gpio_set_level(PIN_IN2, 0);
    } else {
        gpio_set_level(PIN_IN1, 0);
        gpio_set_level(PIN_IN2, 1);
    }
    analog_write_motor(LEDC_CH_A, speed);
    vTaskDelay(pdMS_TO_TICKS(20));
}

static void drive_steer(int turn)
{
    if      (turn > 0) servo_write(60);
    else if (turn < 0) servo_write(120);
    else               servo_write(90);
}

static void drive_rear_wheel(int fwd)
{
    if (fwd > 0) {
        gpio_set_level(PIN_IN3, 0);
        gpio_set_level(PIN_IN4, 1);
        analog_write_motor(LEDC_CH_B, TURN_SPEED);
    } else if (fwd < 0) {
        gpio_set_level(PIN_IN3, 1);
        gpio_set_level(PIN_IN4, 0);
        analog_write_motor(LEDC_CH_B, TURN_SPEED);
    } else {
        analog_write_motor(LEDC_CH_B, 0);
        gpio_set_level(PIN_IN3, 0);
        gpio_set_level(PIN_IN4, 0);
    }
}

// ── ESP-NOW / WiFi init ───────────────────────────────────────────────────
static void espnow_init(void)
{
    ESP_ERROR_CHECK(nvs_flash_init());
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_start());
    ESP_ERROR_CHECK(esp_wifi_disconnect());

    uint8_t mac[6];
    esp_wifi_get_mac(WIFI_IF_STA, mac);
    ESP_LOGI(TAG, "Car MAC: %02X:%02X:%02X:%02X:%02X:%02X",
             mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);

    ESP_ERROR_CHECK(esp_now_init());
    ESP_ERROR_CHECK(esp_now_register_recv_cb(on_recv));
    ESP_LOGI(TAG, "Car ready");
}

// ── Main task (loop equivalent) ───────────────────────────────────────────
static void main_task(void *arg)
{
    for (;;) {
        int32_t cmd = g_cmd;
        float sp  = SETPOINT_BASE;
        int turn  = 0;
        int fwd   = 0;

        if      (cmd >= 1   && cmd <= 100)  { fwd =  1; sp = SETPOINT_BASE + ((float)cmd  / 100.0f) * SETPOINT_MAX; }
        else if (cmd >= -100 && cmd <= -1)  { fwd = -1; sp = SETPOINT_BASE + ((float)cmd  / 100.0f) * SETPOINT_MAX; }
        else if (cmd >= 401 && cmd <= 500)  { turn =  1; }
        else if (cmd >= 300 && cmd <= 399)  { turn = -1; }

        drive_rear_wheel(fwd);
        drive_steer(turn);

        float angle = mpu_read_angle();
        error      = sp - angle;
        integral  += error;
        if (integral >  INTEGRAL_LIMIT) integral =  INTEGRAL_LIMIT;
        if (integral < -INTEGRAL_LIMIT) integral = -INTEGRAL_LIMIT;
        derivative = error - prev_error;
        output     = (Kp * error) + (Ki * integral) + (Kd * derivative);
        prev_error = error;

        drive_reaction_wheel(output);

        ESP_LOGI(TAG, "Angle:%.2f SP:%.2f PID:%.2f CMD:%ld",
                 angle, sp, output, (long)cmd);

        vTaskDelay(pdMS_TO_TICKS(50));
    }
}

// ── Entry point ───────────────────────────────────────────────────────────
void app_main(void)
{
    gpio_motor_init();
    pwm_motor_init();
    pwm_servo_init();
    servo_write(90);   // center servo

    mpu_init();
    espnow_init();

    xTaskCreate(main_task, "main_task", 4096, NULL, 5, NULL);
}