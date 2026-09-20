/*
 * perf_common.h — shared declarations for the Perfpal example corpus.
 *
 * This header is a *real* internal include edge: several example programs
 * include it, which lets the include graph show internal fan-in.
 */
#ifndef PERF_COMMON_H
#define PERF_COMMON_H

#define LABEL_BYTES 64
#define MAX_ENTRIES 1024

struct Range {
    long start;
    long end;
};

/* Defined in 03_deep_nesting.c — the ARCH-003 cross-file call target. */
long scale_checksum(long value);

/* Defined in 01_allocator_churn.c — a second ARCH-003 cross-file edge target.
 * A prototype is not a definition, so the name-match edge still fires. */
void build_row_labels(int rows);

int clamp_int(int value, int lo, int hi);

#endif /* PERF_COMMON_H */
