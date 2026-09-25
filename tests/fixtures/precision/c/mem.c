/* Precision corpus — C block-copy family (PERF-C-MEM-002): planted true
   positive. Silent controls live in mem_silent.c. */

#include <string.h>

/* TP MEM-002: memcpy with a non-literal size inside the loop. */
void copy_blocks(char *dst, const char *src, unsigned long len, unsigned long n) {
    for (unsigned long i = 0; i < n; i++) {
        memcpy(dst, src, len); /* TP PERF-C-MEM-002 */
    }
}
