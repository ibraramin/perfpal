/*
 * 10_cross_file_and_arch.c — architecture-level coupling signals.
 *
 * Expected findings:
 *   PERF-C-ARCH-001  info   unresolved quoted includes (the twelve missing_* headers)
 *   PERF-C-ARCH-003  info   three cross-file call edges to 01_ and 03_ — LOW by
 *                           name matching; MEDIUM when built against the semantic
 *                           tier with libclang present
 *   PERF-C-LOOP-004  info   acc % divisor in aggregate_ranges()
 *
 * PERF-C-ARCH-002 (hot) is emitted for the examples/ component but anchored at
 * its highest-complexity function, which lives in 05_recursion_complexity.c —
 * see that file.
 *
 * Two ARCH-001 sub-cases are deliberately planted here but do NOT currently fire.
 * Both are real scanner limitations, not corpus mistakes:
 *
 *   1. CYCLE — cycle_a.h <-> cycle_b.h is a true include cycle, but each header
 *      guards its #include inside #ifndef/#endif, and the scanner's file-level
 *      pass only walks DIRECT children of the translation unit. Guarded includes
 *      (plus guarded #define/#struct/global declarations) are therefore invisible.
 *      Remove the guards and the cycle finding appears. This is the most common
 *      header shape in real C code, so the impact is broad.
 *
 *   2. HUB FAN-OUT — the threshold (>= 12) counts DISTINCT RESOLVED include
 *      targets. Unresolved quoted includes do not count, and this file resolves
 *      only 2 (perf_common.h, cycle_a.h).
 */

#include "perf_common.h"        /* real internal header */
#include "cycle_a.h"            /* real header, one half of the cycle */

/* Deliberately unresolved quoted includes: they exercise the
 * unresolved-quoted sub-case and push this file's fan-out to hub territory. */
#include "missing_alpha.h"
#include "missing_beta.h"
#include "missing_gamma.h"
#include "missing_delta.h"
#include "missing_epsilon.h"
#include "missing_zeta.h"
#include "missing_eta.h"
#include "missing_theta.h"
#include "missing_iota.h"
#include "missing_kappa.h"
#include "missing_lambda.h"
#include "missing_mu.h"

static long accumulated;

/* ARCH-003: callees defined in OTHER files of this corpus. */
void orchestrate(int n)
{
    build_row_labels(n);                  /* defined in 01_allocator_churn.c */
    accumulated += scale_checksum(n);     /* defined in 03_deep_nesting.c */
}

/* A range fold with a variable divisor in the innermost loop. */
long aggregate_ranges(const struct Range *ranges, int n, int divisor)
{
    int i;
    long acc = 0;

    for (i = 0; i < n; i++) {
        acc += scale_checksum(ranges[i].end - ranges[i].start);   /* ARCH-003 */
        acc = acc % divisor;              /* LOOP-004 (binary form — compound
                                           * assign (`%=`) is NOT modelled) */
    }
    return acc + accumulated;
}

/* File-local helper: callers here resolve locally, so no ARCH-003 edge. */
int clamp_int(int value, int lo, int hi)
{
    if (value < lo) {
        return lo;
    }
    if (value > hi) {
        return hi;
    }
    return value;
}
