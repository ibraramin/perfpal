/*
 * 03_deep_nesting.c — loop nesting, variable-divisor division, global writes.
 *
 * Expected findings:
 *   PERF-C-LOOP-001   warn   loop nesting depth >= 3
 *   PERF-C-LOOP-004   info   % and / by a non-literal divisor, innermost loop
 *   PERF-C-LOOP-005   info   global variable written inside a loop
 *   PERF-C-LOOP-002   hot    printf() inside a loop
 *
 * This file DEFINES scale_checksum(), the target of an ARCH-003 cross-file edge.
 * The finding is anchored at the CALLER, so it is emitted in
 * 10_cross_file_and_arch.c — never here.
 */

#include <stdio.h>

#include "perf_common.h"

static long checksum;          /* file-scope: a LOOP-005 target */
static int error_count;        /* file-scope: a second target */
static long last_value;

/* Cross-file call target (see 10_cross_file_and_arch.c). */
long scale_checksum(long value)
{
    return value * 3 + 1;
}

/* Three levels of nesting with real integer division in the innermost body. */
void triple_nested(int n, int m, int k, int divisor)
{
    int i;
    int j;
    int l;

    for (i = 0; i < n; i++) {
        for (j = 0; j < m; j++) {
            for (l = 0; l < k; l++) {
                int remainder = (i + j + l) % divisor;   /* LOOP-004 */
                int quotient = (i * l) / divisor;        /* LOOP-004 */
                checksum += remainder + quotient;        /* LOOP-005 */
                error_count = remainder == 0 ? 1 : 0;    /* LOOP-005 */
                printf("%d %d\n", remainder, quotient);  /* LOOP-002 */
            }
        }
    }
}

/* Same division shape, one level deep (still innermost here). */
long reduce_series(int n, int step, int modulus)
{
    long acc = 0;
    int i;

    for (i = 0; i < n; i++) {
        acc += (i * step) % modulus;      /* LOOP-004 */
        acc = acc / modulus;              /* LOOP-004 (binary form — compound
                                           * assign (`/=`) is NOT modelled) */
        last_value = acc;                 /* LOOP-005 */
    }
    return acc;
}
