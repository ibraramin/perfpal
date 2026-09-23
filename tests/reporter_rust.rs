//! Rust reporter integration tests (plan `rust-backend` R3 workstream D1):
//! Appendix A catalog coverage for every `PERF-RS-*` rule id and the
//! language-neutral architecture prose.
//!
//! Regression lock for the D1 blocker: before this workstream, any Rust
//! finding reaching Appendix A hit `unwrap_or_else(|| panic!("uncataloged
//! rule_id …"))` because `HEURISTIC_CATALOG_V1` held only the 23 `PERF-C-*`
//! entries. A `report.md` containing a Rust finding therefore PANICKED.
//!
//! Documents are built programmatically (direct `MetricsDocument` literals);
//! the reporter is a pure function of `(doc, emitted_charts)`.

use std::collections::BTreeMap;

use perfpal::analyzer::finding::{
    RS_ALLOC_001_RULE_ID, RS_ALLOC_002_RULE_ID, RS_ALLOC_003_RULE_ID, RS_ARCH_001_RULE_ID,
    RS_ARCH_002_RULE_ID, RS_ARCH_003_RULE_ID, RS_CALL_001_RULE_ID, RS_CALL_002_RULE_ID,
    RS_LOOP_001_RULE_ID, RS_LOOP_002_RULE_ID, RS_LOOP_003_RULE_ID, RS_LOOP_004_RULE_ID,
    RS_LOOP_005_RULE_ID, RS_LOOP_006_RULE_ID, RS_LOOP_007_RULE_ID, RS_LOOP_008_RULE_ID,
    RS_MEM_001_RULE_ID, RS_MEM_002_RULE_ID, RS_STR_001_RULE_ID, RS_STR_002_RULE_ID,
    RS_STR_003_RULE_ID, RS_TYPE_001_RULE_ID, RS_TYPE_002_RULE_ID,
};
use perfpal::analyzer::{Confidence, Severity};
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::render_report;
use perfpal::schema::{
    FileEntry, FindingEntry, MetricsDocument, ParseHealthEntry, ParseHealthSummary, ScanInfo,
    StatsSection, ToolInfo,
};

/// All 23 Rust catalog rule ids, ascending — the set the analyzer registry
/// can emit for Rust files.
const ALL_RS_RULE_IDS: [&str; 23] = [
    RS_ALLOC_001_RULE_ID,
    RS_ALLOC_002_RULE_ID,
    RS_ALLOC_003_RULE_ID,
    RS_ARCH_001_RULE_ID,
    RS_ARCH_002_RULE_ID,
    RS_ARCH_003_RULE_ID,
    RS_CALL_001_RULE_ID,
    RS_CALL_002_RULE_ID,
    RS_LOOP_001_RULE_ID,
    RS_LOOP_002_RULE_ID,
    RS_LOOP_003_RULE_ID,
    RS_LOOP_004_RULE_ID,
    RS_LOOP_005_RULE_ID,
    RS_LOOP_006_RULE_ID,
    RS_LOOP_007_RULE_ID,
    RS_LOOP_008_RULE_ID,
    RS_MEM_001_RULE_ID,
    RS_MEM_002_RULE_ID,
    RS_STR_001_RULE_ID,
    RS_STR_002_RULE_ID,
    RS_STR_003_RULE_ID,
    RS_TYPE_001_RULE_ID,
    RS_TYPE_002_RULE_ID,
];

// ---------------------------------------------------------------------------
// Local document builders
// ---------------------------------------------------------------------------

fn loc(code: u32) -> LocCounts {
    LocCounts {
        code,
        comment: code / 10,
        blank: code / 20,
    }
}

fn ok_health() -> ParseHealthEntry {
    ParseHealthEntry {
        status: ParseStatus::Ok,
        error_regions: None,
        error_node_count: 0,
        resolver: None,
    }
}

fn rust_file_entry(path: &str, code: u32) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "rust".to_string(),
        loc: loc(code),
        function_count: 1,
        global_var_count: 0,
        include_counts: IncludeCounts {
            quoted: 0,
            angle: 0,
        },
        parse_health: ok_health(),
    }
}

fn rs_finding(rule_id: &str, severity: Severity, confidence: Confidence) -> FindingEntry {
    let path = "src/lib.rs";
    FindingEntry {
        finding_id: format!("{rule_id}:{path}:7:9"),
        rule_id: rule_id.to_string(),
        title: format!("Title for {rule_id}"),
        path: path.to_string(),
        symbol: None,
        span: Span::point(7, 9),
        confidence,
        severity,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect: false,
        error_node_overlap: false,
        threshold: None,
        message: format!("Message for {rule_id}"),
        evidence: BTreeMap::new(),
    }
}

