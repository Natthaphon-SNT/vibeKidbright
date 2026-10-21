/* S1 press -> C4..C5 at 120 BPM -> wait release. New generalized note API.
 * LEDC_TIMER_0/CHANNEL_0 are owned by this program. */
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/gpio.h"
#include "driver/ledc.h"
#include "esp_err.h"
#include "kb_board.h"
#include "kb_basic.h"

void app_main(void) {
    gpio_config_t button = {.pin_bit_mask = 1ULL << KB_SW1_GPIO,
        .mode = GPIO_MODE_INPUT, .pull_up_en = GPIO_PULLUP_ENABLE};
    ESP_ERROR_CHECK(gpio_config(&button));
    ledc_timer_config_t timer = {.speed_mode = LEDC_LOW_SPEED_MODE,
        .duty_resolution = LEDC_TIMER_10_BIT, .timer_num = LEDC_TIMER_0,
        .freq_hz = 262, .clk_cfg = LEDC_AUTO_CLK};
    ESP_ERROR_CHECK(ledc_timer_config(&timer));
    ledc_channel_config_t output = {.gpio_num = KB_BUZZER_GPIO,
        .speed_mode = LEDC_LOW_SPEED_MODE, .channel = LEDC_CHANNEL_0,
        .timer_sel = LEDC_TIMER_0, .duty = 0};
    ESP_ERROR_CHECK(ledc_channel_config(&output));
    const unsigned notes[] = {60, 62, 64, 65, 67, 69, 71, 72};
    uint32_t milliseconds = 0;
    if (!kb_music_duration(120, 1, &milliseconds)) return;
    for (;;) {
        while (gpio_get_level(KB_SW1_GPIO)) vTaskDelay(pdMS_TO_TICKS(20) + 1);
        for (unsigned i = 0; i < sizeof(notes) / sizeof(notes[0]); ++i) {
            uint32_t frequency = 0;
            if (!kb_music_frequency(notes[i], &frequency)) return;
            if (!ledc_set_freq(LEDC_LOW_SPEED_MODE, LEDC_TIMER_0, frequency)) return;
            ESP_ERROR_CHECK(ledc_set_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 512));
            ESP_ERROR_CHECK(ledc_update_duty(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0));
            vTaskDelay(pdMS_TO_TICKS(milliseconds) ? pdMS_TO_TICKS(milliseconds) : 1);
            ESP_ERROR_CHECK(ledc_stop(LEDC_LOW_SPEED_MODE, LEDC_CHANNEL_0, 0));
        }
        while (!gpio_get_level(KB_SW1_GPIO)) vTaskDelay(pdMS_TO_TICKS(20) + 1);
    }
}
