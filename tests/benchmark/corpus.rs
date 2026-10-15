//! M2 benchmark corpus generator (plan `evidence-beta` M2, Lane B extension).
//!
//! This module is the SINGLE source of truth for the planted mutations behind
//! the benchmark harness. Every rung is built by composing "units":
//!
//! - the `small` rung plants ONE true positive plus ONE silent control for
//!   EVERY active rule in the 67-rule catalog (23 C + 22 Python + 22 Rust) —
//!   plus the retired `PERF-RS-STR-001` always-silent control. The nine
//!   existential gate constructs are lifted VERBATIM from the committed M1
//!   corpus (`tests/fixtures/precision/`), with every function/definition name
//!   parameterized by a per-replica suffix so replicas can never collide on
//!   the cross-file name-uniqueness gate that ARCH-003 depends on;
//! - `medium`/`large` replicate the NINE gate rules UNCHANGED (the M1
//!   KPI set). The harness scopes each rung's ratchet to the rules that rung
//!   actually plants, so the added small-rung coverage never inflates the
//!   scale-ladder rule set.
//!
//! ## Extension point
//!
//! `gate_c_unit` / `gate_rust_unit` pin the 9-rule gate set; `c_ext` /
//! `rust_ext` (and the collateral labels folded into `small`) pin the
//! remaining rules. `tests/benchmark/mod.rs` and `tests/benchmark.rs` know
//! only about labels, never rule ids.

use super::{Expect, GenFile, Label, Rung};

// ---------------------------------------------------------------------------
// Scoped rule ids (C + Rust). The nine gate rules keep their M1 spelling;
// `PERF-RS-STR-001` is retired (no emitter) and stays as an always-silent
// control, exactly as in M1.
// ---------------------------------------------------------------------------

const C_ALLOC_001: &str = "PERF-C-ALLOC-001";
const C_ALLOC_002: &str = "PERF-C-ALLOC-002";
const C_ALLOC_003: &str = "PERF-C-ALLOC-003";
const C_ARCH_001: &str = "PERF-C-ARCH-001";
const C_ARCH_002: &str = "PERF-C-ARCH-002";
const C_ARCH_003: &str = "PERF-C-ARCH-003";
const C_CALL_001: &str = "PERF-C-CALL-001";
const C_CALL_002: &str = "PERF-C-CALL-002";
const C_LOOP_001: &str = "PERF-C-LOOP-001";
const C_LOOP_002: &str = "PERF-C-LOOP-002";
const C_LOOP_003: &str = "PERF-C-LOOP-003";
const C_LOOP_004: &str = "PERF-C-LOOP-004";
const C_LOOP_005: &str = "PERF-C-LOOP-005";
const C_LOOP_006: &str = "PERF-C-LOOP-006";
const C_LOOP_007: &str = "PERF-C-LOOP-007";
const C_LOOP_008: &str = "PERF-C-LOOP-008";
const C_MEM_001: &str = "PERF-C-MEM-001";
const C_MEM_002: &str = "PERF-C-MEM-002";
const C_STR_001: &str = "PERF-C-STR-001";
const C_STR_002: &str = "PERF-C-STR-002";
const C_STR_003: &str = "PERF-C-STR-003";
const C_TYPE_001: &str = "PERF-C-TYPE-001";
const C_TYPE_002: &str = "PERF-C-TYPE-002";

const RS_ALLOC_001: &str = "PERF-RS-ALLOC-001";
const RS_ALLOC_002: &str = "PERF-RS-ALLOC-002";
const RS_ALLOC_003: &str = "PERF-RS-ALLOC-003";
const RS_ARCH_001: &str = "PERF-RS-ARCH-001";
const RS_ARCH_002: &str = "PERF-RS-ARCH-002";
const RS_ARCH_003: &str = "PERF-RS-ARCH-003";
const RS_CALL_001: &str = "PERF-RS-CALL-001";
const RS_CALL_002: &str = "PERF-RS-CALL-002";
const RS_LOOP_001: &str = "PERF-RS-LOOP-001";
const RS_LOOP_002: &str = "PERF-RS-LOOP-002";
const RS_LOOP_003: &str = "PERF-RS-LOOP-003";
const RS_LOOP_004: &str = "PERF-RS-LOOP-004";
const RS_LOOP_005: &str = "PERF-RS-LOOP-005";
const RS_LOOP_006: &str = "PERF-RS-LOOP-006";
const RS_LOOP_007: &str = "PERF-RS-LOOP-007";
const RS_LOOP_008: &str = "PERF-RS-LOOP-008";
const RS_MEM_001: &str = "PERF-RS-MEM-001";
const RS_MEM_002: &str = "PERF-RS-MEM-002";
const RS_STR_001: &str = "PERF-RS-STR-001";
const RS_STR_002: &str = "PERF-RS-STR-002";
const RS_STR_003: &str = "PERF-RS-STR-003";
const RS_TYPE_001: &str = "PERF-RS-TYPE-001";
const RS_TYPE_002: &str = "PERF-RS-TYPE-002";

const PY_ALLOC_001: &str = "PERF-PY-ALLOC-001";
const PY_ALLOC_003: &str = "PERF-PY-ALLOC-003";
const PY_ALLOC_004: &str = "PERF-PY-ALLOC-004";
const PY_ARCH_001: &str = "PERF-PY-ARCH-001";
const PY_ARCH_002: &str = "PERF-PY-ARCH-002";
const PY_ARCH_003: &str = "PERF-PY-ARCH-003";
const PY_ASYNC_001: &str = "PERF-PY-ASYNC-001";
const PY_ASYNC_002: &str = "PERF-PY-ASYNC-002";
const PY_CALL_001: &str = "PERF-PY-CALL-001";
const PY_CALL_002: &str = "PERF-PY-CALL-002";
const PY_COMPREHENSION_001: &str = "PERF-PY-COMPREHENSION-001";
const PY_EXECUTOR_001: &str = "PERF-PY-EXECUTOR-001";
const PY_LOOP_001: &str = "PERF-PY-LOOP-001";
const PY_LOOP_002: &str = "PERF-PY-LOOP-002";
const PY_LOOP_003: &str = "PERF-PY-LOOP-003";
const PY_LOOP_007: &str = "PERF-PY-LOOP-007";
const PY_LOOP_009: &str = "PERF-PY-LOOP-009";
const PY_MEM_002: &str = "PERF-PY-MEM-002";
const PY_N1_001: &str = "PERF-PY-N1-001";
const PY_PANDAS_001: &str = "PERF-PY-PANDAS-001";
const PY_PANDAS_002: &str = "PERF-PY-PANDAS-002";
const PY_STR_001: &str = "PERF-PY-STR-001";

/// 1-based line of `marker` inside `contents`; the marker MUST be unique —
/// a duplicate or misplaced token fails loudly rather than silently anchoring
/// on the first `contains` match.
fn anchor_line(contents: &str, marker: &str) -> u32 {
    let positions: Vec<u32> = contents
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(marker))
        .map(|(idx, _)| u32::try_from(idx + 1).expect("line fits u32"))
        .collect();
    assert_eq!(
        positions.len(),
        1,
        "anchor marker {marker:?} must appear exactly once in the generated file, \
         found at line(s) {positions:?}"
    );
    positions[0]
}

fn fire(rule_id: &'static str, rel_path: String, contents: &str, marker: &str) -> Label {
    Label {
        rule_id,
        rel_path,
        line: anchor_line(contents, marker),
        expect: Expect::Fire,
    }
}

fn silent(rule_id: &'static str, rel_path: String) -> Label {
    Label {
        rule_id,
        rel_path,
        line: 0,
        expect: Expect::Silent,
    }
}

fn gf(rel_path: String, contents: String) -> GenFile {
    GenFile { rel_path, contents }
}

/// Contents of a generated file, by relative path — used to derive the
/// small rung's collateral labels from the gate templates.
fn contents_of<'a>(files: &'a [GenFile], rel_path: &str) -> &'a str {
    files
        .iter()
        .find(|f| f.rel_path == rel_path)
        .map(|f| f.contents.as_str())
        .unwrap_or_else(|| panic!("generated file {rel_path} missing"))
}

// ===========================================================================
// GATE templates (the nine M1 existential rules). `@S@` is the per-replica
// identifier/filename suffix. These bytes are unchanged from the M1 corpus so
// the committed fire anchors cannot drift.
// ===========================================================================

const C_ALLOC_TP: &str = r#"/* Precision corpus — C alloc family (PERF-C-ALLOC-001): planted true
   positive. Silent controls live in alloc_silent.c. */

#include <stdlib.h>

/* TP ALLOC-001: malloc inside the loop; the finding anchors at the call. */
unsigned long alloc_in_loop@S@(unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        char *p = malloc(16); /* TP PERF-C-ALLOC-001 */
        if (p != 0) {
            total += (unsigned long)p[0];
            free(p);
        }
    }
    return total;
}
"#;
const C_ALLOC_MARK: &str = "/* TP PERF-C-ALLOC-001 */";

const C_ALLOC_SILENT: &str = r#"/* Precision corpus — C alloc-family silent control (must stay silent). */

#include <stdlib.h>

/* SILENT ALLOC-001: the allocation is hoisted out of any loop. */
char *alloc_outside_loop@S@(unsigned long n) {
    char *p = malloc(n); /* silent PERF-C-ALLOC-001 (outside loop) */
    return p;
}
"#;

const C_ARCH_TP: &str = r#"/* Precision corpus — C cross-file call coupling (PERF-C-ARCH-003): planted
   true positive. Silent controls live in arch_silent.c. */

/* TP ARCH-003: compute_total is defined in exactly one other file. */
unsigned long arch_call_unique@S@(unsigned long x) {
    return compute_total@S@(x); /* TP PERF-C-ARCH-003 */
}
"#;
const C_ARCH_MARK: &str = "/* TP PERF-C-ARCH-003 */";

const C_ARCH_HELPERS: &str = r#"/* Precision corpus — C cross-file callee definitions (part 1). */

unsigned long compute_total@S@(unsigned long x) {
    return x + 1;
}

/* One of TWO same-named file-local helpers. The internal linkage is what
   keeps the name genuinely ambiguous for BOTH the syntactic uniqueness gate
   and the semantic resolver (each TU's USR is file-scoped). */
static unsigned long new@S@(unsigned long y) {
    return y;
}
"#;

const C_ARCH_OTHER: &str = r#"/* Precision corpus — C cross-file callee definitions (part 2). */

/* The SECOND same-named file-local helper; its existence makes `new`
   ambiguous at every tier. */
static unsigned long new@S@(unsigned long y) {
    return y + 2;
}
"#;

const C_ARCH_SILENT: &str = r#"/* Precision corpus — C cross-file coupling silent control. */

/* SILENT ARCH-003: `new` is defined in TWO other files — ambiguous. */
unsigned long arch_call_ambiguous@S@(unsigned long y) {
    return new@S@(y); /* silent PERF-C-ARCH-003 (ambiguous callee) */
}
"#;

const C_MEM_TP: &str = r#"/* Precision corpus — C block-copy family (PERF-C-MEM-002): planted true
   positive. Silent controls live in mem_silent.c. */

#include <string.h>

/* TP MEM-002: memcpy with a non-literal size inside the loop. */
void copy_blocks@S@(char *dst, const char *src, unsigned long len, unsigned long n) {
    for (unsigned long i = 0; i < n; i++) {
        memcpy(dst, src, len); /* TP PERF-C-MEM-002 */
    }
}
"#;
const C_MEM_MARK: &str = "/* TP PERF-C-MEM-002 */";

const C_MEM_SILENT: &str = r#"/* Precision corpus — C block-copy silent control (must stay silent). */

#include <string.h>

/* SILENT MEM-002: a literal byte count stays silent. */
void copy_fixed@S@(char *dst, const char *src, unsigned long n) {
    for (unsigned long i = 0; i < n; i++) {
        memcpy(dst, src, 8); /* silent PERF-C-MEM-002 (literal size) */
    }
}
"#;

const C_LOOP_TP: &str = r#"/* Precision corpus — C pointer-chasing chains (PERF-C-LOOP-008): planted
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
unsigned long chase_chain@S@(struct Top *top, unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        total += top->mid.leaf.value; /* TP PERF-C-LOOP-008 */
    }
    return total;
}
"#;
const C_LOOP_MARK: &str = "/* TP PERF-C-LOOP-008 */";

const C_LOOP_SILENT: &str = r#"/* Precision corpus — C pointer-chasing silent control (must stay silent). */

struct Leaf {
    unsigned long value;
};

struct Mid {
    struct Leaf leaf;
};

