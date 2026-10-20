//! `charts` — deterministic SVG emitter (modules.md §3.7; ADR-0002).
//!
//! Consumes: the final document — graphs.* and components[] ONLY (ADR-0002
//! guardrail 1), plus `files[].path` (component membership) and
//! `stats.functions[]` (cost heat — the same extension the coupling overlay
//! consumes). Produces: SVG bytes for chart-set v1 ([`render`]:
//! include-graph.svg, layer-file.svg, component-coupling.svg,
//! hot-components.svg, layer-workspace.svg, plus one
//! `component-<rank>-<slug>.svg` detail page per capped component).
//!
//! Responsibilities: byte-deterministic rendering by construction (ADR-0002
//! guardrail 2); XML-escape all label/path text; deterministic node caps with
//! in-chart truncation markers; presence rule — a ranked overview chart is
//! emitted iff its source section is present and non-empty, while the two
//! layered overviews emit a deterministic stub whenever their source section
//! is present even if empty (symmetric-stub pin); every capped component
//! yields a detail page (a file-less component yields a deterministic stub,
//! never an omission); otherwise omitted (charts/ may be empty).
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
//! Presence rule — symmetric stubs (B5 pin): a section that is PRESENT but
//! empty still emits its layer stub (`layer-file.svg` whenever `graphs` is
//! present, `layer-workspace.svg` whenever `components` is present — the
//! renderers own the empty → stub mapping, never an omission); only a
//! wholly ABSENT section omits its charts. Ranked charts
//! (include-graph, component-coupling, hot-components) and the
//! per-component pages still require non-empty sources.
//!
//! Non-goals: layout quality beyond "legible" (MVP bar); external
//! charting/layout libraries are banned by ADR-0002.

pub mod component;
pub mod dot;
pub mod layers;

use std::collections::{BTreeMap, BTreeSet};

use crate::graph::{break_cycles, topo_layers};
use crate::schema::{
    ComponentEntry, FunctionStats, GraphsSection, IncludeEdgeDto, MetricsDocument,
    format_fixed_point, round_half_up,
};

// ---------------------------------------------------------------------------
// Tunables (fixed constants — part of the byte-determinism surface)
// ---------------------------------------------------------------------------

/// Node cap for include-graph.svg (founder-approved): applied PRE-layout,
/// top-N by degree desc, tie path asc.
pub(crate) const NODE_CAP: usize = 50;
/// I2 reachability-aware cap overflow (plan `map-build`): after the base
/// top-[`NODE_CAP`] election, entry-reachable nodes that would be cut get
/// reserved into at most this many EXTRA slots. The reserve never displaces
/// a base node; the caption reports the actually-shown count (`top S of M`).
pub(crate) const REACHABLE_OVERFLOW: usize = 10;
/// Component cap for both component charts AND per-component detail pages:
/// entries with rank > 50 dropped, then list truncated to this many after
/// `(rank, path_prefix)` sort.
pub(crate) const COMPONENT_CAP: usize = 50;
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

// component-coupling.svg geometry
const CC_MARGIN: i64 = 20;
const CC_ROW_LABEL_W: i64 = 190;
const CC_COL_LABEL_H: i64 = 130;
const CC_CELL: i64 = 24;
/// Caption rows stack 16px apart with a 12px bottom pad (see [`caption_band`]).
const CC_CAPTION_GAP: i64 = 16;
const CC_CAPTION_PAD: i64 = 12;