/// Minimal well-formed Rust-only document: one `.rs` file, the given Rust
/// findings, all optional sections populated.
fn rust_doc(findings: Vec<FindingEntry>) -> MetricsDocument {
    MetricsDocument {
        schema_version: "1.0".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: Some(12),
            languages: vec!["rust".to_string()],
            file_count: 1,
            loc_total: Some(loc(10)),
        },
        parse_health_summary: Some(ParseHealthSummary {
            ok: 1,
            partial: 0,
            failed: 0,
        }),
        files: vec![rust_file_entry("src/lib.rs", 10)],
        stats: StatsSection {
            functions: vec![],
            distributions: None,
        },
        findings,
        graphs: None,
        components: None,
    }
}

/// Slice out `[start_heading, next_marker)` from a rendered report.
fn section<'a>(report: &'a str, start: &str, end: &str) -> &'a str {
    let start_pos = report
        .find(start)
        .unwrap_or_else(|| panic!("{start} missing:\n{report}"));
    let rest = &report[start_pos..];
    let after_start = start.len();
    match rest[after_start..].find(end) {
        Some(end_rel) => &rest[..after_start + end_rel],
        None => rest,
    }
}

// ---------------------------------------------------------------------------
// 1. Every Rust rule id renders without panic and is cataloged in Appendix A
// ---------------------------------------------------------------------------

#[test]
fn all_rust_rule_ids_render_without_panic_and_reach_appendix_a() {
    let findings = ALL_RS_RULE_IDS
        .iter()
        .map(|id| rs_finding(id, Severity::Warn, Confidence::Medium))
        .collect();
    // Before D1 this call panicked on the first Rust rule id.
    let report = render_report(&rust_doc(findings), &[]);

    let appendix = section(&report, "## Appendix A.", "## Appendix B.");
    for id in ALL_RS_RULE_IDS {
        // Rule ids are escape_md'd (`-` → `\-`).
        let escaped = id.replace('-', "\\-");
        assert!(
            appendix.contains(&format!("`{escaped}`")),
            "{id} missing from Appendix A:\n{appendix}"
        );
    }
    let data_rows = appendix
        .lines()
        .filter(|l| l.starts_with("| `PERF"))
        .count();
    assert_eq!(data_rows, 23, "Appendix A must list all 23 Rust ids");
}

// ---------------------------------------------------------------------------
// 2. Appendix A descriptions reflect the implemented detection / reframing
// ---------------------------------------------------------------------------

#[test]
fn rust_appendix_descriptions_are_read_code_accurate() {
    let findings = vec![
        rs_finding(RS_ALLOC_001_RULE_ID, Severity::Hot, Confidence::High),
        rs_finding(RS_ALLOC_003_RULE_ID, Severity::Hot, Confidence::Medium),
        rs_finding(RS_STR_003_RULE_ID, Severity::Warn, Confidence::High),
        rs_finding(RS_MEM_001_RULE_ID, Severity::Warn, Confidence::Medium),
        rs_finding(RS_TYPE_002_RULE_ID, Severity::Info, Confidence::Low),
    ];
    let report = render_report(&rust_doc(findings), &[]);
    let appendix = section(&report, "## Appendix A.", "## Appendix B.");

    // Titles (parentheses are escape_md'd, so match the prefix only).
    assert!(appendix.contains("Heap allocation inside loop"));
    assert!(appendix.contains("Growable\\-buffer additive growth in loop"));
    assert!(appendix.contains("Unbounded unsafe memory operation"));
    assert!(appendix.contains("size\\_of passed as an element count"));
    assert!(appendix.contains("Struct field ordering"));

    // Honest reframing clauses are present where the Rust rule remaps C intent.
    assert!(appendix.contains("REFRAMED for Rust"));
    assert!(appendix.contains("realloc"));
    assert!(appendix.contains("unsafe"));
    assert!(appendix.contains("element count"));
    assert!(appendix.contains("reorder"));
}

// ---------------------------------------------------------------------------
// 3. Architecture prose reads correctly for a Rust-only scan
// ---------------------------------------------------------------------------

#[test]
fn architecture_prose_is_language_neutral_for_rust_only_scan() {
    let findings = vec![rs_finding(
        RS_ALLOC_001_RULE_ID,
        Severity::Hot,
        Confidence::High,
    )];
    let charts = vec!["include-graph".to_string()];
    let report = render_report(&rust_doc(findings), &charts);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");

    // Both languages' edge kinds are named…
    assert!(
        arch.contains("Rust `mod`/`use` module paths"),
        "Rust module edges not described:\n{arch}"
    );
    assert!(
        arch.contains("C `#include` directives"),
        "C include edges not described:\n{arch}"
    );
    // …the chart caption is language-neutral too…
    assert!(
        arch.contains("[Dependency graph — internal #include (C) and mod/use (Rust) dependencies between scanned files.](charts/include-graph.svg)"),
        "chart caption not language-neutral:\n{arch}"
    );
    // …and the retired C-only wording is gone.
    assert!(
        !arch.contains("The include graph reflects resolved internal"),
        "stale C-only architecture prose:\n{arch}"
    );
    assert!(
        !arch.contains("external (system) headers are excluded"),
        "stale C-only system-header wording:\n{arch}"
    );
}
