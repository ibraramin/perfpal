/* End-to-end fixture: a tiny C project exercising the full pipeline.
 * `process` allocates inside a loop → exactly one PERF-C-ALLOC-001 finding
 * is expected from it (the wrapped() call goes through no macro in v1 —
 * keep the fixture dependent only on Wave 1 behavior). */

#include <stdio.h>
#include <stdlib.h>
#include "util.h"

int helper(int x) {
    return x + 1;
}

int process(int n) {
    int total = 0;
    for (int i = 0; i < n; ++i) {
        char *buf = malloc(64);
        buf[0] = (char)i;
        total += buf[0];
        free(buf);
    }
    return total + helper(n);
}

int main(void) {
    printf("%d\n", process(4));
    return 0;
}
