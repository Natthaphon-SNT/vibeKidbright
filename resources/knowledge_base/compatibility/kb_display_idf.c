#include "kb_display_idf.h"

esp_err_t kb_display_open(i2c_master_bus_handle_t bus, uint8_t address,
                          i2c_master_dev_handle_t *display) {
    if (!bus || !display || address < 0x70 || address > 0x77) return ESP_ERR_INVALID_ARG;
    *display = NULL;
    i2c_device_config_t cfg = {
        .dev_addr_length = I2C_ADDR_BIT_LEN_7, .device_address = address,
        .scl_speed_hz = 100000
    };
    esp_err_t result = i2c_master_bus_add_device(bus, &cfg, display);
    if (result != ESP_OK) return result;
    const uint8_t commands[] = {0x21, 0x81, 0xef};
    for (unsigned i = 0; i < sizeof(commands); ++i) {
        result = i2c_master_transmit(*display, &commands[i], 1, 100);
        if (result != ESP_OK) {
            i2c_master_bus_rm_device(*display);
            *display = NULL;
            return result;
        }
    }
    return ESP_OK;
}
esp_err_t kb_display_write(i2c_master_dev_handle_t display, const kb_matrix_t *frame) {
    uint8_t packet[17];
    if (!display || !kb_matrix_pack(frame, packet)) return ESP_ERR_INVALID_ARG;
    return i2c_master_transmit(display, packet, sizeof(packet), 100);
}
esp_err_t kb_display_brightness(i2c_master_dev_handle_t display, uint8_t level) {
    if (!display || level > 15) return ESP_ERR_INVALID_ARG;
    uint8_t command = (uint8_t)(0xe0 | level);
    return i2c_master_transmit(display, &command, 1, 100);
}
esp_err_t kb_display_close(i2c_master_dev_handle_t display) {
    return display ? i2c_master_bus_rm_device(display) : ESP_ERR_INVALID_ARG;
}