/* SILENT LOOP-008: mid->leaf.value is only depth 2 (below the depth-3 floor). */
unsigned long shallow_chain@S@(struct Mid *mid, unsigned long n) {
    unsigned long total = 0;
    for (unsigned long i = 0; i < n; i++) {
        total += mid->leaf.value; /* silent PERF-C-LOOP-008 (depth 2) */
    }
    return total;
}
"#;

const C_STR_TP: &str = r#"/* Precision corpus — C string accumulation (PERF-C-STR-001): planted true
   positive. Silent controls live in str_silent.c. */

#include <string.h>

/* TP STR-001: strcat into an identifier destination inside the loop. */
void accumulate@S@(char *dst, const char *src, unsigned long n) {
    for (unsigned long i = 0; i < n; i++) {
        strcat(dst, src); /* TP PERF-C-STR-001 */
    }
}
"#;
const C_STR_MARK: &str = "/* TP PERF-C-STR-001 */";

const C_STR_SILENT: &str = r#"/* Precision corpus — C string-accumulation silent control (must stay silent). */

#include <string.h>

/* SILENT STR-001: strcat outside any loop. */
void accumulate_once@S@(char *dst, const char *src) {
    strcat(dst, src); /* silent PERF-C-STR-001 (outside loop) */
}
"#;

const RS_ALLOC_TP: &str = r#"//! Precision corpus — Rust alloc family (PERF-RS-ALLOC-001): planted true
//! positive (REAL per-iteration alloc; zero-alloc ctors silent since fix 3; silents in alloc_silent.rs).

/// TP ALLOC-001: a single innermost loop that allocates per iteration. The
/// finding anchors at the LOOP span (the `for` line), not the alloc call.
pub fn alloc_in_loop@S@(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n { // TP PERF-RS-ALLOC-001
        let v: Vec<usize> = vec![i; 4];
        total += v.len() + i;
    }
    total
}
"#;
const RS_ALLOC_MARK: &str = "// TP PERF-RS-ALLOC-001";

const RS_ALLOC_SILENT: &str = r#"//! Precision corpus — Rust alloc-family silent controls (must stay silent).

/// SILENT ALLOC-001: the allocation is hoisted OUT of any loop.
pub fn alloc_outside_loop@S@() -> Vec<u8> {
    let buf: Vec<u8> = Vec::new(); // silent PERF-RS-ALLOC-001 (outside loop)
    buf
}

/// SILENT ALLOC-001: literal-receiver `to_string` is suppressed by the M1
/// literal-receiver gate. SILENT STR-001: the retired rule can never fire, so
/// this loop's `push` is silent for PERF-RS-STR-001 too.
pub fn literal_receiver@S@(items: &[u8], other: &mut Vec<u8>) -> usize {
    let mut count = 0usize;
    for it in items {
        let s = "x".to_string(); // silent PERF-RS-ALLOC-001 (literal receiver)
        count += s.len() + usize::from(*it);
        other.push(*it); // silent PERF-RS-STR-001 (retired rule)
    }
    count
}
"#;

const RS_ARCH_TP: &str = r#"//! Precision corpus — Rust cross-module call coupling (PERF-RS-ARCH-003):
//! planted true positive. Silent controls live in `arch_silent.rs`.

/// TP ARCH-003: `compute_total` is defined in exactly ONE other file
/// (`rust/arch_helpers.rs`), so the M1 uniqueness gate emits an edge.
pub fn call_unique@S@(x: u64) -> u64 {
    compute_total@S@(x) // TP PERF-RS-ARCH-003
}
"#;
const RS_ARCH_MARK: &str = "// TP PERF-RS-ARCH-003";

const RS_ARCH_HELPERS: &str = r#"//! Precision corpus — Rust cross-file callee definitions (part 1).

/// Uniquely defined corpus callee (the ARCH-003 true-positive target).
pub fn compute_total@S@(x: u64) -> u64 {
    x + 1
}

/// One of TWO definitions of `new` (the ambiguity half of the silent control).
pub fn new@S@(y: u64) -> u64 {
    y
}
"#;

const RS_ARCH_EXTRA: &str = r#"//! Precision corpus — Rust cross-file callee definitions (part 2).

/// The SECOND definition of `new`; its existence makes the name ambiguous.
pub fn new@S@(y: u64) -> u64 {
    y + 2
}
"#;

const RS_ARCH_SILENT: &str = r#"//! Precision corpus — Rust cross-module coupling silent control.

/// SILENT ARCH-003: `new` is defined in TWO other files (`arch_helpers.rs`
/// and `arch_extra.rs`) — ambiguous names emit no edge.
pub fn call_ambiguous@S@(y: u64) -> u64 {
    new@S@(y) // silent PERF-RS-ARCH-003 (ambiguous callee)
}
"#;

const RS_MEM_TP: &str = r#"//! Precision corpus — Rust block-copy family (PERF-RS-MEM-002): planted true
//! positive. Silent controls live in `mem_silent.rs`.

/// TP MEM-002: `copy_from_slice` is a block-copy method in an innermost loop;
/// the finding anchors at the call.
pub fn copy_block@S@(dst: &mut [u8], src: &[u8]) {
    for i in 0..src.len() {
        dst.copy_from_slice(src); // TP PERF-RS-MEM-002
        let _ = i;
    }
}
"#;
const RS_MEM_MARK: &str = "// TP PERF-RS-MEM-002";

const RS_MEM_SILENT: &str = r#"//! Precision corpus — Rust block-copy silent controls (must stay silent).

/// SILENT MEM-002: `clone` was removed from the family by the M1 gate
/// (ubiquitous ownership clones are not block copies).
pub fn clone_in_loop@S@(items: &[u8]) -> u64 {
    let mut sum = 0u64;
    for it in items {
        let c = it.clone(); // silent PERF-RS-MEM-002 (clone not in family)
        sum += u64::from(c);
    }
    sum
}

/// SILENT MEM-002: a block copy outside any loop.
pub fn copy_once@S@(dst: &mut [u8], src: &[u8]) {
    dst.copy_from_slice(src); // silent PERF-RS-MEM-002 (outside loop)
}
"#;

const RS_LOOP_TP: &str = r#"//! Precision corpus — Rust pointer/lifetime-chain chasing (PERF-RS-LOOP-008):
//! planted true positive. Silent controls live in `loop_silent.rs`.

pub struct Leaf {
    pub value: u64,
}

pub struct Row {
    pub cells: [Leaf; 4],
}

/// TP LOOP-008: an indexed chain (a subscript step) of depth 3 in an
/// innermost loop; the finding anchors at the chain.
pub fn indexed_chain@S@(row: &Row, i: usize) -> u64 {
    let mut sum = 0u64;
    for k in 0..4 {
        sum += row.cells[i].value + k as u64; // TP PERF-RS-LOOP-008
    }
    sum
}
"#;
const RS_LOOP_MARK: &str = "// TP PERF-RS-LOOP-008";

const RS_LOOP_SILENT: &str = r#"//! Precision corpus — Rust pointer/lifetime-chain silent control.

pub struct Leaf {
    pub value: u64,
}

pub struct Mid {
    pub leaf: Leaf,
}

pub struct Top {
    pub mid: Mid,
}

/// SILENT LOOP-008: a pure member chain of depth 3 with NO subscript step —
/// the M1 gate requires an index step.
pub fn member_chain_only@S@(top: &Top) -> u64 {
    let mut sum = 0u64;
    for k in 0..4 {
        sum += top.mid.leaf.value + k as u64; // silent PERF-RS-LOOP-008 (no subscript)
    }
    sum
}
"#;

// ===========================================================================
// C extension templates (the 18 non-gate C rules). Names carry a unique
// `_cNN` token so they can never collide across files or replicas.
// ===========================================================================

const C_ALLOC002_SILENT: &str = r#"/* Benchmark corpus — C dealloc silent control (PERF-C-ALLOC-002). */

#include <stdlib.h>

/* SILENT ALLOC-002: the free is hoisted out of any loop. */
void free_outside_c02(void *p) {
    free(p); /* silent PERF-C-ALLOC-002 (outside loop) */
}
"#;

const C_ALLOC003_TP: &str = r#"/* Benchmark corpus — C realloc linear growth (PERF-C-ALLOC-003). */

#include <stdlib.h>

/* TP ALLOC-003: additive realloc growth in a loop. The same call is also a
   true-positive PERF-C-ALLOC-001 (an allocation site inside a loop). */
long *grow_add_c03(long n) {
    long *buffer = 0;
    for (long i = 0; i < n; i++) {
        buffer = realloc(buffer, (i + 1) * sizeof(long)); /* TP PERF-C-ALLOC-003 */
    }
    return buffer;
}

/* SILENT ALLOC-003: pointer arithmetic in the FIRST argument (outside loop). */
void realloc_ptr_c03(char *p, unsigned long n) {
    (void)realloc(p + 1, n); /* silent PERF-C-ALLOC-003 (pointer-arith first arg) */
}

/* SILENT ALLOC-003: multiplicative growth is the documented fix, not the
   finding — the size argument is not additive. This is still an allocation
   site inside a loop, so PERF-C-ALLOC-001 fires (labelled in `small`). */
long *grow_mul_c03(long n) {
    long *buffer = 0;
    for (long i = 0; i < n; i++) {
        buffer = realloc(buffer, n * 2); /* silent PERF-C-ALLOC-003 (multiplicative) */
    }
    return buffer;
}
"#;
const C_ALLOC003_MARK: &str = "/* TP PERF-C-ALLOC-003 */";

const C_ARCH001_CYCLE_P: &str = r#"/* Benchmark corpus — C include cycle, half P (PERF-C-ARCH-001). */
#ifndef BENCH_CYCLE_P_C01_H
#define BENCH_CYCLE_P_C01_H

#include "cycle_q_c01.h"

struct CyclePC01 {
    int id;
};

#endif /* BENCH_CYCLE_P_C01_H */
"#;
const C_ARCH001_CYCLE_Q: &str = r#"/* Benchmark corpus — C include cycle, half Q (PERF-C-ARCH-001). */
#ifndef BENCH_CYCLE_Q_C01_H
#define BENCH_CYCLE_Q_C01_H

#include "cycle_p_c01.h" /* TP PERF-C-ARCH-001 (broken edge anchors here) */

struct CycleQC01 {
    int id;
};

#endif /* BENCH_CYCLE_Q_C01_H */
"#;
const C_ARCH001_MARK: &str = "/* TP PERF-C-ARCH-001 (broken edge anchors here) */";

const C_ARCH001_LEAF: &str = r#"/* Benchmark corpus — ARCH-001 resolved-leaf silent control. */
#ifndef BENCH_INC_LEAF_C01_H
#define BENCH_INC_LEAF_C01_H

struct IncLeafC01 {
    int value;
};

#endif /* BENCH_INC_LEAF_C01_H */
"#;
const C_ARCH001_USER: &str = r#"/* Benchmark corpus — ARCH-001 non-cyclic resolved include (silent). */

#include "inc_leaf_c01.h"

int inc_leaf_read_c01(void) {
    struct IncLeafC01 leaf;
    leaf.value = 0;
    return leaf.value;
}
"#;

const C_ARCH002_COLD: &str = r#"/* Benchmark corpus — ARCH-002 cold component silent control (sum < 15). */

int cold_classify_c02(int x) {
    if (x < 0) {
        return -1;
    }
    if (x == 0) {
        return 0;
    }
    return 1;
}
"#;

const C_CALL001_TP: &str = r#"/* Benchmark corpus — C direct recursion (PERF-C-CALL-001). */

long self_sum_c01(long n) {
    if (n <= 0) {
        return 0;
    }
    return n + self_sum_c01(n - 1); /* TP PERF-C-CALL-001 */
}
"#;
const C_CALL001_MARK: &str = "/* TP PERF-C-CALL-001 */";

const C_CALL001_SILENT: &str = r#"/* Benchmark corpus — CALL-001 silent control: mutual recursion. */

long odd_c01(long n);

long even_c01(long n) {
    if (n == 0) {
        return 1;
    }
    return odd_c01(n - 1); /* silent PERF-C-CALL-001 (calls a different fn) */
}

long odd_c01(long n) {
    if (n == 0) {
        return 0;
    }
    return even_c01(n - 1); /* silent PERF-C-CALL-001 */
}
"#;

const C_CALL002_TP: &str = r#"/* Benchmark corpus — C high cyclomatic complexity (PERF-C-CALL-002);
   the same function is the PERF-C-ARCH-002 hot-component anchor. */

