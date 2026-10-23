//! Atlas canvas renderer (plan `atlas-build` step 4).
//!
//! Deterministic, hand-rolled SVG + HTML over the [`AtlasView`] model. No I/O,
//! no clock, no randomness, no hash-map iteration; geometry is integer and
//! every traversal has a total tie-break. The HTML fragment carries CSS-only
//! animation; the canonical [`render_canvas_svg`] artifact is the complete
//! static final frame.
//!
//! Canvas purity (A9): only graph chrome (columns, nodes, edges, hulls, path
//! badges, legend, caps, footer) — no tables, cards, bars or metric selectors.
//! Zero `<script>`, zero event-handler attributes, zero network references.
//!
//! Pause semantics: the checkbox freezes the CSS timeline at its current
//! frame; pausing during the initial reveal delay freezes that frame (the A2
//! timed test judges whether that is acceptable).
//!
//! A deep mandatory path grows the canvas vertically (the SVG height is
//! `max(720, TOP + rows*row_h + BOTTOM)`) so nodes are never clipped; print
//! and A2 should judge the tall case.

use std::collections::BTreeMap;

use crate::charts::escape_xml;
use crate::reporter::atlas::{AtlasColumn, AtlasEdge, AtlasNode, AtlasView, DEPTH_COLUMN_CAP};
use crate::reporter::atlas_brief::StartAction;
use crate::reporter::atlas_metrics::{
    NO_ENTRY, NO_SOURCE_FUNCTIONS, format_cost, format_count, format_demoted_files, format_depth,
    format_distinct_edges, format_edge_mix, format_finding, format_multiple, format_parse,
    format_reachable, format_top, format_unresolved_includes,
};
use crate::reporter::escape_html;
use crate::reporter::views::CostScale;

/// Mandatory disclosure for the default-tier structural call graph. This is
/// the single full occurrence on the page (the key-metrics strip); the brief
/// only points back to the Atlas legend.
pub const STRUCTURAL_CAPTION: &str = "structural (LOW name-based + MEDIUM semantic)";
/// Mandatory label for the representative path trace.
pub const PATH_CAPTION: &str = "representative path (static, BFS-derived)";

const SVG_W: i64 = 1400;
/// Graph width used by the embedded `report.html#map` page (the canvas shares
/// the row with the per-file detail rail). The standalone SVG artifact keeps
/// [`SVG_W`].
const PAGE_GRAPH_W: i64 = 868;
const SVG_H: i64 = 720;
const MARGIN_X: i64 = 20;
/// First-row top. Leaves room above the first node for the component-hull
/// label (baseline `TOP - 16`) and the node badge (baseline `TOP - 3`) while
/// keeping both below the column header at [`HEADER_Y`].
const TOP: i64 = 72;
const BOTTOM: i64 = 16;
const HEADER_Y: i64 = 22;
const MAX_COL_W: i64 = 180;
const MIN_NODE_W: i64 = 96;
const MAX_NODE_W: i64 = 150;
/// Minimum node height keeps four readable text lines inside the rect.
const MIN_NODE_H: i64 = 46;
const MAX_NODE_H: i64 = 80;
/// Vertical gap between node rects so badges above a node never touch the row
/// above. Combined with [`MIN_NODE_H`] it sets the effective row floor.
const ROW_GAP: i64 = 12;
/// Component-hull top margin above the first member node, and the hull label
/// offset below the hull rect top. Together the label baseline sits 16px above
/// the first node top and >= 10px above that node's badge baseline.
const HULL_TOP_MARGIN: i64 = 28;
/// Edge-pill collision geometry: the visible pill is [`PILL_H`] tall and
/// `edge_label_width()` wide; collision checks pad it by [`PILL_PAD`] on every
/// side and clear node rects by [`NODE_CLEAR_PAD`].
const PILL_H: i64 = 12;
const PILL_PAD: i64 = 2;
const NODE_CLEAR_PAD: i64 = 4;
const HULL_LABEL_OFFSET: i64 = 12;

