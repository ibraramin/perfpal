//! Parser-free Wave A fixture tests (ADR-0001 guardrail 2): ALLOC-002,
//! STR-001/002/003, LOOP-001/003, and CALL-001 run against serialized IR
//! under `tests/fixtures/ir/` with NO tree-sitter binding. Covers detection,
//! negatives/suppression, and both ir-v1.md §5 downgrade hooks on the new
//! rules, plus a kitchen-sink synthetic unit tripping all twelve registry
//! rules in report §4 total order.

mod common;

use std::collections::BTreeSet;

use perfpal::analyzer::finding::{
    ALLOC_001_RULE_ID, ALLOC_002_RULE_ID, ALLOC_003_RULE_ID, CALL_001_RULE_ID, CALL_002_RULE_ID,
    LOOP_001_RULE_ID, LOOP_002_RULE_ID, LOOP_003_RULE_ID, LOOP_004_RULE_ID, STR_001_RULE_ID,
    STR_002_RULE_ID, STR_003_RULE_ID,
};
use perfpal::analyzer::{Confidence, Finding, Severity, analyze};
use perfpal::ir::{
    BinaryOpEvent, CallEvent, CalleeKind, EventMeta, FileRecord, FunctionRecord, IncludeCounts,
    LocCounts, LoopKind, LoopNode, ParseHealth, ResolverTier, Shape, ShapeKind, Span,
    StatementRole,
};
use perfpal::scanner::{FileUnit, FunctionUnit};

// ===========================================================================
// PERF-C-ALLOC-002 — deallocation inside loop
// ===========================================================================

