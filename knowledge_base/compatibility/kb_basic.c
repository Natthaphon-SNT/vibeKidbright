#include "kb_basic.h"
#include <math.h>
#include <string.h>

void kb_matrix_clear(kb_matrix_t *frame) {
    if (frame) memset(frame, 0, sizeof(*frame));
}
bool kb_matrix_pixel(kb_matrix_t *frame, unsigned x, unsigned y, bool on) {
    if (!frame || x >= 16 || y >= 8) return false;
    uint8_t bit = (uint8_t)(1u << y);
    if (on) frame->columns[x] |= bit;
    else frame->columns[x] &= (uint8_t)~bit;
    return true;
}
bool kb_matrix_get(const kb_matrix_t *frame, unsigned x, unsigned y, bool *on) {
    if (!frame || !on || x >= 16 || y >= 8) return false;
    *on = (frame->columns[x] & (1u << y)) != 0;
    return true;
}
bool kb_matrix_shift(kb_matrix_t *frame, int columns) {
    if (!frame || columns < -16 || columns > 16) return false;
    if (columns == 16 || columns == -16) kb_matrix_clear(frame);
    else if (columns > 0) {
        memmove(frame->columns + columns, frame->columns, 16u - (unsigned)columns);
        memset(frame->columns, 0, (size_t)columns);
    } else if (columns < 0) {
        unsigned amount = (unsigned)-columns;
        memmove(frame->columns, frame->columns + amount, 16u - amount);
        memset(frame->columns + 16u - amount, 0, amount);
    }
    return true;
}
bool kb_matrix_digit(kb_matrix_t *frame, unsigned digit, unsigned x, unsigned y) {
    static const uint8_t glyphs[10][5] = {
        {0x3e,0x51,0x49,0x45,0x3e}, {0x00,0x42,0x7f,0x40,0x00},
        {0x42,0x61,0x51,0x49,0x46}, {0x21,0x41,0x45,0x4b,0x31},
        {0x18,0x14,0x12,0x7f,0x10}, {0x27,0x45,0x45,0x45,0x39},
        {0x3c,0x4a,0x49,0x49,0x30}, {0x01,0x71,0x09,0x05,0x03},
        {0x36,0x49,0x49,0x49,0x36}, {0x06,0x49,0x49,0x29,0x1e}
    };
    if (!frame || digit > 9 || x > 11 || y > 1) return false;
    for (unsigned col = 0; col < 5; ++col) {
        uint8_t mask = (uint8_t)(0x7fu << y);
        frame->columns[x + col] = (uint8_t)((frame->columns[x + col] & (uint8_t)~mask)
                                           | (glyphs[digit][col] << y));
    }
    return true;
}
bool kb_matrix_pack(const kb_matrix_t *frame, uint8_t packet[17]) {
    if (!frame || !packet) return false;
    packet[0] = 0; /* HT16K33 RAM start address. */
    for (unsigned x = 0; x < 16; ++x) {
        uint8_t reversed = 0;
        for (unsigned y = 0; y < 8; ++y)
            if (frame->columns[x] & (1u << y)) reversed |= (uint8_t)(1u << (7u - y));
        packet[1u + 2u * (x % 8u) + x / 8u] = reversed;
    }
    return true;
}
bool kb_music_frequency(unsigned midi_note, uint32_t *hertz) {
    if (!hertz || midi_note > 127) return false;
    *hertz = (uint32_t)floor(440.0 * pow(2.0, ((double)midi_note - 69.0) / 12.0) + 0.5);
    return true;
}
bool kb_music_duration(double bpm, double beats, uint32_t *milliseconds) {
    if (!milliseconds || !isfinite(bpm) || !isfinite(beats) || bpm <= 0 || beats < 0) return false;
    double duration = (60000.0 / bpm) * beats;
    if (!isfinite(duration) || duration > UINT32_MAX || (beats > 0 && duration < 1.0)) return false;
    *milliseconds = (uint32_t)floor(duration + 0.5);
    return true;
}
