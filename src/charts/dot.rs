//! `charts::dot` — deterministic DOT export (plan `divergence-revamp` step 2).
//!
//! ADR-0002 deferred non-SVG graph exports; this module lands the
//! deterministic serialization half: [`render_dot`] is a pure function of the
//! final document (graphs.* + components[] ONLY — the same ADR-0002 guardrail 1
//! input set as the SVG charts) emitting sorted DOT for the include graph,
//! the call graph, and the component ranking. Wired into the pipeline as the
//! `charts/graph.dot` artifact (cli) and referenced from report.md §5 —
//! consumers call it directly on a parsed metrics.json too.
//!
//! Determinism mechanics (same bar as the SVG charts):
//! - every node/edge line goes through a `BTreeSet` — sorted AND deduped;
//! - attribute emission order is fixed by construction;
//! - labels are DOT-escaped ([`escape_dot`]); no timestamps, no counters.

use std::collections::BTreeSet;

use crate::schema::MetricsDocument;

/// Render `doc`'s graphs + components as deterministic DOT text (LF newlines,
/// exactly one trailing newline).
///
/// Sections, in fixed order:
/// 1. include-graph nodes (`"path";`);
/// 2. resolved include edges (`"from" -> "to";`);
/// 3. call-graph edges — resolved as `"from_function" -> "to_path"`,
///    unresolved as `"from_function" -> "to_name"` — dashed, labelled with
///    the callee name (LOW-confidence hypotheses per ADR-0001);
/// 4. component nodes (`"component:<prefix>" [shape=box, label="<prefix> (rank N)"];`).
#[must_use]
pub fn render_dot(doc: &MetricsDocument) -> String {
    let mut lines: BTreeSet<String> = BTreeSet::new();

    if let Some(graphs) = &doc.graphs {
        let mut nodes: Vec<&str> = graphs
            .include_graph
            .nodes
            .iter()
            .map(|n| n.path.as_str())
            .collect();
        nodes.sort();
        nodes.dedup();
        for node in nodes {
            lines.insert(format!("  \"{}\";", escape_dot(node)));
        }

        let mut include_edges: BTreeSet<(&str, &str)> = BTreeSet::new();
        for edge in &graphs.include_graph.edges {
            if edge.resolved
                && let Some(to) = edge.to.as_deref()
            {
                include_edges.insert((edge.from.as_str(), to));
            }
        }
        for (from, to) in include_edges {
            lines.insert(format!(
                "  \"{}\" -> \"{}\";",
                escape_dot(from),
                escape_dot(to)
            ));
        }

        if let Some(call_graph) = &graphs.call_graph {
            let mut call_edges: BTreeSet<(&str, &str, &str)> = BTreeSet::new();
            for edge in &call_graph.edges {
                let target = edge.to_path.as_deref().unwrap_or(edge.to_name.as_str());
                call_edges.insert((edge.from_function.as_str(), target, edge.to_name.as_str()));
            }
            for (from, target, callee) in call_edges {
                lines.insert(format!(
                    "  \"{}\" -> \"{}\" [style=dashed, label=\"{}\"];",
                    escape_dot(from),
                    escape_dot(target),
                    escape_dot(callee)
                ));
            }
        }
    }

    if let Some(components) = &doc.components {
        let mut ranked: Vec<(&str, u32)> = components
            .iter()
            .map(|c| (c.path_prefix.as_str(), c.rank))
            .collect();
        ranked.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(b.0)));
        for (prefix, rank) in ranked {
            lines.insert(format!(
                "  \"component:{}\" [shape=box, label=\"{} (rank {rank})\"];",
                escape_dot(prefix),
                escape_dot(prefix),
            ));
        }
    }

    let mut out = String::from("digraph perfpal {\n");
    for line in lines {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("}\n");
    out
}

