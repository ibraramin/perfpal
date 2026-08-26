//! `aggregator` — architecture views & document assembly (modules.md §3.4).
//!
//! Consumes: IR stream ([`FileUnit`]s) + analyzer `findings[]` + provenance
//! from cli. Produces: the complete metrics.json document as schema-typed
//! data (all nine sections; P1 emitted the required set plus
//! `parse_health_summary` and `graphs.include_graph`; P4 Wave 2b adds
//! `graphs.call_graph` and `components[]` — `stats.distributions` stays
//! reserved-for-future, see [`schema::MetricsDocument`]).
//!
//! Sort orders ENFORCED here (modules.md §1 Principle 2 ownership table):
//! - `stats.functions[]`: `(path, span.start_line, span.start_col)`;
//! - `graphs.include_graph.nodes`: sorted by path;
//! - `graphs.include_graph.edges`: sorted by `(from, to)` ascending.
//!   Admission (m6): resolved internal edges (quoted or angle-in-repo) PLUS
//!   unresolved QUOTED edges (`to: null`, data-not-error); unresolved ANGLE
//!   edges stay excluded (external bucket). Rust's `Option<String>` Ord
//!   places `None` BEFORE every `Some(_)`, so an includer's unresolved-quoted
//!   edges sort ahead of its resolved edges sharing the same `from` — pinned
//!   by test;
//! - `graphs.call_graph.edges`: aggregator OWNS document order — re-sorted
//!   `(from_function, to_name, from_path)` ascending regardless of producer
//!   order;
//! - `components[]`: ranked by finding_density ↓ → complexity_sum ↓ →
//!   max_loop_depth ↓ → alloc_site_density ↓ → path_prefix ↑ (1-based rank),
//!   emitted in `(rank, path_prefix)` order (path_prefix is the unique
//!   grouping key, so the two orders coincide);
//! - `files[]`: scanner-guaranteed sorted-by-path; re-sorted defensively so
//!   the aggregator's output is correct even if a future scanner regression
//!   slips past its own tests.
//!
//! ARCH-002 asymmetry note (founder-approved): `components[].rank` uses
//! finding_density; the ARCH-002 heuristic ranks IR-only (no density). They
//! may disagree; that is accepted and documented — do not unify.
//!
//! `findings[]` pass through in the ANALYZER's total order untouched —
//! document order equals report order (modules.md §3.3), re-sorting here
//! would risk diverging from the report §4 comparator.
//!
//! Non-goals: rendering; interpretation; chart capping decisions.

use std::collections::BTreeMap;

