//! `charts` — deterministic SVG emitter (modules.md §3.7; ADR-0002).
//!
//! Consumes: the final document — graphs.* and components[] ONLY (ADR-0002
//! guardrail 1). Produces: SVG bytes for chart-set v1 ([`render`]:
//! include-graph.svg, component-coupling.svg, hot-components.svg).
//!
//! Responsibilities: byte-deterministic rendering by construction (ADR-0002
//! guardrail 2); XML-escape all label/path text; deterministic node caps with
//! in-chart truncation markers; presence rule — a chart is emitted iff its
//! source section is present and non-empty, otherwise omitted (charts/ may be
//! empty).
//!
//! Determinism mechanics (ADR-0002 guardrail 2, pinned here where the ADR
//! left room):
//! - Every iteration is sorted; hand-built strings fix attribute emission
//!   order by construction.
//! - Coordinates are integers via the shared [`crate::schema::round_half_up`]
//!   — HALF-UP **away from zero** at the given scale, digit-inspection based
//!   (Rust's `format!`/`{:.prec$}` rounds half-to-even, which is NOT
//!   byte-stable against our contract examples, so we never use it for
//!   output text).
//! - C-locale formatting: Rust std float/text formatting is locale-free by
//!   default; no per-thread locale is ever consulted.
//! - No `<metadata>`, no dates/timestamps, no generator IDs, no counters.
//! - Colors are pure functions of document values (`heat_color`,
//!   fixed palette constants).
//! - Label metric is fixed char-width (`CHAR_W` px/char, monospace
//!   assumption) — fonts are never measured.
//! - Labels longer than [`LABEL_MAX_CHARS`] chars are truncated to the LAST
//!   31 chars prefixed with `…` (tail kept because paths identify best by
//!   their basename-ish end; pinned rule, ADR ambiguity resolved here).
//!
//! Non-goals: layout quality beyond "legible" (MVP bar); external
//! charting/layout libraries are banned by ADR-0002.

use std::collections::BTreeMap;

use crate::graph::{break_cycles, topo_layers};
use crate::schema::{
    ComponentEntry, GraphsSection, IncludeEdgeDto, MetricsDocument, format_fixed_point,
    round_half_up,
};

// ---------------------------------------------------------------------------
// Tunables (fixed constants — part of the byte-determinism surface)
// ---------------------------------------------------------------------------

/// Node cap for include-graph.svg (founder-approved): applied PRE-layout,
/// top-N by degree desc, tie path asc.
const NODE_CAP: usize = 50;
/// Component cap for both component charts: entries with rank > 50 dropped,
/// then list truncated to this many after `(rank, path_prefix)` sort.
const COMPONENT_CAP: usize = 50;
/// Fixed char width for monospace label measurement (never measure fonts).
const CHAR_W: f64 = 6.6;
/// Max label length in chars before tail-truncation (see module docs).
const LABEL_MAX_CHARS: usize = 32;

// include-graph.svg geometry
const IG_MARGIN: i64 = 40;
const IG_LAYER_W: i64 = 220;
const IG_ROW_H: i64 = 44;
const IG_NODE_W: i64 = 170;
const IG_NODE_H: i64 = 30;
const IG_CAPTION_BAND: i64 = 40;

// component-coupling.svg geometry
const CC_MARGIN: i64 = 20;
const CC_ROW_LABEL_W: i64 = 190;
const CC_COL_LABEL_H: i64 = 130;
const CC_CELL: i64 = 24;
const CC_LEGEND_BAND: i64 = 44;
/// Distinct overlay hue for LOW-confidence call-edge contribution.
const CC_CALL_FILL: &str = "#3a3aff";
const CC_CALL_OPACITY: &str = "0.55";

