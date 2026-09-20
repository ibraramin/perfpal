/*
 * cycle_b.h — the other half of the cyclic include pair (with cycle_a.h).
 *
 * ARCH-001 cycle sub-case: cycle_a.h -> cycle_b.h -> cycle_a.h. Both includes
 * sit inside their #ifndef guards and the scanner's recursive file-level pass
 * collects guarded content, so the cycle reports as PERF-C-ARCH-001 (warn /
 * HIGH confidence). The finding anchors HERE because the rule breaks the
 * lowest-sorted edge cycle_b.h -> cycle_a.h.
 *
 * Expected findings:
 *   PERF-C-ARCH-001  warn  include-graph cycle with cycle_a.h (evidence kind=cycle)
 */
#ifndef CYCLE_B_H
#define CYCLE_B_H

#include "cycle_a.h"

struct CycleB {
    int id;
    struct CycleA *peer;
};

#endif /* CYCLE_B_H */