// hot-components.svg geometry
const HC_MARGIN: i64 = 20;
const HC_HEADER_BAND: i64 = 64;
const HC_LABEL_W: i64 = 300;
const HC_PLOT_W: i64 = 420;
const HC_ROW_PITCH: i64 = 26;
const HC_BAR_H: i64 = 16;
/// Bar fill: hottest palette color (pure constant, not value-dependent).
const HC_BAR_FILL: &str = "#b30000";
/// Stub bar length when every density is zero ("minimal stub bars").
const HC_STUB_W: i64 = 2;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Render the chart set v1 for `doc`.
/// Returns ordered `(filename, svg bytes)` pairs. Emission rule (ADR-0002
/// guardrail 5 + symmetric-stub pin): a ranked overview chart appears IFF
/// its source section is present AND non-empty; an empty document yields an
/// empty list. The layered overviews (Option A slice 2) emit STUBS on empty
/// sources — `layer-file.svg` whenever `graphs` is present (even with zero
/// nodes), `layer-workspace.svg` whenever `components` is present (even when
/// empty) — so both stubs are reachable via `render`, symmetrically. Per-component
/// detail pages follow the same gate (components present AND non-empty) —
/// one page per capped component in `(rank, path_prefix)` order, appended
/// AFTER the overview charts (including the layered overviews); a file-less
/// component yields the
/// deterministic stub page, never an omission.
#[must_use]
pub fn render(doc: &MetricsDocument) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    if let Some(graphs) = &doc.graphs {
        if !graphs.include_graph.nodes.is_empty() {
            out.push((
                "include-graph.svg".to_string(),
                render_include_graph(graphs, &doc.stats.functions),
            ));
        }
        // Stub on empty nodes (symmetric with layer-workspace below).
        out.push((
            "layer-file.svg".to_string(),
            layers::render_layer_file(graphs, &doc.stats.functions),
        ));
    }
    if let Some(components) = &doc.components {
        if !components.is_empty() {
            let graphs = doc.graphs.as_ref();
            out.push((
                "component-coupling.svg".to_string(),
                render_component_coupling(components, graphs, &doc.stats.functions),
            ));
            out.push((
                "hot-components.svg".to_string(),
                render_hot_components(components),
            ));
        }
        // Stub on empty components (symmetric with layer-file above).
        out.push((
            "layer-workspace.svg".to_string(),
            layers::render_layer_workspace(components, doc.graphs.as_ref()),
        ));
        if !components.is_empty() {
            let graphs = doc.graphs.as_ref();
            out.extend(component::render_component_pages(
                components,
                graphs,
                &doc.files,
                &doc.stats.functions,
            ));
        }
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

/// Heat color for coupling/bars/component pages: pure function mapping level
/// `0..=10` onto a white→dark-red ramp (integer arithmetic only — no floats
/// in color space).
#[must_use]
pub(crate) fn heat_color(level: u32) -> String {
    let level = level.min(10);
    let r = 255 - (76 * level + 5) / 10;
    let g = (255 * (10 - level) + 5) / 10;
    format!("#{r:02x}{g:02x}{g:02x}")
}

/// Label ink for text drawn directly ON a heat fill: dark fills (level ≥ 7)
/// take white (`#ffffff`, ≈7.2:1 on `#b30000`); lighter fills keep the
/// default black ink (the caller omits the `fill` attribute then).
#[must_use]
pub(crate) fn heat_ink(level: u32) -> Option<&'static str> {
    if level >= 7 { Some("#ffffff") } else { None }
}

/// Estimated monospace text width in px at `font_size` — linear scale of the
/// fixed [`CHAR_W`] metric (pinned for font-size 11), ceiled. `text` must be
/// the RENDERED form (entities resolved: `…`, not `&#8230;`).
#[must_use]
pub(crate) fn est_text_w(text: &str, font_size: f64) -> i64 {
    (text.chars().count() as f64 * CHAR_W * font_size / 11.0).ceil() as i64
}

/// Bottom caption band fitting exactly `lines` caption rows (16px rows + 12px
/// bottom pad); zero when no caption is emitted, so canvases fit content.
#[must_use]
pub(crate) fn caption_band(lines: usize) -> i64 {
    if lines == 0 {
        0
    } else {
        CC_CAPTION_PAD + lines as i64 * CC_CAPTION_GAP
    }
}

