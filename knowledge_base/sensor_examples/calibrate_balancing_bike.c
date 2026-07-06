/**
 * @file self_balance.c
 * @brief Self-balancing Reaction Wheel — MPU6050 + L298N
 *        ESP-IDF v5.x, KidBright Minibike Extension v0.3
 *
 * I2C: SDA=GPIO4, SCL=GPIO5 (MPU6050 @ 0x68)
 * Motor: IN1=GPIO12, IN2=GPIO23, ENA=GPIO26
 */

#include <stdio.h>
#include <math.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c.h"
#include "driver/ledc.h"
#include "driver/gpio.h"
#include "esp_log.h"

static const char *TAG = "SELF_BALANCE";

/* ─── I2C / MPU6050 ─────────────────────────────────────────────────────── */
#define I2C_BUS          I2C_NUM_0
#define I2C_SDA          GPIO_NUM_4
#define I2C_SCL          GPIO_NUM_5
#define I2C_FREQ_HZ      400000
#define MPU6050_ADDR     0x68
#define MPU6050_PWR_MGMT 0x6B
#define MPU6050_ACCEL    0x3B

/* ─── Motor Pins ─────────────────────────────────────────────────────────── */
#define PIN_IN1          GPIO_NUM_12
#define PIN_IN2          GPIO_NUM_23
#define PIN_ENA          GPIO_NUM_26

/* ─── LEDC (PWM) ─────────────────────────────────────────────────────────── */
#define LEDC_TIMER       LEDC_TIMER_0
#define LEDC_MODE        LEDC_LOW_SPEED_MODE
#define LEDC_CHANNEL     LEDC_CHANNEL_0
#define LEDC_FREQ_HZ     1000
#define LEDC_RESOLUTION  LEDC_TIMER_8_BIT   /* 0–255 */

/* ─── PID Parameters ─────────────────────────────────────────────────────── */
#define KP               15.0f
#define KI               0.0f
#define KD               0.8f
#define SETPOINT         -1.0f
#define INTEGRAL_LIMIT   50.0f
#define DEADZONE         5
#define LOOP_MS          50

/* ─── I2C helpers ────────────────────────────────────────────────────────── */
static esp_err_t mpu_write(uint8_t reg, uint8_t val)
{
    uint8_t buf[2] = { reg, val };
    return i2c_master_write_to_device(I2C_BUS, MPU6050_ADDR,
                                      buf, 2, pdMS_TO_TICKS(100));
}

static esp_err_t mpu_read(uint8_t reg, uint8_t *out, size_t len)
{
    return i2c_master_write_read_device(I2C_BUS, MPU6050_ADDR,
                                        &reg, 1, out, len,
                                        pdMS_TO_TICKS(100));
}

/* ─── Init ───────────────────────────────────────────────────────────────── */
static void i2c_init(void)
{
    i2c_config_t cfg = {
        .mode             = I2C_MODE_MASTER,
        .sda_io_num       = I2C_SDA,
        .scl_io_num       = I2C_SCL,
        .sda_pullup_en    = GPIO_PULLUP_ENABLE,
        .scl_pullup_en    = GPIO_PULLUP_ENABLE,
        .master.clk_speed = I2C_FREQ_HZ,
    };
    ESP_ERROR_CHECK(i2c_param_config(I2C_BUS, &cfg));
    ESP_ERROR_CHECK(i2c_driver_install(I2C_BUS, I2C_MODE_MASTER, 0, 0, 0));
}

static void mpu6050_init(void)
{
    ESP_ERROR_CHECK(mpu_write(MPU6050_PWR_MGMT, 0x00)); /* wake up */
    ESP_LOGI(TAG, "MPU6050 ready");
}

static void motor_init(void)
{
    /* GPIO for direction */
    gpio_config_t io = {
        .pin_bit_mask = (1ULL << PIN_IN1) | (1ULL << PIN_IN2),
        .mode         = GPIO_MODE_OUTPUT,
        .pull_up_en   = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type    = GPIO_INTR_DISABLE,
    };
    ESP_ERROR_CHECK(gpio_config(&io));
    gpio_set_level(PIN_IN1, 0);
    gpio_set_level(PIN_IN2, 0);

    /* LEDC for PWM on ENA */
    ledc_timer_config_t timer = {
        .speed_mode      = LEDC_MODE,
        .timer_num       = LEDC_TIMER,
        .duty_resolution = LEDC_RESOLUTION,
        .freq_hz         = LEDC_FREQ_HZ,
        .clk_cfg         = LEDC_AUTO_CLK,
    };
    ESP_ERROR_CHECK(ledc_timer_config(&timer));

    ledc_channel_config_t ch = {
        .gpio_num   = PIN_ENA,
        .speed_mode = LEDC_MODE,
        .channel    = LEDC_CHANNEL,
        .timer_sel  = LEDC_TIMER,
        .duty       = 0,
        .hpoint     = 0,
    };
    ESP_ERROR_CHECK(ledc_channel_config(&ch));
    ESP_LOGI(TAG, "Motor ready");
}

/* ─── Motor drive ────────────────────────────────────────────────────────── */
static void set_pwm(uint32_t duty)
{
    ledc_set_duty(LEDC_MODE, LEDC_CHANNEL, duty);
    ledc_update_duty(LEDC_MODE, LEDC_CHANNEL);
}

static void drive_motor(float pid_value)
{
    int speed = (int)fabsf(pid_value);
    if (speed > 255) speed = 255;

    if (speed < DEADZONE) {
        set_pwm(0);
        gpio_set_level(PIN_IN1, 0);
        gpio_set_level(PIN_IN2, 0);
        return;
    }

    /* หยุดก่อนสลับทิศ */
    set_pwm(0);
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
    set_pwm((uint32_t)speed);
    vTaskDelay(pdMS_TO_TICKS(20));
}

/* ─── Main task ──────────────────────────────────────────────────────────── */
void balance_task(void *pv)
{
    float error = 0, prev_error = 0;
    float integral = 0, derivative = 0;
    float output = 0;

    while (1) {
        /* Read accelerometer */
        uint8_t raw[6];
        if (mpu_read(MPU6050_ACCEL, raw, 6) == ESP_OK) {
            int16_t accY = (int16_t)((raw[2] << 8) | raw[3]);
            int16_t accZ = (int16_t)((raw[4] << 8) | raw[5]);

            float angle = atan2f((float)accY, (float)accZ) * 180.0f / M_PI;

            /* PID */
            error      = SETPOINT - angle;
            integral  += error;
            if (integral >  INTEGRAL_LIMIT) integral =  INTEGRAL_LIMIT;
            if (integral < -INTEGRAL_LIMIT) integral = -INTEGRAL_LIMIT;
            derivative = error - prev_error;
            output     = (KP * error) + (KI * integral) + (KD * derivative);
            prev_error = error;

            drive_motor(output);

            ESP_LOGI(TAG, "Angle:%.2f  PID:%.2f", angle, output);
        }

        vTaskDelay(pdMS_TO_TICKS(LOOP_MS));
    }
}

/* ─── Entry point ────────────────────────────────────────────────────────── */
void app_main(void)
{
    i2c_init();
    mpu6050_init();
    motor_init();
    xTaskCreate(balance_task, "balance", 4096, NULL, 5, NULL);
}