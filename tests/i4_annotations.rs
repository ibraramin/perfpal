//! I4 findings-as-annotations locks (plan `map-build`).
//!
//! Covers: finding dots (severity fill + confidence ring + count numeral +
//! `<title>` counts) on the workspace/component/file/entry/bands maps, the
//! exact-prefix component roll-up, the new `layer-findings.svg` stem
//! (weighted cap, `…+N more`, zero-findings stub, single hot outline,
//! double-render determinism), and the `report.html` breadcrumb / `#map`
//! drill path. The `findings-detail.*` artifacts are asserted to stay free
//! of the new drill markup; their byte-equality against the committed
//! goldens is enforced by `p6_hardening::report_matches_committed_goldens`.

use std::collections::BTreeMap;

use perfpal::analyzer::{Confidence, Severity};
use perfpal::charts::layers::{
    render_layer_bands, render_layer_entry, render_layer_file, render_layer_findings,
    render_layer_workspace,
};
use perfpal::charts::{self, FINDING_LEGEND, FindingIndex};
use perfpal::ir::{IncludeCounts, IncludeForm, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::{render_findings_detail, render_findings_detail_html, render_html};
use perfpal::schema::{
    ComponentEntry, ComponentMetrics, FileEntry, FindingEntry, GraphNode, GraphsSection,
    IncludeEdgeDto, IncludeGraph, MetricsDocument, ParseHealthEntry, ScanInfo, StatsSection,
    ToolInfo,
};

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

fn graphs(nodes: &[&str], pairs: &[(&str, &str)]) -> GraphsSection {
    GraphsSection {
        include_graph: IncludeGraph {
            nodes: nodes
                .iter()
                .map(|p| GraphNode {
                    path: (*p).to_string(),
                })
                .collect(),
            edges: pairs
                .iter()
                .map(|(from, to)| IncludeEdgeDto {
                    from: (*from).to_string(),
                    to: Some((*to).to_string()),
                    form: IncludeForm::Quoted,
                    resolved: true,
                    span: Span::point(1, 1),
                })
                .collect(),
        },
        call_graph: None,
    }
}

fn component(prefix: &str, rank: u32) -> ComponentEntry {
    ComponentEntry {
        component_id: prefix.to_string(),
        path_prefix: prefix.to_string(),
        rank,
        metrics: ComponentMetrics {
            complexity_sum: 0.0,
            alloc_site_density: 0.0,
            max_loop_depth: 0,
            finding_density: 0.0,
        },
        finding_count: None,
    }
}

fn file_entry(path: &str) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "c".to_string(),
        loc: LocCounts {
            code: 10,
            comment: 0,
            blank: 0,
        },
        function_count: 0,
        global_var_count: 0,
        include_counts: IncludeCounts::default(),
        parse_health: ParseHealthEntry {
            status: ParseStatus::Ok,
            error_regions: None,
            error_node_count: 0,
            resolver: None,
        },
        path_class: None,
    }
}

fn finding(
    path: &str,
    rule: &str,
    severity: Severity,
    confidence: Confidence,
    line: u32,
) -> FindingEntry {
    FindingEntry {
        finding_id: format!("{rule}:{path}:{line}:1"),
        rule_id: rule.to_string(),
        title: "candidate".to_string(),
        path: path.to_string(),
        symbol: None,
        span: Span::point(line, 1),
        confidence,
        severity,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect: false,
        error_node_overlap: false,
        threshold: None,
        message: "static candidate".to_string(),
        evidence: BTreeMap::new(),
    }
}

fn doc(
    graphs: Option<GraphsSection>,
    components: Option<Vec<ComponentEntry>>,
    findings: Vec<FindingEntry>,
) -> MetricsDocument {
    MetricsDocument {
        schema_version: perfpal::schema::SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: "test".to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-10-21T00:00:00Z".to_string(),
            duration_ms: None,
            languages: vec!["c".to_string()],
            file_count: 0,
            loc_total: None,
        },
        parse_health_summary: None,
        files: vec![],
        stats: StatsSection {
            functions: vec![],
            distributions: None,
        },
        findings,
        graphs,
        components,
    }
}