/// Bottom band for charts carrying a heat-scale legend above `caps` caption
/// rows: 12px bottom pad + caption rows + 8px gap + 12px swatches.
#[must_use]
pub(crate) fn legend_band(caps: usize) -> i64 {
    CC_CAPTION_PAD + caps as i64 * CC_CAPTION_GAP + 8 + 12
}

/// Top y of the legend swatch row given canvas `height` and `caps` caption
/// rows below it.
#[must_use]
pub(crate) fn legend_top(height: i64, caps: usize) -> i64 {
    height - CC_CAPTION_PAD - caps as i64 * CC_CAPTION_GAP - 8 - 12
}

/// Heat-scale legend row (`cool` + ramp swatches `0..=10` + `hot`) with
/// swatch tops at `y` (12px swatches, 10px text baseline offset). Fixed
/// ≈191px wide at font-size 11 — callers widen narrow canvases via
/// [`HEAT_LEGEND_W`].
pub(crate) const HEAT_LEGEND_W: i64 = 191;

pub(crate) fn push_heat_legend(out: &mut String, x: i64, y: i64) {
    const SW: i64 = 12;
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">cool</text>\n",
        y + 10
    ));
    let mut sx = x + (4.0 * CHAR_W).ceil() as i64 + 6;
    for level in 0..=10 {
        let fill = heat_color(level);
        out.push_str(&format!(
            "<rect x=\"{sx}\" y=\"{y}\" width=\"{SW}\" height=\"12\" fill=\"{fill}\"/>\n"
        ));
        sx += SW;
    }
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">hot</text>\n",
        sx + 6,
        y + 10
    ));
}

