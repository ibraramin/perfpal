//! V4 key-view acceptance tests (map-intelligibility): KV1/KV2/KV3 ordering,
//! Explorer split, drill artifacts, determinism and provenance hygiene.
//!
//! Documents are built programmatically; the reporter is a pure function of
//! `(doc, emitted_charts)`.

use std::collections::BTreeMap;

use perfpal::analyzer::{Confidence, Severity};
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::{
    render_findings_detail, render_findings_detail_html, render_html, render_profile_queue,
    render_profile_queue_html, views,
};
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
        schema_version: "1.7".to_string(),
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
    html.split("<section id=\"map\">")
        .nth(1)
        .expect("#map")
        .split("</section>")
        .next()
        .expect("#map close")
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
fn key_views_are_the_first_blocks_in_order() {
    let doc = doc();
    let charts = perfpal::charts::render(&doc);
    let names: Vec<String> = charts.iter().map(|(name, _)| name.clone()).collect();
    let html = render_html(&doc, &names);
    let map = map_section(&html);
    let kv1 = map.find("Where do I look first?").expect("KV1 question");
    let kv2 = map
        .find("What breaks if I touch it?")
        .expect("KV2 question");
    let kv3 = map
        .find("Where might time plausibly go?")
        .expect("KV3 question");
    assert!(kv1 < kv2 && kv2 < kv3, "key-view order wrong");
    assert!(
        !map[..kv1].contains("<figure") && !map[..kv1].contains("<table"),
        "KV1 must be the first content block inside #map"
    );
    // No workspace figure / component table in #map.
    assert!(!map.contains("data-chart="));
    assert!(!map.contains("Component index"));

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
fn key_views_carry_units_anchors_and_full_rationale() {
    let doc = doc();
    let html = render_html(&doc, &[]);
    let map = map_section(&html);
    assert!(map.contains("static cost units (v2)"));
    assert!(map.contains("static cost units (v1)"));
    assert!(map.contains("% of scanned files"));
    assert!(map.contains("top "));
    assert!(map.contains("× median"));
    assert!(map.contains("dependents"));
    assert!(map.contains(
        "Ranked: production code first, then severity, then confidence, then static cost."
    ));
    let md = perfpal::reporter::render_report(&doc, &[]);
    assert!(md.contains(
        "Ranked: production code first, then severity, then confidence, then static cost."
    ));
    assert!(
        map.contains("This is the complete rationale for PERF-C-CALL-001"),
        "KV1 must show the full message"
    );
    assert!(!map.contains('…'), "key views must not truncate text");
    // KV1 suppresses component aggregates but reports them.
    assert!(!map.contains("Title for PERF-C-ARCH-002"));
    assert!(map.contains("1 component-level finding in Explorer"));
    // Honest demotion/coverage lines.
    assert!(map.contains("1 non-production file demoted"));
    assert!(map.contains("resolved 3 of 3 include edges"));
    assert!(map.contains("1 startup function demoted"));
    assert!(map.contains("1 no-loop function demoted"));
}

#[test]
fn drill_artifacts_are_complete_and_deterministic() {
    let doc = doc();
    let charts = perfpal::charts::render(&doc);
    let names: Vec<String> = charts.iter().map(|(name, _)| name.clone()).collect();
    let html = render_html(&doc, &names);

    let detail = render_findings_detail_html(&doc);
    let detail_md = render_findings_detail(&doc);
    let ranked = detail
        .split("<section id=\"ranked-actions\">")
        .nth(1)
        .expect("ranked-actions section")
        .split("</section>")
        .next()
        .expect("ranked close");
    let actions = views::complete_actions(&doc);
    assert_eq!(actions.len(), 4); // 5 findings - 1 ARCH-002
    for row in &actions {
        let finding = &doc.findings[row.finding_index];
        assert!(
            ranked.contains(&finding.finding_id),
            "ranked list missing {}",
            finding.finding_id
        );
    }
    assert!(!ranked.contains("PERF-C-ARCH-002"));
    assert!(detail_md.contains("<a id=\"ranked-actions\"></a>"));

    let queue = render_profile_queue_html(&doc);
    let queue_md = render_profile_queue(&doc);
    let all = views::complete_profile_queue(&doc);
    assert_eq!(all.len(), doc.stats.functions.len());
    for row in &all {
        assert!(
            queue.contains(&row.function),
            "profile queue missing function {}",
            row.function
        );
        assert!(queue_md.contains(&row.function.replace('_', "\\_")));
    }
    assert!(queue.contains("no-loop — demoted"));
    assert!(queue.contains("startup — demoted"));
    assert!(queue.contains("non-production — demoted"));
    assert!(queue.contains("static cost units (v1)"));
    assert!(queue.contains("unknown-trip ×1 — estimate incomplete"));
    assert!(queue.contains("known trip ×8 — verify it actually runs"));
    assert!(html.contains("profile-queue.html"));
    assert!(
        queue.contains("functions ranked by static cost — candidates, confirm with a profiler")
    );

    // Byte-identical double render, no provenance surfaces.
    assert_eq!(html, render_html(&doc, &names));
    assert_eq!(detail, render_findings_detail_html(&doc));
    assert_eq!(detail_md, render_findings_detail(&doc));
    assert_eq!(queue, render_profile_queue_html(&doc));
    assert_eq!(queue_md, render_profile_queue(&doc));
    for artifact in [&html, &detail, &detail_md, &queue, &queue_md] {
        assert!(!artifact.contains(ROOT));
        assert!(!artifact.contains("SECRET-START"));
        assert!(!artifact.contains("999999"));
    }
}

#[test]
fn kv2_labels_use_full_repo_relative_paths_and_pluralize() {
    let mut d = doc();
    d.files = vec![
        file("tmux.h", "production"),
        file("compat/imsg.h", "production"),
        file("src/user.c", "production"),
    ];
    d.stats.functions = Vec::new();
    d.findings = Vec::new();
    d.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: Vec::new(),
            edges: vec![
                edge("src/user.c", "tmux.h"),
                edge("src/user.c", "compat/imsg.h"),
            ],
        },
        call_graph: None,
    });

    let html = render_html(&d, &[]);
    let map = map_section(&html);
    assert!(
        map.contains("compat/imsg.h"),
        "nested KV2 label must keep its directory prefix"
    );
    assert!(!map.contains(">imsg.h<"), "basename-only KV2 label leaked");
    assert!(
        map.contains("1 dependent ·"),
        "singular counts must read `1 dependent`"
    );

    let md = perfpal::reporter::render_report(&d, &[]);
    assert!(md.contains("compat/imsg") && md.contains("1 dependent"));
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
    let html = render_html(&d, &[]);
    let map = map_section(&html);
    assert!(
        map.contains("same location as #1"),
        "second same-location KV1 row must be annotated"
    );
    assert_eq!(map.matches("same location as #1").count(), 1);
    let md = perfpal::reporter::render_report(&d, &[]);
    assert!(md.contains("(same location as #1)"));
    // Ranks/list are unchanged: two rows, ranks 1 and 2.
    assert_eq!(views::complete_actions(&d).len(), 2);
}
