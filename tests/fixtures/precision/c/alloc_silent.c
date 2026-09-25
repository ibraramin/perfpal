/* Precision corpus — C alloc-family silent control (must stay silent). */

#include <stdlib.h>

/* SILENT ALLOC-001: the allocation is hoisted out of any loop. */
char *alloc_outside_loop(unsigned long n) {
    char *p = malloc(n); /* silent PERF-C-ALLOC-001 (outside loop) */
    return p;
}
