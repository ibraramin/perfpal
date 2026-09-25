/* Precision corpus — C cross-file callee definitions (part 1). */

unsigned long compute_total(unsigned long x) {
    return x + 1;
}

/* One of TWO same-named file-local helpers. The internal linkage is what
   keeps the name genuinely ambiguous for BOTH the syntactic uniqueness gate
   and the semantic resolver (each TU's USR is file-scoped). */
static unsigned long new(unsigned long y) {
    return y;
}
