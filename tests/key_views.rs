//! V4 key-view acceptance tests (map-intelligibility): KV1/KV2/KV3 ordering,
//! Explorer split, drill artifacts, determinism and provenance hygiene.
//!
//! Documents are built programmatically; the reporter is a pure function of
//! `(doc, emitted_charts)`.

use std::collections::BTreeMap;

use perfpal::analyzer::{Confidence, Severity};
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::{render_html, views};
use perfpal::schema::{
    ComponentEntry, ComponentMetrics, FileEntry, FindingEntry, FunctionStats, GraphsSection,
    IncludeEdgeDto, IncludeGraph, MetricsDocument, ParseHealthEntry, ScanInfo, StatsSection,
    ToolInfo,
};

const ROOT: &str = "/SECRET/ROOT";

fn file(path: &str, class: &str) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "c".to_string(),
        loc: LocCounts {
            code: 40,
            comment: 0,
            blank: 0,
        },
        function_count: 2,
        global_var_count: 0,
        include_counts: IncludeCounts {
            quoted: 1,
            angle: 0,
        },
        parse_health: ParseHealthEntry {
            status: ParseStatus::Ok,
            error_regions: None,
            error_node_count: 0,
            resolver: None,
        },
        path_class: Some(class.to_string()),
    }
}

#[allow(clippy::too_many_arguments)]
fn function(
    path: &str,
    name: &str,
    start: u32,
    end: u32,
    cost: Option<u64>,
    loops: u32,
    entry: Option<&str>,
    unknown_trips: u32,
    known_trip: Option<u64>,
) -> FunctionStats {
    FunctionStats {
        id: format!("{path}#{name}#{start}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span {
            start_line: start,
            start_col: 1,
            end_line: end,
            end_col: 1,
        },
        param_count: 1,
        cyclomatic_complexity: 4,
        cognitive_complexity: 2,
        max_loop_depth: loops,
        alloc_site_count: 0,
        length_lines: end - start + 1,
        call_fanout: Some(1),
        call_fanin: Some(1),
        call_depth: Some(1),
        is_recursive: Some(false),
        idiom_hash: None,
        error_path: Some(false),
        alloc_family: None,
        cfg_gated: None,
        entry_kind: entry.map(str::to_string),
        entry_confidence: entry.map(|_| "HIGH".to_string()),
        reachable_from_entry: Some(true),
        function_cost_v2: cost,
        unknown_trip_loops: unknown_trips,
        trip_count_source: None,
        known_trip_value: known_trip,
    }
}

fn finding(
    rule: &str,
    path: &str,
    line: u32,
    severity: Severity,
    confidence: Confidence,
) -> FindingEntry {
    FindingEntry {
        finding_id: format!("{rule}:{path}:{line}:1"),
        rule_id: rule.to_string(),
        title: format!("Title for {rule}"),
        path: path.to_string(),
        symbol: None,
        span: Span::point(line, 1),
        confidence,
        severity,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect: false,
        error_node_overlap: false,
        threshold: None,
        message: format!(
            "This is the complete rationale for {rule} at {path}:{line} with no truncation, including the second sentence that must remain visible."
        ),
        evidence: BTreeMap::new(),
    }
}

fn edge(from: &str, to: &str) -> IncludeEdgeDto {
    IncludeEdgeDto {
        from: from.to_string(),
        to: Some(to.to_string()),
        form: perfpal::ir::IncludeForm::Quoted,
        resolved: true,
        span: Span::point(1, 1),
    }
}

