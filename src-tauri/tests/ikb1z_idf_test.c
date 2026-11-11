/* Contract fixtures, not ESP-IDF build or physical I2C evidence. */
#include "ikb1z_esp_idf.h"
#include "esp_err.h"
#include <assert.h>
#include <stdio.h>

static TickType_t now, lock_wait, write_delay, read_delay;
static int held, takes, gives, writes, reads, allow_lock = 1;
static esp_err_t write_result, read_result;
static int write_timeout, read_timeout;
static uint8_t payload[2];
static void *lock_token = (void *)1;
TickType_t xTaskGetTickCount(void) { return now; }
BaseType_t xSemaphoreTake(SemaphoreHandle_t lock, TickType_t ticks)
{
    assert(lock == lock_token && ticks > 0 && ticks <= 10);
    assert(!held);
    ++takes; now += lock_wait;
    if (!allow_lock) { return pdFALSE; }
    held = 1; return pdTRUE;
}
BaseType_t xSemaphoreGive(SemaphoreHandle_t lock)
{
    assert(lock == lock_token && held);
    held = 0; ++gives; return pdTRUE;
}
static esp_err_t write_op(const uint8_t *tx, size_t size, int timeout)
{
    assert(held && size >= 1 && size <= 2 && timeout > 0 && timeout <= 10);
    ++writes; write_timeout = timeout;
    payload[0] = tx[0]; payload[1] = size == 2 ? tx[1] : 0;
    now += write_delay; return write_result;
}
static esp_err_t read_op(uint8_t *rx, size_t size, int timeout)
{
    assert(held && writes == 1 && size == 2 && timeout > 0 && timeout <= 10);
    ++reads; read_timeout = timeout;
    rx[0] = 2; rx[1] = 0xab;
    now += read_delay; return read_result;
}
#ifdef IKB1Z_USE_LEGACY_I2C
esp_err_t i2c_master_write_to_device(i2c_port_t port, uint8_t address, const uint8_t *tx, size_t size, TickType_t ticks)
{ assert(port == 1 && address == 0x49); return write_op(tx, size, (int)ticks); }
esp_err_t i2c_master_read_from_device(i2c_port_t port, uint8_t address, uint8_t *rx, size_t size, TickType_t ticks)
{ assert(port == 1 && address == 0x49); return read_op(rx, size, (int)ticks); }
#else
static int adds, removes;
esp_err_t i2c_master_bus_add_device(i2c_master_bus_handle_t bus, const i2c_device_config_t *config, i2c_master_dev_handle_t *device)
{
    assert(held && bus == (void *)2 && config->device_address == 0x49);
    assert(config->scl_speed_hz == 100000 && config->dev_addr_length == I2C_ADDR_BIT_LEN_7);
    *device = (void *)3; ++adds; return ESP_OK;
}
esp_err_t i2c_master_bus_rm_device(i2c_master_dev_handle_t device)
{ assert(held && device == (void *)3); ++removes; return ESP_OK; }
esp_err_t i2c_master_transmit(i2c_master_dev_handle_t device, const uint8_t *tx, size_t size, int timeout)
{ assert(device == (void *)3); return write_op(tx, size, timeout); }
esp_err_t i2c_master_receive(i2c_master_dev_handle_t device, uint8_t *rx, size_t size, int timeout)
{ assert(device == (void *)3); return read_op(rx, size, timeout); }
#endif
static void clean(void)
{
    now = lock_wait = write_delay = read_delay = 0;
    held = takes = gives = writes = reads = 0;
    write_result = read_result = ESP_OK; allow_lock = 1;
}
int main(void)
{
    ikb1z_t device = {0};
    ikb1z_esp_idf_t context = {0};
    uint16_t value = 777;
#ifdef IKB1Z_USE_LEGACY_I2C
    assert(ikb1z_esp_idf_init_legacy(&device, &context, 1, lock_token, 0x49, 10) == IKB1Z_OK);
    assert(takes == 0 && writes == 0);
#else
    assert(ikb1z_esp_idf_attach(&device, &context, (void *)2, lock_token, 0x49, 100000, 10) == IKB1Z_OK);
    assert(adds == 1 && writes == 0 && reads == 0 && gives == 1);
    assert(ikb1z_esp_idf_attach(&device, &context, (void *)2, lock_token, 0x49, 100000, 10) == IKB1Z_INVALID_ARGUMENT);
#endif
    clean(); lock_wait = 3; write_delay = 4; read_delay = 2;
    assert(ikb1z_adc10(&device, 7, &value) == IKB1Z_OK && value == 683);
    assert(takes == 1 && gives == 1 && writes == 1 && reads == 1 && !held);
    assert(payload[0] == 0xf0 && write_timeout == 7 && read_timeout == 3);
    clean(); allow_lock = 0; value = 777;
    assert(ikb1z_adc10(&device, 0, &value) == IKB1Z_TIMEOUT && value == 777);
    assert(writes == 0 && reads == 0 && gives == 0);
    clean(); write_result = ESP_FAIL;
    assert(ikb1z_adc10(&device, 0, &value) == IKB1Z_TRANSPORT_ERROR && value == 777);
    assert(writes == 1 && reads == 0 && gives == 1);
    clean(); write_result = ESP_ERR_TIMEOUT;
    assert(ikb1z_adc10(&device, 0, &value) == IKB1Z_TIMEOUT && reads == 0 && gives == 1);
    clean(); write_delay = 10;
    assert(ikb1z_adc10(&device, 0, &value) == IKB1Z_TIMEOUT && reads == 0 && gives == 1);
    clean(); read_delay = 10;
    assert(ikb1z_adc10(&device, 0, &value) == IKB1Z_TIMEOUT && value == 777 && gives == 1);
    clean(); read_result = ESP_FAIL;
    assert(ikb1z_adc10(&device, 0, &value) == IKB1Z_TRANSPORT_ERROR && value == 777 && gives == 1);
    clean();
    assert(ikb1z_motor(&device, 2, -50) == IKB1Z_OK && writes == 1 && reads == 0);
    assert(payload[0] == 0x22 && payload[1] == 0xce);
#ifndef IKB1Z_USE_LEGACY_I2C
    clean();
    assert(ikb1z_esp_idf_detach(&device, &context) == IKB1Z_OK && removes == 1);
    assert(ikb1z_stop(&device) == IKB1Z_INVALID_ARGUMENT && writes == 0);
    assert(ikb1z_esp_idf_detach(&device, &context) == IKB1Z_INVALID_ARGUMENT);
#endif
    puts("iKB-1Z IDF adapter contract: locking, STOP pairs, deadlines, errors and lifecycle passed.");
    return 0;
}