const char *classify_c02(int code) { /* TP PERF-C-CALL-002 + TP PERF-C-ARCH-002 */
    if (code == 0) { return "v0"; }
    if (code == 1) { return "v1"; }
    if (code == 2) { return "v2"; }
    if (code == 3) { return "v3"; }
    if (code == 4) { return "v4"; }
    if (code == 5) { return "v5"; }
    if (code == 6) { return "v6"; }
    if (code == 7) { return "v7"; }
    if (code == 8) { return "v8"; }
    if (code == 9) { return "v9"; }
    if (code == 10) { return "v10"; }
    if (code == 11) { return "v11"; }
    if (code == 12) { return "v12"; }
    if (code == 13) { return "v13"; }
    if (code == 14) { return "v14"; }
    if (code == 15) { return "v15"; }
    if (code == 16) { return "v16"; }
    if (code == 17) { return "v17"; }
    return "other";
}
"#;
const C_CALL002_MARK: &str = "/* TP PERF-C-CALL-002 + TP PERF-C-ARCH-002 */";

const C_CALL002_SILENT: &str = r#"/* Benchmark corpus — CALL-002 silent control (complexity below 15). */

int low_branch_c02(int x) {
    if (x == 0) { return 0; }
    if (x == 1) { return 1; }
    if (x == 2) { return 2; }
    if (x == 3) { return 3; }
    if (x == 4) { return 4; }
    if (x == 5) { return 5; }
    if (x == 6) { return 6; }
    if (x == 7) { return 7; }
    if (x == 8) { return 8; }
    if (x == 9) { return 9; }
    return x;
}
"#;

const C_LOOP001_TP: &str = r#"/* Benchmark corpus — C loop nesting depth 3 (PERF-C-LOOP-001). */

long nest_three_c01(long n) {
    long acc = 0;
    for (long i = 0; i < n; i++) {
        for (long j = 0; j < n; j++) {
            for (long k = 0; k < n; k++) { /* TP PERF-C-LOOP-001 */
                acc += i + j + k;
            }
        }
    }
    return acc;
}
"#;
const C_LOOP001_MARK: &str = "/* TP PERF-C-LOOP-001 */";

const C_LOOP001_SILENT: &str = r#"/* Benchmark corpus — LOOP-001 silent control (depth below 3). */

long nest_two_c01(long n) {
    long acc = 0;
    for (long i = 0; i < n; i++) {
        for (long j = 0; j < n; j++) {
            acc += i + j;
        }
    }
    return acc;
}
"#;

const C_LOOP002_TP: &str = r#"/* Benchmark corpus — C stdio churn in loop (PERF-C-LOOP-002). */

#include <stdio.h>

void emit_loop_c02(int n) {
    for (int i = 0; i < n; i++) {
        printf("%d\n", i); /* TP PERF-C-LOOP-002 */
    }
}

/* SILENT LOOP-002: the same call hoisted out of any loop. */
void emit_once_c02(int x) {
    printf("%d\n", x); /* silent PERF-C-LOOP-002 (outside loop) */
}
"#;
const C_LOOP002_MARK: &str = "/* TP PERF-C-LOOP-002 */";

const C_LOOP003_TP: &str = r#"/* Benchmark corpus — C char-wise stdio in loop (PERF-C-LOOP-003). */

#include <stdio.h>

void relay_c03(FILE *in, FILE *out) {
    int ch;
    while ((ch = fgetc(in)) != EOF) {
        putc(ch, out); /* TP PERF-C-LOOP-003 */
    }
}

/* SILENT LOOP-003: char stdio outside any loop. */
void emit_char_c03(FILE *out) {
    putc('x', out); /* silent PERF-C-LOOP-003 (outside loop) */
}
"#;
const C_LOOP003_MARK: &str = "/* TP PERF-C-LOOP-003 */";

const C_LOOP004_TP: &str = r#"/* Benchmark corpus — C variable-divisor div/mod (PERF-C-LOOP-004). */

long mod_loop_c04(long n, long divisor) {
    long acc = 0;
    for (long i = 0; i < n; i++) {
        acc += i % divisor; /* TP PERF-C-LOOP-004 */
    }
    return acc;
}

/* SILENT LOOP-004: a literal divisor is strength-reduced by the compiler. */
long mod_literal_c04(long n) {
    long acc = 0;
    for (long i = 0; i < n; i++) {
        acc += i % 5; /* silent PERF-C-LOOP-004 (literal divisor) */
    }
    return acc;
}
"#;
const C_LOOP004_MARK: &str = "/* TP PERF-C-LOOP-004 */";

const C_LOOP005_TP: &str = r#"/* Benchmark corpus — C global write in loop (PERF-C-LOOP-005). */

static long g_total_c05;

long fold_global_c05(long n) {
    for (long i = 0; i < n; i++) {
        g_total_c05 += i; /* TP PERF-C-LOOP-005 */
    }
    return g_total_c05;
}

/* SILENT LOOP-005: the store target is a local. */
long fold_local_c05(long n) {
    long total = 0;
    for (long i = 0; i < n; i++) {
        total += i; /* silent PERF-C-LOOP-005 (local) */
    }
    return total;
}
"#;
const C_LOOP005_MARK: &str = "/* TP PERF-C-LOOP-005 */";

const C_LOOP006_TP: &str = r#"/* Benchmark corpus — C indirect call in loop (PERF-C-LOOP-006). */

typedef int (*handler_c06)(int);

static int direct_target_c06(int x) {
    return x + 1;
}

int dispatch_c06(handler_c06 *handlers, int n) {
    int acc = 0;
    for (int i = 0; i < n; i++) {
        acc += handlers[i](i); /* TP PERF-C-LOOP-006 */
    }
    return acc;
}

/* SILENT LOOP-006: a direct call in the loop never fires. */
int call_direct_c06(int n) {
    int acc = 0;
    for (int i = 0; i < n; i++) {
        acc += direct_target_c06(i); /* silent PERF-C-LOOP-006 (direct call) */
    }
    return acc;
}
"#;
const C_LOOP006_MARK: &str = "/* TP PERF-C-LOOP-006 */";

const C_LOOP007_TP: &str = r#"/* Benchmark corpus — C linear membership scan (PERF-C-LOOP-007). */

int contains_any_c07(const int *table, int len, int key, int n) {
    int r;
    int c;
    int hits = 0;

    for (r = 0; r < n; r++) {
        for (c = 0; c < len; c++) {
            if (table[c] == key) { /* TP PERF-C-LOOP-007 */
                hits++;
                break;
            }
        }
    }
    return hits;
}

/* SILENT LOOP-007: depth 1 is below the floor. */
int contains_shallow_c07(const int *table, int len, int key) {
    int c;

    for (c = 0; c < len; c++) {
        if (table[c] == key) { /* silent PERF-C-LOOP-007 (depth 1) */
            return 1;
        }
    }
    return 0;
}
"#;
const C_LOOP007_MARK: &str = "/* TP PERF-C-LOOP-007 */";

const C_MEM001_TP: &str = r#"/* Benchmark corpus — C sizeof misuse in allocation (PERF-C-MEM-001). */

#include <stdlib.h>

/* TP MEM-001: sizeof of a pointer parameter sizes the pointer, not the
   pointee. The call is outside any loop, so ALLOC-001 stays silent. */
void alloc_for_source_c08(char *source, int count) {
    char *copy = malloc(count * sizeof(source)); /* TP PERF-C-MEM-001 */
    if (copy == 0) {
        return;
    }
    free(copy);
}

/* SILENT MEM-001: the correct `count * sizeof(int)` / `sizeof *p` shapes. */
void alloc_correct_c08(int count) {
    int *values = malloc(count * sizeof(int)); /* silent PERF-C-MEM-001 */
    free(values);
}
"#;
const C_MEM001_MARK: &str = "/* TP PERF-C-MEM-001 */";

const C_TYPE001_TP: &str = r#"/* Benchmark corpus — C large struct by value (PERF-C-TYPE-001). */

struct BigC09 {
    double a;
    double b;
    double c;
    double d;
    long handle;
    int kind;
};

void take_big_c09(struct BigC09 value) { /* TP PERF-C-TYPE-001 */
    (void)value;
}

/* SILENT TYPE-001: a pointer parameter passes by reference. */
void take_big_ptr_c09(struct BigC09 *value) {
    (void)value;
}

struct SmallC09 {
    int x;
    int y;
};

/* SILENT TYPE-001: SmallC09 estimates below the 32-byte floor. */
void take_small_c09(struct SmallC09 value) {
    (void)value;
}
"#;
const C_TYPE001_MARK: &str = "/* TP PERF-C-TYPE-001 */";

const C_TYPE002_TP: &str = r#"/* Benchmark corpus — C struct padding candidate (PERF-C-TYPE-002). */

struct PaddedC10 { /* TP PERF-C-TYPE-002 */
    char tag;
    double value;
    int count;
};

/* SILENT TYPE-002: a single-member struct has nothing to reorder. */
struct SingleC10 {
    double value;
};
"#;
const C_TYPE002_MARK: &str = "/* TP PERF-C-TYPE-002 */";

const C_STR002_TP: &str = r#"/* Benchmark corpus — C strlen recomputed in loop (PERF-C-STR-002). */

#include <string.h>

unsigned long count_x_c06(const char *text) {
    unsigned long hits = 0;
    for (unsigned long i = 0; i < strlen(text); i++) { /* TP PERF-C-STR-002 */
        if (text[i] == 'x') {
            hits++;
        }
    }
    return hits;
}

/* SILENT STR-002: the receiver IS reassigned inside the innermost loop. */
unsigned long count_mutating_c06(char *text, char *other) {
    unsigned long hits = 0;
    for (unsigned long i = 0; i < 4; i++) {
        text = other; /* assignment to `text` suppresses STR-002 */
        hits += strlen(text); /* silent PERF-C-STR-002 */
    }
    return hits;
}
"#;
const C_STR002_MARK: &str = "/* TP PERF-C-STR-002 */";

const C_STR003_SILENT: &str = r#"/* Benchmark corpus — C bounded string op silent control (PERF-C-STR-003). */

#include <stdio.h>
#include <string.h>

void bounded_copy_c08(char *dst, const char *src, unsigned long n) {
    strncpy(dst, src, n); /* silent PERF-C-STR-003 (bounded) */
    snprintf(dst, n, "%s", src); /* silent PERF-C-STR-003 (bounded) */
}
"#;

