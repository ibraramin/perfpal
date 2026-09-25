/* Precision corpus — C cross-file coupling silent control. */

/* SILENT ARCH-003: `new` is defined in TWO other files — ambiguous. */
unsigned long arch_call_ambiguous(unsigned long y) {
    return new(y); /* silent PERF-C-ARCH-003 (ambiguous callee) */
}
