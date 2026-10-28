//! C-split locks (founder-approved full-artifact split): the uncapped
//! per-finding detail lives in the canonical `findings-detail.html` artifact
//! — promoted VERBATIM from `html_findings_table`/`html_finding_row` — while
//! the human `report.md` / `report.html` stay bounded (groups + links only)
//! at any scale. `findings-detail.md` is gone (H3 artifact consolidation).
//!
//! Locked here:
//! 1. The detail artifact contains EVERY finding (counts match `metrics.json`).
//! 2. Human files contain ZERO per-finding rows (group rows + links only) and
//!    `report.md` §4 ends with the single `findings-detail.html` pointer.
//! 3. Every human → detail link resolves (detail anchors exist).
//! 4. Double-render byte-identical for the HTML detail (no provenance
//!    surfaces), including a full-pipeline double-scan proof.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::{Confidence, Severity};
use perfpal::cli::run_with;
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::{
    render_atlas_q2_html, render_atlas_weight_html, render_findings_detail_html, render_html,
    render_report,
};
use perfpal::schema::{
    FileEntry, FindingEntry, MetricsDocument, ParseHealthEntry, ParseHealthSummary, ScanInfo,
    StatsSection, ToolInfo,
};

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
        path_class: None,
    }
}

fn finding(rule_id: &str, severity: Severity, path: &str) -> FindingEntry {
    let span = Span::point(7, 9);
    FindingEntry {
        finding_id: format!("{rule_id}:{path}:7:9"),
        rule_id: rule_id.to_string(),
        title: format!("Title for {rule_id}"),
        path: path.to_string(),
        symbol: None,
        span,
        confidence: Confidence::High,
        severity,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect: false,
        error_node_overlap: false,
        threshold: None,
        message: format!("Message for {rule_id}"),
        evidence: BTreeMap::new(),
    }
}

/// Two firing rules, three findings (hot ALLOC ×2 across two files, warn STR
/// ×1) — enough to pin grouping, exemplars, and cross-page anchors.
fn multi_doc() -> MetricsDocument {
    MetricsDocument {
        schema_version: "1.4".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: Some(12),
            languages: vec!["c".to_string()],
            file_count: 2,
            loc_total: Some(loc(10)),
        },
        parse_health_summary: Some(ParseHealthSummary {
            ok: 2,
            partial: 0,
            failed: 0,
        }),
        files: vec![file_entry("a.c", 10), file_entry("b.c", 30)],
        stats: StatsSection {
            functions: vec![],
            distributions: None,
        },
        findings: vec![
            finding("PERF-C-ALLOC-001", Severity::Hot, "b.c"),
            finding("PERF-C-ALLOC-001", Severity::Hot, "a.c"),
            finding("PERF-C-STR-002", Severity::Warn, "b.c"),
        ],
        graphs: None,
        components: None,
    }
}

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
// 1. Detail artifacts contain every finding
// ---------------------------------------------------------------------------

#[test]
fn detail_html_contains_every_finding_row() {
    let doc = multi_doc();
    let detail = render_findings_detail_html(&doc);
    assert_eq!(
        detail.matches("<tr id=\"finding-").count(),
        doc.findings.len(),
        "findings-detail.html must render every finding row:\n{detail}"
    );
    assert!(detail.contains("2 rules: 3 of 3 total findings (2 hot, 1 warn, 0 info)."));
    for f in &doc.findings {
        assert!(
            detail.contains(&perfpal::reporter::escape_html(&f.finding_id)),
            "{} missing from findings-detail.html",
            f.finding_id
        );
    }
    // Per-rule group anchors exist (the human page links here).
    assert!(detail.contains("id=\"rule-PERF-C-ALLOC-001\""));
    assert!(detail.contains("id=\"rule-PERF-C-STR-002\""));
    assert!(detail.ends_with('\n') && !detail.contains('\r'));
}

// ---------------------------------------------------------------------------
// 2. Human files carry zero per-finding rows (groups + links only)
// ---------------------------------------------------------------------------

