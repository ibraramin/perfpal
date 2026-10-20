//! I3 map-layer locks (plan `map-build`): the entry-stratified cost map and
//! the component-bands middle layer.
//!
//! Documents are built programmatically (pure schema values — charts consume
//! metrics.json structures only). Every renderer is a pure function of the
//! document, so each test also locks byte-determinism across two renders.

use perfpal::analyzer::Confidence;
use perfpal::charts::FindingIndex;
use perfpal::charts::layers::{render_layer_bands, render_layer_entry};
use perfpal::ir::{IncludeCounts, IncludeForm, LocCounts, ParseStatus, Span};
use perfpal::schema::{
    CallEdge, CallGraphSection, ComponentEntry, ComponentMetrics, FileEntry, FunctionStats,
    GraphNode, GraphsSection, IncludeEdgeDto, IncludeGraph, ParseHealthEntry,
};

// ---------------------------------------------------------------------------
// Builders
// ---------------------------------------------------------------------------

fn node(path: &str) -> GraphNode {
    GraphNode {
        path: path.to_string(),
    }
}

fn include(from: &str, to: &str) -> IncludeEdgeDto {
    IncludeEdgeDto {
        from: from.to_string(),
        to: Some(to.to_string()),
        form: IncludeForm::Quoted,
        resolved: true,
        span: Span::point(1, 1),
    }
}

fn graphs(nodes: &[&str], includes: &[(&str, &str)], calls: Vec<CallEdge>) -> GraphsSection {
    GraphsSection {
        include_graph: IncludeGraph {
            nodes: nodes.iter().map(|p| node(p)).collect(),
            edges: includes
                .iter()
                .map(|(from, to)| include(from, to))
                .collect(),
        },
        call_graph: Some(CallGraphSection { edges: calls }),
    }
}

fn call(from: &str, to: &str, confidence: Confidence) -> CallEdge {
    CallEdge::with_confidence(
        format!("{from}#f#1:1"),
        "g",
        from,
        Some(to.to_string()),
        confidence,
    )
}

fn function(path: &str, entry_kind: Option<&str>, cost_v2: Option<u64>) -> FunctionStats {
    FunctionStats {
        id: format!("{path}#f#1:1"),
        name: "f".to_string(),
        path: path.to_string(),
        span: Span::point(1, 1),
        param_count: 0,
        cyclomatic_complexity: 1,
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
        entry_kind: entry_kind.map(str::to_string),
        entry_confidence: entry_kind.map(|_| "HIGH".to_string()),
        reachable_from_entry: entry_kind.map(|_| true),
        function_cost_v2: cost_v2,
        unknown_trip_loops: 0,
    }
}

fn file(path: &str) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "c".to_string(),
        loc: LocCounts::default(),
        function_count: 0,
        global_var_count: 0,
        include_counts: IncludeCounts::default(),
        parse_health: ParseHealthEntry {
            status: ParseStatus::Ok,
            error_regions: None,
            error_node_count: 0,
            resolver: None,
        },
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

fn svg_text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).expect("SVG must be UTF-8")
}

// ---------------------------------------------------------------------------
// Entry-stratified cost map
// ---------------------------------------------------------------------------

#[test]
fn entry_layer_is_deterministic_and_excludes_low_edges() {
    let g = graphs(
        &["a.c", "b.c"],
        &[],
        vec![call("a.c", "b.c", Confidence::Low)],
    );
    let functions = vec![function("a.c", Some("main"), Some(9))];
    let first = render_layer_entry(&g, &functions, &FindingIndex::default());
    let second = render_layer_entry(&g, &functions, &FindingIndex::default());
    assert_eq!(first, second, "byte-determinism violated");
    let text = svg_text(first);

    // LOW edges are excluded outright (no drawn line) and honestly counted.
    assert_eq!(text.matches("<line ").count(), 0, "LOW edge drew a line");
    assert!(
        text.contains("1 LOW-confidence call edge excluded (name-based; ADR-0001)"),
        "LOW caption missing:\n{text}"
    );
    assert!(
        text.contains("0 resolved call edges; reachability = entries only (semantic tier off)"),
        "zero-resolved caption missing:\n{text}"
    );
    // b.c is not entry-reachable over the non-LOW set.
    assert!(
        text.contains("1 file has no resolved entry path"),
        "unreached caption missing:\n{text}"
    );
    assert!(text.contains("not reached"), "unreached region missing");
}

