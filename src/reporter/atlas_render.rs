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
use crate::reporter::atlas_metrics::{
    NO_ENTRY, NO_SOURCE_FUNCTIONS, format_cost, format_count, format_demoted_files, format_depth,
    format_distinct_edges, format_edge_mix, format_entry, format_finding, format_multiple,
    format_parse, format_reachable, format_top, format_unresolved_includes,
};
use crate::reporter::escape_html;
use crate::reporter::views::CostScale;

/// Mandatory disclosure for the default-tier structural call graph.
pub const STRUCTURAL_CAPTION: &str =
    "structural (LOW name-based + MEDIUM semantic), may over/under-approximate";
/// Mandatory label for the representative path trace.
pub const PATH_CAPTION: &str = "representative path (static, BFS-derived)";

const SVG_W: i64 = 1400;
const SVG_H: i64 = 720;
const MARGIN_X: i64 = 20;
const TOP: i64 = 52;
const BOTTOM: i64 = 16;
const HEADER_Y: i64 = 22;
const MAX_COL_W: i64 = 180;
const MIN_NODE_W: i64 = 96;
const MAX_NODE_W: i64 = 150;

const HULL_COLORS: [&str; 8] = [
    "#eef2ff", "#f0fdf4", "#fef3c7", "#fce7f3", "#e0f2fe", "#f5f3ff", "#ecfccb", "#ffe4e6",
];

