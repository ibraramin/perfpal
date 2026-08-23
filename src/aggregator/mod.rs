//! `aggregator` — architecture views & document assembly (modules.md §3.4).
//!
//! Consumes: IR stream ([`FileUnit`]s) + analyzer `findings[]` + provenance
//! from cli. Produces: the complete metrics.json document as schema-typed
//! data (all nine sections; P1 emits the required set plus
//! `parse_health_summary` and `graphs.include_graph` — the schema-optional
//! `components[]`, `stats.distributions`, and `graphs.call_graph` are omitted
//! until their owning waves land, see [`schema::MetricsDocument`]).
//!
//! Sort orders ENFORCED here (modules.md §1 Principle 2 ownership table):
//! - `stats.functions[]`: `(path, span.start_line, span.start_col)`;
//! - `graphs.include_graph.nodes`: sorted by path;
//! - `graphs.include_graph.edges`: sorted by `(from, to)`; only RESOLVED
//!   internal edges enter the graph (quoted includes resolving in-repo;
//!   angle-bracket includes resolving to in-repo files per ADR-0002 chart
//!   policy — the v1 scanner buckets all angle includes external, so none
//!   appear today);
//! - `files[]`: scanner-guaranteed sorted-by-path; re-sorted defensively so
//!   the aggregator's output is correct even if a future scanner regression
//!   slips past its own tests.
//!
//! `findings[]` pass through in the ANALYZER's total order untouched —
//! document order equals report order (modules.md §3.3), re-sorting here
//! would risk diverging from the report §4 comparator.
//!
//! Non-goals: rendering; interpretation; chart capping decisions.

use crate::analyzer::Finding;
use crate::ir::{FunctionRecord, IncludeForm};
use crate::scanner::FileUnit;
use crate::schema::{
    self, FileEntry, FindingEntry, GraphNode, GraphsSection, IncludeEdgeDto, IncludeGraph,
    MetricsDocument, ParseHealthEntry, ParseHealthSummary, ScanInfo, StatsSection, ToolInfo,
};

/// Provenance values captured by cli (modules.md §3.1 owns the clock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Scanned root VERBATIM as given to the CLI (output-contract §2).
    pub root_path: String,
    /// UTC RFC3339 scan-start instant.
    pub started_at: String,
    /// Wall-clock scan duration in milliseconds.
    pub duration_ms: u64,
    /// Version of the producing binary (`tool.version`).
    pub tool_version: String,
}

/// Assemble the complete metrics.json document from the IR stream, findings,
/// and cli-captured provenance.
///
/// Deterministic: same inputs ⇒ same document (provenance values are the
/// caller's clock-derived data, not generated here).
#[must_use]
pub fn assemble(
    units: &[FileUnit],
    findings: &[Finding],
    provenance: &Provenance,
) -> MetricsDocument {
    // files[] — defensive re-sort of the scanner-guaranteed order.
    let mut units_sorted: Vec<&FileUnit> = units.iter().collect();
    units_sorted.sort_by(|a, b| a.file.path.cmp(&b.file.path));

    let files: Vec<FileEntry> = units_sorted
        .iter()
        .map(|u| FileEntry {
            path: u.file.path.clone(),
            language: u.file.language.clone(),
            loc: u.file.loc,
            function_count: u.file.function_count,
            global_var_count: u.file.global_var_count,
            include_counts: u.file.include_counts,
            parse_health: parse_health_entry(&u.parse_health),
        })
        .collect();

    let loc_total = if units.is_empty() {
        None
    } else {
        Some(
            files
                .iter()
                .fold(crate::ir::LocCounts::default(), |acc, f| {
                    crate::ir::LocCounts {
                        code: acc.code + f.loc.code,
                        comment: acc.comment + f.loc.comment,
                        blank: acc.blank + f.loc.blank,
                    }
                }),
        )
    };

    let mut ok = 0u32;
    let mut partial = 0u32;
    let mut failed = 0u32;
    for u in units {
        match u.parse_health.status {
            crate::ir::ParseStatus::Ok => ok += 1,
            crate::ir::ParseStatus::Partial => partial += 1,
            crate::ir::ParseStatus::Failed => failed += 1,
        }
    }

    let functions = function_stats(units);
    let graphs = include_graph(units);

    MetricsDocument {
        schema_version: schema::SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: provenance.tool_version.clone(),
        },
        scan: ScanInfo {
            root_path: provenance.root_path.clone(),
            started_at: provenance.started_at.clone(),
            duration_ms: Some(provenance.duration_ms),
            languages: vec!["c".to_string()],
            file_count: units.len().min(u32::MAX as usize) as u32,
            loc_total,
        },
        parse_health_summary: Some(ParseHealthSummary {
            ok,
            partial,
            failed,
        }),
        files,
        stats: StatsSection { functions },
        findings: findings_with_symbols(units_sorted, findings),
        graphs: Some(graphs),
    }
}

