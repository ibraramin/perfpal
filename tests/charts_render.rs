//! Chart-emitter contract tests (ADR-0002 guardrails): byte-determinism by
//! construction, presence-based emission, XML escaping, deterministic caps
//! with in-chart truncation markers, HALF-UP rounding locks (in-module), and
//! distinct LOW-confidence call-edge styling.
//!
//! Documents are built programmatically — pure [`MetricsDocument`] values,
//! no fixtures on disk (charts consume metrics.json structures ONLY,
//! guardrail 1).

use perfpal::charts::render;
use perfpal::ir::{IncludeForm, Span};
use perfpal::schema::{
    ComponentEntry, ComponentMetrics, FunctionStats, GraphNode, GraphsSection, IncludeEdgeDto,
    IncludeGraph, MetricsDocument, SCHEMA_VERSION, ScanInfo, StatsSection, ToolInfo,
};

// ---------------------------------------------------------------------------
// Local document builders
// ---------------------------------------------------------------------------

fn base_doc() -> MetricsDocument {
    MetricsDocument {
        schema_version: SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
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
        findings: vec![],
        graphs: None,
        components: None,
    }
}

fn node(path: &str) -> GraphNode {
    GraphNode {
        path: path.to_string(),
    }
}

fn edge(from: &str, to: &str) -> IncludeEdgeDto {
    IncludeEdgeDto {
        from: from.to_string(),
        to: Some(to.to_string()),
        form: IncludeForm::Quoted,
        resolved: true,
        span: Span::point(1, 1),
    }
}

fn unresolved_edge(from: &str) -> IncludeEdgeDto {
    IncludeEdgeDto {
        from: from.to_string(),
        to: None,
        form: IncludeForm::Quoted,
        resolved: false,
        span: Span::point(2, 1),
    }
}

fn component(prefix: &str, rank: u32, density: f64) -> ComponentEntry {
    ComponentEntry {
        component_id: prefix.to_string(),
        path_prefix: prefix.to_string(),
        rank,
        metrics: ComponentMetrics {
            complexity_sum: 0.0,
            alloc_site_density: 0.0,
            max_loop_depth: 0,
            finding_density: density,
        },
        finding_count: None,
    }
}

/// Doc 1: three-node include cycle (a→b→c→a), plus an unresolved quoted edge
/// that must NOT enter the chart.
fn cycle_doc() -> MetricsDocument {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![node("a.c"), node("b.c"), node("c.c")],
            edges: vec![
                edge("a.c", "b.c"),
                edge("b.c", "c.c"),
                edge("c.c", "a.c"),
                unresolved_edge("a.c"),
            ],
        },
        call_graph: None,
    });
    doc
}

/// Doc 2: 60-node chain — exercises the 50-node cap + truncation marker.
fn big_doc() -> MetricsDocument {
    let paths: Vec<String> = (0..60).map(|i| format!("n{i:02}.c")).collect();
    let mut edges = Vec::new();
    for pair in paths.windows(2) {
        edges.push(edge(&pair[0], &pair[1]));
    }
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: paths.iter().map(|p| node(p)).collect(),
            edges,
        },
        call_graph: None,
    });
    doc
}

/// Doc 3: two components coupled by BOTH a resolved include edge and a
/// cross-file call edge (distinct styling + mandatory ADR-0001 caveat).
fn calls_doc() -> MetricsDocument {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![node("src/a/x.c"), node("src/b/y.c")],
            edges: vec![edge("src/a/x.c", "src/b/y.c")],
        },
        call_graph: Some(perfpal::schema::CallGraphSection {
            edges: vec![perfpal::schema::CallEdge::new(
                "src/a/x.c#f#1:1",
                "g",
                "src/a/x.c",
                Some("src/b/y.c".to_string()),
            )],
        }),
    });
    doc.components = Some(vec![component("src/b", 2, 4.0), component("src/a", 1, 8.0)]);
    doc
}