#[test]
fn free_in_loop_warns_and_cooccurrence_escalates_to_hot() {
    let units = common::load_units_fixture("PERF-C-ALLOC-002__free-in-loop.json");
    let findings = analyze(&units);
    let alloc002 = common::by_rule(&findings, ALLOC_002_RULE_ID);
    assert_eq!(
        alloc002.len(),
        2,
        "one plain free-in-loop + one co-occurring"
    );

    // Plain case: free in a loop with no alloc site in the same function.
    let plain = alloc002
        .iter()
        .find(|f| f.path == "src/mem.c" && f.span.start_line == 6)
        .expect("plain free-in-loop finding");
    assert_eq!(plain.severity, Severity::Warn);
    assert_eq!(plain.confidence, Confidence::High);
    assert!(!plain.evidence.contains_key("alloc_cooccurrence"));
    assert_eq!(
        plain.evidence.get("callee").map(String::as_str),
        Some("free")
    );
    assert_eq!(
        plain.evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
    assert!(plain.message.contains("deallocation via 'free'"));

    // Co-occurrence escalation: malloc + free in the SAME function.
    let cooc = alloc002
        .iter()
        .find(|f| f.span.start_line == 17)
        .expect("co-occurring free-in-loop finding");
    assert_eq!(cooc.severity, Severity::Hot, "alloc pairing escalates");
    assert_eq!(cooc.confidence, Confidence::High);
    assert_eq!(
        cooc.evidence.get("alloc_cooccurrence").map(String::as_str),
        Some("true")
    );
}

#[test]
fn free_not_in_loop_stays_silent() {
    let units = common::load_units_fixture("PERF-C-ALLOC-002__free-not-in-loop.json");
    assert!(
        analyze(&units).is_empty(),
        "free outside any loop must not fire"
    );
}

#[test]
fn alloc002_finding_id_uses_rule_path_line_col() {
    let units = common::load_units_fixture("PERF-C-ALLOC-002__free-in-loop.json");
    let findings = analyze(&units);
    let f = common::by_rule(&findings, ALLOC_002_RULE_ID)[0];
    assert_eq!(
        f.finding_id(),
        format!(
            "{}:{}:{}:{}",
            f.rule_id, f.path, f.span.start_line, f.span.start_col
        )
    );
}

// ===========================================================================
// PERF-C-STR-001 — string accumulation in loop
// ===========================================================================

#[test]
fn strcat_and_strncat_in_loop_fire_hot_with_dest_evidence() {
    let units = common::load_units_fixture("PERF-C-STR-001__strcat-in-loop.json");
    let findings = analyze(&units);
    let str001 = common::by_rule(&findings, STR_001_RULE_ID);
    assert_eq!(str001.len(), 2, "strcat and strncat variants both fire");

    let strcat = str001
        .iter()
        .find(|f| f.evidence.get("callee").map(String::as_str) == Some("strcat"))
        .expect("strcat finding");
    assert_eq!(strcat.severity, Severity::Hot);
    assert_eq!(strcat.confidence, Confidence::High);
    assert_eq!(strcat.evidence.get("dest").map(String::as_str), Some("out"));
    assert_eq!(
        strcat.evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
    assert!(strcat.message.contains("'out'"));

    let strncat = str001
        .iter()
        .find(|f| f.evidence.get("callee").map(String::as_str) == Some("strncat"))
        .expect("strncat finding");
    assert_eq!(strncat.severity, Severity::Hot);
    assert_eq!(
        strncat.evidence.get("dest").map(String::as_str),
        Some("path")
    );
}

#[test]
fn strcat_not_in_loop_emits_no_str_001() {
    let units = common::load_units_fixture("PERF-C-STR-001__strcat-not-in-loop.json");
    let findings = analyze(&units);
    assert!(
        common::by_rule(&findings, STR_001_RULE_ID).is_empty(),
        "accumulation outside any loop must not fire STR-001"
    );
}

// ===========================================================================
// PERF-C-STR-002 — strlen() recomputed in loop
// ===========================================================================

#[test]
fn strlen_in_loop_fires_for_unreassigned_identifier() {
    let units = common::load_units_fixture("PERF-C-STR-002__strlen-in-loop.json");
    let findings = analyze(&units);
    let str002 = common::by_rule(&findings, STR_002_RULE_ID);
    assert_eq!(
        str002.len(),
        1,
        "only the unreassigned in-loop strlen fires"
    );
    let f = str002[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        f.span.start_line, 7,
        "finding anchors at header_len's strlen"
    );
    assert_eq!(
        f.evidence.get("identifier").map(String::as_str),
        Some("name")
    );
    assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
    assert!(f.message.contains("hoisting the length"));
}

#[test]
fn reassignment_suppression_and_outside_loop_stay_silent() {
    let units = common::load_units_fixture("PERF-C-STR-002__strlen-in-loop.json");
    let findings = analyze(&units);
    // grows_each_iter (assignment inside innermost loop, lines 13–21) and
    // outside_len (strlen at depth 0, lines 24–28) emit nothing.
    assert!(
        findings.iter().all(|f| f.span.start_line < 13),
        "suppressed and out-of-loop scenarios must stay silent, got {findings:?}"
    );
}

// ===========================================================================
// PERF-C-STR-003 — unbounded string operation
// ===========================================================================

#[test]
fn unbounded_family_fires_anywhere_without_loop() {
    let units = common::load_units_fixture("PERF-C-STR-003__unbounded-family.json");
    let findings = analyze(&units);
    let str003 = common::by_rule(&findings, STR_003_RULE_ID);
    assert_eq!(str003.len(), 4, "strcpy, sprintf, gets, scanf(%s) fire");

    let callees: Vec<&str> = str003
        .iter()
        .filter_map(|f| f.evidence.get("callee").map(String::as_str))
        .collect();
    assert_eq!(
        callees,
        ["strcpy", "sprintf", "gets", "scanf"],
        "document order follows start_line within the tie-broken total order"
    );

    for f in &str003 {
        assert_eq!(f.severity, Severity::Warn);
        assert_eq!(f.confidence, Confidence::High);
        assert!(f.message.contains("candidate optimization"));
    }

    // Only the %s scanf carries format evidence; copy-family findings don't.
    let scanf = str003[3];
    assert_eq!(scanf.evidence.get("format").map(String::as_str), Some("%s"));
    assert!(scanf.message.contains("width specifier"));
    for f in &str003[..3] {
        assert!(!f.evidence.contains_key("format"));
    }
}

#[test]
fn bounded_variants_and_non_string_formats_stay_silent() {
    let units = common::load_units_fixture("PERF-C-STR-003__unbounded-family.json");
    let findings = analyze(&units);
    assert!(
        findings
            .iter()
            .all(|f| f.evidence.get("callee").map(String::as_str) != Some("snprintf")),
        "snprintf is bounded — outside the family"
    );
    assert_eq!(
        common::by_rule(&findings, STR_003_RULE_ID)
            .iter()
            .filter(|f| f.evidence.get("callee").map(String::as_str) == Some("scanf"))
            .count(),
        1,
        "scanf(\"%d\", …) has no unbounded %s conversion"
    );
    for f in &findings {
        assert!(
            !f.message.contains("bug") && !f.message.contains("unsafe"),
            "catalog framing rule: optimization candidate, never bug/unsafe — {}",
            f.message
        );
    }
}

// ===========================================================================
// Downgrade hooks (ir-v1.md §5) on the new rules
// ===========================================================================

#[test]
fn macro_origin_downgrades_str_003_to_medium() {
    let units = common::load_units_fixture("PERF-C-STR-003__macro-origin.json");
    let findings = analyze(&units);
    let str003 = common::by_rule(&findings, STR_003_RULE_ID);
    assert_eq!(str003.len(), 2, "both macro-tainted calls still fire");
    for f in &str003 {
        assert_eq!(
            f.severity,
            Severity::Warn,
            "severity band unchanged by hook"
        );
        assert_eq!(
            f.confidence,
            Confidence::Medium,
            "macro origin caps confidence"
        );
        assert!(f.macro_suspect);
        assert!(!f.error_node_overlap);
    }
}

#[test]
fn error_region_overlap_downgrades_loop_001_to_medium() {
    let units = common::load_units_fixture("PERF-C-LOOP-001__error-region-overlap.json");
    let findings = analyze(&units);
    assert_eq!(
        findings.len(),
        1,
        "finding inside an error region is downgraded, never silently dropped"
    );
    let f = &findings[0];
    assert_eq!(f.rule_id, LOOP_001_RULE_ID);
    assert_eq!(
        f.severity,
        Severity::Warn,
        "severity band unchanged by hook"
    );
    assert_eq!(f.confidence, Confidence::Medium);
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect);
    assert_eq!(f.evidence.get("nest_depth").map(String::as_str), Some("3"));
}

// ===========================================================================
// PERF-C-LOOP-001 — deeply nested loops
// ===========================================================================

#[test]
fn depth_three_nest_fires_once_per_maximal_entry() {
    let units = common::load_units_fixture("PERF-C-LOOP-001__deep-nest.json");
    let findings = analyze(&units);
    let loop001 = common::by_rule(&findings, LOOP_001_RULE_ID);
    assert_eq!(
        loop001.len(),
        1,
        "one finding per maximal entry even with a depth-4 chain; \
         the double-nest function stays silent"
    );
    let f = loop001[0];
    assert_eq!(f.severity, Severity::Warn);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        (f.span.start_line, f.span.start_col),
        (7, 13),
        "anchors at the depth-3 ancestor of the chain"
    );
    assert_eq!(f.evidence.get("nest_depth").map(String::as_str), Some("3"));
    assert_eq!(f.evidence.get("max_depth").map(String::as_str), Some("4"));
    assert!(f.message.contains("flattening the nesting"));
}