const STATIC_CSS: &str = r#".atlas-svg{background:#ffffff}
.atlas-svg text{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.atlas-edge,.atlas-trace{fill:none}"#;

const ANIMATION_CSS: &str = r#"#atlas-canvas{position:relative;background:#ffffff;color:#0f172a;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
#atlas-canvas .atlas-head{display:flex;flex-wrap:wrap;gap:6px 20px;align-items:baseline;padding:10px 16px;border-bottom:1px solid #e2e8f0;font-size:12px}
#atlas-canvas .atlas-entry{font-weight:700}
#atlas-canvas .atlas-hottest{color:#9a3412}
#atlas-canvas .atlas-caps{color:#475569}
#atlas-canvas .atlas-pause-input{position:absolute;inline-size:1px;block-size:1px;opacity:0}
#atlas-canvas .atlas-pause{display:inline-block;margin:0 16px 8px;padding:3px 10px;border:1px solid #94a3b8;border-radius:999px;background:#f8fafc;font-size:11px;cursor:pointer}
#atlas-canvas .atlas-graph{background:#ffffff;border-bottom:1px solid #e2e8f0}
#atlas-canvas .atlas-svg{display:block;width:100%;height:auto}
#atlas-canvas .atlas-svg .atlas-edge,#atlas-canvas .atlas-svg .atlas-trace{fill:none}
#atlas-canvas .atlas-legend{display:flex;flex-wrap:wrap;gap:4px 14px;padding:8px 16px;border-bottom:1px solid #e2e8f0;font-size:11px;color:#334155}
#atlas-canvas .atlas-structural-caption{font-weight:700;color:#7c2d12}
#atlas-canvas .atlas-footer{padding:8px 16px;font-size:11px;color:#334155}
@keyframes atlas-entry-pulse{0%{opacity:.45}100%{opacity:1}}
@keyframes atlas-column-reveal{0%{opacity:0;transform:translateY(4px)}100%{opacity:1;transform:none}}
 @keyframes atlas-edge-draw{from{stroke-dashoffset:1}to{stroke-dashoffset:0}}
 @keyframes atlas-edge-fade{from{opacity:0}to{opacity:1}}
 @keyframes atlas-trace-draw{from{stroke-dashoffset:1}to{stroke-dashoffset:0}}
 @keyframes atlas-hottest-pulse{0%,100%{opacity:1}50%{opacity:.5}}
 #atlas-canvas .atlas-column{animation:atlas-column-reveal .4s calc(1.2s + var(--atlas-col) * .12s) both}
 #atlas-canvas .atlas-node.is-entry{animation:atlas-entry-pulse 1.2s ease-out both}
 #atlas-canvas .atlas-edge--forward{animation:atlas-edge-draw 1.4s calc(2.8s + var(--atlas-edge,0) * .01s) both}
 #atlas-canvas .atlas-edge--back{animation:atlas-edge-fade .4s calc(2.8s + var(--atlas-edge,0) * .01s) both}
 #atlas-canvas .atlas-trace{animation:atlas-trace-draw 1.2s 4.2s both}
#atlas-canvas .atlas-node.is-hottest{animation:atlas-hottest-pulse .8s 5.4s 2 both}
#atlas-canvas .atlas-pause-input:checked ~ .atlas-graph *{animation-play-state:paused !important}
#atlas-canvas .atlas-pause:focus-visible{outline:2px solid #1d4ed8;outline-offset:2px}
#atlas-canvas .atlas-pause-input:focus-visible + .atlas-pause{outline:2px solid #1d4ed8;outline-offset:2px}
@media (prefers-reduced-motion: reduce){#atlas-canvas *{animation:none !important;opacity:1 !important;transform:none !important;stroke-dashoffset:0 !important}}
@media print{#atlas-canvas{break-inside:avoid}#atlas-canvas *{animation:none !important;opacity:1 !important;transform:none !important;stroke-dashoffset:0 !important}#atlas-canvas .atlas-pause{display:none}}
#atlas-canvas .atlas-pause-input{opacity:0!important;inline-size:1px!important;block-size:1px!important}"#;

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
    row_h: i64,
    node_h: i64,
    height: i64,
    columns: Vec<ColumnLayout>,
    rows: BTreeMap<String, NodeRect>,
}

fn compute_layout(view: &AtlasView) -> Layout {
    let max_items = view
        .columns
        .iter()
        .map(|column| {
            column.files.len()
                + column.aggregated.len()
                + usize::from(column.unreached && column.nonproduction_unreached > 0)
        })
        .max()
        .unwrap_or(1)
        .max(1);
    let available_h = SVG_H - TOP - BOTTOM;
    let row_h = (available_h / max_items as i64).clamp(26, 88);
    let node_h = (row_h - 8).clamp(18, 80);
    let height = SVG_H.max(TOP + max_items as i64 * row_h + BOTTOM);

    let count = view.columns.len();
    let mut columns = Vec::with_capacity(count);
    if count > 0 {
        let column_w = ((SVG_W - 2 * MARGIN_X) / count as i64).min(MAX_COL_W);
        let grid_w = column_w * count as i64;
        let start_x = (SVG_W - grid_w) / 2;
        for index in 0..count {
            let x = start_x + index as i64 * column_w;
            let node_w = (column_w - 16).clamp(MIN_NODE_W, MAX_NODE_W);
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

fn edge_shape(edge: &AtlasEdge, rows: &BTreeMap<String, NodeRect>) -> Option<EdgeShape> {
    let from = rows.get(&edge.from)?;
    let to = rows.get(&edge.to)?;
    let source_x = from.x + from.w;
    let source_y = from.y + from.h / 2;
    let (target_x, target_y, control_one_x, control_two_x) = if edge.back {
        let target_x = to.x + to.w;
        let target_y = to.y + to.h / 2;
        let bulge = (28 + (edge.count.min(8) as i64) * 4).min(72);
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
    let label_x = (source_x + 3 * control_one_x + 3 * control_two_x + target_x) / 8;
    let label_y = (source_y + target_y) / 2;
    Some(EdgeShape {
        path,
        label_x,
        label_y,
    })
}

// ---------------------------------------------------------------------------
// Small text helpers
// ---------------------------------------------------------------------------

fn tail_keep(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 1 {
        return text.chars().take(max_chars).collect();
    }
    let skip = count - (max_chars - 1);
    let mut out = String::from("…");
    out.extend(text.chars().skip(skip));
    out
}

fn files_label(count: usize) -> String {
    format!(
        "{} {}",
        format_count(count as u64),
        if count == 1 { "file" } else { "files" }
    )
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

fn column_header(view: &AtlasView, column: &AtlasColumn) -> String {
    let count = files_label(column.file_count);
    if column.unreached {
        format!("UNREACHED · no forward path · {count}")
    } else if column.folded {
        format!("depth {DEPTH_COLUMN_CAP}+ · {count}")
    } else if column.depth == 0 && view.entry.point.is_some() {
        format!("{} · ENTRY · {count}", format_depth(0))
    } else {
        format!("{} · {count}", format_depth(column.depth))
    }
}

// ---------------------------------------------------------------------------
// SVG fragments
// ---------------------------------------------------------------------------

fn svg_open(standalone: bool, height: i64) -> String {
    let xmlns = if standalone {
        " xmlns=\"http://www.w3.org/2000/svg\""
    } else {
        ""
    };
    format!(
        "<svg{xmlns} viewBox=\"0 0 {SVG_W} {height}\" width=\"{SVG_W}\" height=\"{height}\" \
         class=\"atlas-svg\" role=\"img\" aria-label=\"Perfpal Atlas entry-flow canvas\" \
         preserveAspectRatio=\"xMidYMid meet\">"
    )
}

fn render_defs() -> String {
    concat!(
        "<defs>",
        "<pattern id=\"atlas-hatch\" width=\"6\" height=\"6\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate(45)\">",
        "<rect width=\"6\" height=\"6\" fill=\"#f8fafc\"/>",
        "<line x1=\"0\" y1=\"0\" x2=\"0\" y2=\"6\" stroke=\"#cbd5e1\" stroke-width=\"2\"/>",
        "</pattern>",
        "<marker id=\"atlas-arrow\" viewBox=\"0 0 8 8\" refX=\"7\" refY=\"4\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">",
        "<path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#6b7280\"/>",
        "</marker>",
        "<marker id=\"atlas-arrow-back\" viewBox=\"0 0 8 8\" refX=\"7\" refY=\"4\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">",
        "<path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#b45309\"/>",
        "</marker>",
        "</defs>",
    )
    .to_string()
}

fn render_stub(view: &AtlasView) -> String {
    let (message, detail) = if view.state.empty {
        (NO_SOURCE_FUNCTIONS.to_string(), String::new())
    } else {
        (
            NO_ENTRY.to_ascii_uppercase(),
            "no electable production entry".to_string(),
        )
    };
    let mut out = String::from("<g class=\"atlas-stub\" role=\"note\">");
    out.push_str(
        "<rect x=\"450\" y=\"300\" width=\"500\" height=\"120\" rx=\"12\" fill=\"#f8fafc\" \
         stroke=\"#94a3b8\" stroke-dasharray=\"6 4\"/>",
    );
    out.push_str(&format!(
        "<text x=\"700\" y=\"352\" text-anchor=\"middle\" font-size=\"18\" font-weight=\"700\" \
         fill=\"#334155\">{}</text>",
        escape_xml(&message)
    ));
    if !detail.is_empty() {
        out.push_str(&format!(
            "<text x=\"700\" y=\"376\" text-anchor=\"middle\" font-size=\"11\" fill=\"#64748b\">{}</text>",
            escape_xml(&detail)
        ));
    }
    out.push_str("</g>");
    out
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
    let mut palette_index = 0_usize;
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
        let y = (min_y - 12).max(0);
        let width = (max_x + 8 - x).max(20);
        let height = (max_y + 8 - y).max(20);
        let color = HULL_COLORS[palette_index % HULL_COLORS.len()];
        palette_index += 1;
        out.push_str(&format!(
            "<g class=\"atlas-hull\"><title>approximate prefix group; not a canonical component id</title>\
             <rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"10\" fill=\"{color}\" \
             fill-opacity=\".55\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>\
             <text x=\"{}\" y=\"{}\" font-size=\"7\" font-weight=\"700\" fill=\"#475569\">{}</text></g>",
            x + 8,
            y + 12,
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
         <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" fill=\"#f1f5f9\" stroke=\"#94a3b8\" \
         stroke-dasharray=\"5 4\"/>\
         <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"6.2\" font-weight=\"700\" fill=\"#334155\">+{} more</text>\
         <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"5.6\" fill=\"#475569\">{} calls in</text></g>",
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
) -> String {
    let max_chars = (rect.w / 5).clamp(12, 32) as usize;
    let name = tail_keep(&node.path, max_chars);
    let rank = node_rank_text(node);
    let (name_size, meta_size) = if rect.h >= 60 {
        (8.0, 6.5)
    } else if rect.h >= 40 {
        (7.0, 6.0)
    } else {
        (5.8, 5.2)
    };
    let center = rect.y + rect.h / 2;
    let gap = (rect.h / 3).clamp(6, 14);
    let name_y = center - gap;
    let cost_y = center;
    let rank_y = center + gap;
    let text_fill = if node.entry { "#ffffff" } else { "#1e293b" };
    let (fill, mut stroke, mut stroke_width) = if node.entry {
        ("#1f2937", "#111827", 2)
    } else if node.unreached {
        ("#d1d5db", "#6b7280", 1)
    } else if !node.production {
        ("url(#atlas-hatch)", "#64748b", 1)
    } else {
        ("#f8fafc", "#475569", 1)
    };
    if node.hottest {
        stroke = "#d97706";
        stroke_width = 2;
    }

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

    let mut out = String::new();
    out.push_str(&format!(
        "<g class=\"{classes}\" data-node=\"{}\">",
        escape_xml(&node.path)
    ));
    out.push_str(&format!("<title>{}</title>", escape_xml(&node_title(node))));
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"7\" fill=\"{fill}\" stroke=\"{stroke}\" \
         stroke-width=\"{stroke_width}\"/>",
        rect.x, rect.y, rect.w, rect.h
    ));
    let badges = node_badges(node);
    if !badges.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"6\" fill=\"#475569\">{}</text>",
            rect.x + rect.w - 2,
            rect.y - 3,
            escape_xml(&badges)
        ));
    }
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{name_y}\" text-anchor=\"middle\" font-size=\"{name_size}\" font-weight=\"700\" \
         fill=\"{text_fill}\">{}</text>",
        rect.x + rect.w / 2,
        escape_xml(&name)
    ));
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{cost_y}\" text-anchor=\"middle\" font-size=\"{meta_size}\" fill=\"{text_fill}\">{}</text>",
        rect.x + rect.w / 2,
        escape_xml(&format_cost(node.cost, node.scale))
    ));
    if !rank.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{rank_y}\" text-anchor=\"middle\" font-size=\"{meta_size}\" fill=\"{text_fill}\">{}</text>",
            rect.x + rect.w / 2,
            escape_xml(&rank)
        ));
    }
    if let Some(step) = path_step {
        let cx = rect.x + rect.w - 7;
        let cy = rect.y + 7;
        let title = format!("{PATH_CAPTION} · step {step} of {total_steps}");
        out.push_str(&format!(
            "<g class=\"atlas-path-badge\"><title>{}</title>\
             <circle cx=\"{cx}\" cy=\"{cy}\" r=\"7\" fill=\"#ea580c\" stroke=\"#ffffff\" stroke-width=\"1\"/>\
             <text x=\"{cx}\" y=\"{}\" text-anchor=\"middle\" font-size=\"6.5\" font-weight=\"700\" \
             fill=\"#ffffff\">{step}</text></g>",
            escape_xml(&title),
            cy + 2
        ));
    }
    out.push_str("</g>");
    out
}