/// Doc 4: components whose finding densities are ALL zero.
fn zero_density_doc() -> MetricsDocument {
    let mut doc = base_doc();
    doc.components = Some(vec![
        component("src/quiet", 1, 0.0),
        component("src/calmer", 2, 0.0),
    ]);
    doc
}

/// Doc 5: two ROOT-level files including each other, plus one subdir file
/// for contrast. Root-level endpoints map to the `"."` component prefix —
/// the coupling matrix MUST count their edges (regression: the chart-local
/// parent_dir used to return "" for slash-less paths and missed every
/// root-level endpoint against the aggregator's `"."` axis).
fn root_level_doc() -> MetricsDocument {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![node("a.c"), node("b.c"), node("src/c.c")],
            edges: vec![edge("a.c", "b.c"), edge("b.c", "a.c")],
        },
        call_graph: None,
    });
    doc.components = Some(vec![component(".", 1, 2.0), component("src", 2, 0.0)]);
    doc
}

fn svg_of(chart: &[(String, Vec<u8>)], name: &str) -> String {
    let (_, bytes) = chart
        .iter()
        .find(|(file, _)| file == name)
        .unwrap_or_else(|| panic!("{name} not emitted"));
    String::from_utf8(bytes.clone()).expect("SVG must be UTF-8")
}

fn emitted_names(chart: &[(String, Vec<u8>)]) -> Vec<&str> {
    chart.iter().map(|(name, _)| name.as_str()).collect()
}

fn costly_fn(path: &str, name: &str, cyclomatic: u32) -> FunctionStats {
    FunctionStats {
        id: format!("{path}#{name}#1:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(1, 1),
        param_count: 0,
        cyclomatic_complexity: cyclomatic,
        cognitive_complexity: 0,
        max_loop_depth: 0,
        loop_count: None,
        loop_body_loc: None,
        parse_health: None,
        alloc_sites_in_loops: None,
        io_site_count: None,
        io_sites_in_loops: None,
        resource_sites: Vec::new(),
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

/// v1.4 step-2 cost overlay: the same two-edge shape renders DIFFERENTLY
/// with and without `stats.functions[]` — costly endpoints pull their cell
/// to max heat while the function-less cell stays lighter; with no functions
/// both cells sit at max (legacy count-matrix behavior, byte-identical).
fn overlay_doc(with_functions: bool) -> MetricsDocument {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![
                node("src/a/x.c"),
                node("src/b/y.c"),
                node("src/a/z.c"),
                node("src/c/w.c"),
            ],
            edges: vec![
                edge("src/a/x.c", "src/b/y.c"),
                edge("src/a/z.c", "src/c/w.c"),
            ],
        },
        call_graph: None,
    });
    doc.components = Some(vec![
        component("src/a", 1, 1.0),
        component("src/b", 2, 0.5),
        component("src/c", 3, 0.0),
    ]);
    if with_functions {
        // x.c and y.c each host a costly function (cyclomatic 20 ⇒ cost 20);
        // z.c / w.c host NONE ⇒ file avg 0. max_cost = 20:
        // w(a→b) = 1 + (20+20)/40 = 2.0, w(a→c) = 1 + 0 = 1.0.
        doc.stats.functions = vec![
            costly_fn("src/a/x.c", "hot", 20),
            costly_fn("src/b/y.c", "hot2", 20),
        ];
    }
    doc
}

#[test]
fn cost_overlay_differentiates_cells_only_with_functions() {
    let svg = svg_of(&render(&overlay_doc(true)), "component-coupling.svg");
    // The costly (src/a ↔ src/b) cell is the max ⇒ darkest ramp fill…
    assert!(
        svg.contains("fill=\"#b30000\""),
        "costly cell must render at max heat:\n{svg}"
    );
    // …while the function-less (src/a ↔ src/c) cell renders LIGHTER than
    // max — the overlay differentiated the two single-edge cells.
    let darkest = svg.matches("fill=\"#b30000\"").count();
    let total_cells = svg.matches("<rect ").count();
    assert!(
        darkest < total_cells,
        "overlay must differentiate cells (darkest {darkest} of {total_cells}):\n{svg}"
    );

    // Legacy pin: no functions ⇒ every weight exactly 1.0 ⇒ both coupled
    // cells (plus their symmetric twins) share the max and render darkest.
    let legacy = svg_of(&render(&overlay_doc(false)), "component-coupling.svg");
    assert_eq!(
        legacy.matches("fill=\"#b30000\"").count(),
        4,
        "no-functions overlay must equal the legacy count matrix:\n{legacy}"
    );
}