// ===========================================================================
// PERF-C-LOOP-003 — character-wise stdio as primary loop-body statement
// ===========================================================================

#[test]
fn char_io_primary_statement_fires_info_medium() {
    let units = common::load_units_fixture("PERF-C-LOOP-003__char-io-primary.json");
    let findings = analyze(&units);
    let loop003 = common::by_rule(&findings, LOOP_003_RULE_ID);
    assert_eq!(loop003.len(), 2, "braced fgetc + unbraced getchar fire");
    let callees: Vec<&str> = loop003
        .iter()
        .filter_map(|f| f.evidence.get("callee").map(String::as_str))
        .collect();
    assert_eq!(callees, ["fgetc", "getchar"]);
    for f in &loop003 {
        assert_eq!(f.severity, Severity::Info);
        assert_eq!(f.confidence, Confidence::Medium);
        assert_eq!(f.evidence.get("loop_depth").map(String::as_str), Some("1"));
        assert!(f.message.contains("buffered block reads"));
    }
}

#[test]
fn fread_and_non_primary_role_stay_silent() {
    let units = common::load_units_fixture("PERF-C-LOOP-003__char-io-primary.json");
    let findings = analyze(&units);
    assert!(
        findings
            .iter()
            .all(|f| f.evidence.get("callee").map(String::as_str) != Some("fread")),
        "buffered fread is the optimization target, not a finding"
    );
    let nested_line_34 = common::by_rule(&findings, LOOP_003_RULE_ID)
        .iter()
        .any(|f| f.span.start_line == 34);
    assert!(
        !nested_line_34,
        "nested-role fgetc (line 34) must stay silent — statement_role gate"
    );
}

// ===========================================================================
// PERF-C-CALL-001 — direct recursion
// ===========================================================================

