#ifndef VIBE_IKB1Z_H
#define VIBE_IKB1Z_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Supplied protocol, host-tested only. No bus/pins or actuation at init. */
typedef enum {
    IKB1Z_OK = 0, IKB1Z_INVALID_ARGUMENT, IKB1Z_NOT_SUPPORTED,
    IKB1Z_TRANSPORT_ERROR, IKB1Z_TIMEOUT, IKB1Z_BAD_RESPONSE
} ikb1z_status_t;

/* One serialized write STOP, optional read STOP pair. Return OK only for
 * exact lengths; enforce a finite deadline and a shared bus lock in adapter.
 * Do not retry a partially completed actuator/UART operation automatically. */
typedef ikb1z_status_t (*ikb1z_exchange_fn)(void *context, uint8_t address,
    const uint8_t *tx, size_t tx_size, uint8_t *rx, size_t rx_size);
typedef struct {
    ikb1z_exchange_fn exchange;
    void *context;
    uint8_t address;
} ikb1z_t;

ikb1z_status_t ikb1z_init(ikb1z_t *device, ikb1z_exchange_fn exchange,
    void *context, unsigned address);
ikb1z_status_t ikb1z_encode_io(unsigned channel, unsigned mode, uint8_t out[2]);
ikb1z_status_t ikb1z_encode_motor(unsigned mask, int speed, uint8_t out[2]);
ikb1z_status_t ikb1z_encode_servo(unsigned mask, int scaled, uint8_t out[2]);
ikb1z_status_t ikb1z_decode_adc10(const uint8_t *bytes, size_t size, uint16_t *value);
ikb1z_status_t ikb1z_output(ikb1z_t *device, unsigned channel, bool high);
ikb1z_status_t ikb1z_input(ikb1z_t *device, unsigned channel, bool pullup, uint8_t *value);
ikb1z_status_t ikb1z_adc8(ikb1z_t *device, unsigned channel, uint8_t *value);
ikb1z_status_t ikb1z_adc_percent(ikb1z_t *device, unsigned channel, uint8_t *value);
ikb1z_status_t ikb1z_adc10(ikb1z_t *device, unsigned channel, uint16_t *value);
/* Driver enables onboard M1/M2 only. Pure encoder also describes M3/M4 bytes. */
ikb1z_status_t ikb1z_motor(ikb1z_t *device, unsigned mask, int speed);
ikb1z_status_t ikb1z_stop(ikb1z_t *device);
ikb1z_status_t ikb1z_servo(ikb1z_t *device, unsigned mask, int scaled);
ikb1z_status_t ikb1z_reset(ikb1z_t *device);
/* Deliberate capability gates; no bytes sent until firmware contracts verified. */
ikb1z_status_t ikb1z_servo_free(ikb1z_t *device, unsigned mask);
ikb1z_status_t ikb1z_uart_read(ikb1z_t *device, uint8_t *data, size_t size);
#endif
