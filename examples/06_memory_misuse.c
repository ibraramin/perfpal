/*
 * 06_memory_misuse.c — sizeof-misuse shapes and non-literal block sizes.
 *
 * Expected findings:
 *   PERF-C-MEM-001  warn  pointer_param_sizeof (sizeof of a pointer parameter)
 *   PERF-C-MEM-001  warn  multiply_sizeof (sizeof(T) * sizeof(U))
 *   PERF-C-MEM-001  warn  divide_by_sizeof (n / sizeof(T))
 *   PERF-C-MEM-001  warn  sizeof_of_sizeof (sizeof(sizeof(...)))
 *   PERF-C-MEM-002  warn  memcpy/memmove/memset with a non-literal size in a
 *                         loop — 4 sites, including the memset() in
 *                         alloc_correct() below (`sizeof *values` is not a
 *                         literal, so the "correct" function still reports here)
 */

#include <stdlib.h>
#include <string.h>

#include "perf_common.h"

/* sizeof() applied to a pointer *parameter*: sizes a pointer, not a pointee. */
void alloc_for_source(char *source, int count)
{
    char *copy = malloc(count * sizeof(source));   /* MEM-001 */
    if (copy == NULL) {
        return;
    }
    memcpy(copy, source, count);
    free(copy);
}

/* Three more misuse shapes in one function. */
void alloc_shapes(int count)
{
    int *a = malloc(sizeof(int) * sizeof(long));   /* MEM-001 multiply_sizeof */
    int *b = malloc(count / sizeof(int));          /* MEM-001 divide_by_sizeof */
    int *c = malloc(sizeof(sizeof(int)));          /* MEM-001 sizeof_of_sizeof */

    free(a);
    free(b);
    free(c);
}

/* Non-literal block sizes inside a nested loop. */
void refill_cells(char **cells, int rows, int cols, size_t cell_size)
{
    int r;
    int c;

    for (r = 0; r < rows; r++) {
        for (c = 0; c < cols; c++) {
            memcpy(cells[c], cells[c + 1], cell_size);   /* MEM-002 (innermost) */
            memmove(cells[c], cells[c + 1], cols * 2);   /* MEM-002 (innermost) */
            memset(cells[c], 0, cell_size);              /* MEM-002 (innermost) */
        }
    }
}

/* Correct sizeof usage: silent for MEM-001, for contrast. MEM-002 still
 * reports on the memset() — `sizeof *values` is a non-literal size. */
void alloc_correct(int count)
{
    int *values = malloc((size_t)count * sizeof *values);
    int i;

    if (values == NULL) {
        return;
    }
    for (i = 0; i < count; i++) {
        memset(values + i, 0, sizeof *values);
    }
    free(values);
}
