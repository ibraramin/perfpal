//! `charts::component` — per-component detail pages (Option A slice 1).
//!
//! Renders ONE SVG per capped component (`charts/component-<rank>-<slug>.svg`)
//! showing that component's files as heat-filled nodes over its
//! intra-component include subgraph. Inputs are the same ADR-0002 guardrail 1
//! set as the overview charts (`graphs.*` + `components[]`, plus
//! `files[].path` for membership and `stats.functions[]` for the cost heat —
//! the same extension the coupling cost overlay already consumes).
//!
//! Determinism mechanics (same bar as `super`):
//! - every iteration sorted (`BTreeSet`/`BTreeMap`, path-asc node order);
//! - integer coordinates via [`crate::schema::round_half_up`];
//! - labels through [`super::escape_xml`] + [`super::truncate_label`];
//! - fills are a pure function of document values ([`super::heat_color`]);
//! - no timestamps, no counters, no `<metadata>`;
//! - a component with no member files emits a deterministic stub (header +
//!   `no files recorded` caption) — pages are never absent-or-random.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    COMPONENT_CAP, HEAT_LEGEND_W, barycenter_order, cap_nodes, escape_xml, est_text_w,
    file_heat_costs, heat_color, heat_ink, legend_band, legend_top, parent_dir, push_heat_legend,
    reachable_files, resolved_pairs, truncate_label,
};
use crate::graph::{break_cycles, topo_layers};
use crate::schema::{
    ComponentEntry, FileEntry, FunctionStats, GraphsSection, format_fixed_point, round_half_up,
};

// ---------------------------------------------------------------------------
// Naming (pinned; output-contract §3 kebab-case rule)
// ---------------------------------------------------------------------------

/// Slugify a component `path_prefix` into the kebab-case filename fragment:
/// lowercase alphanumerics kept, every other char → `-`, runs collapsed,
/// leading/trailing `-` trimmed; empty (the `"."` root) → `"root"`.
/// Rank-prefixed filenames stay unique even when two prefixes slug alike.
#[must_use]
pub fn component_slug(prefix: &str) -> String {
    let mut slug = String::with_capacity(prefix.len());
    let mut last_dash = true; // suppress leading runs
    for c in prefix.to_ascii_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "root".to_string()
    } else {
        slug
    }
}

/// Pinned page filename for `(rank, path_prefix)`: zero-padded rank keeps
/// directory listings in rank order.
#[must_use]
pub fn component_filename(rank: u32, prefix: &str) -> String {
    format!("component-{rank:02}-{}.svg", component_slug(prefix))
}

// ---------------------------------------------------------------------------
// Geometry (same grid feel as the include-graph overview, own constants)
// ---------------------------------------------------------------------------

const PC_MARGIN: i64 = 40;
const PC_HEADER_BAND: i64 = 64;
const PC_LAYER_W: i64 = 220;
const PC_ROW_H: i64 = 44;
const PC_NODE_W: i64 = 170;
const PC_NODE_H: i64 = 30;
const PC_CAPTION_BAND: i64 = 40;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Render one page per capped component, in `(rank, path_prefix)` order.
///
/// Axis/cap policy mirrors the overview component charts: entries with
/// `rank > COMPONENT_CAP` are dropped, then the list is truncated to
/// `COMPONENT_CAP`. Every surviving component yields a page — file-less
/// components (synthetic docs, or prefixes with no scanned file) yield the
/// deterministic stub, never an omission.
#[must_use]
pub fn render_component_pages(
    components: &[ComponentEntry],
    graphs: Option<&GraphsSection>,
    files: &[FileEntry],
    functions: &[FunctionStats],
) -> Vec<(String, Vec<u8>)> {
    let mut sorted: Vec<&ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    sorted.retain(|c| c.rank <= u32::try_from(COMPONENT_CAP).unwrap_or(u32::MAX));
    sorted.truncate(COMPONENT_CAP);

    // Membership: union of include-graph nodes and scanned file paths whose
    // parent directory equals the component prefix (BTreeSet ⇒ sorted).
    let mut members: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for comp in &sorted {
        members.insert(comp.path_prefix.clone(), BTreeSet::new());
    }
    if let Some(graphs) = graphs {
        for node in &graphs.include_graph.nodes {
            if let Some(set) = members.get_mut(parent_dir(&node.path)) {
                set.insert(node.path.clone());
            }
        }
    }
    for file in files {
        if let Some(set) = members.get_mut(parent_dir(&file.path)) {
            set.insert(file.path.clone());
        }
    }

    // Per-file heat uses the shared v2-preferred selector (BTreeMap ⇒
    // deterministic); the page-local maximum normalizes the heat ramp.
    let file_cost = file_heat_costs(functions);
    let reachable = reachable_files(functions);

    let include_pairs: Vec<(String, String)> = graphs
        .map(|g| resolved_pairs(&g.include_graph.edges))
        .unwrap_or_default();

    let mut out = Vec::new();
    for comp in sorted {
        let empty = BTreeSet::new();
        let member_set = members.get(&comp.path_prefix).unwrap_or(&empty);
        let name = component_filename(comp.rank, &comp.path_prefix);
        let bytes = if member_set.is_empty() {
            render_component_stub(comp)
        } else {
            render_component_page(comp, member_set, &include_pairs, &file_cost, &reachable)
        };
        out.push((name, bytes));
    }
    out
}