fn doc() -> MetricsDocument {
    let files = vec![
        file("src/a.c", "production"),
        file("src/b.c", "production"),
        file("src/c.c", "production"),
        file("tests/t.c", "test"),
    ];
    let functions = vec![
        function("src/a.c", "hot_loop", 1, 40, Some(900), 2, None, 1, Some(8)),
        function("src/a.c", "leaf", 50, 70, Some(600), 0, None, 0, None),
        function(
            "src/b.c",
            "main",
            1,
            20,
            Some(500),
            1,
            Some("main"),
            0,
            None,
        ),
        function("src/c.c", "v1_only", 1, 30, None, 1, None, 0, None),
        function(
            "tests/t.c",
            "test_helper",
            1,
            20,
            Some(800),
            1,
            None,
            0,
            None,
        ),
    ];
    let findings = vec![
        finding(
            "PERF-C-ALLOC-001",
            "src/a.c",
            5,
            Severity::Warn,
            Confidence::Medium,
        ),
        finding(
            "PERF-C-CALL-001",
            "src/a.c",
            10,
            Severity::Hot,
            Confidence::High,
        ),
        finding(
            "PERF-C-STR-001",
            "tests/t.c",
            5,
            Severity::Hot,
            Confidence::Low,
        ),
        finding(
            "PERF-C-ARCH-002",
            "src/a.c",
            7,
            Severity::Hot,
            Confidence::High,
        ),
        finding(
            "PERF-C-LOOP-001",
            "src/b.c",
            5,
            Severity::Info,
            Confidence::Medium,
        ),
    ];
    MetricsDocument {
        schema_version: "1.8".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: "test".to_string(),
        },
        scan: ScanInfo {
            root_path: ROOT.to_string(),
            started_at: "SECRET-START".to_string(),
            duration_ms: Some(999_999),
            languages: vec!["c".to_string()],
            file_count: files.len() as u32,
            loc_total: Some(LocCounts {
                code: 160,
                comment: 0,
                blank: 0,
            }),
        },
        parse_health_summary: None,
        files,
        stats: StatsSection {
            functions,
            distributions: None,
        },
        findings,
        graphs: Some(GraphsSection {
            include_graph: IncludeGraph {
                nodes: vec![
                    perfpal::schema::GraphNode {
                        path: "src/a.c".to_string(),
                    },
                    perfpal::schema::GraphNode {
                        path: "src/b.c".to_string(),
                    },
                    perfpal::schema::GraphNode {
                        path: "src/c.c".to_string(),
                    },
                    perfpal::schema::GraphNode {
                        path: "tests/t.c".to_string(),
                    },
                ],
                edges: vec![
                    edge("src/b.c", "src/a.c"),
                    edge("src/c.c", "src/a.c"),
                    edge("tests/t.c", "src/b.c"),
                ],
            },
            call_graph: None,
        }),
        components: Some(vec![ComponentEntry {
            component_id: "src".to_string(),
            path_prefix: "src".to_string(),
            rank: 1,
            metrics: ComponentMetrics {
                complexity_sum: 9.0,
                alloc_site_density: 0.0,
                max_loop_depth: 2,
                finding_density: 1.0,
            },
            finding_count: Some(3),
        }]),
    }
}

fn map_section(html: &str) -> &str {
    // `#map` now nests the canvas/brief/drill sections, so the section ends
    // where the sibling `#atlas-story` section begins (ADR-0014), not at the
    // first `</section>`.
    let start = html.find("<section id=\"map\">").expect("#map");
    let end = html[start..]
        .find("<section id=\"atlas-story\"")
        .expect("#atlas-story after map");
    &html[start..start + end]
}

fn explorer_section(html: &str) -> &str {
    html.split("<section id=\"explorer\">")
        .nth(1)
        .expect("#explorer")
        .split("</section>")
        .next()
        .expect("#explorer close")
}

