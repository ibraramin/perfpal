/* Precision corpus — C pointer-chasing silent control (must stay silent). */

struct Leaf {
    unsigned long value;
};

struct Mid {
    struct Leaf leaf;
};

/* SILENT LOOP-008: mid->leaf.value is only depth 2 (below the depth-3 floor). */
unsigned long shallow_chain(struct Mid *mid, unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        total += mid->leaf.value; /* silent PERF-C-LOOP-008 (depth 2) */
    }
    return total;
}
