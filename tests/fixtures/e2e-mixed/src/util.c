#include "util.h"

/* Same-language cross-file definition target for src/main.c's util_init call
 * (PERF-C-ARCH-003). Deliberately loop/alloc/string-free so no other C rule
 * fires here. */
int util_init(int n) {
    return n * 2;
}
