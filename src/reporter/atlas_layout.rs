//! Deterministic JSON projection of the Atlas canvas geometry (plan
//! `atlas-build` S1a).
//!
//! The geometry is **not** recomputed here: layout, edge curves, aggregate
//! boxes, cap chips, hulls and node styling all come from the SVG renderer's
//! own crate-visible helpers, so `atlas-layout.json` can never drift from the
//! bytes `report.html` draws.
//!
//! Every array has a total order; object keys serialize sorted because
//! serde_json runs on default features (`preserve_order` OFF).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::reporter::atlas::{AtlasNode, AtlasView};
use crate::reporter::atlas_brief::StartAction;
use crate::reporter::atlas_metrics::format_cost;
use crate::reporter::atlas_render::{
    EdgeShape, NodeRect, SVG_W, aggregate_rect, cap_chip_rect, caps_line, compute_layout,
    edge_shape, footer_text, hottest_line, hull_boxes, node_badges, node_rank_lines, node_style,
    path_line, strip_entry_line,
};

/// Serialize the Atlas story geometry as pretty JSON.
///
/// `findings_included`/`findings_total` are supplied by the caller (the same
/// numbers the report header prints) so this module stays a pure projection of
/// the view plus the already-ranked start actions.
#[must_use]
pub fn to_json(
    view: &AtlasView,
    actions: &[StartAction],
    findings_included: usize,
    findings_total: usize,
) -> String {
    let layout = compute_layout(view);

    let mut column_of: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, column) in view.columns.iter().enumerate() {
        for path in &column.files {
            column_of.insert(path.as_str(), index);
        }
    }
    let step_of: BTreeMap<&str, usize> = view
        .path
        .iter()
        .map(|step| (step.path.as_str(), step.step))
        .collect();

    let mut nodes: Vec<(usize, i64, String, Value)> = Vec::with_capacity(view.nodes.len());
    for node in &view.nodes {
        let column_index = column_of.get(node.path.as_str()).copied().unwrap_or(0);
        let rect = layout.rows.get(&node.path).copied().unwrap_or(NodeRect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        });
        let (fill, stroke, stroke_width, dash) = node_style(node);
        let (ratio_text, top_text) = node_rank_lines(node);
        let dash_value = if dash.is_empty() {
            Value::Null
        } else {
            Value::String(dash.to_string())
        };
        let value = json!({
            "path": node.path,
            "name": node.file_name,
            "x": rect.x,
            "y": rect.y,
            "w": rect.w,
            "h": rect.h,
            "column": column_index,
            "depth": node.depth,
            "drawn_depth": node.drawn_depth,
            "entry": node.entry,
            "hottest": node.hottest,
            "unreached": node.unreached,
            "production": node.production,
            "path_step": step_of.get(node.path.as_str()).copied(),
            "partial_parse": node.partial_parse,
            "fill": fill,
            "stroke": stroke,
            "stroke_width": stroke_width,
            "dash": dash_value,
            "badge": node_badges(node),
            "cost": node.cost,
            "cost_text": format_cost(node.cost, node.scale),
            "ratio_text": ratio_text,
            "top_text": top_text,
            "findings": {
                "hot": node.findings.hot,
                "warn": node.findings.warn,
                "info": node.findings.info,
            },
            "functions": functions_json(node),
        });
        nodes.push((column_index, rect.y, node.path.clone(), value));
    }
    nodes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let nodes: Vec<Value> = nodes.into_iter().map(|(_, _, _, value)| value).collect();

    let mut edge_rows: Vec<(&crate::reporter::atlas::AtlasEdge, EdgeShape)> = view
        .edges
        .iter()
        .filter_map(|edge| edge_shape(edge, &layout).map(|shape| (edge, shape)))
        .collect();
    edge_rows.sort_by(|a, b| a.0.from.cmp(&b.0.from).then(a.0.to.cmp(&b.0.to)));
    let edges: Vec<Value> = edge_rows
        .iter()
        .map(|(edge, shape)| {
            json!({
                "from": edge.from,
                "to": edge.to,
                "d": shape.path,
                "count": edge.count,
                "back": edge.back,
                "on_path": edge.on_path,
            })
        })
        .collect();

    let mut aggregate_rows: Vec<(usize, i64, Value)> = Vec::new();
    for (index, column) in view.columns.iter().enumerate() {
        for (aggregate_index, aggregate) in column.aggregated.iter().enumerate() {
            let rect = aggregate_rect(&layout.columns[index], &layout, column, aggregate_index);
            aggregate_rows.push((
                index,
                rect.y,
                json!({
                    "column": index,
                    "x": rect.x,
                    "y": rect.y,
                    "w": rect.w,
                    "h": rect.h,
                    "count": aggregate.count,
                    "incoming_calls": aggregate.incoming_calls,
                }),
            ));
        }
    }
    aggregate_rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let aggregates: Vec<Value> = aggregate_rows
        .into_iter()
        .map(|(_, _, value)| value)
        .collect();

    let mut chip_rows: Vec<(usize, i64, Value)> = Vec::new();
    for (index, column) in view.columns.iter().enumerate() {
        if let Some(rect) = cap_chip_rect(view, &layout.columns[index], &layout, column) {
            chip_rows.push((
                index,
                rect.y,
                json!({
                    "column": index,
                    "x": rect.x,
                    "y": rect.y,
                    "w": rect.w,
                    "h": rect.h,
                    "production_unreached": view.coverage.unreached_production,
                    "nonproduction_unreached": view.coverage.unreached_nonproduction,
                }),
            ));
        }
    }
    chip_rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let cap_chips: Vec<Value> = chip_rows.into_iter().map(|(_, _, value)| value).collect();

    let mut hull_rows: Vec<(String, i64, i64, Value)> = hull_boxes(view, &layout.rows)
        .into_iter()
        .map(|hull| {
            let label = hull.label;
            let rect = hull.rect;
            let value = json!({
                "x": rect.x,
                "y": rect.y,
                "w": rect.w,
                "h": rect.h,
                "label": label,
            });
            (label, rect.y, rect.x, value)
        })
        .collect();
    hull_rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let hulls: Vec<Value> = hull_rows
        .into_iter()
        .map(|(_, _, _, value)| value)
        .collect();

    let columns: Vec<Value> = view
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            json!({
                "depth": column.depth,
                "x": layout.columns[index].x,
                "unreached": column.unreached,
                "folded": column.folded,
                "file_count": column.file_count,
                "nonproduction_unreached": column.nonproduction_unreached,
            })
        })
        .collect();

    let actions: Vec<Value> = actions
        .iter()
        .map(|action| {
            json!({
                "location": action.location,
                "rule": action.rule,
                "function": action.function,
                "severity_confidence": action.severity_confidence,
                "production": action.production,
                "cost_anchor": action.cost_anchor,
            })
        })
        .collect();

    let hottest_text = view.hottest.as_ref().map(|_| hottest_line(view));

    let value = json!({
        "layout_version": 1,
        "canvas": {"w": SVG_W, "h": layout.height},
        "meta": {
            "entry_text": strip_entry_line(view),
            "hottest_path": view.hottest,
            "hottest_text": hottest_text,
            "path_text": path_line(view),
            "caps_text": caps_line(view),
            "footer_text": footer_text(view),
            "findings_included": findings_included,
            "findings_total": findings_total,
        },
        "state": {
            "empty": view.state.empty,
            "no_entry": view.state.no_entry,
            "library": view.state.library,
            "all_test": view.state.all_test,
            "capped": view.state.capped,
            "partial_parse": view.state.partial_parse,
            "single_file": view.state.single_file,
        },
        "columns": columns,
        "hulls": hulls,
        "nodes": nodes,
        "edges": edges,
        "aggregates": aggregates,
        "cap_chips": cap_chips,
        "stages": stages_json(view),
        "actions": actions,
    });

    serde_json::to_string_pretty(&value)
        .expect("atlas layout JSON is built from serializable primitives")
}