/// The 18 non-gate C rules: 17 new files plus the two gate-file collateral
/// labels are handled in `small`.
fn c_ext() -> (Vec<GenFile>, Vec<Label>) {
    let alloc002 = C_ALLOC002_SILENT.to_string();
    let alloc003 = C_ALLOC003_TP.to_string();
    let cycle_p = C_ARCH001_CYCLE_P.to_string();
    let cycle_q = C_ARCH001_CYCLE_Q.to_string();
    let inc_leaf = C_ARCH001_LEAF.to_string();
    let inc_user = C_ARCH001_USER.to_string();
    let arch002_cold = C_ARCH002_COLD.to_string();
    let call001 = C_CALL001_TP.to_string();
    let call001_sil = C_CALL001_SILENT.to_string();
    let call002 = C_CALL002_TP.to_string();
    let call002_sil = C_CALL002_SILENT.to_string();
    let loop001 = C_LOOP001_TP.to_string();
    let loop001_sil = C_LOOP001_SILENT.to_string();
    let loop002 = C_LOOP002_TP.to_string();
    let loop003 = C_LOOP003_TP.to_string();
    let loop004 = C_LOOP004_TP.to_string();
    let loop005 = C_LOOP005_TP.to_string();
    let loop006 = C_LOOP006_TP.to_string();
    let loop007 = C_LOOP007_TP.to_string();
    let mem001 = C_MEM001_TP.to_string();
    let type001 = C_TYPE001_TP.to_string();
    let type002 = C_TYPE002_TP.to_string();
    let str002 = C_STR002_TP.to_string();
    let str003_sil = C_STR003_SILENT.to_string();

    let files = vec![
        gf("c/alloc002_silent_c02.c".to_string(), alloc002),
        gf("c/realloc_c03.c".to_string(), alloc003.clone()),
        gf("c/cycle_p_c01.h".to_string(), cycle_p),
        gf("c/cycle_q_c01.h".to_string(), cycle_q.clone()),
        gf("c/inc_leaf_c01.h".to_string(), inc_leaf),
        gf("c/inc_user_c01.c".to_string(), inc_user),
        gf("c/arch002_cold/model_cold_c02.c".to_string(), arch002_cold),
        gf("c/selfcall_c01.c".to_string(), call001.clone()),
        gf("c/mutual_c01.c".to_string(), call001_sil),
        gf("c/complex_c02.c".to_string(), call002.clone()),
        gf("c/complex_low_c02.c".to_string(), call002_sil),
        gf("c/nest_c001.c".to_string(), loop001.clone()),
        gf("c/nest_low_c001.c".to_string(), loop001_sil),
        gf("c/printf_c02.c".to_string(), loop002.clone()),
        gf("c/char_io_c03.c".to_string(), loop003.clone()),
        gf("c/divmod_c04.c".to_string(), loop004.clone()),
        gf("c/global_c05.c".to_string(), loop005.clone()),
        gf("c/indirect_c06.c".to_string(), loop006.clone()),
        gf("c/membership_c07.c".to_string(), loop007.clone()),
        gf("c/mem001_c08.c".to_string(), mem001.clone()),
        gf("c/type001_c09.c".to_string(), type001.clone()),
        gf("c/type002_c10.c".to_string(), type002.clone()),
        gf("c/strlen_c06.c".to_string(), str002.clone()),
        gf("c/bounded_c08.c".to_string(), str003_sil),
    ];

    let labels = vec![
        silent(C_ALLOC_002, "c/alloc002_silent_c02.c".to_string()),
        fire(
            C_ALLOC_003,
            "c/realloc_c03.c".to_string(),
            &alloc003,
            C_ALLOC003_MARK,
        ),
        // Same coordinate: the additive realloc IS an in-loop allocation.
        fire(
            C_ALLOC_001,
            "c/realloc_c03.c".to_string(),
            &alloc003,
            C_ALLOC003_MARK,
        ),
        // The multiplicative realloc is ALSO an in-loop allocation (ALLOC-001),
        // while staying silent for ALLOC-003.
        fire(
            C_ALLOC_001,
            "c/realloc_c03.c".to_string(),
            &alloc003,
            "/* silent PERF-C-ALLOC-003 (multiplicative) */",
        ),
        fire(
            C_ARCH_001,
            "c/cycle_q_c01.h".to_string(),
            &cycle_q,
            C_ARCH001_MARK,
        ),
        silent(C_ARCH_001, "c/inc_user_c01.c".to_string()),
        silent(C_ARCH_002, "c/arch002_cold/model_cold_c02.c".to_string()),
        fire(
            C_CALL_001,
            "c/selfcall_c01.c".to_string(),
            &call001,
            C_CALL001_MARK,
        ),
        silent(C_CALL_001, "c/mutual_c01.c".to_string()),
        fire(
            C_CALL_002,
            "c/complex_c02.c".to_string(),
            &call002,
            C_CALL002_MARK,
        ),
        silent(C_CALL_002, "c/complex_low_c02.c".to_string()),
        fire(
            C_LOOP_001,
            "c/nest_c001.c".to_string(),
            &loop001,
            C_LOOP001_MARK,
        ),
        silent(C_LOOP_001, "c/nest_low_c001.c".to_string()),
        fire(
            C_LOOP_002,
            "c/printf_c02.c".to_string(),
            &loop002,
            C_LOOP002_MARK,
        ),
        fire(
            C_LOOP_003,
            "c/char_io_c03.c".to_string(),
            &loop003,
            C_LOOP003_MARK,
        ),
        fire(
            C_LOOP_004,
            "c/divmod_c04.c".to_string(),
            &loop004,
            C_LOOP004_MARK,
        ),
        fire(
            C_LOOP_005,
            "c/global_c05.c".to_string(),
            &loop005,
            C_LOOP005_MARK,
        ),
        fire(
            C_LOOP_006,
            "c/indirect_c06.c".to_string(),
            &loop006,
            C_LOOP006_MARK,
        ),
        fire(
            C_LOOP_007,
            "c/membership_c07.c".to_string(),
            &loop007,
            C_LOOP007_MARK,
        ),
        fire(
            C_MEM_001,
            "c/mem001_c08.c".to_string(),
            &mem001,
            C_MEM001_MARK,
        ),
        fire(
            C_TYPE_001,
            "c/type001_c09.c".to_string(),
            &type001,
            C_TYPE001_MARK,
        ),
        fire(
            C_TYPE_002,
            "c/type002_c10.c".to_string(),
            &type002,
            C_TYPE002_MARK,
        ),
        fire(
            C_STR_002,
            "c/strlen_c06.c".to_string(),
            &str002,
            C_STR002_MARK,
        ),
        silent(C_STR_003, "c/bounded_c08.c".to_string()),
        // ARCH-002/CALL-002 share the hot-component anchor in complex_c02.c.
        fire(
            C_ARCH_002,
            "c/complex_c02.c".to_string(),
            &call002,
            C_CALL002_MARK,
        ),
        // M4: explicit silent control for every remaining active C rule. Each
        // named path carries the rule's near-miss construct (the multiplicative
        // realloc, the hoisted call, the literal divisor, ...), which is why the
        // rule stays silent there.
        silent(C_ALLOC_003, "c/realloc_c03.c".to_string()),
        silent(C_LOOP_002, "c/printf_c02.c".to_string()),
        silent(C_LOOP_003, "c/char_io_c03.c".to_string()),
        silent(C_LOOP_004, "c/divmod_c04.c".to_string()),
        silent(C_LOOP_005, "c/global_c05.c".to_string()),
        silent(C_LOOP_006, "c/indirect_c06.c".to_string()),
        silent(C_LOOP_007, "c/membership_c07.c".to_string()),
        silent(C_MEM_001, "c/mem001_c08.c".to_string()),
        silent(C_STR_002, "c/strlen_c06.c".to_string()),
        silent(C_TYPE_001, "c/type001_c09.c".to_string()),
        silent(C_TYPE_002, "c/type002_c10.c".to_string()),
    ];

    (files, labels)
}

// ===========================================================================
// Rust extension templates (the 18 non-gate Rust rules).
// ===========================================================================

const RS_ALLOC002: &str = r#"//! Benchmark corpus — Rust deallocation in loop (PERF-RS-ALLOC-002).

/// TP ALLOC-002: a direct `drop` call inside a loop.
pub fn drop_in_loop_r02(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        drop(i as u64); // TP PERF-RS-ALLOC-002
        total += i;
    }
    total
}

/// SILENT ALLOC-002: the `drop` is hoisted out of any loop.
pub fn drop_outside_loop_r02(value: u64) {
    drop(value); // silent PERF-RS-ALLOC-002 (outside loop)
}
"#;
const RS_ALLOC002_MARK: &str = "// TP PERF-RS-ALLOC-002";

const RS_ALLOC003: &str = r#"//! Benchmark corpus — Rust additive buffer growth (PERF-RS-ALLOC-003).

/// TP ALLOC-003: `reserve` grows the buffer additively inside a loop.
pub fn reserve_in_loop_r03(n: usize) -> Vec<u64> {
    let mut buffer: Vec<u64> = Vec::new();
    for i in 0..n {
        buffer.reserve(1); // TP PERF-RS-ALLOC-003
        buffer.push(i as u64);
    }
    buffer
}

/// SILENT ALLOC-003: the reserve is hoisted out of any loop.
pub fn reserve_outside_loop_r03(mut buffer: Vec<u64>) -> Vec<u64> {
    buffer.reserve(8); // silent PERF-RS-ALLOC-003 (outside loop)
    buffer
}
"#;
const RS_ALLOC003_MARK: &str = "// TP PERF-RS-ALLOC-003";

const RS_ARCH001_LIB: &str = r#"//! Benchmark corpus — module root for the ARCH-001 `use`-graph fixtures.

mod cycle_r04a;
mod cycle_r04b;
mod leaf_r04;
mod user_r04;
"#;

const RS_ARCH001_CYCLE_A: &str = r#"//! Benchmark corpus — Rust `use` cycle, half A (PERF-RS-ARCH-001).

use crate::cycle_r04b::cycle_r04b_root;

/// Root item referenced back by `cycle_r04b.rs`.
pub fn cycle_r04a_root(value: u64) -> u64 {
    value.wrapping_add(1)
}
"#;

const RS_ARCH001_CYCLE_B: &str = r#"//! Benchmark corpus — Rust `use` cycle, half B (PERF-RS-ARCH-001).

use crate::cycle_r04a::cycle_r04a_root; // TP PERF-RS-ARCH-001 (broken edge anchors here)

/// Root item referenced back by `cycle_r04a.rs`.
pub fn cycle_r04b_root(value: u64) -> u64 {
    value.wrapping_mul(3)
}
"#;
const RS_ARCH001_MARK: &str = "// TP PERF-RS-ARCH-001 (broken edge anchors here)";

const RS_ARCH001_LEAF: &str = r#"//! Benchmark corpus — ARCH-001 resolved non-cyclic import target.

pub fn leaf_r04_fn(value: u64) -> u64 {
    value ^ 0x1
}
"#;

const RS_ARCH001_USER: &str = r#"//! Benchmark corpus — ARCH-001 non-cyclic resolved import (silent).

use crate::leaf_r04::leaf_r04_fn;

pub fn user_r04_read(value: u64) -> u64 {
    let _ = leaf_r04_fn;
    value
}
"#;

const RS_ARCH002_COLD: &str = r#"//! Benchmark corpus — ARCH-002 cold component silent control (sum < 15).

pub fn cold_r02(value: u64) -> u64 {
    if value == 0 {
        0
    } else {
        value
    }
}
"#;

const RS_CALL001: &str = r#"//! Benchmark corpus — Rust direct recursion (PERF-RS-CALL-001) and its
//! mutual-recursion silent control.

pub fn recurse_r05(n: u64) -> u64 {
    if n == 0 {
        0
    } else {
        n.wrapping_add(recurse_r05(n - 1)) // TP PERF-RS-CALL-001
    }
}

pub fn even_r05(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        odd_r05(n - 1) // silent PERF-RS-CALL-001 (calls a different fn)
    }
}

pub fn odd_r05(n: u64) -> u64 {
    if n == 0 {
        0
    } else {
        even_r05(n - 1) // silent PERF-RS-CALL-001
    }
}
"#;
const RS_CALL001_MARK: &str = "// TP PERF-RS-CALL-001";

const RS_CALL002: &str = r#"//! Benchmark corpus — Rust high cyclomatic complexity (PERF-RS-CALL-002);
//! the same function is the PERF-RS-ARCH-002 hot-component anchor.

pub fn classify_r06(code: u32) -> &'static str { // TP PERF-RS-CALL-002 + TP PERF-RS-ARCH-002
    match code {
        0 => "v0",
        1 => "v1",
        2 => "v2",
        3 => "v3",
        4 => "v4",
        5 => "v5",
        6 => "v6",
        7 => "v7",
        8 => "v8",
        9 => "v9",
        10 => "v10",
        11 => "v11",
        12 => "v12",
        13 => "v13",
        14 => "v14",
        _ => "other",
    }
}
"#;
const RS_CALL002_MARK: &str = "// TP PERF-RS-CALL-002 + TP PERF-RS-ARCH-002";

const RS_CALL002_SILENT: &str = r#"//! Benchmark corpus — RS-CALL-002 silent control (complexity below 15).

pub fn low_branch_r06(code: u32) -> &'static str {
    match code {
        0 => "v0",
        1 => "v1",
        2 => "v2",
        3 => "v3",
        4 => "v4",
        5 => "v5",
        6 => "v6",
        7 => "v7",
        8 => "v8",
        _ => "other",
    }
}
"#;

const RS_LOOP001: &str = r#"//! Benchmark corpus — Rust loop nesting depth 3 (PERF-RS-LOOP-001) and its
//! depth-2 silent control.

pub fn nest_three_r07(n: usize) -> usize {
    let mut acc = 0usize;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n { // TP PERF-RS-LOOP-001
                acc += i + j + k;
            }
        }
    }
    acc
}

pub fn nest_two_r07(n: usize) -> usize {
    let mut acc = 0usize;
    for i in 0..n {
        for j in 0..n {
            acc += i + j;
        }
    }
    acc
}
"#;
const RS_LOOP001_MARK: &str = "// TP PERF-RS-LOOP-001";

const RS_LOOP002: &str = r#"//! Benchmark corpus — Rust expensive I/O in loop (PERF-RS-LOOP-002).

pub fn emit_loop_r08(n: usize) {
    for i in 0..n {
        println!("{i}"); // TP PERF-RS-LOOP-002
    }
}

/// SILENT LOOP-002: the same call hoisted out of any loop.
pub fn emit_once_r08(x: usize) {
    println!("{x}"); // silent PERF-RS-LOOP-002 (outside loop)
}
"#;
const RS_LOOP002_MARK: &str = "// TP PERF-RS-LOOP-002";

