//! `charts::layers` — layered overview SVGs (Option A slice 2).
//!
//! Two static zoom levels over the same ADR-0002 guardrail-1 input set as
//! every other chart (`graphs.*` + `components[]`, plus `stats.functions[]`
//! for the file-layer cost heat — the same extension the coupling cost
//! overlay already consumes):
//!
//! - `charts/layer-workspace.svg` — components as nodes over the aggregated
//!   inter-component call/include edge set, heat by `finding_density`;
//! - `charts/layer-file.svg` — top files by degree over the include DAG,
//!   heat by per-file function cost, with the highest-cost SHOWN file
//!   outlined (the hot-path highlight: single winner — cost desc, then path
//!   asc — elected over the POST-cap node set, i.e. among the files actually
//!   drawn; a file dropped by the top-50 cap can never be outlined).
//!
//! Layer-set decision (pinned): exactly these TWO. A middle "files grouped
//! under component bands" layer was considered and NOT built — and the
//! rejection is honest about what that costs: the slice-1 per-component
//! detail pages (`component-<rank>-<slug>.svg`) draw INTRA-component file
//! edges only, while the workspace draws aggregates, so a cross-component
//! FILE edge (e.g. `src/a/x.c → src/b/y.c`) is currently visible ONLY as an
//! aggregated component-level edge — in NEITHER the workspace (aggregates
//! only) NOR any detail page (intra only). A bands view that draws those
//! edges across bands would carry distinct information and lands as an
//! explicit follow-up NEW third layer, never a silent reshape of these two.
//!
//! Determinism mechanics (same bar as `super`):
//! - every iteration sorted (`BTreeSet`/`BTreeMap`, path-asc node order);
//! - integer coordinates via [`crate::schema::round_half_up`];
//! - labels through [`super::escape_xml`] + [`super::truncate_label`];
//! - fills are pure functions of document values ([`super::heat_color`]);
//! - no timestamps, no counters, no `<metadata>`;
//! - empty inputs yield deterministic stubs (header + `no … recorded`
//!   caption) — layers are never absent-or-random once their emission gate
//!   passes.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    COMPONENT_CAP, HEAT_LEGEND_W, barycenter_order, cap_nodes, escape_xml, est_text_w,
    function_cost, heat_color, heat_ink, legend_band, legend_top, parent_dir, push_heat_legend,
    resolved_pairs, truncate_label,
};
use crate::graph::{break_cycles, topo_layers};
use crate::schema::{
    ComponentEntry, FunctionStats, GraphsSection, format_fixed_point, round_half_up,
};

// ---------------------------------------------------------------------------
// Geometry (same grid feel as the other layered charts, own constants)
// ---------------------------------------------------------------------------

const LW_MARGIN: i64 = 40;
const LW_HEADER_BAND: i64 = 64;
const LW_LAYER_W: i64 = 220;
const LW_ROW_H: i64 = 44;
const LW_NODE_W: i64 = 170;
const LW_NODE_H: i64 = 30;
const LW_CAPTION_BAND: i64 = 40;

const LF_MARGIN: i64 = 40;
const LF_HEADER_BAND: i64 = 64;
const LF_LAYER_W: i64 = 220;
const LF_ROW_H: i64 = 44;
const LF_NODE_W: i64 = 170;
const LF_NODE_H: i64 = 30;
const LF_CAPTION_BAND: i64 = 40;

