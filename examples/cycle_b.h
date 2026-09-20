/*
 * cycle_b.h — the other half of the cyclic include pair (with cycle_a.h).
 *
 * ARCH-001 cycle sub-case: cycle_a.h -> cycle_b.h -> cycle_a.h. Like its pair,
 * this include sits inside the #ifndef guard and therefore does NOT report under
 * the current scanner (see cycle_a.h and 10_cross_file_and_arch.c). Removing
 * both guards makes the cycle finding appear.
 */
#ifndef CYCLE_B_H
#define CYCLE_B_H

#include "cycle_a.h"

struct CycleB {
    int id;
    struct CycleA *peer;
};

#endif /* CYCLE_B_H */
