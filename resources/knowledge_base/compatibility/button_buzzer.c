/* KidBright behavior: forever -> wait S1 pressed -> A4 quarter note at
 * 120 BPM (440 Hz, 500 ms) -> wait S1 released. Board profile is explicit.
 * Equivalent logic, pending physical waveform/button testing. */
#include <stdint.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "esp_err.h"
#include "kb_board.h"
#include "kb_core.h"

void app_main(void) {
    gpio_config_t button = {
        .pin_bit_mask = 1ULL << KB_SW1_GPIO,
        .mode = GPIO_MODE_INPUT, .pull_up_en = GPIO_PULLUP_ENABLE
    };
    ESP_ERROR_CHECK(gpio_config(&button));
    ledc_timer_config_t timer = {
        .speed_mode = LEDC_LOW_SPEED_MODE, .duty_resolution = LEDC_TIMER_10_BIT,
        .timer_num = LEDC_TIMER_0, .freq_hz = 440, .clk_cfg = LEDC_AUTO_CLK
    };
    ESP_ERROR_CHECK(ledc_timer_config(&timer));
    ledc_channel_config_t buzzer = {
        .gpio_num = KB_BUZZER_GPIO, .speed_mode = LEDC_LOW_SPEED_MODE,
        .channel = LEDC_CHANNEL_0, .timer_sel = LEDC_TIMER_0, .duty = 0
    };
    ESP_ERROR_CHECK(ledc_channel_config(&buzzer));
    uint32_t note_ms = 0;
    if (!kb_delay_ms(0.5, &note_ms)) return;
    for (;;) {
        while (gpio_get_level(KB_SW1_GPIO) != 0) vTaskDelay(pdMS_TO_TICKS(40));
        ESP_ERROR_CHECK(ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 512));
        ESP_ERROR_CHECK(ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0));
        vTaskDelay(pdMS_TO_TICKS(note_ms));
        ESP_ERROR_CHECK(ledc_stop(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 0));
        while (gpio_get_level(KB_SW1_GPIO) == 0) vTaskDelay(pdMS_TO_TICKS(40));
    }
}
