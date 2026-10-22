#ifndef KB_DISPLAY_IDF_H
#define KB_DISPLAY_IDF_H
#include "driver/i2c_master.h"
#include "esp_err.h"
#include "kb_basic.h"

/* Caller owns/initializes the new-driver bus and serializes framebuffer access.
 * Never mix with legacy driver/i2c.h. No hidden GPIO/bus selection. */
esp_err_t kb_display_open(i2c_master_bus_handle_t bus, uint8_t address,
                          i2c_master_dev_handle_t *display);
esp_err_t kb_display_write(i2c_master_dev_handle_t display, const kb_matrix_t *frame);
esp_err_t kb_display_brightness(i2c_master_dev_handle_t display, uint8_t level);
esp_err_t kb_display_close(i2c_master_dev_handle_t display);
#endif