#[test]
fn two_self_call_sites_yield_one_finding_at_earliest() {
    let units = common::load_units_fixture("PERF-C-CALL-001__direct-recursion.json");
    let findings = analyze(&units);
    let call001 = common::by_rule(&findings, CALL_001_RULE_ID);
    assert_eq!(call001.len(), 1, "one finding per function, not per site");
    let f = call001[0];
    assert_eq!(f.severity, Severity::Info);
    assert_eq!(f.confidence, Confidence::High);
    assert_eq!(
        f.span.start_line, 7,
        "anchors at the earliest self-call site"
    );
    assert_eq!(
        f.evidence.get("function").map(String::as_str),
        Some("walk_tree")
    );
    assert_eq!(
        f.evidence.get("self_call_sites").map(String::as_str),
        Some("2")
    );
    assert!(f.message.contains("2 site(s)"));
}

#[test]
fn non_recursive_and_indirect_only_stay_silent() {
    let units = common::load_units_fixture("PERF-C-CALL-001__direct-recursion.json");
    let findings = analyze(&units);
    assert_eq!(
        common::by_rule(&findings, CALL_001_RULE_ID).len(),
        1,
        "sum_pair (no self-call) and indirect_spin (indirect-only) stay silent"
    );
    assert!(
        findings
            .iter()
            .all(|f| f.rule_id != CALL_001_RULE_ID || f.span.start_line < 16),
        "no CALL-001 finding may anchor below walk_tree"
    );
}

#[test]
fn call_001_downgrade_hooks_fire_independently() {
    let units = common::load_units_fixture("PERF-C-CALL-001__downgrade-hooks.json");
    let findings = analyze(&units);
    let call001 = common::by_rule(&findings, CALL_001_RULE_ID);
    assert_eq!(
        call001.len(),
        2,
        "one downgraded finding per recursive function, never dropped"
    );

    // (a) error-region overlap: earliest self-call span (7,12)-(7,26)
    // intersects the parse_health error region (7,20)-(8,5).
    let err = call001
        .iter()
        .find(|f| f.error_node_overlap)
        .expect("error-region-overlapped finding");
    assert_eq!(err.rule_id, CALL_001_RULE_ID);
    assert_eq!(
        err.evidence.get("function").map(String::as_str),
        Some("drain_queue")
    );
    assert_eq!(
        err.confidence,
        Confidence::Medium,
        "error-region overlap caps confidence"
    );
    assert!(!err.macro_suspect, "hooks must fire independently");
    assert_eq!(err.span.start_line, 7, "earliest-site anchor preserved");
    assert_eq!(
        err.evidence.get("self_call_sites").map(String::as_str),
        Some("2")
    );

    // (b) macro origin: self-call event carries meta.macro_origin=true.
    let mac = call001
        .iter()
        .find(|f| f.macro_suspect)
        .expect("macro-origin finding");
    assert_eq!(mac.rule_id, CALL_001_RULE_ID);
    assert_eq!(
        mac.evidence.get("function").map(String::as_str),
        Some("macro_walk")
    );
    assert_eq!(
        mac.confidence,
        Confidence::Medium,
        "macro origin caps confidence"
    );
    assert!(!mac.error_node_overlap, "hooks must fire independently");
    assert_eq!(mac.span.start_line, 17, "earliest-site anchor preserved");
    assert_eq!(
        mac.evidence.get("self_call_sites").map(String::as_str),
        Some("2")
    );

    // Severity band is unchanged by either hook.
    for f in &call001 {
        assert_eq!(
            f.severity,
            Severity::Info,
            "severity band unchanged by hook"
        );
    }
}

// ===========================================================================
// Kitchen sink — all twelve rules from one synthetic unit set, total order
// ===========================================================================

/// Point-ish span helper (cols chosen so containment stays plausible).
fn sp(line: u32, sc: u32, ec: u32) -> Span {
    Span {
        start_line: line,
        start_col: sc,
        end_line: line,
        end_col: ec,
    }
}

fn meta(span: Span, depth: u32) -> EventMeta {
    EventMeta {
        span,
        in_loop_depth: depth,
        is_innermost: true,
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
    }
}

fn call(callee: &str, args: &[&str], span: Span, depth: u32, role: StatementRole) -> CallEvent {
    CallEvent {
        meta: meta(span, depth),
        callee_name: callee.to_string(),
        callee_kind: CalleeKind::Direct,
        args: args
            .iter()
            .map(|name| Shape {
                kind: ShapeKind::Identifier,
                name: Some((*name).to_string()),
                ..Shape::default()
            })
            .collect(),
        statement_role: role,
        conditional_guarded: false,
    }
}

