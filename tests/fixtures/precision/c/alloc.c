/* Precision corpus — C alloc family (PERF-C-ALLOC-001): planted true
   positive. Silent controls live in alloc_silent.c. */

#include <stdlib.h>

/* TP ALLOC-001: malloc inside the loop; the finding anchors at the call. */
unsigned long alloc_in_loop(unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        char *p = malloc(16); /* TP PERF-C-ALLOC-001 */
        if (p != 0) {
            total += (unsigned long)p[0];
            free(p);
        }
    }
    return total;
}
