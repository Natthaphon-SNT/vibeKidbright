#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <string.h>
#include "kb_basic.h"
static unsigned checks;
#define CHECK(expr) do { ++checks; assert(expr); } while (0)

int main(void) {
    kb_matrix_t frame = {0};
    uint8_t packet[17];
    bool pixel = true;
    CHECK(kb_matrix_pack(&frame, packet));
    for (unsigned i = 0; i < 17; ++i) CHECK(packet[i] == 0);
    CHECK(kb_matrix_pixel(&frame, 0, 0, true));
    CHECK(kb_matrix_pixel(&frame, 8, 7, true));
    CHECK(kb_matrix_pixel(&frame, 15, 0, true));
    CHECK(kb_matrix_pack(&frame, packet));
    CHECK(packet[0] == 0 && packet[1] == 0x80 && packet[2] == 0x01 && packet[16] == 0x80);
    for (unsigned i = 3; i < 16; ++i) CHECK(packet[i] == 0);
    CHECK(kb_matrix_pixel(&frame, 0, 0, false));
    CHECK(kb_matrix_get(&frame, 0, 0, &pixel) && !pixel);
    kb_matrix_t saved = frame;
    CHECK(!kb_matrix_pixel(&frame, 16, 0, true));
    CHECK(!kb_matrix_pixel(&frame, 0, 8, true));
    CHECK(!kb_matrix_digit(&frame, 10, 0, 0));
    CHECK(!kb_matrix_digit(&frame, 2, 12, 0));
    CHECK(!kb_matrix_digit(&frame, 2, 0, 2));
    CHECK(!kb_matrix_shift(&frame, -17));
    CHECK(memcmp(&saved, &frame, sizeof(frame)) == 0);
    CHECK(!kb_matrix_get(&frame, 16, 0, &pixel));
    CHECK(!kb_matrix_get(&frame, 0, 0, NULL));
    CHECK(!kb_matrix_pack(NULL, packet));
    CHECK(!kb_matrix_pack(&frame, NULL));
    kb_matrix_clear(&frame);
    /* Every logical pixel survives set/get/clear; adjacent pixels stay unchanged. */
    for (unsigned y = 0; y < 8; ++y) for (unsigned x = 0; x < 16; ++x) {
        CHECK(kb_matrix_pixel(&frame, x, y, true));
        CHECK(kb_matrix_get(&frame, x, y, &pixel) && pixel);
    }
    CHECK(kb_matrix_pack(&frame, packet));
    for (unsigned i = 1; i < 17; ++i) CHECK(packet[i] == 0xff);
    CHECK(kb_matrix_shift(&frame, 1));
    CHECK(frame.columns[0] == 0 && frame.columns[15] == 0xff);
    CHECK(kb_matrix_shift(&frame, -2));
    CHECK(frame.columns[0] == 0xff && frame.columns[14] == 0 && frame.columns[15] == 0);
    CHECK(kb_matrix_shift(&frame, 16));
    CHECK(kb_matrix_pack(&frame, packet));
    for (unsigned i = 0; i < 17; ++i) CHECK(packet[i] == 0);
    CHECK(kb_matrix_digit(&frame, 1, 0, 0));
    CHECK(frame.columns[2] == 0x7f && frame.columns[0] == 0);
    CHECK(kb_matrix_digit(&frame, 8, 11, 1));
    CHECK(frame.columns[12] == 0x92 && frame.columns[15] == 0x6c);

    uint32_t result = 999;
    CHECK(kb_music_frequency(69, &result) && result == 440);
    CHECK(kb_music_frequency(81, &result) && result == 880);
    CHECK(kb_music_frequency(60, &result) && result == 262);
    CHECK(kb_music_frequency(0, &result) && result == 8);
    CHECK(kb_music_frequency(127, &result) && result == 12544);
    result = 999;
    CHECK(!kb_music_frequency(128, &result) && result == 999);
    CHECK(!kb_music_frequency(69, NULL));
    CHECK(kb_music_duration(120, 1, &result) && result == 500);
    CHECK(kb_music_duration(120, 0.5, &result) && result == 250);
    CHECK(kb_music_duration(60, 4, &result) && result == 4000);
    CHECK(kb_music_duration(180, 1, &result) && result == 333);
    CHECK(kb_music_duration(120, 0, &result) && result == 0);
    result = 999;
    CHECK(!kb_music_duration(0, 1, &result));
    CHECK(!kb_music_duration(120, -1, &result));
    CHECK(!kb_music_duration(NAN, 1, &result));
    CHECK(!kb_music_duration(120, INFINITY, &result));
    CHECK(!kb_music_duration(0.001, UINT32_MAX, &result));
    CHECK(!kb_music_duration(120000, 1, &result));
    CHECK(result == 999);
    printf("KidBright matrix/music: %u checks passed\n", checks);
    return 0;
}