// hot-components.svg geometry
const HC_MARGIN: i64 = 20;
const HC_LABEL_W: i64 = 300;
const HC_PLOT_W: i64 = 420;
const HC_ROW_PITCH: i64 = 26;
const HC_BAR_H: i64 = 16;
const HC_CAPTION_BAND: i64 = 44;
/// Bar fill: hottest palette color (pure constant, not value-dependent).
const HC_BAR_FILL: &str = "#b30000";
/// Stub bar length when every density is zero ("minimal stub bars").
const HC_STUB_W: i64 = 2;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Render the chart set v1 for `doc`.
///
/// Returns ordered `(filename, svg bytes)` pairs. Emission rule (ADR-0002
/// guardrail 5): a chart appears IFF its source section is present AND
/// non-empty; an empty document yields an empty list.
#[must_use]
pub fn render(doc: &MetricsDocument) -> Vec<(&'static str, Vec<u8>)> {
    let mut out = Vec::new();
    if let Some(graphs) = &doc.graphs
        && !graphs.include_graph.nodes.is_empty()
    {
        out.push(("include-graph.svg", render_include_graph(graphs)));
    }
    if let Some(components) = &doc.components
        && !components.is_empty()
    {
        let graphs = doc.graphs.as_ref();
        out.push((
            "component-coupling.svg",
            render_component_coupling(components, graphs),
        ));
        out.push(("hot-components.svg", render_hot_components(components)));
    }
    out
}

// ---------------------------------------------------------------------------
// Numeric + text primitives (determinism core)
// ---------------------------------------------------------------------------

// Rounding/formatting live in `crate::schema` (`round_half_up`,
// `format_fixed_point`) — ONE shared implementation for charts and reporter,
// imported above. See its docs for the pinned HALF-UP-away-from-zero,
// digit-inspection algorithm and the negative-zero guarantee.

/// XML-escape `& < > " '` for safe embedding in element text/attributes
/// (ADR-0002 guardrail 3). Single pass over chars — `&` cannot be re-escaped.
#[must_use]
pub fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Truncate a label to at most [`LABEL_MAX_CHARS`] chars: over-long labels
/// keep the LAST `LABEL_MAX_CHARS - 1` chars prefixed with `…` (tail kept:
/// the basename-ish end of a path identifies the file best). Char-boundary
/// safe (operates on chars, not bytes).
#[must_use]
pub fn truncate_label(label: &str) -> String {
    let count = label.chars().count();
    if count <= LABEL_MAX_CHARS {
        return label.to_string();
    }
    let skip = count - (LABEL_MAX_CHARS - 1);
    let tail: String = label.chars().skip(skip).collect();
    format!("…{tail}")
}

/// Heat color for coupling/bars: pure function mapping level `0..=10` onto a
/// white→dark-red ramp (integer arithmetic only — no floats in color space).
#[must_use]
fn heat_color(level: u32) -> String {
    let level = level.min(10);
    let r = 255 - (76 * level + 5) / 10;
    let g = (255 * (10 - level) + 5) / 10;
    format!("#{r:02x}{g:02x}{g:02x}")
}

/// Repo-relative parent directory of a POSIX path; root-level files map to
/// `"."` — EXACTLY mirroring the aggregator's `parent_dir`
/// (src/aggregator/mod.rs), whose `"."` prefixes are the coupling-matrix
/// axis labels. A divergent root mapping here would make every
/// root-level endpoint miss the bump-index lookup and silently drop its
/// edges from component-coupling.svg.
fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => ".",
    }
}

// ---------------------------------------------------------------------------
// include-graph.svg
// ---------------------------------------------------------------------------

/// Resolved internal edge pairs for charting (ADR-0002 chart policy):
/// `resolved == true` AND `to` present. Unresolved edges (to: null) are
/// excluded here — they surface as ARCH-001 findings instead.
fn resolved_pairs(edges: &[IncludeEdgeDto]) -> Vec<(String, String)> {
    edges
        .iter()
        .filter(|e| e.resolved)
        .filter_map(|e| e.to.clone().map(|to| (e.from.clone(), to)))
        .collect()
}

