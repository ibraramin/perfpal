/* Precision corpus — C block-copy silent control (must stay silent). */

#include <string.h>

/* SILENT MEM-002: a literal byte count stays silent. */
void copy_fixed(char *dst, const char *src, unsigned long n) {
    for (unsigned long i = 0; i < n; i++) {
        memcpy(dst, src, 8); /* silent PERF-C-MEM-002 (literal size) */
    }
}