#[test]
fn entry_layer_draws_resolved_edges_into_depth_columns() {
    let g = graphs(
        &["a.c", "b.c"],
        &[],
        vec![call("a.c", "b.c", Confidence::Medium)],
    );
    let functions = vec![
        function("a.c", Some("main"), Some(9)),
        function("b.c", None, Some(2)),
    ];
    let first = render_layer_entry(&g, &functions, &FindingIndex::default());
    assert_eq!(
        first,
        render_layer_entry(&g, &functions, &FindingIndex::default())
    );
    let text = svg_text(first);

    assert!(text.contains(">entry</text>"), "entry column:\n{text}");
    assert!(text.contains(">depth 1</text>"), "depth column:\n{text}");
    assert_eq!(text.matches("<line ").count(), 1, "resolved edge drawn");
    assert!(
        text.contains("1 resolved call edge (1 among shown) · 1 entry file"),
        "edge/entry caption missing:\n{text}"
    );
    assert!(
        !text.contains("LOW-confidence call edges excluded"),
        "no LOW caption without LOW edges:\n{text}"
    );
    // The entry file carries the hot-path outline.
    assert!(
        text.contains("stroke=\"#1a1a1a\" stroke-width=\"2\""),
        "hot-path outline missing:\n{text}"
    );
    assert!(
        text.contains("hot-path: a.c at max cost among shown entry-reachable files"),
        "hot-path caption missing:\n{text}"
    );
}

#[test]
fn entry_first_cap_keeps_a_path_late_entry_file() {
    // 60 degree-0 non-entry files plus one path-late entry file: entry-first
    // election must keep z99.c and report the honest top-50 caption.
    let mut universe: Vec<String> = (0..60).map(|i| format!("a{i:02}.c")).collect();
    universe.push("z99.c".to_string());
    let node_refs: Vec<&str> = universe.iter().map(String::as_str).collect();
    let g = graphs(&node_refs, &[], vec![]);
    let functions = vec![function("z99.c", Some("main"), Some(3))];

    let text = svg_text(render_layer_entry(&g, &functions, &FindingIndex::default()));
    assert!(
        text.contains(">z99.c<"),
        "path-late entry file was cut:\n{text}"
    );
    assert!(
        text.contains("showing top 50 of 61 files"),
        "honest cap caption missing:\n{text}"
    );
}

#[test]
fn entry_layer_empty_universe_yields_the_stub() {
    let g = graphs(&[], &[], vec![]);
    let first = render_layer_entry(&g, &[], &FindingIndex::default());
    let second = render_layer_entry(&g, &[], &FindingIndex::default());
    assert_eq!(first, second, "stub must be byte-deterministic");
    let text = svg_text(first);
    assert!(
        text.contains("Entry-stratified cost map"),
        "stub title missing:\n{text}"
    );
    assert!(text.contains("no files recorded"), "stub caption:\n{text}");
}

// ---------------------------------------------------------------------------
// Component bands
// ---------------------------------------------------------------------------

