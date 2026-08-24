/* End-to-end fixture: a tiny C project exercising the full pipeline.
 * Wave A coverage through the real tree-sitter-c backend:
 *   - process  → PERF-C-ALLOC-001 (+ PERF-C-ALLOC-002 via free-in-loop
 *                with same-function alloc co-occurrence)
 *   - drain    → PERF-C-ALLOC-002 (plain free-in-loop)
 *   - join     → PERF-C-STR-001 + PERF-C-STR-003 (strcat accumulation
 *                in a while loop double-fires both string rules)
 *   - deep_sum → PERF-C-LOOP-001 (triple-nested for)
 * Wave B coverage:
 *   - grow     → PERF-C-ALLOC-003 (additive realloc in loop)
 *   - report   → PERF-C-LOOP-002 (printf inside loop)
 *   - deep_sum → PERF-C-LOOP-004 (variable-divisor / in innermost loop)
 *   - dispatch → PERF-C-CALL-002 (cyclomatic complexity ≥ 15)
 * Cross-file (P4): main() calls util_init() defined in src/util.c —
 *   PERF-C-ARCH-003 cross-file call edge + call_graph edge. */

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

int deep_sum(int n, int d) {
    int acc = 0;
    for (int i = 0; i < n; ++i) {
        for (int j = 0; j < n; ++j) {
            for (int k = 0; k < n; ++k) {
                acc += i / d + j + k;
            }
        }
    }
    return acc;
}

char *grow(char *buf, int n, int chunk) {
    size_t len = 16;
    for (int i = 0; i < n; ++i) {
        len += chunk;
        buf = realloc(buf, len + 1);
    }
    return buf;
}

void report(int n) {
    for (int i = 0; i < n; ++i) {
        printf("row %d\n", i);
    }
}

int dispatch(int cmd, int a, int b) {
    if (cmd == 1) { return a + b; }
    if (cmd == 2) { return a - b; }
    if (cmd == 3) { return a * b; }
    if (cmd == 4) { return a / (b ? b : 1); }
    if (cmd == 5) { return a % (b ? b : 1); }
    if (cmd == 6) { return a << 1; }
    if (cmd == 7) { return a >> 1; }
    if (cmd == 8 && a > b) { return a; }
    if (cmd == 9 || b > a) { return b; }
    if (cmd == 10 ? a > 0 : b > 0) { return a > b ? a : b; }
    switch (cmd) {
    case 11: return 11;
    case 12: return 12;
    case 13: return 13;
    default: break;
    }
    while (a > b) { --a; }
    do { ++b; } while (b < a);
    return cmd;
}

int main(void) {
    printf("%d %d\n", process(4), deep_sum(2, 3));
    char *buf = malloc(16);
    printf("%s\n", grow(buf, 4, 8));
    report(2);
    printf("%d\n", dispatch(5, 7, 3));
    printf("%d\n", util_init(21));
    return 0;
}
