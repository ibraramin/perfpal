/*
 * cycle_a.h — one half of a deliberately cyclic include pair (with cycle_b.h).
 *
 * ARCH-001 cycle sub-case: cycle_a.h -> cycle_b.h -> cycle_a.h is a real cycle,
 * but the include below is nested inside the #ifndef guard and the scanner's
 * file-level pass only walks direct children of the translation unit, so guarded
 * includes are currently invisible — this cycle does NOT report today. Remove
 * both guards and the ARCH-001 cycle finding appears. See
 * 10_cross_file_and_arch.c for the full explanation.
 */
#ifndef CYCLE_A_H
#define CYCLE_A_H

#include "cycle_b.h"

struct CycleA {
    int id;
    struct CycleB *peer;
};

#endif /* CYCLE_A_H */
