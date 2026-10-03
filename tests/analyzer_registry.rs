//! Registry mechanics tests (modules.md §3.3): deterministic duplicate
//! collapse and the report §4 total order — built from synthetic IR units in
//! Rust, no parser bound.

use perfpal::analyzer::finding::{
    ALLOC_001_RULE_ID, ALLOC_002_RULE_ID, ARCH_003_RULE_ID, CALL_001_RULE_ID, LOOP_002_RULE_ID,
};
use perfpal::analyzer::{Confidence, Severity, analyze};
use perfpal::ir::{
    CallEvent, CalleeKind, EventMeta, FileRecord, FunctionRecord, IncludeCounts, LocCounts,
    ParseHealth, ResolverTier, Span, StatementRole,
};
use perfpal::scanner::{FileUnit, FunctionUnit};

/// A minimal unit with one function holding one malloc-in-loop call at
/// `path`, `line` (col 1), with configurable loop depth.
fn unit_with_malloc(path: &str, line: u32, loop_depth: u32) -> FileUnit {
    let span = Span {
        start_line: line,
        start_col: 1,
        end_line: line,
        end_col: 12,
    };
    let call = CallEvent {
        meta: EventMeta {
            span,
            in_loop_depth: loop_depth,
            // Scanner contract: never innermost outside any loop.
            is_innermost: loop_depth >= 1,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
        },
        callee_name: "malloc".to_string(),
        callee_kind: CalleeKind::Direct,
        callee_path: None,
        is_macro: false,
        args: Vec::new(),
        statement_role: StatementRole::Nested,
        conditional_guarded: false,
        semantic: None,
        receiver_root: None,
    };
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts::default(),
            function_count: 1,
            global_var_count: 0,
            include_counts: IncludeCounts::default(),
        },
        parse_health: ParseHealth::default(),
        includes: Vec::new(),
        global_vars: Vec::new(),
        structs: Vec::new(),
        macros: Vec::new(),
        functions: vec![FunctionUnit {
            record: FunctionRecord {
                id: format!("{path}#f#{line}:1"),
                name: "f".to_string(),
                path: path.to_string(),
                span,
                param_count: 0,
                param_types: Vec::new(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 0,
                max_loop_depth: loop_depth,
                alloc_site_count: 1,
                length_lines: 1,
                resolver_tier: ResolverTier::default(),
                is_async: false,
                is_module_scope: false,
            },
            calls: vec![call],
            assignments: Vec::new(),
            binary_ops: Vec::new(),
            comparisons: Vec::new(),
            access_chains: Vec::new(),
            loops: Vec::new(),
            awaits: Vec::new(),
        }],
    }
}

#[test]
fn identical_rule_path_line_col_collapses_to_first() {
    // Two units share path AND call span → same (rule_id, path, line, col).
    let units = vec![
        unit_with_malloc("src/a.c", 10, 1),
        unit_with_malloc("src/a.c", 10, 1),
    ];
    let findings = analyze(&units);
    assert_eq!(findings.len(), 1, "duplicate identity keys MUST collapse");
    assert_eq!(
        findings[0].finding_id(),
        format!("{ALLOC_001_RULE_ID}:src/a.c:10:1")
    );
}

/// A `"python"`-language unit is DROPPED by the language gate: the exact same
/// malloc-in-loop that fires PERF-C-ALLOC-001 for a C unit must produce ZERO
/// findings (plan `python-backend` W2, FIX 1). W3 adds RuleLanguage::Python.
#[test]
fn python_language_units_are_not_scored_by_c_rules() {
    let mut unit = unit_with_malloc("src/app.py", 10, 1);
    unit.file.language = "python".to_string();
    let findings = analyze(&[unit]);
    assert!(
        findings.is_empty(),
        "python units must not be scored by C/Rust heuristics: {findings:?}"
    );
}

#[test]
fn findings_emit_in_report_total_order() {
    // Same severity/confidence/rule ⇒ ordered by path then line.
    let units = vec![
        unit_with_malloc("src/b.c", 20, 1),
        unit_with_malloc("src/a.c", 30, 2),
        unit_with_malloc("src/a.c", 10, 1),
    ];
    let findings = analyze(&units);
    assert_eq!(findings.len(), 3);
    let order: Vec<String> = findings
        .iter()
        .map(|f| format!("{}:{}", f.path, f.span.start_line))
        .collect();
    assert_eq!(
        order,
        ["src/a.c:10", "src/a.c:30", "src/b.c:20"],
        "total order: rule_id → path → start_line (severity/confidence tie here)"
    );
    for f in &findings {
        assert_eq!(f.severity, Severity::Hot);
        assert_eq!(f.confidence, Confidence::High);
    }
}

// ---------------------------------------------------------------------------
// Total-order direction regression (output-contract §4)
// ---------------------------------------------------------------------------