fn render_nodes(
    view: &AtlasView,
    layout: &Layout,
    nodes: &BTreeMap<&str, &AtlasNode>,
    path_steps: &BTreeMap<&str, usize>,
) -> String {
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
        let header = column_header(view, column);
        out.push_str(&format!(
            "<text class=\"atlas-column-header\" x=\"{}\" y=\"{HEADER_Y}\" text-anchor=\"middle\" \
             font-size=\"7.5\" font-weight=\"700\" fill=\"#334155\">{}</text>",
            column_layout.x + column_layout.width / 2,
            escape_xml(&header)
        ));
        for path in &column.files {
            let Some(node) = nodes.get(path.as_str()) else {
                continue;
            };
            let Some(rect) = layout.rows.get(path).copied() else {
                continue;
            };
            let step = path_steps.get(path.as_str()).copied();
            out.push_str(&render_node(node, rect, step, view.path.len()));
        }
        for (aggregate_index, aggregate) in column.aggregated.iter().enumerate() {
            let y = TOP + (column.files.len() + aggregate_index) as i64 * layout.row_h;
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
        if column.unreached && column.nonproduction_unreached > 0 {
            let y = TOP + (column.files.len() + column.aggregated.len()) as i64 * layout.row_h;
            let count = format_count(column.nonproduction_unreached as u64);
            let title =
                format!("{count} non-production unreached files (test/bench/other) · not drawn");
            out.push_str(&format!(
                "<g class=\"atlas-unreached-nonprod\" data-nonprod-unreached=\"{}\"\
                 ><title>{}</title><rect x=\"{}\" y=\"{y}\" width=\"{}\" height=\"{}\" rx=\"6\" \
                 fill=\"#f8fafc\" stroke=\"#94a3b8\" stroke-dasharray=\"4 3\"/>\
                 <text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"6.2\" \
                 font-weight=\"700\" fill=\"#475569\">+{count} non-production unreached</text></g>",
                column.nonproduction_unreached,
                escape_xml(&title),
                column_layout.node_x,
                column_layout.node_w,
                layout.node_h,
                column_layout.node_x + column_layout.node_w / 2,
                y + layout.node_h / 2 + 2
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
        out.push_str(&render_stub(view));
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
        let Some(shape) = edge_shape(edge, &layout.rows) else {
            continue;
        };
        let (class, stroke, dasharray, marker) = if edge.back {
            (
                "atlas-edge atlas-edge--back",
                "#b45309",
                "6 4",
                "url(#atlas-arrow-back)",
            )
        } else {
            (
                "atlas-edge atlas-edge--forward",
                "#6b7280",
                "1",
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
            "<path class=\"{class}\" data-edge=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" \
             stroke-dasharray=\"{dasharray}\" stroke-dashoffset=\"0\" marker-end=\"{marker}\"{path_length} \
             style=\"--atlas-edge:{index}\"><title>{}</title></path>",
            escape_xml(&format!("{}->{}", edge.from, edge.to)),
            shape.path,
            escape_xml(&title)
        ));
        edge_labels.push_str(&format!(
            "<text class=\"atlas-edge-label\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"6.5\" \
             fill=\"#334155\" stroke=\"#ffffff\" stroke-width=\"2.5\" paint-order=\"stroke\">{}</text>",
            shape.label_x,
            shape.label_y - 2,
            escape_xml(&format_distinct_edges(edge.count))
        ));
    }

    let mut traces = String::new();
    for (index, window) in view.path.windows(2).enumerate() {
        let from = &window[0].path;
        let to = &window[1].path;
        if let Some(edge) = view
            .edges
            .iter()
            .find(|edge| &edge.from == from && &edge.to == to)
            && let Some(shape) = edge_shape(edge, &layout.rows)
        {
            let step = index + 1;
            let title = format!("{PATH_CAPTION} · step {step} of {}", view.path.len());
            traces.push_str(&format!(
                "<path class=\"atlas-trace\" d=\"{}\" fill=\"none\" stroke=\"#ea580c\" stroke-width=\"3\" \
                 stroke-dasharray=\"1\" stroke-dashoffset=\"0\" pathLength=\"1\" \
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
    let layout = compute_layout(view);
    format!(
        "{}\n<style>{STATIC_CSS}</style>\n{}\n</svg>",
        svg_open(true, layout.height),
        render_graph_inner(view, &layout)
    )
}

/// HTML fragment for `report.html#map`: header, animated SVG canvas, legend,
/// footer. The section carries exactly one scoped `<style>` and no script.
#[must_use]
pub fn render_canvas_html(view: &AtlasView) -> String {
    let entry_label = format_entry(
        view.entry.policy.label(),
        &view.entry.confidence,
        view.entry.alternatives.len(),
    );
    let hottest_line = view
        .hottest_node
        .as_ref()
        .map(|node| format!("HOTTEST: {} · {}", node.path, node_anchor_text(node)))
        .unwrap_or_else(|| "HOTTEST: no file".to_string());
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

    let has_v1 = view.nodes.iter().any(|node| node.scale == CostScale::V1);
    let unit = if has_v1 {
        "static cost units (v2) · v1 fallback rows labelled".to_string()
    } else {
        "static cost units (v2)".to_string()
    };
    let legend_items = [
        unit,
        "depth = BFS from entry".to_string(),
        "solid = forward structural edge".to_string(),
        "dashed amber = back edge".to_string(),
        "hatch = non-production (test/bench/other)".to_string(),
        "grey = unreached".to_string(),
        "× median function = denominator".to_string(),
        "hulls = approximate prefix groups".to_string(),
    ];
    let legend_items = legend_items
        .iter()
        .map(|item| format!("<span>{}</span>", escape_html(item)))
        .collect::<Vec<_>>()
        .join(" · ");

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
    let unreached = format!(
        "{} unreached production · {} unreached non-production",
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
    let footer = footer_parts
        .iter()
        .map(|part| escape_html(part))
        .collect::<Vec<_>>()
        .join(" · ");

    let layout = compute_layout(view);
    let mut out = String::new();
    out.push_str("<section id=\"atlas-canvas\" class=\"atlas-canvas\">\n");
    out.push_str(&format!("<style>{ANIMATION_CSS}</style>\n"));
    out.push_str("<input type=\"checkbox\" id=\"atlas-pause\" class=\"atlas-pause-input\">\n");
    out.push_str("<label for=\"atlas-pause\" class=\"atlas-pause\">Pause animation</label>\n");
    out.push_str("<header class=\"atlas-head\">\n");
    out.push_str(&format!(
        "<span class=\"atlas-entry\">{}</span>\n",
        escape_html(&entry_label)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-hottest\">{}</span>\n",
        escape_html(&hottest_line)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-path-summary\">{}</span>\n",
        escape_html(&path_line)
    ));
    out.push_str(&format!(
        "<span class=\"atlas-caps\">{}</span>\n",
        escape_html(&caps_line)
    ));
    out.push_str("</header>\n");
    out.push_str("<div class=\"atlas-graph\">\n");
    out.push_str(&svg_open(false, layout.height));
    out.push('\n');
    out.push_str(&render_graph_inner(view, &layout));
    out.push_str("\n</svg>\n</div>\n");
    out.push_str(&format!(
        "<div class=\"atlas-legend\">{legend_items} · <span class=\"atlas-structural-caption\">{}</span></div>\n",
        escape_html(STRUCTURAL_CAPTION)
    ));
    out.push_str(&format!(
        "<footer class=\"atlas-footer\">{footer}</footer>\n"
    ));
    out.push_str("</section>\n");
    out
}

/// Minimal standalone HTML document wrapping [`render_canvas_html`]; no scripts.
#[must_use]
pub fn render_canvas_page(view: &AtlasView) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>Perfpal Atlas canvas</title>\n\
         <style>html,body{{margin:0;padding:0;background:#fff;color:#0f172a}}body{{padding:24px}}</style>\n\
         </head>\n<body>\n{}\n</body>\n</html>\n",
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
        assert!(html.contains("ENTRY · production main · confidence HIGH · 0 alternatives"));
        assert!(html.contains("HOTTEST: src/c.c"));
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
            back.contains("stroke-dasharray=\"6 4\""),
            "back edge must keep its visible dash pattern"
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

    #[test]
    fn canvas_renders_no_entry_and_empty_stubs() {
        let no_entry = render_canvas_html(&build_atlas_view(&no_entry_fixture()));
        assert!(no_entry.contains("NO ENTRY"));
        assert!(no_entry.contains("HOTTEST: src/handler.c"));
        assert!(no_entry.contains("PATH · 0 steps"));
        assert!(
            !no_entry.contains("data-node=\"src/handler.c\""),
            "no fake HOTTEST node in NO ENTRY"
        );
        assert!(!no_entry.contains("data-node="), "no fake node group");

        let empty = render_canvas_html(&build_atlas_view(&empty_fixture()));
        assert!(empty.contains("no source functions recorded"));
        assert!(empty.contains("HOTTEST: no file"));
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
        assert!(html.contains(":checked ~ .atlas-graph *"));
        assert!(html.contains("animation-play-state:paused"));
        assert!(html.contains("prefers-reduced-motion: reduce"));
        assert!(html.contains("@media print"));
        assert!(html.contains("break-inside:avoid"));
        assert!(html.contains("Pause animation"));
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
                "id=\"atlas-canvas\"",
                "<svg",
                "class=\"atlas-legend\"",
                "class=\"atlas-footer\"",
                "class=\"atlas-head\"",
            ] {
                assert!(html.contains(needle), "missing `{needle}`");
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
        assert!(html.contains(">src/main.c</text>"));
    }

    #[test]
    fn legend_lists_every_unit_and_style_key() {
        let html = render_canvas_html(&build_atlas_view(&application_fixture()));
        for item in [
            "depth = BFS from entry",
            "solid = forward structural edge",
            "dashed amber = back edge",
            "hatch = non-production (test/bench/other)",
            "grey = unreached",
            "× median function = denominator",
            "hulls = approximate prefix groups",
        ] {
            assert!(html.contains(item), "missing legend item `{item}`");
        }
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
        assert!(html.contains("+1 non-production unreached"));
        assert!(html.contains("non-production unreached files (test/bench/other) · not drawn"));
        assert!(
            !html.contains("data-node=\"tests/t.c\""),
            "non-production unreached must not be a drawn node"
        );
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
        assert!(html.contains("viewBox=\"0 0 1400 900\""));
        assert_eq!(html.matches("data-node=\"").count(), 40);
        let svg = render_canvas_svg(&view);
        assert!(svg.contains("viewBox=\"0 0 1400 900\""));
    }
}