const RS_LOOP003: &str = r#"//! Benchmark corpus — Rust char-wise processing in loop (PERF-RS-LOOP-003).

pub fn charwise_loop_r09(text: &str, n: usize) -> usize {
    let mut total = 0usize;
    for _ in 0..n {
        text.chars(); // TP PERF-RS-LOOP-003
        total += 1;
    }
    total
}

/// SILENT LOOP-003: `chars()` nested inside another call is not primary.
pub fn charwise_nested_r09(text: &str, out: &mut Vec<char>) {
    for _ in 0..1 {
        out.extend(text.chars()); // silent PERF-RS-LOOP-003 (nested)
    }
}
"#;
const RS_LOOP003_MARK: &str = "// TP PERF-RS-LOOP-003";

const RS_LOOP004: &str = r#"//! Benchmark corpus — Rust variable-divisor div/mod (PERF-RS-LOOP-004).

pub fn mod_loop_r10(n: u64, divisor: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..n {
        acc = acc.wrapping_add(i % divisor); // TP PERF-RS-LOOP-004
    }
    acc
}

/// SILENT LOOP-004: a literal divisor is strength-reduced by the compiler.
pub fn mod_literal_r10(n: u64) -> u64 {
    let mut acc = 0u64;
    for i in 0..n {
        acc = acc.wrapping_add(i % 5); // silent PERF-RS-LOOP-004 (literal divisor)
    }
    acc
}
"#;
const RS_LOOP004_MARK: &str = "// TP PERF-RS-LOOP-004";

const RS_LOOP005: &str = r#"//! Benchmark corpus — Rust global write in loop (PERF-RS-LOOP-005).

pub static mut GLOBAL_R11: u64 = 0;

pub fn write_global_r11(n: usize) {
    let mut i = 0usize;
    while i < n {
        unsafe {
            GLOBAL_R11 = GLOBAL_R11.wrapping_add(1); // TP PERF-RS-LOOP-005
        }
        i += 1;
    }
}

/// SILENT LOOP-005: the store target is a local.
pub fn write_local_r11(n: usize) -> usize {
    let mut total = 0usize;
    for i in 0..n {
        total += i; // silent PERF-RS-LOOP-005 (local)
    }
    total
}
"#;
const RS_LOOP005_MARK: &str = "// TP PERF-RS-LOOP-005";

const RS_LOOP006: &str = r#"//! Benchmark corpus — Rust indirect call in loop (PERF-RS-LOOP-006).

pub fn dispatch_loop_r12(handlers: &[fn()], n: usize) {
    for i in 0..n {
        handlers[i](); // TP PERF-RS-LOOP-006
    }
}

pub fn direct_target_r12() {}

/// SILENT LOOP-006: a direct call in the loop never fires.
pub fn call_direct_r12(n: usize) {
    for _ in 0..n {
        direct_target_r12(); // silent PERF-RS-LOOP-006 (direct call)
    }
}
"#;
const RS_LOOP006_MARK: &str = "// TP PERF-RS-LOOP-006";

const RS_LOOP007: &str = r#"//! Benchmark corpus — Rust linear membership scan (PERF-RS-LOOP-007).

pub fn contains_nested_r13(table: &[u64], tlen: usize, keys: &[u64], klen: usize) -> bool {
    for i in 0..klen {
        for j in 0..tlen {
            if table[j] == keys[i] { // TP PERF-RS-LOOP-007
                return true;
            }
        }
    }
    false
}

/// SILENT LOOP-007: depth 1 is below the floor.
pub fn contains_shallow_r13(table: &[u64], tlen: usize, key: u64) -> bool {
    for j in 0..tlen {
        if table[j] == key { // silent PERF-RS-LOOP-007 (depth 1)
            return true;
        }
    }
    false
}
"#;
const RS_LOOP007_MARK: &str = "// TP PERF-RS-LOOP-007";

const RS_MEM001: &str = r#"//! Benchmark corpus — Rust size_of-as-element-count misuse (PERF-RS-MEM-001).

/// TP MEM-001: a byte count where an element count is expected.
pub fn over_reserve_r14() -> Vec<u64> {
    Vec::with_capacity(std::mem::size_of::<u64>()) // TP PERF-RS-MEM-001
}

/// SILENT MEM-001: a plain element count.
pub fn capacity_count_r14(count: usize) -> Vec<u64> {
    Vec::with_capacity(count) // silent PERF-RS-MEM-001 (element count)
}
"#;
const RS_MEM001_MARK: &str = "// TP PERF-RS-MEM-001";

const RS_STR002: &str = r#"//! Benchmark corpus — Rust length query recomputed in loop (PERF-RS-STR-002).

pub fn length_loop_r15(words: &[u64], n: usize) -> usize {
    let mut total = 0usize;
    for _ in 0..n {
        total += words.len(); // TP PERF-RS-STR-002
    }
    total
}

/// SILENT STR-002: the receiver IS reassigned inside the innermost loop.
pub fn length_reassigned_r15(mut s: &[u64], other: &[u64], n: usize) -> usize {
    let mut total = 0usize;
    for _ in 0..n {
        s = other; // assignment to `s` suppresses STR-002
        total += s.len(); // silent PERF-RS-STR-002
    }
    total
}
"#;
const RS_STR002_MARK: &str = "// TP PERF-RS-STR-002";

const RS_STR003: &str = r#"//! Benchmark corpus — Rust unbounded unsafe memory op (PERF-RS-STR-003).

/// TP STR-003: an unchecked `set_len` is the Rust analogue of an unbounded op.
pub fn unbounded_set_len_r16(mut v: Vec<u8>) -> Vec<u8> {
    v.set_len(0); // TP PERF-RS-STR-003
    v
}

/// SILENT STR-003: the bounded block-copy variant.
pub fn bounded_copy_r16(src: &[u8], dst: &mut [u8]) {
    dst.copy_from_slice(src); // silent PERF-RS-STR-003 (bounded)
}
"#;
const RS_STR003_MARK: &str = "// TP PERF-RS-STR-003";

const RS_TYPE001: &str = r#"//! Benchmark corpus — Rust large struct by value (PERF-RS-TYPE-001).

/// Four 8-byte words = 32 bytes under the LP64 model (no padding).
pub struct BigR17 {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: u64,
}

pub fn take_big_r17(value: BigR17) -> u64 { // TP PERF-RS-TYPE-001
    value.a ^ value.b ^ value.c ^ value.d
}

/// SILENT TYPE-001: a reference parameter passes by reference.
pub fn take_big_ref_r17(value: &BigR17) -> u64 {
    value.a
}

pub struct SmallR17 {
    pub x: u32,
    pub y: u32,
}

/// SILENT TYPE-001: SmallR17 estimates below the 32-byte floor.
pub fn take_small_r17(value: SmallR17) -> u32 {
    value.x + value.y
}
"#;
const RS_TYPE001_MARK: &str = "// TP PERF-RS-TYPE-001";

const RS_TYPE002: &str = r#"//! Benchmark corpus — Rust struct padding candidate (PERF-RS-TYPE-002).

pub struct PaddedR18 { // TP PERF-RS-TYPE-002
    pub tag: u8,
    pub wide_a: u64,
    pub flag: u8,
    pub wide_b: u64,
}

/// SILENT TYPE-002: a single-member struct has nothing to reorder.
pub struct SingleR18 {
    pub only: u64,
}

/// SILENT TYPE-002: padding-free declared order.
pub struct TightR18 {
    pub x: u64,
    pub y: u64,
}
"#;
const RS_TYPE002_MARK: &str = "// TP PERF-RS-TYPE-002";

fn rust_ext() -> (Vec<GenFile>, Vec<Label>) {
    let alloc002 = RS_ALLOC002.to_string();
    let alloc003 = RS_ALLOC003.to_string();
    let lib = RS_ARCH001_LIB.to_string();
    let cycle_a = RS_ARCH001_CYCLE_A.to_string();
    let cycle_b = RS_ARCH001_CYCLE_B.to_string();
    let leaf = RS_ARCH001_LEAF.to_string();
    let user = RS_ARCH001_USER.to_string();
    let arch002_cold = RS_ARCH002_COLD.to_string();
    let call001 = RS_CALL001.to_string();
    let call002 = RS_CALL002.to_string();
    let call002_sil = RS_CALL002_SILENT.to_string();
    let loop001 = RS_LOOP001.to_string();
    let loop002 = RS_LOOP002.to_string();
    let loop003 = RS_LOOP003.to_string();
    let loop004 = RS_LOOP004.to_string();
    let loop005 = RS_LOOP005.to_string();
    let loop006 = RS_LOOP006.to_string();
    let loop007 = RS_LOOP007.to_string();
    let mem001 = RS_MEM001.to_string();
    let str002 = RS_STR002.to_string();
    let str003 = RS_STR003.to_string();
    let type001 = RS_TYPE001.to_string();
    let type002 = RS_TYPE002.to_string();

    let files = vec![
        gf("rust/alloc002_c02.rs".to_string(), alloc002.clone()),
        gf("rust/alloc003_c03.rs".to_string(), alloc003.clone()),
        gf("rust/lib.rs".to_string(), lib),
        gf("rust/cycle_r04a.rs".to_string(), cycle_a),
        gf("rust/cycle_r04b.rs".to_string(), cycle_b.clone()),
        gf("rust/leaf_r04.rs".to_string(), leaf),
        gf("rust/user_r04.rs".to_string(), user),
        gf("rust_cold/model_cold_r02.rs".to_string(), arch002_cold),
        gf("rust/selfcall_r05.rs".to_string(), call001.clone()),
        gf("rust/complex_r06.rs".to_string(), call002.clone()),
        gf("rust/complex_low_r06.rs".to_string(), call002_sil),
        gf("rust/nest_r07.rs".to_string(), loop001.clone()),
        gf("rust/io_r08.rs".to_string(), loop002.clone()),
        gf("rust/charwise_r09.rs".to_string(), loop003.clone()),
        gf("rust/divmod_r10.rs".to_string(), loop004.clone()),
        gf("rust/global_r11.rs".to_string(), loop005.clone()),
        gf("rust/indirect_r12.rs".to_string(), loop006.clone()),
        gf("rust/membership_r13.rs".to_string(), loop007.clone()),
        gf("rust/mem001_r14.rs".to_string(), mem001.clone()),
        gf("rust/str002_r15.rs".to_string(), str002.clone()),
        gf("rust/str003_r16.rs".to_string(), str003.clone()),
        gf("rust/type001_r17.rs".to_string(), type001.clone()),
        gf("rust/type002_r18.rs".to_string(), type002.clone()),
    ];

    let labels = vec![
        fire(
            RS_ALLOC_002,
            "rust/alloc002_c02.rs".to_string(),
            &alloc002,
            RS_ALLOC002_MARK,
        ),
        fire(
            RS_ALLOC_003,
            "rust/alloc003_c03.rs".to_string(),
            &alloc003,
            RS_ALLOC003_MARK,
        ),
        fire(
            RS_ARCH_001,
            "rust/cycle_r04b.rs".to_string(),
            &cycle_b,
            RS_ARCH001_MARK,
        ),
        silent(RS_ARCH_001, "rust/user_r04.rs".to_string()),
        silent(RS_ARCH_002, "rust_cold/model_cold_r02.rs".to_string()),
        fire(
            RS_CALL_001,
            "rust/selfcall_r05.rs".to_string(),
            &call001,
            RS_CALL001_MARK,
        ),
        fire(
            RS_CALL_002,
            "rust/complex_r06.rs".to_string(),
            &call002,
            RS_CALL002_MARK,
        ),
        silent(RS_CALL_002, "rust/complex_low_r06.rs".to_string()),
        fire(
            RS_LOOP_001,
            "rust/nest_r07.rs".to_string(),
            &loop001,
            RS_LOOP001_MARK,
        ),
        fire(
            RS_LOOP_002,
            "rust/io_r08.rs".to_string(),
            &loop002,
            RS_LOOP002_MARK,
        ),
        fire(
            RS_LOOP_003,
            "rust/charwise_r09.rs".to_string(),
            &loop003,
            RS_LOOP003_MARK,
        ),
        fire(
            RS_LOOP_004,
            "rust/divmod_r10.rs".to_string(),
            &loop004,
            RS_LOOP004_MARK,
        ),
        fire(
            RS_LOOP_005,
            "rust/global_r11.rs".to_string(),
            &loop005,
            RS_LOOP005_MARK,
        ),
        fire(
            RS_LOOP_006,
            "rust/indirect_r12.rs".to_string(),
            &loop006,
            RS_LOOP006_MARK,
        ),
        fire(
            RS_LOOP_007,
            "rust/membership_r13.rs".to_string(),
            &loop007,
            RS_LOOP007_MARK,
        ),
        fire(
            RS_MEM_001,
            "rust/mem001_r14.rs".to_string(),
            &mem001,
            RS_MEM001_MARK,
        ),
        fire(
            RS_STR_002,
            "rust/str002_r15.rs".to_string(),
            &str002,
            RS_STR002_MARK,
        ),
        fire(
            RS_STR_003,
            "rust/str003_r16.rs".to_string(),
            &str003,
            RS_STR003_MARK,
        ),
        fire(
            RS_TYPE_001,
            "rust/type001_r17.rs".to_string(),
            &type001,
            RS_TYPE001_MARK,
        ),
        fire(
            RS_TYPE_002,
            "rust/type002_r18.rs".to_string(),
            &type002,
            RS_TYPE002_MARK,
        ),
        // ARCH-002/CALL-002 share the hot-component anchor in complex_r06.rs.
        fire(
            RS_ARCH_002,
            "rust/complex_r06.rs".to_string(),
            &call002,
            RS_CALL002_MARK,
        ),
        // M4: explicit silent control for every remaining active Rust rule.
        silent(RS_ALLOC_002, "rust/alloc002_c02.rs".to_string()),
        silent(RS_ALLOC_003, "rust/alloc003_c03.rs".to_string()),
        silent(RS_CALL_001, "rust/selfcall_r05.rs".to_string()),
        silent(RS_LOOP_001, "rust/nest_r07.rs".to_string()),
        silent(RS_LOOP_002, "rust/io_r08.rs".to_string()),
        silent(RS_LOOP_003, "rust/charwise_r09.rs".to_string()),
        silent(RS_LOOP_004, "rust/divmod_r10.rs".to_string()),
        silent(RS_LOOP_005, "rust/global_r11.rs".to_string()),
        silent(RS_LOOP_006, "rust/indirect_r12.rs".to_string()),
        silent(RS_LOOP_007, "rust/membership_r13.rs".to_string()),
        silent(RS_MEM_001, "rust/mem001_r14.rs".to_string()),
        silent(RS_STR_002, "rust/str002_r15.rs".to_string()),
        silent(RS_STR_003, "rust/str003_r16.rs".to_string()),
        silent(RS_TYPE_001, "rust/type001_r17.rs".to_string()),
        silent(RS_TYPE_002, "rust/type002_r18.rs".to_string()),
    ];

    (files, labels)
}