fn call_event(callee: &str, line: u32, loop_depth: u32) -> CallEvent {
    CallEvent {
        meta: EventMeta {
            span: Span {
                start_line: line,
                start_col: 5,
                end_line: line,
                end_col: 20,
            },
            in_loop_depth: loop_depth,
            // Scanner contract: never innermost outside any loop.
            is_innermost: loop_depth >= 1,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
        },
        callee_name: callee.to_string(),
        callee_kind: CalleeKind::Direct,
        callee_path: None,
        is_macro: false,
        args: Vec::new(),
        statement_role: StatementRole::Nested,
        conditional_guarded: false,
        semantic: None,
        receiver_root: None,
    }
}

/// One unit with a single function `name` carrying `calls`; the function
/// span extends past the calls so containment stays plausible.
fn unit_with_fn(path: &str, func_name: &str, calls: Vec<CallEvent>) -> FileUnit {
    let last_line = calls
        .iter()
        .map(|c| c.meta.span.start_line)
        .max()
        .unwrap_or(1);
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts::default(),
            function_count: 1,
            global_var_count: 0,
            include_counts: IncludeCounts::default(),
        },
        parse_health: ParseHealth::default(),
        includes: Vec::new(),
        global_vars: Vec::new(),
        structs: Vec::new(),
        macros: Vec::new(),
        functions: vec![FunctionUnit {
            record: FunctionRecord {
                id: format!("{path}#{func_name}#1:1"),
                name: func_name.to_string(),
                path: path.to_string(),
                span: Span {
                    start_line: 1,
                    start_col: 1,
                    end_line: last_line + 10,
                    end_col: 1,
                },
                param_count: 0,
                param_types: Vec::new(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 0,
                max_loop_depth: calls
                    .iter()
                    .map(|c| c.meta.in_loop_depth)
                    .max()
                    .unwrap_or(0),
                alloc_site_count: 0,
                length_lines: last_line + 10,
                resolver_tier: ResolverTier::default(),
                is_async: false,
                is_module_scope: false,
            },
            calls,
            assignments: Vec::new(),
            binary_ops: Vec::new(),
            comparisons: Vec::new(),
            access_chains: Vec::new(),
            loops: Vec::new(),
            awaits: Vec::new(),
        }],
    }
}

#[test]
fn total_order_is_hot_to_info_and_high_to_low_within_group() {
    // Four findings whose (severity, confidence) pairs are chosen so the
    // CONTRACT order differs from the historically-buggy inverted order.
    // Ranks per output-contract §4, spelled out here independently of both
    // the enum ordering and the production comparator:
    //   hot/medium < warn/high < info/high < info/low
    // The inverted comparator would emit exactly the reverse sequence.
    //
    // Producers:
    // - LOOP-002 printf-in-loop          → Hot / MEDIUM
    // - ALLOC-002 free-in-loop           → Warn / HIGH (no alloc pairing)
    // - CALL-001 direct recursion        → Info / HIGH
    // - ARCH-003 cross-file name match   → Info / LOW
    let hot_medium = unit_with_fn("a_hot.c", "log_spam", vec![call_event("printf", 3, 1)]);
    let warn_high = unit_with_fn("b_warn.c", "releaser", vec![call_event("free", 4, 1)]);
    let info_high = unit_with_fn("c_info_high.c", "walk", vec![call_event("walk", 5, 0)]);
    let info_low = unit_with_fn("d_info_low.c", "caller", vec![call_event("util_fn", 6, 0)]);
    // ARCH-003 target: util_fn defined ONLY in e_util.c (not file-local).
    let target = unit_with_fn("e_util.c", "util_fn", vec![]);

    let units = vec![hot_medium, warn_high, info_high, info_low, target];
    let findings = analyze(&units);
    assert_eq!(findings.len(), 4, "exactly one finding per producer");

    let spec_rank = |f: &perfpal::analyzer::Finding| -> (u8, u8) {
        let sev = match f.severity {
            Severity::Hot => 0,
            Severity::Warn => 1,
            Severity::Info => 2,
        };
        let con = match f.confidence {
            Confidence::High => 0,
            Confidence::Medium => 1,
            Confidence::Low => 2,
        };
        (sev, con)
    };

    // Exact expected sequence under the CONTRACT (spec-derived ranks).
    let expected: [(&str, &str); 4] = [
        (LOOP_002_RULE_ID, "a_hot.c"),
        (ALLOC_002_RULE_ID, "b_warn.c"),
        (CALL_001_RULE_ID, "c_info_high.c"),
        (ARCH_003_RULE_ID, "d_info_low.c"),
    ];
    for (finding, (rule, path)) in findings.iter().zip(expected) {
        assert_eq!(finding.rule_id, rule);
        assert_eq!(finding.path, path);
    }

    // Adjacent-pair check against hand-written rank tuples (NOT the
    // production comparator): ranks must be non-decreasing contract-wise.
    let ranks: Vec<(u8, u8)> = findings.iter().map(spec_rank).collect();
    for pair in ranks.windows(2) {
        assert!(
            pair[0] <= pair[1],
            "severity/confidence ranks must ascend contract-wise: {ranks:?}"
        );
    }
    assert_eq!(
        ranks[0],
        (0, 1),
        "first finding must be Hot/MEDIUM — inverted order would start Info"
    );
    assert_eq!(
        ranks[3],
        (2, 2),
        "last finding must be Info/LOW — inverted order would end Hot"
    );
}