fn svg_text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("SVG must be UTF-8")
}

fn chart<'a>(charts: &'a [(String, Vec<u8>)], name: &str) -> &'a str {
    let (_, bytes) = charts
        .iter()
        .find(|(file, _)| file == name)
        .unwrap_or_else(|| panic!("{name} not emitted"));
    std::str::from_utf8(bytes).expect("SVG must be UTF-8")
}

/// Base fixture: four files, four findings (hot HIGH ×1, warn LOW ×1,
/// info LOW ×2).
fn annotated_doc() -> MetricsDocument {
    doc(
        Some(graphs(
            &["root.c", "src/a/x.c", "src/a/y.c", "src/b/z.c"],
            &[("src/a/x.c", "src/a/y.c"), ("src/a/y.c", "src/b/z.c")],
        )),
        Some(vec![component("src/a", 1), component("src/b", 2)]),
        vec![
            finding(
                "src/a/x.c",
                "PERF-C-LOOP-001",
                Severity::Hot,
                Confidence::High,
                3,
            ),
            finding(
                "src/a/x.c",
                "PERF-C-MEM-001",
                Severity::Info,
                Confidence::Low,
                9,
            ),
            finding(
                "src/a/y.c",
                "PERF-C-STR-001",
                Severity::Warn,
                Confidence::Low,
                4,
            ),
            finding(
                "src/b/z.c",
                "PERF-C-IO-001",
                Severity::Info,
                Confidence::Low,
                7,
            ),
        ],
    )
}

// ---------------------------------------------------------------------------
// 1. Dots on every annotated map stem
// ---------------------------------------------------------------------------

#[test]
fn dots_encode_worst_severity_and_lowest_confidence_per_stem() {
    let doc = annotated_doc();
    let index = FindingIndex::new(&doc.findings);
    let graphs = doc.graphs.as_ref().expect("graphs");
    let empty_functions: Vec<perfpal::schema::FunctionStats> = Vec::new();

    // Workspace: component src/a rolls up x.c(2) + y.c(1) = 3 candidates,
    // worst severity hot, LOWEST confidence LOW (1 HIGH + 2 LOW must not
    // read as thick HIGH).
    let workspace = svg_text(render_layer_workspace(
        doc.components.as_deref().unwrap(),
        Some(graphs),
        &index,
    ));
    assert!(
        workspace.contains(
            "fill=\"#c0392b\" stroke=\"#1b1b1b\" stroke-width=\"0.6\"><title>3 candidate finding(s): 1 hot / 1 warn / 1 info; confidence 1 high / 0 medium / 2 low — static candidates, profile to confirm</title>"
        ),
        "workspace dot fill/ring/title mismatch:\n{workspace}"
    );
    assert_eq!(
        workspace.matches("<circle ").count(),
        2,
        "one dot per annotated component"
    );

    // Precedence: HIGH-only ⇒ thick 2.2; HIGH+MEDIUM (no LOW) ⇒ 1.2.
    let high_g = self::graphs(&["hi.c"], &[]);
    let high_index = FindingIndex::new(&[finding(
        "hi.c",
        "PERF-C-LOOP-001",
        Severity::Hot,
        Confidence::High,
        1,
    )]);
    let high = svg_text(render_layer_file(&high_g, &empty_functions, &high_index));
    assert!(
        high.contains("fill=\"#c0392b\" stroke=\"#1b1b1b\" stroke-width=\"2.2\""),
        "HIGH-only ring must be thick:\n{high}"
    );
    let mixed_g = self::graphs(&["mix.c"], &[]);
    let mixed_index = FindingIndex::new(&[
        finding(
            "mix.c",
            "PERF-C-LOOP-001",
            Severity::Hot,
            Confidence::High,
            1,
        ),
        finding(
            "mix.c",
            "PERF-C-STR-001",
            Severity::Warn,
            Confidence::Medium,
            2,
        ),
    ]);
    let mixed = svg_text(render_layer_file(&mixed_g, &empty_functions, &mixed_index));
    assert!(
        mixed.contains("fill=\"#c0392b\" stroke=\"#1b1b1b\" stroke-width=\"1.2\""),
        "HIGH+MEDIUM ring must be medium:\n{mixed}"
    );

    // File layer: x.c has 2 candidates (numeral shown, width 170 ≥ 56).
    let file = svg_text(render_layer_file(graphs, &empty_functions, &index));
    assert!(
        file.contains(
            "fill=\"#c0392b\" stroke=\"#1b1b1b\" stroke-width=\"0.6\"><title>2 candidate finding(s): 1 hot / 0 warn / 1 info; confidence 1 high / 0 medium / 1 low — static candidates, profile to confirm</title>"
        ),
        "file dot title mismatch:\n{file}"
    );
    assert!(
        file.contains("font-size=\"7\" fill=\"#1b1b1b\">2</text>"),
        "count numeral missing for total ≥ 2:\n{file}"
    );
    // z.c is info + LOW ⇒ info fill, thin ring; root.c has no dot.
    assert!(
        file.contains("fill=\"#2e6da4\" stroke=\"#1b1b1b\" stroke-width=\"0.6\""),
        "info/LOW dot styling missing:\n{file}"
    );
    assert_eq!(
        file.matches("<circle ").count(),
        3,
        "root.c must carry no dot"
    );

    // Entry layer renders the same per-file dots.
    let entry = svg_text(render_layer_entry(graphs, &empty_functions, &index));
    assert_eq!(entry.matches("<circle ").count(), 3, "entry dots");

    // Bands: per-file dots inside the component bands.
    let files = vec![
        file_entry("src/a/x.c"),
        file_entry("src/a/y.c"),
        file_entry("src/b/z.c"),
    ];
    let bands = svg_text(render_layer_bands(
        doc.components.as_deref().unwrap(),
        Some(graphs),
        &files,
        &empty_functions,
        &index,
    ));
    assert_eq!(bands.matches("<circle ").count(), 3, "band dots");

    // Per-component page: src/a carries x.c + y.c dots.
    let pages = perfpal::charts::component::render_component_pages(
        doc.components.as_deref().unwrap(),
        Some(graphs),
        &files,
        &empty_functions,
        &index,
    );
    let page = pages
        .iter()
        .find(|(name, _)| name == "component-01-src-a.svg")
        .expect("src/a page");
    let page_text = std::str::from_utf8(&page.1).expect("SVG UTF-8");
    assert_eq!(
        page_text.matches("<circle ").count(),
        2,
        "component-page dots"
    );
}