/// Per-file parse-health entry; empty error regions are omitted
/// (schema-optional field, omit-don't-null policy).
fn parse_health_entry(health: &crate::ir::ParseHealth) -> ParseHealthEntry {
    ParseHealthEntry {
        status: health.status,
        error_regions: if health.error_regions.is_empty() {
            None
        } else {
            Some(health.error_regions.clone())
        },
        error_node_count: health.error_node_count,
    }
}

/// `stats.functions[]` across ALL files, sorted
/// `(path, span.start_line, span.start_col)` (modules.md §1 aggregator row).
fn function_stats(units: &[FileUnit]) -> Vec<schema::FunctionStats> {
    let mut records: Vec<&FunctionRecord> = units
        .iter()
        .flat_map(|u| u.functions.iter().map(|f| &f.record))
        .collect();
    records.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.span.start_line.cmp(&b.span.start_line))
            .then(a.span.start_col.cmp(&b.span.start_col))
    });
    records
        .into_iter()
        .map(|r| schema::FunctionStats {
            id: r.id.clone(),
            name: r.name.clone(),
            path: r.path.clone(),
            span: r.span,
            param_count: r.param_count,
            cyclomatic_complexity: r.cyclomatic_complexity,
            cognitive_complexity: r.cognitive_complexity,
            max_loop_depth: r.max_loop_depth,
            alloc_site_count: r.alloc_site_count,
            length_lines: r.length_lines,
        })
        .collect()
}

/// `graphs.include_graph`: nodes = every scanned file (sorted by path); edges
/// = resolved internal includes only, sorted `(from, to)` (modules.md §3.4).
fn include_graph(units: &[FileUnit]) -> GraphsSection {
    let mut nodes: Vec<GraphNode> = units
        .iter()
        .map(|u| GraphNode {
            path: u.file.path.clone(),
        })
        .collect();
    nodes.sort_by(|a, b| a.path.cmp(&b.path));

    let mut edges: Vec<IncludeEdgeDto> = units
        .iter()
        .flat_map(|u| u.includes.iter().map(move |e| (u.file.path.as_str(), e)))
        .filter(|(_, e)| e.resolved && e.to_path.is_some())
        .map(|(from, e)| IncludeEdgeDto {
            from: from.to_string(),
            to: e.to_path.clone().unwrap_or_default(),
            form: e.form,
            resolved: e.resolved,
            span: e.span,
        })
        .collect();
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    GraphsSection {
        include_graph: IncludeGraph { nodes, edges },
    }
}

