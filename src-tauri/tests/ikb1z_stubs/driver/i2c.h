#ifndef IKB_TEST_LEGACY_H
#define IKB_TEST_LEGACY_H
#include <stddef.h>
#include <stdint.h>
#include "freertos/FreeRTOS.h"
#include "esp_err.h"
typedef int i2c_port_t;
#define I2C_NUM_MAX 2
esp_err_t i2c_master_write_to_device(i2c_port_t port, uint8_t address, const uint8_t *tx, size_t size, TickType_t ticks);
esp_err_t i2c_master_read_from_device(i2c_port_t port, uint8_t address, uint8_t *rx, size_t size, TickType_t ticks);
#endif
