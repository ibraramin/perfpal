#include "util.h"

/* Cross-file definition target: src/main.c calls util_init() by name, giving
 * the call graph one true cross-file edge (PERF-C-ARCH-003) and adding this
 * file to the include-graph nodes (util.c → util.h resolved quoted edge).
 * Deliberately loop/alloc/str-free so no other rule fires here. */

int util_init(int n) {
    return n * 2;
}
