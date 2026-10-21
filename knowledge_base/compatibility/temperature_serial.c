/* KidBright LM73 recipe: pointer 0x00, signed MSB-first raw /128 C.
 * Read every 500 ms. Faults are reported, never presented as 0 degrees.
 * Uses one I2C driver family (new master API, ESP-IDF 5.5). */
#include <stdint.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "esp_log.h"
#include "kb_board.h"
#include "kb_core.h"

void app_main(void) {
    i2c_master_bus_config_t config = {
        .i2c_port = I2C_NUM_1, .sda_io_num = KB_SENSOR_SDA,
        .scl_io_num = KB_SENSOR_SCL, .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7, .flags.enable_internal_pullup = true
    };
    i2c_master_bus_handle_t bus;
    ESP_ERROR_CHECK(i2c_new_master_bus(&config, &bus));
    i2c_device_config_t device_config = {
        .dev_addr_length = I2C_ADDR_BIT_LEN_7,
        .device_address = KB_LM73_ADDRESS, .scl_speed_hz = 100000
    };
    i2c_master_dev_handle_t sensor;
    ESP_ERROR_CHECK(i2c_master_bus_add_device(bus, &device_config, &sensor));
    uint32_t interval_ms = 0;
    if (!kb_delay_ms(0.5, &interval_ms)) return;
    for (;;) {
        uint8_t reg = 0, bytes[2];
        esp_err_t status = i2c_master_transmit_receive(sensor, &reg, 1, bytes, 2, 100);
        if (status == ESP_OK) {
            ESP_LOGI("LM73", "temperature=%.2f C", kb_lm73_celsius(bytes[0], bytes[1]));
        } else {
            ESP_LOGW("LM73", "read failed: %s", esp_err_to_name(status));
        }
        vTaskDelay(pdMS_TO_TICKS(interval_ms));
    }
}