fn loop_node(kind: LoopKind, depth: u32, span: Span) -> LoopNode {
    LoopNode {
        kind,
        depth,
        induction_vars: Vec::new(),
        span,
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
    }
}

/// Binary-op event with an identifier right operand (LOOP-004 kitchen input).
fn binop(op: &str, span: Span, divisor: &str) -> BinaryOpEvent {
    BinaryOpEvent {
        meta: meta(span, 0),
        op: op.to_string(),
        left: Shape::identifier("acc"),
        right: Shape::identifier(divisor),
    }
}

fn func(
    name: &str,
    path: &str,
    span: Span,
    calls: Vec<CallEvent>,
    loops: Vec<LoopNode>,
) -> FunctionUnit {
    FunctionUnit {
        record: FunctionRecord {
            id: format!("{path}#{name}#{}:{}", span.start_line, span.start_col),
            name: name.to_string(),
            path: path.to_string(),
            span,
            param_count: 0,
            param_types: Vec::new(),
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            max_loop_depth: loops.iter().map(|l| l.depth).max().unwrap_or(0),
            alloc_site_count: 0,
            length_lines: span.end_line - span.start_line + 1,
        },
        calls,
        assignments: Vec::new(),
        binary_ops: Vec::new(),
        comparisons: Vec::new(),
        access_chains: Vec::new(),
        loops,
    }
}

/// One unit, two functions: `kitchen` nests three loops and trips
/// ALLOC-001/002/003, STR-001/002/003, LOOP-001/002/003/004; `recurse`
/// trips CALL-001 and CALL-002.
fn kitchen_units() -> Vec<FileUnit> {
    let path = "src/kitchen.c";
    let mut kitchen_calls = vec![
        call("malloc", &["n"], sp(5, 17, 27), 3, StatementRole::Nested),
        call("free", &["old"], sp(6, 17, 26), 3, StatementRole::Nested),
        call(
            "strcat",
            &["buf", "part"],
            sp(7, 17, 33),
            3,
            StatementRole::Nested,
        ),
        call("strlen", &["buf"], sp(8, 17, 27), 3, StatementRole::Nested),
        call(
            "strcpy",
            &["dst", "src"],
            sp(9, 17, 34),
            3,
            StatementRole::Nested,
        ),
        call("fgetc", &["fp"], sp(10, 17, 24), 3, StatementRole::Primary),
        // ALLOC-003: additive growth — the `+` binop below sits INSIDE
        // this call span (cols 28–38 within 17–45).
        call(
            "realloc",
            &["buf", "size"],
            sp(11, 17, 45),
            3,
            StatementRole::Nested,
        ),
        // LOOP-002: unguarded expensive I/O inside the loop nest.
        call("printf", &["fmt"], sp(12, 17, 29), 3, StatementRole::Nested),
    ];
    // ALLOC-003 two-part proof requires the realloc SIZE arg (last) to be
    // a Binary shape — `realloc(buf, size + chunk)`.
    if let Some(realloc) = kitchen_calls
        .iter_mut()
        .find(|c| c.callee_name == "realloc")
        && let Some(size_arg) = realloc.args.last_mut()
    {
        size_arg.kind = ShapeKind::Binary;
    }
    let kitchen_binops = vec![
        // ALLOC-003 additivity proof: contained in the realloc call span.
        BinaryOpEvent {
            meta: meta(sp(11, 28, 38), 3),
            op: "+".to_string(),
            left: Shape::identifier("size"),
            right: Shape::identifier("chunk"),
        },
        // LOOP-004: variable-divisor division, innermost loop.
        binop("/", sp(13, 17, 24), "n"),
    ];
    let kitchen_loops = vec![
        loop_node(
            LoopKind::For,
            1,
            Span {
                start_line: 2,
                start_col: 5,
                end_line: 40,
                end_col: 6,
            },
        ),
        loop_node(
            LoopKind::For,
            2,
            Span {
                start_line: 3,
                start_col: 9,
                end_line: 39,
                end_col: 10,
            },
        ),
        loop_node(
            LoopKind::For,
            3,
            Span {
                start_line: 4,
                start_col: 13,
                end_line: 38,
                end_col: 14,
            },
        ),
    ];
    let recurse_calls = vec![
        call("recurse", &["a"], sp(43, 5, 15), 0, StatementRole::Primary),
        call("recurse", &["b"], sp(44, 5, 15), 0, StatementRole::Primary),
    ];
    vec![FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts::default(),
            function_count: 2,
            global_var_count: 0,
            include_counts: IncludeCounts::default(),
        },
        parse_health: ParseHealth::default(),
        includes: Vec::new(),
        global_vars: Vec::new(),
        structs: Vec::new(),
        macros: Vec::new(),
        functions: vec![
            {
                let mut kitchen = func(
                    "kitchen",
                    path,
                    Span {
                        start_line: 1,
                        start_col: 5,
                        end_line: 41,
                        end_col: 6,
                    },
                    kitchen_calls,
                    kitchen_loops,
                );
                kitchen.binary_ops = kitchen_binops;
                kitchen
            },
            {
                // CALL-002: complexity far above the threshold of 15.
                let mut recurse = func(
                    "recurse",
                    path,
                    Span {
                        start_line: 42,
                        start_col: 5,
                        end_line: 45,
                        end_col: 6,
                    },
                    recurse_calls,
                    Vec::new(),
                );
                recurse.record.cyclomatic_complexity = 20;
                recurse
            },
        ],
    }]
}

