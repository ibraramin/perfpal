//! Parser-free Wave C fixture tests (ADR-0001 guardrail 2): MEM-001/002,
//! TYPE-001/002, and LOOP-005/006/007/008 run against serialized IR under
//! `tests/fixtures/ir/` with NO tree-sitter binding. Covers detection,
//! negatives/suppression, and the ir-v1.md §5 downgrade hooks on the new
//! rules (hooks are inherited centrally), plus the catalog framing guard
//! over every emitted message.

mod common;

use perfpal::analyzer::finding::{
    LOOP_005_RULE_ID, LOOP_006_RULE_ID, LOOP_007_RULE_ID, LOOP_008_RULE_ID, MEM_001_RULE_ID,
    MEM_002_RULE_ID, TYPE_001_RULE_ID, TYPE_002_RULE_ID,
};
use perfpal::analyzer::{Confidence, Severity, analyze};

// ===========================================================================
// PERF-C-MEM-002 — block copy/set family in loop
// ===========================================================================

#[test]
fn non_literal_block_ops_fire_info_outer_and_warn_innermost() {
    let units = common::load_units_fixture("PERF-C-MEM-002__memcpy-in-loop.json");
    let findings = analyze(&units);
    let mem002 = common::by_rule(&findings, MEM_002_RULE_ID);
    assert_eq!(
        mem002.len(),
        2,
        "innermost memcpy + outer memmove fire; literal-size memset \
         and out-of-loop memcpy stay silent in the same fixture"
    );

    let innermost = mem002
        .iter()
        .find(|f| f.span.start_line == 7)
        .expect("innermost memcpy finding");
    assert_eq!(innermost.severity, Severity::Warn, "innermost ⇒ warn");
    assert_eq!(innermost.confidence, Confidence::Medium);
    assert_eq!(
        innermost.evidence.get("callee").map(String::as_str),
        Some("memcpy")
    );
    assert_eq!(
        innermost.evidence.get("size_arg_kind").map(String::as_str),
        Some("identifier")
    );
    assert_eq!(
        innermost.evidence.get("loop_depth").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        innermost.evidence.get("innermost").map(String::as_str),
        Some("true")
    );
    assert!(innermost.message.contains("candidate optimization"));

    let outer = mem002
        .iter()
        .find(|f| f.span.start_line == 17)
        .expect("outer memmove finding");
    assert_eq!(outer.severity, Severity::Info, "non-innermost stays info");
    assert_eq!(
        outer.evidence.get("callee").map(String::as_str),
        Some("memmove")
    );
}

#[test]
fn literal_size_and_out_of_loop_block_ops_stay_silent() {
    let units = common::load_units_fixture("PERF-C-MEM-002__memcpy-in-loop.json");
    let findings = analyze(&units);
    // memset(hdr, 0, 64) — literal byte count is compiler-friendly.
    assert!(
        findings.iter().all(|f| f.span.start_line != 13),
        "literal-size memset must stay silent (line 13)"
    );
    // memcpy outside any loop — not per-iteration churn.
    assert!(
        findings.iter().all(|f| f.span.start_line != 22),
        "out-of-loop memcpy must stay silent (line 22)"
    );
}

#[test]
fn macro_origin_downgrades_mem_002_confidence_and_flags() {
    let units = common::load_units_fixture("PERF-C-MEM-002__macro-origin.json");
    let findings = analyze(&units);
    let mem002 = common::by_rule(&findings, MEM_002_RULE_ID);
    assert_eq!(mem002.len(), 1, "macro-tainted memcpy still fires");
    let f = mem002[0];
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin caps confidence at MEDIUM"
    );
    assert_eq!(
        f.severity,
        Severity::Warn,
        "severity band unchanged by hook"
    );
}

// ===========================================================================
// PERF-C-MEM-001 — sizeof-misuse shapes in allocation args
// ===========================================================================

