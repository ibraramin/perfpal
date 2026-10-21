//! Reporter render-contract tests (output-contract §5; P4 Wave 3b).
//!
//! Documents are constructed programmatically (direct `MetricsDocument`
//! literals via local helpers) — no scanner/analyzer run needed; the
//! reporter is a pure function of `(doc, emitted_charts)`.

use std::collections::BTreeMap;

use perfpal::analyzer::{Confidence, Severity};
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::{render_findings_detail, render_report};
use perfpal::schema::{
    ComponentEntry, ComponentMetrics, FileEntry, FindingEntry, MetricsDocument, ParseHealthEntry,
    ParseHealthSummary, ScanInfo, StatsSection, ToolInfo,
};

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

fn file_entry(path: &str, code: u32) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "c".to_string(),
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

fn finding(rule_id: &str, severity: Severity, confidence: Confidence, path: &str) -> FindingEntry {
    finding_full(
        rule_id,
        severity,
        confidence,
        path,
        None,
        false,
        false,
        BTreeMap::new(),
    )
}

#[allow(clippy::too_many_arguments)]
fn finding_full(
    rule_id: &str,
    severity: Severity,
    confidence: Confidence,
    path: &str,
    symbol: Option<&str>,
    macro_suspect: bool,
    error_node_overlap: bool,
    evidence: BTreeMap<String, String>,
) -> FindingEntry {
    let span = Span::point(7, 9);
    FindingEntry {
        finding_id: format!("{rule_id}:{path}:7:9"),
        rule_id: rule_id.to_string(),
        title: format!("Title for {rule_id}"),
        path: path.to_string(),
        symbol: symbol.map(str::to_string),
        span,
        confidence,
        severity,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect,
        error_node_overlap,
        threshold: None,
        message: format!("Message for {rule_id}"),
        evidence,
    }
}

/// Minimal well-formed document: one file, one hot finding, all optional
/// sections populated.
fn base_doc() -> MetricsDocument {
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
            languages: vec!["c".to_string()],
            file_count: 1,
            loc_total: Some(loc(10)),
        },
        parse_health_summary: Some(ParseHealthSummary {
            ok: 1,
            partial: 0,
            failed: 0,
        }),
        files: vec![file_entry("a.c", 10)],
        stats: StatsSection {
            functions: vec![],
            distributions: None,
        },
        findings: vec![finding(
            "PERF-C-ALLOC-001",
            Severity::Hot,
            Confidence::High,
            "a.c",
        )],
        graphs: None,
        components: None,
    }
}

// ---------------------------------------------------------------------------
// 1. Determinism: double-render byte-equality modulo provenance
// ---------------------------------------------------------------------------

#[test]
fn double_render_is_byte_equal_modulo_provenance() {
    let mut doc_a = base_doc();
    doc_a.scan.started_at = "2026-08-24T00:00:00Z".to_string();
    doc_a.scan.duration_ms = Some(12);
    let mut doc_b = base_doc();
    doc_b.scan.started_at = "2027-01-02T03:04:05Z".to_string();
    doc_b.scan.duration_ms = Some(987);

    // Provenance values are escape_md'd in the output (RFC3339 hyphens become
    // `\-`), so mask the ESCAPED forms before comparing.
    let mask = |text: &str, doc: &MetricsDocument| -> String {
        let escaped_started = doc.scan.started_at.replace('-', "\\-");
        text.replace(&escaped_started, "<STARTED>").replace(
            &format!("duration {} ms", doc.scan.duration_ms.unwrap()),
            "duration <DUR> ms",
        )
    };

    let charts = vec!["include-graph".to_string()];
    assert_eq!(
        mask(&render_report(&doc_a, &charts), &doc_a),
        mask(&render_report(&doc_b, &charts), &doc_b),
    );
}

// ---------------------------------------------------------------------------
// 2. Section presence + order
// ---------------------------------------------------------------------------

#[test]
fn sections_present_in_fixed_order() {
    let report = render_report(&base_doc(), &[]);
    let headings = [
        "# Perfpal Report",
        "## 1. Executive summary",
        "## 2. Scan provenance & parse health",
        "## 3. Codebase statistics",
        "## 4. Findings",
        "## 5. Architecture",
        "## Appendix A. Heuristic reference",
        "## Appendix B. How to read this report",
        "## Appendix C. All findings (full detail)",
    ];
    let mut cursor = 0;
    for heading in headings {
        let pos = report[cursor..]
            .find(heading)
            .unwrap_or_else(|| panic!("{heading} missing or out of order:\n{report}"))
            + cursor;
        cursor = pos + heading.len();
    }
    // Encoding contract: LF newlines, trailing newline.
    assert!(report.ends_with('\n'));
    assert!(!report.contains("\r"));
    // C-split: report.md carries zero per-finding rows — Appendix C is a
    // link stub into the findings-detail.md artifact.
    assert!(
        !report.contains("#### "),
        "report.md must not render per-finding detail rows"
    );
    assert!(
        report.contains("[findings-detail.md](findings-detail.md)"),
        "Appendix C stub must link the detail artifact"
    );
}

