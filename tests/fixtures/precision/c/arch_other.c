/* Precision corpus — C cross-file callee definitions (part 2). */

/* The SECOND same-named file-local helper; its existence makes `new`
   ambiguous at every tier. */
static unsigned long new(unsigned long y) {
    return y + 2;
}
