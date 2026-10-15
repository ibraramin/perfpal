/* Precision corpus — PERF-C-LOOP-002 do-while(0) FP class (fix 1).
   A `do { ... } while(0)` body executes exactly ONCE (macro-wrapper idiom),
   so an expensive call inside it is not per-iteration churn. Genuine
   `while` / `do-while(cond)` loops must keep firing. */

#include <stdio.h>

/* TP: printf in a genuine while loop still fires. */
void genuine_while(int n) {
    int i = 0;
    while (i < n) {
        printf("i=%d\n", i);
        i++;
    }
}

/* TP: printf in a genuine do-while(cond) loop still fires — the fix gates
   on the (0) condition text, never on LoopKind::Do as such. */
void genuine_do(int n) {
    int i = 0;
    do {
        printf("do=%d\n", i);
        i++;
    } while (i < n);
}

/* SILENT: do { ... } while(0) wrapper body — runs once, not a loop. */
void wrapper_once(void) {
    do {
        printf("once\n");
    } while (0);
}