/// `findings[]` passthrough in the analyzer's total order, each lifted into
/// its schema shape with the enclosing-function symbol resolved when one
/// contains the finding span (schema-optional `symbol`; first hit in
/// `(path, line, col)` order keeps resolution deterministic).
fn findings_with_symbols(units_sorted: Vec<&FileUnit>, findings: &[Finding]) -> Vec<FindingEntry> {
    findings
        .iter()
        .map(|finding| {
            let symbol = units_sorted
                .iter()
                .filter(|u| u.file.path == finding.path)
                .flat_map(|u| u.functions.iter())
                .find(|f| f.record.span.intersects(&finding.span))
                .map(|f| f.record.name.clone());
            schema::finding_entry(finding, symbol)
        })
        .collect()
}

/// Keep the `IncludeForm` import honest: the graph carries both forms today
/// only via resolved quoted edges, but the DTO type references it directly.
const _: Option<IncludeForm> = None;

#[cfg(test)]
mod tests {
    //! Sort-order locks (modules.md §1 rows owned by the aggregator),
    //! fixture-driven per the P1 test plan.

    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::ir::{CallEvent, EventMeta, IncludeEdge, LocCounts, Span};
    use crate::scanner::FunctionUnit;

    fn unit(path: &str) -> FileUnit {
        FileUnit {
            file: crate::ir::FileRecord {
                path: path.to_string(),
                language: "c".to_string(),
                loc: LocCounts::default(),
                function_count: 0,
                global_var_count: 0,
                include_counts: Default::default(),
            },
            ..FileUnit::default()
        }
    }

    fn func(path: &str, name: &str, line: u32, col: u32) -> FunctionUnit {
        FunctionUnit {
            record: FunctionRecord {
                id: format!("{path}#{name}#{line}:{col}"),
                name: name.to_string(),
                path: path.to_string(),
                // A body-extending span so enclosing-function lookups
                // (symbol resolution) can hit findings inside it.
                span: crate::ir::Span {
                    start_line: line,
                    start_col: col,
                    end_line: line + 20,
                    end_col: 1,
                },
                cyclomatic_complexity: 1,
                ..FunctionRecord::default()
            },
            ..FunctionUnit::default()
        }
    }

    fn edge(from: &str, to: &str, resolved: bool) -> IncludeEdge {
        IncludeEdge {
            from_path: from.to_string(),
            to_path: if resolved { Some(to.to_string()) } else { None },
            form: IncludeForm::Quoted,
            resolved,
            span: Span::point(1, 1),
        }
    }

    fn finding(path: &str, line: u32, col: u32) -> Finding {
        Finding {
            rule_id: "PERF-C-ALLOC-001".to_string(),
            title: "Heap allocation inside loop".to_string(),
            message: "candidate".to_string(),
            severity: Severity::Hot,
            confidence: Confidence::High,
            resolver_tier: crate::ir::ResolverTier::Syntactic,
            span: Span::point(line, col),
            path: path.to_string(),
            evidence: Default::default(),
            macro_suspect: false,
            error_node_overlap: false,
        }
    }

    #[test]
    fn functions_sorted_by_path_then_line_then_col() {
        let mut b = unit("b.c");
        b.functions = vec![func("b.c", "second", 10, 1), func("b.c", "first", 2, 5)];
        let mut a = unit("a.c");
        a.functions = vec![func("a.c", "late", 9, 3), func("a.c", "early", 9, 1)];

        let doc = assemble(&[b, a], &[], &prov());
        let keys: Vec<(String, u32, u32)> = doc
            .stats
            .functions
            .iter()
            .map(|f| (f.path.clone(), f.span.start_line, f.span.start_col))
            .collect();
        assert_eq!(
            keys,
            [
                ("a.c".to_string(), 9, 1),
                ("a.c".to_string(), 9, 3),
                ("b.c".to_string(), 2, 5),
                ("b.c".to_string(), 10, 1),
            ]
        );
    }