// ---------------------------------------------------------------------------
// 2b. Per-language totals live in §3 (output-contract §5), NOT §2
// ---------------------------------------------------------------------------

#[test]
fn per_language_totals_render_in_codebase_statistics_not_provenance() {
    let report = render_report(&base_doc(), &[]);
    let sec2 = section(
        &report,
        "## 2. Scan provenance & parse health",
        "## 3. Codebase statistics",
    );
    let sec3 = section(&report, "## 3. Codebase statistics", "## 4. Findings");

    assert!(
        !sec2.contains("- Languages:"),
        "§2 must drop the languages clause:\n{sec2}"
    );
    assert!(
        !sec2.contains("- Total LOC:"),
        "§2 must drop the loc_total clause:\n{sec2}"
    );
    assert!(
        sec3.contains("- Languages: c"),
        "§3 carries languages:\n{sec3}"
    );
    assert!(
        sec3.contains("- Total LOC: 10 code / 1 comment / 0 blank"),
        "§3 carries loc_total:\n{sec3}"
    );
    // Totals LEAD the section, before the top-files table.
    let langs = sec3.find("- Languages:").expect("languages in §3");
    let top = sec3
        .find("Top files by code LOC:")
        .expect("top files in §3");
    assert!(
        langs < top,
        "per-language totals must precede the top-files table"
    );
}

// ---------------------------------------------------------------------------
// 3. Findings grouping, citation format, conditional clauses
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 3. Findings §4 is group-and-fold: ONE row per firing rule in the pinned
// group order (severity-rank → count-desc → rule_id-asc); full detail lives
// uncapped in Appendix C.
// ---------------------------------------------------------------------------

#[test]
fn findings_section4_groups_by_rule_in_pinned_group_order() {
    let mut doc = base_doc();
    doc.findings = vec![
        // Document order deliberately scrambles groups and members — §4
        // must still group by rule in severity-rank → count-desc →
        // rule_id-asc order, with the exemplar first in finding_id order.
        finding("PERF-C-CALL-001", Severity::Info, Confidence::High, "z.c"),
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::High, "b.c"),
        finding("PERF-C-CALL-002", Severity::Warn, Confidence::High, "m.c"),
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::Medium, "a.c"),
        finding("PERF-C-STR-002", Severity::Warn, Confidence::Low, "n.c"),
        finding("PERF-C-CALL-001", Severity::Info, Confidence::Low, "y.c"),
        finding("PERF-C-CALL-001", Severity::Info, Confidence::Low, "x.c"),
    ];
    let report = render_report(&doc, &[]);
    let sec4 = section(&report, "## 4. Findings", "## 5. Architecture");

    // Pinned order: hot groups first (LOOP-002 ×2 before ALLOC-less single),
    // then warn (CALL-002 before STR-002 by rule_id tie-break), then info
    // (CALL-001 ×3 last despite the highest count — severity-rank dominates).
    let order: Vec<usize> = [
        "PERF-C-LOOP-002",
        "PERF-C-CALL-002",
        "PERF-C-STR-002",
        "PERF-C-CALL-001",
    ]
    .iter()
    .map(|rule| {
        // Rule cells are escape_md'd (`-`→`\-`).
        let escaped = rule.replace('-', "\\-");
        sec4.find(&format!("| `{escaped}` |"))
            .unwrap_or_else(|| panic!("{rule} group row missing:\n{sec4}"))
    })
    .collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(order, sorted, "§4 group rows out of order:\n{sec4}");

    // Group counts: LOOP-002 ×2 (2 hot), CALL-002 ×1 (1 warn),
    // STR-002 ×1 (1 warn), CALL-001 ×3 (3 info).
    let sec4_plain = sec4.replace('\\', "");
    assert!(sec4_plain.contains("| `PERF-C-LOOP-002` | 2 | 2 | 0 | 0 |"));
    assert!(sec4_plain.contains("| `PERF-C-CALL-002` | 1 | 0 | 1 | 0 |"));
    assert!(sec4_plain.contains("| `PERF-C-CALL-001` | 3 | 0 | 0 | 3 |"));

    // §4 carries NO per-finding detail headers — those live in the
    // findings-detail.md artifact.
    assert!(
        !sec4.contains("#### "),
        "§4 must not render per-finding detail:\n{sec4}"
    );

    // Honest header: 4 rules, 7 of 7 total.
    assert!(
        sec4.contains("4 rules: 7 of 7 total findings (2 hot, 2 warn, 3 info)."),
        "honest-counts header missing or wrong:\n{sec4}"
    );
}

