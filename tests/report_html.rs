//! `report.html` locks (Option B slice 2: slice-1 LINKED page + one fixed
//! enhancement script — pan/zoom + click-to-filter, no libraries).
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
use perfpal::reporter::{
    ReportWidth, atlas_story, escape_html, render_atlas_q2_html, render_atlas_weight_html,
    render_findings_detail_html, render_html, render_html_with_story_at_width,
    render_profile_queue_html, report_script,
};
use perfpal::schema::{
    ComponentEntry, ComponentMetrics, FileEntry, FindingEntry, FunctionStats, MetricsDocument,
    ParseHealthEntry, ParseHealthSummary, ScanInfo, StatsSection, Threshold, ThresholdValue,
    ToolInfo,
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
        path_class: None,
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

    // The human page carries the hostile path (drill-down + exemplar
    // finding_id) and, since H1, the finding message in the restored action
    // block. Both pages must escape.
    let html = render_html(&doc, &[]);
    let detail = render_findings_detail_html(&doc);

    // The raw payloads must not appear anywhere (element text, attributes,
    // or drill-down hrefs).
    for (page, name) in [(&html, "report.html"), (&detail, "findings-detail.html")] {
        assert!(
            !page.contains(hostile_path),
            "raw hostile path leaked into {name}"
        );
        assert!(
            !page.contains(hostile_message),
            "raw hostile message leaked into {name}"
        );
        // No live scriptable markup from the payload (each page's own fixed
        // script block is the only `<script>` allowed — see the
        // fixed-script test below).
        assert!(
            !page.contains("<script>alert"),
            "payload broke out into a script element in {name}"
        );
        assert!(
            !page.contains("<img src=x"),
            "payload broke out into an attacker-controlled img element in {name}"
        );
        // The escaped forms ARE present (content preserved, neutralized).
        assert!(
            page.contains("&lt;img src=x onerror=alert(1)&gt;.c"),
            "escaped hostile path missing in {name}"
        );
        // Encoding contract: LF only, exactly one trailing newline.
        assert!(!page.contains('\r'), "CR leaked into {name}");
        assert!(page.ends_with('\n'), "trailing newline missing in {name}");
        assert!(!page.ends_with("\n\n"), "double trailing newline in {name}");
    }
    // The restored action block carries the finding message on the human page;
    // it must be escaped, never raw and never executable.
    assert!(
        html.contains("&lt;script&gt;"),
        "escaped finding message missing from report.html"
    );
    assert!(
        detail.contains("&lt;script&gt;"),
        "escaped message missing in detail"
    );
}

// ---------------------------------------------------------------------------
// 3. Fixed enhancement script + LINKED (not inline) charts (slice 2)
// ---------------------------------------------------------------------------

/// One `<script ...>` element split from the page.
struct ScriptBlock<'a> {
    tag: &'a str,
    body: &'a str,
}

/// Extract every `<script ...>…</script>` element in document order.
fn scripts(html: &str) -> Vec<ScriptBlock<'_>> {
    let mut blocks = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        let element = &rest[start..];
        let open_end = element.find('>').expect("script open tag terminated");
        let body_start = start + open_end + 1;
        let body_end = rest[body_start..]
            .find("</script>")
            .expect("script close tag")
            + body_start;
        blocks.push(ScriptBlock {
            tag: &rest[start..start + open_end + 1],
            body: &rest[body_start..body_end],
        });
        rest = &rest[body_end + "</script>".len()..];
    }
    blocks
}

/// The single fixed enhancement block used by the detail/drill pages.
fn script_body(html: &str) -> &str {
    let blocks = scripts(html);
    assert_eq!(blocks.len(), 1, "exactly one inline script block expected");
    blocks[0].body.trim_matches('\n')
}

/// The report's unchanged REPORT_SCRIPT, selected by body identity among the
/// three ADR-0014 script elements.
fn report_script_body(html: &str) -> &str {
    let blocks = scripts(html);
    assert_eq!(
        blocks.len(),
        3,
        "report.html must carry exactly three script elements"
    );
    let matching: Vec<&ScriptBlock<'_>> = blocks
        .iter()
        .filter(|block| block.body.trim_matches('\n') == report_script())
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "exactly one REPORT_SCRIPT block expected"
    );
    matching[0].body.trim_matches('\n')
}

fn story_script(html: &str) -> &str {
    scripts(html)
        .into_iter()
        .find(|block| block.tag.contains("id=\"atlas-story-script\""))
        .map(|block| block.body)
        .expect("atlas-story-script block")
}

fn story_data(html: &str) -> &str {
    scripts(html)
        .into_iter()
        .find(|block| block.tag.contains("type=\"application/json\""))
        .map(|block| block.body)
        .expect("inline story data block")
}