use crate::analyzer::{CrossFileCallEdge, Finding, cross_file_call_edges};
use crate::ir::{FunctionRecord, IncludeForm};
use crate::scanner::FileUnit;
use crate::schema::{
    self, CallEdge, CallGraphSection, ComponentEntry, ComponentMetrics, DistributionEntry,
    FileEntry, FindingEntry, GraphNode, GraphsSection, IncludeEdgeDto, IncludeGraph,
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
    let graphs = GraphsSection {
        include_graph: include_graph(units),
        // ALWAYS Some — even with zero edges — so consumers can rely on the
        // ["edges"] shape (mirrors graphs-always-Some precedent).
        call_graph: Some(call_graph(units)),
    };
    let components = components(units, findings);

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
        stats: StatsSection {
            functions,
            // reserved-for-future: the (metric, bucket) comparator is
            // implemented + unit-tested ([`sort_distributions`]) but no
            // producer populates the field yet.
            distributions: None,
        },
        findings: findings_with_symbols(units_sorted, findings),
        graphs: Some(graphs),
        components: Some(components),
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
/// sorted `(from, to)` ascending. Admission (m6): resolved internal edges
/// (quoted or angle-in-repo) PLUS unresolved QUOTED edges (`to: null` — data,
/// not error, modules.md Principle 4); unresolved ANGLE edges stay excluded
/// (external bucket). Duplicates are preserved, not deduped.
///
/// Sort note: Rust's `Option<String>` Ord places `None` BEFORE every
/// `Some(_)`, so an includer's unresolved-quoted edges sort ahead of its
/// resolved edges sharing the same `from`. Pinned by test.
fn include_graph(units: &[FileUnit]) -> IncludeGraph {
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
        .filter(|(_, e)| e.resolved || e.form == IncludeForm::Quoted)
        .map(|(from, e)| IncludeEdgeDto {
            from: from.to_string(),
            to: e.to_path.clone(),
            form: e.form,
            resolved: e.resolved,
            span: e.span,
        })
        .collect();
    // Option<String> Ord: None < Some(_) — unresolved-quoted edges of one
    // includer precede its resolved edges with the same `from`.
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    IncludeGraph { nodes, edges }
}

/// One [`CrossFileCallEdge`] lifted into its schema shape; the constructor
/// hardcodes the schema-const confidence `"LOW"` (ADR-0001 name-based
/// matching).
fn call_edge_from(edge: CrossFileCallEdge) -> CallEdge {
    CallEdge::new(
        edge.from_function_id,
        edge.callee_name,
        edge.from_path,
        Some(edge.to_path),
    )
}

/// `graphs.call_graph`: cross-file call edges only (intra-file omitted in
/// v1), re-sorted by the AGGREGATOR's document order
/// `(from_function, to_name, from_path)` ascending — producer order is never
/// trusted (modules.md §1 Principle 2).
fn call_graph(units: &[FileUnit]) -> CallGraphSection {
    let mut edges: Vec<CallEdge> = cross_file_call_edges(units)
        .into_iter()
        .map(call_edge_from)
        .collect();
    edges.sort_by(|a, b| {
        a.from_function
            .cmp(&b.from_function)
            .then(a.to_name.cmp(&b.to_name))
            .then(a.from_path.cmp(&b.from_path))
    });
    CallGraphSection { edges }
}

/// Per-directory accumulator feeding [`components`].
struct ComponentAcc {
    complexity_sum: f64,
    max_loop_depth: u32,
    alloc_site_count: u64,
    kloc: f64,
    finding_count: u32,
}

impl ComponentAcc {
    fn new() -> Self {
        Self {
            complexity_sum: 0.0,
            max_loop_depth: 0,
            alloc_site_count: 0,
            kloc: 0.0,
            finding_count: 0,
        }
    }

    /// Densities are per KLOC; a zero-KLOC component (all-comment/blank
    /// files) yields 0.0 rather than ±inf/NaN.
    fn alloc_site_density(&self) -> f64 {
        if self.kloc == 0.0 {
            0.0
        } else {
            self.alloc_site_count as f64 / self.kloc
        }
    }

    fn finding_density(&self) -> f64 {
        if self.kloc == 0.0 {
            0.0
        } else {
            f64::from(self.finding_count) / self.kloc
        }
    }
}

/// Parent directory of a repo-relative POSIX path; root-level files map to
/// `"."`. Scanner paths are already clean POSIX (join_normalized resolves
/// `.`/`..` and collapses separators), so lexical splitting suffices — no
/// extra normalization here.
fn parent_dir(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => ".".to_string(),
    }
}