#[test]
fn all_four_sizeof_misuse_classes_fire_warn_medium() {
    let units = common::load_units_fixture("PERF-C-MEM-001__sizeof-misuse.json");
    let findings = analyze(&units);
    let mem001 = common::by_rule(&findings, MEM_001_RULE_ID);
    assert_eq!(
        mem001.len(),
        4,
        "flagship pointer-param sizeof, multiply-sizeof, \
         sizeof-of-sizeof, and divide-by-sizeof fire"
    );

    let misuse_at = |line: u32| {
        mem001
            .iter()
            .find(|f| f.span.start_line == line)
            .unwrap_or_else(|| panic!("MEM-001 finding at line {line}"))
    };

    // (i) flagship: malloc(count * sizeof(it)) with `struct item *it`.
    let flagship = misuse_at(5);
    assert_eq!(
        flagship.evidence.get("misuse").map(String::as_str),
        Some("pointer_param_sizeof")
    );
    assert!(flagship.message.contains("sizeof misuse shape"));

    // (ii) multiply-sizeof: sizeof(int) * sizeof(char).
    assert_eq!(
        misuse_at(8).evidence.get("misuse").map(String::as_str),
        Some("multiply_sizeof")
    );

    // (iii) sizeof-of-sizeof via direct arg sizeof(sizeof(int)).
    assert_eq!(
        misuse_at(11).evidence.get("misuse").map(String::as_str),
        Some("sizeof_of_sizeof")
    );

    // (iv) divide-by-sizeof: calloc(count, n / sizeof(long)).
    assert_eq!(
        misuse_at(14).evidence.get("misuse").map(String::as_str),
        Some("divide_by_sizeof")
    );

    for f in &mem001 {
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.confidence, Confidence::Medium);
    }
}

#[test]
fn literal_and_non_pointer_sizeof_allocs_stay_silent() {
    let units = common::load_units_fixture("PERF-C-MEM-001__sizeof-misuse.json");
    let findings = analyze(&units);
    // malloc(64): plain literal size.
    assert!(
        findings.iter().all(|f| f.span.start_line != 17),
        "literal-size alloc must stay silent (line 17)"
    );
    // malloc(count * sizeof(struct item)): operand is a TYPE NAME, not a
    // bare identifier resolving to a pointer-typed param.
    assert!(
        findings.iter().all(|f| f.span.start_line != 18),
        "sizeof(struct item) multiplication must stay silent (line 18)"
    );
    // malloc(sizeof(int)): correct element-type sizing.
    assert!(
        findings.iter().all(|f| f.span.start_line != 21),
        "sizeof(int) literal-size alloc must stay silent (line 21)"
    );
}

// ===========================================================================
// PERF-C-TYPE-001 — large struct passed by value
// ===========================================================================

#[test]
fn by_value_large_struct_fires_warn_low_with_size_evidence() {
    let units = common::load_units_fixture("PERF-C-TYPE-001__large-struct-by-value.json");
    let findings = analyze(&units);
    let type001 = common::by_rule(&findings, TYPE_001_RULE_ID);
    assert_eq!(
        type001.len(),
        1,
        "only process_big(struct Big) fires; Small is below the floor \
         and peek takes a pointer"
    );
    let f = type001[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::Low, "approximate size model");
    assert_eq!(f.span.start_line, 12, "anchored at the function record");
    assert_eq!(
        f.evidence.get("params").map(String::as_str),
        Some("struct Big b")
    );
    assert_eq!(f.evidence.get("structs").map(String::as_str), Some("Big"));
    assert_eq!(
        f.evidence.get("estimated_sizes").map(String::as_str),
        Some("48"),
        "LP64 model: char[32] + long + int (+ tail padding)"
    );
    assert_eq!(f.evidence.get("threshold").map(String::as_str), Some("32"));
    assert!(f.message.contains("candidate optimization"));
}

#[test]
fn two_by_value_params_aggregate_into_one_finding() {
    let units = common::load_units_fixture("PERF-C-TYPE-001__two-by-value-params.json");
    let findings = analyze(&units);
    let type001 = common::by_rule(&findings, TYPE_001_RULE_ID);
    assert_eq!(
        type001.len(),
        1,
        "ONE finding per function even with two offending params — \
         per-param findings would share the function span and collapse \
         under deterministic dedup"
    );
    let f = &type001[0];
    assert_eq!(
        f.evidence.get("params").map(String::as_str),
        Some("struct Big a; struct Wide b"),
        "source order preserved"
    );
    assert_eq!(
        f.evidence.get("structs").map(String::as_str),
        Some("Big;Wide")
    );
    assert_eq!(
        f.evidence.get("estimated_sizes").map(String::as_str),
        Some("48;40"),
        "Big: char[32]+long+int+tail; Wide: long+int+tail"
    );
    assert!(
        f.message.contains("'Big'") && f.message.contains("'Wide'"),
        "every offending struct must stay visible in the message: {f:?}"
    );
}