#[test]
fn fixed_script_and_linked_chart_references() {
    let mut doc = base_doc();
    doc.components = Some(vec![component(1, "src")]);
    let charts = vec![
        "include-graph.svg".to_string(),
        "component-coupling.svg".to_string(),
        "hot-components.svg".to_string(),
        "layer-workspace.svg".to_string(),
        "layer-file.svg".to_string(),
        "layer-entry.svg".to_string(),
        "layer-bands.svg".to_string(),
        "component-01-src.svg".to_string(),
    ];
    let html = render_html(&doc, &charts);

    // ADR-0014 (amended A12): exactly three script elements — the unchanged
    // REPORT_SCRIPT, one fixed story script, and one inline JSON data block.
    assert_eq!(report_script_body(&html), report_script());
    assert!(report_script().lines().count() < 60);
    let blocks = scripts(&html);
    assert_eq!(blocks.len(), 3, "exactly three script elements");
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block.tag == "<script>")
            .count(),
        1,
        "exactly one bare REPORT_SCRIPT element"
    );
    assert_eq!(
        blocks
            .iter()
            .filter(|block| block.tag.contains("id=\"atlas-story-script\""))
            .count(),
        1,
        "exactly one fixed story script"
    );
    assert_eq!(
        blocks
            .iter()
            .filter(|block| {
                block.tag.contains("type=\"application/json\"")
                    && block.tag.contains("id=\"perfpal-atlas-story-data\"")
            })
            .count(),
        1,
        "exactly one inline story data block"
    );
    let story_script = story_script(&html);
    assert!(!story_script.trim().is_empty(), "story script is empty");
    for banned in [
        "</script",
        "fetch(",
        "http://",
        "https://",
        "eval(",
        "Function(",
    ] {
        assert!(
            !story_script.contains(banned),
            "banned string {banned:?} present in the story script"
        );
    }
    let data: serde_json::Value =
        serde_json::from_str(story_data(&html)).expect("inline story JSON parses");
    assert_eq!(data["index"]["cases"][0]["id"], "scan");
    let payload = &data["cases"]["scan"];
    for key in [
        "case", "source", "meta", "trust", "nodes", "edges", "state", "trace",
    ] {
        assert!(payload.get(key).is_some(), "payload key {key} missing");
    }
    let fingerprint = payload["source"]["layoutFingerprint"]
        .as_str()
        .expect("layoutFingerprint string");
    assert_eq!(fingerprint.len(), 16);
    assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    let metrics_fingerprint = payload["source"]["metricsContentFingerprint"]
        .as_str()
        .expect("metricsContentFingerprint string");
    assert_eq!(metrics_fingerprint.len(), 16);
    assert!(metrics_fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    // Story placement: after #map, before #explorer.
    let map = html.find("<section id=\"map\">").expect("#map");
    let story = html
        .find("<section id=\"atlas-story\" class=\"atlas-story\" data-atlas-block=\"story\"")
        .expect("#atlas-story");
    let explorer = html.find("<section id=\"explorer\">").expect("#explorer");
    assert!(map < story && story < explorer, "story placement wrong");
    // No libraries, no vendoring, no external references.
    assert!(!html.contains("http://"));
    assert!(!html.contains("https://"));
    assert!(!html.contains("src=\"http"));
    // No inline event-handler attributes — binding is via addEventListener
    // in the one script block.
    assert!(!html.contains("onclick"));
    assert!(!html.contains("onerror"));
    assert!(!html.contains("onload"));
    assert!(!html.contains("javascript:"));
    // LINKED variant for chart stems: relative links, no inlined chart SVG.
    // The one inline SVG is the ratified Atlas canvas (ADR-0009/0010).
    assert!(html.contains("<img src=\"charts/include-graph.svg\""));
    assert!(html.contains("<a href=\"charts/include-graph.svg\""));
    assert_eq!(
        html.matches("<svg").count(),
        1,
        "only the Atlas canvas SVG may be inlined"
    );
    let canvas = html.find("id=\"atlas-canvas\"").expect("atlas canvas");
    let inline = html.find("<svg").expect("inline canvas svg");
    let brief = html.find("id=\"atlas-brief\"").expect("atlas brief");
    assert!(canvas < inline && inline < brief);
    // Pan/zoom figures carry the enhancement hook (buttons are added by the
    // script at runtime, so no-JS output stays fully usable static).
    assert!(html.contains("<figure class=\"pz\" data-chart=\"layer-workspace\">"));
    assert!(html.contains("<figure class=\"pz\" data-chart=\"layer-file\">"));
    assert!(html.contains("<figure class=\"pz\" data-chart=\"layer-entry\">"));
    assert!(html.contains("<figure class=\"pz\" data-chart=\"layer-bands\">"));
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
    // C-split: the detail artifact carries no provenance either.
    let da = render_findings_detail_html(&doc_a);
    let db = render_findings_detail_html(&doc_b);
    assert_eq!(
        da, db,
        "findings-detail.html must be byte-identical across provenance changes (no mask)"
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
        assert!(
            !da.contains(needle),
            "provenance leaked into detail: {needle}"
        );
    }
    // Header still carries the deterministic counts.
    assert!(a.contains("schema_version 1.4"));
    assert!(a.contains("1 files"));
    assert!(a.contains("1 findings"));
    assert!(da.contains("1 findings"));
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
    let detail = render_findings_detail_html(&doc);

    // Grouped findings order: hottest rule-group first (ALLOC hot), then
    // warn (STR), then info (LOOP) — the pinned severity-rank → count-desc →
    // rule_id-asc group order; members within a group stay finding_id-asc.
    // C-split: groups render in BOTH pages (summaries in report.html, full
    // tables in the detail artifact).
    // The detail page nests one rule-section per group — slice it
    // depth-aware; the human page has no nesting.
    let report_table = section(&html, "<section id=\"findings\">", "</section>");
    let detail_table = section_nested(&detail, "<section id=\"findings\">");
    for (table, name) in [(&report_table, "report.html"), (&detail_table, "detail")] {
        let pos_alloc = table.find("PERF-C-ALLOC-001").expect("alloc group");
        let pos_str = table.find("PERF-C-STR-002").expect("str group");
        let pos_loop = table.find("PERF-C-LOOP-001").expect("loop group");
        assert!(
            pos_alloc < pos_str && pos_str < pos_loop,
            "findings not in pinned group order in {name}:\n{table}"
        );
    }
    // C-split: report.html carries zero per-finding rows; the detail page
    // carries all three.
    let report_findings = section(&html, "<section id=\"findings\">", "</section>");
    assert_eq!(
        report_findings.matches("<tr id=\"finding-").count(),
        0,
        "report.html must carry zero per-finding rows"
    );
    let detail_findings = section_nested(&detail, "<section id=\"findings\">");
    assert_eq!(
        detail_findings.matches("<tr id=\"finding-").count(),
        3,
        "detail page must carry every finding row"
    );

    // Every internal href="#..." target exists as an id="..." in the SAME
    // page; every cross-page target exists on its rendered page.
    let ids = collect_ids(&html);
    let pages = cross_page_ids(&doc);
    for href in collect_hrefs(&html) {
        // External/relative file links (charts/..., *.html) are not anchors.
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                ids.iter().any(|id| id == target),
                "drill-down link #{target} has no matching id anchor"
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
    let detail_ids = collect_ids(&detail);
    for href in collect_hrefs(&detail) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                detail_ids.iter().any(|id| id == target),
                "detail in-page link #{target} has no matching id anchor"
            );
        }
    }
    // The three finding anchors live in the detail page; files + component
    // anchors stay in the human page.
    assert!(detail_ids.iter().any(|id| id.starts_with("finding-")));
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