/// `components[]`: ONE component per parent directory of every scanned
/// FileRecord path (repo-relative POSIX; root-level files → prefix `"."`).
///
/// Metrics per component:
/// - `complexity_sum`: Σ cyclomatic_complexity over FunctionRecords in the dir;
/// - `max_loop_depth`: max over those records (0 when none);
/// - `alloc_site_density` / `finding_density`: per KLOC (Σ loc.code / 1000),
///   guarded to 0.0 when KLOC == 0;
/// - `finding_count`: findings whose path's parent dir equals the prefix —
///   ALWAYS populated (Some(n), even 0).
///
/// Rank composite: finding_density ↓ → complexity_sum ↓ → max_loop_depth ↓ →
/// alloc_site_density ↓ → path_prefix ↑; rank is the 1-based position in that
/// order. Emission order = `(rank, path_prefix)` (path_prefix unique ⇒
/// identical). Float comparisons use `total_cmp` for total determinism.
///
/// ARCH-002 asymmetry note: this ranking uses finding_density; the ARCH-002
/// heuristic ranks IR-only (no density). They may disagree by design.
fn components(units: &[FileUnit], findings: &[Finding]) -> Vec<ComponentEntry> {
    let mut acc: BTreeMap<String, ComponentAcc> = BTreeMap::new();

    for unit in units {
        let entry = acc
            .entry(parent_dir(&unit.file.path))
            .or_insert_with(ComponentAcc::new);
        entry.kloc += f64::from(unit.file.loc.code) / 1000.0;

        for func in &unit.functions {
            let record = &func.record;
            let entry = acc
                .entry(parent_dir(&record.path))
                .or_insert_with(ComponentAcc::new);
            entry.complexity_sum += f64::from(record.cyclomatic_complexity);
            entry.max_loop_depth = entry.max_loop_depth.max(record.max_loop_depth);
            entry.alloc_site_count += u64::from(record.alloc_site_count);
        }
    }

    // Findings count into their directory ONLY when that directory exists as
    // a component (components come from FileRecords alone).
    for finding in findings {
        if let Some(entry) = acc.get_mut(&parent_dir(&finding.path)) {
            entry.finding_count += 1;
        }
    }

    let mut entries: Vec<ComponentEntry> = acc
        .into_iter()
        .map(|(prefix, a)| ComponentEntry {
            component_id: prefix.clone(),
            path_prefix: prefix,
            rank: 0, // assigned after sorting below
            metrics: ComponentMetrics {
                complexity_sum: a.complexity_sum,
                alloc_site_density: a.alloc_site_density(),
                max_loop_depth: a.max_loop_depth,
                finding_density: a.finding_density(),
            },
            finding_count: Some(a.finding_count),
        })
        .collect();

    entries.sort_by(|x, y| {
        y.metrics
            .finding_density
            .total_cmp(&x.metrics.finding_density)
            .then(
                y.metrics
                    .complexity_sum
                    .total_cmp(&x.metrics.complexity_sum),
            )
            .then(y.metrics.max_loop_depth.cmp(&x.metrics.max_loop_depth))
            .then(
                y.metrics
                    .alloc_site_density
                    .total_cmp(&x.metrics.alloc_site_density),
            )
            .then(x.path_prefix.cmp(&y.path_prefix))
    });
    for (position, entry) in entries.iter_mut().enumerate() {
        entry.rank = position as u32 + 1;
    }
    entries
}

/// Sort `stats.distributions[]` by `(metric, bucket)` ascending — the
/// aggregator-owned contract per modules.md §1. RESERVED-FOR-FUTURE: no
/// producer populates distributions yet, so this comparator is not called
/// during assembly; it is unit-tested so the ownership row is proven before
/// the first producer lands.
#[allow(dead_code, clippy::ptr_arg)] // reserved-for-future (see doc); proven via unit test
pub(crate) fn sort_distributions(entries: &mut Vec<DistributionEntry>) {
    entries.sort_by(|a, b| a.metric.cmp(&b.metric).then(a.bucket.cmp(&b.bucket)));
}

/// `findings[]` passthrough in the analyzer's total order, each lifted into
/// its schema shape with the enclosing-function symbol resolved when one
/// contains the finding span (schema-optional `symbol`; resolution is
/// FIRST-HIT in backend emission order over the sorted units — deterministic
/// per input, not a `(path, line, col)` ranking of candidates).
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

#[cfg(test)]
mod tests {
    //! Sort-order locks (modules.md §1 rows owned by the aggregator),
    //! fixture-driven per the P1 test plan; m6 include-graph admission;
    //! call_graph + components assembly per P4 Wave 2b.

    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::ir::{
        CallEvent, CalleeKind, EventMeta, IncludeEdge, LocCounts, Span, StatementRole,
    };
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

    fn quoted_edge(from: &str, to: Option<&str>, resolved: bool) -> IncludeEdge {
        IncludeEdge {
            from_path: from.to_string(),
            to_path: to.map(str::to_string),
            form: IncludeForm::Quoted,
            resolved,
            span: Span::point(1, 1),
        }
    }

    fn edge(from: &str, to: &str, resolved: bool) -> IncludeEdge {
        quoted_edge(from, Some(to), resolved)
    }

    /// Unresolved ANGLE edge — raw target text stays in `to_path`
    /// (ir-v1.md §3.1 lifecycle), `resolved=false`: the external bucket.
    fn angle_edge(from: &str, raw_target: &str) -> IncludeEdge {
        IncludeEdge {
            from_path: from.to_string(),
            to_path: Some(raw_target.to_string()),
            form: IncludeForm::Angle,
            resolved: false,
            span: Span::point(1, 1),
        }
    }