// ---------------------------------------------------------------------------
// 1 + 5. Byte-determinism: rendering each fixture twice ⇒ identical bytes
// ---------------------------------------------------------------------------

#[test]
fn double_render_is_byte_identical_across_all_fixtures() {
    for doc in [
        cycle_doc(),
        big_doc(),
        calls_doc(),
        zero_density_doc(),
        root_level_doc(),
    ] {
        let first = render(&doc);
        let second = render(&doc);
        assert_eq!(first, second, "byte-determinism violated");
        assert!(!first.is_empty(), "fixture documents must emit charts");
    }
}

// ---------------------------------------------------------------------------
// Chart-specific content contracts
// ---------------------------------------------------------------------------

#[test]
fn cycle_document_states_deterministic_broken_count() {
    let out = render(&cycle_doc());
    let svg = svg_of(&out, "include-graph.svg");
    assert!(
        svg.contains("1 cyclic edges broken (see ARCH-001 findings)"),
        "broken-cycle caption missing:\n{svg}"
    );
    // The broken edge (c,a) is NOT drawn: exactly 2 kept lines.
    assert_eq!(svg.matches("<line ").count(), 2);
}

#[test]
fn oversized_graph_is_capped_at_50_with_visible_marker() {
    let out = render(&big_doc());
    let svg = svg_of(&out, "include-graph.svg");
    assert_eq!(
        svg.matches("<rect ").count(),
        50,
        "node cap applied pre-layout"
    );
    assert!(
        svg.contains("showing top 50 of 60 files"),
        "truncation marker missing:\n{svg}"
    );
}

#[test]
fn call_edges_fold_into_single_heat_fill_with_low_confidence_legend() {
    let out = render(&calls_doc());
    let svg = svg_of(&out, "component-coupling.svg");
    // Exactly one fill rect per cell (n² = 4): no full-cell call overlay may
    // bury the heat color (a same-size blue@morphism over red reads purple).
    assert_eq!(
        svg.matches("<rect ").count(),
        4,
        "one heat rect per cell:\n{svg}"
    );
    assert!(
        !svg.contains("#3a3aff"),
        "call overlay rect must be gone:\n{svg}"
    );
    // The coupled cell still renders at max heat (include + call weights
    // folded into the same heat)…
    assert!(
        svg.contains("fill=\"#b30000\""),
        "coupled cell value not rendered at max heat:\n{svg}"
    );
    // …with the mandatory ADR-0001 caveat line.
    assert!(
        svg.contains("call edges are LOW-confidence hypotheses (ADR-0001)"),
        "mandatory caveat line missing:\n{svg}"
    );
}

#[test]
fn coupling_axis_orders_by_rank_then_prefix_and_counts_both_edges() {
    let out = render(&calls_doc());
    let svg = svg_of(&out, "component-coupling.svg");
    // rank 1 (src/a) label appears before rank 2 (src/b) row label.
    let a = svg.find(">src/a<").expect("src/a axis label");
    let b = svg.find(">src/b<").expect("src/b axis label");
    assert!(a < b, "axis must follow (rank, path_prefix) order");

    // Real cell-VALUE check (the test name promises counting): the
    // (src/a → src/b) cell sums include(1) + call(1) = 2 = max ⇒ the
    // darkest ramp fill. A zeroed cell would render pure white instead.
    assert!(
        svg.contains("fill=\"#b30000\""),
        "coupled cell value not rendered at max heat:\n{svg}"
    );
}

