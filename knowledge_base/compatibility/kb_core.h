#ifndef KB_REFERENCE_CORE_H
#define KB_REFERENCE_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Portable behavioral core. Source contracts: KidBright.zip generators and
 * pid_control/{pid_con,filter,mjoystick}; esp_now/esp_now_class.
 * See README.md for the explicit repairs and limits of compatibility. */
bool kb_delay_ms(double seconds, uint32_t *milliseconds);
double kb_lm73_celsius(uint8_t msb, uint8_t lsb);

typedef struct {
    bool started;
    int64_t previous_us;
    double previous_error;
    double integral;
} kb_pid_t;
bool kb_pid_step(kb_pid_t *state, double kp, double ki, double kd,
                 double setpoint, double feedback, int64_t now_us, double *output);

typedef struct {
    double values[50];
    double sum;
    unsigned window, count, next;
} kb_sma_t;
bool kb_sma_step(kb_sma_t *state, unsigned window, double sample, double *output);

typedef struct {
    float normalize_slope, output_slope;
    int raw_start, output_start, midpoint, center;
    bool ready;
} kb_joystick_t;
bool kb_joystick_init(kb_joystick_t *state, int release, int raw_start,
                      int raw_end, int output_start, int output_end);
bool kb_joystick_read(const kb_joystick_t *state, int raw, int *output);

/* Legacy numeric ESP-NOW payload is ASCII %f, excluding the terminating NUL.
 * Do not decode these packets as raw int32_t or binary float. */
#define KB_ASCII_PACKET_MAX 64
size_t kb_ascii_format(float value, char *buffer, size_t capacity);
bool kb_ascii_parse(const uint8_t *packet, size_t length, float *value);

#endif