    fn direct_call(callee: &str, line: u32, col: u32) -> CallEvent {
        CallEvent {
            meta: EventMeta {
                span: Span::point(line, col),
                ..EventMeta::default()
            },
            callee_name: callee.to_string(),
            callee_kind: CalleeKind::Direct,
            args: vec![],
            statement_role: StatementRole::Unknown,
            conditional_guarded: false,
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

    /// m6 admission semantics (Wave 2b — deliberately replaces the old
    /// unresolved-exclusion contract): unresolved QUOTED edges PRESENT
    /// (`to: null`, `resolved: false`); unresolved ANGLE edges ABSENT;
    /// resolved internal edges unchanged; `(from, to)` sort with
    /// None-before-Some pinned; duplicates preserved.
    #[test]
    fn include_graph_m6_admission_sort_and_duplicates() {
        let mut z = unit("z.c");
        z.includes = vec![
            edge("z.c", "shared.h", true),
            // Unresolved QUOTED: stays in the graph, to_path null.
            quoted_edge("z.c", None, false),
            // Duplicate of the same miss at a different directive site:
            // preserved, distinguishable by span.
            {
                let mut e = quoted_edge("z.c", None, false);
                e.span = Span::point(7, 3);
                e
            },
            // Unresolved ANGLE (external bucket): excluded.
            angle_edge("z.c", "<stdio.h>"),
        ];
        let mut a = unit("a.c");
        a.includes = vec![edge("a.c", "z.c", true)];
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

        // Angle-unresolved absent everywhere.
        assert!(
            graph
                .edges
                .iter()
                .all(|e| e.form != IncludeForm::Angle || e.resolved),
            "unresolved angle edges must stay excluded"
        );
        // Resolved edges unchanged.
        assert!(
            graph
                .edges
                .iter()
                .filter(|e| e.resolved)
                .all(|e| e.to.is_some()),
            "resolved edges keep their to_path"
        );

        // The two z.c → null duplicates survive with distinct spans.
        let misses: Vec<&IncludeEdgeDto> = graph
            .edges
            .iter()
            .filter(|e| e.from == "z.c" && !e.resolved)
            .collect();
        assert_eq!(misses.len(), 2, "duplicates must be preserved");
        assert!(misses.iter().all(|e| e.to.is_none()));
        let mut miss_spans: Vec<_> = misses.iter().map(|e| e.span).collect();
        miss_spans.sort();
        assert_ne!(miss_spans[0], miss_spans[1], "distinguishable by span");

        // Sort pin: (from, to) ascending with None BEFORE Some(_) among the
        // same from. z.c's edges must therefore be [null-miss, null-miss,
        // "shared.h"].
        let z_edges: Vec<Option<&str>> = graph
            .edges
            .iter()
            .filter(|e| e.from == "z.c")
            .map(|e| e.to.as_deref())
            .collect();
        assert_eq!(
            z_edges,
            [None, None, Some("shared.h")],
            "Option<String> Ord puts None before every Some(_)"
        );
        // Whole-edge-list non-decreasing under the same comparator.
        let keys: Vec<(&str, Option<&str>)> = graph
            .edges
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_deref()))
            .collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys);
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
        let graphs = doc.graphs.expect("graphs present");
        assert!(graphs.include_graph.nodes.is_empty() && graphs.include_graph.edges.is_empty());
        // call_graph ALWAYS Some — empty vec on a zero-edge scan.
        assert!(
            graphs
                .call_graph
                .as_ref()
                .is_some_and(|cg| cg.edges.is_empty()),
            "call_graph must be Some(empty), got {graphs:?}"
        );
        // components ALWAYS Some — empty vec on a zero-file scan.
        assert!(
            doc.components.as_ref().is_some_and(Vec::is_empty),
            "components must be Some(empty), got {:?}",
            doc.components
        );
    }

    /// call_graph assembly: a genuine cross-file Direct call maps field-for-
    /// field onto CallEdge (confidence locked LOW via the constructor), and
    /// the aggregator re-sorts by `(from_function, to_name, from_path)`.
    #[test]
    fn call_graph_maps_cross_file_edges_and_reorders() {
        // a.c: two callers; b.c defines both callees. Producer emits per
        // unit/function walk; aggregator owns final document order.
        let mut caller_a = func("a.c", "zcaller", 4, 1);
        caller_a.calls = vec![direct_call("helper", 6, 5)];
        let mut caller_b = func("a.c", "acaller", 30, 1);
        caller_b.calls = vec![direct_call("util", 32, 5)];
        let mut a = unit("a.c");
        a.functions = vec![caller_b, caller_a]; // deliberately unsorted

        let mut helper_def = func("b.c", "helper", 2, 1);
        helper_def.record.cyclomatic_complexity = 3;
        let mut util_def = func("b.c", "util", 10, 1);
        util_def.record.cyclomatic_complexity = 3;
        let mut b = unit("b.c");
        b.functions = vec![helper_def, util_def];

        // Intra-file call (a.c defines its own local_fn): must NOT appear.
        let mut self_contained = func("c.c", "lonely", 1, 1);
        self_contained.calls = vec![direct_call("local_fn", 3, 5)];
        let mut local = func("c.c", "local_fn", 8, 1);
        local.record.cyclomatic_complexity = 3;
        let mut c = unit("c.c");
        c.functions = vec![self_contained, local];

        let doc = assemble(&[a, b, c], &[], &prov());
        let call_graph = doc
            .graphs
            .as_ref()
            .expect("graphs present")
            .call_graph
            .clone()
            .expect("call_graph always Some");

        let keys: Vec<(String, String, String)> = call_graph
            .edges
            .iter()
            .map(|e| {
                (
                    e.from_function.clone(),
                    e.to_name.clone(),
                    e.from_path.clone(),
                )
            })
            .collect();
        assert_eq!(
            keys,
            [
                (
                    "a.c#acaller#30:1".to_string(),
                    "util".to_string(),
                    "a.c".to_string()
                ),
                (
                    "a.c#zcaller#4:1".to_string(),
                    "helper".to_string(),
                    "a.c".to_string()
                ),
            ],
            "(from_function, to_name, from_path) ascending"
        );

        let edge = &call_graph.edges[0];
        assert_eq!(edge.confidence, "LOW");
        assert_eq!(edge.to_path.as_deref(), Some("b.c"));
        // Intra-file edge excluded entirely.
        assert!(
            call_graph.edges.iter().all(|e| e.to_name != "local_fn"),
            "file-local calls are omitted in v1"
        );

        // Unit with no cross-file calls ⇒ still Some(empty).
        let solo_doc = assemble(&[unit("solo.c")], &[], &prov());
        let solo_cg = solo_doc.graphs.expect("graphs").call_graph.expect("Some");
        assert!(solo_cg.edges.is_empty());
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

    /// components[]: directory grouping (incl. root "."), density math with
    /// the KLOC==0 guard, finding_count population, rank under density
    /// inversion (higher density beats higher complexity), emission order
    /// == (rank, path_prefix).
    #[test]
    fn components_group_rank_and_density_math() {
        // app/main.c: 1000 code LOC ⇒ kloc 1.0; one modest function.
        let mut app_unit = unit("app/main.c");
        app_unit.file.loc = LocCounts {
            code: 1000,
            comment: 0,
            blank: 0,
        };
        let mut app_fn = func("app/main.c", "run", 4, 1);
        app_fn.record.cyclomatic_complexity = 5;
        app_fn.record.max_loop_depth = 2;
        app_fn.record.alloc_site_count = 3;
        app_unit.functions = vec![app_fn];

        // lib/util.c: 2000 code LOC ⇒ kloc 2.0; HIGH complexity but LOW
        // density inputs.
        let mut lib_unit = unit("lib/util.c");
        lib_unit.file.loc = LocCounts {
            code: 2000,
            comment: 0,
            blank: 0,
        };
        let mut lib_f1 = func("lib/util.c", "big_one", 2, 1);
        lib_f1.record.cyclomatic_complexity = 10;
        lib_f1.record.max_loop_depth = 1;
        let mut lib_f2 = func("lib/util.c", "big_two", 40, 1);
        lib_f2.record.cyclomatic_complexity = 10;
        lib_f2.record.alloc_site_count = 2;
        lib_unit.functions = vec![lib_f1, lib_f2];

        // Root file: zero code LOC ⇒ KLOC==0 guard must zero the densities.
        let mut root_unit = unit("top.c");
        let mut root_fn = func("top.c", "tiny", 1, 1);
        root_fn.record.cyclomatic_complexity = 1;
        root_unit.functions = vec![root_fn];

        // Findings: 2 in app/, 1 in lib/, 0 at root.
        let findings = vec![
            finding("app/main.c", 6, 5),
            finding("app/main.c", 8, 5),
            finding("lib/util.c", 44, 5),
        ];

        let doc = assemble(&[root_unit, lib_unit, app_unit], &findings, &prov());
        let comps = doc.components.expect("components always Some");

        let prefixes: Vec<&str> = comps.iter().map(|c| c.path_prefix.as_str()).collect();
        assert_eq!(
            prefixes,
            ["app", "lib", "."],
            "emission order (rank, path_prefix)"
        );
        let ranks: Vec<u32> = comps.iter().map(|c| c.rank).collect();
        assert_eq!(ranks, [1, 2, 3]);
        // component_id mirrors path_prefix verbatim.
        assert!(
            comps.iter().all(|c| c.component_id == c.path_prefix),
            "component_id == path_prefix"
        );

        // Rank-1 "app": density inversion — complexity_sum 5 < lib's 20, yet
        // finding_density 2.0 > lib's 0.5 wins rank 1.
        let app = &comps[0];
        assert_eq!(app.finding_count, Some(2));
        assert_eq!(app.metrics.complexity_sum, 5.0);
        assert_eq!(app.metrics.max_loop_depth, 2);
        assert_eq!(app.metrics.alloc_site_density, 3.0); // 3 sites / 1.0 kloc
        assert_eq!(app.metrics.finding_density, 2.0); // 2 findings / 1.0 kloc

        let lib = &comps[1];
        assert_eq!(lib.finding_count, Some(1));
        assert_eq!(lib.metrics.complexity_sum, 20.0);
        assert_eq!(lib.metrics.max_loop_depth, 1);
        assert_eq!(lib.metrics.alloc_site_density, 1.0); // 2 / 2.0
        assert_eq!(lib.metrics.finding_density, 0.5); // 1 / 2.0

        // Root ".": KLOC==0 ⇒ densities guarded to 0.0 (never NaN/inf);
        // finding_count still populated as Some(0).
        let root = &comps[2];
        assert_eq!(root.path_prefix, ".");
        assert_eq!(root.finding_count, Some(0));
        assert_eq!(root.metrics.complexity_sum, 1.0);
        assert_eq!(root.metrics.max_loop_depth, 0);
        assert_eq!(root.metrics.alloc_site_density, 0.0);
        assert_eq!(root.metrics.finding_density, 0.0);
    }

    /// §1 ownership row for `stats.distributions`: the `(metric, bucket)`
    /// comparator is proven on synthetic entries even though no producer
    /// populates the field yet (reserved-for-future).
    #[test]
    fn sort_distributions_orders_by_metric_then_bucket() {
        let mut entries = vec![
            DistributionEntry {
                metric: "length_lines".to_string(),
                bucket: "11-20".to_string(),
                count: 3,
            },
            DistributionEntry {
                metric: "cyclomatic_complexity".to_string(),
                bucket: "6-10".to_string(),
                count: 2,
            },
            DistributionEntry {
                metric: "length_lines".to_string(),
                bucket: "1-10".to_string(),
                count: 1,
            },
            DistributionEntry {
                metric: "cyclomatic_complexity".to_string(),
                bucket: "1-5".to_string(),
                count: 5,
            },
        ];
        sort_distributions(&mut entries);
        let keys: Vec<(String, String)> = entries
            .iter()
            .map(|e| (e.metric.clone(), e.bucket.clone()))
            .collect();
        assert_eq!(
            keys,
            [
                ("cyclomatic_complexity".to_string(), "1-5".to_string()),
                ("cyclomatic_complexity".to_string(), "6-10".to_string()),
                ("length_lines".to_string(), "1-10".to_string()),
                ("length_lines".to_string(), "11-20".to_string()),
            ]
        );
        // Counts travel with their buckets.
        assert_eq!(entries[0].count, 5);
        assert_eq!(entries[3].count, 3);
    }

    fn prov() -> Provenance {
        Provenance {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: 7,
            tool_version: "0.1.0".to_string(),
        }
    }
}