#[test]
fn bands_aggregate_cross_component_edges_into_one_counted_ribbon() {
    let comps = vec![component("src/a", 1), component("src/b", 2)];
    let files = vec![file("src/a/x.c"), file("src/a/x2.c"), file("src/b/y.c")];
    let g = graphs(
        &["src/a/x.c", "src/a/x2.c", "src/b/y.c", "src/c/outside.c"],
        &[
            ("src/a/x.c", "src/b/y.c"),
            ("src/a/x2.c", "src/b/y.c"),
            ("src/a/x.c", "src/c/outside.c"),
        ],
        vec![],
    );
    // src/c/outside.c has no shown component ⇒ one outside cross edge.
    let first = render_layer_bands(&comps, Some(&g), &files, &[], &FindingIndex::default());
    assert_eq!(
        first,
        render_layer_bands(&comps, Some(&g), &files, &[], &FindingIndex::default())
    );
    let text = svg_text(first);

    assert!(
        text.contains("1. src/a — 2 files"),
        "band header missing:\n{text}"
    );
    // Two x.c→y.c edges aggregate into ONE ribbon with count 2.
    assert_eq!(text.matches("stroke=\"#7a8ba6\"").count(), 1);
    assert!(
        text.contains("stroke-width=\"3\""),
        "aggregated ribbon width (1+min(2,5)) missing:\n{text}"
    );
    assert!(
        text.contains(">2</text>"),
        "ribbon count label missing:\n{text}"
    );
    assert!(
        text.contains("cross-component file edges aggregated (includes + resolved calls)"),
        "aggregation caption missing:\n{text}"
    );
    assert!(
        text.contains("&#8230;+1 more cross-component edges outside shown scope"),
        "outside-scope caption missing:\n{text}"
    );
    assert!(
        !text.contains("LOW-confidence call edges excluded"),
        "no LOW caption without LOW edges:\n{text}"
    );
}

#[test]
fn bands_cap_each_file_row_and_mark_the_hidden_count() {
    let comps = vec![component("src/a", 1)];
    let paths: Vec<String> = (0..12).map(|i| format!("src/a/f{i:02}.c")).collect();
    let node_refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let g = graphs(&node_refs, &[], vec![]);
    let text = svg_text(render_layer_bands(
        &comps,
        Some(&g),
        &[],
        &[],
        &FindingIndex::default(),
    ));

    assert!(
        text.contains("1. src/a — 12 files"),
        "full membership count missing:\n{text}"
    );
    assert!(
        text.contains("&#8230;+2 more files in shown bands"),
        "per-band cap caption missing:\n{text}"
    );
    // Exactly BAND_FILE_CAP file rects (plus the band background rect).
    assert_eq!(text.matches("width=\"150\" height=\"30\"").count(), 10);
}

#[test]
fn bands_fileless_components_render_stub_bands() {
    let comps = vec![component("src/empty", 1), component("src/also", 2)];
    let first = render_layer_bands(&comps, None, &[], &[], &FindingIndex::default());
    let second = render_layer_bands(&comps, None, &[], &[], &FindingIndex::default());
    assert_eq!(first, second, "stub bands must be byte-deterministic");
    let text = svg_text(first);

    assert!(
        text.contains("Component bands — 2 components"),
        "title missing:\n{text}"
    );
    assert_eq!(
        text.matches("no files recorded").count(),
        2,
        "one stub per file-less band:\n{text}"
    );
    assert!(
        !text.contains("cross-component file edges aggregated"),
        "no aggregation claim without cross-band edges:\n{text}"
    );
}

#[test]
fn entry_layer_ignores_unknown_entry_stubs() {
    // `unknown-entry` is an honesty stub, never an entry point: the layer
    // must fall back to topo and caption the stubs instead.
    let g = graphs(
        &["a.c", "b.c"],
        &[],
        vec![call("a.c", "b.c", Confidence::Medium)],
    );
    let functions = vec![function("a.c", Some("unknown-entry"), Some(1))];
    let first = render_layer_entry(&g, &functions, &FindingIndex::default());
    assert_eq!(
        first,
        render_layer_entry(&g, &functions, &FindingIndex::default())
    );
    let text = svg_text(first);

    assert!(
        text.contains("no entry points recorded; topo fallback"),
        "topo fallback caption missing:\n{text}"
    );
    assert!(
        !text.contains(">entry</text>"),
        "unknown-entry stub became an entry column:\n{text}"
    );
    assert!(
        text.contains("1 unknown-entry stub file (no entry detected; shown as not reached)"),
        "stub caption missing:\n{text}"
    );
}

