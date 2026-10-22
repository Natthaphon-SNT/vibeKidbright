#include <stdio.h>
#include <string.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/gpio.h"
#include "driver/i2c.h"
#include "esp_err.h"
#include "esp_log.h"

#define MATRIX_I2C_PORT I2C_NUM_0
#define MATRIX_SDA_GPIO GPIO_NUM_21
#define MATRIX_SCL_GPIO GPIO_NUM_22
#define MATRIX_I2C_ADDRESS 0x70
#define MATRIX_I2C_FREQUENCY_HZ 100000
#define MATRIX_TIMEOUT_MS 100
#define SCROLL_DELAY_MS 80

static const char *TAG = "matrix_az";

static const uint8_t FONT_5X7_UPPERCASE[26][5] = {
    {0x7E,0x11,0x11,0x11,0x7E},{0x7F,0x49,0x49,0x49,0x36},{0x3E,0x41,0x41,0x41,0x22},{0x7F,0x41,0x41,0x22,0x1C},
    {0x7F,0x49,0x49,0x49,0x41},{0x7F,0x09,0x09,0x09,0x01},{0x3E,0x41,0x49,0x49,0x7A},{0x7F,0x08,0x08,0x08,0x7F},
    {0x00,0x41,0x7F,0x41,0x00},{0x20,0x40,0x41,0x3F,0x01},{0x7F,0x08,0x14,0x22,0x41},{0x7F,0x40,0x40,0x40,0x40},
    {0x7F,0x02,0x0C,0x02,0x7F},{0x7F,0x04,0x08,0x10,0x7F},{0x3E,0x41,0x41,0x41,0x3E},{0x7F,0x09,0x09,0x09,0x06},
    {0x3E,0x41,0x51,0x21,0x5E},{0x7F,0x09,0x19,0x29,0x46},{0x46,0x49,0x49,0x49,0x31},{0x01,0x01,0x7F,0x01,0x01},
    {0x3F,0x40,0x40,0x40,0x3F},{0x1F,0x20,0x40,0x20,0x1F},{0x3F,0x40,0x38,0x40,0x3F},{0x63,0x14,0x08,0x14,0x63},
    {0x07,0x08,0x70,0x08,0x07},{0x61,0x51,0x49,0x45,0x43},
};

static void i2c_init_bus0(void)
{
    i2c_config_t config = {
        .mode = I2C_MODE_MASTER,
        .sda_io_num = MATRIX_SDA_GPIO,
        .scl_io_num = MATRIX_SCL_GPIO,
        .sda_pullup_en = GPIO_PULLUP_ENABLE,
        .scl_pullup_en = GPIO_PULLUP_ENABLE,
        .master.clk_speed = MATRIX_I2C_FREQUENCY_HZ,
        .clk_flags = 0,
    };

    ESP_ERROR_CHECK(i2c_param_config(MATRIX_I2C_PORT, &config));
    ESP_ERROR_CHECK(i2c_driver_install(MATRIX_I2C_PORT, config.mode, 0, 0, 0));
}

static esp_err_t matrix_command(uint8_t command)
{
    return i2c_master_write_to_device(
        MATRIX_I2C_PORT,
        MATRIX_I2C_ADDRESS,
        &command,
        1,
        pdMS_TO_TICKS(MATRIX_TIMEOUT_MS));
}

static void matrix_init(void)
{
    esp_err_t ret = matrix_command(0x21);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Matrix oscillator command failed: %s", esp_err_to_name(ret));
    }

    vTaskDelay(pdMS_TO_TICKS(10));

    ret = matrix_command(0x81);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Matrix display command failed: %s", esp_err_to_name(ret));
    }

    ret = matrix_command(0xEF);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Matrix brightness command failed: %s", esp_err_to_name(ret));
    }
}

static void matrix_draw(const uint8_t cols[16])
{
    uint8_t buffer[17] = {0};
    buffer[0] = 0x00;

    for (int column = 0; column < 8; column++) {
        buffer[1 + (column * 2)] = cols[column];
        buffer[2 + (column * 2)] = cols[column + 8];
    }

    esp_err_t ret = i2c_master_write_to_device(
        MATRIX_I2C_PORT,
        MATRIX_I2C_ADDRESS,
        buffer,
        sizeof(buffer),
        pdMS_TO_TICKS(MATRIX_TIMEOUT_MS));
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Matrix draw failed: %s", esp_err_to_name(ret));
    }
}

static uint8_t reverse_bits(uint8_t value)
{
    value = (uint8_t)(((value & 0xF0U) >> 4) | ((value & 0x0FU) << 4));
    value = (uint8_t)(((value & 0xCCU) >> 2) | ((value & 0x33U) << 2));
    value = (uint8_t)(((value & 0xAAU) >> 1) | ((value & 0x55U) << 1));
    return value;
}

static uint8_t get_font_column(char character, int column)
{
    if (column < 0 || column >= 5) {
        return 0x00;
    }

    if (character >= 'A' && character <= 'Z') {
        return reverse_bits(FONT_5X7_UPPERCASE[character - 'A'][column]);
    }

    return 0x00;
}

static void matrix_scroll_text(const char *text)
{
    int total_width = (int)strlen(text) * 6;

    for (int x = 16; x > -total_width; x--) {
        uint8_t cols[16] = {0};

        for (int column = 0; column < 16; column++) {
            int text_x = column - x;
            if (text_x >= 0 && text_x < total_width) {
                cols[column] = get_font_column(text[text_x / 6], text_x % 6);
            }
        }

        matrix_draw(cols);
        vTaskDelay(pdMS_TO_TICKS(SCROLL_DELAY_MS));
    }
}

void app_main(void)
{
    i2c_init_bus0();
    matrix_init();

    while (1) {
        matrix_scroll_text("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    }
}