/// Hot-path outline: near-black at 2px — perceivable on BOTH the dark-red
/// max-heat fill (`#b30000` renders the hot file, so a red-on-red outline
/// would be invisible) AND the white page (a white outline's outer half
/// merges with the background). Single-stroke like every sibling chart.
const LF_HOT_STROKE: &str = "#1a1a1a";
/// Default node outline (same hue as the other layered charts).
const LAYER_NODE_STROKE: &str = "#4a6fa5";

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Render `charts/layer-workspace.svg`: capped components as nodes over the
/// aggregated inter-component edge set (resolved include edges plus resolved
/// call edges, both mapped through `parent_dir` and restricted to the capped
/// axis; intra-component pairs excluded — they live on the slice-1 detail
/// pages), laid out with the shared pipeline (dedupe+break cycles → topo
/// layers → 3-pass barycenter → integer grid), heat by `finding_density`
/// normalized against the layer maximum.
///
/// Axis/cap policy mirrors the overview component charts: entries with
/// `rank > COMPONENT_CAP` are dropped, then the list is truncated to
/// `COMPONENT_CAP`. An empty component list yields the deterministic stub.
#[must_use]
pub fn render_layer_workspace(
    components: &[ComponentEntry],
    graphs: Option<&GraphsSection>,
) -> Vec<u8> {
    let mut sorted: Vec<&ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    sorted.retain(|c| c.rank <= u32::try_from(COMPONENT_CAP).unwrap_or(u32::MAX));
    let overflow = sorted.len() > COMPONENT_CAP;
    sorted.truncate(COMPONENT_CAP);
    if sorted.is_empty() {
        return workspace_stub();
    }

    let axis: BTreeSet<String> = sorted.iter().map(|c| c.path_prefix.clone()).collect();
    let density: BTreeMap<&str, f64> = sorted
        .iter()
        .map(|c| (c.path_prefix.as_str(), c.metrics.finding_density))
        .collect();

    // Inter-component edge set (BTreeSet ⇒ sorted AND deduped): include
    // pairs plus resolved call pairs, both folded to (parent, parent).
    let mut edge_set: BTreeSet<(String, String)> = BTreeSet::new();
    if let Some(graphs) = graphs {
        for (from, to) in resolved_pairs(&graphs.include_graph.edges) {
            let a = parent_dir(&from);
            let b = parent_dir(&to);
            if a != b && axis.contains(a) && axis.contains(b) {
                edge_set.insert((a.to_string(), b.to_string()));
            }
        }
        if let Some(call_graph) = &graphs.call_graph {
            for edge in &call_graph.edges {
                if let Some(to) = &edge.to_path {
                    let a = parent_dir(&edge.from_path);
                    let b = parent_dir(to);
                    if a != b && axis.contains(a) && axis.contains(b) {
                        edge_set.insert((a.to_string(), b.to_string()));
                    }
                }
            }
        }
    }

    let nodes: Vec<String> = axis.into_iter().collect();
    let pairs: Vec<(String, String)> = edge_set.into_iter().collect();
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
                    LW_MARGIN + l as i64 * LW_LAYER_W,
                    LW_MARGIN + LW_HEADER_BAND + i as i64 * LW_ROW_H,
                ),
            );
        }
    }

    // Heat is layer-global: the densest component renders at max heat.
    let max_density = nodes
        .iter()
        .map(|n| density.get(n.as_str()).copied().unwrap_or(0.0))
        .fold(0.0_f64, f64::max);
    let all_zero = max_density <= 0.0;

    let rows = layers.iter().map(Vec::len).max().unwrap_or(1);
    let grid_w = 2 * LW_MARGIN + (layers.len().saturating_sub(1)) as i64 * LW_LAYER_W + LW_NODE_W;
    let title = format!(
        "Workspace — {} {}",
        nodes.len(),
        if nodes.len() == 1 {
            "component"
        } else {
            "components"
        },
    );
    let subtitle = "heat = finding density per KLOC";
    // Caption rows bottom-first: cap marker, all-zero note, cycle note.
    let mut captions: Vec<String> = Vec::new();
    if overflow {
        captions.push("… more components omitted".to_string());
    }
    if all_zero {
        captions.push("no findings recorded".to_string());
    }
    if broken_count > 0 {
        captions.push(format!(
            "{broken_count} cyclic edges broken (see ARCH-001 findings)"
        ));
    }
    // Narrow grids are widened so the title, subtitle, legend, and every
    // caption keeps ≥10px from the right edge (never clipped).
    let mut width = grid_w
        .max(LW_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(LW_MARGIN + est_text_w(subtitle, 11.0) + 10)
        .max(LW_MARGIN + HEAT_LEGEND_W + 10);
    for caption in &captions {
        width = width.max(LW_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height =
        2 * LW_MARGIN + LW_HEADER_BAND + rows as i64 * LW_ROW_H + legend_band(captions.len());

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LW_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{}</text>\n",
        LW_MARGIN + 16,
        escape_xml(&title),
    ));
    svg.push_str(&format!(
        "<text x=\"{LW_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{subtitle}</text>\n",
        LW_MARGIN + 36,
    ));

    // Edges first (under nodes): kept inter-component DAG edges only. The
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
            let x1 = fx + LW_NODE_W;
            let y1 = fy + LW_NODE_H / 2;
            let y2 = ty + LW_NODE_H / 2;
            svg.push_str(&format!(
                "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\"/>\n"
            ));
        }
        svg.push_str("</g>\n");
    }

    // Nodes: heat-filled rect + `rank prefix density` label, fixed order.
    // Labels on dark fills (level ≥ 7) take white ink for contrast.
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for node in &nodes {
        let Some(&(x, y)) = pos.get(node.as_str()) else {
            continue;
        };
        let level = if all_zero {
            0
        } else {
            let d = density.get(node.as_str()).copied().unwrap_or(0.0);
            round_half_up(d / max_density * 10.0, 0).clamp(0, 10) as u32
        };
        let fill = heat_color(level);
        let rank = sorted
            .iter()
            .find(|c| c.path_prefix == *node)
            .map(|c| c.rank)
            .unwrap_or(0);
        let label = escape_xml(&format!(
            "{rank} {} {}",
            truncate_label(node),
            format_fixed_point(density.get(node.as_str()).copied().unwrap_or(0.0), 2),
        ));
        let text_x = x + 6 + 2; // ≈ CHAR_W/2 + 2 in the overview's fixed metric
        let text_y = y + LW_NODE_H / 2 + 4;
        let ink = match heat_ink(level) {
            Some(ink) => format!(" fill=\"{ink}\""),
            None => String::new(),
        };
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{LW_NODE_W}\" height=\"{LW_NODE_H}\" fill=\"{fill}\" stroke=\"{LAYER_NODE_STROKE}\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{text_x}\" y=\"{text_y}\"{ink}>{label}</text>\n"
        ));
    }
    svg.push_str("</g>\n");

    // Heat-scale legend (the subtitle defines "heat ="), then captions
    // bottom row first so the band fits content exactly.
    push_heat_legend(&mut svg, LW_MARGIN, legend_top(height, captions.len()));
    for (k, caption) in captions.iter().enumerate() {
        let y = height - 12 - k as i64 * 16;
        let out = caption.replace('…', "&#8230;");
        svg.push_str(&format!(
            "<text x=\"{LW_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{out}</text>\n"
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

/// Render `charts/layer-file.svg`: top files by degree over the include DAG
/// (resolve filter → the shared top-50 cap → dedupe+break cycles → topo
/// layers → 3-pass barycenter → integer grid), heat by per-file function cost
/// (Σ `function_cost`, normalized against the layer maximum), with the
/// highest-cost SHOWN file given the distinct hot-path outline: a SINGLE
/// winner (cost desc, then path asc) elected over the POST-cap node set —
/// the caption states this scope, so the highlight always discriminates.
///
/// An empty node list yields the deterministic stub.
#[must_use]
pub fn render_layer_file(graphs: &GraphsSection, functions: &[FunctionStats]) -> Vec<u8> {
    let mut nodes: Vec<String> = graphs
        .include_graph
        .nodes
        .iter()
        .map(|n| n.path.clone())
        .collect();
    nodes.sort();
    nodes.dedup();
    if nodes.is_empty() {
        return file_stub();
    }

    let all_pairs = resolved_pairs(&graphs.include_graph.edges);
    let (nodes, dropped) = cap_nodes(&nodes, &all_pairs);
    let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let pairs: Vec<(String, String)> = all_pairs
        .into_iter()
        .filter(|(f, t)| node_set.contains(f.as_str()) && node_set.contains(t.as_str()))
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
                    LF_MARGIN + l as i64 * LF_LAYER_W,
                    LF_MARGIN + LF_HEADER_BAND + i as i64 * LF_ROW_H,
                ),
            );
        }
    }

    // Per-file cost: Σ function_cost over that file's functions; the
    // layer-global maximum normalizes the heat ramp. The hot-path outline
    // elects a SINGLE winner over the POST-cap (drawn) node set — cost
    // desc, then path asc, so ties always discriminate to one file — and
    // the caption below states this scope truthfully.
    let mut file_cost: BTreeMap<&str, f64> = BTreeMap::new();
    for f in functions {
        *file_cost.entry(f.path.as_str()).or_insert(0.0) += function_cost(f);
    }
    let max_cost = nodes
        .iter()
        .map(|n| file_cost.get(n.as_str()).copied().unwrap_or(0.0))
        .fold(0.0_f64, f64::max);
    let hot: Option<&str> = if max_cost <= 0.0 {
        None
    } else {
        nodes
            .iter()
            .filter(|n| file_cost.get(n.as_str()).copied().unwrap_or(0.0) >= max_cost)
            .map(String::as_str)
            .min()
    };

    let rows = layers.iter().map(Vec::len).max().unwrap_or(1);
    let grid_w = 2 * LF_MARGIN + (layers.len().saturating_sub(1)) as i64 * LF_LAYER_W + LF_NODE_W;
    let title = format!(
        "File hot-path map — {} {}",
        nodes.len(),
        if nodes.len() == 1 { "file" } else { "files" },
    );
    let subtitle = "heat = function cost; hot-path outlined (top-50 scope)";
    // Caption rows bottom-first: cap marker, cost/hot-path note, cycle note.
    let mut captions: Vec<String> = Vec::new();
    if dropped > 0 {
        captions.push(format!("… {dropped} more files omitted"));
    }
    if max_cost <= 0.0 {
        captions.push("no function cost recorded".to_string());
    } else if let Some(h) = hot {
        captions.push(format!(
            "hot-path: {} at max cost among shown files",
            truncate_label(h)
        ));
    }
    if broken_count > 0 {
        captions.push(format!(
            "{broken_count} cyclic edges broken (see ARCH-001 findings)"
        ));
    }
    // Narrow grids are widened so the title, subtitle, legend, and every
    // caption keeps ≥10px from the right edge (never clipped).
    let mut width = grid_w
        .max(LF_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(LF_MARGIN + est_text_w(subtitle, 11.0) + 10)
        .max(LF_MARGIN + HEAT_LEGEND_W + 10);
    for caption in &captions {
        width = width.max(LF_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height =
        2 * LF_MARGIN + LF_HEADER_BAND + rows as i64 * LF_ROW_H + legend_band(captions.len());

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{}</text>\n",
        LF_MARGIN + 16,
        escape_xml(&title),
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{subtitle}</text>\n",
        LF_MARGIN + 36,
    ));

    // Edges first (under nodes): kept DAG edges only. The group is omitted
    // outright when empty (no empty `<g stroke>` in output).
    if !cycle.kept.is_empty() {
        svg.push_str("<g stroke=\"#999999\" stroke-width=\"1\">\n");
        for (from, to) in &cycle.kept {
            let Some(&(fx, fy)) = pos.get(from.as_str()) else {
                continue;
            };
            let Some(&(tx, ty)) = pos.get(to.as_str()) else {
                continue;
            };
            let x1 = fx + LF_NODE_W;
            let y1 = fy + LF_NODE_H / 2;
            let y2 = ty + LF_NODE_H / 2;
            svg.push_str(&format!(
                "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\"/>\n"
            ));
        }
        svg.push_str("</g>\n");
    }

    // Nodes: heat-filled rect + label; hot-path files get the distinct
    // outline, everything else the shared node stroke. Labels on dark fills
    // (level ≥ 7) take white ink for contrast.
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for node in &nodes {
        let Some(&(x, y)) = pos.get(node.as_str()) else {
            continue;
        };
        let level = if max_cost <= 0.0 {
            0
        } else {
            let cost = file_cost.get(node.as_str()).copied().unwrap_or(0.0);
            round_half_up(cost / max_cost * 10.0, 0).clamp(0, 10) as u32
        };
        let fill = heat_color(level);
        let (stroke, stroke_w) = if hot == Some(node.as_str()) {
            (LF_HOT_STROKE, 2)
        } else {
            (LAYER_NODE_STROKE, 1)
        };
        let label = escape_xml(&truncate_label(node));
        let text_x = x + 6 + 2; // ≈ CHAR_W/2 + 2 in the overview's fixed metric
        let text_y = y + LF_NODE_H / 2 + 4;
        let ink = match heat_ink(level) {
            Some(ink) => format!(" fill=\"{ink}\""),
            None => String::new(),
        };
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{LF_NODE_W}\" height=\"{LF_NODE_H}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_w}\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{text_x}\" y=\"{text_y}\"{ink}>{label}</text>\n"
        ));
    }
    svg.push_str("</g>\n");

    // Heat-scale legend (the subtitle defines "heat ="), then captions
    // bottom row first so the band fits content exactly.
    push_heat_legend(&mut svg, LF_MARGIN, legend_top(height, captions.len()));
    for (k, caption) in captions.iter().enumerate() {
        let y = height - 12 - k as i64 * 16;
        let out = escape_xml(caption).replace('…', "&#8230;");
        svg.push_str(&format!(
            "<text x=\"{LF_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{out}</text>\n"
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

// ---------------------------------------------------------------------------
// Deterministic stubs
// ---------------------------------------------------------------------------

/// Stub for an empty component list: header + caption, no nodes.
fn workspace_stub() -> Vec<u8> {
    let width = 2 * LW_MARGIN + LW_NODE_W;
    let height = LW_MARGIN + LW_HEADER_BAND + LW_CAPTION_BAND;
    let mut svg = String::with_capacity(1024);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LW_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">Workspace — no components</text>\n",
        LW_MARGIN + 16
    ));
    svg.push_str(&format!(
        "<text x=\"{LW_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">no components recorded</text>\n",
        height - 12
    ));
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

/// Stub for an empty file list: header + caption, no nodes.
fn file_stub() -> Vec<u8> {
    let width = 2 * LF_MARGIN + LF_NODE_W;
    let height = LF_MARGIN + LF_HEADER_BAND + LF_CAPTION_BAND;
    let mut svg = String::with_capacity(1024);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">File hot-path map — no files</text>\n",
        LF_MARGIN + 16
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">no files recorded</text>\n",
        height - 12
    ));
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

#[cfg(test)]
mod tests {
    //! Contract locks: stub shapes, cap policy, heat normalization, hot-path
    //! election, hostile escaping, double-render determinism.

    use super::*;
    use crate::ir::IncludeForm;
    use crate::ir::Span;
    use crate::schema::{ComponentMetrics, FunctionStats, GraphNode, IncludeEdgeDto, IncludeGraph};

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
                    .map(|(f, t)| IncludeEdgeDto {
                        from: (*f).to_string(),
                        to: Some((*t).to_string()),
                        form: IncludeForm::Quoted,
                        resolved: true,
                        span: Span::point(1, 1),
                    })
                    .collect(),
            },
            call_graph: None,
        }
    }

    fn costly_fn(path: &str, cyclomatic: u32) -> FunctionStats {
        FunctionStats {
            id: format!("{path}#f#1:1"),
            name: "f".to_string(),
            path: path.to_string(),
            span: Span::point(1, 1),
            param_count: 0,
            cyclomatic_complexity: cyclomatic,
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

    #[test]
    fn empty_inputs_yield_deterministic_stubs() {
        let first = render_layer_workspace(&[], None);
        let second = render_layer_workspace(&[], None);
        assert_eq!(first, second);
        let text = String::from_utf8(first).expect("SVG UTF-8");
        assert!(
            text.contains("no components recorded"),
            "stub caption:\n{text}"
        );

        let graphs = graphs(&[], &[]);
        let first = render_layer_file(&graphs, &[]);
        let second = render_layer_file(&graphs, &[]);
        assert_eq!(first, second);
        let text = String::from_utf8(first).expect("SVG UTF-8");
        assert!(text.contains("no files recorded"), "stub caption:\n{text}");
    }

    #[test]
    fn workspace_couples_components_over_include_edges() {
        let comps = vec![comp("src/a", 1, 8.0), comp("src/b", 2, 4.0)];
        let g = graphs(&["src/a/x.c", "src/b/y.c"], &[("src/a/x.c", "src/b/y.c")]);
        let first = render_layer_workspace(&comps, Some(&g));
        let second = render_layer_workspace(&comps, Some(&g));
        assert_eq!(first, second, "byte-determinism violated");
        let text = String::from_utf8(first).expect("SVG UTF-8");
        // Both components as nodes, one inter-component edge, heat extremes.
        assert!(text.contains(">1 src/a 8.00<"), "rank-1 node:\n{text}");
        assert!(text.contains(">2 src/b 4.00<"), "rank-2 node:\n{text}");
        assert_eq!(text.matches("<line ").count(), 1);
        assert!(
            text.contains("fill=\"#b30000\""),
            "densest component at max heat:\n{text}"
        );
    }

    #[test]
    fn workspace_cap_mirrors_the_overview_charts() {
        let comps: Vec<ComponentEntry> = (1..=60)
            .map(|r| comp(&format!("dir{r:02}"), r, 0.0))
            .collect();
        let text = String::from_utf8(render_layer_workspace(&comps, None)).expect("SVG UTF-8");
        assert_eq!(text.matches("width=\"170\"").count(), COMPONENT_CAP);
        assert!(
            !text.contains("dir51"),
            "rank-51 component must be dropped like the overviews drop it"
        );
    }

    #[test]
    fn file_layer_outlines_the_highest_cost_file() {
        let g = graphs(&["a.c", "b.c"], &[("a.c", "b.c")]);
        let functions = vec![costly_fn("a.c", 20), costly_fn("b.c", 2)];
        let first = render_layer_file(&g, &functions);
        let second = render_layer_file(&g, &functions);
        assert_eq!(first, second, "byte-determinism violated");
        let text = String::from_utf8(first).expect("SVG UTF-8");
        assert!(
            text.contains("stroke=\"#1a1a1a\" stroke-width=\"2\""),
            "hot-path outline missing:\n{text}"
        );
        assert!(
            text.contains("hot-path: a.c at max cost among shown files"),
            "hot-path caption missing:\n{text}"
        );
        // The costly file renders at max heat.
        assert!(
            text.contains("fill=\"#b30000\""),
            "max heat missing:\n{text}"
        );
    }

    #[test]
    fn file_layer_hot_path_tie_outlines_exactly_one_file() {
        // B2 lock: equal max costs MUST still discriminate — single winner
        // (cost desc, then path asc), elected among the drawn files.
        let g = graphs(&["a.c", "b.c"], &[("a.c", "b.c")]);
        let functions = vec![costly_fn("a.c", 10), costly_fn("b.c", 10)];
        let first = render_layer_file(&g, &functions);
        let second = render_layer_file(&g, &functions);
        assert_eq!(first, second, "byte-determinism violated");
        let text = String::from_utf8(first).expect("SVG UTF-8");
        assert_eq!(
            text.matches("stroke=\"#1a1a1a\" stroke-width=\"2\"")
                .count(),
            1,
            "tie must outline exactly one file:\n{text}"
        );
        assert!(
            text.contains("hot-path: a.c at max cost among shown files"),
            "path-asc winner caption missing:\n{text}"
        );
    }

    #[test]
    fn file_layer_without_cost_renders_uniformly() {
        let g = graphs(&["a.c"], &[]);
        let text = String::from_utf8(render_layer_file(&g, &[])).expect("SVG UTF-8");
        assert!(text.contains("no function cost recorded"));
        assert!(
            !text.contains("stroke=\"#1a1a1a\" stroke-width=\"2\""),
            "no hot outline without cost:\n{text}"
        );
    }

    #[test]
    fn workspace_escapes_hostile_prefixes() {
        let comps = vec![comp("we<ird>&", 1, 1.0)];
        let text = String::from_utf8(render_layer_workspace(&comps, None)).expect("SVG UTF-8");
        assert!(
            text.contains("we&lt;ird&gt;&amp;"),
            "hostile prefix not escaped:\n{text}"
        );
        assert!(!text.contains("we<ird>"), "raw hostile bytes leaked");
    }
}
