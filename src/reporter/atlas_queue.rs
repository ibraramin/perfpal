//! Story-only whole-repo KV3 profile queue (P7, `docs/plans/atlas-s4.md`).
//!
//! The queue is a **projection** of the authoritative
//! [`crate::reporter::views::complete_profile_queue`] ordering: it never
//! re-ranks, re-sorts or recomputes a candidate. Only the first
//! [`MAX_ROWS`] production, non-demoted rows are copied; every other row is
//! represented by the counts so a consumer can honestly say "showing 12 of N".
//!
//! Story-only: [`crate::reporter::atlas_layout_json`] is the only caller, so
//! normal scans keep byte-identical output.

use serde::Serialize;

use crate::reporter::views;
use crate::schema::MetricsDocument;

/// `queue_version` — bumped only when the shape of the profile block changes.
pub(crate) const QUEUE_VERSION: u8 = 1;

/// Maximum number of ranked rows carried into the story.
const MAX_ROWS: usize = 12;

/// The story-only `profile` block as emitted into `atlas-layout.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ProfileQueue {
    pub(crate) queue_version: u8,
    /// Every function in the authoritative queue (demoted rows included).
    pub(crate) total_ranked: usize,
    /// Rows whose file is production code.
    pub(crate) production_ranked: usize,
    /// Production, non-demoted rows (the only eligible candidates).
    pub(crate) candidate_total: usize,
    /// Production rows excluded by a demotion (`startup` / `no-loop`).
    pub(crate) demoted_production: usize,
    /// Rows actually carried: `min(candidate_total, MAX_ROWS)`.
    pub(crate) shown: usize,
    /// The first `shown` eligible rows, in authoritative queue order.
    pub(crate) rows: Vec<ProfileRow>,
}

/// One profile row, copied verbatim from the authoritative queue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ProfileRow {
    /// 1-based position in the complete authoritative queue (demoted rows
    /// included), not a re-numbered story rank.
    pub(crate) rank: usize,
    /// Canonical IR identity (`path#name#line:col`) from `stats.functions[]`.
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) path_class: String,
    pub(crate) production: bool,
    pub(crate) cost: f64,
    pub(crate) cost_text: String,
    pub(crate) unknown_trip_loops: u32,
    pub(crate) known_trip_value: Option<u64>,
    /// Empty for every carried row (only non-demoted rows are eligible).
    pub(crate) demotions: Vec<String>,
}

