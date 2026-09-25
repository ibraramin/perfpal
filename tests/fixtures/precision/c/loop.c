/* Precision corpus — C pointer-chasing chains (PERF-C-LOOP-008): planted
   true positive. Silent controls live in loop_silent.c. */

struct Leaf {
    unsigned long value;
};

struct Mid {
    struct Leaf leaf;
};

struct Top {
    struct Mid mid;
};

/* TP LOOP-008: top->mid.leaf.value is a depth-3 chain in the innermost
   loop; the finding anchors at the chain. */
unsigned long chase_chain(struct Top *top, unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        total += top->mid.leaf.value; /* TP PERF-C-LOOP-008 */
    }
    return total;
}
