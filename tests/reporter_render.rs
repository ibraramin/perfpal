//! Reporter render-contract tests (output-contract §5; P4 Wave 3b).
//!
//! Documents are constructed programmatically (direct `MetricsDocument`
//! literals via local helpers) — no scanner/analyzer run needed; the
//! reporter is a pure function of `(doc, emitted_charts)`.

use std::collections::BTreeMap;

use perfpal::analyzer::{Confidence, Severity};
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::render_report;
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

#[test]
fn findings_grouped_hot_warn_info_with_document_order_within_group() {
    let mut doc = base_doc();
    doc.findings = vec![
        // Document order deliberately interleaves severities and scrambles
        // sort keys within a group — the reporter must partition stably.
        finding("PERF-C-CALL-001", Severity::Info, Confidence::High, "z.c"),
        finding("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding("PERF-C-STR-002", Severity::Warn, Confidence::High, "m.c"),
        finding("PERF-C-LOOP-002", Severity::Hot, Confidence::Medium, "b.c"),
        finding("PERF-C-CALL-002", Severity::Warn, Confidence::Low, "n.c"),
        finding("PERF-C-LOOP-003", Severity::Info, Confidence::Low, "y.c"),
    ];
    let report = render_report(&doc, &[]);

    let order: Vec<usize> = [
        "PERF-C-ALLOC-001:a.c:7:9",
        "PERF-C-LOOP-002:b.c:7:9",
        "PERF-C-STR-002:m.c:7:9",
        "PERF-C-CALL-002:n.c:7:9",
        "PERF-C-CALL-001:z.c:7:9",
        "PERF-C-LOOP-003:y.c:7:9",
    ]
    .iter()
    .map(|id| {
        // Citations are escape_md'd (`-`→`\-`, `.`→`\.`).
        let escaped = id.replace('-', "\\-").replace('.', "\\.");
        report
            .find(&format!("(`{escaped}`)"))
            .unwrap_or_else(|| panic!("{id} not cited:\n{report}"))
    })
    .collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(
        order, sorted,
        "findings not grouped hot→warn→info:\n{report}"
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
    let report = render_report(&doc, &[]);

    let flagged_section = &report[report.find("#### ").unwrap()..];
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

    assert!(arch.contains("[Dependency graph — internal #include (C) and mod/use (Rust) dependencies between scanned files.](charts/include-graph.svg)"));
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
            "component-coupling.svg".to_string(),
            "hot-components.svg".to_string(),
            "layer-workspace.svg".to_string(),
        ],
    );
    let arch = section(&report, "## 5. Architecture", "## Appendix A.");

    let workspace = "[Workspace map — components as nodes over inter-component call/include edges, heat by finding density.](charts/layer-workspace.svg)";
    let file = "[File hot-path map — top files by degree, heat by function cost; highest-cost shown file outlined (ties → path-asc first).](charts/layer-file.svg)";
    for expected in [
        "[Dependency graph — internal #include (C) and mod/use (Rust) dependencies between scanned files.](charts/include-graph.svg)",
        file,
        "[Component coupling — cross-file call edges between components.](charts/component-coupling.svg)",
        "[Hot components — ranking by aggregated bottleneck metrics.](charts/hot-components.svg)",
        workspace,
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
        "charts/component-coupling.svg",
        "charts/hot-components.svg",
        "charts/layer-workspace.svg",
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