/// Extract the `<section ...>` element starting at `start`, matching nested
/// `<section>` elements by depth (the C-split detail page nests one
/// `rule-…` section per firing rule inside `#findings`, so the naive
/// first-`</section>` slice would truncate after the first group).
fn section_nested<'a>(html: &'a str, start: &str) -> &'a str {
    let from = html
        .find(start)
        .unwrap_or_else(|| panic!("{start} missing"));
    let rest = &html[from..];
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < rest.len() {
        if rest[i..].starts_with("<section") {
            depth += 1;
            i += "<section".len();
        } else if rest[i..].starts_with("</section>") {
            depth -= 1;
            i += "</section>".len();
            if depth == 0 {
                return &rest[..i];
            }
        } else {
            // Char-boundary-safe advance (the template carries multibyte
            // `—`/`…`/`▸` literals).
            i += rest[i..].chars().next().map_or(1, |c| c.len_utf8());
        }
    }
    panic!("unbalanced <section> elements from {start}");
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

/// IDs on every cross-page artifact the Atlas page links to.
fn cross_page_ids(doc: &MetricsDocument) -> BTreeMap<String, Vec<String>> {
    let mut pages = BTreeMap::new();
    pages.insert(
        "findings-detail.html".to_string(),
        collect_ids(&render_findings_detail_html(doc)),
    );
    pages.insert(
        "atlas-weight.html".to_string(),
        collect_ids(&render_atlas_weight_html(doc)),
    );
    pages.insert(
        "atlas-q2.html".to_string(),
        collect_ids(&render_atlas_q2_html(doc)),
    );
    pages.insert(
        "profile-queue.html".to_string(),
        collect_ids(&render_profile_queue_html(doc)),
    );
    pages
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

    // C-split: file anchors live in report.html, finding anchors in the
    // detail artifact (same assignment map — every cross-page href resolves).
    let html = render_html(&doc, &[]);
    let detail = render_findings_detail_html(&doc);
    let ids = collect_ids(&html);
    let detail_ids = collect_ids(&detail);

    // All id= values unique within each page.
    for (page_ids, name) in [(&ids, "report.html"), (&detail_ids, "detail")] {
        let mut deduped = page_ids.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(
            page_ids.len(),
            deduped.len(),
            "duplicate id anchors survive disambiguation in {name}: {page_ids:?}"
        );
    }
    // First colliding use keeps the bare slug, the second gets -2.
    assert!(
        ids.iter().any(|id| id == "file-a-b-c"),
        "bare slug missing: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == "file-a-b-c-2"),
        "disambiguator suffix missing: {ids:?}"
    );
    let finding_ids: Vec<&String> = detail_ids
        .iter()
        .filter(|id| id.starts_with("finding-"))
        .collect();
    assert_eq!(
        finding_ids.len(),
        2,
        "both findings need anchors in the detail page: {detail_ids:?}"
    );
    assert_ne!(finding_ids[0], finding_ids[1]);

    // Every href="#…" resolves in-page; every cross-page target resolves.
    let pages = cross_page_ids(&doc);
    for href in collect_hrefs(&html) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                ids.iter().any(|id| id == target),
                "drill-down link #{target} has no matching id anchor"
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
    for href in collect_hrefs(&detail) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                detail_ids.iter().any(|id| id == target),
                "detail in-page link #{target} has no matching id anchor"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 7b. BLOCK-remediation lock: slug-colliding findings link to their OWN rows
// even when ranked-file order disagrees with finding_id order
// ---------------------------------------------------------------------------

/// One costly function in `path` (cost = cyclomatic) to flip the file
/// drill-down ranking without touching any finding identity.
fn costly_fn(path: &str) -> FunctionStats {
    FunctionStats {
        id: format!("{path}#work#1:1"),
        name: "work".to_string(),
        path: path.to_string(),
        span: Span::point(1, 1),
        param_count: 0,
        cyclomatic_complexity: 50,
        cognitive_complexity: 0,
        max_loop_depth: 0,
        alloc_site_count: 0,
        length_lines: 10,
        call_fanout: Some(0),
        call_fanin: Some(0),
        call_depth: Some(0),
        is_recursive: Some(false),
        idiom_hash: None,
        error_path: Some(false),
        alloc_family: None,
        cfg_gated: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: None,
        function_cost_v2: None,
        unknown_trip_loops: 0,
        trip_count_source: None,
        known_trip_value: None,
    }
}

/// Value of attribute `name="…"` in a tag/row fragment.
fn attr_value(fragment: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let from = fragment.find(&needle)? + needle.len();
    let to = fragment[from..].find('"')?;
    Some(fragment[from..from + to].to_string())
}

/// `(data-path, href-target)` for every finding link in each `<tr …>` row
/// of a section, in row order (file drill-down rows carry the finding
/// links — C-split: `href="findings-detail.html#…"` cross-page targets;
/// the `#` fragment is returned so callers compare anchors directly).
fn file_row_links(files_section: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for chunk in files_section.split("<tr ").skip(1) {
        let row = &chunk[..chunk.find("</tr>").expect("row close")];
        let path = attr_value(row, "data-path").expect("file row data-path");
        let mut rest = row;
        while let Some(i) = rest.find("href=\"") {
            let from = i + "href=\"".len();
            let to = rest[from..].find('"').expect("href close");
            let href = &rest[from..from + to];
            if let Some(hash) = href.find('#') {
                out.push((path.clone(), href[hash + 1..].to_string()));
            }
            rest = &rest[from + to + 1..];
        }
    }
    out
}

/// `id="…" → data-path="…"` for every `<tr …>` row of a section (findings
/// table rows: anchor → owning file).
fn row_paths_by_id(findings_section: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for chunk in findings_section.split("<tr ").skip(1) {
        let row = &chunk[..chunk.find("</tr>").expect("row close")];
        let id = attr_value(row, "id").expect("finding row id");
        let path = attr_value(row, "data-path").expect("finding row data-path");
        map.insert(id, path);
    }
    map
}

#[test]
fn slug_colliding_findings_link_to_own_rows_despite_ranked_order() {
    // Same rule/line/col on two paths whose finding anchors slug-collide
    // (`-` vs `/` both map to `-`): distinct ids sharing one bare anchor.
    let mut doc = base_doc();
    doc.files = vec![file_entry("a-b.c", 10), file_entry("a/b.c", 10)];
    doc.scan.file_count = 2;
    doc.findings = vec![
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a-b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a/b.c"),
    ];
    let id_lo = "PERF-C-ALLOC-001:a-b.c:7:9";
    let id_hi = "PERF-C-ALLOC-001:a/b.c:7:9";
    assert_eq!(doc.findings[0].finding_id, id_lo);
    assert_eq!(doc.findings[1].finding_id, id_hi);
    assert!(
        id_lo < id_hi,
        "test needs `-` (0x2D) to sort before `/` (0x2F)"
    );
    // Cost flips the ranked file order (degree 0 = 0, cost 50 > 0): the `/`
    // file renders FIRST while its finding_id sorts SECOND.
    doc.stats.functions = vec![costly_fn("a/b.c")];

    // C-split: file rows stay in report.html, finding rows live in the
    // detail artifact (same anchor map — pairing is cross-page).
    let html = render_html(&doc, &[]);
    let detail = render_findings_detail_html(&doc);
    let files = section(&html, "<section id=\"files\">", "</section>");
    let findings = section_nested(&detail, "<section id=\"findings\">");

    // Precondition: ranked-file order really disagrees with finding_id order.
    let pos_hi_file = files.find("data-path=\"a/b.c\"").expect("hi file row");
    let pos_lo_file = files.find("data-path=\"a-b.c\"").expect("lo file row");
    assert!(
        pos_hi_file < pos_lo_file,
        "ranked order not flipped — lock precondition broken:\n{files}"
    );
    // Findings table keeps finding_id-ascending order: the `-` row owns the
    // bare slug, the `/` row the `-2` suffixed anchor.
    let bare = "finding-PERF-C-ALLOC-001-a-b-c-7-9";
    let suffixed = "finding-PERF-C-ALLOC-001-a-b-c-7-9-2";
    let owners = row_paths_by_id(findings);
    assert_eq!(
        owners.get(bare),
        Some(&"a-b.c".to_string()),
        "bare anchor owned by the wrong row: {owners:?}"
    );
    assert_eq!(
        owners.get(suffixed),
        Some(&"a/b.c".to_string()),
        "suffixed anchor owned by the wrong row: {owners:?}"
    );
    let pos_lo_row = findings.find(bare).expect("bare row");
    let pos_hi_row = findings.find(suffixed).expect("suffixed row");
    assert!(
        pos_lo_row < pos_hi_row,
        "findings table not in finding_id order:\n{findings}"
    );
    // Row↔link pairing: each file row's drill-down href resolves to the
    // findings row with the SAME data-path (not merely to a live anchor).
    let links = file_row_links(files);
    assert_eq!(
        links.len(),
        2,
        "one drill-down link per file row: {links:?}"
    );
    for (file_path, target) in &links {
        let owner = owners
            .get(target)
            .unwrap_or_else(|| panic!("drill-down href #{target} has no findings row"));
        assert_eq!(
            owner, file_path,
            "file row {file_path} links to #{target} owned by {owner}"
        );
    }
    // Exact pairing (the old queue-pop order handed the first-rendered file
    // the bare anchor): `-` file → bare, `/` file → `-2`.
    for (file_path, target) in &links {
        let expected = if file_path == "a-b.c" { bare } else { suffixed };
        assert_eq!(
            target, expected,
            "file row {file_path} links to #{target}, expected #{expected}"
        );
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

    // C-split: the full §4-parity table lives in findings-detail.html;
    // report.html carries groups only.
    let html = render_html(&doc, &[]);
    let detail = render_findings_detail_html(&doc);
    let report_findings = section(&html, "<section id=\"findings\">", "</section>");
    assert_eq!(
        report_findings.matches("<tr id=\"finding-").count(),
        0,
        "report.html must carry zero per-finding rows"
    );
    let table = section_nested(&detail, "<section id=\"findings\">");

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
// 15. C-full-split follow-up: component index caps at top-10 with honest
// counts (same defect class as the #files take(10) precedent)
// ---------------------------------------------------------------------------

#[test]
fn component_index_caps_at_top_ten_with_honest_note() {
    let mut doc = base_doc();
    doc.components = Some(
        (1..=12)
            .map(|r| component(r, &format!("c{r:02}")))
            .collect(),
    );
    let html = render_html(&doc, &[]);
    let comps = section(&html, "<section id=\"components\">", "</section>");
    assert!(
        comps.contains("<p>showing top 10 of 12 components</p>"),
        "honest cap note missing:\n{comps}"
    );
    assert_eq!(
        comps.matches("<tr id=\"component-").count(),
        10,
        "component index must cap at 10 rows:\n{comps}"
    );
    // Rank order preserved: c01 first, c10 last; c11/c12 absent.
    let first = comps.find(">c01<").expect("c01 row");
    let tenth = comps.find(">c10<").expect("c10 row");
    assert!(first < tenth, "rank order broken:\n{comps}");
    assert!(
        !comps.contains(">c11<") && !comps.contains(">c12<"),
        "rows beyond top-10 leaked:\n{comps}"
    );
    // Under-cap docs stay honest; component-less docs carry no note.
    let mut small = base_doc();
    small.components = Some(vec![component(1, "c01"), component(2, "c02")]);
    let small_html = render_html(&small, &[]);
    let small_comps = section(&small_html, "<section id=\"components\">", "</section>");
    assert!(small_comps.contains("<p>showing top 2 of 2 components</p>"));
    let none_html = render_html(&base_doc(), &[]);
    let none_comps = section(&none_html, "<section id=\"components\">", "</section>");
    assert!(none_comps.contains("No components recorded."));
    assert!(!none_comps.contains("showing top"));
}

// ---------------------------------------------------------------------------
// 16. C-full-split follow-up: per-file finding links cap at top-10 by
// severity-rank with an honest `…+N more` marker
// ---------------------------------------------------------------------------

/// One finding of `rule_id` at `path:line:9` (distinct finding_ids within
/// one file via the line number).
fn finding_at_line(rule_id: &str, severity: Severity, path: &str, line: u32) -> FindingEntry {
    let mut f = finding_at(rule_id, severity, Confidence::High, path);
    f.span = Span::point(line, 9);
    f.finding_id = format!("{rule_id}:{path}:{line}:9");
    f
}

/// Link text of every drill-down finding link in a `#files` section, in
/// render order.
fn drilldown_link_texts(files_section: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = files_section;
    while let Some(i) = rest.find("findings-detail.html#finding-") {
        let text_start = rest[i..].find('>').expect("link text open") + i + 1;
        let text_end = rest[text_start..].find("</a>").expect("link close") + text_start;
        out.push(rest[text_start..text_end].to_string());
        rest = &rest[text_end..];
    }
    out
}

#[test]
fn file_drilldown_caps_finding_links_at_top_ten_by_severity_rank() {
    // 13 findings in one file: hot findings sit at HIGH lines (11-13) so
    // finding_id order alone would bury them — severity-rank must win.
    let mut findings: Vec<FindingEntry> = (1..=8)
        .map(|line| finding_at_line("PERF-C-LOOP-001", Severity::Info, "big.c", line))
        .collect();
    findings.extend(
        [9, 10].map(|line| finding_at_line("PERF-C-LOOP-001", Severity::Warn, "big.c", line)),
    );
    findings.extend(
        [11, 12, 13].map(|line| finding_at_line("PERF-C-LOOP-001", Severity::Hot, "big.c", line)),
    );
    let mut doc = base_doc();
    doc.files = vec![file_entry("big.c", 100)];
    doc.scan.file_count = 1;
    doc.findings = findings;
    let html = render_html(&doc, &[]);
    let files = section(&html, "<section id=\"files\">", "</section>");

    // Exactly 10 links + an honest `…+3 more` marker.
    assert_eq!(
        files.matches("findings-detail.html#finding-").count(),
        10,
        "per-file links must cap at 10:\n{files}"
    );
    assert!(
        files.contains("…+3 more"),
        "honest hidden-link marker missing:\n{files}"
    );
    // Severity-rank order: hot (11-13), then warn (10, 9 — finding_id
    // string order within rank), then info (1-5); lines 6-8 hidden.
    let texts = drilldown_link_texts(files);
    let expected: Vec<String> = [11, 12, 13, 10, 9, 1, 2, 3, 4, 5]
        .iter()
        .map(|line| format!("PERF-C-LOOP-001:big.c:{line}:9"))
        .collect();
    assert_eq!(texts, expected, "per-file links not in severity-rank order");
    for hidden in [6, 7, 8] {
        assert!(
            !files.contains(&format!("big.c:{hidden}:9")),
            "line {hidden} leaked past the cap:\n{files}"
        );
    }
    // Every SHOWN href still resolves in the detail artifact (the cap only
    // removes links — full data stays in findings-detail.html).
    let detail = render_findings_detail_html(&doc);
    let detail_ids = collect_ids(&detail);
    for href in collect_hrefs(&html) {
        if let Some(hash) = href.find('#')
            && href.starts_with("findings-detail.html#")
        {
            let target = &href[hash + 1..];
            assert!(
                detail_ids.iter().any(|id| id == target),
                "capped link {href} has no matching id anchor"
            );
        }
    }
    assert_eq!(
        detail.matches("<tr id=\"finding-").count(),
        13,
        "detail artifact must keep every finding uncapped"
    );
    // Determinism: shuffled input order ⇒ byte-identical section.
    let mut shuffled = doc;
    shuffled.findings.reverse();
    let html_b = render_html(&shuffled, &[]);
    let files_b = section(&html_b, "<section id=\"files\">", "</section>").to_string();
    assert_eq!(
        files, files_b,
        "capped drill-down must be input-order independent"
    );
}

#[test]
fn file_drilldown_shows_no_marker_when_links_fit() {
    let mut doc = base_doc();
    doc.files = vec![file_entry("big.c", 100)];
    doc.scan.file_count = 1;
    doc.findings = vec![
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "big.c"),
        finding_at(
            "PERF-C-STR-002",
            Severity::Warn,
            Confidence::Medium,
            "big.c",
        ),
    ];
    let html = render_html(&doc, &[]);
    let files = section(&html, "<section id=\"files\">", "</section>");
    assert_eq!(
        files.matches("findings-detail.html#finding-").count(),
        2,
        "all links must render when under cap:\n{files}"
    );
    assert!(
        !files.contains("more"),
        "marker must not appear when nothing is hidden:\n{files}"
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
    let first_detail_html =
        fs::read(out.join("findings-detail.html")).expect("findings-detail.html emitted");
    let first_detail_md =
        fs::read(out.join("findings-detail.md")).expect("findings-detail.md emitted");
    assert_eq!(run_with(argv(&out_str, &target)), 0);
    let second = fs::read(out.join("report.html")).expect("report.html emitted");
    let second_detail_html = fs::read(out.join("findings-detail.html")).expect("detail emitted");
    let second_detail_md = fs::read(out.join("findings-detail.md")).expect("detail md emitted");

    assert_eq!(
        first, second,
        "report.html diverged between scans (must be byte-identical UNMASKED)"
    );
    // C-split: detail artifacts carry no provenance — byte-identical UNMASKED.
    assert_eq!(
        first_detail_html, second_detail_html,
        "findings-detail.html diverged between scans (must be byte-identical UNMASKED)"
    );
    assert_eq!(
        first_detail_md, second_detail_md,
        "findings-detail.md diverged between scans (must be byte-identical UNMASKED)"
    );
    let text = String::from_utf8(first).expect("report.html is UTF-8");
    // Slice-2 fixed script block present and byte-identical to the source.
    assert_eq!(report_script_body(&text), report_script());
    assert!(text.ends_with('\n'));
    assert!(!text.contains(&target), "absolute scan root leaked");
    let detail_text = String::from_utf8(first_detail_html).expect("detail HTML is UTF-8");
    assert_eq!(script_body(&detail_text), report_script());
    assert!(
        !detail_text.contains(&target),
        "absolute scan root leaked into detail"
    );
}

// ---------------------------------------------------------------------------
// 6b. `--report-width` is a layout-only knob
// ---------------------------------------------------------------------------

/// Drop every `<style>…</style>` element so only non-CSS markup remains.
fn without_style_blocks(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(open) = rest.find("<style>") {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        let close = after.find("</style>").expect("style close") + "</style>".len();
        rest = &after[close..];
    }
    out.push_str(rest);
    out
}

#[test]
fn report_width_changes_only_the_layout_surface() {
    let mut doc = base_doc();
    doc.components = Some(vec![component(1, "src")]);
    let charts = vec![
        "include-graph.svg".to_string(),
        "layer-workspace.svg".to_string(),
    ];
    let story = atlas_story::build(&doc, None);

    let auto = render_html_with_story_at_width(&doc, &charts, &story, ReportWidth::Auto);
    assert_eq!(
        auto,
        render_html(&doc, &charts),
        "Auto must equal render_html"
    );
    assert!(auto.contains("--report-max:100vw"));
    assert!(auto.contains("<body data-report-width=\"auto\">"));
    let auto_payload = story_data(&auto).to_string();
    let auto_scripts = scripts(&auto).len();
    let auto_svgs = auto.matches("<svg").count();
    let auto_markup = without_style_blocks(&auto);

    for (width, marker, css) in [
        (ReportWidth::W1280, "1280", "1280px"),
        (ReportWidth::W1440, "1440", "1440px"),
        (ReportWidth::W1920, "1920", "1920px"),
        (ReportWidth::W2560, "2560", "2560px"),
        (ReportWidth::W3072, "3072", "3072px"),
    ] {
        let html = render_html_with_story_at_width(&doc, &charts, &story, width);
        assert!(
            html.contains(&format!("--report-max:{css}")),
            "expected --report-max:{css} for {width:?}"
        );
        assert!(
            html.contains(&format!("<body data-report-width=\"{marker}\">")),
            "expected exact marker data-report-width=\"{marker}\" for {width:?}"
        );
        assert_eq!(html.matches("<script").count(), 3, "A12: three scripts");
        assert_eq!(scripts(&html).len(), auto_scripts);
        assert_eq!(html.matches("<svg").count(), auto_svgs, "no extra SVG");
        assert_eq!(
            story_data(&html),
            auto_payload.as_str(),
            "story data changed"
        );
        assert_eq!(story_script(&html), story_script(&auto));
        assert_eq!(report_script_body(&html), report_script());
        // Non-style markup is identical: only the CSS custom property and the
        // body width marker may differ.
        let normalized = without_style_blocks(&html).replace(
            &format!("data-report-width=\"{marker}\""),
            "data-report-width=\"auto\"",
        );
        assert_eq!(
            normalized, auto_markup,
            "width {css} changed markup outside the layout surface"
        );
    }
}

// ---------------------------------------------------------------------------
// 11. Slice-2 locks: banned script APIs absent from the emitted page
// ---------------------------------------------------------------------------

#[test]
fn enhancement_script_contains_no_banned_apis() {
    let mut doc = base_doc();
    doc.files = vec![file_entry("b.c", 30), file_entry("a.c", 10)];
    doc.scan.file_count = 2;
    doc.findings = vec![
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
    ];
    doc.components = Some(vec![component(1, "src")]);
    let charts = vec![
        "layer-workspace.svg".to_string(),
        "layer-file.svg".to_string(),
        "layer-entry.svg".to_string(),
        "layer-bands.svg".to_string(),
    ];
    let html = render_html(&doc, &charts);
    let script = report_script_body(&html);
    let story = story_script(&html);
    let executable = format!("{script}\n{story}");

    // Hard JS safety rules scoped to EXECUTABLE scripts (ADR-0014: JSON data
    // content is not code).
    for banned in [
        "Date(",
        "random(",
        "fetch(",
        "XMLHttpRequest",
        "eval(",
        "Function(",
        "setTimeout",
        "setInterval",
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
    ] {
        assert!(
            !executable.contains(banned),
            "banned API {banned:?} present in an executable script"
        );
    }
    // No inline event-handler ATTRIBUTES anywhere in the template
    // (all binding is via addEventListener in the one script block).
    // Scan attribute positions only (`="`), so handler *method* names in
    // the script (addEventListener("click"…)) do not false-positive.
    let mut rest = html.as_str();
    while let Some(i) = rest.find('=') {
        let attr_start = rest[..i].rfind([' ', '<']).map_or(0, |j| j + 1);
        let attr = &rest[attr_start..i];
        assert!(
            !attr.to_lowercase().starts_with("on"),
            "inline event-handler attribute {attr:?} in report.html"
        );
        rest = &rest[i + 1..];
    }
    assert!(script.contains("addEventListener"));
    assert!(script.contains("textContent"));
    // No data duplication for JS: the script carries no finding payload —
    // filtering runs over existing DOM ids/anchors only.
    assert!(!script.contains("finding_id"));
    assert!(!script.contains("PERF-C-"));
}

// ---------------------------------------------------------------------------
// 12. Slice-2 locks: click-to-filter controls + DOM hooks present
// ---------------------------------------------------------------------------

#[test]
fn click_to_filter_controls_and_hooks_present() {
    let mut doc = base_doc();
    doc.files = vec![file_entry("b.c", 30), file_entry("a.c", 10)];
    doc.scan.file_count = 2;
    doc.findings = vec![
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
    ];
    doc.components = Some(vec![component(1, "src")]);
    let html = render_html(&doc, &["include-graph.svg".to_string()]);

    // Visible filter bar with a reset control (hidden until a filter applies)
    // on BOTH pages (the detail page reuses the bar so click-to-filter keeps
    // working over its rows).
    let detail = render_findings_detail_html(&doc);
    for (page, name) in [(&html, "report.html"), (&detail, "detail")] {
        assert!(
            page.contains("<div id=\"filter-bar\">"),
            "filter bar missing in {name}"
        );
        assert!(
            page.contains("<span id=\"filter-label\">all findings</span>"),
            "filter label missing in {name}"
        );
        assert!(
            page.contains("<button id=\"filter-clear\" type=\"button\" hidden>clear</button>"),
            "filter clear missing in {name}"
        );
    }
    // Detail findings rows carry the filter key (escaped at render time).
    assert!(detail.contains("data-path=\"a.c\""));
    assert!(detail.contains("data-path=\"b.c\""));
    // File + component rows carry the click-source keys.
    let files = section(&html, "<section id=\"files\">", "</section>");
    assert!(files.contains("data-path=\"a.c\""));
    let comps = section(&html, "<section id=\"components\">", "</section>");
    assert!(comps.contains("data-prefix=\"src\""));
}

// ---------------------------------------------------------------------------
// 13. Slice-2 locks: no-JS content completeness (all rows present static)
// ---------------------------------------------------------------------------

#[test]
fn all_content_present_without_javascript() {
    let mut doc = base_doc();
    doc.files = vec![file_entry("b.c", 30), file_entry("a.c", 10)];
    doc.scan.file_count = 2;
    doc.findings = vec![
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding_at("PERF-C-LOOP-001", Severity::Info, Confidence::Low, "a.c"),
    ];
    doc.components = Some(vec![component(1, "src")]);
    let html = render_html(&doc, &["include-graph.svg".to_string()]);
    let detail = render_findings_detail_html(&doc);

    // C-split: the script only ENHANCES — every row is already in the
    // static markup of the page that owns it (groups in report.html, rows
    // in the detail artifact).
    let report_findings = section(&html, "<section id=\"findings\">", "</section>");
    assert_eq!(
        report_findings.matches("<details>").count(),
        3,
        "every rule group must render statically (no-JS degradation)"
    );
    assert_eq!(
        report_findings.matches("<tr id=\"finding-").count(),
        0,
        "report.html carries zero rows post-split"
    );
    let table = section_nested(&detail, "<section id=\"findings\">");
    assert_eq!(
        table.matches("<tr id=\"finding-").count(),
        3,
        "every finding must render statically in the detail page (no-JS degradation)"
    );
    // No row is hidden by default — filtering only ever hides at runtime
    // (the single static `hidden` is the clear-button reset control).
    for (page_section, name) in [(&report_findings, "report"), (&table, "detail")] {
        assert_eq!(
            page_section.matches("hidden").count(),
            1,
            "only the filter-clear button may carry `hidden` statically in {name}"
        );
    }
    let files = section(&html, "<section id=\"files\">", "</section>");
    assert_eq!(files.matches("<tr id=\"file-").count(), 2);
    // Figures + captions render without JS.
    assert!(html.contains("<figure class=\"pz\""));
    assert!(html.contains("<figcaption>"));
    // Grouped order (severity-rank → count-desc → rule_id-asc): the hot
    // ALLOC group folds first, then warn STR, then info LOOP — in both pages.
    for (page_section, name) in [(&report_findings, "report"), (&table, "detail")] {
        let pos_alloc = page_section.find("PERF-C-ALLOC-001").expect("alloc group");
        let pos_str = page_section.find("PERF-C-STR-002").expect("str group");
        let pos_loop = page_section.find("PERF-C-LOOP-001").expect("loop group");
        assert!(
            pos_alloc < pos_str && pos_str < pos_loop,
            "order wrong in {name}"
        );
    }
}

// ---------------------------------------------------------------------------
// 14. Group-and-fold locks: per-rule <details> groups, honest header,
// exemplar links, determinism, no-JS completeness
// ---------------------------------------------------------------------------

#[test]
fn findings_fold_into_per_rule_details_with_honest_summary() {
    let mut doc = base_doc();
    doc.files = vec![file_entry("b.c", 30), file_entry("a.c", 10)];
    doc.scan.file_count = 2;
    doc.findings = vec![
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::Medium, "b.c"),
    ];
    let html = render_html(&doc, &["include-graph.svg".to_string()]);
    let detail = render_findings_detail_html(&doc);
    let findings = section(&html, "<section id=\"findings\">", "</section>");

    // Honest header: 2 rules, 3 of 3 total — mirrored in the detail page.
    for (page_section, name) in [
        (&findings, "report"),
        (
            &section_nested(&detail, "<section id=\"findings\">"),
            "detail",
        ),
    ] {
        assert!(
            page_section.contains("<p>2 rules: 3 of 3 total findings (2 hot, 1 warn, 0 info).</p>"),
            "honest-counts header missing in {name}:\n{page_section}"
        );
    }
    // One <details> group per firing rule, hottest group first.
    assert_eq!(
        findings.matches("<details>").count(),
        2,
        "one details group per rule:\n{findings}"
    );
    let pos_alloc = findings.find("PERF-C-ALLOC-001").expect("alloc group");
    let pos_str = findings.find("PERF-C-STR-002").expect("str group");
    assert!(pos_alloc < pos_str, "hot group must fold first");
    // Summaries carry rule + count + severity split + exemplar link.
    assert!(
        findings.contains(
            "<summary><code>PERF-C-ALLOC-001</code> — 2 findings (2 hot, 0 warn, 0 info);"
        ),
        "alloc summary shape wrong:\n{findings}"
    );
    assert!(
        findings
            .contains("<summary><code>PERF-C-STR-002</code> — 1 findings (0 hot, 1 warn, 0 info);"),
        "str summary shape wrong:\n{findings}"
    );
    // C-split: zero rows in report.html; every row present statically in
    // the detail page, exactly once.
    assert_eq!(
        findings.matches("<tr id=\"finding-").count(),
        0,
        "report.html must carry zero rows:\n{findings}"
    );
    let detail_findings = section_nested(&detail, "<section id=\"findings\">");
    assert_eq!(
        detail_findings.matches("<tr id=\"finding-").count(),
        3,
        "every finding row must render statically in the detail page:\n{detail_findings}"
    );
    // Exemplar is the first finding in finding_id order: the a.c ALLOC row
    // (finding_id `PERF-C-ALLOC-001:a.c:7:9`) sorts before the b.c one.
    let summary_end = pos_alloc
        + findings[pos_alloc..]
            .find("</summary>")
            .expect("summary close");
    let summary = &findings[pos_alloc..summary_end];
    assert!(
        summary.contains("PERF-C-ALLOC-001:a.c:7:9"),
        "exemplar must be first in finding_id order:\n{summary}"
    );
    // Exemplar links point cross-page into the detail artifact…
    assert!(
        summary.contains("findings-detail.html#finding-"),
        "exemplar must link the detail page:\n{summary}"
    );
    assert!(
        summary.contains("findings-detail.html#rule-PERF-C-ALLOC-001"),
        "group detail link missing:\n{summary}"
    );
    // …and every cross-page href resolves to a rendered id.
    let pages = cross_page_ids(&doc);
    let report_ids = collect_ids(&html);
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
    // No row hidden by default; groups fold via <details>, not `hidden`.
    assert_eq!(
        findings.matches("hidden").count(),
        1,
        "only the filter-clear button may carry `hidden` statically"
    );
}

#[test]
fn grouped_findings_double_render_byte_identical() {
    // Shuffled input order ⇒ byte-identical findings section (groups and
    // exemplars depend only on document content, never on input order) —
    // on BOTH the human page and the detail artifact.
    let mut doc_a = base_doc();
    doc_a.findings = vec![
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
    ];
    let mut doc_b = base_doc();
    doc_b.findings = vec![
        finding_at("PERF-C-ALLOC-001", Severity::Hot, Confidence::High, "a.c"),
        finding_at("PERF-C-STR-002", Severity::Warn, Confidence::Medium, "b.c"),
    ];
    assert_eq!(
        render_html(&doc_a, &[]),
        render_html(&doc_b, &[]),
        "grouped #findings must be byte-identical regardless of input order"
    );
    assert_eq!(
        render_findings_detail_html(&doc_a),
        render_findings_detail_html(&doc_b),
        "detail #findings must be byte-identical regardless of input order"
    );
}
