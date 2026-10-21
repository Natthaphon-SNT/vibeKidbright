/* Classic v1.3 matrix wiring only; physical orientation remains to be checked.
 * New 5x7 digits, not a reproduction of KidBright's text/scroll/font engine. */
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "kb_board.h"
#include "kb_basic.h"
#include "kb_display_idf.h"

void app_main(void) {
    i2c_master_bus_config_t config = {
        .i2c_port = I2C_NUM_0, .sda_io_num = KB_MATRIX_SDA,
        .scl_io_num = KB_MATRIX_SCL, .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7, .flags.enable_internal_pullup = true
    };
    i2c_master_bus_handle_t bus;
    i2c_master_dev_handle_t display;
    ESP_ERROR_CHECK(i2c_new_master_bus(&config, &bus));
    ESP_ERROR_CHECK(kb_display_open(bus, KB_MATRIX_ADDRESS, &display));
    kb_matrix_t frame = {0};
    uint32_t period = 0;
    if (!kb_music_duration(120, 1, &period)) return;
    for (unsigned value = 0;; value = (value + 1) % 100) {
        kb_matrix_clear(&frame);
        kb_matrix_digit(&frame, value / 10, 2, 0);
        kb_matrix_digit(&frame, value % 10, 9, 0);
        ESP_ERROR_CHECK(kb_display_write(display, &frame));
        vTaskDelay(pdMS_TO_TICKS(period) ? pdMS_TO_TICKS(period) : 1);
    }
}