#[test]
fn group_exemplar_is_first_finding_in_finding_id_order() {
    let mut doc = base_doc();
    doc.findings = vec![
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::High, "b.c"),
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::Medium, "a.c"),
    ];
    let report = render_report(&doc, &[]);
    let sec4 = section(&report, "## 4. Findings", "## 5. Architecture");
    // `a.c` sorts before `b.c`: the exemplar must be the a.c finding even
    // though document order lists b.c first.
    let sec4_plain = sec4.replace('\\', "");
    assert!(
        sec4_plain.contains("(`PERF-C-LOOP-002:a.c:7:9`)"),
        "exemplar must be the first finding in finding_id order:\n{sec4}"
    );
    assert!(
        !sec4_plain.contains("(`PERF-C-LOOP-002:b.c:7:9`)"),
        "§4 must cite only the exemplar, not every member:\n{sec4}"
    );
}

#[test]
fn group_exemplar_location_renders_unambiguous_path_line_col() {
    // Vision-flag lock: `src/main.c:34:21` is line 34, col 21 (the
    // `rule:path:line:col` finding identity — col 21 < line 34 is normal,
    // a column is not an end line), NOT an inverted range. The §4 exemplar
    // cell must therefore carry the unambiguous three-part `path:line:col`
    // form, matching the HTML row location cell.
    let mut doc = base_doc();
    doc.findings = vec![FindingEntry {
        finding_id: "PERF-C-ALLOC-001:src/main.c:34:21".to_string(),
        rule_id: "PERF-C-ALLOC-001".to_string(),
        title: "Title for PERF-C-ALLOC-001".to_string(),
        path: "src/main.c".to_string(),
        symbol: None,
        span: Span::point(34, 21),
        confidence: Confidence::High,
        severity: Severity::Hot,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect: false,
        error_node_overlap: false,
        threshold: None,
        message: "Message for PERF-C-ALLOC-001".to_string(),
        evidence: BTreeMap::new(),
    }];
    let report = render_report(&doc, &[]);
    let sec4 = section(&report, "## 4. Findings", "## 5. Architecture");
    let sec4_plain = sec4.replace('\\', "");
    assert!(
        sec4_plain.contains("— `src/main.c:34:21`"),
        "exemplar location must render unambiguous path:line:col:\n{sec4}"
    );
}

#[test]
fn group_counts_sum_to_total_and_match_appendix_c() {
    let mut doc = base_doc();
    doc.findings = vec![
        finding("PERF-C-CALL-001", Severity::Info, Confidence::High, "z.c"),
        finding("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding("PERF-C-STR-002", Severity::Warn, Confidence::High, "m.c"),
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::Medium, "b.c"),
        finding("PERF-C-CALL-002", Severity::Warn, Confidence::Low, "n.c"),
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::Low, "c.c"),
    ];
    let report = render_report(&doc, &[]);
    let sec4 = section(&report, "## 4. Findings", "## 5. Architecture");
    let sec4_plain = sec4.replace('\\', "");

    // Per-row counts sum to the honest total (5 rules, 6 findings).
    let mut row_sum = 0;
    for line in sec4_plain.lines() {
        if line.starts_with("| `PERF") {
            let count: usize = line
                .split('|')
                .nth(2)
                .expect("group row shape")
                .trim()
                .parse()
                .expect("group count parses");
            row_sum += count;
        }
    }
    assert_eq!(row_sum, 6, "group counts must sum to total:\n{sec4}");
    assert!(sec4.contains("5 rules: 6 of 6 total findings"));

    // C-split: report.md Appendix C is a link stub (zero rows); the uncapped
    // per-finding detail lives in findings-detail.md — one #### per finding.
    let appendix_c = section(&report, "## Appendix C.", "## Appendix B.");
    assert_eq!(
        appendix_c.matches("#### ").count(),
        0,
        "Appendix C stub must carry zero detail rows:\n{appendix_c}"
    );
    assert!(appendix_c.contains("[findings-detail.md](findings-detail.md)"));
    let detail = render_findings_detail(&doc);
    assert_eq!(
        detail.matches("#### ").count(),
        6,
        "findings-detail.md must render every finding:\n{detail}"
    );
    assert!(detail.contains("6 of 6 total findings."));
    for id in [
        "PERF-C-ALLOC-001:a.c:7:9",
        "PERF-C-LOOP-002:b.c:7:9",
        "PERF-C-LOOP-002:c.c:7:9",
        "PERF-C-STR-002:m.c:7:9",
        "PERF-C-CALL-002:n.c:7:9",
        "PERF-C-CALL-001:z.c:7:9",
    ] {
        let escaped = id.replace('-', "\\-").replace('.', "\\.");
        assert!(
            detail.contains(&format!("(`{escaped}`)")),
            "{id} missing from findings-detail.md:\n{detail}"
        );
    }
}