// ---------------------------------------------------------------------------
// 2. Legend present on annotated stems, absent from the legacy include graph
// ---------------------------------------------------------------------------

#[test]
fn annotation_legend_is_on_annotated_stems_only() {
    let doc = annotated_doc();
    let charts = charts::render(&doc);
    for stem in [
        "layer-workspace.svg",
        "layer-file.svg",
        "layer-entry.svg",
        "layer-bands.svg",
        "layer-findings.svg",
        "component-01-src-a.svg",
    ] {
        let text = chart(&charts, stem);
        assert!(
            text.contains(FINDING_LEGEND),
            "annotation legend missing from {stem}:\n{text}"
        );
    }
    let include = chart(&charts, "include-graph.svg");
    assert!(
        !include.contains(FINDING_LEGEND),
        "include-graph.svg is legacy and must stay unannotated"
    );
    assert_eq!(
        include.matches("<circle ").count(),
        0,
        "include graph draws no dots"
    );
}

// ---------------------------------------------------------------------------
// 3. Component aggregation is exact-prefix only
// ---------------------------------------------------------------------------

#[test]
fn component_rollup_counts_direct_parent_only() {
    let findings = vec![
        finding(
            "src/a/x.c",
            "PERF-C-LOOP-001",
            Severity::Hot,
            Confidence::High,
            1,
        ),
        finding(
            "src/a/sub/y.c",
            "PERF-C-STR-001",
            Severity::Warn,
            Confidence::Medium,
            2,
        ),
        finding(
            "src/z.c",
            "PERF-C-IO-001",
            Severity::Info,
            Confidence::Low,
            3,
        ),
    ];
    let index = FindingIndex::new(&findings);
    // Exact direct-parent membership: `src` owns only src/z.c, `src/a` owns
    // only src/a/x.c, and the subdirectory owns its own file.
    assert_eq!(index.for_component("src").map(|d| d.total), Some(1));
    assert_eq!(index.for_component("src/a").map(|d| d.total), Some(1));
    assert_eq!(index.for_component("src/a/sub").map(|d| d.total), Some(1));
    assert_eq!(index.for_component("src/").map(|d| d.total), None);

    // Workspace proves the same mapping end-to-end.
    let doc = doc(
        Some(graphs(&["src/a/x.c", "src/a/sub/y.c", "src/z.c"], &[])),
        Some(vec![
            component("src", 1),
            component("src/a", 2),
            component("src/a/sub", 3),
        ]),
        findings,
    );
    let charts = charts::render(&doc);
    let workspace = chart(&charts, "layer-workspace.svg");
    assert!(
        workspace.contains("<title>1 candidate finding(s): 0 hot / 0 warn / 1 info"),
        "src must count only its direct child src/z.c:\n{workspace}"
    );
    assert!(
        workspace.contains("<title>1 candidate finding(s): 1 hot / 0 warn / 0 info"),
        "src/a must count only its direct child src/a/x.c:\n{workspace}"
    );
}