/// DOT-escape a label: backslash, double quote, and line breaks. Operates on
/// chars (boundary-safe); other characters pass through verbatim.
#[must_use]
pub fn escape_dot(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' | '\r' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    //! Stub locks: byte-determinism, sorted/deduped emission, hostile-label
    //! escaping, empty-document shape.

    use super::*;
    use crate::ir::{IncludeForm, Span};
    use crate::schema::{
        CallEdge, CallGraphSection, ComponentEntry, ComponentMetrics, GraphNode, GraphsSection,
        IncludeEdgeDto, IncludeGraph, SCHEMA_VERSION, ScanInfo, StatsSection, ToolInfo,
    };

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

    fn sample_doc() -> MetricsDocument {
        let mut doc = base_doc();
        doc.graphs = Some(GraphsSection {
            include_graph: IncludeGraph {
                nodes: vec![
                    GraphNode {
                        path: "b.c".to_string(),
                    },
                    GraphNode {
                        path: "a.c".to_string(),
                    },
                    GraphNode {
                        path: "b.c".to_string(),
                    },
                ],
                edges: vec![
                    IncludeEdgeDto {
                        from: "b.c".to_string(),
                        to: Some("a.c".to_string()),
                        form: IncludeForm::Quoted,
                        resolved: true,
                        span: Span::point(1, 1),
                    },
                    // Duplicate + unresolved-quoted (excluded: data, not chart).
                    IncludeEdgeDto {
                        from: "b.c".to_string(),
                        to: Some("a.c".to_string()),
                        form: IncludeForm::Quoted,
                        resolved: true,
                        span: Span::point(9, 1),
                    },
                    IncludeEdgeDto {
                        from: "a.c".to_string(),
                        to: None,
                        form: IncludeForm::Quoted,
                        resolved: false,
                        span: Span::point(2, 1),
                    },
                ],
            },
            call_graph: Some(CallGraphSection {
                edges: vec![
                    CallEdge::new("b.c#g#9:1", "f", "b.c", Some("a.c".to_string())),
                    CallEdge::new("b.c#g#9:1", "missing", "b.c", None),
                ],
            }),
        });
        doc.components = Some(vec![ComponentEntry {
            component_id: ".".to_string(),
            path_prefix: ".".to_string(),
            rank: 1,
            metrics: ComponentMetrics {
                complexity_sum: 3.0,
                alloc_site_density: 0.0,
                max_loop_depth: 1,
                finding_density: 0.0,
            },
            finding_count: Some(0),
        }]);
        doc
    }

    #[test]
    fn double_render_is_byte_identical() {
        assert_eq!(render_dot(&sample_doc()), render_dot(&sample_doc()));
    }

    #[test]
    fn emission_is_sorted_deduped_and_section_ordered() {
        let dot = render_dot(&sample_doc());
        let body: Vec<&str> = dot.lines().skip(1).take_while(|l| *l != "}").collect();
        let mut sorted = body.clone();
        sorted.sort();
        assert_eq!(body, sorted, "DOT body lines must be sorted:\n{dot}");
        // Dedup: the doubled b.c→a.c include edge emits once.
        assert_eq!(
            body.iter()
                .filter(|l| l.contains("\"b.c\" -> \"a.c\";"))
                .count(),
            1
        );
        // Nodes, include edge, both call edges (resolved + unresolved), component.
        for expected in [
            "\"a.c\";",
            "\"b.c\";",
            "\"b.c\" -> \"a.c\";",
            "\"b.c#g#9:1\" -> \"a.c\" [style=dashed, label=\"f\"];",
            "\"b.c#g#9:1\" -> \"missing\" [style=dashed, label=\"missing\"];",
            "\"component:.\" [shape=box, label=\". (rank 1)\"];",
        ] {
            assert!(dot.contains(expected), "missing line {expected}:\n{dot}");
        }
        // Unresolved-quoted include edge is NOT charted.
        assert!(!dot.contains("null"), "null target leaked:\n{dot}");
    }

    #[test]
    fn hostile_labels_are_escaped() {
        let mut doc = base_doc();
        doc.graphs = Some(GraphsSection {
            include_graph: IncludeGraph {
                nodes: vec![GraphNode {
                    path: "we\"ird\\name.c".to_string(),
                }],
                edges: vec![],
            },
            call_graph: None,
        });
        let dot = render_dot(&doc);
        assert!(
            dot.contains("\"we\\\"ird\\\\name.c\";"),
            "hostile node not escaped:\n{dot}"
        );
        assert_eq!(escape_dot("a\nb"), "a\\nb");
    }

    #[test]
    fn empty_document_renders_empty_digraph() {
        assert_eq!(render_dot(&base_doc()), "digraph perfpal {\n}\n");
    }
}