#[test]
fn human_report_md_carries_zero_per_finding_rows() {
    let doc = multi_doc();
    let report = render_report(&doc, &[]);
    // Zero detail rows anywhere in the human file…
    assert_eq!(
        report.matches("#### ").count(),
        0,
        "report.md must carry zero per-finding rows:\n{report}"
    );
    // …but the §4 group rows survive (one per firing rule)…
    let sec4_plain = report.replace('\\', "");
    assert!(sec4_plain.contains("| `PERF-C-ALLOC-001` | 2 | 2 | 0 | 0 |"));
    assert!(sec4_plain.contains("| `PERF-C-STR-002` | 1 | 0 | 1 | 0 |"));
    assert!(sec4_plain.contains("2 rules: 3 of 3 total findings"));
    // …and §4 closes with the canonical HTML detail pointer instead.
    assert!(
        report.contains(
            "All 3 of 3 findings: [findings-detail.html](findings-detail.html) — complete ranked action list and per-finding detail (2 rules)."
        ),
        "§4 must carry the full disclosure pointer:\n{report}"
    );
    assert!(!report.contains("findings-detail.md"));
}

#[test]
fn human_report_html_carries_zero_per_finding_rows() {
    let doc = multi_doc();
    let html = render_html(&doc, &[]);
    let findings = section(&html, "<section id=\"findings\">", "</section>");
    // Zero per-finding rows in the human page…
    assert_eq!(
        findings.matches("<tr id=\"finding-").count(),
        0,
        "report.html #findings must carry zero per-finding rows:\n{findings}"
    );
    // …but one <details> group per firing rule, each linking the detail page.
    assert_eq!(
        findings.matches("<details>").count(),
        2,
        "one details group per rule:\n{findings}"
    );
    assert!(findings.contains("findings-detail.html#finding-"));
    assert!(findings.contains("findings-detail.html#rule-PERF-C-ALLOC-001"));
    assert!(findings.contains("findings-detail.html#rule-PERF-C-STR-002"));
    assert!(findings.contains("<a href=\"findings-detail.html\">findings-detail.html</a>"));
}

#[test]
fn empty_scan_renders_no_findings_everywhere() {
    let mut doc = multi_doc();
    doc.findings = vec![];
    let report = render_report(&doc, &[]);
    assert!(report.contains("No findings."));
    assert_eq!(report.matches("#### ").count(), 0);
    assert!(!report.contains("findings-detail.html"));
    let html = render_html(&doc, &[]);
    assert!(section(&html, "<section id=\"findings\">", "</section>").contains("No findings."));
    let detail_html = render_findings_detail_html(&doc);
    assert!(
        section(&detail_html, "<section id=\"findings\">", "</section>").contains("No findings.")
    );
}

// ---------------------------------------------------------------------------
// 3. Every human → detail link resolves
// ---------------------------------------------------------------------------

