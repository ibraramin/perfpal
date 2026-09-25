/* Precision corpus — C cross-file call coupling (PERF-C-ARCH-003): planted
   true positive. Silent controls live in arch_silent.c. */

/* TP ARCH-003: compute_total is defined in exactly one other file. */
unsigned long arch_call_unique(unsigned long x) {
    return compute_total(x); /* TP PERF-C-ARCH-003 */
}