// ---------------------------------------------------------------------------
// 4. layer-findings.svg
// ---------------------------------------------------------------------------

#[test]
fn findings_map_is_symmetric_and_byte_deterministic() {
    let doc = annotated_doc();
    let charts = charts::render(&doc);
    let text = chart(&charts, "layer-findings.svg");
    assert!(
        text.contains("Findings map — 4 files"),
        "title missing:\n{text}"
    );
    assert!(
        text.contains("heat = severity-weighted finding density (hot 3 / warn 2 / info 1 per candidate) — profile to confirm"),
        "documented subtitle missing:\n{text}"
    );
    assert_eq!(
        charts::render(&doc),
        charts,
        "double render must be identical"
    );
    // Single hottest file (x.c: hot 3 + info 1 = 4) is outlined exactly once.
    assert_eq!(
        text.matches("stroke=\"#1a1a1a\" stroke-width=\"2\"")
            .count(),
        1,
        "exactly one hottest outline:\n{text}"
    );

    // Graphs absent ⇒ no extra stem (the file/entry layers' gate).
    let mut no_graphs = doc.clone();
    no_graphs.graphs = None;
    let names: Vec<String> = charts::render(&no_graphs)
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert!(
        !names.iter().any(|n| n == "layer-findings.svg"),
        "layer-findings must ride the graphs gate: {names:?}"
    );
}

#[test]
fn findings_map_caps_by_weight_and_reports_the_tail() {
    let mut nodes: Vec<String> = (0..60).map(|i| format!("f{i:02}.c")).collect();
    nodes.push("hot.c".to_string());
    let refs: Vec<&str> = nodes.iter().map(String::as_str).collect();
    let mut findings: Vec<FindingEntry> = refs
        .iter()
        .filter(|p| **p != "hot.c")
        .enumerate()
        .map(|(i, path)| {
            finding(
                path,
                "PERF-C-INFO-001",
                Severity::Info,
                Confidence::Low,
                u32::try_from(i + 1).unwrap(),
            )
        })
        .collect();
    findings.push(finding(
        "hot.c",
        "PERF-C-LOOP-001",
        Severity::Hot,
        Confidence::High,
        1,
    ));
    let g = graphs(&refs, &[]);
    let text = svg_text(render_layer_findings(
        Some(&g),
        &findings,
        &FindingIndex::new(&findings),
    ));
    assert!(
        text.contains("showing top 50 of 61 files by weighted findings (&#8230;+11 more)"),
        "weighted cap caption missing:\n{text}"
    );
    assert!(
        text.contains(">hot.c<"),
        "the hottest file must survive the weighted cap:\n{text}"
    );
    assert_eq!(
        text.matches("<circle ").count(),
        50,
        "one dot per shown node"
    );
}

