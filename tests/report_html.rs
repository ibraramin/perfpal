//! `report.html` locks (Option B slice 1, LINKED variant — zero JavaScript).
//!
//! The HTML artifact is a pure function of `(doc, emitted_charts)` with NO
//! provenance surfaces (no `started_at` / `duration_ms` / `root_path`), so
//! double-render AND double-scan comparisons are byte-exact UNMASKED.
//! Every interpolated path/title/message goes through
//! [`perfpal::reporter::escape_html`]; the hostile-input test below is the
//! lock for that rule.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::{Confidence, Severity};
use perfpal::cli::run_with;
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::{escape_html, render_html};
use perfpal::schema::{
    ComponentEntry, ComponentMetrics, FileEntry, FindingEntry, MetricsDocument, ParseHealthEntry,
    ParseHealthSummary, ScanInfo, StatsSection, Threshold, ThresholdValue, ToolInfo,
};

mod common;

// ---------------------------------------------------------------------------
// Local document builders (mirroring tests/reporter_render.rs)
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

fn finding_at(
    rule_id: &str,
    severity: Severity,
    confidence: Confidence,
    path: &str,
) -> FindingEntry {
    let span = Span::point(7, 9);
    FindingEntry {
        finding_id: format!("{rule_id}:{path}:7:9"),
        rule_id: rule_id.to_string(),
        title: format!("Title for {rule_id}"),
        path: path.to_string(),
        symbol: None,
        span,
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

fn base_doc() -> MetricsDocument {
    MetricsDocument {
        schema_version: "1.4".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scan: ScanInfo {
            root_path: "/abs/secret/root".to_string(),
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
        findings: vec![finding_at(
            "PERF-C-ALLOC-001",
            Severity::Hot,
            Confidence::High,
            "a.c",
        )],
        graphs: None,
        components: None,
    }
}

fn component(rank: u32, prefix: &str) -> ComponentEntry {
    ComponentEntry {
        component_id: format!("comp-{prefix}"),
        path_prefix: prefix.to_string(),
        rank,
        metrics: ComponentMetrics {
            complexity_sum: 22.0,
            alloc_site_density: 1.5,
            max_loop_depth: 3,
            finding_density: 8.25,
        },
        finding_count: Some(4),
    }
}

// ---------------------------------------------------------------------------
// 1. Shared HTML escaper pins the five XML-significant chars
// ---------------------------------------------------------------------------

#[test]
fn escape_html_neutralizes_all_five_hostile_chars() {
    assert_eq!(escape_html("plain/path.c"), "plain/path.c");
    assert_eq!(
        escape_html(r#"<a href="x">&'b"#),
        "&lt;a href=&quot;x&quot;&gt;&amp;&#39;b"
    );
    assert_eq!(escape_html("a&b"), "a&amp;b");
    // Single pass: `&` in an existing entity is escaped, never double-passed.
    assert_eq!(escape_html("&lt;"), "&amp;lt;");
}

// ---------------------------------------------------------------------------
// 2. Hostile path / message lock: no markup breakout, no live handlers
// ---------------------------------------------------------------------------

#[test]
fn hostile_path_and_message_are_fully_escaped() {
    let hostile_path = "<img src=x onerror=alert(1)>.c";
    let hostile_message = "<script>alert('xss')</script> & \"quoted\"";
    let mut doc = base_doc();
    doc.files = vec![file_entry(hostile_path, 10)];
    doc.scan.file_count = 1;
    let mut f = finding_at(
        "PERF-C-STR-003",
        Severity::Hot,
        Confidence::High,
        hostile_path,
    );
    f.message = hostile_message.to_string();
    f.finding_id = format!("PERF-C-STR-003:{hostile_path}:7:9");
    doc.findings = vec![f];

    let html = render_html(&doc, &[]);

    // The raw payloads must not appear anywhere (element text, attributes,
    // or drill-down hrefs).
    assert!(
        !html.contains(hostile_path),
        "raw hostile path leaked into report.html"
    );
    assert!(
        !html.contains(hostile_message),
        "raw hostile message leaked into report.html"
    );
    // No live scriptable markup from the payload.
    assert!(
        !html.contains("<script"),
        "payload broke out into a script element"
    );
    assert!(
        !html.contains("<img src=x"),
        "payload broke out into an attacker-controlled img element"
    );
    // The escaped forms ARE present (content preserved, neutralized).
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;.c"));
    assert!(html.contains("&lt;script&gt;"));
    // Encoding contract: LF only, exactly one trailing newline.
    assert!(!html.contains('\r'));
    assert!(html.ends_with('\n'));
    assert!(!html.ends_with("\n\n"));
}

// ---------------------------------------------------------------------------
// 3. Zero JavaScript + LINKED (not inline) charts
// ---------------------------------------------------------------------------

#[test]
fn zero_javascript_and_linked_chart_references() {
    let mut doc = base_doc();
    doc.components = Some(vec![component(1, "src")]);
    let charts = vec![
        "include-graph.svg".to_string(),
        "component-coupling.svg".to_string(),
        "hot-components.svg".to_string(),
        "component-01-src.svg".to_string(),
    ];
    let html = render_html(&doc, &charts);

    // Zero JavaScript surface.
    assert!(!html.contains("<script"));
    assert!(!html.contains("javascript:"));
    assert!(!html.contains("onclick"));
    assert!(!html.contains("onerror"));
    assert!(!html.contains("onload"));
    // LINKED variant: relative links, no inlined SVG content.
    assert!(html.contains("<img src=\"charts/include-graph.svg\""));
    assert!(html.contains("<a href=\"charts/include-graph.svg\""));
    assert!(!html.contains("<svg"));
    // Component detail page linked by pinned filename.
    assert!(html.contains("charts/component-01-src.svg"));
    // Still offline/self-contained: no external references.
    assert!(!html.contains("http://"));
    assert!(!html.contains("https://"));
}

// ---------------------------------------------------------------------------
// 4. No provenance surfaces + double-render byte-identical UNMASKED
// ---------------------------------------------------------------------------

#[test]
fn no_provenance_surfaces_and_double_render_byte_identical() {
    let mut doc_a = base_doc();
    doc_a.scan.started_at = "2026-08-24T00:00:00Z".to_string();
    doc_a.scan.duration_ms = Some(12);
    doc_a.scan.root_path = "/machine/a/checkout".to_string();
    let mut doc_b = base_doc();
    doc_b.scan.started_at = "2027-01-02T03:04:05Z".to_string();
    doc_b.scan.duration_ms = Some(987);
    doc_b.scan.root_path = "/other/b/tree".to_string();

    let charts = vec!["include-graph".to_string()];
    let a = render_html(&doc_a, &charts);
    let b = render_html(&doc_b, &charts);
    assert_eq!(
        a, b,
        "report.html must be byte-identical across provenance changes (no mask)"
    );

    // Neither provenance value may surface anywhere.
    for needle in [
        "2026-08-24T00:00:00Z",
        "2027-01-02T03:04:05Z",
        "/machine/a/checkout",
        "/other/b/tree",
        "duration ",
    ] {
        assert!(!a.contains(needle), "provenance leaked: {needle}");
    }
    // Header still carries the deterministic counts.
    assert!(a.contains("schema_version 1.4"));
    assert!(a.contains("1 files"));
    assert!(a.contains("1 findings"));
}

// ---------------------------------------------------------------------------
// 5. Findings sorted by finding_id; every drill-down anchor resolves
// ---------------------------------------------------------------------------

#[test]
fn findings_sorted_and_all_anchors_resolve() {
    let mut doc = base_doc();
    doc.files = vec![file_entry("b.c", 30), file_entry("a.c", 10)];
    doc.scan.file_count = 2;
    // Deliberately unsorted input order.
    doc.findings = vec![
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding_at("PERF-C-LOOP-001", Severity::Info, Confidence::Low, "a.c"),
    ];
    doc.components = Some(vec![component(1, "src")]);

    let html = render_html(&doc, &["include-graph.svg".to_string()]);

    // Findings-table order is finding_id-ascending regardless of input order.
    let table = section(&html, "<section id=\"findings\">", "</section>");
    let pos_alloc = table.find("PERF-C-ALLOC-001").expect("alloc row");
    let pos_loop = table.find("PERF-C-LOOP-001").expect("loop row");
    let pos_str = table.find("PERF-C-STR-002").expect("str row");
    assert!(
        pos_alloc < pos_loop && pos_loop < pos_str,
        "findings table not in finding_id order:\n{table}"
    );

    // Every internal href="#..." target exists as an id="...".
    let ids = collect_ids(&html);
    for href in collect_hrefs(&html) {
        // External/relative file links (charts/...) are not anchors.
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                ids.iter().any(|id| id == target),
                "drill-down link #{target} has no matching id anchor"
            );
        }
    }
    // The three findings + two files + one component anchors all exist.
    assert!(ids.iter().any(|id| id.starts_with("finding-")));
    assert!(ids.iter().any(|id| id.starts_with("file-")));
    assert!(ids.iter().any(|id| id.starts_with("component-")));
}

/// Extract the `<section ...>...</section>` slice starting at `start`.
fn section<'a>(html: &'a str, start: &str, end: &str) -> &'a str {
    let from = html
        .find(start)
        .unwrap_or_else(|| panic!("{start} missing"));
    let rest = &html[from..];
    let to = rest.find(end).unwrap_or_else(|| panic!("{end} missing"));
    &rest[..to + end.len()]
}

