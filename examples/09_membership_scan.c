/*
 * 09_membership_scan.c — hand-rolled O(n^2) lookups.
 *
 * Expected findings:
 *   PERF-C-LOOP-007  info  comparison against an array element indexed by an
 *                          induction variable, nested at depth >= 2
 *   PERF-C-LOOP-001  warn  loop nesting depth >= 3 (scan_triple)
 *   PERF-C-LOOP-004  info  variable-divisor modulo in the innermost loop
 */

#include <stdio.h>

/* Classic linear membership test nested inside another loop. */
int contains_any(const int *haystack, int hlen, const int *needles, int nlen)
{
    int i;
    int j;
    int hits = 0;

    for (i = 0; i < nlen; i++) {
        for (j = 0; j < hlen; j++) {
            if (haystack[j] == needles[i]) {    /* LOOP-007 */
                hits++;
                break;
            }
        }
    }
    return hits;
}

/* Same shape with several comparisons per inner iteration. */
int find_slot(const int *table, int len, int key, int n)
{
    int r;
    int c;

    for (r = 0; r < n; r++) {
        for (c = 0; c < len; c++) {
            if (table[c] == key) {              /* LOOP-007 */
                return c;
            }
            if (table[c] < key) {               /* LOOP-007 */
                continue;
            }
        }
    }
    return -1;
}

/* Depth-3 scan with a variable modulo in the innermost body. */
int scan_triple(const int *table, int len, int key, int bucket)
{
    int r;
    int c;
    int d;
    int found = 0;

    for (r = 0; r < bucket; r++) {
        for (c = 0; c < len; c++) {
            for (d = 0; d < len; d++) {
                if (table[d] == key) {          /* LOOP-007 */
                    found++;
                }
                if ((table[d] % bucket) == r) { /* LOOP-007 + LOOP-004 */
                    found++;
                }
            }
        }
    }
    return found;
}

/* Sentinel comparison against a LITERAL: excluded by design (non-literal
 * guard on the opposite side is what makes a real lookup candidate). */
int scan_cstr(const char *text, int len)
{
    int i;
    int j;

    for (i = 0; i < len; i++) {
        for (j = 0; j < len; j++) {
            if (text[j] == '\0') {
                return j;
            }
        }
    }
    return -1;
}