// ===========================================================================
// Python extension templates (plan `python-backend` W4). The six KPI rules'
// fixtures are lifted VERBATIM from the committed `tests/fixtures/precision/
// python/` corpus (so `assert_small_anchors_match_m1` holds); the rest are
// dedicated minimal files under `py/`.
// ===========================================================================

const PY_KPI_ALLOC: &str = r#"# Precision corpus — Python alloc family (PERF-PY-ALLOC-001): planted true
# positive. Silent controls live in `alloc_silent.py`.


def build(n):
    for _ in range(n):
        buf = dict()  # TP PERF-PY-ALLOC-001
        del buf
"#;
const PY_KPI_ALLOC_SILENT: &str = r#"# Precision corpus — Python alloc-family silent control (must stay silent).


def hoisted():
    return dict()  # silent PERF-PY-ALLOC-001 (constructor outside every loop)
"#;
const PY_KPI_ALLOC3: &str = r#"# Precision corpus — Python accumulation (PERF-PY-ALLOC-003): planted true
# positive. Silent controls live in `alloc3_silent.py`.


def grow(items):
    acc = []
    for item in items:
        acc = acc + [item]  # TP PERF-PY-ALLOC-003
    return acc
"#;
const PY_KPI_ALLOC3_SILENT: &str = r#"# Precision corpus — Python accumulation silent control (must stay silent).


def rebuild(rows):
    count = 0
    for row in rows:
        tmp = []
        tmp.append(row)  # silent PERF-PY-ALLOC-003 (receiver rebuilt in loop)
        count += len(tmp)
    return count
"#;
const PY_KPI_STR: &str = r#"# Precision corpus — Python string building (PERF-PY-STR-001): planted true
# positive. Silent controls live in `str_silent.py`.


def build(pieces):
    s = ""
    for piece in pieces:
        s += piece  # TP PERF-PY-STR-001
    return s
"#;
const PY_KPI_STR_SILENT: &str = r#"# Precision corpus — Python string silent control (must stay silent).


def prefix_read(pieces):
    prefix = "["
    last = ""
    for piece in pieces:
        last = prefix + piece  # silent PERF-PY-STR-001 (not accumulated)
    return last
"#;
const PY_KPI_LOOP: &str = r#"# Precision corpus — Python list scan (PERF-PY-LOOP-007): planted true
# positive. Silent controls live in `loop_silent.py`.


def contains(rows):
    table = [1, 2, 3]
    hits = 0
    for r in rows:
        if r in table:  # TP PERF-PY-LOOP-007
            hits += 1
    return hits
"#;
const PY_KPI_LOOP_SILENT: &str = r#"# Precision corpus — Python list-scan silent control (must stay silent).


def in_set(rows):
    seen = {1, 2, 3}
    for r in rows:
        if r in seen:  # silent PERF-PY-LOOP-007 (set/dict container excluded)
            return True
    return False
"#;
const PY_KPI_ARCH: &str = r#"# Precision corpus — Python cross-module call coupling (PERF-PY-ARCH-003):
# planted true positive. Silent controls live in `arch_silent.py`.


def use(raw):
    return py_unique_helper(raw, 0, 100)  # TP PERF-PY-ARCH-003
"#;
const PY_KPI_ARCH_HELPERS: &str = r#"# Precision corpus — Python cross-file callee definitions.


def py_unique_helper(value, lo, hi):
    """Uniquely defined corpus callee (the ARCH-003 true-positive target)."""
    if value < lo:
        return lo
    if value > hi:
        return hi
    return value
"#;
const PY_KPI_ARCH_SILENT: &str = r#"# Precision corpus — Python cross-module coupling silent control.


def wrapper(row):
    return same_file_only(row)  # silent PERF-PY-ARCH-003 (same-file def)


def same_file_only(row):
    return row
"#;
const PY_KPI_MEM: &str = r#"# Precision corpus — Python container copy (PERF-PY-MEM-002): planted true
# positive. Silent controls live in `mem_silent.py`.

import copy


def clone(rows):
    last = None
    for row in rows:
        last = copy.deepcopy(row)  # TP PERF-PY-MEM-002
    return last
"#;
const PY_KPI_MEM_SILENT: &str = r#"# Precision corpus — Python container-copy silent control (must stay silent).

import copy


def clone_once(rows):
    return copy.deepcopy(rows)  # silent PERF-PY-MEM-002 (outside any loop)
"#;

const PY_ALLOC004: &str = r#"def materialise(rows):
    total = 0
    for row in rows:
        total += len(list(row))  # TP PERF-PY-ALLOC-004
    return total
"#;
const PY_ALLOC004_MARK: &str = "TP PERF-PY-ALLOC-004";
const PY_ALLOC004_SILENT: &str = r#"def hoisted(rows):
    return list(rows[0])
"#;

const PY_CYCLE_A: &str = r#"from .cycle_b import ping_b


def walk(n):
    return n
"#;
const PY_CYCLE_B: &str = r#"from .cycle_a import walk  # TP PERF-PY-ARCH-001


def ping_b(n):
    return n
"#;
const PY_CYCLE_MARK: &str = "TP PERF-PY-ARCH-001";
const PY_CYCLE_LEAF: &str = r#"def leaf():
    return 1
"#;

const PY_ASYNC001: &str = r#"import requests


async def fetch(url):
    return requests.get(url)  # TP PERF-PY-ASYNC-001
"#;
const PY_ASYNC001_MARK: &str = "TP PERF-PY-ASYNC-001";
const PY_ASYNC001_SILENT: &str = r#"def fetch_sync(url):
    return url
"#;

const PY_ASYNC002: &str = r#"async def fetch(url):
    return url


async def await_each(urls):
    last = None
    for url in urls:
        last = await fetch(url)  # TP PERF-PY-ASYNC-002
    return last
"#;
const PY_ASYNC002_MARK: &str = "TP PERF-PY-ASYNC-002";
const PY_ASYNC002_SILENT: &str = r#"async def fetch(url):
    return url


async def one(url):
    return await fetch(url)
"#;

const PY_CALL001: &str = r#"def walk(n):
    if n <= 0:
        return 0
    return walk(n - 1)  # TP PERF-PY-CALL-001
"#;
const PY_CALL001_MARK: &str = "TP PERF-PY-CALL-001";
const PY_CALL001_SILENT: &str = r#"def step(n):
    return n - 1
"#;

const PY_COMPLEX: &str = r#"def complex_py(value):  # TP PERF-PY-CALL-002
    score = 0
    if value > 1:
        score += 1
    if value > 2:
        score += 1
    if value > 3:
        score += 1
    if value > 4:
        score += 1
    if value > 5:
        score += 1
    if value > 6:
        score += 1
    if value > 7:
        score += 1
    if value > 8:
        score += 1
    if value > 9:
        score += 1
    if value > 10:
        score += 1
    if value > 11:
        score += 1
    if value > 12:
        score += 1
    if value > 13:
        score += 1
    if value > 14:
        score += 1
    if value > 15:
        score += 1
    if value > 16:
        score += 1
    return score
"#;
const PY_COMPLEX_MARK: &str = "TP PERF-PY-CALL-002";
const PY_COMPLEX_SILENT: &str = r#"def tiny(value):
    return value + 1
"#;

const PY_COMPREHENSION: &str = r#"def transform(row):
    return row * 2


def collect(rows):
    out = []
    for row in rows:
        out.append(transform(row))  # TP PERF-PY-COMPREHENSION-001
    return out
"#;
const PY_COMPREHENSION_MARK: &str = "TP PERF-PY-COMPREHENSION-001";
const PY_COMPREHENSION_SILENT: &str = r#"def plain(rows):
    total = 0
    for row in rows:
        total += row
    return total
"#;

const PY_EXECUTOR: &str = r#"from concurrent.futures import ThreadPoolExecutor


def fan_out(tasks):
    for task in tasks:
        with ThreadPoolExecutor() as pool:  # TP PERF-PY-EXECUTOR-001
            pool.submit(task)
"#;
const PY_EXECUTOR_MARK: &str = "TP PERF-PY-EXECUTOR-001";
const PY_EXECUTOR_SILENT: &str = r#"from concurrent.futures import ThreadPoolExecutor


def once(tasks):
    with ThreadPoolExecutor() as pool:
        return pool
"#;

const PY_LOOP001: &str = r#"def triple(a, b, c):
    total = 0
    for x in a:
        for y in b:
            for z in c:  # TP PERF-PY-LOOP-001
                total += z
    return total
"#;
const PY_LOOP001_MARK: &str = "TP PERF-PY-LOOP-001";
const PY_LOOP001_SILENT: &str = r#"def double(a, b):
    total = 0
    for x in a:
        for y in b:
            total += x + y
    return total
"#;

const PY_LOOP002: &str = r#"def read_each(paths):
    total = 0
    for p in paths:
        fh = open(p)  # TP PERF-PY-LOOP-002
        total += 1
        del fh
    return total
"#;
const PY_LOOP002_MARK: &str = "TP PERF-PY-LOOP-002";
const PY_LOOP002_SILENT: &str = r#"def log_each(rows):
    for row in rows:
        print(row)
"#;

const PY_LOOP003: &str = r#"def positional(rows, seq):
    total = 0
    for _ in rows:
        for i in range(len(seq)):  # TP PERF-PY-LOOP-003
            total += i
    return total
"#;
const PY_LOOP003_MARK: &str = "TP PERF-PY-LOOP-003";
const PY_LOOP003_SILENT: &str = r#"def count(n):
    total = 0
    for i in range(n):
        total += i
    return total
"#;

const PY_LOOP009: &str = r#"def swallow(rows):
    ok = 0
    for row in rows:
        try:  # TP PERF-PY-LOOP-009
            ok += int(row)
        except ValueError:
            pass
    return ok
"#;
const PY_LOOP009_MARK: &str = "TP PERF-PY-LOOP-009";
const PY_LOOP009_SILENT: &str = r#"def guarded(rows):
    try:
        return rows[0]
    except IndexError:
        return None
"#;

const PY_N1: &str = r#"def fetch_each(conn, ids):
    last = None
    for i in ids:
        last = conn.execute(i)  # TP PERF-PY-N1-001
    return last