    #[test]
    fn include_graph_nodes_and_edges_sorted_unresolved_excluded() {
        let mut z = unit("z.c");
        z.includes = vec![
            edge("z.c", "shared.h", true),
            edge("z.c", "<stdio.h>", false), // angle/external: excluded
            edge("z.c", "missing.h", false), // unresolvable: excluded
        ];
        let mut a = unit("a.c");
        a.includes = vec![edge("a.c", "z.c", true)];
        // Same (from,to) pair twice → stable adjacent duplicates allowed;
        // ordering contract is non-decreasing, not deduplication.
        let mut m = unit("m.c");
        m.includes = vec![edge("m.c", "shared.h", true)];

        let doc = assemble(&[z, m, a], &[], &prov());
        let graph = doc
            .graphs
            .as_ref()
            .expect("graphs present")
            .include_graph
            .clone();
        let node_paths: Vec<&str> = graph.nodes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(node_paths, ["a.c", "m.c", "z.c"]);

        let edge_keys: Vec<(String, String)> = graph
            .edges
            .iter()
            .map(|e| (e.from.clone(), e.to.clone()))
            .collect();
        assert_eq!(
            edge_keys,
            [
                ("a.c".to_string(), "z.c".to_string()),
                ("m.c".to_string(), "shared.h".to_string()),
                ("z.c".to_string(), "shared.h".to_string()),
            ]
        );
        assert!(
            graph.edges.iter().all(|e| e.resolved),
            "unresolved edges must not enter the graph"
        );
    }

    #[test]
    fn findings_pass_through_in_analyzer_order_with_symbols() {
        // Analyzer already ordered these (hot→…); aggregator must NOT reorder.
        let ordered = vec![finding("b.c", 20, 1), finding("a.c", 5, 2)];
        let mut b = unit("b.c");
        b.functions = vec![func("b.c", "enclosing", 18, 1)];
        let a = unit("a.c");

        let doc = assemble(&[b, a], &ordered, &prov());
        let ids: Vec<String> = doc.findings.iter().map(|f| f.finding_id.clone()).collect();
        assert_eq!(
            ids,
            ["PERF-C-ALLOC-001:b.c:20:1", "PERF-C-ALLOC-001:a.c:5:2",]
        );
        assert_eq!(doc.findings[0].symbol.as_deref(), Some("enclosing"));
        assert_eq!(
            doc.findings[1].symbol, None,
            "no enclosing function ⇒ omitted"
        );
    }

    #[test]
    fn zero_file_scan_yields_valid_empty_document() {
        let doc = assemble(&[], &[], &prov());
        assert_eq!(doc.scan.languages, ["c"]);
        assert_eq!(doc.scan.file_count, 0);
        assert!(doc.files.is_empty());
        assert!(doc.findings.is_empty());
        assert_eq!(
            doc.parse_health_summary,
            Some(ParseHealthSummary {
                ok: 0,
                partial: 0,
                failed: 0
            })
        );
        assert!(doc.scan.loc_total.is_none());
        let graph = doc.graphs.expect("graphs present").include_graph;
        assert!(graph.nodes.is_empty() && graph.edges.is_empty());
    }

    #[test]
    fn parse_health_summary_counts_statuses() {
        let mut ok_u = unit("ok.c");
        ok_u.parse_health.status = crate::ir::ParseStatus::Ok;
        let mut part = unit("part.c");
        part.parse_health.status = crate::ir::ParseStatus::Partial;
        let mut bad = unit("bad.c");
        bad.parse_health.status = crate::ir::ParseStatus::Failed;

        let doc = assemble(&[bad, ok_u, part], &[], &prov());
        assert_eq!(
            doc.parse_health_summary,
            Some(ParseHealthSummary {
                ok: 1,
                partial: 1,
                failed: 1
            })
        );
    }

    fn prov() -> Provenance {
        Provenance {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: 7,
            tool_version: "0.1.0".to_string(),
        }
    }

    /// Silence unused-import lint for CallEvent/EventMeta kept for upcoming
    /// call-graph assembly tests (P2).
    const _: Option<CallEvent> = None;
    const _: Option<EventMeta> = None;
}