/// Collect every `id="..."` value in document order.
fn collect_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("id=\"") {
        let from = i + 4;
        if let Some(j) = rest[from..].find('"') {
            ids.push(rest[from..from + j].to_string());
            rest = &rest[from + j + 1..];
        } else {
            break;
        }
    }
    ids
}

/// Collect every `href="..."` value in document order.
fn collect_hrefs(html: &str) -> Vec<String> {
    let mut hrefs = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("href=\"") {
        let from = i + 6;
        if let Some(j) = rest[from..].find('"') {
            hrefs.push(rest[from..from + j].to_string());
            rest = &rest[from + j + 1..];
        } else {
            break;
        }
    }
    hrefs
}

// ---------------------------------------------------------------------------
// 7. FIX 1 lock: colliding slugs yield unique anchors, every href resolves
// ---------------------------------------------------------------------------

#[test]
fn colliding_slugs_yield_unique_anchors_and_all_hrefs_resolve() {
    // `a-b.c` and `a/b.c` slugify to the same `a-b-c` fragment (lossy
    // slugs); anchors must stay collision-free via bare / -2 suffixing.
    let mut doc = base_doc();
    doc.files = vec![file_entry("a-b.c", 10), file_entry("a/b.c", 10)];
    doc.scan.file_count = 2;
    doc.findings = vec![
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a-b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a/b.c"),
    ];

    let html = render_html(&doc, &[]);
    let ids = collect_ids(&html);

    // All id= values unique.
    let mut deduped = ids.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(
        ids.len(),
        deduped.len(),
        "duplicate id anchors survive disambiguation: {ids:?}"
    );
    // First colliding use keeps the bare slug, the second gets -2.
    assert!(
        ids.iter().any(|id| id == "file-a-b-c"),
        "bare slug missing: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == "file-a-b-c-2"),
        "disambiguator suffix missing: {ids:?}"
    );
    let finding_ids: Vec<&String> = ids.iter().filter(|id| id.starts_with("finding-")).collect();
    assert_eq!(finding_ids.len(), 2, "both findings need anchors: {ids:?}");
    assert_ne!(finding_ids[0], finding_ids[1]);

    // Every href="#…" resolves to an existing id="…".
    for href in collect_hrefs(&html) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                ids.iter().any(|id| id == target),
                "drill-down link #{target} has no matching id anchor"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 8. FIX 2 lock: hostile chart stems never become links
// ---------------------------------------------------------------------------

#[test]
fn hostile_chart_stem_never_becomes_a_link() {
    let doc = base_doc();
    let hostile = "\"><img src=x onerror=alert(1)>.svg".to_string();
    let html = render_html(&doc, &[hostile, "include-graph.svg".to_string()]);

    // No attribute breakout from the hostile stem.
    assert!(
        !html.contains("alert(1)"),
        "hostile stem content reached the emitted HTML"
    );
    assert!(!html.contains("onerror"), "event-handler breakout in HTML");
    // Unknown stems must not become links at all.
    assert!(!html.contains("charts/\"><img"));
    // The allowlisted chart still links normally.
    assert!(html.contains("<img src=\"charts/include-graph.svg\""));
    assert!(html.contains("<a href=\"charts/include-graph.svg\""));
}

// ---------------------------------------------------------------------------
// 9. FIX 3 lock: findings table carries full §4 parity (io_kind present,
//    hostile evidence escaped)
// ---------------------------------------------------------------------------

#[test]
fn findings_table_renders_full_section4_parity() {
    let mut doc = base_doc();
    let mut f = finding_at("PERF-C-LOOP-002", Severity::Warn, Confidence::Medium, "a.c");
    f.symbol = Some("process".to_string());
    f.resolver_tier = ResolverTier::Syntactic;
    f.macro_suspect = true;
    f.threshold = Some(Threshold {
        name: "min_nesting_depth".to_string(),
        value: ThresholdValue::Number(3.0),
    });
    let hostile_value = "</td><script>alert('evil')</script>".to_string();
    f.evidence = BTreeMap::from([
        ("io_kind".to_string(), "streaming".to_string()),
        ("payload".to_string(), hostile_value.clone()),
    ]);
    doc.findings = vec![f];

    let html = render_html(&doc, &[]);
    let table = section(&html, "<section id=\"findings\">", "</section>");

    // LOOP-002 messages reference io_kind — it must be present.
    assert!(
        table.contains("io_kind=streaming"),
        "io_kind evidence missing from findings table:\n{table}"
    );
    assert!(table.contains("process"), "symbol missing:\n{table}");
    assert!(
        table.contains("syntactic"),
        "resolver tier missing:\n{table}"
    );
    assert!(table.contains("macro_suspect"), "flags missing:\n{table}");
    assert!(
        table.contains("min_nesting_depth=3"),
        "threshold missing:\n{table}"
    );
    // Hostile evidence value is escaped, never raw (review bypass closed).
    assert!(
        !table.contains(&hostile_value),
        "raw hostile evidence leaked into findings table"
    );
    assert!(table.contains("&lt;/td&gt;"), "evidence not escaped");
    assert!(!table.contains("<script"), "script breakout via evidence");
}

// ---------------------------------------------------------------------------
// 10. Small-fix lock: top-10 file drill-down marks hidden files
// ---------------------------------------------------------------------------

#[test]
fn file_drilldown_marks_hidden_files_beyond_top10() {
    let mut doc = base_doc();
    doc.files = (0..12)
        .map(|i| file_entry(&format!("f{i:02}.c"), 10))
        .collect();
    doc.scan.file_count = 12;
    doc.findings = vec![];

    let html = render_html(&doc, &[]);
    let files = section(&html, "<section id=\"files\">", "</section>");
    assert_eq!(
        files.matches("<tr id=\"file-").count(),
        10,
        "drill-down must still cap at 10 rows"
    );
    assert!(
        files.contains("…+2 more (see findings table)"),
        "hidden-file marker missing:\n{files}"
    );

    // No marker when nothing is hidden.
    let small_html = render_html(&base_doc(), &[]);
    assert!(
        !small_html.contains("more (see findings table)"),
        "marker must not appear when no files are hidden"
    );
}

// ---------------------------------------------------------------------------
// 6. Double-scan byte-identical proof (full pipeline, unmasked)
// ---------------------------------------------------------------------------

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("perfpal-html-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(output: &str, target: &str) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain([
            OsString::from("--output"),
            OsString::from(output),
            OsString::from(target),
        ])
        .collect()
}

#[test]
fn double_scan_report_html_byte_identical_unmasked() {
    let root = temp_dir("scan");
    let out = root.join("perfpal-report");
    let out_str = out.to_string_lossy().into_owned();
    let target = fixture_root().to_string_lossy().into_owned();

    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let first = fs::read(out.join("report.html")).expect("report.html emitted");
    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let second = fs::read(out.join("report.html")).expect("report.html emitted");

    assert_eq!(
        first, second,
        "report.html diverged between scans (must be byte-identical UNMASKED)"
    );
    let text = String::from_utf8(first).expect("report.html is UTF-8");
    assert!(!text.contains("<script"));
    assert!(text.ends_with('\n'));
    assert!(!text.contains(&target), "absolute scan root leaked");
}
