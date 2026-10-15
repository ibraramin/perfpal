/* Precision corpus — PERF-C-ALLOC-003 capacity-guard FP class (fix 4).
   A `realloc` that runs only when a capacity check demands it
   (`if (len >= cap)`) is guarded growth, not blind per-iteration
   reallocation. Unguarded additive growth must keep firing. */

#include <stdlib.h>

/* TP: unguarded additive growth still fires. */
char *grow_unguarded(char *buf, int n, int chunk) {
    size_t len = 16;
    for (int i = 0; i < n; ++i) {
        len += chunk;
        buf = realloc(buf, len + 1);
    }
    return buf;
}

/* SILENT: capacity-guarded growth — realloc runs only when needed. */
char *grow_guarded(char *buf, int n, int chunk, size_t cap) {
    size_t len = 16;
    for (int i = 0; i < n; ++i) {
        len += chunk;
        if (len >= cap) {
            buf = realloc(buf, len + 1);
        }
    }
    return buf;
}
