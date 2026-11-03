//! `charts::layers` — layered overview SVGs (Option A slice 2 + I3 map-build).
//!
//! Four static zoom levels over the same ADR-0002 guardrail-1 input set as
//! every other chart (`graphs.*` + `components[]` + `files[]`, plus
//! `stats.functions[]` for the cost heat — the same extension the coupling
//! cost overlay already consumes):
//!
//! - `charts/layer-workspace.svg` — components as nodes over the aggregated
//!   inter-component call/include edge set, heat by `finding_density`;
//! - `charts/layer-file.svg` — top files by degree over the include DAG,
//!   heat by per-file function cost, with the highest-cost SHOWN file
//!   outlined (the hot-path highlight: single winner — cost desc, then path
//!   asc — elected over the POST-cap node set, i.e. among the files actually
//!   drawn; a file dropped by the top-50 cap can never be outlined);
//! - `charts/layer-entry.svg` — entry-stratified cost map: files stratified
//!   by BFS distance from resolved entry points over the non-LOW call edge
//!   set, entry-first capped, heat by per-file function cost, single
//!   hot-path outline among shown entry-reachable files;
//! - `charts/layer-bands.svg` — middle bands (A-pf-077): files grouped under
//!   capped components in vertical bands, intra-band include/resolved-call
//!   edges drawn under the nodes, cross-band file edges aggregated into
//!   counted ribbons in a right-hand gutter plus an honest outside-scope
//!   caption.
//!
//! Layer-set decision history (pinned): the middle "files grouped under
//! component bands" layer was initially considered and NOT built — the
//! slice-1 per-component detail pages (`component-<rank>-<slug>.svg`) draw
//! INTRA-component file edges only, while the workspace draws aggregates, so
//! a cross-component FILE edge (e.g. `src/a/x.c → src/b/y.c`) was visible
//! ONLY as an aggregated component-level edge — in NEITHER the workspace
//! (aggregates only) NOR any detail page (intra only). The bands view that
//! closes exactly that gap landed as an explicit NEW layer (I3, A-pf-077),
//! never as a silent reshape of the existing two. The entry-stratified layer
//! is likewise a NEW third file-level view, added alongside — not instead
//! of — the degree-ordered file hot-path map.
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

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{
    COMPONENT_CAP, FINDING_LEGEND, FINDING_LEGEND_ROW, FindingIndex, HEAT_LEGEND_W, NODE_CAP,
    REACHABLE_OVERFLOW, barycenter_order, cap_nodes, escape_xml, est_text_w, file_heat_costs,
    heat_color, heat_ink, legend_band, legend_top, parent_dir, push_finding_dot,
    push_finding_legend, push_heat_legend, reachable_files, resolved_pairs, truncate_label,
};
use crate::graph::{break_cycles, topo_layers};
use crate::schema::{
    ComponentEntry, FileEntry, FindingEntry, FunctionStats, GraphsSection, format_fixed_point,
    round_half_up,
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

// Component-bands geometry (I3, A-pf-077). Bands stack vertically; files sit
// in one horizontal row per band, with a fixed right-hand ribbon gutter.
const LB_MARGIN: i64 = 40;
const LB_HEADER_BAND: i64 = 64;
const LB_BAND_HEADER: i64 = 24;
const LB_BAND_PAD: i64 = 12;
const LB_FILE_W: i64 = 150;
const LB_FILE_H: i64 = 30;
const LB_FILE_STEP: i64 = 180;
const LB_BAND_H: i64 = LB_BAND_HEADER + LB_BAND_PAD + LB_FILE_H + LB_BAND_PAD;
const LB_BAND_GAP: i64 = 16;
const LB_MIN_BAND_W: i64 = 220;
const LB_CAPTION_BAND: i64 = 40;
/// Per-band file cap (A-pf-077): files past this many are summarized by the
/// `…+N more files in shown bands` caption, never silently dropped.
const BAND_FILE_CAP: usize = 10;
const LB_GUTTER_W: i64 = 120;

/// Entry-stratified layer: deepest column rendered individually; farther
/// files fold into one `depth ≥ 9` column so the x-axis stays bounded.
const ENTRY_DEPTH_CAP: u32 = 8;

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
    findings: &FindingIndex,
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
    let subtitle = "heat = candidate finding density per KLOC — profile to confirm";
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
        .max(LW_MARGIN + HEAT_LEGEND_W + 10)
        .max(LW_MARGIN + est_text_w(FINDING_LEGEND, 11.0) + 10);
    for caption in &captions {
        width = width.max(LW_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height = 2 * LW_MARGIN
        + LW_HEADER_BAND
        + rows as i64 * LW_ROW_H
        + legend_band(captions.len())
        + FINDING_LEGEND_ROW;

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
        if let Some(dot) = findings.for_component(node) {
            push_finding_dot(&mut svg, x + LW_NODE_W - 5, y + 5, dot, LW_NODE_W >= 56);
        }
    }
    svg.push_str("</g>\n");

    // Annotation legend (above the heat ramp), then the heat legend, then
    // captions bottom row first so the band fits content exactly.
    let legend_y = legend_top(height, captions.len());
    push_finding_legend(&mut svg, LW_MARGIN, legend_y - FINDING_LEGEND_ROW + 10);
    push_heat_legend(&mut svg, LW_MARGIN, legend_y);
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
/// (resolve filter → the shared reachability-aware top-50 cap → dedupe+break
/// cycles → topo layers → 3-pass barycenter → integer grid), heat by per-file
/// function cost (Σ `function_heat_cost` — v2 preferred, v1 fallback —
/// normalized against the layer maximum), with the
/// highest-cost SHOWN file given the distinct hot-path outline: a SINGLE
/// winner (cost desc, then path asc) elected over the POST-cap node set —
/// the caption states this scope, so the highlight always discriminates.
///
/// An empty node list yields the deterministic stub.
#[must_use]
pub fn render_layer_file(
    graphs: &GraphsSection,
    functions: &[FunctionStats],
    findings: &FindingIndex,
) -> Vec<u8> {
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
    let file_cost = file_heat_costs(functions);
    let reachable = reachable_files(functions);
    let (nodes, total) = cap_nodes(&nodes, &all_pairs, &file_cost, &reachable);
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

    // Per-file heat is the shared v2-preferred selector (see
    // `charts::function_heat_cost`); the layer-global maximum normalizes the
    // heat ramp. The hot-path outline elects a SINGLE winner over the
    // POST-cap (drawn) node set — cost desc, then path asc, so ties always
    // discriminate to one file — and the caption below states this scope
    // truthfully.
    let max_cost = nodes
        .iter()
        .map(|n| file_cost.get(n).copied().unwrap_or(0.0))
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
    let subtitle = "heat = candidate function cost (v2 where available; v1 fallback); hot-path outlined (top-50 + reachable reserve) — profile to confirm";
    // Caption rows bottom-first: cap marker, cost/hot-path note, cycle note.
    let mut captions: Vec<String> = Vec::new();
    if total > nodes.len() {
        captions.push(format!("showing top {} of {} files", nodes.len(), total));
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
        .max(LF_MARGIN + HEAT_LEGEND_W + 10)
        .max(LF_MARGIN + est_text_w(FINDING_LEGEND, 11.0) + 10);
    for caption in &captions {
        width = width.max(LF_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height = 2 * LF_MARGIN
        + LF_HEADER_BAND
        + rows as i64 * LF_ROW_H
        + legend_band(captions.len())
        + FINDING_LEGEND_ROW;

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
        if let Some(dot) = findings.for_file(node) {
            push_finding_dot(&mut svg, x + LF_NODE_W - 5, y + 5, dot, LF_NODE_W >= 56);
        }
    }
    svg.push_str("</g>\n");

    // Annotation legend (above the heat ramp), then the heat legend, then
    // captions bottom row first so the band fits content exactly.
    let legend_y = legend_top(height, captions.len());
    push_finding_legend(&mut svg, LF_MARGIN, legend_y - FINDING_LEGEND_ROW + 10);
    push_heat_legend(&mut svg, LF_MARGIN, legend_y);
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

/// Render `charts/layer-entry.svg` (I3, `map-build`): an entry-stratified
/// cost map.
///
/// Universe = include-graph node paths (sorted/deduped). REAL entry points
/// are `entry_kind` values other than the `unknown-entry` honesty stub; stub
/// files are captioned and never seed the strata. BFS distance is computed
/// BEFORE the cap over the FULL non-LOW call pair set, so a path through a
/// capped-out intermediate still yields the correct depth. Election is
/// entry-first and bounded by [`NODE_CAP`] + [`REACHABLE_OVERFLOW`]: real
/// entry files by cost desc/path asc first, non-entry files by non-LOW call
/// degree desc/path asc fill the base slots, then up to
/// [`REACHABLE_OVERFLOW`] cut files REACHABLE in the distance map are
/// reserved (cost desc/path asc) — stub-only files never qualify. Columns
/// are the distances actually present, ascending, capped at
/// [`ENTRY_DEPTH_CAP`]; deeper files fold into one `depth ≥ 9` column.
/// Shown files no entry can reach land in a final region sub-layered with
/// [`topo_layers`] on the induced unreached subgraph. A document with no
/// REAL entry points falls back to a plain topo call-DAG (captioned).
/// LOW-confidence call edges are EXCLUDED from the strata (name-based,
/// ADR-0001) and counted in a caption.
///
/// An empty node list yields the deterministic stub.
#[must_use]
pub fn render_layer_entry(
    graphs: &GraphsSection,
    functions: &[FunctionStats],
    findings: &FindingIndex,
) -> Vec<u8> {
    let mut universe: Vec<String> = graphs
        .include_graph
        .nodes
        .iter()
        .map(|n| n.path.clone())
        .collect();
    universe.sort();
    universe.dedup();
    if universe.is_empty() {
        return entry_stub();
    }
    let total = universe.len();
    let universe_set: BTreeSet<&str> = universe.iter().map(String::as_str).collect();

    let file_cost = file_heat_costs(functions);
    let cost_of = |path: &str| file_cost.get(path).copied().unwrap_or(0.0);
    // Real entry points only: `unknown-entry` is an honesty stub, not an
    // entry. Stub files are tracked separately so they can be captioned and
    // can never seed the strata or earn the reachability reserve.
    let real_entry_files: BTreeSet<String> = functions
        .iter()
        .filter(|function| {
            function
                .entry_kind
                .as_deref()
                .is_some_and(|kind| kind != "unknown-entry")
        })
        .map(|function| function.path.clone())
        .collect();
    let stub_files: BTreeSet<String> = functions
        .iter()
        .filter(|function| function.entry_kind.as_deref() == Some("unknown-entry"))
        .map(|function| function.path.clone())
        .filter(|path| !real_entry_files.contains(path))
        .collect();
    let entry_set: BTreeSet<&str> = real_entry_files.iter().map(String::as_str).collect();

    // Resolved (non-LOW) call pairs over the full graph: the strata input.
    // LOW-confidence edges are counted, never drawn (ADR-0001).
    let mut call_pair_set: BTreeSet<(String, String)> = BTreeSet::new();
    let mut low_count = 0usize;
    if let Some(call_graph) = &graphs.call_graph {
        for edge in &call_graph.edges {
            if edge.confidence == "LOW" {
                low_count += 1;
            } else if let Some(to) = &edge.to_path {
                call_pair_set.insert((edge.from_path.clone(), to.clone()));
            }
        }
    }
    let call_pairs: Vec<(String, String)> = call_pair_set.into_iter().collect();

    // Degree over non-LOW call pairs (universe-restricted: phantom endpoints
    // never earn degree, mirroring `cap_nodes`).
    let mut degree: BTreeMap<&str, usize> = universe.iter().map(|p| (p.as_str(), 0)).collect();
    for (from, to) in &call_pairs {
        if universe_set.contains(from.as_str()) && universe_set.contains(to.as_str()) {
            if let Some(d) = degree.get_mut(from.as_str()) {
                *d += 1;
            }
            if let Some(d) = degree.get_mut(to.as_str()) {
                *d += 1;
            }
        }
    }

    // ---- strata: BFS distance from REAL entry files (before the cap) ----
    // The full non-LOW pair set is the strata input: a path through a file
    // that the cap will drop still determines the depth of a shown file.
    let mut adjacency: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (from, to) in &call_pairs {
        adjacency
            .entry(from.as_str())
            .or_default()
            .insert(to.as_str());
    }
    let mut distance: BTreeMap<&str, u32> = BTreeMap::new();
    let mut frontier: VecDeque<&str> = VecDeque::new();
    for seed in &real_entry_files {
        if distance.insert(seed.as_str(), 0).is_none() {
            frontier.push_back(seed.as_str());
        }
    }
    while let Some(node) = frontier.pop_front() {
        let d = distance.get(node).copied().unwrap_or(0);
        if let Some(nexts) = adjacency.get(node) {
            for next in nexts {
                if !distance.contains_key(*next) {
                    distance.insert(*next, d.saturating_add(1));
                    frontier.push_back(*next);
                }
            }
        }
    }

    // ---- entry-first cap -------------------------------------------------
    let mut selected: Vec<String> = Vec::new();
    let mut selected_set: BTreeSet<String> = BTreeSet::new();
    let mut entry_ranked: Vec<&str> = real_entry_files
        .iter()
        .map(String::as_str)
        .filter(|p| universe_set.contains(*p))
        .collect();
    entry_ranked.sort_by(|a, b| {
        cost_of(b)
            .partial_cmp(&cost_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    for path in entry_ranked.into_iter().take(NODE_CAP) {
        selected.push(path.to_string());
        selected_set.insert(path.to_string());
    }

    // Base slots filled by non-entry nodes, degree desc then path asc.
    let base_slots = NODE_CAP.saturating_sub(selected.len());
    let mut non_entry: Vec<&str> = universe
        .iter()
        .map(String::as_str)
        .filter(|p| !entry_set.contains(*p) && !selected_set.contains(*p))
        .collect();
    non_entry.sort_by(|a, b| {
        degree
            .get(*b)
            .copied()
            .unwrap_or(0)
            .cmp(&degree.get(*a).copied().unwrap_or(0))
            .then(a.cmp(b))
    });
    for path in non_entry.into_iter().take(base_slots) {
        selected.push(path.to_string());
        selected_set.insert(path.to_string());
    }

    // Reachability reserve: cut non-entry files an entry can reach, cost
    // desc then path asc, bounded by REACHABLE_OVERFLOW.
    let mut reserve: Vec<&str> = universe
        .iter()
        .map(String::as_str)
        .filter(|p| {
            !entry_set.contains(*p) && !selected_set.contains(*p) && distance.contains_key(*p)
        })
        .collect();
    reserve.sort_by(|a, b| {
        cost_of(b)
            .partial_cmp(&cost_of(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    for path in reserve.into_iter().take(REACHABLE_OVERFLOW) {
        selected.push(path.to_string());
        selected_set.insert(path.to_string());
    }

    selected.sort();
    let nodes = selected;
    let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();

    // Edges among shown nodes: dedupe + break cycles (kept edges only drawn).
    let shown_pairs: Vec<(String, String)> = call_pairs
        .iter()
        .filter(|(from, to)| node_set.contains(from.as_str()) && node_set.contains(to.as_str()))
        .cloned()
        .collect();
    let cycle = break_cycles(&shown_pairs);
    let broken_count = cycle.broken.len();
    let kept = cycle.kept;

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let mut unreached: Vec<String> = Vec::new();
    let mut folded_depth: usize = 0;
    if real_entry_files.is_empty() {
        // No real entry points recorded: plain topo call DAG fallback.
        rows = topo_layers(&nodes, &kept);
        labels = (0..rows.len()).map(|d| format!("layer {d}")).collect();
    } else {
        let mut reached_rows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for node in &nodes {
            match distance.get(node.as_str()) {
                Some(&d) => reached_rows.entry(d).or_default().push(node.clone()),
                None => unreached.push(node.clone()),
            }
        }
        // Only distances actually present, ascending (no empty columns),
        // up to the cap; everything deeper folds into ONE final column.
        let mut folded: Vec<String> = Vec::new();
        for (d, members) in reached_rows {
            if d <= ENTRY_DEPTH_CAP {
                labels.push(if d == 0 {
                    "entry".to_string()
                } else {
                    format!("depth {d}")
                });
                rows.push(members);
            } else {
                folded.extend(members);
            }
        }
        if !folded.is_empty() {
            folded.sort();
            folded_depth = folded.len();
            labels.push(format!("depth ≥ {}", ENTRY_DEPTH_CAP + 1));
            rows.push(folded);
        }
        unreached.sort();
        if !unreached.is_empty() {
            let unreached_set: BTreeSet<&str> = unreached.iter().map(String::as_str).collect();
            let induced: Vec<(String, String)> = kept
                .iter()
                .filter(|(from, to)| {
                    unreached_set.contains(from.as_str()) && unreached_set.contains(to.as_str())
                })
                .cloned()
                .collect();
            let sublayers = topo_layers(&unreached, &induced);
            let n = sublayers.len();
            for (i, layer) in sublayers.into_iter().enumerate() {
                labels.push(if n == 1 {
                    "not reached".to_string()
                } else {
                    format!("not reached ({} of {n})", i + 1)
                });
                rows.push(layer);
            }
        }
    }

    // Exactly one 3-pass barycenter run orders nodes within each row.
    barycenter_order(&mut rows, &kept);

    let mut pos: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for (col, members) in rows.iter().enumerate() {
        for (i, node) in members.iter().enumerate() {
            pos.insert(
                node.as_str(),
                (
                    LF_MARGIN + col as i64 * LF_LAYER_W,
                    LF_MARGIN + LF_HEADER_BAND + i as i64 * LF_ROW_H,
                ),
            );
        }
    }

    let max_cost = nodes.iter().map(|n| cost_of(n)).fold(0.0_f64, f64::max);
    let entry_reachable_shown: Vec<&str> = nodes
        .iter()
        .filter(|n| distance.contains_key(n.as_str()))
        .map(String::as_str)
        .collect();
    let max_entry_cost = entry_reachable_shown
        .iter()
        .map(|n| cost_of(n))
        .fold(0.0_f64, f64::max);
    let hot: Option<&str> = if max_entry_cost <= 0.0 {
        None
    } else {
        entry_reachable_shown
            .iter()
            .copied()
            .filter(|n| cost_of(n) >= max_entry_cost)
            .min()
    };

    let cols = rows.len();
    let row_max = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let grid_w = 2 * LF_MARGIN + cols.saturating_sub(1) as i64 * LF_LAYER_W + LF_NODE_W;
    let title = format!(
        "Entry-stratified cost map — {} {}",
        nodes.len(),
        if nodes.len() == 1 { "file" } else { "files" },
    );
    let subtitle = "heat = candidate file function cost (v2 where available; v1 fallback), normalized over shown files; strata = resolved entry reachability — profile to confirm";

    let mut captions: Vec<String> = Vec::new();
    if nodes.len() < total {
        captions.push(format!("showing top {} of {} files", nodes.len(), total));
    }
    if max_entry_cost <= 0.0 {
        captions.push(if max_cost <= 0.0 {
            "no function cost recorded".to_string()
        } else {
            "no entry-reachable function cost recorded".to_string()
        });
    } else if let Some(h) = hot {
        captions.push(format!(
            "hot-path: {} at max cost among shown entry-reachable files",
            truncate_label(h)
        ));
    }
    if call_pairs.is_empty() {
        captions.push(
            "0 resolved call edges; reachability = entries only (semantic tier off)".to_string(),
        );
    } else {
        let edge_word = if call_pairs.len() == 1 {
            "edge"
        } else {
            "edges"
        };
        let entry_word = if real_entry_files.len() == 1 {
            "file"
        } else {
            "files"
        };
        captions.push(format!(
            "{} resolved call {edge_word} ({} among shown) · {} entry {entry_word}",
            call_pairs.len(),
            shown_pairs.len(),
            real_entry_files.len()
        ));
    }
    if !stub_files.is_empty() {
        let n = stub_files.len();
        captions.push(if n == 1 {
            "1 unknown-entry stub file (no entry detected; shown as not reached)".to_string()
        } else {
            format!("{n} unknown-entry stub files (no entry detected; shown as not reached)")
        });
    }
    if folded_depth > 0 {
        captions.push(format!(
            "{folded_depth} files at depth ≥ {} folded into one column",
            ENTRY_DEPTH_CAP + 1
        ));
    }
    if low_count > 0 {
        let edge_word = if low_count == 1 { "edge" } else { "edges" };
        captions.push(format!(
            "{low_count} LOW-confidence call {edge_word} excluded (name-based; ADR-0001)"
        ));
    }
    if broken_count > 0 {
        let edge_word = if broken_count == 1 { "edge" } else { "edges" };
        captions.push(format!("{broken_count} cyclic call {edge_word} broken"));
    }
    if !unreached.is_empty() {
        let n = unreached.len();
        captions.push(if n == 1 {
            "1 file has no resolved entry path".to_string()
        } else {
            format!("{n} files have no resolved entry path")
        });
    }
    if real_entry_files.is_empty() {
        captions.push("no entry points recorded; topo fallback".to_string());
    }

    let mut width = grid_w
        .max(LF_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(LF_MARGIN + est_text_w(subtitle, 11.0) + 10)
        .max(LF_MARGIN + HEAT_LEGEND_W + 10)
        .max(LF_MARGIN + est_text_w(FINDING_LEGEND, 11.0) + 10);
    for caption in &captions {
        width = width.max(LF_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height = 2 * LF_MARGIN
        + LF_HEADER_BAND
        + row_max as i64 * LF_ROW_H
        + legend_band(captions.len())
        + FINDING_LEGEND_ROW;

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
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{}</text>\n",
        LF_MARGIN + 36,
        escape_xml(subtitle),
    ));
    // Column labels in the header band: entry / depth k / not reached.
    for (col, label) in labels.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#4a6fa5\">{}</text>\n",
            LF_MARGIN + col as i64 * LF_LAYER_W,
            LF_MARGIN + LF_HEADER_BAND - 10,
            escape_xml(label),
        ));
    }

    // Edges first (under nodes), source right edge → target left edge.
    if !kept.is_empty() {
        svg.push_str("<g stroke=\"#999999\" stroke-width=\"1\">\n");
        for (from, to) in &kept {
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

    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for node in &nodes {
        let Some(&(x, y)) = pos.get(node.as_str()) else {
            continue;
        };
        let level = if max_cost <= 0.0 {
            0
        } else {
            round_half_up(cost_of(node) / max_cost * 10.0, 0).clamp(0, 10) as u32
        };
        let fill = heat_color(level);
        let (stroke, stroke_w) = if hot == Some(node.as_str()) {
            (LF_HOT_STROKE, 2)
        } else {
            (LAYER_NODE_STROKE, 1)
        };
        let label = escape_xml(&truncate_label(node));
        let text_x = x + 6 + 2;
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
        if let Some(dot) = findings.for_file(node) {
            push_finding_dot(&mut svg, x + LF_NODE_W - 5, y + 5, dot, LF_NODE_W >= 56);
        }
    }
    svg.push_str("</g>\n");

    // Annotation legend (above the heat ramp), then the heat legend, then
    // captions bottom row first so the band fits content exactly.
    let legend_y = legend_top(height, captions.len());
    push_finding_legend(&mut svg, LF_MARGIN, legend_y - FINDING_LEGEND_ROW + 10);
    push_heat_legend(&mut svg, LF_MARGIN, legend_y);
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

/// Render `charts/layer-bands.svg` (I3, A-pf-077): files grouped under
/// capped components in vertical bands.
///
/// Membership is the union of include-graph nodes and `files[].path` whose
/// `parent_dir` equals the component `path_prefix` (the same mapping the
/// per-component pages use). Bands stack in `(rank, prefix)` order; each band
/// keeps [`BAND_FILE_CAP`] files ordered cost desc/path asc and captions the
/// remainder. Intra-band include/resolved-call edges are drawn under the
/// nodes (intra-band pairs are deduped and cycle-broken via the shared
/// policy, with the broken count captioned); cross-band file edges are
/// aggregated by ordered band pair into counted ribbons in a right-hand
/// gutter. LOW-confidence call edges are excluded and counted, as are
/// cross-component edges whose endpoints fall outside the shown
/// file/component scope.
///
/// No components yields the deterministic stub; present-but-fileless
/// components still render their bands (`no files recorded`).
#[must_use]
pub fn render_layer_bands(
    components: &[ComponentEntry],
    graphs: Option<&GraphsSection>,
    files: &[FileEntry],
    functions: &[FunctionStats],
    findings: &FindingIndex,
) -> Vec<u8> {
    let total_components = components.len();
    let mut sorted: Vec<&ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    sorted.retain(|c| c.rank <= u32::try_from(COMPONENT_CAP).unwrap_or(u32::MAX));
    sorted.truncate(COMPONENT_CAP);
    if sorted.is_empty() {
        return bands_stub();
    }
    let shown_components = sorted.len();

    // Membership union (same mapping as src/charts/component.rs).
    let mut members: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for comp in &sorted {
        members.insert(comp.path_prefix.clone(), BTreeSet::new());
    }
    let mut universe_files: BTreeSet<String> = BTreeSet::new();
    if let Some(graphs) = graphs {
        for node in &graphs.include_graph.nodes {
            universe_files.insert(node.path.clone());
        }
    }
    for file in files {
        universe_files.insert(file.path.clone());
    }
    for path in &universe_files {
        if let Some(set) = members.get_mut(parent_dir(path)) {
            set.insert(path.clone());
        }
    }
    let outside_components = universe_files
        .iter()
        .filter(|path| !members.contains_key(parent_dir(path)))
        .count();

    let file_cost = file_heat_costs(functions);
    let cost_of = |path: &str| file_cost.get(path).copied().unwrap_or(0.0);

    // Resolved edge set: include pairs + non-LOW call pairs (deduped/sorted).
    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut low_count = 0usize;
    if let Some(graphs) = graphs {
        for pair in resolved_pairs(&graphs.include_graph.edges) {
            pairs.insert(pair);
        }
        if let Some(call_graph) = &graphs.call_graph {
            for edge in &call_graph.edges {
                if edge.confidence == "LOW" {
                    low_count += 1;
                } else if let Some(to) = &edge.to_path {
                    pairs.insert((edge.from_path.clone(), to.clone()));
                }
            }
        }
    }

    // Per-band full membership + capped, cost-ordered file row.
    let mut bands: Vec<(u32, String, Vec<String>, Vec<String>)> = Vec::new();
    for comp in &sorted {
        let empty = BTreeSet::new();
        let set = members.get(&comp.path_prefix).unwrap_or(&empty);
        let full: Vec<String> = set.iter().cloned().collect();
        let mut shown = full.clone();
        shown.sort_by(|a, b| {
            cost_of(b)
                .partial_cmp(&cost_of(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(b))
        });
        shown.truncate(BAND_FILE_CAP);
        bands.push((comp.rank, comp.path_prefix.clone(), full, shown));
    }
    let hidden_in_bands: usize = bands
        .iter()
        .map(|(_, _, full, shown)| full.len().saturating_sub(shown.len()))
        .sum();

    let mut shown_band: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, (_, _, _, shown)) in bands.iter().enumerate() {
        for path in shown {
            shown_band.insert(path.as_str(), index);
        }
    }

    // Intra-band edges vs aggregated cross-band counts vs outside-scope
    // cross edges.
    let mut intra: Vec<(String, String)> = Vec::new();
    let mut cross: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut outside_cross = 0usize;
    for (from, to) in &pairs {
        match (shown_band.get(from.as_str()), shown_band.get(to.as_str())) {
            (Some(&a), Some(&b)) => {
                if a == b {
                    intra.push((from.clone(), to.clone()));
                } else {
                    *cross.entry((a, b)).or_insert(0) += 1;
                }
            }
            _ => {
                if parent_dir(from) != parent_dir(to) {
                    outside_cross += 1;
                }
            }
        }
    }
    // Intra-band edges go through the shared cycle policy so a 2-cycle is
    // drawn once, not twice (broken edges are captioned, never silent).
    let intra_cycle = break_cycles(&intra);
    let intra_broken = intra_cycle.broken.len();
    let intra = intra_cycle.kept;

    let max_cost = bands
        .iter()
        .flat_map(|(_, _, _, shown)| shown.iter())
        .map(|p| cost_of(p))
        .fold(0.0_f64, f64::max);

    // Geometry.
    let max_shown = bands
        .iter()
        .map(|(_, _, _, shown)| shown.len())
        .max()
        .unwrap_or(0);
    let file_row_w = if max_shown == 0 {
        LB_FILE_W
    } else {
        (max_shown as i64 - 1) * LB_FILE_STEP + LB_FILE_W
    };
    let mut band_w = (2 * LB_BAND_PAD + file_row_w).max(LB_MIN_BAND_W);
    for (rank, prefix, full, _) in &bands {
        band_w =
            band_w.max(2 * LB_BAND_PAD + est_text_w(&band_header(*rank, prefix, full.len()), 11.0));
    }
    let gutter_x = LB_MARGIN + band_w + 30;
    let gutter_w = if cross.is_empty() { 0 } else { LB_GUTTER_W };

    let title = format!(
        "Component bands — {} {}",
        shown_components,
        if shown_components == 1 {
            "component"
        } else {
            "components"
        },
    );
    let subtitle = "heat = candidate file function cost (v2 where available; v1 fallback), normalized over shown files; files grouped by capped component; cross-component edges aggregated — profile to confirm";

    let mut captions: Vec<String> = Vec::new();
    if shown_components < total_components {
        captions.push(format!(
            "showing top {} of {} components",
            shown_components, total_components
        ));
    }
    if outside_components > 0 {
        captions.push(format!(
            "…+{outside_components} more files outside shown components"
        ));
    }
    if hidden_in_bands > 0 {
        captions.push(format!("…+{hidden_in_bands} more files in shown bands"));
    }
    if outside_cross > 0 {
        captions.push(format!(
            "…+{outside_cross} more cross-component edges outside shown scope"
        ));
    }
    if !cross.is_empty() {
        captions
            .push("cross-component file edges aggregated (includes + resolved calls)".to_string());
    }
    if low_count > 0 {
        let edge_word = if low_count == 1 { "edge" } else { "edges" };
        captions.push(format!(
            "{low_count} LOW-confidence call {edge_word} excluded (name-based; ADR-0001)"
        ));
    }
    if intra_broken > 0 {
        let edge_word = if intra_broken == 1 { "edge" } else { "edges" };
        captions.push(format!(
            "{intra_broken} cyclic intra-band {edge_word} broken"
        ));
    }

    let mut width = 2 * LB_MARGIN + band_w + gutter_w;
    width = width
        .max(LB_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(LB_MARGIN + est_text_w(subtitle, 11.0) + 10)
        .max(LB_MARGIN + HEAT_LEGEND_W + 10)
        .max(LB_MARGIN + est_text_w(FINDING_LEGEND, 11.0) + 10);
    for caption in &captions {
        width = width.max(LB_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let bands_h = bands.len() as i64 * (LB_BAND_H + LB_BAND_GAP);
    let height =
        2 * LB_MARGIN + LB_HEADER_BAND + bands_h + legend_band(captions.len()) + FINDING_LEGEND_ROW;

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LB_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{}</text>\n",
        LB_MARGIN + 16,
        escape_xml(&title),
    ));
    svg.push_str(&format!(
        "<text x=\"{LB_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{}</text>\n",
        LB_MARGIN + 36,
        escape_xml(subtitle),
    ));

    // Band backgrounds + headers.
    let mut band_tops: Vec<i64> = Vec::with_capacity(bands.len());
    let mut band_top = LB_MARGIN + LB_HEADER_BAND;
    for (rank, prefix, full, _) in &bands {
        band_tops.push(band_top);
        svg.push_str(&format!(
            "<rect x=\"{LB_MARGIN}\" y=\"{band_top}\" width=\"{band_w}\" height=\"{LB_BAND_H}\" fill=\"#f3f6fb\" stroke=\"#d0d9e8\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#2b3a55\">{}</text>\n",
            LB_MARGIN + LB_BAND_PAD,
            band_top + 18,
            escape_xml(&band_header(*rank, prefix, full.len())),
        ));
        band_top += LB_BAND_H + LB_BAND_GAP;
    }

    // File positions (inside their band row).
    let mut pos: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for (index, (_, _, _, shown)) in bands.iter().enumerate() {
        let y = band_tops[index] + LB_BAND_HEADER + LB_BAND_PAD;
        for (i, path) in shown.iter().enumerate() {
            pos.insert(
                path.as_str(),
                (LB_MARGIN + LB_BAND_PAD + i as i64 * LB_FILE_STEP, y),
            );
        }
    }

    // Intra-band edges under the nodes.
    if !intra.is_empty() {
        svg.push_str("<g stroke=\"#999999\" stroke-width=\"1\">\n");
        for (from, to) in &intra {
            let Some(&(fx, fy)) = pos.get(from.as_str()) else {
                continue;
            };
            let Some(&(tx, ty)) = pos.get(to.as_str()) else {
                continue;
            };
            let x1 = fx + LB_FILE_W;
            let y1 = fy + LB_FILE_H / 2;
            let y2 = ty + LB_FILE_H / 2;
            svg.push_str(&format!(
                "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\"/>\n"
            ));
        }
        svg.push_str("</g>\n");
    }

    // Cross-band counted ribbons in the right-hand gutter.
    if !cross.is_empty() {
        svg.push_str("<g font-family=\"monospace\" font-size=\"11\" fill=\"#2b3a55\">\n");
        for (index, ((from_band, to_band), count)) in cross.iter().enumerate() {
            let x = gutter_x + 10 + (index as i64 % 4) * 8;
            let y1 = band_tops[*from_band] + LB_BAND_H / 2;
            let y2 = band_tops[*to_band] + LB_BAND_H / 2;
            let stroke_w = 1 + (*count).min(5);
            svg.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" stroke=\"#7a8ba6\" stroke-width=\"{stroke_w}\"/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{count}</text>\n",
                x + 6,
                (y1 + y2) / 2 + 4,
            ));
        }
        svg.push_str("</g>\n");
    }

    // File nodes: heat fill + truncated label.
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for (_, _, _, shown) in &bands {
        for path in shown {
            let Some(&(x, y)) = pos.get(path.as_str()) else {
                continue;
            };
            let level = if max_cost <= 0.0 {
                0
            } else {
                round_half_up(cost_of(path) / max_cost * 10.0, 0).clamp(0, 10) as u32
            };
            let fill = heat_color(level);
            let label = escape_xml(&truncate_label(path));
            let text_x = x + 6 + 2;
            let text_y = y + LB_FILE_H / 2 + 4;
            let ink = match heat_ink(level) {
                Some(ink) => format!(" fill=\"{ink}\""),
                None => String::new(),
            };
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{LB_FILE_W}\" height=\"{LB_FILE_H}\" fill=\"{fill}\" stroke=\"{LAYER_NODE_STROKE}\"/>\n"
            ));
            svg.push_str(&format!(
                "<text x=\"{text_x}\" y=\"{text_y}\"{ink}>{label}</text>\n"
            ));
            if let Some(dot) = findings.for_file(path) {
                push_finding_dot(&mut svg, x + LB_FILE_W - 5, y + 5, dot, LB_FILE_W >= 56);
            }
        }
    }
    svg.push_str("</g>\n");

    // Annotation legend (above the heat ramp), then the heat legend, then
    // captions bottom row first so the band fits content exactly.
    let legend_y = legend_top(height, captions.len());
    push_finding_legend(&mut svg, LB_MARGIN, legend_y - FINDING_LEGEND_ROW + 10);
    push_heat_legend(&mut svg, LB_MARGIN, legend_y);
    for (k, caption) in captions.iter().enumerate() {
        let y = height - 12 - k as i64 * 16;
        let out = escape_xml(caption).replace('…', "&#8230;");
        svg.push_str(&format!(
            "<text x=\"{LB_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{out}</text>\n"
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

fn band_header(rank: u32, prefix: &str, full_count: usize) -> String {
    if full_count == 0 {
        format!("{rank}. {} — no files recorded", truncate_label(prefix))
    } else {
        let word = if full_count == 1 { "file" } else { "files" };
        format!("{rank}. {} — {full_count} {word}", truncate_label(prefix))
    }
}

/// Render `charts/layer-findings.svg` (I4, `map-build`): a severity-weighted
/// findings map.
///
/// Node set = include-graph nodes UNION the file paths that carry findings
/// (sorted). When the universe exceeds [`NODE_CAP`], nodes are elected by
/// `(weighted desc, path asc)` where `weighted = 3*hot + 2*warn + 1*info`
/// (fixed weights, stated in the subtitle); the kept set is re-sorted
/// path-ascending for the shared DAG layout (break cycles → topo layers →
/// 3-pass barycenter). Heat level = node weight / max shown weight (0..10).
/// Every node carries its finding dot; the single hottest shown file is
/// outlined (weight desc, then path asc). Zero findings yields a
/// deterministic stub — never a blank chart.
///
/// `index` is the shared roll-up built ONCE by [`super::render`] (this
/// renderer never rebuilds it); `findings` supplies the exact set of paths
/// to union in and the empty-document stub signal.
#[must_use]
pub fn render_layer_findings(
    graphs: Option<&GraphsSection>,
    findings: &[FindingEntry],
    index: &FindingIndex,
) -> Vec<u8> {
    if findings.is_empty() {
        return findings_stub();
    }

    let mut universe: BTreeSet<String> = BTreeSet::new();
    if let Some(graphs) = graphs {
        for node in &graphs.include_graph.nodes {
            universe.insert(node.path.clone());
        }
    }
    for finding in findings {
        universe.insert(finding.path.clone());
    }
    let total = universe.len();
    let weight_of = |path: &str| index.for_file(path).map_or(0, |dot| dot.weighted());

    let mut ranked: Vec<String> = universe.into_iter().collect();
    ranked.sort_by(|a, b| weight_of(b).cmp(&weight_of(a)).then_with(|| a.cmp(b)));
    ranked.truncate(NODE_CAP);
    ranked.sort();
    let nodes = ranked;
    let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();

    let all_pairs: Vec<(String, String)> = graphs
        .map(|g| resolved_pairs(&g.include_graph.edges))
        .unwrap_or_default();
    let pairs: Vec<(String, String)> = all_pairs
        .into_iter()
        .filter(|(from, to)| node_set.contains(from.as_str()) && node_set.contains(to.as_str()))
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

    let max_weight = nodes.iter().map(|n| weight_of(n)).max().unwrap_or(0);
    // Single hottest shown file: weight desc, path asc (ties discriminate).
    let hot: Option<&str> = if max_weight == 0 {
        None
    } else {
        nodes
            .iter()
            .filter(|n| weight_of(n) >= max_weight)
            .map(String::as_str)
            .min()
    };

    let rows = layers.iter().map(Vec::len).max().unwrap_or(1);
    let grid_w = 2 * LF_MARGIN + (layers.len().saturating_sub(1)) as i64 * LF_LAYER_W + LF_NODE_W;
    let title = format!(
        "Findings map — {} {}",
        nodes.len(),
        if nodes.len() == 1 { "file" } else { "files" },
    );
    let subtitle = "heat = severity-weighted finding density (hot 3 / warn 2 / info 1 per candidate) — profile to confirm";
    let mut captions: Vec<String> = Vec::new();
    if total > nodes.len() {
        captions.push(format!(
            "showing top {} of {} files by weighted findings (…+{} more)",
            nodes.len(),
            total,
            total - nodes.len()
        ));
    }
    if broken_count > 0 {
        captions.push(format!(
            "{broken_count} cyclic edges broken (see ARCH-001 findings)"
        ));
    }

    let mut width = grid_w
        .max(LF_MARGIN + est_text_w(&title, 13.0) + 10)
        .max(LF_MARGIN + est_text_w(subtitle, 11.0) + 10)
        .max(LF_MARGIN + HEAT_LEGEND_W + 10)
        .max(LF_MARGIN + est_text_w(FINDING_LEGEND, 11.0) + 10);
    for caption in &captions {
        width = width.max(LF_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height = 2 * LF_MARGIN
        + LF_HEADER_BAND
        + rows as i64 * LF_ROW_H
        + legend_band(captions.len())
        + FINDING_LEGEND_ROW;

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
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{}</text>\n",
        LF_MARGIN + 36,
        escape_xml(subtitle),
    ));

    // Edges first (under nodes): kept DAG edges only.
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

    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for node in &nodes {
        let Some(&(x, y)) = pos.get(node.as_str()) else {
            continue;
        };
        let weight = weight_of(node);
        let level = if max_weight == 0 {
            0
        } else {
            round_half_up(f64::from(weight) / f64::from(max_weight) * 10.0, 0).clamp(0, 10) as u32
        };
        let fill = heat_color(level);
        let (stroke, stroke_w) = if hot == Some(node.as_str()) {
            (LF_HOT_STROKE, 2)
        } else {
            (LAYER_NODE_STROKE, 1)
        };
        let label = escape_xml(&truncate_label(node));
        let text_x = x + 6 + 2;
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
        if let Some(dot) = index.for_file(node) {
            push_finding_dot(&mut svg, x + LF_NODE_W - 5, y + 5, dot, LF_NODE_W >= 56);
        }
    }
    svg.push_str("</g>\n");

    let legend_y = legend_top(height, captions.len());
    push_finding_legend(&mut svg, LF_MARGIN, legend_y - FINDING_LEGEND_ROW + 10);
    push_heat_legend(&mut svg, LF_MARGIN, legend_y);
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

/// Stub for an empty entry-layer universe: header + caption, no nodes.
fn entry_stub() -> Vec<u8> {
    let title = "Entry-stratified cost map — no files";
    let width = (2 * LF_MARGIN + LF_NODE_W).max(LF_MARGIN + est_text_w(title, 13.0) + 10);
    let height = LF_MARGIN + LF_HEADER_BAND + LF_CAPTION_BAND;
    let mut svg = String::with_capacity(1024);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{title}</text>\n",
        LF_MARGIN + 16
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">no files recorded</text>\n",
        height - 12
    ));
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

/// Stub for an empty component list: header + caption, no bands.
fn bands_stub() -> Vec<u8> {
    let title = "Component bands — no components";
    let width = (2 * LB_MARGIN + LB_MIN_BAND_W).max(LB_MARGIN + est_text_w(title, 13.0) + 10);
    let height = LB_MARGIN + LB_HEADER_BAND + LB_CAPTION_BAND;
    let mut svg = String::with_capacity(1024);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LB_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{title}</text>\n",
        LB_MARGIN + 16
    ));
    svg.push_str(&format!(
        "<text x=\"{LB_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">no components recorded</text>\n",
        height - 12
    ));
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

/// Stub for a document with zero recorded findings: header + the honest
/// "nothing to annotate" body (never blank).
fn findings_stub() -> Vec<u8> {
    let title = "Findings map — no findings";
    let body = "no findings recorded — nothing to annotate (heuristic candidates only)";
    let width = (2 * LF_MARGIN + LF_NODE_W)
        .max(LF_MARGIN + est_text_w(title, 13.0) + 10)
        .max(LF_MARGIN + est_text_w(body, 11.0) + 10);
    let height = LF_MARGIN + LF_HEADER_BAND + LF_CAPTION_BAND;
    let mut svg = String::with_capacity(1024);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">{title}</text>\n",
        LF_MARGIN + 16
    ));
    svg.push_str(&format!(
        "<text x=\"{LF_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{body}</text>\n",
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

    #[test]
    fn empty_inputs_yield_deterministic_stubs() {
        let first = render_layer_workspace(&[], None, &FindingIndex::default());
        let second = render_layer_workspace(&[], None, &FindingIndex::default());
        assert_eq!(first, second);
        let text = String::from_utf8(first).expect("SVG UTF-8");
        assert!(
            text.contains("no components recorded"),
            "stub caption:\n{text}"
        );

        let graphs = graphs(&[], &[]);
        let first = render_layer_file(&graphs, &[], &FindingIndex::default());
        let second = render_layer_file(&graphs, &[], &FindingIndex::default());
        assert_eq!(first, second);
        let text = String::from_utf8(first).expect("SVG UTF-8");
        assert!(text.contains("no files recorded"), "stub caption:\n{text}");
    }

    #[test]
    fn workspace_couples_components_over_include_edges() {
        let comps = vec![comp("src/a", 1, 8.0), comp("src/b", 2, 4.0)];
        let g = graphs(&["src/a/x.c", "src/b/y.c"], &[("src/a/x.c", "src/b/y.c")]);
        let first = render_layer_workspace(&comps, Some(&g), &FindingIndex::default());
        let second = render_layer_workspace(&comps, Some(&g), &FindingIndex::default());
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
        let text = String::from_utf8(render_layer_workspace(
            &comps,
            None,
            &FindingIndex::default(),
        ))
        .expect("SVG UTF-8");
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
        let first = render_layer_file(&g, &functions, &FindingIndex::default());
        let second = render_layer_file(&g, &functions, &FindingIndex::default());
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
        let first = render_layer_file(&g, &functions, &FindingIndex::default());
        let second = render_layer_file(&g, &functions, &FindingIndex::default());
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
        let text = String::from_utf8(render_layer_file(&g, &[], &FindingIndex::default()))
            .expect("SVG UTF-8");
        assert!(text.contains("no function cost recorded"));
        assert!(
            !text.contains("stroke=\"#1a1a1a\" stroke-width=\"2\""),
            "no hot outline without cost:\n{text}"
        );
    }

    #[test]
    fn workspace_escapes_hostile_prefixes() {
        let comps = vec![comp("we<ird>&", 1, 1.0)];
        let text = String::from_utf8(render_layer_workspace(
            &comps,
            None,
            &FindingIndex::default(),
        ))
        .expect("SVG UTF-8");
        assert!(
            text.contains("we&lt;ird&gt;&amp;"),
            "hostile prefix not escaped:\n{text}"
        );
        assert!(!text.contains("we<ird>"), "raw hostile bytes leaked");
    }
}