#[test]
fn every_human_link_resolves_to_a_detail_anchor() {
    let doc = multi_doc();
    let html = render_html(&doc, &[]);
    let detail = render_findings_detail_html(&doc);
    let report_ids = collect_ids(&html);
    let detail_ids = collect_ids(&detail);

    let pages: BTreeMap<&str, Vec<String>> = [
        ("findings-detail.html", detail_ids.clone()),
        (
            "atlas-weight.html",
            collect_ids(&render_atlas_weight_html(&doc)),
        ),
        ("atlas-q2.html", collect_ids(&render_atlas_q2_html(&doc))),
    ]
    .into_iter()
    .collect();
    for href in collect_hrefs(&html) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                report_ids.iter().any(|id| id == target),
                "in-page link #{target} has no matching id in report.html"
            );
        } else if let Some(hash) = href.find('#') {
            let (page, target) = href.split_at(hash);
            let target = &target[1..];
            let page_ids = pages
                .get(page)
                .unwrap_or_else(|| panic!("unexpected cross-page link {href}"));
            assert!(
                page_ids.iter().any(|id| id == target),
                "link {href} has no matching id anchor on {page}"
            );
        }
    }
    // Detail in-page links resolve too.
    for href in collect_hrefs(&detail) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                detail_ids.iter().any(|id| id == target),
                "detail in-page link #{target} has no matching id"
            );
        }
    }
    // The exemplar + file drill-down links point at REAL finding rows.
    let row_ids: Vec<&String> = detail_ids
        .iter()
        .filter(|id| id.starts_with("finding-"))
        .collect();
    assert_eq!(row_ids.len(), 3, "all three finding rows need anchors");
    for href in collect_hrefs(&html) {
        if href.starts_with("findings-detail.html#finding-") {
            let target = href.split('#').nth(1).unwrap();
            assert!(
                row_ids.iter().any(|id| id.as_str() == target),
                "finding link {href} points at no row"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Double-render byte-identical (no provenance in either artifact)
// ---------------------------------------------------------------------------

#[test]
fn detail_artifacts_double_render_byte_identical_without_provenance() {
    let mut doc_a = multi_doc();
    doc_a.scan.started_at = "2026-08-24T00:00:00Z".to_string();
    doc_a.scan.duration_ms = Some(12);
    doc_a.scan.root_path = "/machine/a/checkout".to_string();
    let mut doc_b = multi_doc();
    doc_b.scan.started_at = "2027-01-02T03:04:05Z".to_string();
    doc_b.scan.duration_ms = Some(987);
    doc_b.scan.root_path = "/other/b/tree".to_string();

    assert_eq!(
        render_findings_detail_html(&doc_a),
        render_findings_detail_html(&doc_b),
        "findings-detail.html must be byte-identical across provenance changes"
    );
    for needle in [
        "2026-08-24T00:00:00Z",
        "2027-01-02T03:04:05Z",
        "/machine/a/checkout",
        "/other/b/tree",
    ] {
        assert!(!render_findings_detail_html(&doc_a).contains(needle));
    }
}

// ---------------------------------------------------------------------------
// 5. Full-pipeline proof: detail counts match metrics.json on the e2e tree
// ---------------------------------------------------------------------------

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-split-{tag}-{}-{nanos}",
        std::process::id()
    ));
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
fn pipeline_detail_counts_match_metrics_and_human_stays_bounded() {
    let root = temp_dir("counts");
    let out = root.join("perfpal-report");
    let out_str = out.to_string_lossy().into_owned();
    let target = fixture_root().to_string_lossy().into_owned();
    assert_eq!(run_with(argv(&out_str, &target)), 0);

    let metrics_raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json");
    let metrics: serde_json::Value = serde_json::from_str(&metrics_raw).expect("metrics parses");
    let finding_count = metrics["findings"].as_array().expect("findings[]").len();

    // The Markdown detail artifact is gone (H3 consolidation).
    assert!(
        !out.join("findings-detail.md").exists(),
        "findings-detail.md must not be emitted"
    );

    // The canonical HTML detail contains every finding row.
    let detail_html = fs::read_to_string(out.join("findings-detail.html")).expect("detail html");
    assert_eq!(
        detail_html.matches("<tr id=\"finding-").count(),
        finding_count,
        "findings-detail.html must render every metrics.json finding row"
    );

    // Human files stay bounded: zero per-finding rows, one §4 pointer.
    let report = fs::read_to_string(out.join("report.md")).expect("report.md");
    assert_eq!(report.matches("#### ").count(), 0);
    assert!(
        report.contains(&format!(
            "All {finding_count} of {finding_count} findings: [findings-detail.html](findings-detail.html)"
        )),
        "report.md §4 must carry the full-disclosure pointer"
    );
    let html = fs::read_to_string(out.join("report.html")).expect("report.html");
    let findings = section(&html, "<section id=\"findings\">", "</section>");
    assert_eq!(findings.matches("<tr id=\"finding-").count(), 0);

    // Double-scan: the HTML detail is byte-identical UNMASKED (no provenance).
    let html1 = fs::read(out.join("findings-detail.html")).expect("detail html bytes");
    assert_eq!(run_with(argv(&out_str, &target)), 0);
    assert_eq!(html1, fs::read(out.join("findings-detail.html")).unwrap());
}

fn section<'a>(html: &'a str, start: &str, end: &str) -> &'a str {
    let from = html
        .find(start)
        .unwrap_or_else(|| panic!("{start} missing"));
    let rest = &html[from..];
    let to = rest.find(end).unwrap_or_else(|| panic!("{end} missing"));
    &rest[..to + end.len()]
}
