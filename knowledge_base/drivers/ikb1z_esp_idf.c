#include "ikb1z_esp_idf.h"
#include "freertos/task.h"
#include "esp_err.h"

static ikb1z_status_t mapped(esp_err_t error)
{
    if (error == ESP_OK) { return IKB1Z_OK; }
    return error == ESP_ERR_TIMEOUT ? IKB1Z_TIMEOUT : IKB1Z_TRANSPORT_ERROR;
}

static TickType_t budget_ticks(const ikb1z_esp_idf_t *context)
{
    TickType_t ticks = pdMS_TO_TICKS(context->timeout_ms);
    return ticks > 0 ? ticks : 1;
}

static TickType_t remaining(const ikb1z_esp_idf_t *context, TickType_t started)
{
    TickType_t elapsed = xTaskGetTickCount() - started;
    TickType_t budget = budget_ticks(context);
    return elapsed < budget ? budget - elapsed : 0;
}

static bool valid_config(unsigned address, uint32_t timeout_ms, SemaphoreHandle_t lock)
{
    return (address == 0x48u || address == 0x49u) && lock != NULL &&
        timeout_ms >= 1u && timeout_ms <= 60000u;
}

static ikb1z_status_t idf_exchange(void *opaque, uint8_t address,
    const uint8_t *tx, size_t tx_size, uint8_t *rx, size_t rx_size)
{
    ikb1z_esp_idf_t *context = opaque;
    TickType_t started, left;
    esp_err_t error;
    if (context == NULL || !valid_config(address, context->timeout_ms, context->shared_lock) ||
        address != context->address || tx == NULL || tx_size == 0u || tx_size > 2u ||
        rx_size > 2u || (rx_size != 0u && rx == NULL)) { return IKB1Z_INVALID_ARGUMENT; }
#ifndef IKB1Z_USE_LEGACY_I2C
    if (context->device == NULL) { return IKB1Z_INVALID_ARGUMENT; }
#endif
    started = xTaskGetTickCount();
    if (xSemaphoreTake(context->shared_lock, budget_ticks(context)) != pdTRUE) {
        return IKB1Z_TIMEOUT;
    }
    left = remaining(context, started);
    if (left == 0) { error = ESP_ERR_TIMEOUT; }
    else {
#ifdef IKB1Z_USE_LEGACY_I2C
        error = i2c_master_write_to_device(context->port, address, tx, tx_size, left);
#else
        int timeout = (int)(((uint64_t)left * 1000u + configTICK_RATE_HZ - 1u) / configTICK_RATE_HZ);
        error = i2c_master_transmit(context->device, tx, tx_size, timeout);
#endif
    }
    /* Separate calls preserve STOP between command and read. The shared mutex
     * remains held across both; no repeated START or automatic retry implied. */
    left = remaining(context, started);
    if (error == ESP_OK && left == 0) { error = ESP_ERR_TIMEOUT; }
    if (error == ESP_OK && rx_size != 0u) {
#ifdef IKB1Z_USE_LEGACY_I2C
        error = i2c_master_read_from_device(context->port, address, rx, rx_size, left);
#else
        int timeout = (int)(((uint64_t)left * 1000u + configTICK_RATE_HZ - 1u) / configTICK_RATE_HZ);
        error = i2c_master_receive(context->device, rx, rx_size, timeout);
#endif
    }
    if (error == ESP_OK && remaining(context, started) == 0) { error = ESP_ERR_TIMEOUT; }
    xSemaphoreGive(context->shared_lock);
    return mapped(error);
}

#ifdef IKB1Z_USE_LEGACY_I2C
ikb1z_status_t ikb1z_esp_idf_init_legacy(ikb1z_t *device, ikb1z_esp_idf_t *context,
    i2c_port_t port, SemaphoreHandle_t shared_lock, unsigned address, uint32_t timeout_ms)
{
    if (device == NULL || context == NULL || !valid_config(address, timeout_ms, shared_lock) ||
        port < 0 || port >= I2C_NUM_MAX) { return IKB1Z_INVALID_ARGUMENT; }
    context->port = port;
    context->shared_lock = shared_lock;
    context->timeout_ms = timeout_ms;
    context->address = (uint8_t)address;
    return ikb1z_init(device, idf_exchange, context, address);
}
#else
ikb1z_status_t ikb1z_esp_idf_attach(ikb1z_t *device, ikb1z_esp_idf_t *context,
    i2c_master_bus_handle_t bus, SemaphoreHandle_t shared_lock,
    unsigned address, uint32_t clock_hz, uint32_t timeout_ms)
{
    i2c_device_config_t config = {0};
    esp_err_t error;
    if (device == NULL || context == NULL || context->device != NULL || bus == NULL ||
        clock_hz == 0u || !valid_config(address, timeout_ms, shared_lock)) {
        return IKB1Z_INVALID_ARGUMENT;
    }
    context->shared_lock = shared_lock;
    context->timeout_ms = timeout_ms;
    context->address = (uint8_t)address;
    config.dev_addr_length = I2C_ADDR_BIT_LEN_7;
    config.device_address = (uint16_t)address;
    config.scl_speed_hz = clock_hz;
    if (xSemaphoreTake(shared_lock, budget_ticks(context)) != pdTRUE) { return IKB1Z_TIMEOUT; }
    error = i2c_master_bus_add_device(bus, &config, &context->device);
    xSemaphoreGive(shared_lock);
    if (error != ESP_OK) { return mapped(error); }
    return ikb1z_init(device, idf_exchange, context, address);
}

ikb1z_status_t ikb1z_esp_idf_detach(ikb1z_t *device, ikb1z_esp_idf_t *context)
{
    esp_err_t error;
    if (context == NULL || device == NULL || context->device == NULL ||
        device->context != context || device->exchange != idf_exchange ||
        !valid_config(context->address, context->timeout_ms, context->shared_lock)) {
        return IKB1Z_INVALID_ARGUMENT;
    }
    if (xSemaphoreTake(context->shared_lock, budget_ticks(context)) != pdTRUE) { return IKB1Z_TIMEOUT; }
    error = i2c_master_bus_rm_device(context->device);
    if (error == ESP_OK) { context->device = NULL; device->exchange = NULL; }
    xSemaphoreGive(context->shared_lock);
    return mapped(error);
}
#endif
