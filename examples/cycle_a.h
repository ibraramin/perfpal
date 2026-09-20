/*
 * cycle_a.h — one half of a deliberately cyclic include pair (with cycle_b.h).
 *
 * ARCH-001 cycle sub-case: cycle_a.h -> cycle_b.h -> cycle_a.h is a real cycle.
 * The include below sits inside the #ifndef guard — the everyday header shape —
 * and the scanner's recursive file-level pass collects guarded content, so the
 * cycle IS reported. The finding anchors at the lowest-sorted broken edge
 * (cycle_b.h -> cycle_a.h), so it is listed under cycle_b.h, not here. See
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