/// Per-node functions, sorted by `(cost desc, line, name)`.
fn functions_json(node: &AtlasNode) -> Vec<Value> {
    let mut functions: Vec<&crate::reporter::atlas::AtlasFunction> =
        node.top_functions.iter().collect();
    functions.sort_by(|a, b| {
        b.cost
            .cmp(&a.cost)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.name.cmp(&b.name))
    });
    functions
        .into_iter()
        .map(|function| {
            json!({
                "id": function.id,
                "name": function.name,
                "line": function.line,
                "cost": function.cost,
                "loop_depth": function.loop_depth,
                "trip_source": function.trip_source.clone().unwrap_or_default(),
                "unknown_trip_loops": function.unknown_trip_loops,
                "fan_in": function.fan_in,
                "fan_out": function.fan_out,
                "recursive": function.recursive,
            })
        })
        .collect()
}

/// Derived stages from the representative path: the first step is the `ENTRY`
/// stage, later steps are `step N`, and `leaf` marks the last. No separate
/// ENTRY stage is prepended, so no two consecutive stages repeat a file. A
/// pathless view collapses to one `ENTRY` stage from the entry point (or the
/// hottest node); empty when nothing was recorded.
fn stages_json(view: &AtlasView) -> Vec<Value> {
    if view.path.is_empty() {
        let file = view
            .entry
            .point
            .as_ref()
            .map(|point| point.path.clone())
            .or_else(|| view.hottest.clone());
        return file
            .into_iter()
            .map(|file| {
                json!({
                    "order": 0,
                    "label": "ENTRY",
                    "depth": 0,
                    "files": [file],
                    "leaf": true,
                })
            })
            .collect();
    }

    let last = view.path.len() - 1;
    view.path
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let label = if index == 0 {
                "ENTRY".to_string()
            } else {
                format!("step {}", step.step)
            };
            json!({
                "order": index,
                "label": label,
                "depth": step.depth,
                "files": [step.path],
                "leaf": index == last,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporter::atlas::build_atlas_view;
    use crate::reporter::atlas::test_support::application_fixture;

    fn render(doc: &crate::schema::MetricsDocument) -> String {
        let view = build_atlas_view(doc);
        let actions = crate::reporter::atlas_brief::start_actions(doc);
        let included = view
            .nodes
            .iter()
            .map(|node| node.findings.hot + node.findings.warn + node.findings.info)
            .sum();
        to_json(&view, &actions, included, doc.findings.len())
    }

    #[test]
    fn layout_json_version_canvas_and_rect_bounds() {
        let doc = application_fixture();
        let view = build_atlas_view(&doc);
        let layout = compute_layout(&view);
        let raw = render(&doc);
        let value: Value = serde_json::from_str(&raw).expect("atlas layout JSON parses");

        assert_eq!(value["layout_version"], 1);
        assert_eq!(value["canvas"]["w"], SVG_W);
        assert_eq!(value["canvas"]["h"], layout.height);

        let nodes = value["nodes"].as_array().expect("nodes array");
        assert_eq!(nodes.len(), view.nodes.len());
        let canvas_w = value["canvas"]["w"].as_i64().unwrap();
        let canvas_h = value["canvas"]["h"].as_i64().unwrap();
        for node in nodes {
            let x = node["x"].as_i64().unwrap();
            let y = node["y"].as_i64().unwrap();
            let w = node["w"].as_i64().unwrap();
            let h = node["h"].as_i64().unwrap();
            assert!(x >= 0 && y >= 0, "node origin off-canvas: {node}");
            assert!(
                x + w <= canvas_w && y + h <= canvas_h,
                "node rect outside canvas: {node}"
            );
        }

        for edge in value["edges"].as_array().expect("edges array") {
            assert!(
                edge["d"].as_str().is_some_and(|path| !path.is_empty()),
                "edge path is empty"
            );
        }
    }

    #[test]
    fn layout_json_is_byte_identical_on_double_render() {
        let doc = application_fixture();
        assert_eq!(render(&doc), render(&doc));
    }

    #[test]
    fn findings_included_is_the_drawn_node_sum() {
        let doc = application_fixture();
        let view = build_atlas_view(&doc);
        let raw = render(&doc);
        let value: Value = serde_json::from_str(&raw).expect("atlas layout JSON parses");

        let included = value["meta"]["findings_included"].as_u64().unwrap();
        let total = value["meta"]["findings_total"].as_u64().unwrap();
        assert!(included <= total, "included findings exceed the scan total");

        let drawn: u64 = value["nodes"]
            .as_array()
            .expect("nodes array")
            .iter()
            .map(|node| {
                node["findings"]["hot"].as_u64().unwrap()
                    + node["findings"]["warn"].as_u64().unwrap()
                    + node["findings"]["info"].as_u64().unwrap()
            })
            .sum();
        assert_eq!(included, drawn, "included != sum of drawn-node counts");
        let view_sum: u64 = view
            .nodes
            .iter()
            .map(|node| node.findings.hot + node.findings.warn + node.findings.info)
            .map(|count| count as u64)
            .sum();
        assert_eq!(included, view_sum);
    }

    #[test]
    fn stages_follow_the_path_without_duplicates() {
        let doc = application_fixture();
        let view = build_atlas_view(&doc);
        assert!(!view.path.is_empty(), "fixture must have a path");
        let raw = render(&doc);
        let value: Value = serde_json::from_str(&raw).expect("atlas layout JSON parses");
        let stages = value["stages"].as_array().expect("stages array");

        assert_eq!(stages.len(), view.path.len());
        let mut seen = std::collections::BTreeSet::new();
        for (index, stage) in stages.iter().enumerate() {
            let files = stage["files"].as_array().expect("files array");
            assert_eq!(files.len(), 1, "each stage names exactly one file");
            let file = files[0].as_str().unwrap();
            assert_eq!(file, view.path[index].path);
            assert!(seen.insert(file.to_string()), "duplicate stage file {file}");
            assert_eq!(stage["order"].as_u64().unwrap(), index as u64);
            assert_eq!(
                stage["depth"].as_u64().unwrap(),
                u64::from(view.path[index].depth)
            );
            let expected_label = if index == 0 {
                "ENTRY".to_string()
            } else {
                format!("step {}", view.path[index].step)
            };
            assert_eq!(stage["label"].as_str(), Some(expected_label.as_str()));
            assert_eq!(stage["leaf"].as_bool().unwrap(), index == stages.len() - 1);
        }
    }

    #[test]
    fn layout_json_does_not_enable_preserve_order() {
        let manifest = include_str!("../../Cargo.toml");
        assert!(
            !manifest.contains("\"preserve_order\""),
            "preserve_order must stay OFF (byte-determinism guard)"
        );
    }
}