/// Repo-relative parent directory of a POSIX path; root-level files map to
/// `"."` — EXACTLY mirroring the aggregator's `parent_dir`
/// (src/aggregator/mod.rs), whose `"."` prefixes are the coupling-matrix
/// axis labels. A divergent root mapping here would make every
/// root-level endpoint miss the bump-index lookup and silently drop its
/// edges from component-coupling.svg. Shared with the per-component pages
/// for file↔component membership (same mapping, same guarantee).
pub(crate) fn parent_dir(path: &str) -> &str {
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
/// excluded here — they surface as ARCH-001 findings instead. Shared with
/// the per-component pages (intra-component subgraph filter).
pub(crate) fn resolved_pairs(edges: &[IncludeEdgeDto]) -> Vec<(String, String)> {
    edges
        .iter()
        .filter(|e| e.resolved)
        .filter_map(|e| e.to.clone().map(|to| (e.from.clone(), to)))
        .collect()
}

/// Apply the node cap PRE-layout (I2 reachability-aware election).
///
/// Base ranking is unchanged — top-[`NODE_CAP`] nodes by degree desc (degree
/// = incident resolved edges), tie path asc. After that base cut, nodes that
/// are (a) entry-reachable and (b) would have been dropped are RESERVED into
/// up to [`REACHABLE_OVERFLOW`] extra slots. Candidate election among the
/// reachable tail uses cost desc then path asc (so the extra slots go to the
/// costliest reachable tail); the final kept set is re-sorted path-ascending
/// for deterministic layout. Base nodes are never displaced.
///
/// Returns `(kept paths sorted ascending, TOTAL input node count)` so callers
/// can render the honest `showing top S of M` caption with `S = kept.len()`
/// (the reserve makes `S` exceed [`NODE_CAP`] by design) and `M = total`.
///
/// Edge pairs are FIRST filtered to endpoints present in the node set: an
/// include can resolve to a real file that is not a scanned chart node
/// (discovery only picks `.c`/`.h`/`.rs`, e.g. a `version.inc`) — counting such
/// phantom endpoints would let them displace real files in the degree
/// ranking, render as nodes, and skew the truncation count. Shared with the
/// per-component pages (same top-50-pre-layout cap at component scope).
pub(crate) fn cap_nodes(
    nodes: &[String],
    pairs: &[(String, String)],
    costs: &BTreeMap<String, f64>,
    reachable: &BTreeSet<String>,
) -> (Vec<String>, usize) {
    let total = nodes.len();
    if total <= NODE_CAP {
        return (nodes.to_vec(), total);
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
    let mut kept: Vec<&str> = ranked.iter().take(NODE_CAP).map(|(p, _)| *p).collect();

    // Reachability reserve: only nodes the BASE ranking cut, cost desc then
    // path asc for a principled, input-order-independent election.
    let mut reserve: Vec<&str> = ranked
        .iter()
        .skip(NODE_CAP)
        .map(|(p, _)| *p)
        .filter(|path| reachable.contains(*path))
        .collect();
    reserve.sort_by(|a, b| {
        costs
            .get(*b)
            .copied()
            .unwrap_or(0.0)
            .partial_cmp(&costs.get(*a).copied().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(b))
    });
    kept.extend(reserve.into_iter().take(REACHABLE_OVERFLOW));

    let mut kept_sorted: Vec<String> = kept.into_iter().map(str::to_string).collect();
    // Re-sort ascending: downstream layout expects path order and the
    // overflow slots must not perturb byte-determinism.
    kept_sorted.sort();
    (kept_sorted, total)
}

/// Within-layer barycenter ordering (ADR-0002 layout policy), EXACTLY 3
/// passes (pinned — the ADR states the pass count but not the sweep shape).
/// One pass = single downward sweep over layers `1..n`: each layer is
/// reordered by the mean position of its predecessors in the immediately
/// preceding layer; a node with NO predecessors in that layer falls back to
/// its current index (stability fallback, pinned). Ties break on path asc.
/// Shared with the per-component pages (same 3-pass layout at scope).
pub(crate) fn barycenter_order(layers: &mut [Vec<String>], dag: &[(String, String)]) {
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
/// node). Pipeline: resolve filter → reachability-aware cap → dedupe+break
/// cycles → topo layers → 3-pass barycenter → integer grid positions.
///
/// I2: the `functions` handle supplies per-file heat costs + entry
/// reachability to the cap election (the include graph has no function data
/// of its own); rendering itself is unchanged.
#[must_use]
fn render_include_graph(graphs: &GraphsSection, functions: &[FunctionStats]) -> Vec<u8> {
    let mut nodes: Vec<String> = graphs
        .include_graph
        .nodes
        .iter()
        .map(|n| n.path.clone())
        .collect();
    nodes.sort();
    nodes.dedup();

    let all_pairs = resolved_pairs(&graphs.include_graph.edges);
    let costs = file_heat_costs(functions);
    let reachable = reachable_files(functions);
    let (nodes, total) = cap_nodes(&nodes, &all_pairs, &costs, &reachable);
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
    // Caption rows bottom-first: truncation marker, then the cycle note.
    let mut captions: Vec<String> = Vec::new();
    if total > nodes.len() {
        captions.push(format!("showing top {} of {} files", nodes.len(), total));
    }
    if broken_count > 0 {
        captions.push(format!(
            "{broken_count} cyclic edges broken (see ARCH-001 findings)"
        ));
    }
    let height = 2 * IG_MARGIN + rows as i64 * IG_ROW_H + caption_band(captions.len());

    let mut svg = String::with_capacity(4096);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));

    // Edges first (under nodes): kept DAG edges only — broken cyclic edges
    // are NOT drawn (they live in ARCH-001 findings + caption). The group is
    // omitted outright when empty (no empty `<g stroke>` in output).
    if !cycle.kept.is_empty() {
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
    }

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

    // Captions: truncation marker first (never silent), then cycle note —
    // bottom row first so the band fits content exactly.
    for (k, caption) in captions.iter().enumerate() {
        let y = height - CC_CAPTION_PAD - k as i64 * CC_CAPTION_GAP;
        svg.push_str(&format!(
            "<text x=\"{IG_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{caption}</text>\n"
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
/// call contribution counts cross-file call edges likewise (LOW-confidence —
/// folded into the SAME cell heat as the include weight, flagged by the
/// ADR-0001 caveat caption; there is exactly one fill rect per cell, never a
/// full-cell overlay that would bury the heat color). Pinned symmetry: an off-diagonal edge increments BOTH
/// (A,B) and (B,A); a same-component edge increments the diagonal once.
///
/// v1.4 STEP 2 cost overlay (plan `divergence-revamp`): cells accumulate
/// EDGE WEIGHTS, not bare counts — each edge weighs `1 + normalized endpoint
/// cost`, so hot, deep, alloc-heavy, or highly-coupled functions pull their
/// component's coupling cells darker. Formula (documented here AND in
/// report.md §5 — charts stay pure-fn-of-JSON, consuming
/// `stats.functions[]` + `components[]` only):
///
/// ```text
/// cost(f) = cyclomatic + cognitive + 2*max_loop_depth + 3*alloc_site_count
///         + call_fanout + call_fanin + call_depth
///         + 2*[is_recursive] + [error_path] + [alloc_family present]
/// w_include(F→T) = 1 + (avg_cost(F) + avg_cost(T)) / (2 * max_cost)
/// w_call(caller→to) = 1 + (cost(caller) + avg_cost(to_file)) / (2 * max_cost)
/// ```
///
/// `avg_cost(file)` is the mean over that file's functions (0 when the file
/// has none). Unresolved call edges (`to_path` None) never enter the matrix
/// (legacy admission — a hypothesis without a target file has no cell).
/// `max_cost` is the scan maximum; when it is 0 (no functions, or all-zero
/// metrics) every weight is exactly 1.0 and the overlay is byte-identical
/// to the legacy count matrix (pinned by the no-functions chart tests).
///
/// Determinism: averages come from `BTreeMap`s, edge iteration follows
/// document order (the aggregator's fixed sort), and per-cell float sums
/// accumulate in that same fixed order — identical documents ⇒ identical
/// bytes.
struct Coupling {
    axis: Vec<String>,
    include: Vec<Vec<f64>>,
    call: Vec<Vec<f64>>,
}

/// Per-function cost feeding the coupling overlay (see [`Coupling`]) and the
/// per-component page heat. Missing (`None`) v1.4 fields contribute 0 —
/// parser-free or pre-step-2 documents simply overlay less.
pub(crate) fn function_cost(f: &FunctionStats) -> f64 {
    f64::from(f.cyclomatic_complexity)
        + f64::from(f.cognitive_complexity)
        + 2.0 * f64::from(f.max_loop_depth)
        + 3.0 * f64::from(f.alloc_site_count)
        + f64::from(f.call_fanout.unwrap_or(0))
        + f64::from(f.call_fanin.unwrap_or(0))
        + f64::from(f.call_depth.unwrap_or(0))
        + if f.is_recursive.unwrap_or(false) {
            2.0
        } else {
            0.0
        }
        + if f.error_path.unwrap_or(false) {
            1.0
        } else {
            0.0
        }
        + if f.alloc_family.is_some() { 1.0 } else { 0.0 }
}

/// THE heat-cost selector (I2 plan `map-build`, documented once here and
/// reused by every heat consumer): prefer the aggregator's
/// [`FunctionStats::function_cost_v2`] when present (per-iteration weighted),
/// else fall back to the retained v1 [`function_cost`] (pre-I2/parser-free
/// documents). Coupling overlays deliberately stay on v1 until cutover.
pub(crate) fn function_heat_cost(function: &FunctionStats) -> f64 {
    function
        .function_cost_v2
        .map_or_else(|| function_cost(function), |cost| cost as f64)
}

/// Per-file heat = Σ [`function_heat_cost`] over that file's functions
/// (`BTreeMap` ⇒ deterministic). Shared by the file heat map, component
/// pages, and the reachability-aware cap election.
pub(crate) fn file_heat_costs(functions: &[FunctionStats]) -> BTreeMap<String, f64> {
    let mut costs: BTreeMap<String, f64> = BTreeMap::new();
    for function in functions {
        *costs.entry(function.path.clone()).or_insert(0.0) += function_heat_cost(function);
    }
    costs
}

/// Files containing at least one entry-reachable function (I2 cap election
/// input; `BTreeSet` ⇒ deterministic). Unknown reachability never reserves.
pub(crate) fn reachable_files(functions: &[FunctionStats]) -> BTreeSet<String> {
    functions
        .iter()
        .filter(|function| function.reachable_from_entry == Some(true))
        .map(|function| function.path.clone())
        .collect()
}

impl Coupling {
    fn build(
        components: &[ComponentEntry],
        graphs: Option<&GraphsSection>,
        functions: &[FunctionStats],
    ) -> Self {
        let axis: Vec<String> = components.iter().map(|c| c.path_prefix.clone()).collect();
        let index: BTreeMap<String, usize> = axis
            .iter()
            .enumerate()
            .map(|(i, p)| (p.clone(), i))
            .collect();
        let n = axis.len();
        let mut cell = Coupling {
            axis,
            include: vec![vec![0.0; n]; n],
            call: vec![vec![0.0; n]; n],
        };

        // Cost lookup tables (BTreeMaps ⇒ deterministic).
        let max_cost = functions.iter().map(function_cost).fold(0.0_f64, f64::max);
        let mut file_sum: BTreeMap<&str, (f64, u32)> = BTreeMap::new();
        let mut id_cost: BTreeMap<&str, f64> = BTreeMap::new();
        for f in functions {
            let cost = function_cost(f);
            id_cost.insert(f.id.as_str(), cost);
            let slot = file_sum.entry(f.path.as_str()).or_insert((0.0, 0));
            slot.0 += cost;
            slot.1 += 1;
        }
        let file_avg = |path: &str| -> f64 {
            file_sum
                .get(path)
                .map_or(0.0, |(sum, count)| sum / f64::from(*count))
        };
        // Normalize one endpoint-cost pair into the [0, 1] weight bonus.
        // Zero max ⇒ legacy weight 1.0 exactly (no overlay).
        let bonus = |a: f64, b: f64| -> f64 {
            if max_cost <= 0.0 {
                0.0
            } else {
                (a + b) / (2.0 * max_cost)
            }
        };

        let bump = |matrix: &mut Vec<Vec<f64>>, from: &str, to: &str, weight: f64| {
            if let (Some(&i), Some(&j)) = (index.get(parent_dir(from)), index.get(parent_dir(to))) {
                matrix[i][j] += weight;
                if i != j {
                    matrix[j][i] += weight;
                }
            }
        };

        if let Some(graphs) = graphs {
            for (from, to) in resolved_pairs(&graphs.include_graph.edges) {
                let weight = 1.0 + bonus(file_avg(&from), file_avg(&to));
                bump(&mut cell.include, &from, &to, weight);
            }
            if let Some(call_graph) = &graphs.call_graph {
                for edge in &call_graph.edges {
                    // Legacy admission: resolved edges only — unresolved
                    // (to_path None) LOW-hypotheses without a target file
                    // never enter the matrix (keeps the no-functions overlay
                    // byte-identical to the legacy count matrix).
                    if let Some(to) = &edge.to_path {
                        let caller_cost = id_cost
                            .get(edge.from_function.as_str())
                            .copied()
                            .unwrap_or(0.0);
                        let weight = 1.0 + bonus(caller_cost, file_avg(to));
                        bump(&mut cell.call, &edge.from_path, to, weight);
                    }
                }
            }
        }
        cell
    }

    fn max_cell(&self) -> f64 {
        self.include
            .iter()
            .zip(&self.call)
            .flat_map(|(inc, call)| inc.iter().zip(call).map(|(a, b)| a + b))
            .fold(0.0_f64, f64::max)
    }
}

/// Render component-coupling.svg (caller guarantees non-empty components).
#[must_use]
fn render_component_coupling(
    components: &[ComponentEntry],
    graphs: Option<&GraphsSection>,
    functions: &[FunctionStats],
) -> Vec<u8> {
    // Axis order (rank, path_prefix); rank ≤ COMPONENT_CAP, then truncate.
    let mut sorted: Vec<&ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    sorted.retain(|c| c.rank <= u32::try_from(COMPONENT_CAP).unwrap_or(u32::MAX));
    let overflow = sorted.len() > COMPONENT_CAP;
    sorted.truncate(COMPONENT_CAP);
    let owned: Vec<ComponentEntry> = sorted.into_iter().cloned().collect();

    let coupling = Coupling::build(&owned, graphs, functions);
    let max = coupling.max_cell();
    let n = coupling.axis.len();

    let grid_w = CC_MARGIN + CC_ROW_LABEL_W + n as i64 * CC_CELL + CC_MARGIN;
    // Legend/caption band: the standing ADR-0001 caveat always, plus the
    // truncation marker when capped — bottom row first.
    let legend = "call edges are LOW-confidence hypotheses (ADR-0001)";
    let mut captions: Vec<String> = vec![legend.to_string()];
    if overflow {
        captions.push("… more components omitted".to_string());
    }
    // The caveat caption is wider than narrow grids: widen the canvas so the
    // longest caption keeps ≥10px from the right edge (never clipped).
    let mut width = grid_w;
    for caption in &captions {
        width = width.max(CC_MARGIN + est_text_w(caption, 11.0) + 10);
    }
    let height = CC_MARGIN + CC_COL_LABEL_H + n as i64 * CC_CELL + caption_band(captions.len());
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

    // Cells: ONE heat rect per cell (include + call weights folded into the
    // same heat — a full-cell call overlay would bury the heat color).
    svg.push_str("<g stroke=\"#dddddd\" stroke-width=\"1\">\n");
    for (i, row_inc) in coupling.include.iter().enumerate() {
        for (j, inc) in row_inc.iter().enumerate() {
            let total = inc + coupling.call[i][j];
            let x = grid_x + j as i64 * CC_CELL;
            let y = grid_y + i as i64 * CC_CELL;
            let fill = if max <= 0.0 {
                heat_color(0) // all-zero ⇒ uniform minimal fill
            } else {
                let level = round_half_up(total / max * 10.0, 0);
                heat_color(level.clamp(0, 10) as u32)
            };
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"{CC_CELL}\" height=\"{CC_CELL}\" fill=\"{fill}\"/>\n"
            ));
        }
    }
    svg.push_str("</g>\n");

    // Legend/caption band: standing ADR-0001 caveat + truncation marker,
    // bottom row first so the band fits content exactly. Captions are stored
    // rendered (`…` = 1 glyph for width); the emitter spells it as the
    // `&#8230;` entity like every other chart.
    for (k, caption) in captions.iter().enumerate() {
        let y = height - CC_CAPTION_PAD - k as i64 * CC_CAPTION_GAP;
        let out = caption.replace('…', "&#8230;");
        svg.push_str(&format!(
            "<text x=\"{CC_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{out}</text>\n"
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

    // Caption rows bottom-first: the all-zero note, then the cap marker.
    let mut captions: Vec<String> = Vec::new();
    if all_zero {
        captions.push("no findings recorded".to_string());
    }
    if overflow {
        captions.push("… more components omitted".to_string());
    }
    let width = 2 * HC_MARGIN + HC_LABEL_W + HC_PLOT_W;
    let height = 2 * HC_MARGIN
        + HC_HEADER_BAND
        + sorted.len() as i64 * HC_ROW_PITCH
        + caption_band(captions.len());
    let plot_x = HC_MARGIN + HC_LABEL_W;

    let mut svg = String::with_capacity(2048);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    // Heading + scale reference: the subtitle names the unit AND the value a
    // full-width bar represents, so bar lengths are interpretable.
    svg.push_str(&format!(
        "<text x=\"{HC_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\">Hot components — {} {}</text>\n",
        HC_MARGIN + 16,
        sorted.len(),
        if sorted.len() == 1 {
            "component"
        } else {
            "components"
        },
    ));
    svg.push_str(&format!(
        "<text x=\"{HC_MARGIN}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\">bar = finding density per KLOC; max {}</text>\n",
        HC_MARGIN + 36,
        format_fixed_point(max_density, 2),
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
        let y = HC_MARGIN + HC_HEADER_BAND + i as i64 * HC_ROW_PITCH;
        let label = escape_xml(&format!(
            "{} {} {}",
            comp.rank,
            truncate_label(&comp.path_prefix),
            format_fixed_point(comp.metrics.finding_density, 2)
        ));
        // Row labels are right-aligned into the label gutter (closing the
        // label↔bar gap) and vertically centered on their bar.
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\">{label}</text>\n",
            plot_x - 8,
            y + HC_BAR_H / 2 + 4
        ));
        svg.push_str(&format!(
            "<rect x=\"{plot_x}\" y=\"{y}\" width=\"{bar_len}\" height=\"{HC_BAR_H}\" fill=\"{HC_BAR_FILL}\"/>\n"
        ));
    }
    svg.push_str("</g>\n");

    for (k, caption) in captions.iter().enumerate() {
        let y = height - CC_CAPTION_PAD - k as i64 * CC_CAPTION_GAP;
        let out = caption.replace('…', "&#8230;");
        svg.push_str(&format!(
            "<text x=\"{HC_MARGIN}\" y=\"{y}\" font-family=\"monospace\" font-size=\"11\">{out}</text>\n"
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
    fn function_cost_sums_documented_terms() {
        use crate::ir::Span;
        use crate::schema::FunctionStats;
        let costly = FunctionStats {
            id: "a.c#f#1:1".to_string(),
            name: "f".to_string(),
            path: "a.c".to_string(),
            span: Span::point(1, 1),
            param_count: 0,
            cyclomatic_complexity: 3,
            cognitive_complexity: 2,
            max_loop_depth: 1,
            alloc_site_count: 2,
            length_lines: 10,
            call_fanout: Some(4),
            call_fanin: Some(1),
            call_depth: Some(2),
            is_recursive: Some(true),
            idiom_hash: None,
            error_path: Some(true),
            alloc_family: Some("real_alloc".to_string()),
            cfg_gated: None,
            // v1.5: entry/reachability play no role in cost.
            entry_kind: None,
            entry_confidence: None,
            reachable_from_entry: None,
            function_cost_v2: None,
            unknown_trip_loops: 0,
        };
        // 3 + 2 + 2*1 + 3*2 + 4 + 1 + 2 + 2 + 1 + 1 = 24.
        assert_eq!(function_cost(&costly), 24.0);
        // Heat selector: v2 preferred when present, v1 fallback otherwise.
        assert_eq!(function_heat_cost(&costly), 24.0);
        let mut with_v2 = costly.clone();
        with_v2.function_cost_v2 = Some(99);
        assert_eq!(function_heat_cost(&with_v2), 99.0);
        let bare = FunctionStats {
            id: "a.c#g#2:1".to_string(),
            name: "g".to_string(),
            path: "a.c".to_string(),
            span: Span::point(2, 1),
            param_count: 0,
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            max_loop_depth: 0,
            alloc_site_count: 0,
            length_lines: 2,
            // Missing v1.4 fields contribute 0 (pre-step-2 documents overlay
            // less, never NaN).
            call_fanout: None,
            call_fanin: None,
            call_depth: None,
            is_recursive: None,
            idiom_hash: None,
            error_path: None,
            alloc_family: None,
            cfg_gated: None,
            entry_kind: None,
            entry_confidence: None,
            reachable_from_entry: None,
            function_cost_v2: None,
            unknown_trip_loops: 0,
        };
        assert_eq!(function_cost(&bare), 1.0);
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