/// Apply the node cap PRE-layout: top-[`NODE_CAP`] nodes by degree desc
/// (degree = incident resolved edges), tie path asc; returns the kept node
/// paths sorted ascending plus the number of dropped files.
///
/// Edge pairs are FIRST filtered to endpoints present in the node set: an
/// include can resolve to a real file that is not a scanned chart node
/// (discovery only picks `.c`/`.h`, e.g. a `version.inc`) — counting such
/// phantom endpoints would let them displace real files in the degree
/// ranking, render as nodes, and skew the truncation count.
fn cap_nodes(nodes: &[String], pairs: &[(String, String)]) -> (Vec<String>, usize) {
    if nodes.len() <= NODE_CAP {
        return (nodes.to_vec(), 0);
    }
    let node_set: std::collections::BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let mut degree: BTreeMap<&str, usize> = BTreeMap::new();
    for node in nodes {
        degree.entry(node.as_str()).or_insert(0);
    }
    for (from, to) in pairs {
        if !node_set.contains(from.as_str()) || !node_set.contains(to.as_str()) {
            continue; // phantom endpoint — not a chart node, no degree credit
        }
        *degree.entry(from.as_str()).or_insert(0) += 1;
        *degree.entry(to.as_str()).or_insert(0) += 1;
    }
    let mut ranked: Vec<(&str, usize)> = degree.iter().map(|(p, d)| (*p, *d)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    ranked.truncate(NODE_CAP);
    let kept: Vec<String> = ranked.into_iter().map(|(p, _)| p.to_string()).collect();
    // Re-sort ascending: downstream layout expects path order.
    let mut kept_sorted = kept;
    kept_sorted.sort();
    (kept_sorted, nodes.len() - NODE_CAP)
}

/// Within-layer barycenter ordering (ADR-0002 layout policy), EXACTLY 3
/// passes (pinned — the ADR states the pass count but not the sweep shape).
/// One pass = single downward sweep over layers `1..n`: each layer is
/// reordered by the mean position of its predecessors in the immediately
/// preceding layer; a node with NO predecessors in that layer falls back to
/// its current index (stability fallback, pinned). Ties break on path asc.
fn barycenter_order(layers: &mut [Vec<String>], dag: &[(String, String)]) {
    if layers.is_empty() {
        return;
    }
    for _pass in 0..3 {
        for l in 1..layers.len() {
            let prev_pos: BTreeMap<&str, usize> = layers[l - 1]
                .iter()
                .enumerate()
                .map(|(i, n)| (n.as_str(), i))
                .collect();
            let preds_of = |node: &str| -> Vec<usize> {
                dag.iter()
                    .filter(|(_, to)| to == node)
                    .filter_map(|(from, _)| prev_pos.get(from.as_str()).copied())
                    .collect()
            };
            let mut scored: Vec<(f64, &String)> = layers[l]
                .iter()
                .map(|node| {
                    let preds = preds_of(node);
                    let bary = if preds.is_empty() {
                        // Stability fallback: keep current slot value.
                        layers[l].iter().position(|n| n == node).unwrap_or(0) as f64
                    } else {
                        let sum: usize = preds.iter().sum();
                        sum as f64 / preds.len() as f64
                    };
                    (bary, node)
                })
                .collect();
            scored.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.1.cmp(b.1))
            });
            layers[l] = scored.into_iter().map(|(_, n)| n.clone()).collect();
        }
    }
}