#[test]
fn root_level_files_coupling_edges_are_counted() {
    let out = render(&root_level_doc());
    let svg = svg_of(&out, "component-coupling.svg");
    // The "." component is a real axis label.
    assert!(
        svg.contains(">.<"),
        "root-level '.' axis label missing:\n{svg}"
    );
    // Both a.c↔b.c edges land in the "." diagonal cell (total 2 = max) ⇒
    // darkest heat fill. Under the historical ""-for-root parent_dir bug,
    // every root endpoint missed the bump index and ALL cells stayed white.
    assert!(
        svg.contains("fill=\"#b30000\""),
        "root-level coupling cell not counted:\n{svg}"
    );
}

#[test]
fn phantom_edge_endpoints_neither_render_nor_displace_real_nodes() {
    // 55-node chain (cap 50): p01..p53 have degree 2, p00/p54 degree 1 —
    // top-50 keeps p01..p50 ascending, omitting exactly 5. Then MANY real
    // files include a NON-node target ("version.inc" — resolvable on disk,
    // but discovery only picks .c/.h so it is absent from nodes[]). Its
    // degree must NOT count: it must not appear as a node, and the kept
    // top-50 must be unaffected.
    let paths: Vec<String> = (0..55).map(|i| format!("p{i:02}.c")).collect();
    let mut edges: Vec<IncludeEdgeDto> = paths.windows(2).map(|p| edge(&p[0], &p[1])).collect();
    for p in paths.iter().take(50) {
        edges.push(edge(p, "version.inc"));
    }
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: paths.iter().map(|p| node(p)).collect(),
            edges,
        },
        call_graph: None,
    });
    let out = render(&doc);
    let svg = svg_of(&out, "include-graph.svg");

    assert!(
        !svg.contains("version.inc"),
        "phantom non-node endpoint rendered as a node:\n{svg}"
    );
    assert!(
        svg.contains("showing top 50 of 55 files"),
        "truncation count wrong (phantom degrees skewed ranking):\n{svg}"
    );
    assert!(svg.contains(">p01.c<"), "degree-2 head must be kept");
    assert!(
        svg.contains(">p50.c<"),
        "last degree-2 node by path asc kept"
    );
    assert!(
        !svg.contains(">p51.c<"),
        "51st-ranked real file must be dropped"
    );
    assert!(
        !svg.contains(">p00.c<"),
        "degree-1 chain tail must be dropped"
    );
}

#[test]
fn all_zero_densities_render_stub_bars_and_caption() {
    let out = render(&zero_density_doc());
    let svg = svg_of(&out, "hot-components.svg");
    assert!(
        svg.contains("no findings recorded"),
        "all-zero caption missing:\n{svg}"
    );
    assert!(
        svg.contains("width=\"2\""),
        "minimal stub bars expected:\n{svg}"
    );
    assert!(svg.contains("1 src/quiet 0.00"));
    assert!(svg.contains("2 src/calmer 0.00"));
}

#[test]
fn hot_components_bar_length_is_proportional_to_max_density() {
    let mut doc = base_doc();
    doc.components = Some(vec![
        component("src/hot", 1, 10.0),
        component("src/warm", 2, 5.0),
    ]);
    let out = render(&doc);
    let svg = svg_of(&out, "hot-components.svg");
    assert!(
        svg.contains("width=\"420\""),
        "max density = full plot width"
    );
    assert!(svg.contains("width=\"210\""), "half density = half width");
    assert!(!svg.contains("no findings recorded"));
}

// ---------------------------------------------------------------------------
// 2. Emission rule: present AND non-empty, else omitted
// ---------------------------------------------------------------------------

#[test]
fn empty_document_emits_no_charts() {
    assert!(render(&base_doc()).is_empty(), "empty doc ⇒ empty output");
}

#[test]
fn empty_include_nodes_omit_include_graph_but_keep_component_charts() {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![],
            edges: vec![],
        },
        call_graph: None,
    });
    doc.components = Some(vec![component("src/solo", 1, 1.0)]);
    let out = render(&doc);
    // Symmetric-stub pin (B5): the ranked include-graph is omitted on empty
    // nodes, but the layer-file STUB still rides the present graphs section.
    assert_eq!(
        emitted_names(&out),
        [
            "layer-file.svg",
            "layer-entry.svg",
            "component-coupling.svg",
            "hot-components.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "layer-findings.svg",
            "component-01-src-solo.svg",
        ],
        "a file-less component still yields its deterministic stub page"
    );
    let stub = svg_of(&out, "layer-file.svg");
    assert!(
        stub.contains("no files recorded"),
        "layer-file stub caption missing:\n{stub}"
    );
}