#[test]
fn pointer_param_and_small_struct_params_stay_silent() {
    let units = common::load_units_fixture("PERF-C-TYPE-001__large-struct-by-value.json");
    let findings = analyze(&units);
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id != TYPE_001_RULE_ID || f.span.start_line != 19),
        "`struct Big *b` passes by reference — never fires"
    );
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id != TYPE_001_RULE_ID || f.span.start_line != 16),
        "`struct Small s` estimates below the 32-byte floor"
    );
}

// ===========================================================================
// PERF-C-TYPE-002 — struct member ordering / padding candidate
// ===========================================================================

#[test]
fn padding_waste_fires_info_low_only_above_threshold() {
    let units = common::load_units_fixture("PERF-C-TYPE-002__padding.json");
    let findings = analyze(&units);
    let type002 = common::by_rule(&findings, TYPE_002_RULE_ID);
    assert_eq!(
        type002.len(),
        1,
        "Wasteful fires; Tight (zero padding) and Solo (single member) \
         stay silent"
    );
    let f = type002[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::Low, "approximate size model");
    assert_eq!(f.span.start_line, 3, "anchored at the struct def span");
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id != TYPE_002_RULE_ID || f.span.start_line != 15),
        "single-member structs never fire — ordering cannot reclaim \
         anything when there is nothing to reorder"
    );
    assert_eq!(
        f.evidence.get("struct").map(String::as_str),
        Some("Wasteful")
    );
    assert_eq!(
        f.evidence.get("padding_bytes").map(String::as_str),
        Some("14"),
        "aligned estimate 32 vs packed sum 18"
    );
    assert_eq!(f.evidence.get("packed_sum").map(String::as_str), Some("18"));
    assert_eq!(
        f.evidence.get("aligned_estimate").map(String::as_str),
        Some("32")
    );
    assert_eq!(f.evidence.get("threshold").map(String::as_str), Some("8"));
}

#[test]
fn error_region_overlap_downgrades_type_002_to_medium() {
    let units = common::load_units_fixture("PERF-C-TYPE-002__error-region-overlap.json");
    let findings = analyze(&units);
    assert_eq!(
        findings.len(),
        1,
        "finding inside an error region is downgraded, never silently dropped"
    );
    let f = &findings[0];
    assert_eq!(f.rule_id, TYPE_002_RULE_ID);
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps LOW confidence at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect);
}

// ===========================================================================
// PERF-C-LOOP-005 — global variable written inside loop
// ===========================================================================

#[test]
fn global_writes_in_loop_fire_info_medium_with_dedup() {
    let units = common::load_units_fixture("PERF-C-LOOP-005__global-write-in-loop.json");
    let findings = analyze(&units);
    let loop005 = common::by_rule(&findings, LOOP_005_RULE_ID);
    assert_eq!(
        loop005.len(),
        3,
        "g_counter written twice in the SAME loop dedups to ONE finding \
         anchored at the earliest span; g_other and the while-loop write \
         are separate (target, loop) groups"
    );

    // Dedup proof: (g_counter, for-loop) anchors at line 7, not line 8.
    let counter_for = loop005
        .iter()
        .find(|f| f.evidence.get("target").map(String::as_str) == Some("g_counter"))
        .expect("g_counter finding");
    assert_eq!(
        counter_for.span.start_line, 7,
        "earliest assignment span wins within one (target, loop) group"
    );
    assert_eq!(counter_for.severity, Severity::Info);
    assert_eq!(counter_for.confidence, Confidence::Medium);
    assert_eq!(
        counter_for.evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
    assert!(counter_for.message.contains("candidate optimization"));

    // Second global in the same loop is its own finding.
    let other = loop005
        .iter()
        .find(|f| f.evidence.get("target").map(String::as_str) == Some("g_other"))
        .expect("g_other finding");
    assert_eq!(other.span.start_line, 9);

    // Same target in a DIFFERENT enclosing loop is its own finding.
    let counter_while = loop005
        .iter()
        .filter(|f| f.evidence.get("target").map(String::as_str) == Some("g_counter"))
        .count();
    assert_eq!(
        counter_while, 2,
        "for-loop and while-loop groups are distinct"
    );
}

#[test]
fn out_of_loop_and_local_target_writes_stay_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-005__global-write-in-loop.json");
    let findings = analyze(&units);
    // g_counter = 0 outside any loop — not per-iteration churn.
    assert!(
        findings.iter().all(|f| f.span.start_line != 14),
        "out-of-loop global write must stay silent (line 14)"
    );
    // x = 1 targets a local — scope gate.
    assert!(
        findings.iter().all(|f| f.span.start_line != 19),
        "local-target assignment must stay silent (line 19)"
    );
    // Defensive gate at the IR-contract level: a Global-classified
    // assignment whose target is NOT an Identifier never fires. Today's
    // backend only classifies bare identifiers, but the parser-free
    // harness pins the guarantee for ANY IR producer.
    assert!(
        findings.iter().all(|f| f.span.start_line != 25),
        "non-Identifier target must stay silent even when \
         scope_class == global (line 25)"
    );
}