/// Render include-graph.svg from the graphs section (caller guarantees ≥1
/// node). Pipeline: resolve filter → cap → dedupe+break cycles → topo
/// layers → 3-pass barycenter → integer grid positions.
#[must_use]
fn render_include_graph(graphs: &GraphsSection) -> Vec<u8> {
    let mut nodes: Vec<String> = graphs
        .include_graph
        .nodes
        .iter()
        .map(|n| n.path.clone())
        .collect();
    nodes.sort();
    nodes.dedup();

    let all_pairs = resolved_pairs(&graphs.include_graph.edges);
    let (nodes, dropped) = cap_nodes(&nodes, &all_pairs);
    let node_set: std::collections::BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let pairs: Vec<(String, String)> = all_pairs
        .into_iter()
        .filter(|(f, t)| node_set.contains(f.as_str()) && node_set.contains(t.as_str()))
        .collect();

    let cycle = break_cycles(&pairs);
    let broken_count = cycle.broken.len();
    let mut layers = topo_layers(&nodes, &cycle.kept);
    barycenter_order(&mut layers, &cycle.kept);

    // Positions: x = margin + layer*LAYER_W, y = margin + index*ROW_H.
    let mut pos: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for (l, members) in layers.iter().enumerate() {
        for (i, node) in members.iter().enumerate() {
            pos.insert(
                node.as_str(),
                (
                    IG_MARGIN + l as i64 * IG_LAYER_W,
                    IG_MARGIN + i as i64 * IG_ROW_H,
                ),
            );
        }
    }

    let rows = layers.iter().map(Vec::len).max().unwrap_or(1);
    let width = 2 * IG_MARGIN + (layers.len().saturating_sub(1)) as i64 * IG_LAYER_W + IG_NODE_W;
    let height = 2 * IG_MARGIN + rows as i64 * IG_ROW_H + IG_CAPTION_BAND;

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));

    // Edges first (under nodes): kept DAG edges only — broken cyclic edges
    // are NOT drawn (they live in ARCH-001 findings + caption).
    svg.push_str("<g stroke=\"#999999\" stroke-width=\"1\">\n");
    for (from, to) in &cycle.kept {
        let Some(&(fx, fy)) = pos.get(from.as_str()) else {
            continue;
        };
        let Some(&(tx, ty)) = pos.get(to.as_str()) else {
            continue;
        };
        let x1 = fx + IG_NODE_W;
        let y1 = fy + IG_NODE_H / 2;
        let y2 = ty + IG_NODE_H / 2;
        svg.push_str(&format!(
            "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{tx}\" y2=\"{y2}\"/>\n"
        ));
    }
    svg.push_str("</g>\n");

    // Nodes: rect + label, fixed attribute order.
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for node in &nodes {
        let Some(&(x, y)) = pos.get(node.as_str()) else {
            continue;
        };
        let label = escape_xml(&truncate_label(node));
        let text_x = x + CHAR_W as i64 / 2 + 2;
        let text_y = y + IG_NODE_H / 2 + 4;
        svg.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{IG_NODE_W}\" height=\"{IG_NODE_H}\" fill=\"#dbe9ff\" stroke=\"#4a6fa5\"/>\n"
        ));
        svg.push_str(&format!(
            "<text x=\"{text_x}\" y=\"{text_y}\">{label}</text>\n"
        ));
    }
    svg.push_str("</g>\n");

    // Captions: truncation marker first (never silent), then cycle note.
    let caption_y = height - 12;
    if dropped > 0 {
        svg.push_str(&format!(
            "<text x=\"{IG_MARGIN}\" y=\"{caption_y}\" font-family=\"monospace\" font-size=\"11\">&#8230; {dropped} more files omitted</text>\n"
        ));
    }
    if broken_count > 0 {
        svg.push_str(&format!(
            "<text x=\"{IG_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">{broken_count} cyclic edges broken (see ARCH-001 findings)</text>\n",
            caption_y - 16
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

// ---------------------------------------------------------------------------
// component-coupling.svg
// ---------------------------------------------------------------------------

/// Coupling weights between axis components. Include contribution counts
/// resolved internal include edges between the two parent directories;
/// call contribution counts cross-file call edges likewise (LOW-confidence,
/// styled distinctly). Pinned symmetry: an off-diagonal edge increments BOTH
/// (A,B) and (B,A); a same-component edge increments the diagonal once.
struct Coupling {
    axis: Vec<String>,
    include: Vec<Vec<u64>>,
    call: Vec<Vec<u64>>,
}

impl Coupling {
    fn build(components: &[ComponentEntry], graphs: Option<&GraphsSection>) -> Self {
        let axis: Vec<String> = components.iter().map(|c| c.path_prefix.clone()).collect();
        let index: BTreeMap<String, usize> = axis
            .iter()
            .enumerate()
            .map(|(i, p)| (p.clone(), i))
            .collect();
        let n = axis.len();
        let mut cell = Coupling {
            axis,
            include: vec![vec![0; n]; n],
            call: vec![vec![0; n]; n],
        };

        let bump = |matrix: &mut Vec<Vec<u64>>, from: &str, to: &str| {
            if let (Some(&i), Some(&j)) = (index.get(parent_dir(from)), index.get(parent_dir(to))) {
                matrix[i][j] += 1;
                if i != j {
                    matrix[j][i] += 1;
                }
            }
        };

        if let Some(graphs) = graphs {
            for (from, to) in resolved_pairs(&graphs.include_graph.edges) {
                bump(&mut cell.include, &from, &to);
            }
            if let Some(call_graph) = &graphs.call_graph {
                for edge in &call_graph.edges {
                    if let Some(to) = &edge.to_path {
                        bump(&mut cell.call, &edge.from_path, to);
                    }
                }
            }
        }
        cell
    }

    fn max_cell(&self) -> u64 {
        self.include
            .iter()
            .zip(&self.call)
            .flat_map(|(inc, call)| inc.iter().zip(call).map(|(a, b)| a + b))
            .max()
            .unwrap_or(0)
    }
}

/// Render component-coupling.svg (caller guarantees non-empty components).
#[must_use]
fn render_component_coupling(
    components: &[ComponentEntry],
    graphs: Option<&GraphsSection>,
) -> Vec<u8> {
    // Axis order (rank, path_prefix); rank ≤ COMPONENT_CAP, then truncate.
    let mut sorted: Vec<&ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    sorted.retain(|c| c.rank <= u32::try_from(COMPONENT_CAP).unwrap_or(u32::MAX));
    let overflow = sorted.len() > COMPONENT_CAP;
    sorted.truncate(COMPONENT_CAP);
    let owned: Vec<ComponentEntry> = sorted.into_iter().cloned().collect();

    let coupling = Coupling::build(&owned, graphs);
    let max = coupling.max_cell();
    let n = coupling.axis.len();

    let width = CC_MARGIN + CC_ROW_LABEL_W + n as i64 * CC_CELL + CC_MARGIN;
    let height = CC_MARGIN + CC_COL_LABEL_H + n as i64 * CC_CELL + CC_LEGEND_BAND;
    let grid_x = CC_MARGIN + CC_ROW_LABEL_W;
    let grid_y = CC_MARGIN + CC_COL_LABEL_H;

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");

    for (i, prefix) in coupling.axis.iter().enumerate() {
        let label = escape_xml(&truncate_label(prefix));
        // Row label (plain, right-aligned into the gutter).
        let ry = grid_y + i as i64 * CC_CELL + CC_CELL / 2 + 4;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{ry}\" text-anchor=\"end\">{label}</text>\n",
            grid_x - 6
        ));
        // Column label (rotated -90° above the grid).
        let cx = grid_x + i as i64 * CC_CELL + CC_CELL / 2 + 4;
        svg.push_str(&format!(
            "<text transform=\"translate({cx},{}) rotate(-90)\">{label}</text>\n",
            grid_y - 6
        ));
    }
    svg.push_str("</g>\n");

    // Cells: include heat rect, then distinct call overlay when present.
    svg.push_str("<g stroke=\"#dddddd\" stroke-width=\"1\">\n");
    for (i, row_inc) in coupling.include.iter().enumerate() {
        for (j, inc) in row_inc.iter().enumerate() {
            let total = inc + coupling.call[i][j];
            let x = grid_x + j as i64 * CC_CELL;
            let y = grid_y + i as i64 * CC_CELL;
            let fill = if max == 0 {
                heat_color(0) // all-zero ⇒ uniform minimal fill
            } else {
                let level = round_half_up(total as f64 / max as f64 * 10.0, 0);
                heat_color(level.clamp(0, 10) as u32)
            };
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{CC_CELL}\" height=\"{CC_CELL}\" fill=\"{fill}\"/>\n"
            ));
            if coupling.call[i][j] > 0 {
                svg.push_str(&format!(
                    "<rect x=\"{x}\" y=\"{y}\" width=\"{CC_CELL}\" height=\"{CC_CELL}\" fill=\"{CC_CALL_FILL}\" fill-opacity=\"{CC_CALL_OPACITY}\"/>\n"
                ));
            }
        }
    }
    svg.push_str("</g>\n");

    // Legend/caption band: standing ADR-0001 caveat + truncation marker.
    let legend_y = height - 12;
    svg.push_str(&format!(
        "<text x=\"{CC_MARGIN}\" y=\"{legend_y}\" font-family=\"monospace\" font-size=\"11\">call edges are LOW-confidence hypotheses (ADR-0001)</text>\n"
    ));
    if overflow {
        svg.push_str(&format!(
            "<text x=\"{CC_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">&#8230; more components omitted</text>\n",
            legend_y - 16
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

// ---------------------------------------------------------------------------
// hot-components.svg
// ---------------------------------------------------------------------------

/// Render hot-components.svg (caller guarantees non-empty components):
/// horizontal bars in `(rank, path_prefix)` order, length ∝ finding_density
/// (max density = full plot width; all-zero ⇒ minimal stub bars + caption).
#[must_use]
fn render_hot_components(components: &[ComponentEntry]) -> Vec<u8> {
    let mut sorted: Vec<&ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    sorted.retain(|c| c.rank <= u32::try_from(COMPONENT_CAP).unwrap_or(u32::MAX));
    let overflow = sorted.len() > COMPONENT_CAP;
    sorted.truncate(COMPONENT_CAP);

    let max_density = sorted
        .iter()
        .map(|c| c.metrics.finding_density)
        .fold(0.0_f64, f64::max);
    let all_zero = max_density <= 0.0;

    let width = 2 * HC_MARGIN + HC_LABEL_W + HC_PLOT_W;
    let height = 2 * HC_MARGIN + sorted.len() as i64 * HC_ROW_PITCH + HC_CAPTION_BAND;
    let plot_x = HC_MARGIN + HC_LABEL_W;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str("<g font-family=\"monospace\" font-size=\"11\">\n");
    for (i, comp) in sorted.iter().enumerate() {
        let bar_len = if all_zero {
            HC_STUB_W
        } else {
            round_half_up(
                comp.metrics.finding_density / max_density * HC_PLOT_W as f64,
                0,
            )
        };
        let y = HC_MARGIN + i as i64 * HC_ROW_PITCH;
        let label = escape_xml(&format!(
            "{} {} {}",
            comp.rank,
            truncate_label(&comp.path_prefix),
            format_fixed_point(comp.metrics.finding_density, 2)
        ));
        svg.push_str(&format!(
            "<text x=\"{HC_MARGIN}\" y=\"{}\">{label}</text>\n",
            y + HC_BAR_H
        ));
        svg.push_str(&format!(
            "<rect x=\"{plot_x}\" y=\"{y}\" width=\"{bar_len}\" height=\"{HC_BAR_H}\" fill=\"{HC_BAR_FILL}\"/>\n"
        ));
    }
    svg.push_str("</g>\n");

    let caption_y = height - 12;
    if all_zero {
        svg.push_str(&format!(
            "<text x=\"{HC_MARGIN}\" y=\"{caption_y}\" font-family=\"monospace\" font-size=\"11\">no findings recorded</text>\n"
        ));
    }
    if overflow {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">&#8230; more components omitted</text>\n",
            HC_MARGIN, caption_y - 16
        ));
    }
    svg.push_str("</svg>\n");
    svg.into_bytes()
}

// ---------------------------------------------------------------------------
// In-module unit locks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    //! Contract locks: HALF-UP rounding (incl. negatives away-from-zero),
    //! fixed-point formatting, XML escaping of hostile paths, tail label
    //! truncation, and palette purity.

    use super::*;

    #[test]
    fn round_half_up_positive_halves_go_up() {
        assert_eq!(round_half_up(0.125, 2), 13, "0.125 @2 → 0.13");
        assert_eq!(round_half_up(2.5, 0), 3, "2.5 @0 → 3");
        assert_eq!(round_half_up(0.5, 0), 1);
        assert_eq!(round_half_up(1.25, 2), 125, "exact values unchanged");
    }

    #[test]
    fn round_half_up_negative_halves_go_away_from_zero() {
        // Pinned choice: HALF-UP means AWAY FROM ZERO for negatives too.
        assert_eq!(round_half_up(-2.5, 0), -3);
        assert_eq!(round_half_up(-0.125, 2), -13, "-0.125 @2 → -0.13");
        assert_eq!(round_half_up(-0.5, 0), -1);
        assert_eq!(round_half_up(-1.24, 2), -124);
    }

    #[test]
    fn format_fixed_emits_fixed_point_text() {
        assert_eq!(format_fixed_point(0.125, 2), "0.13");
        assert_eq!(format_fixed_point(2.5, 0), "3");
        assert_eq!(format_fixed_point(-2.5, 0), "-3");
        assert_eq!(format_fixed_point(8.25, 2), "8.25");
        assert_eq!(format_fixed_point(0.0, 2), "0.00");
        assert_eq!(
            format_fixed_point(-0.004, 2),
            "0.00",
            "scaled value is an i64 — no negative zero can leak into output"
        );
    }

    #[test]
    fn escape_xml_neutralizes_all_five_hostile_chars() {
        assert_eq!(escape_xml(r#"<a>&"'b"#), "&lt;a&gt;&amp;&quot;&#39;b");
        assert_eq!(escape_xml("plain/path.c"), "plain/path.c");
    }

    #[test]
    fn truncate_label_keeps_tail_past_32_chars() {
        let short = "src/net/socket.c";
        assert_eq!(truncate_label(short), short);
        let long = "very/deep/nested/directory/tree/that/exceeds/limit/module.c";
        let truncated = truncate_label(long);
        assert_eq!(truncated.chars().count(), LABEL_MAX_CHARS);
        assert!(truncated.starts_with('…'));
        assert!(long.ends_with(truncated.trim_start_matches('…')));
    }

    #[test]
    fn heat_color_is_a_pure_ramp() {
        assert_eq!(heat_color(0), "#ffffff");
        assert_eq!(heat_color(10), "#b30000");
        assert_eq!(heat_color(11), heat_color(10), "clamped");
        assert_eq!(heat_color(5), heat_color(5), "pure function");
    }

    #[test]
    fn parent_dir_splits_on_last_slash() {
        assert_eq!(parent_dir("src/net/a.c"), "src/net");
        // Root-level files map to "." — MUST mirror the aggregator's
        // parent_dir exactly, or root-level coupling edges are dropped.
        assert_eq!(parent_dir("a.c"), ".");
    }

    #[test]
    fn round_half_up_matches_digit_method_on_binary_borderline() {
        // The former charts-local binary-scaling impl floored this to 267;
        // the shared digit-based method (pinned) gives 268 — same text the
        // reporter has always produced for the same density.
        assert_eq!(round_half_up(2.675, 2), 268);
        assert_eq!(format_fixed_point(2.675, 2), "2.68");
    }
}