"#;
const PY_N1_MARK: &str = "TP PERF-PY-N1-001";
const PY_N1_SILENT: &str = r#"def fetch_once(conn):
    return conn.execute("select 1")
"#;

const PY_PANDAS001: &str = r#"import pandas as pd


def row_wise(df):
    total = 0
    for _, row in df.iterrows():  # TP PERF-PY-PANDAS-001
        total += row["x"]
    return total
"#;
const PY_PANDAS001_MARK: &str = "TP PERF-PY-PANDAS-001";
const PY_PANDAS001_SILENT: &str = r#"def vectorised(df):
    return df["x"].mean()
"#;

const PY_PANDAS002: &str = r#"import pandas as pd


def append_frames(frames):
    out = None
    for frame in frames:
        out = pd.concat([out, frame])  # TP PERF-PY-PANDAS-002
    return out
"#;
const PY_PANDAS002_MARK: &str = "TP PERF-PY-PANDAS-002";
const PY_PANDAS002_SILENT: &str = r#"import pandas as pd


def combine(frames):
    return pd.concat(frames)
"#;

/// The 22 Python rules: the six lifted KPI fixtures (`python/`) plus sixteen
/// dedicated `py/` templates. Mirrors `rust_ext()`.
fn py_ext() -> (Vec<GenFile>, Vec<Label>) {
    let files = vec![
        gf("python/alloc.py".to_string(), PY_KPI_ALLOC.to_string()),
        gf(
            "python/alloc_silent.py".to_string(),
            PY_KPI_ALLOC_SILENT.to_string(),
        ),
        gf("python/alloc3.py".to_string(), PY_KPI_ALLOC3.to_string()),
        gf(
            "python/alloc3_silent.py".to_string(),
            PY_KPI_ALLOC3_SILENT.to_string(),
        ),
        gf("python/str.py".to_string(), PY_KPI_STR.to_string()),
        gf(
            "python/str_silent.py".to_string(),
            PY_KPI_STR_SILENT.to_string(),
        ),
        gf("python/loop.py".to_string(), PY_KPI_LOOP.to_string()),
        gf(
            "python/loop_silent.py".to_string(),
            PY_KPI_LOOP_SILENT.to_string(),
        ),
        gf("python/arch.py".to_string(), PY_KPI_ARCH.to_string()),
        gf(
            "python/arch_helpers.py".to_string(),
            PY_KPI_ARCH_HELPERS.to_string(),
        ),
        gf(
            "python/arch_silent.py".to_string(),
            PY_KPI_ARCH_SILENT.to_string(),
        ),
        gf("python/mem.py".to_string(), PY_KPI_MEM.to_string()),
        gf(
            "python/mem_silent.py".to_string(),
            PY_KPI_MEM_SILENT.to_string(),
        ),
        gf("py/alloc4.py".to_string(), PY_ALLOC004.to_string()),
        gf(
            "py/alloc4_silent.py".to_string(),
            PY_ALLOC004_SILENT.to_string(),
        ),
        gf("py/cycle_a.py".to_string(), PY_CYCLE_A.to_string()),
        gf("py/cycle_b.py".to_string(), PY_CYCLE_B.to_string()),
        gf("py/cycle_leaf.py".to_string(), PY_CYCLE_LEAF.to_string()),
        gf("py/asyncblock.py".to_string(), PY_ASYNC001.to_string()),
        gf(
            "py/asyncsilent.py".to_string(),
            PY_ASYNC001_SILENT.to_string(),
        ),
        gf("py/awaitloop.py".to_string(), PY_ASYNC002.to_string()),
        gf(
            "py/awaitsilent.py".to_string(),
            PY_ASYNC002_SILENT.to_string(),
        ),
        gf("py/selfcall.py".to_string(), PY_CALL001.to_string()),
        gf(
            "py/selfcall_silent.py".to_string(),
            PY_CALL001_SILENT.to_string(),
        ),
        gf("py/complex_py.py".to_string(), PY_COMPLEX.to_string()),
        gf(
            "py/complex_low.py".to_string(),
            PY_COMPLEX_SILENT.to_string(),
        ),
        gf(
            "py/comprehension.py".to_string(),
            PY_COMPREHENSION.to_string(),
        ),
        gf(
            "py/comprehension_silent.py".to_string(),
            PY_COMPREHENSION_SILENT.to_string(),
        ),
        gf("py/executor.py".to_string(), PY_EXECUTOR.to_string()),
        gf(
            "py/executor_silent.py".to_string(),
            PY_EXECUTOR_SILENT.to_string(),
        ),
        gf("py/nest.py".to_string(), PY_LOOP001.to_string()),
        gf(
            "py/nest_silent.py".to_string(),
            PY_LOOP001_SILENT.to_string(),
        ),
        gf("py/io.py".to_string(), PY_LOOP002.to_string()),
        gf("py/io_silent.py".to_string(), PY_LOOP002_SILENT.to_string()),
        gf("py/range.py".to_string(), PY_LOOP003.to_string()),
        gf(
            "py/range_silent.py".to_string(),
            PY_LOOP003_SILENT.to_string(),
        ),
        gf("py/tryloop.py".to_string(), PY_LOOP009.to_string()),
        gf(
            "py/tryloop_silent.py".to_string(),
            PY_LOOP009_SILENT.to_string(),
        ),
        gf("py/n1.py".to_string(), PY_N1.to_string()),
        gf("py/n1_silent.py".to_string(), PY_N1_SILENT.to_string()),
        gf("py/pandas1.py".to_string(), PY_PANDAS001.to_string()),
        gf(
            "py/pandas1_silent.py".to_string(),
            PY_PANDAS001_SILENT.to_string(),
        ),
        gf("py/pandas2.py".to_string(), PY_PANDAS002.to_string()),
        gf(
            "py/pandas2_silent.py".to_string(),
            PY_PANDAS002_SILENT.to_string(),
        ),
    ];

    let labels = vec![
        fire(
            PY_ALLOC_001,
            "python/alloc.py".to_string(),
            PY_KPI_ALLOC,
            "TP PERF-PY-ALLOC-001",
        ),
        silent(PY_ALLOC_001, "python/alloc_silent.py".to_string()),
        fire(
            PY_ALLOC_003,
            "python/alloc3.py".to_string(),
            PY_KPI_ALLOC3,
            "TP PERF-PY-ALLOC-003",
        ),
        silent(PY_ALLOC_003, "python/alloc3_silent.py".to_string()),
        fire(
            PY_ALLOC_004,
            "py/alloc4.py".to_string(),
            PY_ALLOC004,
            PY_ALLOC004_MARK,
        ),
        silent(PY_ALLOC_004, "py/alloc4_silent.py".to_string()),
        fire(
            PY_ARCH_001,
            "py/cycle_b.py".to_string(),
            PY_CYCLE_B,
            PY_CYCLE_MARK,
        ),
        silent(PY_ARCH_001, "py/cycle_leaf.py".to_string()),
        fire(
            PY_ARCH_002,
            "py/complex_py.py".to_string(),
            PY_COMPLEX,
            PY_COMPLEX_MARK,
        ),
        silent(PY_ARCH_002, "py/cycle_leaf.py".to_string()),
        fire(
            PY_ARCH_003,
            "python/arch.py".to_string(),
            PY_KPI_ARCH,
            "TP PERF-PY-ARCH-003",
        ),
        silent(PY_ARCH_003, "python/arch_silent.py".to_string()),
        fire(
            PY_ASYNC_001,
            "py/asyncblock.py".to_string(),
            PY_ASYNC001,
            PY_ASYNC001_MARK,
        ),
        silent(PY_ASYNC_001, "py/asyncsilent.py".to_string()),
        fire(
            PY_ASYNC_002,
            "py/awaitloop.py".to_string(),
            PY_ASYNC002,
            PY_ASYNC002_MARK,
        ),
        silent(PY_ASYNC_002, "py/awaitsilent.py".to_string()),
        fire(
            PY_CALL_001,
            "py/selfcall.py".to_string(),
            PY_CALL001,
            PY_CALL001_MARK,
        ),
        silent(PY_CALL_001, "py/selfcall_silent.py".to_string()),
        fire(
            PY_CALL_002,
            "py/complex_py.py".to_string(),
            PY_COMPLEX,
            PY_COMPLEX_MARK,
        ),
        silent(PY_CALL_002, "py/complex_low.py".to_string()),
        fire(
            PY_COMPREHENSION_001,
            "py/comprehension.py".to_string(),
            PY_COMPREHENSION,
            PY_COMPREHENSION_MARK,
        ),
        // Collateral: the single-append body also accumulates into `out`, so
        // ALLOC-003 fires at the same coordinate.
        fire(
            PY_ALLOC_003,
            "py/comprehension.py".to_string(),
            PY_COMPREHENSION,
            PY_COMPREHENSION_MARK,
        ),
        silent(
            PY_COMPREHENSION_001,
            "py/comprehension_silent.py".to_string(),
        ),
        fire(
            PY_EXECUTOR_001,
            "py/executor.py".to_string(),
            PY_EXECUTOR,
            PY_EXECUTOR_MARK,
        ),
        silent(PY_EXECUTOR_001, "py/executor_silent.py".to_string()),
        fire(
            PY_LOOP_001,
            "py/nest.py".to_string(),
            PY_LOOP001,
            PY_LOOP001_MARK,
        ),
        silent(PY_LOOP_001, "py/nest_silent.py".to_string()),
        fire(
            PY_LOOP_002,
            "py/io.py".to_string(),
            PY_LOOP002,
            PY_LOOP002_MARK,
        ),
        silent(PY_LOOP_002, "py/io_silent.py".to_string()),
        fire(
            PY_LOOP_003,
            "py/range.py".to_string(),
            PY_LOOP003,
            PY_LOOP003_MARK,
        ),
        silent(PY_LOOP_003, "py/range_silent.py".to_string()),
        fire(
            PY_LOOP_007,
            "python/loop.py".to_string(),
            PY_KPI_LOOP,
            "TP PERF-PY-LOOP-007",
        ),
        silent(PY_LOOP_007, "python/loop_silent.py".to_string()),
        fire(
            PY_LOOP_009,
            "py/tryloop.py".to_string(),
            PY_LOOP009,
            PY_LOOP009_MARK,
        ),
        silent(PY_LOOP_009, "py/tryloop_silent.py".to_string()),
        fire(
            PY_MEM_002,
            "python/mem.py".to_string(),
            PY_KPI_MEM,
            "TP PERF-PY-MEM-002",
        ),
        silent(PY_MEM_002, "python/mem_silent.py".to_string()),
        fire(PY_N1_001, "py/n1.py".to_string(), PY_N1, PY_N1_MARK),
        silent(PY_N1_001, "py/n1_silent.py".to_string()),
        fire(
            PY_PANDAS_001,
            "py/pandas1.py".to_string(),
            PY_PANDAS001,
            PY_PANDAS001_MARK,
        ),
        silent(PY_PANDAS_001, "py/pandas1_silent.py".to_string()),
        fire(
            PY_PANDAS_002,
            "py/pandas2.py".to_string(),
            PY_PANDAS002,
            PY_PANDAS002_MARK,
        ),
        silent(PY_PANDAS_002, "py/pandas2_silent.py".to_string()),
        fire(
            PY_STR_001,
            "python/str.py".to_string(),
            PY_KPI_STR,
            "TP PERF-PY-STR-001",
        ),
        silent(PY_STR_001, "python/str_silent.py".to_string()),
    ];

    (files, labels)
}

// ===========================================================================
// Gate units (the nine M1 rules) — UNCHANGED, and the only units replicated
// into the medium/large scale ladder.
// ===========================================================================

