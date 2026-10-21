#ifndef KB_BASIC_H
#define KB_BASIC_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Logical origin is top left; bit y in columns[x] denotes a lit pixel.
 * Packing targets the classic KidBright HT16K33 wiring only. Numeric glyphs
 * are a new 5x7 font, not a byte-for-byte copy of the original font engine. */
typedef struct { uint8_t columns[16]; } kb_matrix_t;
void kb_matrix_clear(kb_matrix_t *frame);
bool kb_matrix_pixel(kb_matrix_t *frame, unsigned x, unsigned y, bool on);
bool kb_matrix_get(const kb_matrix_t *frame, unsigned x, unsigned y, bool *on);
bool kb_matrix_shift(kb_matrix_t *frame, int columns);
bool kb_matrix_digit(kb_matrix_t *frame, unsigned digit, unsigned x, unsigned y);
bool kb_matrix_pack(const kb_matrix_t *frame, uint8_t packet[17]);

/* MIDI note numbers use A4=69=440 Hz. Beats are quarter-note units.
 * Conversion validates inputs and leaves output unchanged on invalid input. */
bool kb_music_frequency(unsigned midi_note, uint32_t *hertz);
bool kb_music_duration(double bpm, double beats, uint32_t *milliseconds);
#endif
