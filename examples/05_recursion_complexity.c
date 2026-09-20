/*
 * 05_recursion_complexity.c — direct recursion and extreme branching.
 *
 * Expected findings:
 *   PERF-C-CALL-001  info   direct recursion (fib, ackermann)
 *   PERF-C-CALL-002  warn   cyclomatic complexity above the threshold (>= 15),
 *                           anchored at classify()
 *   PERF-C-ARCH-002  hot    hot-component pick for the examples/ component —
 *                           anchored at classify(), this component's
 *                           highest-complexity function
 */

#include <stdio.h>

/* Classic exponential recursion; no memoization. */
long fib(long n)
{
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);          /* CALL-001 */
}

/* Deeply recursive by construction (the origin of the "Ackermann" cost). */
long ackermann(long m, long n)
{
    if (m == 0) {
        return n + 1;
    }
    if (n == 0) {
        return ackermann(m - 1, 1);          /* CALL-001 */
    }
    return ackermann(m - 1, ackermann(m, n - 1));   /* CALL-001 */
}

/* A flat classifier with well over 15 branch points: the CALL-002 target. */
const char *classify(int code, int flags)
{
    if (code < 0) {
        return "negative";
    }
    if (code == 0) {
        return "zero";
    }
    if (code < 10) {
        return "small";
    }
    if (code < 20) {
        return "medium";
    }
    if (code < 30) {
        return "large";
    }
    if (code < 40) {
        return "extra-large";
    }
    if (code < 50) {
        return "jumbo";
    }
    if (code < 60) {
        return "huge";
    }
    if (flags & 0x01) {
        return "flag-a";
    }
    if (flags & 0x02) {
        return "flag-b";
    }
    if (flags & 0x04) {
        return "flag-c";
    }
    if (flags & 0x08) {
        return "flag-d";
    }
    if (flags & 0x10) {
        return "flag-e";
    }
    if (flags & 0x20) {
        return "flag-f";
    }
    if (flags & 0x40) {
        return "flag-g";
    }
    if (flags & 0x80) {
        return "flag-h";
    }
    if (code > 1000 && flags == 0) {
        return "big-clean";
    }
    if (code > 2000 || flags != 0) {
        return "big-dirty";
    }
    while (code > 100) {
        code -= 7;
    }
    switch (code % 5) {
    case 0:
        return "r0";
    case 1:
        return "r1";
    case 2:
        return "r2";
    case 3:
        return "r3";
    default:
        return "r4";
    }
}

/* Iterative fib: silence in contrast to fib() above. */
long fib_iterative(long n)
{
    long a = 0;
    long b = 1;
    long i;

    for (i = 0; i < n; i++) {
        long next = a + b;
        a = b;
        b = next;
    }
    return a;
}