/// Build the story queue from the authoritative KV3 ordering.
#[must_use]
pub(crate) fn build(doc: &MetricsDocument) -> ProfileQueue {
    let queue = views::complete_profile_queue(doc);
    let total_ranked = queue.len();
    let production_ranked = queue.iter().filter(|row| row.production).count();
    let candidate_total = queue
        .iter()
        .filter(|row| row.production && !row.is_demoted())
        .count();
    let demoted_production = queue
        .iter()
        .filter(|row| row.production && row.is_demoted())
        .count();

    let rows: Vec<ProfileRow> = queue
        .iter()
        .filter(|row| row.production && !row.is_demoted())
        .take(MAX_ROWS)
        .map(|row| {
            let function = &doc.stats.functions[row.function_index];
            ProfileRow {
                rank: row.rank,
                id: function.id.clone(),
                name: row.function.clone(),
                path: row.path.clone(),
                line: function.span.start_line as usize,
                path_class: row.path_class.clone(),
                production: row.production,
                cost: row.cost,
                cost_text: row.cost_anchor.clone(),
                unknown_trip_loops: row.unknown_trip_loops,
                known_trip_value: row.known_trip_value,
                demotions: row
                    .demotions
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            }
        })
        .collect();
    let shown = rows.len();

    ProfileQueue {
        queue_version: QUEUE_VERSION,
        total_ranked,
        production_ranked,
        candidate_total,
        demoted_production,
        shown,
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IncludeCounts, LocCounts, ParseStatus, Span};
    use crate::schema::{
        FunctionStats, GraphsSection, IncludeGraph, MetricsDocument, ParseHealthEntry, ScanInfo,
        StatsSection, ToolInfo,
    };

    fn file(path: &str, class: &str, function_count: u32) -> crate::schema::FileEntry {
        crate::schema::FileEntry {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts {
                code: 10,
                comment: 0,
                blank: 0,
            },
            function_count,
            global_var_count: 0,
            include_counts: IncludeCounts {
                quoted: 0,
                angle: 0,
            },
            parse_health: ParseHealthEntry {
                status: ParseStatus::Ok,
                error_regions: None,
                error_node_count: 0,
                resolver: None,
            },
            path_class: Some(class.to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn function(
        path: &str,
        name: &str,
        line: u32,
        cost: u64,
        loops: u32,
        entry: Option<&str>,
        unknown_trips: u32,
        known_trip: Option<u64>,
    ) -> FunctionStats {
        FunctionStats {
            id: format!("{path}#{name}#{line}:1"),
            name: name.to_string(),
            path: path.to_string(),
            span: Span {
                start_line: line,
                start_col: 1,
                end_line: line + 5,
                end_col: 1,
            },
            param_count: 0,
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            max_loop_depth: loops,
            alloc_site_count: 0,
            length_lines: 6,
            call_fanout: Some(0),
            call_fanin: Some(0),
            call_depth: Some(0),
            is_recursive: Some(false),
            idiom_hash: None,
            error_path: Some(false),
            alloc_family: None,
            cfg_gated: None,
            entry_kind: entry.map(str::to_string),
            entry_confidence: None,
            reachable_from_entry: None,
            function_cost_v2: Some(cost),
            unknown_trip_loops: unknown_trips,
            trip_count_source: None,
            known_trip_value: known_trip,
        }
    }

    /// `eligible` production loop-bearing functions (cost descending), plus
    /// one demoted production row (`main`, no loop) and one non-production
    /// row (`tests/test.c`).
    fn fixture(eligible: usize) -> MetricsDocument {
        let mut functions = Vec::new();
        for index in 0..eligible {
            functions.push(function(
                "src/prod.c",
                &format!("f{index:02}"),
                100 + index as u32,
                1_000 - index as u64,
                1,
                None,
                if index == 0 { 2 } else { 0 },
                if index == 0 { Some(3) } else { None },
            ));
        }
        functions.push(function(
            "src/prod.c",
            "slow_start",
            50,
            5_000,
            0,
            Some("main"),
            0,
            None,
        ));
        functions.push(function(
            "tests/test.c",
            "test_case",
            10,
            9_000,
            1,
            None,
            0,
            None,
        ));

        MetricsDocument {
            schema_version: "1.7".to_string(),
            tool: ToolInfo {
                name: "perfpal".to_string(),
                version: "test".to_string(),
            },
            scan: ScanInfo {
                root_path: ".".to_string(),
                started_at: "2026-10-22T00:00:00Z".to_string(),
                duration_ms: None,
                languages: vec!["c".to_string()],
                file_count: 2,
                loc_total: None,
            },
            parse_health_summary: None,
            files: vec![
                file("src/prod.c", "production", eligible as u32 + 1),
                file("tests/test.c", "test", 1),
            ],
            stats: StatsSection {
                functions,
                distributions: None,
            },
            findings: Vec::new(),
            graphs: Some(GraphsSection {
                include_graph: IncludeGraph {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                },
                call_graph: None,
            }),
            components: None,
        }
    }

    fn eligible_rows(doc: &MetricsDocument) -> Vec<views::Kv3Row> {
        views::complete_profile_queue(doc)
            .into_iter()
            .filter(|row| row.production && !row.is_demoted())
            .collect()
    }

    #[test]
    fn rows_match_the_authoritative_queue_order() {
        let doc = fixture(3);
        let queue = build(&doc);
        let expected = eligible_rows(&doc);
        assert_eq!(queue.rows.len(), expected.len());
        for (row, source) in queue.rows.iter().zip(expected.iter()) {
            let function = &doc.stats.functions[source.function_index];
            assert_eq!(row.rank, source.rank);
            assert_eq!(row.id, function.id);
            assert_eq!(row.name, source.function);
            assert_eq!(row.path, source.path);
            assert_eq!(row.line, function.span.start_line as usize);
            assert_eq!(row.path_class, source.path_class);
            assert_eq!(row.cost, source.cost);
            assert_eq!(row.cost_text, source.cost_anchor);
            assert_eq!(row.unknown_trip_loops, source.unknown_trip_loops);
            assert_eq!(row.known_trip_value, source.known_trip_value);
            assert!(row.production);
            assert!(row.demotions.is_empty());
        }
        assert_eq!(queue.rows[0].unknown_trip_loops, 2);
        assert_eq!(queue.rows[0].known_trip_value, Some(3));
    }

    #[test]
    fn non_production_rows_are_excluded() {
        let doc = fixture(4);
        let queue = build(&doc);
        assert!(
            queue
                .rows
                .iter()
                .all(|row| row.path != "tests/test.c" && row.production),
            "non-production rows leaked into the story queue"
        );
        assert_eq!(queue.production_ranked, 5);
        assert_eq!(queue.total_ranked, 6);
    }

    #[test]
    fn demoted_production_rows_are_excluded() {
        let doc = fixture(4);
        let queue = build(&doc);
        assert!(
            queue.rows.iter().all(|row| row.name != "slow_start"),
            "demoted production row leaked into the story queue"
        );
        assert_eq!(queue.demoted_production, 1);
        assert_eq!(queue.candidate_total, 4);
    }

    #[test]
    fn rows_are_capped_at_twelve() {
        let doc = fixture(14);
        let queue = build(&doc);
        assert_eq!(queue.candidate_total, 14);
        assert_eq!(queue.shown, 12);
        assert_eq!(queue.rows.len(), 12);
    }

    #[test]
    fn counts_reconcile() {
        let doc = fixture(14);
        let queue = build(&doc);
        assert!(queue.shown <= 12);
        assert!(queue.candidate_total >= queue.shown);
        assert!(queue.production_ranked >= queue.candidate_total);
        assert!(queue.total_ranked >= queue.production_ranked);
        assert_eq!(
            queue.production_ranked,
            queue.candidate_total + queue.demoted_production
        );
        assert_eq!(queue.shown, queue.rows.len());
    }

    #[test]
    fn top_row_is_the_first_eligible_authoritative_row() {
        let doc = fixture(14);
        let queue = build(&doc);
        let expected = eligible_rows(&doc);
        let first = expected.first().expect("fixture must have an eligible row");
        let function = &doc.stats.functions[first.function_index];
        assert_eq!(queue.rows[0].id, function.id);
        assert_eq!(queue.rows[0].rank, first.rank);
    }

    #[test]
    fn build_is_deterministic() {
        let doc = fixture(14);
        let first = build(&doc);
        let second = build(&doc);
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
    }
}