// ---------------------------------------------------------------------------
// Page renderers
// ---------------------------------------------------------------------------

/// Deterministic stub for a file-less component: header + caption, no nodes.
/// The canvas widens past the stub grid when the header lines need it, so
/// long prefixes keep ≥10px from the right edge (never clipped).
fn render_component_stub(comp: &ComponentEntry) -> Vec<u8> {
    let (title, subtitle) = header_lines(comp, 0);
    let grid_w = 2 * PC_MARGIN + PC_NODE_W;
    let width = grid_w
        .max(PC_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(PC_MARGIN + est_text_w(&subtitle, 11.0) + 10);
    let height = PC_MARGIN + PC_HEADER_BAND + PC_CAPTION_BAND;
    let mut svg = String::with_capacity(1024);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    push_header(&mut svg, &title, &subtitle);
    svg.push_str(&format!(
        "<text x=\"{PC_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">no files recorded</text>\n",
        height - 12
    ));
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

/// Full page: member files as heat-filled nodes over the intra-component
/// include subgraph (resolve filter → cap → dedupe+break cycles → topo
/// layers → 3-pass barycenter → integer grid), mirroring the include-graph
/// overview pipeline at component scope.
fn render_component_page(
    comp: &ComponentEntry,
    member_set: &BTreeSet<String>,
    include_pairs: &[(String, String)],
    file_cost: &BTreeMap<String, f64>,
    reachable: &BTreeSet<String>,
) -> Vec<u8> {
    // Intra-component scope FIRST: only edges with both endpoints in this
    // component's member set may influence the cap ranking. Ranking by the
    // scan-global set would let cross-component (or phantom) edges displace
    // the files that define this page and silently drop intra-component
    // edges — so the cap below ranks by INTRA-component degree only.
    let intra_pairs: Vec<(String, String)> = include_pairs
        .iter()
        .filter(|(f, t)| member_set.contains(f) && member_set.contains(t))
        .cloned()
        .collect();
    let members: Vec<String> = member_set.iter().cloned().collect();
    let (nodes, total) = cap_nodes(&members, &intra_pairs, file_cost, reachable);
    // `nodes ⊆ member_set` by construction (`cap_nodes` returns a subset of
    // its input), so the capped node set alone admits the intra-component
    // subgraph — no second membership check needed.
    let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let pairs: Vec<(String, String)> = include_pairs
        .iter()
        .filter(|(f, t)| node_set.contains(f.as_str()) && node_set.contains(t.as_str()))
        .cloned()
        .collect();

    let cycle = break_cycles(&pairs);
    let broken_count = cycle.broken.len();
    let mut layers = topo_layers(&nodes, &cycle.kept);
    barycenter_order(&mut layers, &cycle.kept);

    let mut pos: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for (l, layer_members) in layers.iter().enumerate() {
        for (i, node) in layer_members.iter().enumerate() {
            pos.insert(
                node.as_str(),
                (
                    PC_MARGIN + l as i64 * PC_LAYER_W,
                    PC_MARGIN + PC_HEADER_BAND + i as i64 * PC_ROW_H,
                ),
            );
        }
    }

    // Heat normalization is page-local (component scope): the hottest file
    // in THIS component renders at max heat.
    let max_cost = nodes
        .iter()
        .map(|n| file_cost.get(n).copied().unwrap_or(0.0))
        .fold(0.0_f64, f64::max);

    let rows = layers.iter().map(Vec::len).max().unwrap_or(1);
    let grid_w = 2 * PC_MARGIN + (layers.len().saturating_sub(1)) as i64 * PC_LAYER_W + PC_NODE_W;
    let (title, subtitle) = header_lines(comp, nodes.len());
    // Caption rows bottom-first: cap marker, then the cycle note.
    let mut captions: Vec<String> = Vec::new();
    if total > nodes.len() {
        captions.push(format!("showing top {} of {} files", nodes.len(), total));
    }
    if broken_count > 0 {
        captions.push(format!(
            "{broken_count} cyclic edges broken (see ARCH-001 findings)"
        ));
    }
    // Narrow (e.g. single-column) pages widen past the grid when the header,
    // legend, or any caption needs it — text keeps ≥10px from the right edge.
    let mut width = grid_w
        .max(PC_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(PC_MARGIN + est_text_w(&subtitle, 11.0) + 10)
        .max(PC_MARGIN + HEAT_LEGEND_W + 10);
    for caption in &captions {
        width = width.max(PC_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height =
        2 * PC_MARGIN + PC_HEADER_BAND + rows as i64 * PC_ROW_H + legend_band(captions.len());

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    push_header(&mut svg, &title, &subtitle);

    // Edges first (under nodes): kept intra-component DAG edges only. The
    // group is omitted outright when empty (no empty `<g stroke>` in output).
    if !cycle.kept.is_empty() {
        svg.push_str("<g stroke=\"#999999\" stroke-width=\"1\">\n");
        for (from, to) in &cycle.kept {
            let Some(&(fx, fy)) = pos.get(from.as_str()) else {
                continue;
            };
            let Some(&(tx, ty)) = pos.get(to.as_str()) else {
                continue;
            };
            let x1 = fx + PC_NODE_W;
            let y1 = fy + PC_NODE_H / 2;
            let y2 = ty + PC_NODE_H / 2;
            svg.push_str(&format!(
                "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\"/>\n"
            ));
        }
        svg.push_str("</g>\n");
    }

    // Nodes: heat-filled rect + label, fixed attribute order. Labels on dark
    // fills (level ≥ 7) take white ink for contrast.
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for node in &nodes {
        let Some(&(x, y)) = pos.get(node.as_str()) else {
            continue;
        };
        let level = if max_cost <= 0.0 {
            0
        } else {
            let cost = file_cost.get(node).copied().unwrap_or(0.0);
            round_half_up(cost / max_cost * 10.0, 0).clamp(0, 10) as u32
        };
        let fill = heat_color(level);
        let label = escape_xml(&truncate_label(node));
        let text_x = x + 9; // ≈ CHAR_W/2 + 2 in the overview's fixed metric
        let text_y = y + PC_NODE_H / 2 + 4;
        let ink = match heat_ink(level) {
            Some(ink) => format!(" fill=\"{ink}\""),
            None => String::new(),
        };
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{PC_NODE_W}\" height=\"{PC_NODE_H}\" fill=\"{fill}\" stroke=\"#4a6fa5\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{text_x}\" y=\"{text_y}\"{ink}>{label}</text>\n"
        ));
    }
    svg.push_str("</g>\n");

    // Heat-scale legend (same band as the layer overviews), then captions
    // bottom row first so the band fits content exactly.
    push_heat_legend(&mut svg, PC_MARGIN, legend_top(height, captions.len()));
    for (k, caption) in captions.iter().enumerate() {
        let y = height - 12 - k as i64 * 16;
        let out = caption.replace('…', "&#8230;");
        svg.push_str(&format!(
            "<text x=\"{PC_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{out}</text>\n"
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

/// Shared page header lines (rendered form): component identity title +
/// finding-density subtitle (density via the shared HALF-UP fixed-point
/// formatter, scale 2 — the same numeric text the reporter prints).
fn header_lines(comp: &ComponentEntry, file_count: usize) -> (String, String) {
    let title = truncate_label(&format!("Component {}: {}", comp.rank, comp.path_prefix));
    let files_word = if file_count == 1 { "file" } else { "files" };
    let subtitle = format!(
        "finding density {} per KLOC · {file_count} {files_word}",
        format_fixed_point(comp.metrics.finding_density, 2),
    );
    (title, subtitle)
}

/// Shared page header: component identity + finding-density caption.
fn push_header(out: &mut String, title: &str, subtitle: &str) {
    out.push_str(&format!(
        "<text x=\"{PC_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{}</text>\n",
        PC_MARGIN + 16,
        escape_xml(title),
    ));
    out.push_str(&format!(
        "<text x=\"{PC_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{}</text>\n",
        PC_MARGIN + 36,
        escape_xml(subtitle),
    ));
}

#[cfg(test)]
mod tests {
    //! Contract locks: slug/filename pinning, stub shape, page determinism.

    use super::*;
    use crate::schema::ComponentMetrics;

    fn comp(prefix: &str, rank: u32, density: f64) -> ComponentEntry {
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

    #[test]
    fn slug_conventions_are_pinned() {
        assert_eq!(component_slug("src/net"), "src-net");
        assert_eq!(component_slug("."), "root");
        assert_eq!(component_slug("Src/A_B"), "src-a-b");
        assert_eq!(component_slug("a//b"), "a-b", "runs collapse");
        assert_eq!(component_slug("weird<name>"), "weird-name");
        assert_eq!(component_filename(1, "src/a"), "component-01-src-a.svg");
        assert_eq!(component_filename(2, "."), "component-02-root.svg");
    }

    #[test]
    fn file_less_component_renders_deterministic_stub() {
        let first = render_component_pages(&[comp("src/ghost", 1, 0.0)], None, &[], &[]);
        let second = render_component_pages(&[comp("src/ghost", 1, 0.0)], None, &[], &[]);
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, "component-01-src-ghost.svg");
        let text = String::from_utf8(first[0].1.clone()).expect("SVG UTF-8");
        assert!(text.contains("no files recorded"), "stub caption:\n{text}");
        assert!(text.contains("finding density 0.00 per KLOC"));
    }

    #[test]
    fn rank_cap_mirrors_the_overview_charts() {
        let comps: Vec<ComponentEntry> = (1..=60)
            .map(|r| comp(&format!("dir{r:02}"), r, 0.0))
            .collect();
        let pages = render_component_pages(&comps, None, &[], &[]);
        assert_eq!(pages.len(), COMPONENT_CAP);
        assert_eq!(pages[0].0, "component-01-dir01.svg");
    }

    #[test]
    fn intra_component_edge_survives_the_single_node_set_filter() {
        // Lock for the double-filter simplification: `nodes ⊆ member_set`
        // by construction, so the capped node set alone must admit the
        // intra-component edge (exactly one drawn line).
        use crate::ir::{IncludeForm, Span};
        use crate::schema::{GraphNode, GraphsSection, IncludeEdgeDto, IncludeGraph};
        let graphs = GraphsSection {
            include_graph: IncludeGraph {
                nodes: vec![
                    GraphNode {
                        path: "src/a/x.c".to_string(),
                    },
                    GraphNode {
                        path: "src/a/y.c".to_string(),
                    },
                ],
                edges: vec![IncludeEdgeDto {
                    from: "src/a/x.c".to_string(),
                    to: Some("src/a/y.c".to_string()),
                    form: IncludeForm::Quoted,
                    resolved: true,
                    span: Span::point(1, 1),
                }],
            },
            call_graph: None,
        };
        let pages = render_component_pages(&[comp("src/a", 1, 1.0)], Some(&graphs), &[], &[]);
        assert_eq!(pages.len(), 1);
        let text = String::from_utf8(pages[0].1.clone()).expect("SVG UTF-8");
        assert!(text.contains(">src/a/x.c<"), "member node:\n{text}");
        assert!(text.contains(">src/a/y.c<"), "member node:\n{text}");
        assert_eq!(text.matches("<line ").count(), 1, "edge drawn:\n{text}");
    }

    #[test]
    fn cap_ranks_by_intra_component_degree_so_the_hub_survives() {
        // B1 lock: the top-50 cap MUST rank by INTRA-component degree. The
        // planted victim (`big/f00.c`, 20 intra edges, zero cross edges)
        // must survive the cap even though every other file carries
        // cross-component edges that a global-degree ranking would count.
        use crate::ir::{IncludeForm, Span};
        use crate::schema::{GraphNode, GraphsSection, IncludeEdgeDto, IncludeGraph};
        let paths: Vec<String> = (0..60).map(|i| format!("big/f{i:02}.c")).collect();
        let pair = |from: &str, to: &str| IncludeEdgeDto {
            from: from.to_string(),
            to: Some(to.to_string()),
            form: IncludeForm::Quoted,
            resolved: true,
            span: Span::point(1, 1),
        };
        let mut edges = Vec::new();
        // Victim fan: f00 → f01..=f20 (20 intra edges, degree 20).
        for i in 1..=20 {
            edges.push(pair(&paths[0], &paths[i]));
        }
        // Backbone chain f01 → f02 → … → f59 (degrees 1–3 elsewhere).
        for pair_paths in paths[1..].windows(2) {
            edges.push(pair(&pair_paths[0], &pair_paths[1]));
        }
        // Cross-component decoys: every non-victim file also includes an
        // unscanned external file. These endpoints are not chart nodes, so
        // an intra-component ranking ignores them; a global ranking would
        // let them outrank the victim.
        for p in paths.iter().skip(1) {
            edges.push(pair(p, "other/external.c"));
        }
        let graphs = GraphsSection {
            include_graph: IncludeGraph {
                nodes: paths
                    .iter()
                    .map(|p| GraphNode { path: p.clone() })
                    .collect(),
                edges,
            },
            call_graph: None,
        };
        let pages = render_component_pages(&[comp("big", 1, 1.0)], Some(&graphs), &[], &[]);
        assert_eq!(pages.len(), 1);
        let text = String::from_utf8(pages[0].1.clone()).expect("SVG UTF-8");
        assert_eq!(text.matches("width=\"170\"").count(), 50, "node cap");
        assert!(
            text.contains(">big/f00.c<"),
            "high-intra victim must survive the cap:\n{text}"
        );
        assert!(
            text.contains("showing top 50 of 60 files"),
            "honest truncation caption missing:\n{text}"
        );
    }
}
