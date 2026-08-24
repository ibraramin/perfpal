/* End-to-end fixture: a tiny C project exercising the full pipeline.
 * Wave A coverage through the real tree-sitter-c backend:
 *   - process  → PERF-C-ALLOC-001 (+ PERF-C-ALLOC-002 via free-in-loop
 *                with same-function alloc co-occurrence)
 *   - drain    → PERF-C-ALLOC-002 (plain free-in-loop)
 *   - join     → PERF-C-STR-001 + PERF-C-STR-003 (strcat accumulation
 *                in a while loop double-fires both string rules)
 *   - deep_sum → PERF-C-LOOP-001 (triple-nested for) */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
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

void drain(char **bufs, int n) {
    for (int i = 0; i < n; ++i) {
        free(bufs[i]);
    }
}

void join(const char *part, int n) {
    char out[128] = "";
    int i = 0;
    while (i < n) {
        strcat(out, part);
        ++i;
    }
}

int deep_sum(int n) {
    int acc = 0;
    for (int i = 0; i < n; ++i) {
        for (int j = 0; j < n; ++j) {
            for (int k = 0; k < n; ++k) {
                acc += i + j + k;
            }
        }
    }
    return acc;
}

int main(void) {
    printf("%d %d\n", process(4), deep_sum(2));
    return 0;
}
