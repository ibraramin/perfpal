//! Registry mechanics tests (modules.md §3.3): deterministic duplicate
//! collapse and the report §4 total order — built from synthetic IR units in
//! Rust, no parser bound.

use perfpal::analyzer::finding::ALLOC_001_RULE_ID;
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
            is_innermost: true,
            resolver_tier: ResolverTier::Syntactic,
            macro_origin: false,
        },
        callee_name: "malloc".to_string(),
        callee_kind: CalleeKind::Direct,
        args: Vec::new(),
        statement_role: StatementRole::Nested,
        conditional_guarded: false,
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
            },
            calls: vec![call],
            assignments: Vec::new(),
            binary_ops: Vec::new(),
            comparisons: Vec::new(),
            access_chains: Vec::new(),
            loops: Vec::new(),
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