#[test]
fn findings_map_zero_findings_is_an_honest_stub() {
    let g = graphs(&["a.c"], &[]);
    let first = svg_text(render_layer_findings(
        Some(&g),
        &[],
        &FindingIndex::default(),
    ));
    let second = svg_text(render_layer_findings(
        Some(&g),
        &[],
        &FindingIndex::default(),
    ));
    assert_eq!(first, second, "stub must be byte-deterministic");
    assert!(
        first.contains("no findings recorded — nothing to annotate (heuristic candidates only)"),
        "stub body missing:\n{first}"
    );
    assert!(first.contains("Findings map — no findings"));
}

// ---------------------------------------------------------------------------
// 5. report.html breadcrumb + #map drill path
// ---------------------------------------------------------------------------

#[test]
fn report_breadcrumb_and_map_drill_path_resolve() {
    let mut components: Vec<ComponentEntry> = (1..=12)
        .map(|rank| component(&format!("src/c{rank:02}"), rank))
        .collect();
    components.sort_by(|a, b| a.rank.cmp(&b.rank));
    let mut findings = Vec::new();
    for i in 0..12 {
        findings.push(finding(
            &format!("src/c{i:02}/f.c"),
            "PERF-C-LOOP-001",
            if i % 3 == 0 {
                Severity::Hot
            } else {
                Severity::Warn
            },
            Confidence::High,
            u32::try_from(i + 1).unwrap(),
        ));
    }
    let nodes: Vec<String> = (0..12).map(|i| format!("src/c{i:02}/f.c")).collect();
    let refs: Vec<&str> = nodes.iter().map(String::as_str).collect();
    let doc = doc(Some(graphs(&refs, &[])), Some(components), findings);
    let mut doc = doc;
    doc.files = nodes.iter().map(|p| file_entry(p)).collect();
    doc.scan.file_count = nodes.len() as u32;
    let charts = charts::render(&doc);
    let names: Vec<String> = charts.iter().map(|(n, _)| n.clone()).collect();
    let html = render_html(&doc, &names);
    let detail = render_findings_detail_html(&doc);

    // Breadcrumb after the h1, before the nav/map.
    let h1 = html.find("<h1>Perfpal Report</h1>").expect("h1");
    let crumb = html.find("class=\"breadcrumb\"").expect("breadcrumb");
    assert!(h1 < crumb);
    for anchor in ["#map", "#components", "#files"] {
        assert!(
            html.contains(&format!("<a href=\"{anchor}\">")),
            "breadcrumb anchor {anchor} missing"
        );
    }
    let crumb_line = html
        .split("class=\"breadcrumb\"")
        .nth(1)
        .expect("breadcrumb body")
        .split("</p>")
        .next()
        .expect("breadcrumb close");
    assert!(
        !crumb_line.contains("#architecture") && !crumb_line.contains("Workspace"),
        "breadcrumb must not claim #architecture is the workspace section: {crumb_line}"
    );
    // The Atlas has its own nav entry (first), then the Explorer tier.
    let nav_atlas = html
        .find("<li><a href=\"#map\">Atlas</a></li>")
        .expect("Atlas nav item");
    let nav_explorer = html
        .find("<li><a href=\"#explorer\">Explorer</a></li>")
        .expect("Explorer nav item");
    assert!(nav_atlas < nav_explorer, "Atlas must precede Explorer");
    assert!(html.contains("<li><a href=\"#explorer\">Explorer</a></li>"));
    assert!(!html.contains("<li><a href=\"#architecture\">Architecture</a></li>"));
    assert!(html.contains("<a href=\"findings-detail.html\">Findings detail</a>"));

    // Nav → #map → #explorer, exactly one #map.
    let nav = html.find("</nav>").expect("nav close");
    let map = html.find("<section id=\"map\">").expect("#map");
    let explorer = html.find("<section id=\"explorer\">").expect("#explorer");
    assert!(
        nav < map && map < explorer,
        "map/explorer section order wrong"
    );
    assert_eq!(html.matches("<section id=\"map\">").count(), 1);

    // #map opens with the canvas, then the answer line (A8 amended).
    let map_start = html.find("<section id=\"map\">").expect("#map body");
    let map_end = html[map_start..]
        .find("<section id=\"explorer\">")
        .expect("#explorer after map");
    let map_body = &html[map_start..map_start + map_end];
    assert!(
        map_body.starts_with("<section id=\"map\">\n<section id=\"atlas-canvas\""),
        "#map must open with the Atlas canvas"
    );
    let canvas = map_body.find("id=\"atlas-canvas\"").expect("canvas");
    let answer = map_body.find("id=\"atlas-answer\"").expect("answer line");
    assert!(canvas < answer, "answer line must follow the canvas");
    for block in [
        "answer", "actions", "weight", "blast", "cards", "noise", "links",
    ] {
        assert!(
            map_body.contains(&format!("data-atlas-block=\"{block}\"")),
            "missing Atlas block {block}"
        );
    }
    assert!(!map_body.contains("data-chart=\"layer-workspace\""));
    assert!(!map_body.contains(FINDING_LEGEND));

    // The eight stems render under Explorer, including the workspace figure
    // and every emitted component detail page.
    let explorer_section = html
        .split("<section id=\"explorer\">")
        .nth(1)
        .expect("#explorer body")
        .split("</section>")
        .next()
        .expect("#explorer close");
    assert!(
        explorer_section.contains("<figure class=\"pz\" data-chart=\"layer-workspace\">"),
        "workspace figure missing from #explorer"
    );
    let components_section = html
        .split("<section id=\"components\">")
        .nth(1)
        .expect("#components body")
        .split("</section>")
        .next()
        .expect("#components close");
    assert!(
        components_section.contains("showing top 10 of 12 components"),
        "honest component-index cap note missing"
    );
    for name in &names {
        if name.starts_with("component-") && name != "component-coupling.svg" {
            assert!(
                explorer_section.contains(&format!("charts/{name}")),
                "#explorer must link emitted component page {name}"
            );
        }
    }

    // Every findings-detail anchor emitted by the key views resolves in the
    // detail artifact (same finding_anchor_assignments map).
    let detail_ids = collect_ids(&detail);
    let mut checked = 0;
    let mut rest = html.as_str();
    while let Some(i) = rest.find("findings-detail.html#") {
        let tail = &rest[i + "findings-detail.html#".len()..];
        let end = tail.find('"').expect("href close");
        let anchor = &tail[..end];
        assert!(
            detail_ids.iter().any(|id| id == anchor),
            "#map link findings-detail.html#{anchor} does not resolve"
        );
        checked += 1;
        rest = &tail[end..];
    }
    assert!(checked >= 1, "expected KV1 finding links, got {checked}");
    assert!(html.contains("findings-detail.html#ranked-actions"));

    // #files rows link the emitted parent-component page.
    let files_section = html
        .split("<section id=\"files\">")
        .nth(1)
        .expect("#files section")
        .split("</section>")
        .next()
        .expect("section body");
    assert!(
        files_section.contains("charts/component-01-src-c01.svg"),
        "#files row lacks the parent component-page link:\n{files_section}"
    );

    // Exactly one fixed enhancement script; no new external references.
    assert_eq!(html.matches("<script>").count(), 1);
    assert_eq!(detail.matches("<script>").count(), 1);
    assert!(!html.contains("http://") && !html.contains("https://"));
    assert!(!detail.contains("http://") && !detail.contains("https://"));
}

#[test]
fn detail_artifacts_keep_no_new_drill_markup() {
    let doc = annotated_doc();
    let html = render_findings_detail_html(&doc);
    let md = render_findings_detail(&doc);
    assert_eq!(html, render_findings_detail_html(&doc));
    assert_eq!(md, render_findings_detail(&doc));
    assert!(!html.contains("class=\"breadcrumb\""));
    assert!(!html.contains("<section id=\"map\">"));
    assert!(!html.contains(FINDING_LEGEND));
    assert!(!md.contains(FINDING_LEGEND));
    // Per-finding rows still live here (the human page carries none).
    assert!(html.contains("<tr id=\"finding-"));
}

fn collect_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find(" id=\"") {
        let tail = &rest[i + " id=\"".len()..];
        let end = tail.find('"').expect("id close");
        ids.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    ids
}