#[test]
fn empty_components_emit_workspace_stub_but_omit_ranked_charts() {
    let mut doc = cycle_doc();
    doc.components = Some(vec![]);
    let out = render(&doc);
    // Symmetric-stub pin (B5): ranked component charts are omitted on empty
    // components, but the layer-workspace STUB still rides the present
    // components section — mirroring the layer-file stub above.
    assert_eq!(
        emitted_names(&out),
        [
            "include-graph.svg",
            "layer-file.svg",
            "layer-entry.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "layer-findings.svg",
        ]
    );
    let stub = svg_of(&out, "layer-workspace.svg");
    assert!(
        stub.contains("no components recorded"),
        "workspace stub caption missing:\n{stub}"
    );
}

#[test]
fn present_but_empty_sections_emit_exactly_the_two_layer_stubs() {
    // B5 symmetry lock: graphs present-but-empty AND components
    // present-but-empty ⇒ exactly the two deterministic stubs, byte-identical
    // across renders, and nothing else.
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![],
            edges: vec![],
        },
        call_graph: None,
    });
    doc.components = Some(vec![]);
    let out = render(&doc);
    assert_eq!(
        emitted_names(&out),
        [
            "layer-file.svg",
            "layer-entry.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "layer-findings.svg",
        ],
        "symmetric stubs only"
    );
    assert_eq!(render(&doc), out, "stubs must be byte-deterministic");
    let file = svg_of(&out, "layer-file.svg");
    assert!(file.contains("no files recorded"));
    let workspace = svg_of(&out, "layer-workspace.svg");
    assert!(workspace.contains("no components recorded"));
}

#[test]
fn chart_emission_order_is_frozen() {
    let mut doc = cycle_doc();
    doc.components = Some(vec![component("src/x", 1, 1.0)]);
    let out = render(&doc);
    assert_eq!(
        emitted_names(&out),
        [
            "include-graph.svg",
            "layer-file.svg",
            "layer-entry.svg",
            "component-coupling.svg",
            "hot-components.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "layer-findings.svg",
            // Per-component pages trail the overviews in (rank, prefix)
            // order; src/x has no scanned file here ⇒ stub page.
            "component-01-src-x.svg",
        ]
    );
}

#[test]
fn calls_doc_emits_real_per_component_pages_in_rank_order() {
    let out = render(&calls_doc());
    assert_eq!(
        emitted_names(&out),
        [
            "include-graph.svg",
            "layer-file.svg",
            "layer-entry.svg",
            "component-coupling.svg",
            "hot-components.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "layer-findings.svg",
            "component-01-src-a.svg",
            "component-02-src-b.svg",
        ]
    );
    // Rank-1 page: the src/a file as a node, header + density caption.
    let svg = svg_of(&out, "component-01-src-a.svg");
    assert!(svg.contains(">src/a/x.c<"), "member file node:\n{svg}");
    assert!(
        svg.contains("finding density 8.00 per KLOC"),
        "density caption:\n{svg}"
    );
    assert!(svg.contains("Component 1: src/a"), "header:\n{svg}");
}

#[test]
fn component_pages_are_byte_identical_and_heat_hot_files() {
    // Costly endpoints (cyclomatic 20) render the hottest file at max heat.
    let mut doc = calls_doc();
    doc.stats.functions = vec![
        costly_fn("src/a/x.c", "hot", 20),
        costly_fn("src/b/y.c", "warm", 2),
    ];
    let first = render(&doc);
    let second = render(&doc);
    assert_eq!(first, second, "byte-determinism violated");
    let svg = svg_of(&first, "component-01-src-a.svg");
    assert!(
        svg.contains("fill=\"#b30000\""),
        "hottest file must render at max heat:\n{svg}"
    );
}