#[test]
fn macro_origin_downgrades_loop_005_and_flags() {
    let units = common::load_units_fixture("PERF-C-LOOP-005__macro-origin.json");
    let findings = analyze(&units);
    let loop005 = common::by_rule(&findings, LOOP_005_RULE_ID);
    assert_eq!(loop005.len(), 1, "macro-tainted global write still fires");
    let f = loop005[0];
    assert!(f.macro_suspect, "central hook flags macro origin");
    assert!(!f.error_node_overlap, "hooks fire independently");
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "macro origin caps confidence at MEDIUM"
    );
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
}

// ===========================================================================
// PERF-C-LOOP-006 — indirect call via function pointer in innermost loop
// ===========================================================================

#[test]
fn indirect_call_innermost_fires_info_low() {
    let units = common::load_units_fixture("PERF-C-LOOP-006__indirect-call-innermost.json");
    let findings = analyze(&units);
    let loop006 = common::by_rule(&findings, LOOP_006_RULE_ID);
    assert_eq!(
        loop006.len(),
        1,
        "only the innermost-loop indirect call fires"
    );
    let f = loop006[0];
    assert_eq!(f.span.start_line, 7);
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(
        f.confidence,
        Confidence::Low,
        "no type info confirming a function pointer — LOW mandatory"
    );
    assert_eq!(
        f.evidence.get("callee").map(String::as_str),
        Some("handler")
    );
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("2"));
    assert!(f.message.contains("candidate optimization"));
}

#[test]
fn out_of_loop_indirect_and_direct_calls_stay_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-006__indirect-call-innermost.json");
    let findings = analyze(&units);
    // handler(v) outside any loop — not per-iteration dispatch.
    assert!(
        findings.iter().all(|f| f.span.start_line != 10),
        "out-of-loop indirect call must stay silent (line 10)"
    );
    // compute(v) is a DIRECT call — callee_kind gate.
    assert!(
        findings.iter().all(|f| f.span.start_line != 11),
        "direct call must stay silent (line 11)"
    );
}

#[test]
fn error_region_overlap_downgrades_loop_006_to_medium() {
    let units = common::load_units_fixture("PERF-C-LOOP-006__error-region-overlap.json");
    let findings = analyze(&units);
    assert_eq!(
        findings.len(),
        1,
        "finding inside an error region is downgraded, never silently dropped"
    );
    let f = &findings[0];
    assert_eq!(f.rule_id, LOOP_006_RULE_ID);
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps LOW confidence at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect);
}

// ===========================================================================
// PERF-C-LOOP-007 — linear membership scan nested in loop
// ===========================================================================

#[test]
fn induced_subscript_vs_non_literal_at_depth_two_fires_info_low() {
    let units = common::load_units_fixture("PERF-C-LOOP-007__membership-scan.json");
    let findings = analyze(&units);
    let loop007 = common::by_rule(&findings, LOOP_007_RULE_ID);
    assert_eq!(
        loop007.len(),
        1,
        "only arr[j] == key at depth 2 with j an induction var of the \
         enclosing loop fires"
    );
    let f = loop007[0];
    assert_eq!(f.span.start_line, 5);
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(
        f.confidence,
        Confidence::Low,
        "candidate-grade only — noisy without types"
    );
    assert_eq!(f.evidence.get("root").map(String::as_str), Some("arr"));
    assert_eq!(f.evidence.get("index_ident").map(String::as_str), Some("j"));
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("2"));
    assert!(f.message.contains("candidate optimization"));
}

