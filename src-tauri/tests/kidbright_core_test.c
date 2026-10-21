#include "kb_core.h"
#include <float.h>
#include <math.h>
#include <stdio.h>
#include <string.h>

static unsigned checks;
#define CHECK(expr) do { checks++; if (!(expr)) { \
    fprintf(stderr, "FAIL line %d: %s\n", __LINE__, #expr); return 1; \
} } while (0)

int main(void) {
    uint32_t ms = 123;
    CHECK(kb_delay_ms(0.02, &ms) && ms == 20);
    CHECK(kb_delay_ms(0.05, &ms) && ms == 50);
    CHECK(kb_delay_ms(0.0009, &ms) && ms == 0);
    CHECK(!kb_delay_ms(-1, &ms));
    CHECK(!kb_delay_ms(NAN, &ms));
    CHECK(!kb_delay_ms(INFINITY, &ms));
    CHECK(!kb_delay_ms(1e20, &ms));
    CHECK(!kb_delay_ms(0, NULL));
    CHECK(kb_lm73_celsius(0x0e, 0x80) == 29.0);
    CHECK(kb_lm73_celsius(0xfd, 0x80) == -5.0);
    CHECK(kb_lm73_celsius(0, 0) == 0.0);

    kb_pid_t pid = {0}, other_pid = {0};
    double out = 0;
    CHECK(kb_pid_step(&pid, 15, .01, 1, 2, 0, 1000000, &out));
    CHECK(fabs(out - 30) < 1e-9 && pid.integral == 0);
    CHECK(kb_pid_step(&pid, 15, .01, 1, 2, 1, 1020000, &out));
    CHECK(fabs(out - (-34.9998)) < 1e-6);
    kb_pid_t saved = pid;
    CHECK(!kb_pid_step(&pid, 15, .01, 1, 2, 1, 1020000, &out));
    CHECK(memcmp(&saved, &pid, sizeof(pid)) == 0);
    CHECK(!kb_pid_step(&pid, NAN, 0, 0, 0, 0, 2000000, &out));
    CHECK(kb_pid_step(&pid, 15, .01, 1, 1000, 0, 2020000, &out));
    CHECK(out == 100 && pid.integral == 60);
    CHECK(kb_pid_step(&other_pid, 15, .01, 1, 0, 0, 1000000, &out));
    CHECK(out == 0 && other_pid.integral == 0);

    kb_sma_t sma = {0}, other_sma = {0};
    CHECK(kb_sma_step(&sma, 3, 1, &out) && out == 1);
    CHECK(kb_sma_step(&sma, 3, 3, &out) && out == 2);
    CHECK(kb_sma_step(&sma, 3, 5, &out) && out == 3);
    CHECK(kb_sma_step(&sma, 3, 7, &out) && out == 5);
    kb_sma_t saved_sma = sma;
    CHECK(!kb_sma_step(&sma, 0, 99, &out));
    CHECK(!kb_sma_step(&sma, 51, 99, &out));
    CHECK(!kb_sma_step(&sma, 3, NAN, &out));
    CHECK(memcmp(&saved_sma, &sma, sizeof(sma)) == 0);
    CHECK(kb_sma_step(&sma, 1, 9, &out) && out == 9);
    CHECK(kb_sma_step(&other_sma, 20, 100, &out) && out == 100);
    CHECK(kb_sma_step(&sma, 1, 7, &out) && out == 7);
    for (int i = 1; i <= 60; i++) CHECK(kb_sma_step(&other_sma, 50, i, &out));
    CHECK(fabs(out - 35.5) < 1e-9); /* samples 11..60 */

    kb_joystick_t joy = {0};
    int position = 0;
    CHECK(!kb_joystick_read(&joy, 1787, &position));
    CHECK(kb_joystick_init(&joy, 1787, 4095, 0, 120, 60));
    CHECK(kb_joystick_read(&joy, 4095, &position) && position == 120);
    CHECK(kb_joystick_read(&joy, 0, &position) && position == 60);
    CHECK(kb_joystick_read(&joy, 1787, &position) && position == 90);
    CHECK(kb_joystick_read(&joy, 1787 + 499, &position) && position == 90);
    CHECK(kb_joystick_read(&joy, 1787 + 500, &position) && position != 90);
    CHECK(kb_joystick_read(&joy, 1787 - 500, &position) && position != 90);
    CHECK(!kb_joystick_read(&joy, 4096, &position));
    CHECK(!kb_joystick_init(&joy, 10, 10, 10, -100, 100));
    CHECK(!kb_joystick_init(&joy, 4096, 0, 4095, -100, 100));

    char packet[KB_ASCII_PACKET_MAX + 1];
    float value = 0;
    size_t length = kb_ascii_format(90, packet, sizeof(packet));
    CHECK(length == 9 && strcmp(packet, "90.000000") == 0);
    CHECK(kb_ascii_parse((const uint8_t *)packet, length, &value) && value == 90);
    CHECK(kb_ascii_format(-80, packet, sizeof(packet)) == 10);
    CHECK(kb_ascii_format(NAN, packet, sizeof(packet)) == 0);
    CHECK(kb_ascii_format(90, packet, 4) == 0 && packet[0] == '\0');
    CHECK(kb_ascii_parse((const uint8_t *)" 1000.000000 ", 13, &value) && value == 1000);
    const char *bad[] = {"", "nan", "inf", "1e99", "90junk", "1 2", " "};
    for (unsigned i = 0; i < sizeof(bad) / sizeof(bad[0]); i++) {
        CHECK(!kb_ascii_parse((const uint8_t *)bad[i], strlen(bad[i]), &value));
    }
    const uint8_t hidden[] = {'9','0',0,'5'};
    CHECK(!kb_ascii_parse(hidden, sizeof(hidden), &value));
    uint8_t oversized[KB_ASCII_PACKET_MAX + 1] = {0};
    CHECK(!kb_ascii_parse(oversized, sizeof(oversized), &value));
    CHECK(kb_ascii_format(FLT_MAX, packet, sizeof(packet)) > 0);
    CHECK(kb_ascii_parse((const uint8_t *)packet, strlen(packet), &value));
    printf("KidBright portable core: %u checks passed\n", checks);
    return 0;
}
