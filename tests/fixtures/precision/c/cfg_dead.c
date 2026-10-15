/* Precision corpus — cfg-gated reachability (fix 2).
   Code inside `#if 0` never compiles, so findings there are false positives.
   `#ifdef`-guarded code (the everyday header shape) stays live and must keep
   firing. */

#include <stdlib.h>

/* TP: live allocation inside a loop still fires. */
unsigned long live_alloc(unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        char *p = malloc(16);
        if (p != 0) {
            total += (unsigned long)p[0];
            free(p);
        }
    }
    return total;
}

#if 0
/* SILENT: provably dead — the compiler never sees this function. */
unsigned long dead_alloc(unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        char *q = malloc(32);
        if (q != 0) {
            total += (unsigned long)q[0];
            free(q);
        }
    }
    return total;
}
#endif

#ifdef USE_EXTRA
/* TP: conditionally compiled code is still live code and must keep firing. */
unsigned long extra_alloc(unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        char *r = malloc(64);
        if (r != 0) {
            total += (unsigned long)r[0];
            free(r);
        }
    }
    return total;
}
#endif
