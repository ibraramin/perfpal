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
use perfpal::reporter::{escape_html, render_html, report_script};
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
    // No live scriptable markup from the payload (the page's own fixed
    // slice-2 script block is the only `<script>` allowed — see the
    // fixed-script test below).
    assert!(
        !html.contains("<script>alert"),
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
// 3. Fixed enhancement script + LINKED (not inline) charts (slice 2)
// ---------------------------------------------------------------------------

/// Extract the single inline `<script>` block body (between `<script>\n` and
/// `\n</script>`), asserting there is exactly one script block.
fn script_body(html: &str) -> &str {
    assert_eq!(
        html.matches("<script>").count(),
        1,
        "exactly one inline script block expected"
    );
    assert_eq!(
        html.matches("</script>").count(),
        1,
        "exactly one inline script close expected"
    );
    let from = html.find("<script>").unwrap() + "<script>".len();
    let to = html.find("</script>").unwrap();
    html[from..to].trim_matches('\n')
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
        "component-01-src.svg".to_string(),
    ];
    let html = render_html(&doc, &charts);

    // Exactly one inline script block, byte-identical to the fixed source
    // (deterministic output — no timestamps/ids/counters in generated code).
    assert_eq!(script_body(&html), report_script());
    assert!(report_script().lines().count() < 60);
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
    // LINKED variant: relative links, no inlined SVG content.
    assert!(html.contains("<img src=\"charts/include-graph.svg\""));
    assert!(html.contains("<a href=\"charts/include-graph.svg\""));
    assert!(!html.contains("<svg"));
    // Pan/zoom figures carry the enhancement hook (buttons are added by the
    // script at runtime, so no-JS output stays fully usable static).
    assert!(html.contains("<figure class=\"pz\" data-chart=\"layer-workspace\">"));
    assert!(html.contains("<figure class=\"pz\" data-chart=\"layer-file\">"));
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

    // Grouped findings order: hottest rule-group first (ALLOC hot), then
    // warn (STR), then info (LOOP) — the pinned severity-rank → count-desc →
    // rule_id-asc group order; members within a group stay finding_id-asc.
    let table = section(&html, "<section id=\"findings\">", "</section>");
    let pos_alloc = table.find("PERF-C-ALLOC-001").expect("alloc row");
    let pos_str = table.find("PERF-C-STR-002").expect("str row");
    let pos_loop = table.find("PERF-C-LOOP-001").expect("loop row");
    assert!(
        pos_alloc < pos_str && pos_str < pos_loop,
        "findings not in pinned group order:\n{table}"
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
    }
}

/// Value of attribute `name="…"` in a tag/row fragment.
fn attr_value(fragment: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let from = fragment.find(&needle)? + needle.len();
    let to = fragment[from..].find('"')?;
    Some(fragment[from..from + to].to_string())
}

/// `(data-path, href-target)` for every `href="#…"` in each `<tr …>` row of
/// a section, in row order (file drill-down rows carry the finding links).
fn file_row_links(files_section: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for chunk in files_section.split("<tr ").skip(1) {
        let row = &chunk[..chunk.find("</tr>").expect("row close")];
        let path = attr_value(row, "data-path").expect("file row data-path");
        let mut rest = row;
        while let Some(i) = rest.find("href=\"#") {
            let from = i + "href=\"#".len();
            let to = rest[from..].find('"').expect("href close");
            out.push((path.clone(), rest[from..from + to].to_string()));
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

    let html = render_html(&doc, &[]);
    let files = section(&html, "<section id=\"files\">", "</section>");
    let findings = section(&html, "<section id=\"findings\">", "</section>");

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
    // Slice-2 fixed script block present and byte-identical to the source.
    assert_eq!(script_body(&text), report_script());
    assert!(text.ends_with('\n'));
    assert!(!text.contains(&target), "absolute scan root leaked");
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
    ];
    let html = render_html(&doc, &charts);
    let script = script_body(&html);

    // Hard JS safety rules: none of these may appear in the emitted page
    // (script or markup).
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
            !html.contains(banned),
            "banned API {banned:?} present in report.html"
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

    // Visible filter bar with a reset control (hidden until a filter applies).
    assert!(html.contains("<div id=\"filter-bar\">"));
    assert!(html.contains("<span id=\"filter-label\">all findings</span>"));
    assert!(html.contains("<button id=\"filter-clear\" type=\"button\" hidden>clear</button>"));
    // Findings rows carry the filter key (escaped at render time).
    assert!(html.contains("data-path=\"a.c\""));
    assert!(html.contains("data-path=\"b.c\""));
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

    // The script only ENHANCES: every row is already in the static markup.
    let table = section(&html, "<section id=\"findings\">", "</section>");
    assert_eq!(
        table.matches("<tr id=\"finding-").count(),
        3,
        "every finding must render statically (no-JS degradation)"
    );
    // No row is hidden by default — filtering only ever hides at runtime
    // (the single static `hidden` is the clear-button reset control).
    assert_eq!(
        table.matches("hidden").count(),
        1,
        "only the filter-clear button may carry `hidden` statically"
    );
    let files = section(&html, "<section id=\"files\">", "</section>");
    assert_eq!(files.matches("<tr id=\"file-").count(), 2);
    // Figures + captions render without JS.
    assert!(html.contains("<figure class=\"pz\""));
    assert!(html.contains("<figcaption>"));
    // Grouped order (severity-rank → count-desc → rule_id-asc): the hot
    // ALLOC group folds first, then warn STR, then info LOOP.
    let pos_alloc = table.find("PERF-C-ALLOC-001").expect("alloc row");
    let pos_str = table.find("PERF-C-STR-002").expect("str row");
    let pos_loop = table.find("PERF-C-LOOP-001").expect("loop row");
    assert!(pos_alloc < pos_str && pos_str < pos_loop);
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
    let findings = section(&html, "<section id=\"findings\">", "</section>");

    // Honest header: 2 rules, 3 of 3 total.
    assert!(
        findings.contains("<p>2 rules: 3 of 3 total findings (2 hot, 1 warn, 0 info).</p>"),
        "honest-counts header missing:\n{findings}"
    );
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
    // No-JS completeness: every row present statically, exactly once.
    assert_eq!(
        findings.matches("<tr id=\"finding-").count(),
        3,
        "every finding row must render statically:\n{findings}"
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
    // Every exemplar href="#…" resolves to a rendered row id.
    let ids = collect_ids(&html);
    for href in collect_hrefs(&html) {
        if let Some(target) = href.strip_prefix('#') {
            assert!(
                ids.iter().any(|id| id == target),
                "exemplar/drill-down link #{target} has no matching id anchor"
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
    // exemplars depend only on document content, never on input order).
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
}