#[test]
fn grouped_output_is_deterministic_regardless_of_input_order() {
    // Same findings, shuffled document order ⇒ identical §4 + Appendix C.
    let findings_a = vec![
        finding("PERF-C-CALL-001", Severity::Info, Confidence::High, "z.c"),
        finding("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding("PERF-C-STR-002", Severity::Warn, Confidence::High, "m.c"),
    ];
    let mut findings_b = findings_a.clone();
    findings_b.reverse();
    // FindingEntry must be Clone for this — build via two docs instead.
    let mut doc_a = base_doc();
    doc_a.findings = findings_a;
    let mut doc_b = base_doc();
    doc_b.findings = findings_b;
    let report_a = render_report(&doc_a, &[]);
    let report_b = render_report(&doc_b, &[]);
    let sec4_a = section(&report_a, "## 4. Findings", "## 5. Architecture");
    let sec4_b = section(&report_b, "## 4. Findings", "## 5. Architecture");
    assert_eq!(
        sec4_a, sec4_b,
        "§4 must be byte-identical regardless of input order"
    );
    // C-split: Appendix C is a link stub (identical — no rows to order);
    // the findings-detail.md artifact preserves the legacy document-order
    // total order, so it legitimately differs — but the SORTED citation
    // multiset must agree.
    let app_a = section(&report_a, "## Appendix C.", "## Appendix B.");
    let app_b = section(&report_b, "## Appendix C.", "## Appendix B.");
    assert_eq!(app_a, app_b, "Appendix C stub must be order-independent");
    let detail_a = render_findings_detail(&doc_a);
    let detail_b = render_findings_detail(&doc_b);
    let mut cites_a: Vec<&str> = detail_a
        .match_indices("(`PERF")
        .map(|(i, _)| &detail_a[i..i + 32])
        .collect();
    let mut cites_b: Vec<&str> = detail_b
        .match_indices("(`PERF")
        .map(|(i, _)| &detail_b[i..i + 32])
        .collect();
    cites_a.sort_unstable();
    cites_b.sort_unstable();
    assert_eq!(cites_a, cites_b, "findings-detail.md citation sets differ");
}

#[test]
fn empty_findings_render_no_findings_in_section4_and_appendix_c() {
    let mut doc = base_doc();
    doc.findings = vec![];
    let report = render_report(&doc, &[]);
    let findings_section = section(&report, "## 4. Findings", "## 5. Architecture");
    assert_eq!(findings_section.trim(), "## 4. Findings\n\nNo findings.");
    let appendix_c = section(&report, "## Appendix C.", "## Appendix B.");
    assert!(
        appendix_c.contains("No findings."),
        "Appendix C must handle the empty scan:\n{appendix_c}"
    );
    let detail = render_findings_detail(&doc);
    assert!(
        detail.contains("No findings."),
        "findings-detail.md must handle the empty scan:\n{detail}"
    );
}

#[test]
fn flags_and_evidence_clauses_appear_only_when_applicable() {
    let mut doc = base_doc();
    let mut evidence = BTreeMap::new();
    evidence.insert("loop_depth".to_string(), "3".to_string());
    doc.findings = vec![
        finding_full(
            "PERF-C-LOOP-001",
            Severity::Warn,
            Confidence::Medium,
            "flagged.c",
            Some("worker"),
            true,
            true,
            evidence,
        ),
        finding(
            "PERF-C-CALL-001",
            Severity::Info,
            Confidence::High,
            "clean.c",
        ),
    ];
    // C-split: per-finding clauses live in findings-detail.md, not report.md.
    let detail = render_findings_detail(&doc);

    let flagged_section = &detail[detail.find("#### ").unwrap()..];
    assert!(flagged_section.contains("- Flags: macro_suspect, error_node_overlap"));
    // Evidence keys are escape_md'd too (`_` → `\_`).
    assert!(flagged_section.contains(r"- Evidence: loop\_depth=3"));
    assert!(flagged_section.contains(r"- Location: `flagged\.c:7` (worker)"));

    let clean_start = flagged_section
        .find(r"clean\.c")
        .expect("second finding present");
    let clean_section = &flagged_section[clean_start..];
    assert!(!clean_section.contains("- Flags:"));
    assert!(!clean_section.contains("- Evidence:"));
    assert!(
        !clean_section.contains("(symbol)"),
        "no phantom symbol clause"
    );
}

#[test]
fn empty_findings_render_no_findings_line() {
    let mut doc = base_doc();
    doc.findings = vec![];
    let report = render_report(&doc, &[]);
    let findings_section = section(&report, "## 4. Findings", "## 5. Architecture");
    assert_eq!(findings_section.trim(), "## 4. Findings\n\nNo findings.");
}

// ---------------------------------------------------------------------------
// 4. Trivial-scan well-formedness (zero files, absent optionals)
// ---------------------------------------------------------------------------

#[test]
fn trivial_scan_renders_wellformed_without_optionals() {
    let mut doc = base_doc();
    doc.scan.file_count = 0;
    doc.scan.loc_total = None;
    doc.scan.duration_ms = None;
    doc.parse_health_summary = None;
    doc.files = vec![];
    doc.findings = vec![];
    doc.graphs = None;
    doc.components = None;

    let report = render_report(&doc, &[]); // must not panic

    assert!(report.contains("duration n/a ms"));
    assert!(report.contains("- Total LOC: n/a"));
    assert!(report.contains("Parse health: n/a."));
    // Parse-health clause omitted entirely when the summary is absent.
    assert!(
        !section(
            &report,
            "## 2. Scan provenance & parse health",
            "## 3. Codebase statistics"
        )
        .contains("### Parse health summary")
    );
    assert!(report.contains("No files scanned."));
    assert!(report.contains("No charts were emitted for this scan."));
    assert!(report.ends_with('\n') && !report.contains("\r"));
}

// ---------------------------------------------------------------------------
// 5. Top-10 pinning + path tie-break
// ---------------------------------------------------------------------------

#[test]
fn top_ten_files_pinned_with_path_tiebreak() {
    let mut doc = base_doc();
    doc.files = vec![
        file_entry("drop_me.c", 5), // smallest code LOC → dropped
        file_entry("tie_b.c", 100), // tie on 100 → tie_b before tie_a? NO:
        file_entry("tie_a.c", 100), // path ASC ⇒ tie_a first
        file_entry("mid.c", 90),
        file_entry("big.c", 500),
        file_entry("x1.c", 80),
        file_entry("x2.c", 70),
        file_entry("x3.c", 60),
        file_entry("x4.c", 50),
        file_entry("x5.c", 40),
        file_entry("x6.c", 30),
        file_entry("x7.c", 20),
    ];
    doc.scan.file_count = doc.files.len().min(u32::MAX as usize) as u32;
    let report = render_report(&doc, &[]);
    let table = section(&report, "Top files by code LOC:", "## 4. Findings");

    let expected_order = [
        "big.c", "tie_a.c", "tie_b.c", "mid.c", "x1.c", "x2.c", "x3.c", "x4.c", "x5.c", "x6.c",
    ];
    let mut rows: Vec<String> = Vec::new();
    // Slice begins at the intro line: heading, blank, header, separator.
    for line in table.lines().skip(4).filter(|l| l.starts_with('|')) {
        // Table cells are escape_md'd (dots become `\.`) — unescape for the
        // comparison.
        let path = line
            .split('|')
            .nth(1)
            .expect("table row shape")
            .trim()
            .replace('\\', "");
        rows.push(path);
    }
    assert_eq!(rows, expected_order, "top-10 ordering wrong:\n{table}");
    assert!(
        !table.contains("drop_me"),
        "11th-ranked file leaked into top-10"
    );
}

// ---------------------------------------------------------------------------
// 5b. Component table caps at top-10 with honest counts (C-full-split
// follow-up: same defect class as the §3 take(10) precedent)
// ---------------------------------------------------------------------------

fn component_entry(rank: u32, prefix: &str) -> ComponentEntry {
    ComponentEntry {
        component_id: format!("comp-{prefix}"),
        path_prefix: prefix.to_string(),
        rank,
        metrics: ComponentMetrics {
            complexity_sum: 1.0,
            alloc_site_density: 0.0,
            max_loop_depth: 0,
            finding_density: 1.0,
        },
        finding_count: Some(1),
    }
}

#[test]
fn component_table_caps_at_top_ten_with_honest_header() {
    let mut doc = base_doc();
    doc.components = Some(
        (1..=12)
            .map(|r| component_entry(r, &format!("c{r:02}")))
            .collect(),
    );
    let report = render_report(&doc, &[]);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");

    // Honest header: showing top 10 of 12.
    assert!(
        arch.contains("showing top 10 of 12 components"),
        "honest cap header missing:\n{arch}"
    );
    // Exactly 10 data rows in document (rank) order; ranks 11-12 absent.
    // Cells are escape_md'd (`-` → `\-`) — strip backslashes to compare.
    let plain = arch.replace('\\', "");
    let rows: Vec<&str> = plain.lines().filter(|l| l.starts_with("| comp-")).collect();
    assert_eq!(rows.len(), 10, "component table must cap at 10:\n{arch}");
    assert!(
        plain.contains("| comp-c10 | c10 | 10 |"),
        "rank 10 row missing"
    );
    assert!(
        !plain.contains("c11") && !plain.contains("c12"),
        "ranks beyond top-10 leaked:\n{arch}"
    );
    // Double-render byte-identical modulo provenance (cap is content-only).
    let again = render_report(&doc, &[]);
    assert_eq!(report, again, "capped §5 must double-render byte-identical");
}

#[test]
fn component_table_shows_honest_counts_when_under_cap() {
    let mut doc = base_doc();
    doc.components = Some(vec![component_entry(1, "c01"), component_entry(2, "c02")]);
    let report = render_report(&doc, &[]);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");
    assert!(
        arch.contains("showing top 2 of 2 components"),
        "under-cap header must stay honest:\n{arch}"
    );
    let plain = arch.replace('\\', "");
    assert_eq!(
        plain.lines().filter(|l| l.starts_with("| comp-")).count(),
        2
    );
}

// ---------------------------------------------------------------------------
// 6. Markdown-injection escaping
// ---------------------------------------------------------------------------

#[test]
fn hostile_path_cannot_inject_markdown() {
    let hostile = r#"#incl<ude>&"x".c"#;
    let mut doc = base_doc();
    doc.files = vec![file_entry(hostile, 42)];
    doc.scan.file_count = 1;
    doc.findings = vec![finding_full(
        "PERF-C-ALLOC-001",
        Severity::Hot,
        Confidence::High,
        hostile,
        None,
        false,
        false,
        BTreeMap::new(),
    )];
    let report = render_report(&doc, &[]);

    // Raw metacharacter sequence must never survive verbatim…
    assert!(
        !report.contains(hostile),
        "hostile path unescaped:\n{report}"
    );
    // …and must not produce a link or a heading from the filename.
    assert!(
        !report.contains(&format!("[](charts/{hostile}")),
        "hostile path injected a chart link"
    );
    // Escaped form appears (backslash-pinned set; `"` is not a Markdown
    // metacharacter and stays verbatim).
    assert!(
        report.contains(r#"\#incl\<ude\>\&"x"\.c"#),
        "escaped hostile path missing:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// 7. Appendix A covers exactly the rules present
// ---------------------------------------------------------------------------

#[test]
fn appendix_a_covers_exactly_rules_present() {
    let mut doc = base_doc();
    doc.findings = vec![
        finding("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding("PERF-C-LOOP-003", Severity::Info, Confidence::Medium, "b.c"),
    ];
    let report = render_report(&doc, &[]);
    let appendix = section(&report, "## Appendix A.", "## Appendix B.");

    assert!(appendix.contains("`PERF\\-C\\-ALLOC\\-001`"));
    assert!(appendix.contains("Heap allocation inside loop"));
    assert!(appendix.contains("`PERF\\-C\\-LOOP\\-003`"));
    // A rule with no findings is ABSENT from the table.
    assert!(!appendix.contains("PERF-C-STR-001"));
    // Exactly two data rows.
    let data_rows = appendix
        .lines()
        .filter(|l| l.starts_with("| `PERF"))
        .count();
    assert_eq!(data_rows, 2, "Appendix A row count wrong:\n{appendix}");
}

// ---------------------------------------------------------------------------
// 8. Architecture reflects emitted_charts only
// ---------------------------------------------------------------------------

#[test]
fn architecture_renders_links_only_for_emitted_charts() {
    let mut doc = base_doc();
    doc.components = Some(vec![ComponentEntry {
        component_id: "src/net".to_string(),
        path_prefix: "src/net".to_string(),
        rank: 1,
        metrics: ComponentMetrics {
            complexity_sum: 42.0,
            alloc_site_density: 12.5,
            max_loop_depth: 3,
            finding_density: 8.25,
        },
        finding_count: Some(5),
    }]);
    let charts = vec![
        "include-graph".to_string(),
        "hot-components.svg".to_string(), // filename form also accepted
    ];
    let report = render_report(&doc, &charts);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");

    assert!(arch.contains("[Dependency graph — internal #include dependencies between scanned files.](charts/include-graph.svg)"));
    assert!(arch.contains("](charts/hot-components.svg)"));
    assert!(
        !arch.contains("component-coupling"),
        "unemitted chart leaked in"
    );
    assert!(arch.contains("| src/net | src/net | 1 | 8.25 |"));
    assert!(arch.contains("LOW confidence until a semantic resolver tier exists (ADR-0001)"));
}

#[test]
fn unknown_chart_name_gets_generic_caption() {
    let report = render_report(&base_doc(), &["future-chart".to_string()]);
    assert!(report.contains("[Chart: future-chart.](charts/future-chart.svg)"));
}

#[test]
fn architecture_indexes_emitted_component_pages_under_the_table() {
    let mut doc = base_doc();
    doc.components = Some(vec![
        ComponentEntry {
            component_id: "src/a".to_string(),
            path_prefix: "src/a".to_string(),
            rank: 1,
            metrics: ComponentMetrics {
                complexity_sum: 10.0,
                alloc_site_density: 0.0,
                max_loop_depth: 1,
                finding_density: 8.25,
            },
            finding_count: Some(2),
        },
        ComponentEntry {
            component_id: "src/b".to_string(),
            path_prefix: "src/b".to_string(),
            rank: 2,
            metrics: ComponentMetrics {
                complexity_sum: 1.0,
                alloc_site_density: 0.0,
                max_loop_depth: 0,
                finding_density: 0.0,
            },
            finding_count: Some(0),
        },
    ]);
    // Only the rank-1 page was emitted: the hook links exactly that one, in
    // document (rank) order, AFTER the component table — and the overview
    // link list must not duplicate it.
    let report = render_report(
        &doc,
        &[
            "hot-components.svg".to_string(),
            "component-01-src-a.svg".to_string(),
        ],
    );
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");
    let table_pos = arch
        .find("| src/a | src/a | 1 | 8.25 |")
        .expect("table row");
    let hook_pos = arch
        .find("[Component src/a — per-file hot-path map.](charts/component-01-src-a.svg)")
        .expect("index hook link");
    assert!(table_pos < hook_pos, "hook must follow the table");
    assert!(
        !arch.contains("component-02-src-b"),
        "unemitted page must not link"
    );
    assert_eq!(
        arch.matches("component-01-src-a.svg").count(),
        1,
        "page must link exactly once (no overview duplication):\n{arch}"
    );
}

#[test]
fn architecture_omits_hook_when_no_component_page_emitted() {
    let mut doc = base_doc();
    doc.components = Some(vec![ComponentEntry {
        component_id: "src/a".to_string(),
        path_prefix: "src/a".to_string(),
        rank: 1,
        metrics: ComponentMetrics {
            complexity_sum: 10.0,
            alloc_site_density: 0.0,
            max_loop_depth: 1,
            finding_density: 8.25,
        },
        finding_count: Some(2),
    }]);
    let report = render_report(&doc, &["hot-components.svg".to_string()]);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");
    assert!(!arch.contains("Component detail pages"));
    assert!(!arch.contains("component-"));
}

// ---------------------------------------------------------------------------
// 8b. Layered overviews link in emission order; detail-page predicate is
// anchored on ^component-\d+- (Option A slice 2)
// ---------------------------------------------------------------------------

#[test]
fn architecture_links_layered_overviews_in_emission_order() {
    let report = render_report(
        &base_doc(),
        &[
            "include-graph.svg".to_string(),
            "layer-file.svg".to_string(),
            "layer-entry.svg".to_string(),
            "component-coupling.svg".to_string(),
            "hot-components.svg".to_string(),
            "layer-workspace.svg".to_string(),
            "layer-bands.svg".to_string(),
        ],
    );
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");

    let workspace = "[Workspace map — components as nodes over inter-component call/include edges, heat by candidate finding density.](charts/layer-workspace.svg)";
    let file = "[File hot-path map — top files by degree with reachable reserve, heat by candidate function cost (v2 where available; v1 fallback); highest-cost shown file outlined (ties → path-asc first).](charts/layer-file.svg)";
    let entry = "[Entry-stratified cost map — files stratified by resolved entry reachability, heat by candidate function cost; LOW-confidence call edges excluded.](charts/layer-entry.svg)";
    let bands = "[Component bands — files grouped under capped components, cross-component file edges aggregated as counted bands.](charts/layer-bands.svg)";
    for expected in [
        "[Dependency graph — internal #include dependencies between scanned files.](charts/include-graph.svg)",
        file,
        entry,
        "[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)",
        "[Hot components — ranking by aggregated bottleneck metrics.](charts/hot-components.svg)",
        workspace,
        bands,
    ] {
        assert!(
            arch.contains(expected),
            "overview link missing:\n{expected}\n{arch}"
        );
    }
    // Emission order preserved verbatim (the self-caught BTreeSet-reorder
    // class from slice 1: never re-sort the overview list).
    let positions: Vec<usize> = [
        "charts/include-graph.svg",
        "charts/layer-file.svg",
        "charts/layer-entry.svg",
        "charts/component-coupling.svg",
        "charts/hot-components.svg",
        "charts/layer-workspace.svg",
        "charts/layer-bands.svg",
    ]
    .iter()
    .map(|name| arch.find(name).unwrap_or_else(|| panic!("{name} missing")))
    .collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted, "overview links out of order:\n{arch}");
}

#[test]
fn non_numeric_component_stem_is_an_overview_not_a_detail_page() {
    // Exact-registry routing (B4 pin): a `component-<word>` stem must NOT be
    // swallowed by the detail-page path — it links inline with the overviews
    // under the generic caption.
    let report = render_report(&base_doc(), &["component-notes.svg".to_string()]);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");
    assert!(
        arch.contains("[Chart: component-notes.](charts/component-notes.svg)"),
        "non-numeric component stem must take the overview path:\n{arch}"
    );
    assert!(
        !arch.contains("Component detail — per-file hot-path map."),
        "detail caption leaked onto an overview stem:\n{arch}"
    );

    // B4 numeric-overview collision: `component-5-notes` LOOKS like a
    // `component-<rank>-<slug>` page but names no component in this
    // document — with an empty registry it must NOT route to detail.
    let report = render_report(&base_doc(), &["component-5-notes.svg".to_string()]);
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");
    assert!(
        arch.contains("[Chart: component-5-notes.](charts/component-5-notes.svg)"),
        "numeric colliding stem must take the overview path:\n{arch}"
    );
    assert!(
        !arch.contains("Component detail — per-file hot-path map."),
        "detail caption leaked onto a colliding stem:\n{arch}"
    );
}

#[test]
fn colliding_numeric_stem_is_an_overview_even_with_components_present() {
    // Same collision with a NON-empty registry: only the page reconstructed
    // from a real component (`component-01-src-a`) routes to detail; the
    // lookalike `component-5-notes` still links as a generic overview.
    let mut doc = base_doc();
    doc.components = Some(vec![ComponentEntry {
        component_id: "src/a".to_string(),
        path_prefix: "src/a".to_string(),
        rank: 1,
        metrics: ComponentMetrics {
            complexity_sum: 10.0,
            alloc_site_density: 0.0,
            max_loop_depth: 1,
            finding_density: 8.25,
        },
        finding_count: Some(2),
    }]);
    let report = render_report(
        &doc,
        &[
            "hot-components.svg".to_string(),
            "component-5-notes.svg".to_string(),
            "component-01-src-a.svg".to_string(),
        ],
    );
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");
    assert!(
        arch.contains("[Chart: component-5-notes.](charts/component-5-notes.svg)"),
        "colliding stem must stay on the overview path:\n{arch}"
    );
    assert!(
        arch.contains("[Component src/a — per-file hot-path map.](charts/component-01-src-a.svg)"),
        "genuine registry page must still hook as detail:\n{arch}"
    );
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Slice out `[start_heading, next_marker)` from a rendered report; the end
/// marker is searched AFTER the start marker itself (so an end of "\n\n"
/// means "end of the block following the start text").
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

/// I5 G4: the human report's tier boilerplate must follow the data — a
/// semantic-tier run must not print the default-build "purely syntactic"
/// caveat, and the standing caveat must describe resolved edges as MEDIUM.
#[test]
fn semantic_tier_boilerplate_follows_the_data() {
    let mut doc = base_doc();
    let charts: Vec<String> = Vec::new();
    let md = render_report(&doc, &charts);
    assert!(
        md.contains("purely syntactic tier (`resolver_tier: syntactic`)"),
        "default build keeps the syntactic caveat:\n{md}"
    );
    assert!(
        md.contains(
            "cross-file call edges are LOW confidence until a semantic resolver tier exists"
        ),
        "default build keeps the LOW standing caveat:\n{md}"
    );

    doc.findings[0].resolver_tier = ResolverTier::Semantic;
    let md = render_report(&doc, &charts);
    assert!(
        md.contains("semantic resolver tier active for C"),
        "semantic build must disclose the active tier:\n{md}"
    );
    assert!(
        md.contains("resolved cross-file call edges are MEDIUM confidence"),
        "semantic standing caveat must say MEDIUM:\n{md}"
    );
    assert!(
        !md.contains("purely syntactic tier"),
        "semantic build must not print the syntactic-tier claim:\n{md}"
    );

    let html = perfpal::reporter::render_html(&doc, &charts);
    assert!(
        html.contains("resolved cross-file call edges are MEDIUM confidence"),
        "semantic HTML standing caveat must say MEDIUM:\n{html}"
    );
    assert!(
        !html.contains("LOW confidence until a semantic resolver tier exists"),
        "semantic HTML must not print the default standing caveat:\n{html}"
    );
}