#[test]
fn atlas_page_orders_canvas_brief_blocks_then_explorer() {
    let doc = doc();
    let charts = perfpal::charts::render(&doc);
    let names: Vec<String> = charts.iter().map(|(name, _)| name.clone()).collect();
    let html = render_html(&doc, &names);
    let map = map_section(&html);
    assert!(
        map.starts_with("<section id=\"map\">\n<div class=\"atlas-page-chrome\""),
        "the first-page chrome (masthead + key-metrics strip) must open #map"
    );
    let canvas = map.find("<section id=\"atlas-canvas\"").expect("canvas");
    let mut last = canvas;
    for block in ["weight-chart", "blast-chart", "file-chart", "coverage"] {
        let marker = format!("data-atlas-block=\"{block}\"");
        let position = map.find(&marker).unwrap_or_else(|| panic!("{block} block"));
        assert!(position > last, "{block} block out of order");
        last = position;
    }
    // The seven branch blocks are restored additively; the inline drills stay
    // out (they are standalone artifacts linked from the charts).
    let answer = map.find("id=\"atlas-answer\"").expect("answer block");
    assert!(answer < canvas, "answer block must precede the canvas");
    for block in [
        "answer",
        "weight",
        "weight-chart",
        "blast",
        "blast-chart",
        "file-chart",
        "coverage",
        "actions",
        "cards",
        "noise",
        "links",
    ] {
        assert!(
            map.contains(&format!("data-atlas-block=\"{block}\"")),
            "missing restored block {block}"
        );
    }
    assert!(!map.contains("id=\"atlas-drill-q2\""));
    assert!(!map.contains("id=\"atlas-drill-weight\""));
    assert!(map.contains("atlas-weight.html"));
    assert!(map.contains("atlas-q2.html"));
    // No workspace figure / component table / chart stems in #map.
    assert!(!map.contains("data-chart="));
    assert!(!map.contains("Component index"));

    // ADR-0014: the story section is a sibling after #map and before #explorer.
    let story = html
        .find("<section id=\"atlas-story\"")
        .expect("#atlas-story present");
    let explorer_start = html.find("<section id=\"explorer\">").expect("#explorer");
    assert!(
        html.find("<section id=\"map\">").expect("#map") < story && story < explorer_start,
        "story must sit between #map and #explorer"
    );
    assert!(
        html.contains("data-atlas-block=\"story\""),
        "story block marker missing"
    );

    let explorer = explorer_section(&html);
    for stem in [
        "include-graph",
        "layer-file",
        "layer-entry",
        "component-coupling",
        "hot-components",
        "layer-workspace",
        "layer-bands",
        "layer-findings",
    ] {
        assert!(
            explorer.contains(&format!("charts/{stem}.svg")),
            "Explorer missing emitted stem {stem}"
        );
        assert!(
            !map.contains(&format!("charts/{stem}.svg")),
            "key views must not carry the {stem} stem"
        );
    }
}

#[test]
fn atlas_charts_carry_units_anchors_and_reachable_rationale() {
    let doc = doc();
    let html = render_html(&doc, &[]);
    let map = map_section(&html);
    assert!(map.contains("static cost units (v2)"));
    assert!(map.contains("static cost units (v1)"));
    assert!(map.contains("% of scanned static cost"));
    assert!(map.contains("× median function"));
    assert!(map.contains("of functions"));
    assert!(map.contains("of files"));
    assert!(map.contains("dependent file"));
    assert!(map.contains("production first"));
    // The full rationale moved to the C-split detail artifact; the map keeps
    // the dominant-finding title and the reachable link.
    let detail = perfpal::reporter::render_findings_detail_html(&doc);
    assert!(
        detail.contains("This is the complete rationale for PERF-C-CALL-001"),
        "the full finding rationale must stay reachable in findings-detail.html"
    );
    assert!(
        map.contains("findings-detail.html#finding-"),
        "the map must link the reachable finding detail"
    );
    // The chart brief itself must not truncate text (the canvas may
    // mid-ellipsise long paths, with the full path in the node title).
    let brief = map
        .split("id=\"atlas-brief\"")
        .nth(1)
        .expect("brief body")
        .split("</section>")
        .next()
        .expect("brief close");
    assert!(
        !brief.contains('…'),
        "the chart brief must not truncate text"
    );
    assert!(
        brief.contains("Component shares include every member file"),
        "component disclosure must survive the restructure"
    );
}

#[test]
fn kv1_marks_repeated_inline_locations_without_changing_ranks() {
    let mut d = doc();
    d.findings = vec![
        finding(
            "PERF-C-ALLOC-001",
            "src/a.c",
            5,
            Severity::Hot,
            Confidence::High,
        ),
        finding(
            "PERF-C-STR-001",
            "src/a.c",
            5,
            Severity::Hot,
            Confidence::High,
        ),
    ];
    let md = perfpal::reporter::render_report(&d, &[]);
    assert!(md.contains("(same location as #1)"));
    // Ranks/list are unchanged: two rows, ranks 1 and 2.
    assert_eq!(views::complete_actions(&d).len(), 2);
}
