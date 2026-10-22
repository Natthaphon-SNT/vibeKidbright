#include "kb_core.h"

#include <errno.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

static double clamp(double value, double low, double high) {
    return value < low ? low : (value > high ? high : value);
}

bool kb_delay_ms(double seconds, uint32_t *milliseconds) {
    double ms = seconds * 1000.0;
    if (!milliseconds || !isfinite(ms) || ms < 0 || ms > UINT32_MAX) return false;
    *milliseconds = (uint32_t)ms;
    return true;
}

double kb_lm73_celsius(uint8_t msb, uint8_t lsb) {
    int32_t raw = ((int32_t)msb << 8) | lsb;
    if (raw & 0x8000) raw -= 65536;
    return raw / 128.0;
}

bool kb_pid_step(kb_pid_t *state, double kp, double ki, double kd,
                 double setpoint, double feedback, int64_t now_us, double *output) {
    if (!state || !output || now_us < 0 || !isfinite(kp) || !isfinite(ki)
        || !isfinite(kd) || !isfinite(setpoint) || !isfinite(feedback)) return false;
    if (state->started && now_us <= state->previous_us) return false;
    double error = setpoint - feedback;
    double dt = state->started ? (now_us - state->previous_us) * 1e-6 : 0;
    double integral = clamp(state->integral + error * dt, -60, 60);
    double derivative = state->started ? (error - state->previous_error) / dt : 0;
    double result = kp * error + ki * integral + kd * derivative;
    if (!isfinite(result)) return false;
    *output = clamp(result, -100, 100);
    state->integral = integral;
    state->previous_error = error;
    state->previous_us = now_us;
    state->started = true;
    return true;
}

bool kb_sma_step(kb_sma_t *state, unsigned window, double sample, double *output) {
    if (!state || !output || window < 1 || window > 50 || !isfinite(sample)) return false;
    kb_sma_t next = *state;
    if (next.window != window) {
        memset(&next, 0, sizeof(next));
        next.window = window;
    }
    if (next.count == window) next.sum -= next.values[next.next];
    else next.count++;
    next.sum += sample;
    if (!isfinite(next.sum)) return false;
    next.values[next.next] = sample;
    next.next = (next.next + 1) % window;
    *output = next.sum / next.count;
    *state = next;
    return true;
}

bool kb_joystick_init(kb_joystick_t *state, int release, int raw_start,
                      int raw_end, int output_start, int output_end) {
    if (!state || raw_start < 0 || raw_start > 4095 || raw_end < 0 || raw_end > 4095
        || raw_start == raw_end || release < (raw_start < raw_end ? raw_start : raw_end)
        || release > (raw_start > raw_end ? raw_start : raw_end)
        || output_start < -10000 || output_start > 10000
        || output_end < -10000 || output_end > 10000) return false;
    float slope = 4095.0f / (float)(raw_end - raw_start);
    *state = (kb_joystick_t){
        .normalize_slope = slope, .output_slope = (output_end - output_start) / 4095.0f,
        .raw_start = raw_start, .output_start = output_start,
        .midpoint = (output_start + output_end) / 2,
        .center = (int)(slope * (float)(release - raw_start)), .ready = true
    };
    return true;
}

bool kb_joystick_read(const kb_joystick_t *state, int raw, int *output) {
    if (!state || !state->ready || !output || raw < 0 || raw > 4095) return false;
    int normalized = (int)(state->normalize_slope * (float)(raw - state->raw_start));
    *output = normalized > state->center - 500 && normalized < state->center + 500
        ? state->midpoint : (int)(normalized * state->output_slope + state->output_start);
    return true;
}

size_t kb_ascii_format(float value, char *buffer, size_t capacity) {
    if (!buffer || capacity == 0 || !isfinite(value)) return 0;
    int count = snprintf(buffer, capacity, "%f", (double)value);
    if (count <= 0 || (size_t)count >= capacity || count > KB_ASCII_PACKET_MAX) {
        buffer[0] = '\0';
        return 0;
    }
    return (size_t)count;
}

bool kb_ascii_parse(const uint8_t *packet, size_t length, float *value) {
    if (!packet || !value || !length || length > KB_ASCII_PACKET_MAX
        || memchr(packet, '\0', length)) return false;
    char buffer[KB_ASCII_PACKET_MAX + 1];
    memcpy(buffer, packet, length);
    buffer[length] = '\0';
    char *end;
    errno = 0;
    float parsed = strtof(buffer, &end);
    if (end == buffer || errno == ERANGE || !isfinite(parsed)) return false;
    while (*end && isspace((unsigned char)*end)) end++;
    if (*end) return false;
    *value = parsed;
    return true;
}