#[test]
fn entry_layer_depth_axis_folds_past_cap() {
    // a0 → a1 → … → a11 (all MEDIUM): depths 0..11, so 9..11 fold into one
    // `depth ≥ 9` column and the x-axis stays bounded.
    let paths: Vec<String> = (0..12).map(|i| format!("a{i:02}.c")).collect();
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let calls: Vec<CallEdge> = paths
        .windows(2)
        .map(|pair| call(&pair[0], &pair[1], Confidence::Medium))
        .collect();
    let g = graphs(&refs, &[], calls);
    let functions = vec![function("a00.c", Some("main"), Some(1))];
    let text = svg_text(render_layer_entry(&g, &functions, &FindingIndex::default()));

    assert!(text.contains(">depth 8</text>"), "depth 8 column:\n{text}");
    assert!(
        text.contains(">depth ≥ 9</text>"),
        "folded column missing:\n{text}"
    );
    assert!(
        text.contains("3 files at depth ≥ 9 folded into one column"),
        "folded caption missing:\n{text}"
    );
    let width: i64 = text
        .split("width=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|w| w.parse().ok())
        .expect("SVG width");
    assert!(width < 3000, "depth axis not bounded: width {width}");
}

#[test]
fn entry_layer_reserve_keeps_reachable_cut_file() {
    // 60 degree-heavy chain files + entry z.c + reachable low-degree zz_r.c:
    // the entry-first cap fills all base slots, and the pre-cap BFS distance
    // reserve must still draw zz_r.c (S = 51).
    let mut paths: Vec<String> = (0..60).map(|i| format!("h{i:02}.c")).collect();
    paths.push("z.c".to_string());
    paths.push("zz_r.c".to_string());
    let mut calls: Vec<CallEdge> = Vec::new();
    for pair in paths[..60].windows(2) {
        calls.push(call(&pair[0], &pair[1], Confidence::Medium));
    }
    calls.push(call("z.c", "zz_r.c", Confidence::Medium));
    let refs: Vec<&str> = paths.iter().map(String::as_str).collect();
    let g = graphs(&refs, &[], calls);
    let functions = vec![function("z.c", Some("main"), Some(1))];
    let text = svg_text(render_layer_entry(&g, &functions, &FindingIndex::default()));

    assert!(
        text.contains(">zz_r.c<"),
        "reachable reserve file was cut:\n{text}"
    );
    assert!(
        text.contains("showing top 51 of 62 files"),
        "reserve not reflected in the cap caption:\n{text}"
    );
}

#[test]
fn bands_component_cap_and_low_caption() {
    let comps: Vec<ComponentEntry> = (0..51u32)
        .map(|i| component(&format!("c{i:02}"), i + 1))
        .collect();
    let files: Vec<FileEntry> = (0..51).map(|i| file(&format!("c{i:02}/f.c"))).collect();
    let refs: Vec<String> = (0..51).map(|i| format!("c{i:02}/f.c")).collect();
    let node_refs: Vec<&str> = refs.iter().map(String::as_str).collect();
    let g = graphs(
        &node_refs,
        &[],
        vec![call("c00/f.c", "c01/f.c", Confidence::Low)],
    );
    let text = svg_text(render_layer_bands(
        &comps,
        Some(&g),
        &files,
        &[],
        &FindingIndex::default(),
    ));

    assert!(
        text.contains("showing top 50 of 51 components"),
        "component cap caption missing:\n{text}"
    );
    assert!(
        text.contains("1 LOW-confidence call edge excluded (name-based; ADR-0001)"),
        "LOW caption missing:\n{text}"
    );
}