#[test]
fn depth_one_literal_and_non_induction_comparisons_stay_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-007__membership-scan.json");
    let findings = analyze(&units);
    // arr[k] == key: k is NOT an induction var of any enclosing loop.
    assert!(
        findings.iter().all(|f| f.span.start_line != 6),
        "non-induction index comparison must stay silent (line 6)"
    );
    // arr[j] == '\0': literal other side — sentinel/bounds idiom guard.
    assert!(
        findings.iter().all(|f| f.span.start_line != 7),
        "literal-vs-subscript comparison must stay silent (line 7)"
    );
    // arr[i] == key at depth 1 — no outer×inner product.
    assert!(
        findings.iter().all(|f| f.span.start_line != 11),
        "depth-1 comparison must stay silent (line 11)"
    );
}

// ===========================================================================
// PERF-C-LOOP-008 — pointer-chasing chain in innermost loop
// ===========================================================================

#[test]
fn deep_chains_innermost_fire_info_medium_including_vtable_fp_class() {
    let units = common::load_units_fixture("PERF-C-LOOP-008__pointer-chase.json");
    let findings = analyze(&units);
    let loop008 = common::by_rule(&findings, LOOP_008_RULE_ID);
    assert_eq!(
        loop008.len(),
        2,
        "both depth-3 innermost chains fire — including the ctx->ops->fn \
         vtable-shaped chain, which is the catalog-documented FP class \
         that still reports at Info/MEDIUM by contract"
    );
    for f in &loop008 {
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.confidence, Confidence::Medium);
        assert_eq!(f.evidence.get("root").map(String::as_str), Some("p"));
        assert_eq!(f.evidence.get("chain_depth").map(String::as_str), Some("3"));
        assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
        assert!(f.message.contains("candidate optimization"));
    }
    let lines: Vec<u32> = loop008.iter().map(|f| f.span.start_line).collect();
    assert_eq!(lines, [10, 11], "document order preserved");
}

#[test]
fn shallow_chain_and_out_of_loop_chain_stay_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-008__pointer-chase.json");
    let findings = analyze(&units);
    // p->next->rank: chain depth 2 below the ≥3 floor.
    assert!(
        findings.iter().all(|f| f.span.start_line != 12),
        "depth-2 chain must stay silent (line 12)"
    );
    // head->data->value->weight outside any loop — not per-iteration.
    assert!(
        findings.iter().all(|f| f.span.start_line != 14),
        "deep chain outside a loop must stay silent (line 14)"
    );
}

// ===========================================================================
// Framing guard (catalog rule): optimization candidate, never bug/unsafe
// ===========================================================================

#[test]
fn wave_c_messages_never_frame_as_bug_or_unsafe() {
    const FIXTURES: [&str; 13] = [
        "PERF-C-MEM-001__sizeof-misuse.json",
        "PERF-C-MEM-002__memcpy-in-loop.json",
        "PERF-C-MEM-002__macro-origin.json",
        "PERF-C-TYPE-001__large-struct-by-value.json",
        "PERF-C-TYPE-001__two-by-value-params.json",
        "PERF-C-TYPE-002__padding.json",
        "PERF-C-TYPE-002__error-region-overlap.json",
        "PERF-C-LOOP-005__global-write-in-loop.json",
        "PERF-C-LOOP-005__macro-origin.json",
        "PERF-C-LOOP-006__indirect-call-innermost.json",
        "PERF-C-LOOP-006__error-region-overlap.json",
        "PERF-C-LOOP-007__membership-scan.json",
        "PERF-C-LOOP-008__pointer-chase.json",
    ];
    for name in FIXTURES {
        let units = common::load_units_fixture(name);
        for f in analyze(&units) {
            assert!(
                !f.message.contains("bug") && !f.message.contains("unsafe"),
                "{name}: catalog framing rule violated — {}",
                f.message
            );
        }
    }
}
