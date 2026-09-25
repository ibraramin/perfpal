/* Precision corpus — C string accumulation (PERF-C-STR-001): planted true
   positive. Silent controls live in str_silent.c. */

#include <string.h>

/* TP STR-001: strcat into an identifier destination inside the loop. */
void accumulate(char *dst, const char *src, unsigned long n) {
    for (unsigned long i = 0; i < n; i++) {
        strcat(dst, src); /* TP PERF-C-STR-001 */
    }
}