#[test]
fn file_less_components_emit_deterministic_stubs() {
    let out = render(&zero_density_doc());
    assert_eq!(
        emitted_names(&out),
        [
            "component-coupling.svg",
            "hot-components.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "component-01-src-quiet.svg",
            "component-02-src-calmer.svg",
        ]
    );
    for name in ["component-01-src-quiet.svg", "component-02-src-calmer.svg"] {
        let svg = svg_of(&out, name);
        assert!(
            svg.contains("no files recorded"),
            "stub caption missing in {name}:\n{svg}"
        );
    }
    // Stub determinism across independent constructions.
    assert_eq!(render(&zero_density_doc()), out);
}

#[test]
fn component_page_escapes_hostile_prefix_and_path() {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![node("we<ird>&/ho\"st.c")],
            edges: vec![],
        },
        call_graph: None,
    });
    doc.components = Some(vec![component("we<ird>&", 1, 1.0)]);
    let out = render(&doc);
    // Hostile prefix slugifies to kebab-safe ASCII (no raw metacharacters in
    // the artifact name).
    assert_eq!(
        emitted_names(&out),
        [
            "include-graph.svg",
            "layer-file.svg",
            "layer-entry.svg",
            "component-coupling.svg",
            "hot-components.svg",
            "layer-workspace.svg",
            "layer-bands.svg",
            "layer-findings.svg",
            "component-01-we-ird.svg",
        ]
    );
    let svg = svg_of(&out, "component-01-we-ird.svg");
    assert!(
        svg.contains("we&lt;ird&gt;&amp;/ho&quot;st.c"),
        "hostile member path not escaped:\n{svg}"
    );
    assert!(
        svg.contains("Component 1: we&lt;ird&gt;&amp;"),
        "hostile prefix not escaped in header:\n{svg}"
    );
    assert!(!svg.contains("we<ird>"), "raw hostile bytes leaked:\n{svg}");
}

#[test]
fn oversized_component_is_capped_at_50_with_visible_marker() {
    // 60 files in ONE component: the page keeps the top-50 by degree.
    let paths: Vec<String> = (0..60).map(|i| format!("big/n{i:02}.c")).collect();
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: paths.iter().map(|p| node(p)).collect(),
            edges: paths.windows(2).map(|p| edge(&p[0], &p[1])).collect(),
        },
        call_graph: None,
    });
    doc.components = Some(vec![component("big", 1, 1.0)]);
    let out = render(&doc);
    let svg = svg_of(&out, "component-01-big.svg");
    // Node rects are 170px wide; the heat-legend swatches are 12px, so a
    // width-170 count pins the cap without tripping over the legend.
    assert_eq!(svg.matches("width=\"170\"").count(), 50, "node cap");
    assert!(
        svg.contains("showing top 50 of 60 files"),
        "truncation marker missing:\n{svg}"
    );
}

// ---------------------------------------------------------------------------
// 3. XML escaping of hostile label/path text
// ---------------------------------------------------------------------------

#[test]
fn hostile_path_is_entity_escaped_in_text_nodes() {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![node("<a>&\"'b.c")],
            edges: vec![],
        },
        call_graph: None,
    });
    let out = render(&doc);
    let svg = svg_of(&out, "include-graph.svg");
    assert!(
        svg.contains("&lt;a&gt;&amp;&quot;&#39;b.c"),
        "hostile path not fully escaped:\n{svg}"
    );
    assert!(
        !svg.contains("<a>&"),
        "raw unescaped hostile bytes leaked into output:\n{svg}"
    );
}

// ---------------------------------------------------------------------------
// Determinism across runs (fresh documents, same values)
// ---------------------------------------------------------------------------

#[test]
fn identical_documents_render_identically_across_construction() {
    // Two INDEPENDENT constructions of the same logical document.
    let a = render(&calls_doc());
    let b = render(&calls_doc());
    assert_eq!(a, b);
}

