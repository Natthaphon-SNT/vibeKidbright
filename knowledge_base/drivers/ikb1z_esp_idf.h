#ifndef VIBE_IKB1Z_ESP_IDF_H
#define VIBE_IKB1Z_ESP_IDF_H
#include "ikb1z.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
/* Choose ONE driver family for the complete app. Legacy requires this define
 * for all files using this adapter; never link legacy and new IDF I2C together. */
#ifdef IKB1Z_USE_LEGACY_I2C
#include "driver/i2c.h"
#else
#include "driver/i2c_master.h"
#endif

typedef struct {
    SemaphoreHandle_t shared_lock; /* Borrowed; all bus users share this mutex. */
    uint32_t timeout_ms; /* Project policy: 1..60000 ms, including lock wait. */
    uint8_t address;
#ifdef IKB1Z_USE_LEGACY_I2C
    i2c_port_t port;
#else
    i2c_master_dev_handle_t device; /* Owned; context must be zero initialized. */
#endif
} ikb1z_esp_idf_t;

#ifdef IKB1Z_USE_LEGACY_I2C
/* Existing initialized port only; no i2c_driver_install/delete or pin changes. */
ikb1z_status_t ikb1z_esp_idf_init_legacy(ikb1z_t *device, ikb1z_esp_idf_t *context,
    i2c_port_t port, SemaphoreHandle_t shared_lock, unsigned address, uint32_t timeout_ms);
#else
/* Adds a device on the caller's EXISTING bus; never creates/deletes that bus.
 * Frequency is a caller-confirmed bus policy, not a certified device maximum. */
ikb1z_status_t ikb1z_esp_idf_attach(ikb1z_t *device, ikb1z_esp_idf_t *context,
    i2c_master_bus_handle_t bus, SemaphoreHandle_t shared_lock,
    unsigned address, uint32_t clock_hz, uint32_t timeout_ms);
ikb1z_status_t ikb1z_esp_idf_detach(ikb1z_t *device, ikb1z_esp_idf_t *context);
#endif
#endif