/// One C gate replica: 12 files, 5 fired gate rules + silent controls.
fn gate_c_unit(s: &str) -> (Vec<GenFile>, Vec<Label>) {
    let alloc_tp = C_ALLOC_TP.replace("@S@", s);
    let alloc_silent = C_ALLOC_SILENT.replace("@S@", s);
    let arch_tp = C_ARCH_TP.replace("@S@", s);
    let arch_helpers = C_ARCH_HELPERS.replace("@S@", s);
    let arch_other = C_ARCH_OTHER.replace("@S@", s);
    let arch_silent = C_ARCH_SILENT.replace("@S@", s);
    let mem_tp = C_MEM_TP.replace("@S@", s);
    let mem_silent = C_MEM_SILENT.replace("@S@", s);
    let loop_tp = C_LOOP_TP.replace("@S@", s);
    let loop_silent = C_LOOP_SILENT.replace("@S@", s);
    let str_tp = C_STR_TP.replace("@S@", s);
    let str_silent = C_STR_SILENT.replace("@S@", s);

    let files = vec![
        gf(format!("c/alloc{s}.c"), alloc_tp.clone()),
        gf(format!("c/alloc_silent{s}.c"), alloc_silent.clone()),
        gf(format!("c/arch{s}.c"), arch_tp.clone()),
        gf(format!("c/arch_helpers{s}.c"), arch_helpers.clone()),
        gf(format!("c/arch_other{s}.c"), arch_other.clone()),
        gf(format!("c/arch_silent{s}.c"), arch_silent.clone()),
        gf(format!("c/mem{s}.c"), mem_tp.clone()),
        gf(format!("c/mem_silent{s}.c"), mem_silent.clone()),
        gf(format!("c/loop{s}.c"), loop_tp.clone()),
        gf(format!("c/loop_silent{s}.c"), loop_silent.clone()),
        gf(format!("c/str{s}.c"), str_tp.clone()),
        gf(format!("c/str_silent{s}.c"), str_silent.clone()),
    ];

    let labels = vec![
        fire(
            C_ALLOC_001,
            format!("c/alloc{s}.c"),
            &alloc_tp,
            C_ALLOC_MARK,
        ),
        silent(C_ALLOC_001, format!("c/alloc_silent{s}.c")),
        fire(C_ARCH_003, format!("c/arch{s}.c"), &arch_tp, C_ARCH_MARK),
        silent(C_ARCH_003, format!("c/arch_silent{s}.c")),
        fire(C_MEM_002, format!("c/mem{s}.c"), &mem_tp, C_MEM_MARK),
        silent(C_MEM_002, format!("c/mem_silent{s}.c")),
        fire(C_LOOP_008, format!("c/loop{s}.c"), &loop_tp, C_LOOP_MARK),
        silent(C_LOOP_008, format!("c/loop_silent{s}.c")),
        fire(C_STR_001, format!("c/str{s}.c"), &str_tp, C_STR_MARK),
        silent(C_STR_001, format!("c/str_silent{s}.c")),
    ];

    (files, labels)
}

/// One Rust gate replica: 10 files, 4 fired gate rules + silent controls,
/// plus the retired `PERF-RS-STR-001` always-silent control.
fn gate_rust_unit(s: &str) -> (Vec<GenFile>, Vec<Label>) {
    let alloc_tp = RS_ALLOC_TP.replace("@S@", s);
    let alloc_silent = RS_ALLOC_SILENT.replace("@S@", s);
    let arch_tp = RS_ARCH_TP.replace("@S@", s);
    let arch_helpers = RS_ARCH_HELPERS.replace("@S@", s);
    let arch_extra = RS_ARCH_EXTRA.replace("@S@", s);
    let arch_silent = RS_ARCH_SILENT.replace("@S@", s);
    let mem_tp = RS_MEM_TP.replace("@S@", s);
    let mem_silent = RS_MEM_SILENT.replace("@S@", s);
    let loop_tp = RS_LOOP_TP.replace("@S@", s);
    let loop_silent = RS_LOOP_SILENT.replace("@S@", s);

    let files = vec![
        gf(format!("rust/alloc{s}.rs"), alloc_tp.clone()),
        gf(format!("rust/alloc_silent{s}.rs"), alloc_silent.clone()),
        gf(format!("rust/arch{s}.rs"), arch_tp.clone()),
        gf(format!("rust/arch_helpers{s}.rs"), arch_helpers.clone()),
        gf(format!("rust/arch_extra{s}.rs"), arch_extra.clone()),
        gf(format!("rust/arch_silent{s}.rs"), arch_silent.clone()),
        gf(format!("rust/mem{s}.rs"), mem_tp.clone()),
        gf(format!("rust/mem_silent{s}.rs"), mem_silent.clone()),
        gf(format!("rust/loop_chain{s}.rs"), loop_tp.clone()),
        gf(format!("rust/loop_silent{s}.rs"), loop_silent.clone()),
    ];

    let labels = vec![
        fire(
            RS_ALLOC_001,
            format!("rust/alloc{s}.rs"),
            &alloc_tp,
            RS_ALLOC_MARK,
        ),
        silent(RS_ALLOC_001, format!("rust/alloc_silent{s}.rs")),
        fire(
            RS_ARCH_003,
            format!("rust/arch{s}.rs"),
            &arch_tp,
            RS_ARCH_MARK,
        ),
        silent(RS_ARCH_003, format!("rust/arch_silent{s}.rs")),
        fire(RS_MEM_002, format!("rust/mem{s}.rs"), &mem_tp, RS_MEM_MARK),
        silent(RS_MEM_002, format!("rust/mem_silent{s}.rs")),
        fire(
            RS_LOOP_008,
            format!("rust/loop_chain{s}.rs"),
            &loop_tp,
            RS_LOOP_MARK,
        ),
        silent(RS_LOOP_008, format!("rust/loop_silent{s}.rs")),
        // Retired rule: no emitter, so the control is silent by construction.
        silent(RS_STR_001, format!("rust/alloc_silent{s}.rs")),
    ];

    (files, labels)
}

/// Files produced by one C+Rust gate replica (used to size the scale ladder).
const FILES_PER_UNIT: usize = 22;

/// The `small` rung: the nine M1 gate constructs plus a true positive and a
/// silent control for EVERY remaining rule in the 67-rule catalog.
/// Precision FP-class fix fixtures (fixes 1-4): the committed M1
/// silent-case + TP fixtures from `tests/fixtures/precision/`, lifted via
/// `include_str!` — byte-verbatim BY CONSTRUCTION, so the M1 anchors can
/// never drift from the KPI corpus (function names are unique across the
/// generated tree, so the ARCH-003 uniqueness gate is undisturbed). Labels
/// use explicit anchor lines where TP markers repeat across sibling
/// functions (`realloc_guard.c`); rung scoring verifies every anchor
/// end-to-end (drift fails loudly as FN/FP).
const FIX1_DOWHILE: &str = include_str!("../fixtures/precision/c/loop_dowhile.c");
const FIX2_CFGDEAD: &str = include_str!("../fixtures/precision/c/cfg_dead.c");
const FIX4_REALLOC: &str = include_str!("../fixtures/precision/c/realloc_guard.c");
const FIX3_EMPTYCTOR: &str = include_str!("../fixtures/precision/rust/alloc_empty.rs");

/// Plant the fix fixtures at their M1 paths with the M1 anchor lines.
fn fix_units() -> (Vec<GenFile>, Vec<Label>) {
    let files = vec![
        gf("c/loop_dowhile.c".to_string(), FIX1_DOWHILE.to_string()),
        gf("c/cfg_dead.c".to_string(), FIX2_CFGDEAD.to_string()),
        gf("c/realloc_guard.c".to_string(), FIX4_REALLOC.to_string()),
        gf(
            "rust/alloc_empty.rs".to_string(),
            FIX3_EMPTYCTOR.to_string(),
        ),
    ];
    let at = |rule_id: &'static str, rel_path: &str, line: u32| Label {
        rule_id,
        rel_path: rel_path.to_string(),
        line,
        expect: Expect::Fire,
    };
    let labels = vec![
        at(C_LOOP_002, "c/loop_dowhile.c", 12),
        at(C_LOOP_002, "c/loop_dowhile.c", 22),
        at(C_ALLOC_001, "c/cfg_dead.c", 12),
        at(C_ALLOC_002, "c/cfg_dead.c", 15),
        at(C_ALLOC_001, "c/cfg_dead.c", 41),
        at(C_ALLOC_002, "c/cfg_dead.c", 44),
        at(C_ALLOC_001, "c/realloc_guard.c", 13),
        at(C_ALLOC_003, "c/realloc_guard.c", 13),
        at(C_ALLOC_001, "c/realloc_guard.c", 24),
        at(RS_ALLOC_001, "rust/alloc_empty.rs", 28),
    ];
    // Marker-anchored duplicates of the unambiguous anchors above: the
    // markers MUST be unique or `fire` fails loudly, so these double as
    // verbatim-lift drift locks (a fixture edit that shifts lines breaks
    // the M1 KPI first, and a const edit that shifts lines breaks here).
    let dowhile = contents_of(&files, "c/loop_dowhile.c").to_string();
    assert_eq!(anchor_line(&dowhile, "i=%d"), 12);
    assert_eq!(anchor_line(&dowhile, "do=%d"), 22);
    let cfgdead = contents_of(&files, "c/cfg_dead.c").to_string();
    assert_eq!(anchor_line(&cfgdead, "malloc(16)"), 12);
    assert_eq!(anchor_line(&cfgdead, "malloc(64)"), 41);
    let emptyctor = contents_of(&files, "rust/alloc_empty.rs").to_string();
    assert_eq!(anchor_line(&emptyctor, "v[0]"), 30);
    (files, labels)
}

pub fn small() -> Rung {
    let (mut files, mut labels) = gate_c_unit("");
    let (mut rf, mut rl) = gate_rust_unit("");
    files.append(&mut rf);
    labels.append(&mut rl);
    // Collateral true positives in the committed M1 gate files, now that the
    // full catalog is scoped: the in-loop `free` (ALLOC-002), the unbounded
    // `strcat` sites (STR-003), and the recomputed `len()` calls (STR-002).
    let alloc_c = contents_of(&files, "c/alloc.c").to_string();
    labels.push(fire(
        C_ALLOC_002,
        "c/alloc.c".to_string(),
        &alloc_c,
        "free(p);",
    ));
    let str_c = contents_of(&files, "c/str.c").to_string();
    labels.push(fire(
        C_STR_003,
        "c/str.c".to_string(),
        &str_c,
        "strcat(dst, src);",
    ));
    let str_sil = contents_of(&files, "c/str_silent.c").to_string();
    labels.push(fire(
        C_STR_003,
        "c/str_silent.c".to_string(),
        &str_sil,
        "strcat(dst, src);",
    ));
    let rust_alloc = contents_of(&files, "rust/alloc.rs").to_string();
    labels.push(fire(
        RS_STR_002,
        "rust/alloc.rs".to_string(),
        &rust_alloc,
        "v.len()",
    ));
    let rust_alloc_sil = contents_of(&files, "rust/alloc_silent.rs").to_string();
    labels.push(fire(
        RS_STR_002,
        "rust/alloc_silent.rs".to_string(),
        &rust_alloc_sil,
        "s.len()",
    ));

    // Precision FP-class fix fixtures (fixes 1-4): the committed M1
    // silent-case + TP fixtures, lifted verbatim so every fix anchor the M1
    // KPI pins is also planted here (`assert_small_anchors_match_m1`).
    let (mut ff, mut fl) = fix_units();
    files.append(&mut ff);
    labels.append(&mut fl);

    // The 36 remaining rules: dedicated minimal files.
    let (mut ef, mut el) = c_ext();
    files.append(&mut ef);
    labels.append(&mut el);
    let (mut ef, mut el) = rust_ext();
    files.append(&mut ef);
    labels.append(&mut el);

    let (mut ef, mut el) = py_ext();
    files.append(&mut ef);
    labels.append(&mut el);

    Rung {
        name: "small",
        files,
        labels,
    }
}

/// A `medium`/`large` rung: N replicas of the NINE gate-rule mutation set,
/// each carrying UNIQUE per-file function/definition names (the `_k` suffix)
/// so no replica can create cross-file name ambiguity, a false ARCH-003 edge,
/// or an accidental include/component hub. `target_files` is rounded UP to
/// whole replicas of [`FILES_PER_UNIT`].
pub fn replicated(name: &'static str, target_files: usize) -> Rung {
    let replicas = target_files.div_ceil(FILES_PER_UNIT).max(1);
    let mut files = Vec::new();
    let mut labels = Vec::new();
    for k in 0..replicas {
        let suffix = format!("_{k}");
        let (mut cf, mut cl) = gate_c_unit(&suffix);
        files.append(&mut cf);
        labels.append(&mut cl);
        let (mut rf, mut rl) = gate_rust_unit(&suffix);
        files.append(&mut rf);
        labels.append(&mut rl);
    }
    Rung {
        name,
        files,
        labels,
    }
}

/// The labeled scale ladder (deterministic order). Scale targets follow the
/// plan's suggested sizes (medium ~120, large ~400; rounded up to whole
/// replicas of [`FILES_PER_UNIT`]). The `small` rung covers the full catalog;
/// `medium`/`large` replicate the nine gate rules only.
pub fn catalog() -> Vec<Rung> {
    vec![small(), replicated("medium", 120), replicated("large", 400)]
}
