/*
 * 10_cross_file_and_arch.c — architecture-level coupling signals.
 *
 * Expected findings:
 *   PERF-C-ARCH-001  info   unresolved quoted includes (the twelve missing_* headers)
 *                           — plus the include cycle reported under cycle_b.h
 *   PERF-C-ARCH-003  info   three cross-file call edges to 01_ and 03_ — LOW by
 *                           name matching; MEDIUM when built against the semantic
 *                           tier with libclang present
 *   PERF-C-LOOP-004  info   acc % divisor in aggregate_ranges()
 *
 * PERF-C-ARCH-002 (hot) is emitted for the examples/ component but anchored at
 * its highest-complexity function, which lives in 05_recursion_complexity.c —
 * see that file.
 *
 * Two ARCH-001 sub-cases are deliberately planted here:
 *
 *   1. CYCLE — cycle_a.h <-> cycle_b.h is a true include cycle. Both headers
 *      guard their #include inside #ifndef/#endif; the scanner's recursive
 *      file-level pass collects guarded content, so the cycle DOES report as
 *      PERF-C-ARCH-001 (warn / HIGH confidence). The anchor is the lowest-sorted
 *      broken edge (cycle_b.h -> cycle_a.h), so the finding is listed under
 *      cycle_b.h, not here.
 *
 *   2. HUB FAN-OUT — planted but does NOT fire. The threshold (>= 12) counts
 *      DISTINCT RESOLVED include targets. Unresolved quoted includes do not
 *      count, and this file resolves only 2 (perf_common.h, cycle_a.h).
 */

#include "perf_common.h"        /* real internal header */
#include "cycle_a.h"            /* real header, one half of the cycle */

/* Deliberately unresolved quoted includes: they exercise the
 * unresolved-quoted sub-case. Note they do NOT count toward the hub fan-out
 * threshold, which sums distinct RESOLVED targets only. */
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