const STATIC_CSS: &str = r#".atlas-svg{background:#fbf9f4}
.atlas-svg text{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.atlas-svg .atlas-column-header,.atlas-svg .atlas-hull text{font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
.atlas-edge,.atlas-trace{fill:none}"#;

/// First-page chrome + graph CSS. Every selector is scoped to `#map` so the
/// warm band never touches the white global report; the brief keeps its own
/// scoped styles. Sans chrome, mono identifiers/figures.
const ANIMATION_CSS: &str = r#"#map{background:#f4f1ea;color:#202b35;font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;margin:0 -1rem;padding:8px 1rem;border-radius:0}
#map .atlas-mono{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
#map .atlas-masthead{height:58px;box-sizing:border-box;display:flex;align-items:center;gap:14px;border-bottom:1px solid #d6d0c4}
#map .atlas-brand{flex:0 0 244px;min-width:0}
#map .atlas-kicker{font-size:8px;font-weight:800;letter-spacing:.16em;color:#6f6a61;text-transform:uppercase}
#map .atlas-wordmark{font-size:21px;font-weight:800;letter-spacing:-.015em;line-height:1.05;color:#202b35;white-space:nowrap}
#map .atlas-subline{display:block;font-size:9px;font-weight:600;color:#6f6a61;letter-spacing:.02em;line-height:1.05;margin-top:1px;white-space:normal}
#map .atlas-masthead-spacer{flex:1;min-width:0}
#map .atlas-masthead-actions{flex:0 0 auto;display:flex;align-items:center;gap:8px}
#map .atlas-pause-input{position:absolute;inline-size:1px;block-size:1px;opacity:0}
#map .atlas-pause{display:inline-block;border:1px solid #9d9588;background:#fffdf8;color:#3c4650;padding:6px 10px;border-radius:4px;font-size:9px;font-weight:700;cursor:pointer;user-select:none}
#map .atlas-pause-input:focus-visible + .atlas-pause{outline:2px solid #c2410c;outline-offset:2px}
#map #atlas-pause:checked + .atlas-pause{background:#ece8df;border-color:#c2410c}
#map .atlas-explorer{display:inline-block;background:#1f2d3a;color:#fff;text-decoration:none;padding:7px 10px;border-radius:4px;font-size:9px;font-weight:750}
#map .atlas-keymetrics{border-bottom:1px solid #d6d0c4;padding:2px 0;font-size:9px;line-height:1.15;color:#6f6a61}
#map .atlas-keymetrics-line{display:flex;flex-wrap:wrap;align-items:baseline;gap:1px 4px}
#map .atlas-keymetrics b{color:#202b35;font-weight:700}
#map .atlas-hot{color:#c2410c;font-weight:700}
#map .atlas-chip{display:inline-block;color:#202b35;text-decoration:none;border:1px solid #b8b2a7;border-radius:3px;background:#fffdf8;padding:0 4px;font-size:9.5px;font-weight:600;line-height:1.15;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:34rem;vertical-align:bottom}
#map .atlas-chip:hover{border-color:#c2410c}
#map .atlas-chip .atlas-mono{font-size:9.5px}
#map .atlas-chip--stub{color:#6f6a61;border-style:dashed}
#map .atlas-entry-value{font-size:10px;font-weight:600}
#map .atlas-legend{display:inline-flex;flex-wrap:wrap;align-items:center;gap:2px 9px}
#map .atlas-sw{display:inline-block;width:9px;height:9px;border-radius:2px;vertical-align:-1px;margin-right:3px}
#map .atlas-sw-prod{background:#fdfbf6;border:1px solid #8b8173}
#map .atlas-sw-test{background:#eef2f7;border:1px dashed #7f93aa}
#map .atlas-sw-unreached{background:#e8e5df;border:1px solid #b8b2a7}
#map .atlas-sw-forward{border-top:2px solid #8d8577;width:12px;height:0;border-radius:0;background:none}
#map .atlas-sw-back{border-top:2px dashed #8d8577;width:12px;height:0;border-radius:0;background:none}
#map .atlas-sw-path{border-top:2.3px solid #c2410c;width:12px;height:0;border-radius:0;background:none}
#map .atlas-unit{color:#6f6a61}
#map .atlas-structural-caption{font-weight:700;color:#7c2d12}
#map #atlas-canvas{position:relative;background:transparent;color:#202b35}
#map .atlas-graph{background:#fbf9f4;border:1px solid #d6d0c4;border-radius:6px;overflow:hidden}
#map .atlas-svg{display:block;width:100%;height:auto;background:#fbf9f4}
#map .atlas-svg text{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
#map .atlas-svg .atlas-column-header,#map .atlas-svg .atlas-hull text{font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
#map .atlas-svg .atlas-edge,#map .atlas-svg .atlas-trace{fill:none}
#map .atlas-footer{padding:5px 8px 2px;font-size:9px;line-height:1.35;color:#6f6a61;font-variant-numeric:tabular-nums}
#map .atlas-footer .atlas-mono{color:#202b35}
#map .atlas-footer-part{white-space:nowrap}
#map .atlas-stage{display:flex;gap:10px;align-items:flex-start;justify-content:center}
#map #atlas-canvas{flex:0 0 868px;max-width:868px;min-width:0}
#map .atlas-rail{flex:1 1 0;min-width:0;max-width:530px;color:#202b35}
#map .atlas-rail-title{display:flex;justify-content:space-between;align-items:baseline;gap:8px;padding:2px 1px 5px;border-bottom:1px solid #d6d0c4;font-size:8.5px;font-weight:800;letter-spacing:.11em;color:#6f6a61;text-transform:uppercase}
#map .atlas-rail-title span{font-weight:600;letter-spacing:0;text-transform:none;color:#6f6a61}
#map .atlas-rail-cards{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:6px;margin-top:6px}
#map .atlas-rail-card{min-width:0;overflow:hidden;border:1px solid #d8d2c6;border-left:3px solid #8b8173;border-radius:4px;background:#fffdf8;padding:4px 6px 5px}
#map .atlas-rail-card.is-entry{border-left-color:#1f2d3a;background:#f3f1eb}
#map .atlas-rail-card.is-hot{border-left-color:#c2410c}
#map .atlas-rail-card.is-test{border-left-color:#5c7391;background:#f4f6f9}
#map .atlas-rail-card.is-bench{border-left-color:#7a4f7d;background:#f8f3f8}
#map .atlas-rail-card.is-unreached{border-left-color:#b3aca1;background:#efede8}
#map .atlas-rail-head{display:flex;align-items:baseline;justify-content:space-between;gap:6px}
#map .atlas-rail-name{display:block;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:10px;font-weight:700;color:#202b35;text-decoration:none}
#map .atlas-rail-name:hover{text-decoration:underline}
#map .atlas-rail-role{flex:0 0 auto;font-size:8.5px;font-weight:800;letter-spacing:.08em;color:#3c4650}
#map .atlas-rail-card.is-production .atlas-rail-role{color:#3f6d6a}
#map .atlas-rail-card.is-entry .atlas-rail-role{color:#1f2d3a}
#map .atlas-rail-card.is-test .atlas-rail-role{color:#546a86}
#map .atlas-rail-card.is-bench .atlas-rail-role{color:#7a4f7d}
#map .atlas-rail-card.is-unreached .atlas-rail-role{color:#5f584f}
#map .atlas-rail-anchor{margin-top:2px;font-size:8.5px;line-height:1.25;color:#6f6a61;overflow-wrap:anywhere}
#map .atlas-rail-fn{margin-top:2px;font-size:8.5px;line-height:1.3;color:#3f4952;overflow-wrap:anywhere}
#map .atlas-rail-fn a{color:#202b35;text-decoration:none}
#map .atlas-rail-fn a:hover{text-decoration:underline}
#map .atlas-rail-find{margin-top:3px;font-size:8.5px;line-height:1.25;color:#8a5a2b;font-weight:650;overflow-wrap:anywhere}
#map .atlas-rail-find a{color:#7c2d12}
#map .atlas-rail-stub{margin:6px 0 0;padding:6px 8px;border:1px dashed #b8b2a7;border-radius:4px;font-size:9px;color:#6f6a61}
#map .atlas-rail-more{margin:6px 0 0;font-size:8.5px;color:#6f6a61}
@media (max-width:1359px){#map .atlas-stage{flex-direction:column;align-items:center}#map #atlas-canvas,#map .atlas-rail{flex:0 0 auto;width:868px;max-width:100%}}
@media (min-width:1360px) and (max-width:1439px){#map .atlas-rail-name{white-space:normal;overflow-wrap:anywhere;text-overflow:clip}}
@media print{#map .atlas-stage{display:block}#map .atlas-rail{width:auto;max-width:none;margin-top:8px}}
@keyframes atlas-entry-pulse{0%{opacity:.45}100%{opacity:1}}
@keyframes atlas-column-reveal{0%{opacity:0;transform:translateY(4px)}100%{opacity:1;transform:none}}
 @keyframes atlas-edge-draw{from{stroke-dashoffset:1}to{stroke-dashoffset:0}}
 @keyframes atlas-edge-fade{from{opacity:0}to{opacity:1}}
 @keyframes atlas-trace-draw{from{stroke-dashoffset:1}to{stroke-dashoffset:0}}
 @keyframes atlas-hottest-pulse{0%,100%{opacity:1}50%{opacity:.5}}
 #map #atlas-canvas .atlas-column{animation:atlas-column-reveal .4s calc(1.2s + var(--atlas-col) * .12s) both}
 #map #atlas-canvas .atlas-node.is-entry{animation:atlas-entry-pulse 1.2s ease-out both}
 #map #atlas-canvas .atlas-edge--forward{animation:atlas-edge-draw 1.4s calc(2.8s + var(--atlas-edge,0) * .01s) both}
 #map #atlas-canvas .atlas-edge--back{animation:atlas-edge-fade .4s calc(2.8s + var(--atlas-edge,0) * .01s) both}
 #map #atlas-canvas .atlas-trace{animation:atlas-trace-draw 1.2s 4.2s both}
#map #atlas-canvas .atlas-node.is-hottest{animation:atlas-hottest-pulse .8s 5.4s 2 both}
#map:has(#atlas-pause:checked) #atlas-canvas .atlas-graph *{animation-play-state:paused !important}
@media (prefers-reduced-motion: reduce){#map #atlas-canvas *{animation:none !important;opacity:1 !important;transform:none !important;stroke-dashoffset:0 !important}}
@media print{#map{background:#fff}#map #atlas-canvas{break-inside:avoid}#map #atlas-canvas *{animation:none !important;opacity:1 !important;transform:none !important;stroke-dashoffset:0 !important}#map .atlas-pause,#map .atlas-masthead-actions{display:none}}
#map .atlas-pause-input{opacity:0!important;inline-size:1px!important;block-size:1px!important}"#;

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct NodeRect {
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

#[derive(Debug, Clone, Copy)]
struct ColumnLayout {
    x: i64,
    width: i64,
    node_x: i64,
    node_w: i64,
}

struct Layout {
    width: i64,
    row_h: i64,
    node_h: i64,
    height: i64,
    columns: Vec<ColumnLayout>,
    rows: BTreeMap<String, NodeRect>,
}

fn compute_layout(view: &AtlasView, width: i64) -> Layout {
    let max_items = view
        .columns
        .iter()
        .map(|column| {
            column.files.len()
                + column.aggregated.len()
                + usize::from(
                    column.unreached
                        && (view.coverage.unreached_production > 0
                            || view.coverage.unreached_nonproduction > 0),
                )
        })
        .max()
        .unwrap_or(1)
        .max(1);
    let available_h = SVG_H - TOP - BOTTOM;
    // `row_h = max(54, available_h / max_items)` floors at 54 in the plan, but
    // the 46px node minimum plus the 12px inter-row gap require 58 for badges
    // to clear the row above. No upper cap: capped/deep states grow the SVG.
    let row_h = (available_h / max_items as i64).max(MIN_NODE_H + ROW_GAP);
    let node_h = (row_h - ROW_GAP).clamp(MIN_NODE_H, MAX_NODE_H);
    // Rows start directly below the column headers: no vertical centring of
    // the row block inside a taller canvas. Sparse states keep the minimum
    // canvas height and strand the leftover space below the last row.
    let height = 480.max(TOP + max_items as i64 * row_h + BOTTOM);

    let count = view.columns.len();
    let mut columns = Vec::with_capacity(count);
    if count > 0 {
        let column_w = ((width - 2 * MARGIN_X) / count as i64).min(MAX_COL_W);
        let grid_w = column_w * count as i64;
        let start_x = (width - grid_w) / 2;
        for index in 0..count {
            let x = start_x + index as i64 * column_w;
            let node_w = (column_w - 16)
                .clamp(MIN_NODE_W, MAX_NODE_W)
                .min(column_w - 8);
            let node_x = x + (column_w - node_w) / 2;
            columns.push(ColumnLayout {
                x,
                width: column_w,
                node_x,
                node_w,
            });
        }
    }

    let mut rows: BTreeMap<String, NodeRect> = BTreeMap::new();
    for (index, column) in view.columns.iter().enumerate() {
        let layout = &columns[index];
        for (row, path) in column.files.iter().enumerate() {
            let y = TOP + row as i64 * row_h;
            rows.insert(
                path.clone(),
                NodeRect {
                    x: layout.node_x,
                    y,
                    w: layout.node_w,
                    h: node_h,
                },
            );
        }
    }

    Layout {
        width,
        row_h,
        node_h,
        height,
        columns,
        rows,
    }
}

#[derive(Debug, Clone)]
struct EdgeShape {
    path: String,
    label_x: i64,
    label_y: i64,
}

/// Bulge used by back-edge cubic controls. Kept in one place so the collision
/// resolver can place a pill just outside the same outer curve.
fn back_bulge(edge: &AtlasEdge) -> i64 {
    (28 + (edge.count.min(8) as i64) * 4).min(72)
}

/// Padded pill box used only for collision checks (not for drawing).
fn edge_pill_rect(label_x: i64, label_y: i64, count: usize) -> NodeRect {
    let w = edge_label_width(count) + 2 * PILL_PAD;
    let h = PILL_H + 2 * PILL_PAD;
    NodeRect {
        x: label_x - w / 2,
        y: label_y - h / 2,
        w,
        h,
    }
}

fn rects_overlap(a: NodeRect, b: NodeRect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn pill_is_clear(pill: NodeRect, layout: &Layout) -> bool {
    layout.rows.values().all(|rect| {
        let clear = NodeRect {
            x: rect.x - NODE_CLEAR_PAD,
            y: rect.y - NODE_CLEAR_PAD,
            w: rect.w + 2 * NODE_CLEAR_PAD,
            h: rect.h + 2 * NODE_CLEAR_PAD,
        };
        !rects_overlap(pill, clear)
    })
}

fn pill_in_bounds(pill: NodeRect, layout: &Layout) -> bool {
    pill.x >= MARGIN_X
        && pill.x + pill.w <= layout.width - MARGIN_X
        && pill.y >= 0
        && pill.y + pill.h <= layout.height
}

/// True when the drawn pill (no collision padding) overlaps a node rect. This
/// is the trigger: pills that merely sit close to a node keep their midpoint,
/// while the 4px node clearance plus pill padding is applied to every
/// candidate position considered by [`resolve_edge_label`].
fn pill_hits_node(label_x: i64, label_y: i64, count: usize, layout: &Layout) -> bool {
    let width = edge_label_width(count);
    let pill = NodeRect {
        x: label_x - width / 2,
        y: label_y - PILL_H / 2,
        w: width,
        h: PILL_H,
    };
    layout.rows.values().any(|rect| rects_overlap(pill, *rect))
}

/// Deterministic edge-pill placement: keep the curve midpoint unless the drawn
/// pill overlaps a node. Candidates are tried in a fixed order — outer side of
/// the back-edge bulge, vertical row offsets, then horizontal column offsets —
/// and the first point that is in-bounds and clear of every node rect
/// (expanded by [`NODE_CLEAR_PAD`], with the pill padded by [`PILL_PAD`]) wins.
/// If nothing clears, the base point is kept so no label is ever dropped.
fn resolve_edge_label(
    edge: &AtlasEdge,
    from: NodeRect,
    to: NodeRect,
    base_x: i64,
    base_y: i64,
    layout: &Layout,
) -> (i64, i64) {
    if !pill_hits_node(base_x, base_y, edge.count, layout) {
        return (base_x, base_y);
    }

    let candidate = |x: i64, y: i64| {
        let pill = edge_pill_rect(x, y, edge.count);
        (pill_in_bounds(pill, layout) && pill_is_clear(pill, layout)).then_some((x, y))
    };

    if edge.back {
        let outer_x = (from.x + from.w).max(to.x + to.w) + back_bulge(edge) / 2 + 8;
        if let Some(position) = candidate(outer_x, base_y) {
            return position;
        }
    }

    for factor in [0.5_f64, 1.0, 1.5] {
        let delta = (layout.row_h as f64 * factor).round() as i64;
        if let Some(position) = candidate(base_x, base_y - delta) {
            return position;
        }
        if let Some(position) = candidate(base_x, base_y + delta) {
            return position;
        }
    }

    let half_column = layout.columns.first().map_or(0, |column| column.width / 2);
    if let Some(position) = candidate(base_x - half_column, base_y) {
        return position;
    }
    if let Some(position) = candidate(base_x + half_column, base_y) {
        return position;
    }

    (base_x, base_y)
}

fn edge_shape(edge: &AtlasEdge, layout: &Layout) -> Option<EdgeShape> {
    let rows = &layout.rows;
    let from = *rows.get(&edge.from)?;
    let to = *rows.get(&edge.to)?;
    let source_x = from.x + from.w;
    let source_y = from.y + from.h / 2;
    let (target_x, target_y, control_one_x, control_two_x) = if edge.back {
        let target_x = to.x + to.w;
        let target_y = to.y + to.h / 2;
        let bulge = back_bulge(edge);
        (target_x, target_y, source_x + bulge, target_x + bulge)
    } else {
        let target_x = to.x;
        let target_y = to.y + to.h / 2;
        let reach = ((target_x - source_x).abs() / 2).max(20);
        (target_x, target_y, source_x + reach, target_x - reach)
    };
    let path = format!(
        "M {source_x} {source_y} C {control_one_x} {source_y} {control_two_x} {target_y} {target_x} {target_y}"
    );
    let base_x = (source_x + 3 * control_one_x + 3 * control_two_x + target_x) / 8;
    let base_y = (source_y + target_y) / 2;
    let (label_x, label_y) = resolve_edge_label(edge, from, to, base_x, base_y, layout);
    Some(EdgeShape {
        path,
        label_x,
        label_y,
    })
}

// ---------------------------------------------------------------------------
// Small text helpers
// ---------------------------------------------------------------------------

/// Mid-ellipsis: keep the first ~half and the last ~half of `text` so a
/// truncated path still starts with its real first segment and never begins
/// with a dot/ellipsis (the F6 audit defect was leading-dot truncation). The
/// full path always stays available in the `<title>`.
pub(crate) fn mid_ellipsis(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return text.chars().take(max_chars).collect();
    }
    let keep = max_chars - 1;
    let front = keep.div_ceil(2);
    let back = keep - front;
    let mut out: String = text.chars().take(front).collect();
    out.push('…');
    if back > 0 {
        out.extend(text.chars().skip(count - back));
    }
    out
}

fn files_label(count: usize) -> String {
    format!(
        "{} {}",
        format_count(count as u64),
        if count == 1 { "file" } else { "files" }
    )
}

/// Monospace advance estimate (~0.62em per character). When a line would
/// exceed the box, SVG `textLength` compresses the glyph run instead of
/// shrinking the font or spilling past the node rect.
fn fit_attrs(text: &str, size: f64, max_w: i64) -> String {
    let estimated = text.chars().count() as f64 * size * 0.62;
    if estimated > max_w as f64 {
        format!(
            " textLength=\"{}\" lengthAdjust=\"spacingAndGlyphs\"",
            max_w.max(1)
        )
    } else {
        String::new()
    }
}

/// Compact visible edge label (M-10 metric stays in the group `<title>`).
/// Counts above 999 are clamped so the visible pill stays bounded; the full
/// count remains in the `<title>` via [`format_distinct_edges`].
fn edge_label_text(count: usize) -> String {
    if count > 999 {
        "×999+".to_string()
    } else {
        format!("×{count}")
    }
}

/// Estimated pill width: monospace advance (~0.62em) plus horizontal padding,
/// so the label fits the ~30px inter-column gap. Five-character labels (the
/// `×999+` clamp) use 5.5px padding to stay inside the 32px preflight bound at
/// the 8.5px visible floor.
fn edge_label_width(count: usize) -> i64 {
    let text = edge_label_text(count);
    let padding = if text.chars().count() >= 5 { 5.5 } else { 8.0 };
    let advance = text.chars().count() as f64 * 8.5 * 0.62;
    (advance + padding).ceil() as i64
}

fn node_rank_text(node: &AtlasNode) -> String {
    let mut parts = Vec::new();
    if let Some(ratio) = node.ratio {
        parts.push(format!("{} median function", format_multiple(ratio)));
    }
    if node.file_population > 0 {
        parts.push(format!(
            "{} of files",
            format_top(node.file_rank, node.file_population)
        ));
    }
    parts.join(" · ")
}

/// The two rank lines drawn inside a node (the joined anchor text used in
/// titles and the answer line stays [`node_rank_text`]).
fn node_rank_lines(node: &AtlasNode) -> (Option<String>, Option<String>) {
    let multiple = node
        .ratio
        .map(|ratio| format!("{} median function", format_multiple(ratio)));
    let top = (node.file_population > 0).then(|| {
        format!(
            "{} of files",
            format_top(node.file_rank, node.file_population)
        )
    });
    (multiple, top)
}

fn node_anchor_text(node: &AtlasNode) -> String {
    let mut parts = vec![format_cost(node.cost, node.scale)];
    let rank = node_rank_text(node);
    if !rank.is_empty() {
        parts.push(rank);
    }
    parts.join(" · ")
}

fn node_badges(node: &AtlasNode) -> String {
    let mut badges = Vec::new();
    if node.entry {
        badges.push("ENTRY".to_string());
    }
    if node.hottest {
        badges.push("HOTTEST".to_string());
    }
    if let Some(finding) = &node.finding {
        badges.push(format_finding(finding.severity, finding.count));
    }
    if node.unreached {
        badges.push("unreached".to_string());
    }
    if node.partial_parse {
        badges.push("partial".to_string());
    }
    if !node.production {
        badges.push(node.path_class.clone());
    }
    badges.join(" · ")
}

fn node_title(node: &AtlasNode) -> String {
    let mut parts = vec![
        node.path.clone(),
        node.path_class.clone(),
        format!("file total {}", node_anchor_text(node)),
    ];
    if !node.top_functions.is_empty() {
        let functions = node
            .top_functions
            .iter()
            .map(|function| format!("{} @ {}", function.name, function.line))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("top functions: {functions}"));
    }
    parts.join(" · ")
}

/// Header font size and line-2 baseline. Column headers are two lines so the
/// long `UNREACHED` label keeps to its own column; line 2 clears the first
/// node row (`TOP = 72`) with room to spare.
const HEADER_FONT_SIZE: f64 = 10.5;
const HEADER_LINE2_Y: i64 = 36;
/// Monospace advance estimate (~0.62em per character) used to keep each
/// header line inside its column.
const HEADER_CHAR_EM: f64 = 0.62;

/// Estimated rendered width of `text` at `size` in the header's monospace face.
fn estimated_header_width(text: &str, size: f64) -> f64 {
    text.chars().count() as f64 * size * HEADER_CHAR_EM
}

/// Mid-ellipsis `line` so its estimated width stays within `max_w`.
fn fit_header_line(line: &str, max_w: i64) -> String {
    if estimated_header_width(line, HEADER_FONT_SIZE) <= max_w as f64 {
        return line.to_string();
    }
    let max_chars = (max_w as f64 / (HEADER_FONT_SIZE * HEADER_CHAR_EM)).floor() as usize;
    mid_ellipsis(line, max_chars.max(1))
}

/// Two-line column header. Line 1 names the depth/entry (or `UNREACHED`);
/// line 2 carries the file count and aggregation note.
fn column_header_lines(view: &AtlasView, column: &AtlasColumn) -> (String, String) {
    let mut count = files_label(column.file_count);
    if !column.aggregated.is_empty() {
        count.push_str(&format!(" · {} aggregated", column.aggregated.len()));
    }
    if column.unreached {
        // Global unreached totals live in the footer and the combined chip;
        // the header only names the reason (F9: no ambiguous "N shown").
        ("UNREACHED".to_string(), "no forward path".to_string())
    } else if column.folded {
        (format!("depth {DEPTH_COLUMN_CAP}+"), count)
    } else if column.depth == 0 && view.entry.point.is_some() {
        (format!("{} · ENTRY", format_depth(0)), count)
    } else {
        (format_depth(column.depth), count)
    }
}

// ---------------------------------------------------------------------------
// SVG fragments
// ---------------------------------------------------------------------------

fn svg_open(standalone: bool, width: i64, height: i64) -> String {
    let xmlns = if standalone {
        " xmlns=\"http://www.w3.org/2000/svg\""
    } else {
        ""
    };
    format!(
        "<svg{xmlns} viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" \
         class=\"atlas-svg\" role=\"img\" aria-label=\"Perfpal Atlas entry-flow canvas\" \
         preserveAspectRatio=\"xMidYMid meet\">"
    )
}

fn render_defs() -> String {
    concat!(
        "<defs>",
        "<marker id=\"atlas-arrow\" viewBox=\"0 0 8 8\" refX=\"7\" refY=\"4\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">",
        "<path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#8d8577\"/>",
        "</marker>",
        "<marker id=\"atlas-arrow-back\" viewBox=\"0 0 8 8\" refX=\"7\" refY=\"4\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">",
        "<path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#8d8577\"/>",
        "</marker>",
        "<marker id=\"atlas-arrow-path\" viewBox=\"0 0 8 8\" refX=\"7\" refY=\"4\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">",
        "<path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#c2410c\"/>",
        "</marker>",
        "</defs>",
    )
    .to_string()
}

fn render_stub(view: &AtlasView, width: i64) -> String {
    let (message, detail) = if view.state.empty {
        (NO_SOURCE_FUNCTIONS.to_string(), String::new())
    } else {
        (
            NO_ENTRY.to_ascii_uppercase(),
            "no electable production entry".to_string(),
        )
    };
    let center = width / 2;
    let mut out = String::from("<g class=\"atlas-stub\" role=\"note\">");
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"300\" width=\"500\" height=\"120\" rx=\"12\" fill=\"#fbf9f4\" \
         stroke=\"#b8b2a7\" stroke-dasharray=\"6 4\"/>",
        center - 250
    ));
    out.push_str(&format!(
        "<text x=\"{center}\" y=\"352\" text-anchor=\"middle\" font-size=\"18\" font-weight=\"700\" \
         fill=\"#202b35\">{}</text>",
        escape_xml(&message)
    ));
    if !detail.is_empty() {
        out.push_str(&format!(
            "<text x=\"{center}\" y=\"376\" text-anchor=\"middle\" font-size=\"13.5\" fill=\"#6f6a61\">{}</text>",
            escape_xml(&detail)
        ));
    }
    out.push_str("</g>");
    out
}

/// Hull label baseline for a group whose topmost member starts at `min_y`.
/// The `.max(0)` keeps the group rect on-canvas for the first row.
fn hull_label_baseline(min_y: i64) -> i64 {
    (min_y - HULL_TOP_MARGIN).max(0) + HULL_LABEL_OFFSET
}

/// Node badge baseline for a node whose top edge is `rect.y`. Badges always sit
/// above the node: `compute_layout` guarantees `row_h >= MIN_NODE_H + ROW_GAP`
/// and `node_h <= row_h - ROW_GAP`, so the inter-row gap always holds a badge.
fn node_badge_baseline(rect: NodeRect) -> i64 {
    rect.y - 3
}

fn render_hulls(view: &AtlasView, rows: &BTreeMap<String, NodeRect>) -> String {
    let mut groups: BTreeMap<&str, Vec<&AtlasNode>> = BTreeMap::new();
    for node in &view.nodes {
        groups
            .entry(node.component.as_str())
            .or_default()
            .push(node);
    }
    let mut out = String::new();
    for (prefix, members) in &groups {
        if members.len() < 2 {
            continue;
        }
        let mut min_x = i64::MAX;
        let mut min_y = i64::MAX;
        let mut max_x = i64::MIN;
        let mut max_y = i64::MIN;
        for member in members {
            if let Some(rect) = rows.get(&member.path) {
                min_x = min_x.min(rect.x);
                min_y = min_y.min(rect.y);
                max_x = max_x.max(rect.x + rect.w);
                max_y = max_y.max(rect.y + rect.h);
            }
        }
        if min_x == i64::MAX {
            continue;
        }
        let x = (min_x - 8).max(0);
        let y = (min_y - HULL_TOP_MARGIN).max(0);
        let width = (max_x + 8 - x).max(20);
        let height = (max_y + 8 - y).max(20);
        out.push_str(&format!(
            "<g class=\"atlas-hull\"><title>approximate prefix group; not a canonical component id</title>\
             <rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"10\" fill=\"none\" \
             stroke=\"#d6d0c4\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>\
             <text x=\"{}\" y=\"{}\" font-size=\"9.2\" font-weight=\"700\" fill=\"#6f6a61\">{}</text></g>",
            x + 8,
            hull_label_baseline(min_y),
            escape_xml(&format!("{prefix} · {}", members.len()))
        ));
    }
    out
}

fn ordered_edges<'a>(view: &'a AtlasView, rows: &BTreeMap<String, NodeRect>) -> Vec<&'a AtlasEdge> {
    let mut edges: Vec<&AtlasEdge> = view.edges.iter().collect();
    edges.sort_by_key(|edge| {
        let from = rows.get(&edge.from);
        let to = rows.get(&edge.to);
        (
            from.map_or(i64::MAX, |rect| rect.x),
            from.map_or(i64::MAX, |rect| rect.y),
            to.map_or(i64::MAX, |rect| rect.x),
            to.map_or(i64::MAX, |rect| rect.y),
            edge.from.clone(),
            edge.to.clone(),
        )
    });
    edges
}

fn render_aggregate(aggregate: &crate::reporter::atlas::AtlasAggregate, rect: NodeRect) -> String {
    let title = format!(
        "{} · {} aggregated files · {} calls in",
        format_depth(aggregate.depth),
        format_count(aggregate.count as u64),
        format_count(aggregate.incoming_calls as u64)
    );
    format!(
        "<g class=\"atlas-aggregate\" data-aggregate=\"depth-{}\"><title>{}</title>\
         <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"#f0ede6\" stroke=\"#b8b2a7\" \
         stroke-dasharray=\"5 4\"/>\
         <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9.1\" font-weight=\"700\" fill=\"#3c4650\">+{} more</text>\
         <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"8.6\" fill=\"#6f6a61\">{} calls in</text></g>",
        aggregate.depth,
        escape_xml(&title),
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        rect.x + rect.w / 2,
        rect.y + rect.h / 2 - 1,
        format_count(aggregate.count as u64),
        rect.x + rect.w / 2,
        rect.y + rect.h / 2 + 8,
        format_count(aggregate.incoming_calls as u64)
    )
}

fn render_node(
    node: &AtlasNode,
    rect: NodeRect,
    path_step: Option<usize>,
    total_steps: usize,
    label_source: &str,
) -> String {
    // Tall nodes get a slightly larger name; every node keeps the four-line
    // block (name, cost, multiple, top) at >=10px / >=8.5px.
    let (name_size, meta_size) = if rect.h >= 60 {
        (11.5_f64, 9.0_f64)
    } else {
        (10.0_f64, 8.5_f64)
    };
    let name_max_w = rect.w - 10 - if path_step.is_some() { 14 } else { 0 };
    let max_chars = ((name_max_w.max(20) as f64) / (name_size * 0.62))
        .floor()
        .max(4.0) as usize;
    // The canvas label is the file name (spec §3.2), with a parent segment
    // when drawn files share a name; the full path stays in the node
    // `<title>` and on the rail card.
    let name = mid_ellipsis(label_source, max_chars);
    let (multiple, top) = node_rank_lines(node);
    let cost = format_cost(node.cost, node.scale);

    // Distribute four baselines through the rect. At the 46px node floor the
    // last descender can graze the bottom edge after baseline rounding; taller
    // rows (the common case) keep it comfortably inside.
    let gap = ((rect.h as f64 - 12.0) / 4.0).clamp(8.5, 15.0);
    let top_pad = (rect.h as f64 - 3.0 * gap) / 2.0;
    let name_y = (rect.y as f64 + top_pad + name_size * 0.75).round() as i64;
    let cost_y = name_y + gap.round() as i64;
    let multiple_y = cost_y + gap.round() as i64;
    let top_y = multiple_y + gap.round() as i64;

    let text_fill = if node.entry { "#ffffff" } else { "#23303a" };
    // Opaque G fills for non-production nodes (no hatch-through-text); bench
    // keeps its own lilac fill. Entry/unreached/production stay solid.
    let (fill, mut stroke, mut stroke_width, dash) = if node.entry {
        ("#1f2d3a", "#1f2d3a", 2, "")
    } else if node.unreached {
        ("#e8e5df", "#b8b2a7", 1, "")
    } else if !node.production {
        if node.path_class == "bench" {
            ("#f7f0f7", "#8a6f9e", 1, "3 2")
        } else {
            ("#eef2f7", "#7f93aa", 1, "3 2")
        }
    } else {
        ("#fdfbf6", "#8b8173", 1, "")
    };
    if node.hottest {
        stroke = "#c2410c";
        stroke_width = 2;
    }
    let dash_attr = if dash.is_empty() {
        String::new()
    } else {
        format!(" stroke-dasharray=\"{dash}\"")
    };

    let mut classes = String::from("atlas-node");
    if node.entry {
        classes.push_str(" is-entry");
    }
    if node.hottest {
        classes.push_str(" is-hottest");
    }
    if node.unreached {
        classes.push_str(" is-unreached");
    }
    if !node.production {
        classes.push_str(" is-nonproduction");
    }
    if node.partial_parse {
        classes.push_str(" is-partial");
    }

    let text_max_w = rect.w - 8;
    let mut out = String::new();
    out.push_str(&format!(
        "<g class=\"{classes}\" data-node=\"{}\">",
        escape_xml(&node.path)
    ));
    out.push_str(&format!("<title>{}</title>", escape_xml(&node_title(node))));
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"7\" fill=\"{fill}\" stroke=\"{stroke}\" \
         stroke-width=\"{stroke_width}\"{dash_attr}/>",
        rect.x, rect.y, rect.w, rect.h
    ));
    let badges = node_badges(node);
    if !badges.is_empty() {
        // The layout guarantees a full inter-row gap, so badges sit above the
        // node's top-right corner (never over the centred name block).
        let (badge_x, badge_y) = (rect.x + rect.w - 2, node_badge_baseline(rect));
        out.push_str(&format!(
            "<text class=\"atlas-node-badge\" x=\"{}\" y=\"{}\" text-anchor=\"end\" \
             font-size=\"8.5\" fill=\"#6f6a61\"{}>{}</text>",
            badge_x,
            badge_y,
            fit_attrs(&badges, 8.5, rect.w - 4),
            escape_xml(&badges)
        ));
    }
    out.push_str(&format!(
        "<text class=\"atlas-node-name\" x=\"{}\" y=\"{name_y}\" text-anchor=\"middle\" \
         font-size=\"{name_size}\" font-weight=\"700\" fill=\"{text_fill}\">{}</text>",
        rect.x + rect.w / 2,
        escape_xml(&name)
    ));
    out.push_str(&format!(
        "<text class=\"atlas-node-meta atlas-node-cost\" x=\"{}\" y=\"{cost_y}\" \
         text-anchor=\"middle\" font-size=\"{meta_size}\" fill=\"{text_fill}\"{}>{}</text>",
        rect.x + rect.w / 2,
        fit_attrs(&cost, meta_size, text_max_w),
        escape_xml(&cost)
    ));
    if let Some(multiple) = &multiple {
        out.push_str(&format!(
            "<text class=\"atlas-node-meta atlas-node-rank\" x=\"{}\" y=\"{multiple_y}\" \
             text-anchor=\"middle\" font-size=\"{meta_size}\" fill=\"{text_fill}\"{}>{}</text>",
            rect.x + rect.w / 2,
            fit_attrs(multiple, meta_size, text_max_w),
            escape_xml(multiple)
        ));
    }
    if let Some(top) = &top {
        out.push_str(&format!(
            "<text class=\"atlas-node-meta atlas-node-rank\" x=\"{}\" y=\"{top_y}\" \
             text-anchor=\"middle\" font-size=\"{meta_size}\" fill=\"{text_fill}\"{}>{}</text>",
            rect.x + rect.w / 2,
            fit_attrs(top, meta_size, text_max_w),
            escape_xml(top)
        ));
    }
    if let Some(step) = path_step {
        let cx = rect.x + rect.w - 10;
        let cy = rect.y + 10;
        let title = format!("{PATH_CAPTION} · step {step} of {total_steps}");
        out.push_str(&format!(
            "<g class=\"atlas-path-badge\"><title>{}</title>\
             <circle cx=\"{cx}\" cy=\"{cy}\" r=\"8\" fill=\"#c2410c\" stroke=\"#ffffff\" stroke-width=\"1\"/>\
             <text x=\"{cx}\" y=\"{}\" text-anchor=\"middle\" font-size=\"9.5\" font-weight=\"700\" \
             fill=\"#ffffff\">{step}</text></g>",
            escape_xml(&title),
            cy + 3
        ));
    }
    out.push_str("</g>");
    out
}

/// Collision-aware canvas labels. The file name is the default; when two drawn
/// files share a name, add the parent directory (`time/mod.rs`); if that still
/// collides, fall back to the full path. The full path always stays in the
/// node `<title>`, and the rail carries the path for every top file.
fn node_labels<'a>(nodes: &BTreeMap<&'a str, &'a AtlasNode>) -> BTreeMap<&'a str, String> {
    let mut name_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for node in nodes.values() {
        *name_counts.entry(node.file_name.as_str()).or_insert(0) += 1;
    }
    let mut candidates: Vec<(&'a str, String)> = Vec::with_capacity(nodes.len());
    for (path, node) in nodes {
        let label = if name_counts
            .get(node.file_name.as_str())
            .copied()
            .unwrap_or(0)
            > 1
        {
            match std::path::Path::new(path)
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|parent| parent.to_str())
            {
                Some(parent) => format!("{parent}/{}", node.file_name),
                None => node.file_name.clone(),
            }
        } else {
            node.file_name.clone()
        };
        candidates.push((*path, label));
    }
    let mut candidate_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (_, label) in &candidates {
        *candidate_counts.entry(label.clone()).or_insert(0) += 1;
    }
    candidates
        .into_iter()
        .map(|(path, label)| {
            let final_label = if candidate_counts.get(&label).copied().unwrap_or(0) > 1 {
                path.to_string()
            } else {
                label
            };
            (path, final_label)
        })
        .collect()
}

fn render_nodes(
    view: &AtlasView,
    layout: &Layout,
    nodes: &BTreeMap<&str, &AtlasNode>,
    path_steps: &BTreeMap<&str, usize>,
) -> String {
    let labels = node_labels(nodes);
    let mut out = String::new();
    for (index, column) in view.columns.iter().enumerate() {
        let column_layout = &layout.columns[index];
        let mut classes = String::from("atlas-column");
        if column.unreached {
            classes.push_str(" is-unreached");
        }
        out.push_str(&format!(
            "<g class=\"{classes}\" style=\"--atlas-col:{index}\">"
        ));
        let (line1, line2) = column_header_lines(view, column);
        let fitted1 = fit_header_line(&line1, column_layout.width);
        let fitted2 = fit_header_line(&line2, column_layout.width);
        let title = if fitted1 != line1 || fitted2 != line2 {
            format!(
                "<title>{}</title>",
                escape_xml(&format!("{line1} · {line2}"))
            )
        } else {
            String::new()
        };
        let center_x = column_layout.x + column_layout.width / 2;
        out.push_str(&format!(
            "<g class=\"atlas-column-header-group\">{title}\
             <text class=\"atlas-column-header\" x=\"{center_x}\" y=\"{HEADER_Y}\" text-anchor=\"middle\" \
             font-size=\"{HEADER_FONT_SIZE}\" font-weight=\"700\" fill=\"#202b35\">{}</text>\
             <text class=\"atlas-column-header\" x=\"{center_x}\" y=\"{HEADER_LINE2_Y}\" text-anchor=\"middle\" \
             font-size=\"{HEADER_FONT_SIZE}\" fill=\"#6f6a61\">{}</text></g>",
            escape_xml(&fitted1),
            escape_xml(&fitted2)
        ));
        for path in &column.files {
            let Some(node) = nodes.get(path.as_str()) else {
                continue;
            };
            let Some(rect) = layout.rows.get(path).copied() else {
                continue;
            };
            let step = path_steps.get(path.as_str()).copied();
            let label = labels
                .get(path.as_str())
                .map(String::as_str)
                .unwrap_or(node.file_name.as_str());
            out.push_str(&render_node(node, rect, step, view.path.len(), label));
        }
        for (aggregate_index, aggregate) in column.aggregated.iter().enumerate() {
            let row = (column.files.len() + aggregate_index) as i64;
            let y = TOP + row * layout.row_h;
            out.push_str(&render_aggregate(
                aggregate,
                NodeRect {
                    x: column_layout.node_x,
                    y,
                    w: column_layout.node_w,
                    h: layout.node_h,
                },
            ));
        }
        // The chip reports the same global unreached totals as the footer,
        // never a not-drawn subset (so the two always agree).
        let prod_unreached = view.coverage.unreached_production;
        let nonprod_unreached = view.coverage.unreached_nonproduction;
        if column.unreached && (prod_unreached > 0 || nonprod_unreached > 0) {
            let row = (column.files.len() + column.aggregated.len()) as i64;
            let y = TOP + row * layout.row_h;
            let mut lines = Vec::new();
            if prod_unreached > 0 {
                lines.push(format!(
                    "production unreached: {}",
                    format_count(prod_unreached as u64)
                ));
            }
            if nonprod_unreached > 0 {
                lines.push(format!(
                    "non-production unreached: {}",
                    format_count(nonprod_unreached as u64)
                ));
            }
            let title = format!(
                "{} production unreached · {} non-production unreached · no estimated reachability",
                format_count(prod_unreached as u64),
                format_count(nonprod_unreached as u64)
            );
            let center_x = column_layout.node_x + column_layout.node_w / 2;
            let mut text = String::new();
            if lines.len() == 1 {
                text.push_str(&format!(
                    "<text x=\"{center_x}\" y=\"{}\" text-anchor=\"middle\" font-size=\"8.5\" \
                     font-weight=\"700\" fill=\"#6f6a61\"{}>{}</text>",
                    y + layout.node_h / 2 + 3,
                    fit_attrs(&lines[0], 8.5, column_layout.node_w - 8),
                    escape_xml(&lines[0])
                ));
            } else {
                for (index, line) in lines.iter().enumerate() {
                    let baseline = y + layout.node_h / 2 - 2 + index as i64 * 11;
                    text.push_str(&format!(
                        "<text x=\"{center_x}\" y=\"{baseline}\" text-anchor=\"middle\" \
                         font-size=\"8.5\" font-weight=\"700\" fill=\"#6f6a61\"{}>{}</text>",
                        fit_attrs(line, 8.5, column_layout.node_w - 8),
                        escape_xml(line)
                    ));
                }
            }
            out.push_str(&format!(
                "<g class=\"atlas-unreached-chip\" data-prod-unreached=\"{prod_unreached}\" \
                 data-nonprod-unreached=\"{nonprod_unreached}\"><title>{}</title>\
                 <rect x=\"{}\" y=\"{y}\" width=\"{}\" height=\"{}\" rx=\"6\" \
                 fill=\"#efede8\" stroke=\"#b8b2a7\" stroke-dasharray=\"4 3\"/>{text}</g>",
                escape_xml(&title),
                column_layout.node_x,
                column_layout.node_w,
                layout.node_h,
            ));
        }
        out.push_str("</g>");
    }
    out
}

fn render_graph_inner(view: &AtlasView, layout: &Layout) -> String {
    let mut out = String::new();
    out.push_str(&render_defs());
    if view.columns.is_empty() {
        out.push_str(&render_stub(view, layout.width));
        return out;
    }

    let node_map: BTreeMap<&str, &AtlasNode> = view
        .nodes
        .iter()
        .map(|node| (node.path.as_str(), node))
        .collect();
    let path_steps: BTreeMap<&str, usize> = view
        .path
        .iter()
        .map(|step| (step.path.as_str(), step.step))
        .collect();

    out.push_str(&render_hulls(view, &layout.rows));

    let mut edge_paths = String::new();
    let mut edge_labels = String::new();
    for (index, edge) in ordered_edges(view, &layout.rows).iter().enumerate() {
        let Some(shape) = edge_shape(edge, layout) else {
            continue;
        };
        let (class, dasharray, opacity, marker) = if edge.back {
            (
                "atlas-edge atlas-edge--back",
                "4 3",
                "0.45",
                "url(#atlas-arrow-back)",
            )
        } else {
            (
                "atlas-edge atlas-edge--forward",
                "1",
                "0.34",
                "url(#atlas-arrow)",
            )
        };
        let path_length = if edge.back { "" } else { " pathLength=\"1\"" };
        let title = format!(
            "{} → {} · {}",
            edge.from,
            edge.to,
            format_distinct_edges(edge.count)
        );
        edge_paths.push_str(&format!(
            "<path class=\"{class}\" data-edge=\"{}\" d=\"{}\" fill=\"none\" stroke=\"#8d8577\" stroke-width=\"2\" \
             stroke-opacity=\"{opacity}\" stroke-dasharray=\"{dasharray}\" stroke-dashoffset=\"0\" marker-end=\"{marker}\"{path_length} \
             style=\"--atlas-edge:{index}\"><title>{}</title></path>",
            escape_xml(&format!("{}->{}", edge.from, edge.to)),
            shape.path,
            escape_xml(&title)
        ));
        // Visible `×N` pill only on the representative path; every edge keeps
        // its full distinct-edge count in the path `<title>` above (F7).
        if edge.on_path {
            edge_labels.push_str(&format!(
                "<g class=\"atlas-edge-label\"><title>{}</title>\
                 <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"12\" rx=\"3\" fill=\"#fffdf8\" \
                 stroke=\"#c2410c\" stroke-width=\"1\"/>\
                 <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"8.5\" \
                 fill=\"#c2410c\">{}</text></g>",
                escape_xml(&format_distinct_edges(edge.count)),
                shape.label_x - edge_label_width(edge.count) / 2,
                shape.label_y - 6,
                edge_label_width(edge.count),
                shape.label_x,
                shape.label_y + 3,
                escape_xml(&edge_label_text(edge.count))
            ));
        }
    }

    let mut traces = String::new();
    for (index, window) in view.path.windows(2).enumerate() {
        let from = &window[0].path;
        let to = &window[1].path;
        if let Some(edge) = view
            .edges
            .iter()
            .find(|edge| &edge.from == from && &edge.to == to)
            && let Some(shape) = edge_shape(edge, layout)
        {
            let step = index + 1;
            let title = format!("{PATH_CAPTION} · step {step} of {}", view.path.len());
            traces.push_str(&format!(
                "<path class=\"atlas-trace\" d=\"{}\" fill=\"none\" stroke=\"#c2410c\" stroke-width=\"2.3\" \
                 stroke-dasharray=\"1\" stroke-dashoffset=\"0\" pathLength=\"1\" \
                 marker-end=\"url(#atlas-arrow-path)\" \
                 style=\"--atlas-trace:{index}\"><title>{}</title></path>",
                shape.path,
                escape_xml(&title)
            ));
        }
    }

    out.push_str(&edge_paths);
    out.push_str(&traces);
    out.push_str(&render_nodes(view, layout, &node_map, &path_steps));
    out.push_str(&edge_labels);
    out
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Standalone static SVG graph final frame (no animation, no header/legend/
/// footer — those are HTML chrome in [`render_canvas_html`]).
#[must_use]
pub fn render_canvas_svg(view: &AtlasView) -> String {
    let layout = compute_layout(view, SVG_W);
    format!(
        "{}\n<style>{STATIC_CSS}</style>\n{}\n</svg>",
        svg_open(true, SVG_W, layout.height),
        render_graph_inner(view, &layout)
    )
}

/// View-level cost-scale tag for the first-screen units vocabulary:
/// all-v2 documents report `v2`, pre-I2 (all-v1) documents report `v1`, and a
/// document that mixes both scales reports `v1/v2`.
fn view_scale_tag(view: &AtlasView) -> &'static str {
    let has_v1 = view.nodes.iter().any(|node| node.scale == CostScale::V1);
    let has_v2 = view.nodes.iter().any(|node| node.scale == CostScale::V2);
    match (has_v1, has_v2) {
        (true, false) => "v1",
        (true, true) => "v1/v2",
        _ => "v2",
    }
}

/// Entry description for the key-metrics strip (policy, `path:line function`,
/// confidence, alternatives) — no repo name / root_path provenance.
fn strip_entry_line(view: &AtlasView) -> String {
    let confidence = {
        let trimmed = view.entry.confidence.trim();
        if trimmed.is_empty() {
            "UNKNOWN".to_string()
        } else {
            trimmed.to_ascii_uppercase()
        }
    };
    let noun = if view.entry.alternatives.len() == 1 {
        "alternative"
    } else {
        "alternatives"
    };
    let mut line = view.entry.policy.label().to_string();
    if let Some(point) = &view.entry.point {
        line.push_str(&format!(" · {}:{} {}", point.path, point.line, point.name));
    }
    line.push_str(&format!(
        " · confidence {confidence} · {} {noun}",
        format_count(view.entry.alternatives.len() as u64)
    ));
    line
}

/// The three inline KV1 action chips (visible `path:line — rule`, full title).
fn render_start_chips(actions: &[StartAction]) -> String {
    if actions.is_empty() {
        return "<span class=\"atlas-chip atlas-chip--stub\">no ranked action recorded</span>"
            .to_string();
    }
    actions
        .iter()
        .map(|action| {
            format!(
                "<a class=\"atlas-chip\" href=\"{}\" title=\"{}\"><span class=\"atlas-mono\">{}</span> — {}</a>",
                escape_html(&action.href),
                escape_html(&action.title()),
                escape_html(&action.location),
                escape_html(&action.rule),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// First-page chrome: masthead + key-metrics strip. It sits **before**
/// `#atlas-canvas` so the graph panel itself stays free of tables, cards and
/// metric panels (A9); the strip is the first answer-bearing block.
fn render_page_chrome(
    entry_line: &str,
    hottest_line: &str,
    path_line: &str,
    caps_line: &str,
    unit_line: &str,
    start_chips: &str,
) -> String {
    let mut out = String::new();
    out.push_str("<div class=\"atlas-page-chrome\">\n");
    out.push_str(&format!("<style>{ANIMATION_CSS}</style>\n"));
    out.push_str("<header class=\"atlas-masthead\">\n");
    out.push_str("<div class=\"atlas-brand\">\n");
    out.push_str("<div class=\"atlas-kicker\">PERFPAL · ATLAS</div>\n");
    out.push_str("<div class=\"atlas-wordmark\">Atlas</div>\n");
    out.push_str(
        "<div class=\"atlas-subline\">entry-flow map · static, profile to confirm</div>\n",
    );
    out.push_str("</div>\n");
    out.push_str("<div class=\"atlas-masthead-spacer\"></div>\n");
    out.push_str("<div class=\"atlas-masthead-actions\">\n");
    out.push_str("<input type=\"checkbox\" id=\"atlas-pause\" class=\"atlas-pause-input\" aria-label=\"Pause the animation\">\n");
    out.push_str("<label for=\"atlas-pause\" class=\"atlas-pause\">Pause</label>\n");
    out.push_str("<a class=\"atlas-explorer\" href=\"#explorer\">Explorer →</a>\n");
    out.push_str("</div>\n</header>\n");
    out.push_str("<div class=\"atlas-keymetrics\">\n");
    out.push_str("<div class=\"atlas-keymetrics-line\">\n");
    out.push_str(&format!(
        "<span><b>ENTRY</b> <span class=\"atlas-mono atlas-entry-value\">{}</span></span>\n",
        escape_html(entry_line)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-hot\">HOTTEST: <span class=\"atlas-mono\">{}</span></span>\n",
        escape_html(hottest_line)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-mono\">{}</span>\n",
        escape_html(path_line)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-mono\">{}</span>\n",
        escape_html(caps_line)
    ));
    out.push_str("</div>\n");
    out.push_str("<div class=\"atlas-keymetrics-line\">\n");
    out.push_str("<b>START</b>\n");
    out.push_str(start_chips);
    out.push_str("\n</div>\n");
    out.push_str("<div class=\"atlas-keymetrics-line\">\n");
    out.push_str("<span class=\"atlas-legend\">");
    for (class, label) in [
        ("atlas-sw-prod", "production"),
        ("atlas-sw-test", "test/bench"),
        ("atlas-sw-unreached", "unreached"),
        ("atlas-sw-forward", "forward"),
        ("atlas-sw-back", "back"),
        ("atlas-sw-path", "path"),
    ] {
        out.push_str(&format!(
            "<span><span class=\"atlas-sw {class}\"></span>{label}</span>"
        ));
    }
    out.push_str("</span>\n");
    out.push_str(&format!(
        "<span class=\"atlas-unit\">{}</span>\n",
        escape_html(unit_line)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-structural-caption\">{}</span>\n",
        escape_html(STRUCTURAL_CAPTION)
    ));
    out.push_str("</div>\n");
    out.push_str("</div>\n");
    out.push_str("</div>\n");
    out
}

/// HTML fragment for `report.html#map`: first-page chrome (masthead +
/// key-metrics strip), then the animated SVG canvas and its one footer. The
/// chrome carries exactly one scoped `<style>` and no script; the brief is a
/// sibling rendered after this fragment.
#[must_use]
pub fn render_canvas_html_with_actions(view: &AtlasView, start_actions: &[StartAction]) -> String {
    render_canvas_html_with_actions_and_rail(view, start_actions, "")
}

/// Same first-page shell with an optional per-file detail rail appended inside
/// the `atlas-stage` row, after the canvas section. The rail is empty in the
/// standalone/test wrappers.
#[must_use]
pub fn render_canvas_html_with_actions_and_rail(
    view: &AtlasView,
    start_actions: &[StartAction],
    rail_html: &str,
) -> String {
    let entry_line = strip_entry_line(view);
    let hottest_line = view
        .hottest_node
        .as_ref()
        .map(|node| {
            // Path, cost and the `× median` anchor (the `top N%` rank stays in
            // the node title / brief; the strip stays on one compact line).
            let mut parts = vec![node.path.clone(), format_cost(node.cost, node.scale)];
            if let Some(ratio) = node.ratio {
                parts.push(format!("{} median function", format_multiple(ratio)));
            }
            parts.join(" · ")
        })
        .unwrap_or_else(|| "no file".to_string());
    let path_line = format!(
        "PATH · {} steps · {PATH_CAPTION}",
        format_count(view.path.len() as u64)
    );
    let caps_line = format!(
        "{} of {} reachable files shown · {} aggregated",
        format_count(view.caps.reachable_shown as u64),
        format_count(view.caps.reachable_files as u64),
        format_count(view.caps.aggregated_files as u64)
    );

    let scale_tag = view_scale_tag(view);
    let scale_label = format!("static cost units ({scale_tag})");
    // Preserve the v1-fallback note whenever the view contains v1 rows (a
    // pre-I2 document is entirely fallback rows; a mixed one is partly).
    let unit_line = if scale_tag == "v2" {
        format!("{scale_label} — relative scale · × median function = all functions in the scan")
    } else {
        format!(
            "{scale_label} — relative scale · v1 fallback rows labelled · × median function = all functions in the scan"
        )
    };
    let start_chips = render_start_chips(start_actions);

    let reachable = format_reachable(
        view.reachability.reachable_files,
        view.reachability.source_files,
    );
    let pairs = format!(
        "{} of {} file pairs drawn",
        format_count(view.caps.edges_drawn as u64),
        format_count(view.caps.edge_pairs_total as u64)
    );
    let mix = format_edge_mix(view.edge_mix.medium, view.edge_mix.low, view.edge_mix.high);
    // F9: one unambiguous unreached label — production and non-production.
    let unreached = format!(
        "unreached production: {} · non-production: {}",
        format_count(view.coverage.unreached_production as u64),
        format_count(view.coverage.unreached_nonproduction as u64)
    );
    let footer_parts = [
        reachable,
        pairs,
        mix,
        unreached,
        format_parse(
            view.coverage.parse_ok,
            view.coverage.parse_partial,
            view.coverage.parse_failed,
        ),
        format_unresolved_includes(
            view.coverage.unresolved_includes,
            view.blast_radius.include_edges_total,
        ),
        format_demoted_files(view.coverage.demoted_files),
    ];
    // Each metric phrase stays on one line when the narrower canvas wraps the
    // footer to two lines (`33/238` must not separate from `unresolved includes`).
    let footer = footer_parts
        .iter()
        .map(|part| {
            format!(
                "<span class=\"atlas-footer-part\">{}</span>",
                escape_html(part)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");

    let layout = compute_layout(view, PAGE_GRAPH_W);
    let mut out = String::new();
    out.push_str(&render_page_chrome(
        &entry_line,
        &hottest_line,
        &path_line,
        &caps_line,
        &unit_line,
        &start_chips,
    ));
    out.push_str("<div class=\"atlas-stage\">\n");
    out.push_str("<section id=\"atlas-canvas\" class=\"atlas-canvas\">\n");
    out.push_str("<div class=\"atlas-graph\">\n");
    out.push_str(&svg_open(false, PAGE_GRAPH_W, layout.height));
    out.push('\n');
    out.push_str(&render_graph_inner(view, &layout));
    out.push_str("\n</svg>\n</div>\n");
    out.push_str(&format!(
        "<footer class=\"atlas-footer\">{footer}</footer>\n"
    ));
    out.push_str("</section>\n");
    if !rail_html.is_empty() {
        out.push_str(rail_html);
        if !rail_html.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("</div>\n");
    out
}

/// Standalone/test wrapper: the same shell with the named zero-action stub.
#[must_use]
pub fn render_canvas_html(view: &AtlasView) -> String {
    render_canvas_html_with_actions(view, &[])
}

/// Minimal standalone HTML document wrapping [`render_canvas_html`] in the
/// `#map` scope (so the scoped chrome/graph CSS applies); no scripts.
#[must_use]
pub fn render_canvas_page(view: &AtlasView) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Perfpal Atlas canvas</title>\n\
         <style>html,body{{margin:0;padding:0;background:#fff;color:#0f172a}}body{{padding:24px}}</style>\n\
         </head>\n<body>\n<section id=\"map\">\n{}\n</section>\n</body>\n</html>\n",
        render_canvas_html(view)
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::reporter::atlas::build_atlas_view;
    use crate::reporter::atlas::test_support::*;

    fn has_event_handler(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut index = 0;
        while index + 2 < bytes.len() {
            if bytes[index] == b'o' && bytes[index + 1] == b'n' {
                let previous_ok = index == 0
                    || matches!(
                        bytes[index - 1],
                        b' ' | b'\n' | b'\t' | b'\r' | b'<' | b'"' | b'\''
                    );
                let mut cursor = index + 2;
                while cursor < bytes.len() && bytes[cursor].is_ascii_lowercase() {
                    cursor += 1;
                }
                if previous_ok
                    && cursor > index + 2
                    && cursor < bytes.len()
                    && bytes[cursor] == b'='
                {
                    return true;
                }
            }
            index += 1;
        }
        false
    }

    fn contains_word(haystack: &str, needle: &str) -> bool {
        let haystack = haystack.to_ascii_lowercase();
        let bytes = haystack.as_bytes();
        let mut start = 0;
        while let Some(offset) = haystack[start..].find(needle) {
            let index = start + offset;
            let before = index.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(index + needle.len()).copied();
            let boundary =
                |byte: Option<u8>| byte.is_none_or(|value| !value.is_ascii_alphanumeric());
            if boundary(before) && boundary(after) {
                return true;
            }
            start = index + needle.len().max(1);
            if start >= haystack.len() {
                break;
            }
        }
        false
    }

    #[test]
    fn html_double_render_is_byte_identical() {
        let view = build_atlas_view(&application_fixture());
        assert_eq!(render_canvas_html(&view), render_canvas_html(&view));
    }

    #[test]
    fn svg_double_render_is_byte_identical() {
        let view = build_atlas_view(&application_fixture());
        assert_eq!(render_canvas_svg(&view), render_canvas_svg(&view));
    }

    #[test]
    fn canvas_adds_no_script_handler_or_network() {
        let view = build_atlas_view(&application_fixture());
        let artifacts = [
            render_canvas_html(&view),
            render_canvas_svg(&view),
            render_canvas_page(&view),
        ];
        for artifact in artifacts {
            assert!(!artifact.contains("<script"), "script tag emitted");
            assert!(!artifact.contains("href=\"http"), "network href emitted");
            assert!(!artifact.contains("src=\"http"), "network src emitted");
            assert!(!artifact.contains("url(http"), "network url emitted");
            assert!(!artifact.contains("@import"), "@import emitted");
            assert!(
                !has_event_handler(&artifact),
                "event handler attribute emitted"
            );
        }
    }

    #[test]
    fn canvas_has_no_metric_panels() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        for needle in [
            "<table",
            "<select",
            "<ul>",
            "<ul ",
            "<li>",
            "<li ",
            "class=\"card",
            "class=\"bar",
            "role=\"table",
        ] {
            assert!(
                !html.to_ascii_lowercase().contains(needle),
                "forbidden metric panel markup `{needle}`"
            );
        }
    }

    #[test]
    fn canvas_carries_entry_hottest_path_and_caps() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        assert!(html.contains("production main"));
        assert!(html.contains("confidence HIGH"));
        assert!(html.contains("0 alternatives"));
        assert!(html.contains("HOTTEST:"));
        assert!(html.contains("src/c.c"));
        assert!(html.contains("PATH · 3 steps"));
        assert!(html.contains("reachable files shown"));
        assert!(html.contains("aggregated"));
    }

    #[test]
    fn canvas_carries_units_anchors_and_structural_caption() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        assert!(html.contains(STRUCTURAL_CAPTION));
        assert!(html.contains("× median function"));
        assert!(html.contains("static cost units"));
        assert!(html.contains("MEDIUM /"));
        assert!(html.contains("LOW"));
    }

    #[test]
    fn canvas_renders_depth_columns_nodes_edges_hulls_and_path_badges() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        assert!(html.contains("depth 0 · ENTRY"));
        assert!(html.contains("depth 1"));
        assert!(html.contains("ENTRY"));
        assert!(html.contains("HOTTEST"));
        assert!(html.contains("approximate prefix group"));
        assert!(html.contains("atlas-hull"));
        assert!(html.contains("atlas-path-badge"));
        assert!(html.contains("atlas-edge--forward"));
        assert!(html.contains("atlas-edge--back"));
        assert!(html.contains("atlas-arrow-path"));
        assert!(html.contains("marker-end=\"url(#atlas-arrow-path)\""));
        assert!(html.contains(PATH_CAPTION));
    }

    #[test]
    fn back_edges_are_dashed_without_path_length_normalization() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        let start = html
            .find("class=\"atlas-edge atlas-edge--back\"")
            .expect("back edge markup");
        let end = start + html[start..].find("</path>").expect("edge close");
        let back = &html[start..end];
        assert!(
            back.contains("stroke-dasharray=\"4 3\""),
            "back edge must keep its visible dash pattern"
        );
        assert!(
            back.contains("stroke-opacity=\"0.45\""),
            "back edge must stay recessive (neutral, not amber)"
        );
        assert!(
            !back.contains("pathLength"),
            "pathLength=1 normalizes dash units and would render the back edge solid"
        );
    }

    #[test]
    fn canvas_renders_unreached_column_for_library() {
        let html = render_canvas_html(&build_atlas_view(&library_fixture()));
        assert!(html.contains("UNREACHED"));
        assert!(html.contains("no forward path"));
        assert!(html.contains("caller1.c"));
        assert!(html.contains("caller2.c"));
    }

    fn emitted_column_header_lines(html: &str) -> Vec<String> {
        const OPEN: &str = "<text class=\"atlas-column-header\"";
        let mut lines = Vec::new();
        let mut rest = html;
        while let Some(start) = rest.find(OPEN) {
            let after = &rest[start..];
            let content_start = after.find('>').expect("header text open") + 1;
            let content_end = after[content_start..]
                .find("</text>")
                .expect("header text close");
            lines.push(after[content_start..content_start + content_end].to_string());
            rest = &after[content_start + content_end + "</text>".len()..];
        }
        lines
    }

    #[test]
    fn every_column_header_line_fits_its_column_width() {
        for (name, doc) in [
            ("application", application_fixture()),
            ("library", library_fixture()),
            ("all-test", all_test_fixture()),
            ("capped", capped_fixture()),
            ("single-file", single_file_fixture()),
        ] {
            let view = build_atlas_view(&doc);
            let layout = compute_layout(&view, PAGE_GRAPH_W);
            let html = render_canvas_html(&view);
            let lines = emitted_column_header_lines(&html);
            assert_eq!(
                lines.len(),
                view.columns.len() * 2,
                "{name}: every column must emit exactly two header lines"
            );
            for (index, column_layout) in layout.columns.iter().enumerate() {
                for line in &lines[index * 2..index * 2 + 2] {
                    let estimated = estimated_header_width(line, HEADER_FONT_SIZE);
                    assert!(
                        estimated <= column_layout.width as f64,
                        "{name}: header line `{line}` is {estimated:.1}px wide but the column is {}px",
                        column_layout.width
                    );
                }
            }
        }
    }

    #[test]
    fn unreached_header_is_split_across_two_lines() {
        let view = build_atlas_view(&library_fixture());
        let unreached = view
            .columns
            .iter()
            .find(|column| column.unreached)
            .expect("library fixture has an unreached column");
        let (line1, line2) = column_header_lines(&view, unreached);
        assert_eq!(line1, "UNREACHED");
        assert_eq!(
            line2, "no forward path",
            "line 2 must carry only the unreached reason (no ambiguous `N shown`)"
        );
        assert!(
            !line2.contains("shown"),
            "the drawn-subset count belongs in the chip, not the header"
        );

        let html = render_canvas_html(&view);
        assert!(html.contains(">UNREACHED</text>"));
        assert!(html.contains(">no forward path"));
        assert!(
            !html.contains("UNREACHED · no forward path"),
            "the unreached header must not be emitted as one long line"
        );
    }

    #[test]
    fn canvas_renders_no_entry_and_empty_stubs() {
        let no_entry = render_canvas_html(&build_atlas_view(&no_entry_fixture()));
        assert!(no_entry.contains("NO ENTRY"));
        assert!(no_entry.contains("HOTTEST:"));
        assert!(no_entry.contains("src/handler.c"));
        assert!(no_entry.contains("PATH · 0 steps"));
        assert!(
            !no_entry.contains("data-node=\"src/handler.c\""),
            "no fake HOTTEST node in NO ENTRY"
        );
        assert!(!no_entry.contains("data-node="), "no fake node group");

        let empty = render_canvas_html(&build_atlas_view(&empty_fixture()));
        assert!(empty.contains("no source functions recorded"));
        assert!(empty.contains("HOTTEST:"));
        assert!(empty.contains("no file"));
        assert!(
            !empty.contains("data-node="),
            "no fake node group in empty state"
        );
        assert!(empty.contains("no include edges recorded"));
        assert!(!empty.contains("0/0"), "empty footer must not emit 0/0");
    }

    #[test]
    fn canvas_renders_capped_aggregate_and_chip() {
        let html = render_canvas_html(&build_atlas_view(&capped_fixture()));
        assert!(html.contains("aggregated"));
        assert!(html.contains("+5 more"));
        assert!(html.contains("+1 more"));
        assert!(html.contains("calls in"));
        assert!(html.contains("reachable files shown"));
    }

    #[test]
    fn canvas_folds_deep_depths() {
        let mut builder = DocBuilder::new().file(file("src/f0.c", "production", 1));
        let mut functions = vec![function("src/f0.c", "main", 1, Some(100), 0, Some("main"))];
        let mut edges = Vec::new();
        for index in 1..10_u64 {
            let path = format!("src/f{index}.c");
            builder = builder.file(file(&path, "production", 1));
            functions.push(function(
                &path,
                &format!("f{index}"),
                1,
                Some(100 - index),
                0,
                None,
            ));
            edges.push(call_edge(
                &format!("src/f{}.c#f{}#1:1", index - 1, index - 1),
                &format!("f{index}"),
                &format!("src/f{}.c", index - 1),
                Some(&path),
                Confidence::Low,
            ));
        }
        for function in functions {
            builder = builder.function(function);
        }
        for edge in edges {
            builder = builder.call_edge(edge);
        }
        let html = render_canvas_html(&build_atlas_view(&builder.build()));
        assert!(html.contains("depth 8+"));
    }

    #[test]
    fn canvas_has_pause_reduced_motion_and_print_rules() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        // CSS-only pause in the masthead, keyed through `#map:has(...)`.
        assert!(html.contains("#map:has(#atlas-pause:checked) #atlas-canvas .atlas-graph *"));
        assert!(html.contains("animation-play-state:paused"));
        assert!(html.contains("prefers-reduced-motion: reduce"));
        assert!(html.contains("@media print"));
        assert!(html.contains("break-inside:avoid"));
        assert!(html.contains(">Pause</label>"));
        assert!(html.contains("atlas-pause-input:focus-visible + .atlas-pause"));
        assert!(html.contains("#atlas-pause:checked + .atlas-pause"));
        assert!(html.contains("background:#ece8df"));
        assert!(html.contains("opacity:0!important"));
        assert!(html.contains("inline-size:1px!important"));
        assert!(html.contains("var(--atlas-col) * .12s"));
    }

    #[test]
    fn canvas_escapes_hostile_paths() {
        let hostile = "src/evil<&>\"'.c";
        let doc = DocBuilder::new()
            .file(file(hostile, "production", 1))
            .function(function(hostile, "main", 1, Some(100), 0, Some("main")))
            .build();
        let view = build_atlas_view(&doc);
        let svg = render_canvas_svg(&view);
        let html = render_canvas_html(&view);
        assert!(svg.contains("&lt;"));
        assert!(svg.contains("&amp;"));
        assert!(svg.contains("&gt;"));
        assert!(svg.contains("&quot;"));
        assert!(svg.contains("&#39;"));
        assert!(!svg.contains("evil<&>"));
        assert!(!html.contains("evil<&>"));
    }

    #[test]
    fn canvas_every_node_and_edge_has_title() {
        let view = build_atlas_view(&application_fixture());
        let html = render_canvas_html(&view);
        for node in &view.nodes {
            let marker = format!("data-node=\"{}\"", escape_xml(&node.path));
            let start = html
                .find(&marker)
                .unwrap_or_else(|| panic!("missing node marker for {}", node.path));
            let end = html[start..]
                .find("</g>")
                .unwrap_or_else(|| panic!("unterminated node group for {}", node.path));
            assert!(
                html[start..start + end].contains("<title>"),
                "node {} has no title",
                node.path
            );
        }
        for edge in &view.edges {
            let marker = format!(
                "data-edge=\"{}\"",
                escape_xml(&format!("{}->{}", edge.from, edge.to))
            );
            let start = html
                .find(&marker)
                .unwrap_or_else(|| panic!("missing edge marker for {}->{}", edge.from, edge.to));
            let end = html[start..]
                .find("</path>")
                .unwrap_or_else(|| panic!("unterminated edge path for {}->{}", edge.from, edge.to));
            assert!(
                html[start..start + end].contains("<title>"),
                "edge {}->{} has no title",
                edge.from,
                edge.to
            );
        }
    }

    #[test]
    fn static_svg_is_final_frame_without_animation() {
        let svg = render_canvas_svg(&build_atlas_view(&application_fixture()));
        assert!(!svg.contains("@keyframes"));
        assert!(!svg.contains("animation"));
        assert!(!svg.contains("<script"));
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("atlas-node"));
        assert!(svg.contains("atlas-edge"));
    }

    #[test]
    fn canvas_renders_all_states_same_shell() {
        let fixtures = [
            application_fixture(),
            library_fixture(),
            no_entry_fixture(),
            empty_fixture(),
            all_test_fixture(),
            capped_fixture(),
            partial_parse_fixture(),
            single_file_fixture(),
        ];
        for fixture in fixtures {
            let html = render_canvas_html(&build_atlas_view(&fixture));
            for needle in [
                "class=\"atlas-page-chrome\"",
                "class=\"atlas-masthead\"",
                "class=\"atlas-keymetrics\"",
                "class=\"atlas-legend\"",
                "id=\"atlas-canvas\"",
                "<svg",
                "class=\"atlas-footer\"",
            ] {
                assert!(html.contains(needle), "missing `{needle}`");
            }
            // The chrome is before the canvas; the graph panel is clean.
            let chrome = html.find("class=\"atlas-page-chrome\"").expect("chrome");
            let canvas = html.find("id=\"atlas-canvas\"").expect("canvas");
            assert!(chrome < canvas, "chrome must precede the canvas");
            let canvas_body = html
                .split("id=\"atlas-canvas\"")
                .nth(1)
                .expect("canvas body");
            for forbidden in [
                "class=\"atlas-masthead\"",
                "class=\"atlas-keymetrics\"",
                "class=\"atlas-legend\"",
                "class=\"atlas-head\"",
                "class=\"atlas-keys\"",
            ] {
                assert!(
                    !canvas_body.contains(forbidden),
                    "canvas must not carry chrome `{forbidden}`"
                );
            }
        }
    }

    #[test]
    fn partial_parse_tag_is_rendered() {
        let html = render_canvas_html(&build_atlas_view(&partial_parse_fixture()));
        let marker = "data-node=\"src/broken.c\"";
        let start = html.find(marker).expect("broken node");
        let end = html[start..].find("</g>").expect("node group");
        assert!(
            html[start..start + end].contains("partial"),
            "partial node badge missing"
        );
        assert!(html.contains("parse 1 ok / 1 partial / 0 failed"));
    }

    #[test]
    fn edges_are_unfilled_uniform_width_and_labelled() {
        let view = build_atlas_view(&application_fixture());
        let html = render_canvas_html(&view);
        for edge in &view.edges {
            let marker = format!(
                "data-edge=\"{}\"",
                escape_xml(&format!("{}->{}", edge.from, edge.to))
            );
            let marker_start = html
                .find(&marker)
                .unwrap_or_else(|| panic!("missing edge marker for {}->{}", edge.from, edge.to));
            let path_start = html[..marker_start].rfind("<path").expect("edge path open");
            let path_end = marker_start
                + html[marker_start..]
                    .find("</path>")
                    .expect("edge path close");
            let chunk = &html[path_start..path_end];
            assert!(chunk.contains("fill=\"none\""), "edge fill missing");
            assert!(
                chunk.contains("stroke-width=\"2\""),
                "edge width is not uniform"
            );
        }
        // M-10 visible label (not a bare count).
        assert!(html.contains(&format_distinct_edges(1)));
    }

    #[test]
    fn edge_labels_are_compact_pills_with_full_metric_title() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        assert!(
            html.contains("class=\"atlas-edge-label\""),
            "compact pill group missing"
        );
        assert!(
            html.contains(">×1</text>"),
            "compact visible edge label `×1` missing"
        );
        assert!(
            html.contains("font-size=\"8.5\" fill=\"#c2410c\""),
            "visible edge pill must use the 8.5px floor"
        );
        assert!(
            html.contains(&format!("<title>{}</title>", format_distinct_edges(1))),
            "full M-10 metric missing from the pill title"
        );
        assert!(
            html.contains(&format_distinct_edges(1)),
            "full M-10 metric must remain available"
        );
    }

    #[test]
    fn compact_edge_label_fits_the_inter_column_gap() {
        for count in 1..=12_usize {
            let text = edge_label_text(count);
            assert!(
                text.chars().count() <= 6,
                "compact edge label `{text}` longer than 6 chars"
            );
            let width = edge_label_width(count);
            assert!(
                width <= 30,
                "pill for {count} distinct edges is {width}px, wider than the ~30px gap"
            );
        }
    }

    #[test]
    fn edge_pills_never_overlap_node_rects() {
        for (name, doc) in [
            ("library", library_fixture()),
            ("application", application_fixture()),
            ("capped", capped_fixture()),
            ("partial-parse", partial_parse_fixture()),
        ] {
            let view = build_atlas_view(&doc);
            let layout = compute_layout(&view, PAGE_GRAPH_W);
            let mut pills = 0_usize;
            for edge in ordered_edges(&view, &layout.rows) {
                let Some(shape) = edge_shape(edge, &layout) else {
                    continue;
                };
                let width = edge_label_width(edge.count);
                let pill = NodeRect {
                    x: shape.label_x - width / 2,
                    y: shape.label_y - 6,
                    w: width,
                    h: 12,
                };
                for (path, rect) in &layout.rows {
                    let overlap = pill.x < rect.x + rect.w
                        && rect.x < pill.x + pill.w
                        && pill.y < rect.y + rect.h
                        && rect.y < pill.y + pill.h;
                    assert!(
                        !overlap,
                        "{name}: edge {} -> {} pill at ({}, {}) {}x{} overlaps node `{path}` \
                         rect ({}, {}) {}x{}",
                        edge.from,
                        edge.to,
                        pill.x,
                        pill.y,
                        pill.w,
                        pill.h,
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h
                    );
                }
                pills += 1;
            }
            assert!(pills > 0, "{name}: expected at least one edge pill");
        }
    }

    #[test]
    fn footer_always_prints_parse_coverage() {
        let all_ok = render_canvas_html(&build_atlas_view(&application_fixture()));
        assert!(all_ok.contains("parse "));
        assert!(all_ok.contains("unresolved includes"));
        assert!(all_ok.contains("files demoted"));

        let partial = render_canvas_html(&build_atlas_view(&partial_parse_fixture()));
        assert!(partial.contains("parse 1 ok / 1 partial / 0 failed"));
    }

    #[test]
    fn caps_chip_excludes_forced_unreached_hottest() {
        let mut builder = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .function(function("src/main.c", "main", 1, Some(10), 0, Some("main")));
        let mut edges = Vec::new();
        for index in 0..19_u64 {
            let path = format!("src/f{index}.c");
            builder = builder
                .file(file(&path, "production", 1))
                .function(function(
                    &path,
                    &format!("f{index}"),
                    1,
                    Some(100 - index),
                    0,
                    None,
                ));
            edges.push(call_edge(
                "src/main.c#main#1:1",
                &format!("f{index}"),
                "src/main.c",
                Some(&path),
                Confidence::Low,
            ));
        }
        builder = builder
            .file(file("src/hot.c", "production", 1))
            .function(function("src/hot.c", "hot", 1, Some(9000), 0, None));
        for edge in edges {
            builder = builder.call_edge(edge);
        }
        let view = build_atlas_view(&builder.build());
        assert_eq!(view.caps.reachable_files, 20);
        assert_eq!(view.caps.aggregated_files, 1);
        assert_eq!(view.caps.nodes_shown, 20);
        assert_eq!(view.caps.reachable_shown, 19);
        let html = render_canvas_html(&view);
        assert!(html.contains("19 of 20 reachable files shown · 1 aggregated"));
    }

    #[test]
    fn node_title_carries_file_total_anchor_and_full_path_label() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        assert!(html.contains("file total "));
        // Canvas label = file name; the full path stays in the node title.
        assert!(html.contains(">main.c</text>"));
        assert!(html.contains("<title>src/main.c · "));
    }

    #[test]
    fn duplicate_file_names_get_a_parent_segment_in_the_canvas_label() {
        let doc = DocBuilder::new()
            .file(file("src/a/mod.rs", "production", 1))
            .file(file("src/b/mod.rs", "production", 1))
            .function(function(
                "src/a/mod.rs",
                "a",
                1,
                Some(100),
                1,
                Some("lib-root"),
            ))
            .function(function("src/b/mod.rs", "b", 1, Some(50), 0, None))
            .build();
        let html = render_canvas_html(&build_atlas_view(&doc));
        assert!(
            html.contains(">a/mod.rs</text>"),
            "colliding basenames must add the parent segment"
        );
        assert!(
            html.contains(">b/mod.rs</text>"),
            "both colliding nodes must be distinguishable"
        );
        assert!(
            !html.contains(">mod.rs</text>"),
            "no bare duplicate basename may remain on the canvas"
        );
    }

    #[test]
    fn rail_role_colour_rules_keep_entry_and_unreached_above_production() {
        let production = ANIMATION_CSS
            .find(".is-production .atlas-rail-role")
            .expect("production role rule");
        let entry = ANIMATION_CSS
            .find(".is-entry .atlas-rail-role")
            .expect("entry role rule");
        let unreached = ANIMATION_CSS
            .find(".is-unreached .atlas-rail-role")
            .expect("unreached role rule");
        assert!(
            production < entry && production < unreached,
            "the production rule must stay before the kind-specific role rules"
        );
    }

    #[test]
    fn legend_lists_every_unit_and_style_key() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        // Exactly one swatch legend, inside the strip (not the canvas).
        assert_eq!(html.matches("class=\"atlas-legend\"").count(), 1);
        for item in [
            "atlas-sw-prod",
            "atlas-sw-test",
            "atlas-sw-unreached",
            "atlas-sw-forward",
            "atlas-sw-back",
            "atlas-sw-path",
            "production",
            "test/bench",
            "unreached",
            "forward",
            "back",
            "path",
        ] {
            assert!(html.contains(item), "missing legend item `{item}`");
        }
        let legend_start = html.find("class=\"atlas-legend\"").expect("legend");
        let legend = &html[legend_start..];
        assert!(
            legend.contains("static cost units (v2) — relative scale")
                && legend.contains("× median function = all functions in the scan"),
            "the strip must carry the one unit definition"
        );
        assert!(legend.contains(STRUCTURAL_CAPTION));
    }

    #[test]
    fn hot_finding_renders_as_text_annotation() {
        let doc = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .function(function(
                "src/main.c",
                "main",
                1,
                Some(100),
                0,
                Some("main"),
            ))
            .finding(finding(
                "PERF-C-ALLOC-001",
                "src/main.c",
                1,
                Severity::Hot,
                "hot allocation",
            ))
            .build();
        let html = render_canvas_html(&build_atlas_view(&doc));
        assert!(html.contains("HOT ×1"));
    }

    #[test]
    fn emitted_surfaces_never_claim_time_or_runtime() {
        let view = build_atlas_view(&application_fixture());
        let artifacts = [
            render_canvas_html(&view),
            render_canvas_svg(&view),
            render_canvas_page(&view),
        ];
        for artifact in artifacts {
            for token in ["ms", "µs", "us"] {
                assert!(
                    !contains_word(&artifact, token),
                    "forbidden unit token `{token}` emitted"
                );
            }
            let lowered = artifact.to_ascii_lowercase();
            for token in ["runtime", "speedup", "latency", "throughput", "hot path"] {
                assert!(
                    !lowered.contains(token),
                    "forbidden runtime claim `{token}` emitted"
                );
            }
        }
    }

    #[test]
    fn nonproduction_unreached_renders_as_a_chip_not_a_node() {
        let doc = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .file(file("src/a.c", "production", 1))
            .file(file("tests/t.c", "test", 1))
            .function(function(
                "src/main.c",
                "main",
                1,
                Some(100),
                0,
                Some("main"),
            ))
            .function(function("src/a.c", "a", 1, Some(50), 0, None))
            .function(function("tests/t.c", "test_fn", 1, Some(40), 0, None))
            .call_edge(call_edge(
                "src/main.c#main#1:1",
                "a",
                "src/main.c",
                Some("src/a.c"),
                Confidence::Low,
            ))
            .build();
        let view = build_atlas_view(&doc);
        assert_eq!(
            view.columns
                .last()
                .map(|column| column.nonproduction_unreached),
            Some(1)
        );
        let html = render_canvas_html(&view);
        assert!(html.contains("UNREACHED"));
        assert!(html.contains("non-production unreached: 1"));
        assert!(!html.contains("+1 non-production unreached"));
        assert!(html.contains("data-prod-unreached=\"0\""));
        assert!(html.contains("data-nonprod-unreached=\"1\""));
        assert!(html.contains(
            "0 production unreached · 1 non-production unreached · no estimated reachability"
        ));
        assert!(
            !html.contains("data-node=\"tests/t.c\""),
            "non-production unreached must not be a drawn node"
        );
    }

    #[test]
    fn unreached_chip_reports_production_and_nonproduction_counts() {
        let mut builder = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .function(function(
                "src/main.c",
                "main",
                1,
                Some(9000),
                0,
                Some("main"),
            ));
        let mut edges = Vec::new();
        for index in 0..19_u64 {
            let path = format!("src/f{index}.c");
            builder = builder
                .file(file(&path, "production", 1))
                .function(function(
                    &path,
                    &format!("f{index}"),
                    1,
                    Some(100 - index),
                    0,
                    None,
                ));
            edges.push(call_edge(
                "src/main.c#main#1:1",
                &format!("f{index}"),
                "src/main.c",
                Some(&path),
                Confidence::Low,
            ));
        }
        builder = builder
            .file(file("src/cold.c", "production", 1))
            .function(function("src/cold.c", "cold", 1, Some(10), 0, None))
            .file(file("tests/t.c", "test", 1))
            .function(function("tests/t.c", "test_fn", 1, Some(40), 0, None));
        for edge in edges {
            builder = builder.call_edge(edge);
        }
        let view = build_atlas_view(&builder.build());
        assert_eq!(view.coverage.unreached_production, 1);
        assert_eq!(view.coverage.unreached_nonproduction, 1);
        let html = render_canvas_html(&view);
        assert!(html.contains("data-prod-unreached=\"1\""));
        assert!(html.contains("data-nonprod-unreached=\"1\""));
        assert!(html.contains("production unreached: 1"));
        assert!(html.contains("non-production unreached: 1"));
        assert!(!html.contains("+1 production unreached"));
        assert!(!html.contains("+1 non-production unreached"));
        assert!(html.contains(
            "1 production unreached · 1 non-production unreached · no estimated reachability"
        ));
    }

    #[test]
    fn deep_mandatory_path_grows_canvas_height() {
        let mut builder = DocBuilder::new().file(file("src/f0.c", "production", 1));
        let mut functions = vec![function("src/f0.c", "main", 1, Some(100), 0, Some("main"))];
        let mut edges = Vec::new();
        for index in 1..40_u64 {
            let path = format!("src/f{index}.c");
            builder = builder.file(file(&path, "production", 1));
            functions.push(function(
                &path,
                &format!("f{index}"),
                1,
                Some(100 - index),
                0,
                None,
            ));
            edges.push(call_edge(
                &format!("src/f{}.c#f{}#1:1", index - 1, index - 1),
                &format!("f{index}"),
                &format!("src/f{}.c", index - 1),
                Some(&path),
                Confidence::Low,
            ));
        }
        for function in functions {
            builder = builder.function(function);
        }
        for edge in edges {
            builder = builder.call_edge(edge);
        }
        let view = build_atlas_view(&builder.build());
        assert_eq!(view.nodes.len(), 40);
        let html = render_canvas_html(&view);
        // 32 rows in the folded depth column at the 58px floor (46px node +
        // 12px gap): 72 + 32*58 + 16. The page canvas is the narrower
        // `PAGE_GRAPH_W`; the standalone artifact keeps `SVG_W`.
        assert!(html.contains("viewBox=\"0 0 868 1944\""));
        assert_eq!(html.matches("data-node=\"").count(), 40);
        let svg = render_canvas_svg(&view);
        assert!(svg.contains("viewBox=\"0 0 1400 1944\""));
    }

    #[test]
    fn canvas_header_carries_units_denominator_and_structural_caption() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        let strip_start = html
            .find("class=\"atlas-keymetrics\"")
            .expect("key-metrics strip");
        let canvas = html.find("id=\"atlas-canvas\"").expect("canvas");
        assert!(strip_start < canvas, "strip must precede the canvas");
        let strip = &html[strip_start..canvas];
        assert!(strip.contains("static cost units (v2)"));
        assert!(strip.contains("× median function = all functions in the scan"));
        assert!(strip.contains(STRUCTURAL_CAPTION));
    }

    #[test]
    fn canvas_keys_line_explains_units_edges_and_path_badges_first_screen() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        let masthead_end = html.find("</header>").expect("masthead close");
        let strip_start = html
            .find("class=\"atlas-keymetrics\"")
            .expect("key-metrics strip");
        let canvas_start = html.find("id=\"atlas-canvas\"").expect("atlas canvas");
        assert!(
            masthead_end < strip_start && strip_start < canvas_start,
            "the key-metrics strip must sit between the masthead and the canvas"
        );
        let strip = &html[strip_start..canvas_start];
        for needle in [
            "static cost units (v2) — relative scale",
            "× median function = all functions in the scan",
            STRUCTURAL_CAPTION,
        ] {
            assert!(strip.contains(needle), "key strip is missing `{needle}`");
        }
        // Action chips are inline `<a>` elements, not a list/table/card grid.
        assert!(strip.contains("atlas-chip"));
        for forbidden in ["<table", "<ul", "<li", "<select", "class=\"card"] {
            assert!(
                !strip.contains(forbidden),
                "key strip must not contain `{forbidden}`"
            );
        }
        // One unit definition and one structural caveat for the whole page.
        assert_eq!(
            html.matches("× median function = all functions in the scan")
                .count(),
            1
        );
        assert_eq!(html.matches(STRUCTURAL_CAPTION).count(), 1);
    }

    #[test]
    fn canvas_keys_line_reports_the_v1_scale_for_pre_i2_documents() {
        let html = render_canvas_html(&build_atlas_view(&v1_only_fixture()));
        let unit_start = html
            .find("class=\"atlas-unit\"")
            .expect("key strip unit span");
        let unit_end = unit_start + html[unit_start..].find("</span>").expect("unit close");
        let unit = &html[unit_start..unit_end];
        assert!(
            unit.contains("static cost units (v1) — relative scale"),
            "v1-only documents must not claim v2 units: {unit}"
        );
        assert!(
            unit.contains("v1 fallback rows labelled"),
            "v1-only documents must carry the fallback note: {unit}"
        );
        assert!(!unit.contains("static cost units (v2)"));
    }

    #[test]
    fn compact_edge_label_clamps_counts_above_999() {
        assert_eq!(edge_label_text(1000), "×999+");
        assert_eq!(edge_label_text(9999), "×999+");
        for count in [1000_usize, 9999] {
            assert!(
                edge_label_text(count).chars().count() <= 5,
                "clamped edge label must stay within five visible characters"
            );
            assert!(
                edge_label_width(count) <= 32,
                "clamped edge pill for {count} is {}px, wider than the 32px bound",
                edge_label_width(count)
            );
        }
        // The full count stays available in the pill title.
        assert!(format_distinct_edges(9999).contains("9,999"));
    }

    #[test]
    fn hull_label_sits_at_least_ten_px_above_first_node_badge() {
        let view = build_atlas_view(&application_fixture());
        let layout = compute_layout(&view, PAGE_GRAPH_W);
        assert!(
            layout.row_h - layout.node_h >= ROW_GAP,
            "layout must always leave a full inter-row badge gap"
        );
        let mut groups: BTreeMap<&str, Vec<&AtlasNode>> = BTreeMap::new();
        for node in &view.nodes {
            groups
                .entry(node.component.as_str())
                .or_default()
                .push(node);
        }
        let mut checked = 0_usize;
        for members in groups.values().filter(|members| members.len() >= 2) {
            let placed: Vec<(&AtlasNode, NodeRect)> = members
                .iter()
                .filter_map(|node| layout.rows.get(&node.path).map(|rect| (*node, *rect)))
                .collect();
            let Some(min_y) = placed.iter().map(|(_, rect)| rect.y).min() else {
                continue;
            };
            let (first_node, first_rect) = placed
                .iter()
                .find(|(_, rect)| rect.y == min_y)
                .expect("first hull row");
            assert!(
                !node_badges(first_node).is_empty(),
                "first hull node must carry a badge for the clearance check"
            );
            let label_baseline = hull_label_baseline(min_y);
            let badge_baseline = node_badge_baseline(*first_rect);
            assert!(
                label_baseline > HEADER_Y + 10,
                "hull label baseline {label_baseline} must clear column header {HEADER_Y}"
            );
            assert!(
                badge_baseline - label_baseline >= 10,
                "hull label baseline {label_baseline} is only {}px above badge \
                 baseline {badge_baseline}",
                badge_baseline - label_baseline
            );
            checked += 1;
        }
        assert!(checked > 0, "application fixture has no multi-node hull");
    }

    #[test]
    fn node_name_and_meta_font_sizes_meet_legibility_floor() {
        let fixtures = [
            application_fixture(),
            library_fixture(),
            capped_fixture(),
            partial_parse_fixture(),
            single_file_fixture(),
        ];
        for fixture in fixtures {
            let html = render_canvas_html(&build_atlas_view(&fixture));
            let mut names = 0_usize;
            let mut metas = 0_usize;
            for chunk in html.split("class=\"atlas-node-name\"").skip(1) {
                let tag = &chunk[..chunk.find('>').expect("name tag close")];
                let size = font_size_attr(tag);
                assert!(size >= 9.0, "node name font-size {size} below 9px");
                names += 1;
            }
            for chunk in html.split("class=\"atlas-node-meta").skip(1) {
                let tag = &chunk[..chunk.find('>').expect("meta tag close")];
                let size = font_size_attr(tag);
                assert!(size >= 8.0, "node meta font-size {size} below 8px");
                metas += 1;
            }
            assert!(names > 0, "no node name text emitted");
            assert!(metas > 0, "no node meta text emitted");
        }
    }

    #[test]
    fn capped_state_node_height_stays_readable() {
        let svg = render_canvas_svg(&build_atlas_view(&capped_fixture()));
        let mut nodes = 0_usize;
        for chunk in svg.split("data-node=\"").skip(1) {
            let start = chunk.find("<rect ").expect("node rect");
            let rect = &chunk[start..];
            let rect = &rect[..rect.find("/>").expect("rect close")];
            let height = rect
                .split("height=\"")
                .nth(1)
                .expect("rect height")
                .split('"')
                .next()
                .expect("height close")
                .parse::<i64>()
                .expect("height integer");
            assert!(height >= 46, "capped node height {height} below 46px");
            nodes += 1;
        }
        assert!(nodes > 0, "capped state emitted no nodes");
    }

    #[test]
    fn node_label_is_the_file_name_and_title_keeps_the_full_path() {
        let long = "src/very/deep/module/statistics.rs";
        // The helper still keeps mid-ellipsis semantics for headers/rail paths.
        let shortened = mid_ellipsis(long, 20);
        assert!(
            shortened.starts_with("src/"),
            "truncated path must keep its first segment: {shortened}"
        );
        assert!(
            !shortened.starts_with('…') && !shortened.starts_with(".."),
            "leading-dot truncation is forbidden: {shortened}"
        );
        assert!(shortened.contains('…'));
        assert!(shortened.chars().count() <= 20);
        assert_eq!(mid_ellipsis("src/a.c", 20), "src/a.c");

        let doc = DocBuilder::new()
            .file(file(long, "production", 1))
            .function(function(long, "main", 1, Some(100), 0, Some("main")))
            .build();
        let html = render_canvas_html(&build_atlas_view(&doc));
        let name_start = html
            .find("class=\"atlas-node-name\"")
            .expect("node name text");
        let text_start = name_start + html[name_start..].find('>').expect("name open") + 1;
        let text_end = text_start + html[text_start..].find("</text>").expect("name close");
        let label = &html[text_start..text_end];
        assert_eq!(
            label, "statistics.rs",
            "the canvas label is the file name (spec §3.2), not the path"
        );
        assert!(
            !label.starts_with('…') && !label.starts_with(".."),
            "leading-dot truncation is forbidden: {label}"
        );
        assert!(
            html.contains(&format!("<title>{long} · ")),
            "the full path must stay in the node title"
        );
    }

    #[test]
    fn edge_labels_render_only_on_path_and_every_edge_has_title() {
        let view = build_atlas_view(&application_fixture());
        let html = render_canvas_html(&view);
        let on_path = view.edges.iter().filter(|edge| edge.on_path).count();
        assert!(on_path > 0, "fixture must carry a representative path");
        assert!(
            on_path < view.edges.len(),
            "fixture must carry a non-path edge for the path-only assertion"
        );
        assert_eq!(
            html.matches("class=\"atlas-edge-label\"").count(),
            on_path,
            "only on-path edges may render a visible ×N pill"
        );
        for edge in &view.edges {
            let marker = format!(
                "data-edge=\"{}\"",
                escape_xml(&format!("{}->{}", edge.from, edge.to))
            );
            let start = html
                .find(&marker)
                .unwrap_or_else(|| panic!("missing edge {marker}"));
            let end = start + html[start..].find("</path>").expect("edge close");
            assert!(
                html[start..end].contains("<title>"),
                "every edge keeps its full-count title: {} -> {}",
                edge.from,
                edge.to
            );
        }
    }

    #[test]
    fn footer_reports_explicit_unreached_production_and_nonproduction() {
        let html = render_canvas_html(&build_atlas_view(&library_fixture()));
        assert!(
            html.contains("unreached production:"),
            "the footer must name unreached production explicitly"
        );
        assert!(
            html.contains("non-production:"),
            "the footer must name unreached non-production explicitly"
        );
        assert!(
            !html.contains("unreached production ·"),
            "the ambiguous footer wording must be gone"
        );
        assert_eq!(html.matches("class=\"atlas-footer\"").count(), 1);
    }

    #[test]
    fn standalone_canvas_page_is_map_scoped() {
        let page = render_canvas_page(&build_atlas_view(&application_fixture()));
        assert!(page.contains("<section id=\"map\">"));
        assert!(page.contains("class=\"atlas-masthead\""));
        assert!(page.contains("class=\"atlas-graph\""));
    }

    fn font_size_attr(tag: &str) -> f64 {
        tag.split("font-size=\"")
            .nth(1)
            .expect("font-size attribute")
            .split('"')
            .next()
            .expect("font-size close")
            .parse::<f64>()
            .expect("font-size number")
    }
}
