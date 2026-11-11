#include "ikb1z.h"

static bool valid_mask(unsigned mask, unsigned allowed)
{
    return mask != 0u && (mask & ~allowed) == 0u;
}

ikb1z_status_t ikb1z_init(ikb1z_t *device, ikb1z_exchange_fn exchange,
    void *context, unsigned address)
{
    if (device == NULL || exchange == NULL || (address != 0x48u && address != 0x49u)) {
        return IKB1Z_INVALID_ARGUMENT;
    }
    device->exchange = exchange;
    device->context = context;
    device->address = (uint8_t)address;
    return IKB1Z_OK;
}

ikb1z_status_t ikb1z_encode_io(unsigned channel, unsigned mode, uint8_t out[2])
{
    if (out == NULL || channel > 7u || mode > 5u) { return IKB1Z_INVALID_ARGUMENT; }
    out[0] = (uint8_t)(0x08u + channel);
    out[1] = (uint8_t)mode;
    return IKB1Z_OK;
}

ikb1z_status_t ikb1z_encode_motor(unsigned mask, int speed, uint8_t out[2])
{
    if (out == NULL || !valid_mask(mask, 0x0fu) || speed < -100 || speed > 100) {
        return IKB1Z_INVALID_ARGUMENT;
    }
    out[0] = (uint8_t)(0x20u | mask);
    out[1] = (uint8_t)speed; /* Cast after range check; never borrow/carry command. */
    return IKB1Z_OK;
}

ikb1z_status_t ikb1z_encode_servo(unsigned mask, int scaled, uint8_t out[2])
{
    if (out == NULL || !valid_mask(mask, 0x3fu)) { return IKB1Z_INVALID_ARGUMENT; }
    if (scaled == -1) { return IKB1Z_NOT_SUPPORTED; }
    if (scaled < 0 || scaled > 200) { return IKB1Z_INVALID_ARGUMENT; }
    out[0] = (uint8_t)(0x40u | mask);
    out[1] = (uint8_t)scaled;
    return IKB1Z_OK;
}

ikb1z_status_t ikb1z_decode_adc10(const uint8_t *bytes, size_t size, uint16_t *value)
{
    uint16_t raw;
    if (bytes == NULL || value == NULL) { return IKB1Z_INVALID_ARGUMENT; }
    if (size != 2u) { return IKB1Z_BAD_RESPONSE; }
    raw = (uint16_t)(((uint16_t)bytes[0] << 8) | bytes[1]);
    if (raw > 1023u) { return IKB1Z_BAD_RESPONSE; }
    *value = raw;
    return IKB1Z_OK;
}

static ikb1z_status_t exchange(ikb1z_t *device, const uint8_t *tx, size_t tx_size,
    uint8_t *rx, size_t rx_size)
{
    if (device == NULL || device->exchange == NULL ||
        (device->address != 0x48u && device->address != 0x49u)) {
        return IKB1Z_INVALID_ARGUMENT;
    }
    return device->exchange(device->context, device->address, tx, tx_size, rx, rx_size);
}

ikb1z_status_t ikb1z_output(ikb1z_t *device, unsigned channel, bool high)
{
    uint8_t tx[2];
    ikb1z_status_t status = ikb1z_encode_io(channel, high ? 1u : 0u, tx);
    return status == IKB1Z_OK ? exchange(device, tx, sizeof(tx), NULL, 0) : status;
}

static ikb1z_status_t read_byte(ikb1z_t *device, unsigned channel, unsigned mode,
    uint8_t maximum, uint8_t *value)
{
    uint8_t tx[2], rx = 0;
    ikb1z_status_t status;
    if (value == NULL) { return IKB1Z_INVALID_ARGUMENT; }
    status = ikb1z_encode_io(channel, mode, tx);
    if (status != IKB1Z_OK) { return status; }
    status = exchange(device, tx, sizeof(tx), &rx, 1);
    if (status != IKB1Z_OK) { return status; }
    if (rx > maximum) { return IKB1Z_BAD_RESPONSE; }
    *value = rx;
    return IKB1Z_OK;
}

ikb1z_status_t ikb1z_input(ikb1z_t *device, unsigned channel, bool pullup, uint8_t *value)
{ return read_byte(device, channel, pullup ? 3u : 2u, 1u, value); }
ikb1z_status_t ikb1z_adc8(ikb1z_t *device, unsigned channel, uint8_t *value)
{ return read_byte(device, channel, 4u, 255u, value); }
ikb1z_status_t ikb1z_adc_percent(ikb1z_t *device, unsigned channel, uint8_t *value)
{ return read_byte(device, channel, 5u, 100u, value); }

ikb1z_status_t ikb1z_adc10(ikb1z_t *device, unsigned channel, uint16_t *value)
{
    uint8_t tx, rx[2];
    ikb1z_status_t status;
    if (channel > 7u || value == NULL) { return IKB1Z_INVALID_ARGUMENT; }
    tx = (uint8_t)(0x80u | (channel << 4));
    status = exchange(device, &tx, 1, rx, sizeof(rx));
    return status == IKB1Z_OK ? ikb1z_decode_adc10(rx, sizeof(rx), value) : status;
}

ikb1z_status_t ikb1z_motor(ikb1z_t *device, unsigned mask, int speed)
{
    uint8_t tx[2];
    ikb1z_status_t status = ikb1z_encode_motor(mask, speed, tx);
    if (status != IKB1Z_OK) { return status; }
    if (!valid_mask(mask, 0x03u)) { return IKB1Z_NOT_SUPPORTED; }
    return exchange(device, tx, sizeof(tx), NULL, 0);
}

ikb1z_status_t ikb1z_stop(ikb1z_t *device) { return ikb1z_motor(device, 3u, 0); }

ikb1z_status_t ikb1z_servo(ikb1z_t *device, unsigned mask, int scaled)
{
    uint8_t tx[2];
    ikb1z_status_t status = ikb1z_encode_servo(mask, scaled, tx);
    return status == IKB1Z_OK ? exchange(device, tx, sizeof(tx), NULL, 0) : status;
}

ikb1z_status_t ikb1z_reset(ikb1z_t *device)
{
    const uint8_t tx = 0;
    return exchange(device, &tx, 1, NULL, 0);
}

ikb1z_status_t ikb1z_servo_free(ikb1z_t *device, unsigned mask)
{
    (void)device;
    return valid_mask(mask, 0x3fu) ? IKB1Z_NOT_SUPPORTED : IKB1Z_INVALID_ARGUMENT;
}

ikb1z_status_t ikb1z_uart_read(ikb1z_t *device, uint8_t *data, size_t size)
{
    (void)device; (void)data; (void)size;
    return IKB1Z_NOT_SUPPORTED;
}
