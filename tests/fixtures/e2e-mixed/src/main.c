/* End-to-end MIXED C+Rust fixture — C side (plan `rust-backend`, R5 lane C1).
 *
 * C rules deliberately fired by `c_loop`:
 *   - PERF-C-ALLOC-001  (malloc inside the for loop)
 *   - PERF-C-ALLOC-002  (free inside the same loop + alloc co-occurrence)
 *   - PERF-C-STR-001    (strcat accumulation inside the loop)
 *   - PERF-C-STR-003    (unbounded strcat anywhere)
 *
 * Cross-file, SAME language: `util_init` is defined in src/util.c, so the
 * call below emits PERF-C-ARCH-003 and one call_graph edge
 * (src/main.c#c_loop -> src/util.c).
 *
 * Cross-file, CROSS language (deliberate): `rust_surface` is defined in
 * src/lib.rs. `cross_file_call_edges` keys its definition map on
 * `(language, name)` and checks resolved sites against the same-language
 * file set (src/analyzer/heuristics/arch.rs), so this call emits NO edge —
 * the mirror-direction call lives in src/lib.rs. The corpus locks that gate;
 * see tests/mixed_language_e2e.rs. */
#include <stdlib.h>
#include <string.h>
#include "util.h"

int c_loop(int n) {
    char *acc = malloc(8);
    int i;
    for (i = 0; i < n; i++) {
        char *tmp = malloc(16);
        strcat(acc, "x");
        free(tmp);
    }
    free(acc);
    return util_init(n) + rust_surface(n);
}
