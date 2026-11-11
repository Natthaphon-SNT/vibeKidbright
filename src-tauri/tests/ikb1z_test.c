#include "ikb1z.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

static unsigned calls;
static uint8_t last[2], reply[2];
static size_t last_size, reply_size;
static ikb1z_status_t result;
static ikb1z_status_t fake(void *context, uint8_t address, const uint8_t *tx,
    size_t tx_size, uint8_t *rx, size_t rx_size)
{
    (void)context;
    assert(address == 0x49);
    assert(tx_size <= sizeof(last));
    ++calls;
    memcpy(last, tx, tx_size);
    last_size = tx_size;
    if (result != IKB1Z_OK) { return result; }
    assert(rx_size == reply_size);
    if (rx_size != 0) { memcpy(rx, reply, rx_size); }
    return IKB1Z_OK;
}

int main(void)
{
    ikb1z_t device = {0};
    uint8_t frame[2], value = 77;
    uint16_t adc = 777;
    unsigned channel, mode, mask;
    int speed, scaled;
    assert(ikb1z_init(&device, fake, NULL, 0x49) == IKB1Z_OK);
    assert(calls == 0);
    assert(ikb1z_init(NULL, fake, NULL, 0x48) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_init(&device, fake, NULL, 0x90) == IKB1Z_INVALID_ARGUMENT);
    for (channel = 0; channel < 8; ++channel) {
        for (mode = 0; mode <= 5; ++mode) {
            assert(ikb1z_encode_io(channel, mode, frame) == IKB1Z_OK);
            assert(frame[0] == 8 + channel && frame[1] == mode);
        }
    }
    for (mask = 1; mask < 16; ++mask) {
        for (speed = -100; speed <= 100; ++speed) {
            assert(ikb1z_encode_motor(mask, speed, frame) == IKB1Z_OK);
            assert(frame[0] == (0x20 | mask));
            assert(frame[1] == (speed < 0 ? 256 + speed : speed));
        }
    }
    for (mask = 1; mask < 64; ++mask) {
        for (scaled = 0; scaled <= 200; ++scaled) {
            assert(ikb1z_encode_servo(mask, scaled, frame) == IKB1Z_OK);
            assert(frame[0] == (0x40 | mask) && frame[1] == scaled);
        }
    }
    assert(ikb1z_encode_servo(1, -1, frame) == IKB1Z_NOT_SUPPORTED);
    assert(ikb1z_encode_servo(64, 100, frame) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_encode_motor(0, 0, frame) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_encode_motor(1, -101, frame) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_encode_motor(1, 101, frame) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_encode_io(8, 0, frame) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_encode_io(0, 6, frame) == IKB1Z_INVALID_ARGUMENT);
    assert(ikb1z_encode_io(0, 1, NULL) == IKB1Z_INVALID_ARGUMENT);
    reply_size = 0;
    assert(ikb1z_motor(&device, 2, -50) == IKB1Z_OK);
    assert(last[0] == 0x22 && last[1] == 0xce && last_size == 2);
    assert(ikb1z_stop(&device) == IKB1Z_OK);
    assert(last[0] == 0x23 && last[1] == 0);
    calls = 0;
    assert(ikb1z_motor(&device, 4, 50) == IKB1Z_NOT_SUPPORTED);
    assert(ikb1z_servo_free(&device, 1) == IKB1Z_NOT_SUPPORTED);
    assert(ikb1z_uart_read(&device, &value, 1) == IKB1Z_NOT_SUPPORTED);
    assert(calls == 0);
    reply_size = 2; reply[0] = 2; reply[1] = 0xab;
    assert(ikb1z_adc10(&device, 7, &adc) == IKB1Z_OK);
    assert(adc == 683 && last_size == 1 && last[0] == 0xf0);
    reply[0] = 4; adc = 777;
    assert(ikb1z_adc10(&device, 0, &adc) == IKB1Z_BAD_RESPONSE && adc == 777);
    assert(ikb1z_decode_adc10(reply, 1, &adc) == IKB1Z_BAD_RESPONSE && adc == 777);
    result = IKB1Z_TIMEOUT;
    assert(ikb1z_adc10(&device, 0, &adc) == IKB1Z_TIMEOUT && adc == 777);
    assert(ikb1z_adc8(&device, 0, &value) == IKB1Z_TIMEOUT && value == 77);
    result = IKB1Z_OK; reply_size = 1; reply[0] = 101;
    assert(ikb1z_adc_percent(&device, 0, &value) == IKB1Z_BAD_RESPONSE && value == 77);
    assert(ikb1z_input(&device, 0, true, &value) == IKB1Z_BAD_RESPONSE && value == 77);
    reply[0] = 1;
    assert(ikb1z_input(&device, 0, true, &value) == IKB1Z_OK && value == 1);
    assert(last[0] == 8 && last[1] == 3);
    reply_size = 0;
    assert(ikb1z_output(&device, 5, true) == IKB1Z_OK && last[0] == 13 && last[1] == 1);
    assert(ikb1z_servo(&device, 3, 100) == IKB1Z_OK && last[0] == 0x43 && last[1] == 100);
    assert(ikb1z_reset(&device) == IKB1Z_OK && last_size == 1 && last[0] == 0);
    puts("iKB-1Z C protocol: exhaustive ranges, commands, gates and error propagation passed.");
    return 0;
}