/// Lexicographic check of the report §4 total order between adjacent pairs:
/// severity desc → confidence desc → rule_id asc → path asc → line asc → col asc.
fn adjacently_in_total_order(a: &Finding, b: &Finding) -> bool {
    use std::cmp::Ordering::Equal;
    (
        b.severity.cmp(&a.severity),
        b.confidence.cmp(&a.confidence),
        a.rule_id.cmp(&b.rule_id),
        a.path.cmp(&b.path),
        a.span.start_line.cmp(&b.span.start_line),
        a.span.start_col.cmp(&b.span.start_col),
    ) <= (Equal, Equal, Equal, Equal, Equal, Equal)
}

#[test]
fn kitchen_sink_trips_all_registry_rules_in_total_order() {
    let all_rules: BTreeSet<&str> = [
        ALLOC_001_RULE_ID,
        ALLOC_002_RULE_ID,
        ALLOC_003_RULE_ID,
        CALL_001_RULE_ID,
        CALL_002_RULE_ID,
        LOOP_001_RULE_ID,
        LOOP_002_RULE_ID,
        LOOP_003_RULE_ID,
        LOOP_004_RULE_ID,
        STR_001_RULE_ID,
        STR_002_RULE_ID,
        STR_003_RULE_ID,
    ]
    .into_iter()
    .collect();

    let findings = analyze(&kitchen_units());
    let seen: BTreeSet<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert_eq!(
        seen, all_rules,
        "every registry rule must trip: {findings:?}"
    );

    for pair in findings.windows(2) {
        assert!(
            adjacently_in_total_order(&pair[0], &pair[1]),
            "total order violated between {:?} and {:?}",
            pair[0].finding_id(),
            pair[1].finding_id()
        );
    }

    // Spot-check the multi-finding families behave as designed here too.
    assert_eq!(
        common::by_rule(&findings, STR_003_RULE_ID).len(),
        2,
        "strcat + strcpy"
    );
    assert_eq!(
        common::by_rule(&findings, ALLOC_002_RULE_ID)[0]
            .evidence
            .get("alloc_cooccurrence")
            .map(String::as_str),
        Some("true"),
        "malloc + free share the kitchen function"
    );
    // Wave B spot-checks inside the same synthetic unit.
    let alloc003 = common::by_rule(&findings, ALLOC_003_RULE_ID);
    assert_eq!(alloc003.len(), 1, "only the additive realloc fires");
    assert_eq!(
        alloc003[0].evidence.get("growth").map(String::as_str),
        Some("additive")
    );
    let loop002 = common::by_rule(&findings, LOOP_002_RULE_ID);
    assert_eq!(loop002.len(), 1, "unguarded printf");
    assert_eq!(loop002[0].severity, Severity::Hot);
    let loop004 = common::by_rule(&findings, LOOP_004_RULE_ID);
    assert_eq!(loop004.len(), 1, "variable-divisor division");
    assert_eq!(
        loop004[0].evidence.get("divisor_kind").map(String::as_str),
        Some("identifier")
    );
    let call002 = common::by_rule(&findings, CALL_002_RULE_ID);
    assert_eq!(call002.len(), 1, "recurse complexity 20 ≥ 15");
    assert_eq!(
        call002[0].evidence.get("complexity").map(String::as_str),
        Some("20")
    );
}
