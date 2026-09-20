/*
 * 01_allocator_churn.c — per-iteration heap allocation and teardown.
 *
 * Expected findings:
 *   PERF-C-ALLOC-001  hot   heap allocation inside loop, 4 sites
 *                           (malloc, strdup, calloc, and realloc in the loop)
 *   PERF-C-ALLOC-002  hot   free() inside loop, escalated by same-function
 *                           ALLOC-001 co-occurrence
 *   PERF-C-ALLOC-003  hot   realloc() additive (n + 1) growth in loop
 *   PERF-C-LOOP-002   hot   sprintf/printf family inside loop
 *   PERF-C-LOOP-004   info  i % MAX_ENTRIES — a macro divisor counts as
 *                           non-literal (a documented accepted FP class)
 *   PERF-C-STR-003    warn  unbounded sprintf
 *   PERF-C-MEM-002    warn  memcpy with a non-literal size inside a nested loop
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "perf_common.h"

static char *row_labels[MAX_ENTRIES];

/* Allocate, format, duplicate, and free one heap block per iteration. */
void build_row_labels(int rows)
{
    int i;

    for (i = 0; i < rows; i++) {
        char *scratch = malloc(LABEL_BYTES);          /* ALLOC-001 */
        if (scratch == NULL) {
            return;
        }
        sprintf(scratch, "row-%d", i);                /* STR-003 + LOOP-002 */
        row_labels[i % MAX_ENTRIES] = strdup(scratch); /* ALLOC-001 */
        free(scratch);                                /* ALLOC-002 */
    }
}

/* Additive growth: realloc(p, n + 1) copies the whole block every iteration. */
int *grow_one_at_a_time(int n)
{
    int *buffer = NULL;
    int i;

    for (i = 0; i < n; i++) {
        buffer = realloc(buffer, (i + 1) * sizeof(int));   /* ALLOC-003 */
        if (buffer == NULL) {
            return NULL;
        }
        buffer[i] = i * 2;
        printf("%d\n", buffer[i]);                          /* LOOP-002 */
    }
    return buffer;
}

/* calloc + block copy + free, inside a two-level nest. */
void grid_reset(char **grid, int rows, int cols, int width)
{
    int r;
    int c;

    for (r = 0; r < rows; r++) {
        for (c = 0; c < cols; c++) {
            char *cell = calloc(1, (size_t)width);   /* ALLOC-001 */
            if (cell == NULL) {
                continue;
            }
            memcpy(cell, grid[c], width);            /* MEM-002 (innermost) */
            free(grid[c]);                           /* ALLOC-002 */
            grid[c] = cell;
        }
    }
}
