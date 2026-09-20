/*
 * 08_globals_and_chains.c — global writes, indirect calls, pointer chasing.
 *
 * Expected findings:
 *   PERF-C-LOOP-005  info  file-scope globals written inside a loop
 *   PERF-C-LOOP-006  info  indirect call through a function pointer in the
 *                          innermost loop
 *   PERF-C-LOOP-008  info  member-access chain of depth >= 3 in the innermost
 *                          loop (pointer-chasing candidate)
 *   PERF-C-LOOP-002  hot   printf() inside the loop
 *   PERF-C-LOOP-001  warn  loop nesting depth >= 3
 */

#include <stdio.h>

static long total;          /* global: LOOP-005 target */
static int cfg_depth;       /* global: LOOP-005 target */
static int last_handler;    /* global: LOOP-005 target */

typedef int (*handler_fn)(int);

struct Ops {
    int bias;
};

struct Ctx {
    struct Ops *ops;
    int depth;
};

struct Engine {
    struct Ctx *ctx;
    int id;
};

struct Session {
    struct Engine *engine;
    const char *name;
};

/* Indirect dispatch plus deep member chains, per innermost iteration. */
void dispatch_all(struct Session *sessions, handler_fn *handlers, int n, int m)
{
    int i;
    int j;

    for (i = 0; i < n; i++) {
        for (j = 0; j < m; j++) {
            total += handlers[j](i);                     /* LOOP-006 + LOOP-005 */
            cfg_depth = sessions[i].engine->ctx->depth;   /* LOOP-008 + LOOP-005 */
            last_handler = j;                            /* LOOP-005 */
            printf("%d %d\n", total, cfg_depth);         /* LOOP-002 */
        }
    }
}

/* Third nesting level: depth 3 overall, with more global stores. */
void fold_three_ways(int a, int b, int c, int step)
{
    int i;
    int j;
    int k;

    for (i = 0; i < a; i++) {
        for (j = 0; j < b; j++) {
            for (k = 0; k < c; k++) {
                total += i * step;                 /* LOOP-005 */
                cfg_depth = k;                     /* LOOP-005 */
            }
        }
    }
}

/* Direct call in a loop: silence (LOOP-006 is indirect-only). */
int sum_direct(int n)
{
    int acc = 0;
    int i;

    for (i = 0; i < n; i++) {
        acc += i;
    }
    return acc;
}