// ---------------------------------------------------------------------------
// Option A slice 2: layered overviews (workspace + file)
// ---------------------------------------------------------------------------

#[test]
fn layered_overviews_ride_their_source_gates_in_emission_order() {
    // calls_doc has both graphs and components: both layers emitted, in
    // frozen positions (file with the include gate, workspace with the
    // components gate, pages trailing).
    let out = render(&calls_doc());
    let names = emitted_names(&out);
    let file_pos = names
        .iter()
        .position(|n| *n == "layer-file.svg")
        .expect("layer-file emitted");
    let workspace_pos = names
        .iter()
        .position(|n| *n == "layer-workspace.svg")
        .expect("layer-workspace emitted");
    let entry_pos = names
        .iter()
        .position(|n| *n == "layer-entry.svg")
        .expect("layer-entry emitted");
    let bands_pos = names
        .iter()
        .position(|n| *n == "layer-bands.svg")
        .expect("layer-bands emitted");
    assert_eq!(names[file_pos - 1], "include-graph.svg");
    assert_eq!(names[file_pos + 1], "layer-entry.svg");
    assert_eq!(names[entry_pos + 1], "component-coupling.svg");
    assert_eq!(names[workspace_pos - 1], "hot-components.svg");
    assert_eq!(names[workspace_pos + 1], "layer-bands.svg");
    assert_eq!(names[bands_pos + 1], "layer-findings.svg");
    assert_eq!(names[bands_pos + 2], "component-01-src-a.svg");

    // Workspace content: both components as nodes over one coupling edge.
    let workspace = svg_of(&out, "layer-workspace.svg");
    assert!(
        workspace.contains(">1 src/a 8.00<"),
        "rank-1 node:\n{workspace}"
    );
    assert!(
        workspace.contains(">2 src/b 4.00<"),
        "rank-2 node:\n{workspace}"
    );
    assert_eq!(workspace.matches("<line ").count(), 1);
    assert!(
        workspace.contains("heat = candidate finding density per KLOC — profile to confirm"),
        "heat caption:\n{workspace}"
    );

    // File content: both files as nodes over the include edge.
    let file = svg_of(&out, "layer-file.svg");
    assert!(file.contains(">src/a/x.c<"), "file node:\n{file}");
    assert!(file.contains(">src/b/y.c<"), "file node:\n{file}");
    assert_eq!(file.matches("<line ").count(), 1);
}

#[test]
fn layered_overviews_are_byte_identical_and_capped() {
    // 60-node chain: the file layer keeps the top-50 by degree with the
    // shared truncation marker, byte-identical across renders.
    let first = render(&big_doc());
    assert_eq!(first, render(&big_doc()));
    let file = svg_of(&first, "layer-file.svg");
    assert_eq!(file.matches("width=\"170\"").count(), 50, "node cap");
    assert!(
        file.contains("showing top 50 of 60 files"),
        "truncation marker missing:\n{file}"
    );
}

#[test]
fn layer_file_escapes_hostile_paths_and_highlights_hot_path() {
    let mut doc = base_doc();
    doc.graphs = Some(GraphsSection {
        include_graph: IncludeGraph {
            nodes: vec![node("<a>&\"'b.c")],
            edges: vec![],
        },
        call_graph: None,
    });
    doc.stats.functions = vec![costly_fn("<a>&\"'b.c", "hot", 20)];
    let out = render(&doc);
    let file = svg_of(&out, "layer-file.svg");
    assert!(
        file.contains("&lt;a&gt;&amp;&quot;&#39;b.c"),
        "hostile path not fully escaped:\n{file}"
    );
    assert!(
        !file.contains("<a>&"),
        "raw unescaped hostile bytes leaked:\n{file}"
    );
    assert!(
        file.contains("stroke=\"#1a1a1a\" stroke-width=\"2\""),
        "hot-path outline missing:\n{file}"
    );
    assert!(
        file.contains("hot-path: &lt;a&gt;&amp;&quot;&#39;b.c at max cost among shown files"),
        "hot-path caption missing:\n{file}"
    );
}
