//! Atlas entry-flow view model (plan `atlas-build` step 2).
//!
//! Pure, deterministic derivation over a finished [`MetricsDocument`]: no I/O,
//! no clock, no randomness, no hash-map iteration. Every ordering has a total
//! tie-break so a double build over the same document is identical.
//!
//! This module owns the chart-side derivations recorded in
//! `docs/research/atlas-data-gap-audit.md` §5 — entry election + alternatives,
//! structural adjacency and confidence mix, BFS depth/parent/reachability,
//! node/edge caps and aggregation, file cost (v2 with v1 fallback), component
//! grouping, dominant finding, top functions, the representative path trace,
//! blast radius and coverage counts. No schema field is added or changed.
//!
//! Reachability here is **estimated structural reachability** over the
//! default-tier call graph (LOW name-based + MEDIUM semantic); it is
//! deliberately not the schema's resolved-only `reachable_from_entry`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::analyzer::Severity;
use crate::ir::ParseStatus;
use crate::reporter::views::{CostScale, ScaleOrdering, function_value, scale_of};
use crate::schema::{FunctionStats, MetricsDocument};

/// Maximum number of drawn file nodes (mandatory nodes may exceed it).
pub const NODE_CAP: usize = 20;
/// Maximum number of drawn file-pair edges (forced path edges may exceed it).
pub const EDGE_CAP: usize = 55;
/// Last depth column before deeper files fold into it.
pub const DEPTH_COLUMN_CAP: u32 = 8;
/// Unreached files are only filled into spare slots up to this depth.
pub const UNREACHED_COLUMN_MAX_DEPTH: u32 = 2;

/// Entry-election policy (M-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPolicy {
    ProductionMain,
    ProductionScript,
    LibraryRoot,
    NoEntry,
}

impl EntryPolicy {
    /// Canonical M-01 policy label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            EntryPolicy::ProductionMain => "production main",
            EntryPolicy::ProductionScript => "production script (Python __main__)",
            EntryPolicy::LibraryRoot => "library root (no production main)",
            EntryPolicy::NoEntry => "no entry",
        }
    }
}

/// The elected entry function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryPoint {
    pub id: String,
    pub path: String,
    pub name: String,
    pub line: u32,
    pub cost: u64,
    pub fan_in: u32,
}

/// Another eligible candidate under the elected policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryAlternative {
    pub path: String,
    pub name: String,
    pub line: u32,
    pub fan_in: u32,
}

/// Entry election result (M-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasEntry {
    pub policy: EntryPolicy,
    pub point: Option<EntryPoint>,
    pub confidence: String,
    pub alternatives: Vec<EntryAlternative>,
}

/// Dominant finding annotation for one file (M-18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasFinding {
    pub severity: Severity,
    pub rule_id: String,
    pub title: String,
    pub count: usize,
}

/// One function inside a file card / node tooltip.
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasFunction {
    pub id: String,
    pub name: String,
    pub line: u32,
    pub cost: u64,
    pub scale: CostScale,
    pub ratio: Option<f64>,
    /// 1-based rank inside this function's cost scale (M-05 anchor).
    pub rank: usize,
    /// Population of this function's cost scale (M-05 anchor).
    pub population: usize,
    /// Nearest-rank median of this function's cost scale (M-05 multiple).
    pub median: Option<f64>,
    pub top_pct: usize,
    pub fan_in: u32,
    /// `max_loop_depth` carried for the JSON story projection.
    pub loop_depth: u32,
    /// `trip_count_source` (absent when the function has no loops).
    pub trip_source: Option<String>,
    /// Loops whose trip count is genuinely unknown.
    pub unknown_trip_loops: u32,
    /// Distinct direct-callee count.
    pub fan_out: u32,
    /// True when the function can reach itself statically.
    pub recursive: bool,
}

/// Per-node finding severity counts (drives the JSON story projection).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AtlasFindingCounts {
    pub hot: usize,
    pub warn: usize,
    pub info: usize,
}

/// One drawn file node.
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasNode {
    pub path: String,
    pub file_name: String,
    pub path_class: String,
    pub production: bool,
    pub component: String,
    pub depth: Option<u32>,
    pub drawn_depth: Option<u32>,
    pub cost: u64,
    pub scale: CostScale,
    pub file_rank: usize,
    pub file_population: usize,
    pub top_pct: usize,
    pub ratio: Option<f64>,
    pub entry: bool,
    pub hottest: bool,
    pub unreached: bool,
    pub partial_parse: bool,
    pub finding: Option<AtlasFinding>,
    pub findings: AtlasFindingCounts,
    pub top_functions: Vec<AtlasFunction>,
}

/// One source file in the production-first file ordering (below-graph brief).
///
/// Unlike [`AtlasNode`], this summary exists for **every** source file, so the
/// weight strip, file cards and weight drill never depend on the canvas cap.
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasFile {
    pub path: String,
    pub file_name: String,
    pub path_class: String,
    pub production: bool,
    pub component: String,
    pub cost: u64,
    pub scale: CostScale,
    pub file_rank: usize,
    pub file_population: usize,
    pub top_pct: usize,
    pub ratio: Option<f64>,
    /// Nearest-rank median of this file's cost scale (M-04 multiple).
    pub median: Option<f64>,
    pub fan_in: u32,
    pub finding: Option<AtlasFinding>,
    pub top_functions: Vec<AtlasFunction>,
}

/// One drawn structural file-pair edge (distinct-edge count, M-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasEdge {
    pub from: String,
    pub to: String,
    pub count: usize,
    pub back: bool,
    pub on_path: bool,
}

/// One aggregated box inside a depth column (M-14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasAggregate {
    pub depth: u32,
    pub count: usize,
    pub incoming_calls: usize,
}

/// One depth column (or the trailing unreached column).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasColumn {
    pub depth: u32,
    pub folded: bool,
    pub unreached: bool,
    pub files: Vec<String>,
    pub aggregated: Vec<AtlasAggregate>,
    pub file_count: usize,
    pub nonproduction_unreached: usize,
}

/// One chart-side component (approximate prefix group).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasComponent {
    pub prefix: String,
    pub cost: u64,
    pub production_cost: u64,
    pub file_count: usize,
    pub production_file_count: usize,
    pub members: Vec<String>,
}

/// One numbered step on the representative path (M-16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasPathStep {
    pub step: usize,
    pub path: String,
    pub depth: u32,
    pub cost: u64,
}

/// Blast-radius counts for the entry file (M-08 / M-09).
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasBlastRadius {
    pub target: Option<String>,
    pub dependents: usize,
    pub top_dependents: Vec<(String, usize)>,
    pub caller_files: usize,
    pub callee_files: usize,
    pub include_edges_total: usize,
    pub include_edges_resolved: usize,
    pub median_dependents: Option<f64>,
    pub dependent_multiple: Option<f64>,
    pub dependent_rank: Option<usize>,
    pub dependent_population: usize,
    pub dependent_top_pct: Option<usize>,
}

// `f64` prevents a derived `Eq`; the required public shape carries it, and
// every ratio here is a finite reverse-include multiple.
impl Eq for AtlasBlastRadius {}

/// Coverage / noise counts (M-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasCoverage {
    pub parse_ok: u32,
    pub parse_partial: u32,
    pub parse_failed: u32,
    pub unresolved_includes: usize,
    pub demoted_files: usize,
    pub unreached_production: usize,
    pub unreached_nonproduction: usize,
    pub files_without_functions: usize,
    pub partial_parse_files: usize,
}

/// Cap / aggregation counts (M-14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasCaps {
    pub nodes_shown: usize,
    pub reachable_files: usize,
    pub aggregated_files: usize,
    pub edges_drawn: usize,
    pub edge_pairs_total: usize,
    pub depth_folded: usize,
    pub reachable_shown: usize,
}

/// Estimated structural reachability counts (M-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasReachability {
    pub reachable_files: usize,
    pub source_files: usize,
}

/// Distinct structural-edge confidence mix (M-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeConfidenceMix {
    pub medium: usize,
    pub low: usize,
    pub high: usize,
}

/// Page-header scope counts and the global cost summary (M-17 / M-04).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtlasTotals {
    pub total_cost: u64,
    pub median_function_cost: Option<f64>,
    pub function_count: usize,
    pub file_count: usize,
    pub finding_count: usize,
}

// `f64` prevents a derived `Eq`; the required public shape carries it, and
// every value here is a finite cost ratio, so the marker promise holds.
impl Eq for AtlasTotals {}

/// State flags driving the fixed page shape (A10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasState {
    pub empty: bool,
    pub no_entry: bool,
    pub library: bool,
    pub all_test: bool,
    pub capped: bool,
    pub partial_parse: bool,
    pub single_file: bool,
}

/// The complete Atlas view model.
#[derive(Debug, Clone, PartialEq)]
pub struct AtlasView {
    pub entry: AtlasEntry,
    pub hottest: Option<String>,
    pub hottest_node: Option<AtlasNode>,
    pub nodes: Vec<AtlasNode>,
    /// Every source file (production-first, cost desc, fan-in desc, path asc).
    pub files: Vec<AtlasFile>,
    pub edges: Vec<AtlasEdge>,
    pub columns: Vec<AtlasColumn>,
    pub unreachable_production: Vec<String>,
    pub unreachable_nonproduction: Vec<String>,
    pub path: Vec<AtlasPathStep>,
    pub components: Vec<AtlasComponent>,
    pub blast_radius: AtlasBlastRadius,
    pub coverage: AtlasCoverage,
    pub caps: AtlasCaps,
    pub reachability: AtlasReachability,
    pub edge_mix: EdgeConfidenceMix,
    pub totals: AtlasTotals,
    pub state: AtlasState,
}

// ---------------------------------------------------------------------------
// Internal derivations
// ---------------------------------------------------------------------------

struct FileInfo {
    path: String,
    class: String,
    production: bool,
    cost: u64,
    cost_exact: f64,
    scale: CostScale,
    fan_in: usize,
    rank: usize,
    population: usize,
    top_pct: usize,
    ratio: Option<f64>,
    median: Option<f64>,
}

struct Structural {
    pairs: BTreeMap<(String, String), usize>,
    fan_in: BTreeMap<String, BTreeSet<String>>,
    mix: EdgeConfidenceMix,
}

fn final_segment(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Chart-side approximate component grouping (`domain_of`).
pub(crate) fn domain_of(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if path.starts_with("src/") && parts.len() >= 3 {
        return format!("src/{}", parts[1]);
    }
    if let Some((first, _)) = path.split_once('/') {
        return first.to_string();
    }
    let file = final_segment(path);
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    // A leading `_`/`-` (e.g. `_private.py`) must not yield an empty prefix.
    stem.split(['-', '_'])
        .find(|part| !part.is_empty())
        .unwrap_or(stem)
        .to_string()
}

/// `max(1, round(100·rank/n))`, capped at 100 — identical to
/// [`crate::reporter::views::top_label`]'s arithmetic.
fn top_pct(rank: usize, population: usize) -> usize {
    if population == 0 {
        return 1;
    }
    ((100 * rank + population / 2) / population).clamp(1, 100)
}

pub(crate) fn function_cost_u64(function: &FunctionStats) -> u64 {
    let rounded = crate::schema::round_half_up(function_value(function), 0).max(0);
    u64::try_from(rounded).unwrap_or(u64::MAX)
}

fn function_fan_in(function: &FunctionStats) -> u32 {
    function.call_fanin.unwrap_or(0)
}

fn build_structural(doc: &MetricsDocument, source_paths: &BTreeSet<String>) -> Structural {
    let mut distinct: BTreeMap<(String, String, String, String), String> = BTreeMap::new();
    if let Some(graphs) = &doc.graphs
        && let Some(call_graph) = &graphs.call_graph
    {
        for edge in &call_graph.edges {
            let Some(to_path) = edge.to_path.as_deref() else {
                continue;
            };
            if edge.from_path == to_path {
                continue;
            }
            if !source_paths.contains(&edge.from_path) || !source_paths.contains(to_path) {
                continue;
            }
            distinct.insert(
                (
                    edge.from_function.clone(),
                    edge.to_name.clone(),
                    edge.from_path.clone(),
                    to_path.to_string(),
                ),
                edge.confidence.clone(),
            );
        }
    }

    let mut pairs: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut fan_in: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut mix = EdgeConfidenceMix {
        medium: 0,
        low: 0,
        high: 0,
    };
    for ((_from_function, _to_name, from_path, to_path), confidence) in &distinct {
        *pairs
            .entry((from_path.clone(), to_path.clone()))
            .or_insert(0) += 1;
        fan_in
            .entry(to_path.clone())
            .or_default()
            .insert(from_path.clone());
        match confidence.as_str() {
            "MEDIUM" => mix.medium += 1,
            "LOW" => mix.low += 1,
            "HIGH" => mix.high += 1,
            _ => {}
        }
    }
    Structural { pairs, fan_in, mix }
}

/// Shared per-file cost rule (Atlas file cards, scene aggregation).
///
/// Returns `(display_cost, exact_cost, scale)`: v2 sums `function_cost_v2`
/// directly; v1 sums [`crate::charts::function_cost`] and half-up rounds the
/// displayed integer exactly as [`build_file_infos`] does. `exact_cost` is the
/// unrounded sum (the v2 integer sum for v2 files), used for anchor multiples.
pub(crate) fn file_cost(
    doc: &MetricsDocument,
    indices: &[usize],
) -> (u64, f64, crate::reporter::views::CostScale) {
    use crate::reporter::views::CostScale;
    let all_v2 = indices
        .iter()
        .all(|&index| doc.stats.functions[index].function_cost_v2.is_some());
    let scale = if indices.is_empty() || all_v2 {
        CostScale::V2
    } else {
        CostScale::V1
    };
    match scale {
        CostScale::V2 => {
            let cost = indices
                .iter()
                .filter_map(|&index| doc.stats.functions[index].function_cost_v2)
                .fold(0_u64, u64::saturating_add);
            (cost, cost as f64, scale)
        }
        CostScale::V1 => {
            let sum: f64 = indices
                .iter()
                .map(|&index| crate::charts::function_cost(&doc.stats.functions[index]))
                .sum();
            let rounded = crate::schema::round_half_up(sum, 0).max(0);
            (u64::try_from(rounded).unwrap_or(u64::MAX), sum, scale)
        }
    }
}

fn build_file_infos(
    doc: &MetricsDocument,
    functions_by_path: &BTreeMap<String, Vec<usize>>,
    structural: &Structural,
    v2: &ScaleOrdering,
    v1: &ScaleOrdering,
) -> BTreeMap<String, FileInfo> {
    let mut infos: BTreeMap<String, FileInfo> = BTreeMap::new();
    for file in &doc.files {
        if file.function_count == 0 {
            continue;
        }
        let indices = functions_by_path
            .get(&file.path)
            .cloned()
            .unwrap_or_default();
        // v1 file cost is summed unrounded for the anchor multiple; only the
        // displayed integer is HALF-UP rounded (ScaleOrdering parity).
        let (cost, cost_exact, scale) = file_cost(doc, &indices);
        let class = file
            .path_class
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        infos.insert(
            file.path.clone(),
            FileInfo {
                path: file.path.clone(),
                production: class == "production",
                class,
                cost,
                cost_exact,
                scale,
                fan_in: structural.fan_in.get(&file.path).map_or(0, BTreeSet::len),
                rank: 0,
                population: 0,
                top_pct: 1,
                ratio: None,
                median: None,
            },
        );
    }

    for scale in [CostScale::V2, CostScale::V1] {
        let ordering = if scale == CostScale::V2 { v2 } else { v1 };
        let mut members: Vec<String> = infos
            .values()
            .filter(|info| info.scale == scale)
            .map(|info| info.path.clone())
            .collect();
        members.sort_by(|a, b| {
            let left = &infos[a];
            let right = &infos[b];
            right
                .cost
                .cmp(&left.cost)
                .then_with(|| right.fan_in.cmp(&left.fan_in))
                .then_with(|| left.path.cmp(&right.path))
        });
        let population = members.len();
        let median = ordering.median();
        for (index, path) in members.iter().enumerate() {
            if let Some(info) = infos.get_mut(path) {
                info.rank = index + 1;
                info.population = population;
                info.top_pct = top_pct(index + 1, population);
                info.median = median;
                info.ratio = median.and_then(|value| {
                    if value > 0.0 {
                        Some(info.cost_exact / value)
                    } else {
                        None
                    }
                });
            }
        }
    }

    infos
}

fn main_like_order(left: &FunctionStats, right: &FunctionStats) -> std::cmp::Ordering {
    function_cost_u64(right)
        .cmp(&function_cost_u64(left))
        .then_with(|| function_fan_in(right).cmp(&function_fan_in(left)))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.id.cmp(&right.id))
}

fn library_root_order(left: &FunctionStats, right: &FunctionStats) -> std::cmp::Ordering {
    function_fan_in(right)
        .cmp(&function_fan_in(left))
        .then_with(|| function_cost_u64(right).cmp(&function_cost_u64(left)))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.id.cmp(&right.id))
}

fn alternative(function: &FunctionStats) -> EntryAlternative {
    EntryAlternative {
        path: function.path.clone(),
        name: function.name.clone(),
        line: function.span.start_line,
        fan_in: function_fan_in(function),
    }
}

fn point(function: &FunctionStats) -> EntryPoint {
    EntryPoint {
        id: function.id.clone(),
        path: function.path.clone(),
        name: function.name.clone(),
        line: function.span.start_line,
        cost: function_cost_u64(function),
        fan_in: function_fan_in(function),
    }
}

fn elect_entry(doc: &MetricsDocument, files: &BTreeMap<String, FileInfo>) -> AtlasEntry {
    let eligible = |function: &FunctionStats| -> bool {
        files
            .get(&function.path)
            .is_some_and(|info| info.production)
            && final_segment(&function.path) != "build.rs"
    };
    let mut mains: Vec<&FunctionStats> = doc
        .stats
        .functions
        .iter()
        .filter(|function| function.entry_kind.as_deref() == Some("main") && eligible(function))
        .collect();
    let mut scripts: Vec<&FunctionStats> = doc
        .stats
        .functions
        .iter()
        .filter(|function| function.entry_kind.as_deref() == Some("script") && eligible(function))
        .collect();

    if !mains.is_empty() {
        mains.sort_by(|a, b| main_like_order(a, b));
        let chosen = mains[0];
        return AtlasEntry {
            policy: EntryPolicy::ProductionMain,
            point: Some(point(chosen)),
            confidence: chosen
                .entry_confidence
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            alternatives: mains[1..]
                .iter()
                .map(|function| alternative(function))
                .collect(),
        };
    }
    if !scripts.is_empty() {
        scripts.sort_by(|a, b| main_like_order(a, b));
        let chosen = scripts[0];
        return AtlasEntry {
            policy: EntryPolicy::ProductionScript,
            point: Some(point(chosen)),
            confidence: chosen
                .entry_confidence
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            alternatives: scripts[1..]
                .iter()
                .map(|function| alternative(function))
                .collect(),
        };
    }

    let mut roots: Vec<&FunctionStats> = doc
        .stats
        .functions
        .iter()
        .filter(|function| function.entry_kind.as_deref() == Some("lib-root") && eligible(function))
        .collect();
    if !roots.is_empty() {
        roots.sort_by(|a, b| library_root_order(a, b));
        let chosen = roots[0];
        let threshold = f64::from(function_fan_in(chosen)) * 0.20;
        let alternatives = roots[1..]
            .iter()
            .filter(|function| f64::from(function_fan_in(function)) >= threshold)
            .take(5)
            .map(|function| alternative(function))
            .collect();
        return AtlasEntry {
            policy: EntryPolicy::LibraryRoot,
            point: Some(point(chosen)),
            confidence: chosen
                .entry_confidence
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            alternatives,
        };
    }

    AtlasEntry {
        policy: EntryPolicy::NoEntry,
        point: None,
        confidence: "UNKNOWN".to_string(),
        alternatives: Vec::new(),
    }
}

fn build_adjacency(
    pairs: &BTreeMap<(String, String), usize>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (from, to) in pairs.keys() {
        adjacency
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
    }
    adjacency
}

fn bfs(
    entry: &str,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
    costs: &BTreeMap<String, u64>,
) -> (BTreeMap<String, u32>, BTreeMap<String, String>) {
    let mut depth: BTreeMap<String, u32> = BTreeMap::new();
    let mut parent: BTreeMap<String, String> = BTreeMap::new();
    depth.insert(entry.to_string(), 0);
    let mut queue: VecDeque<String> = VecDeque::from([entry.to_string()]);
    while let Some(node) = queue.pop_front() {
        let next_depth = depth[&node] + 1;
        let Some(neighbors) = adjacency.get(&node) else {
            continue;
        };
        let mut ordered: Vec<&String> = neighbors.iter().collect();
        ordered.sort_by(|a, b| {
            let left = costs.get(a.as_str()).copied().unwrap_or(0);
            let right = costs.get(b.as_str()).copied().unwrap_or(0);
            right.cmp(&left).then_with(|| a.cmp(b))
        });
        for next in ordered {
            if !depth.contains_key(next) {
                depth.insert(next.clone(), next_depth);
                parent.insert(next.clone(), node.clone());
                queue.push_back(next.clone());
            }
        }
    }
    (depth, parent)
}

fn representative_path(
    entry: &str,
    depth: &BTreeMap<String, u32>,
    parent: &BTreeMap<String, String>,
    costs: &BTreeMap<String, u64>,
) -> Vec<String> {
    if depth.len() <= 1 {
        return vec![entry.to_string()];
    }
    let with_child: BTreeSet<&str> = parent.values().map(String::as_str).collect();
    let cost_of = |path: &String| costs.get(path).copied().unwrap_or(0);
    let mut leaves: Vec<&String> = depth
        .keys()
        .filter(|path| path.as_str() != entry && !with_child.contains(path.as_str()))
        .collect();
    let endpoint = if leaves.is_empty() {
        let mut fallback: Vec<&String> =
            depth.keys().filter(|path| path.as_str() != entry).collect();
        fallback.sort_by(|a, b| {
            depth[b.as_str()]
                .cmp(&depth[a.as_str()])
                .then_with(|| cost_of(b).cmp(&cost_of(a)))
                .then_with(|| a.cmp(b))
        });
        fallback.first().copied()
    } else {
        leaves.sort_by(|a, b| {
            cost_of(b)
                .cmp(&cost_of(a))
                .then_with(|| depth[b.as_str()].cmp(&depth[a.as_str()]))
                .then_with(|| a.cmp(b))
        });
        leaves.first().copied()
    };
    let Some(endpoint) = endpoint else {
        return vec![entry.to_string()];
    };
    let mut reversed: Vec<String> = Vec::new();
    let mut node = endpoint.clone();
    while node != entry {
        reversed.push(node.clone());
        match parent.get(&node) {
            Some(next) => node = next.clone(),
            None => break,
        }
    }
    reversed.push(entry.to_string());
    reversed.reverse();
    reversed
}

fn finding_counts(findings: &[&crate::schema::FindingEntry]) -> AtlasFindingCounts {
    let mut counts = AtlasFindingCounts::default();
    for finding in findings {
        match finding.severity {
            Severity::Hot => counts.hot += 1,
            Severity::Warn => counts.warn += 1,
            Severity::Info => counts.info += 1,
        }
    }
    counts
}

fn dominant_finding(findings: &[&crate::schema::FindingEntry]) -> Option<AtlasFinding> {
    if findings.is_empty() {
        return None;
    }
    let mut by_rule: BTreeMap<&str, Vec<&crate::schema::FindingEntry>> = BTreeMap::new();
    for finding in findings {
        by_rule
            .entry(finding.rule_id.as_str())
            .or_default()
            .push(finding);
    }
    let severity_rank = |severity: Severity| match severity {
        Severity::Hot => 0_u8,
        Severity::Warn => 1,
        Severity::Info => 2,
    };
    let mut groups: Vec<(&str, Vec<&crate::schema::FindingEntry>)> = by_rule.into_iter().collect();
    groups.sort_by(|(left_rule, left), (right_rule, right)| {
        let left_severity = left
            .iter()
            .map(|finding| severity_rank(finding.severity))
            .min()
            .unwrap_or(2);
        let right_severity = right
            .iter()
            .map(|finding| severity_rank(finding.severity))
            .min()
            .unwrap_or(2);
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left_severity.cmp(&right_severity))
            .then_with(|| left_rule.cmp(right_rule))
    });
    let (rule_id, mut group) = groups.into_iter().next()?;
    let count = group.len();
    group.sort_by(|a, b| {
        severity_rank(a.severity)
            .cmp(&severity_rank(b.severity))
            .then_with(|| a.finding_id.cmp(&b.finding_id))
    });
    let top = group.first()?;
    Some(AtlasFinding {
        severity: top.severity,
        rule_id: rule_id.to_string(),
        title: top.title.clone(),
        count,
    })
}

fn top_functions(
    doc: &MetricsDocument,
    indices: &[usize],
    v2: &ScaleOrdering,
    v1: &ScaleOrdering,
) -> Vec<AtlasFunction> {
    let mut ordered: Vec<usize> = indices.to_vec();
    ordered.sort_by(|&a, &b| {
        let left = &doc.stats.functions[a];
        let right = &doc.stats.functions[b];
        function_cost_u64(right)
            .cmp(&function_cost_u64(left))
            .then_with(|| function_fan_in(right).cmp(&function_fan_in(left)))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    ordered
        .into_iter()
        .take(3)
        .map(|index| {
            let function = &doc.stats.functions[index];
            let scale = scale_of(function);
            let ordering = if scale == CostScale::V2 { v2 } else { v1 };
            let rank = ordering.rank(&function.id).unwrap_or(1);
            let population = ordering.total();
            let cost = function_cost_u64(function);
            let ratio = ordering.median().and_then(|median| {
                if median > 0.0 {
                    Some(function_value(function) / median)
                } else {
                    None
                }
            });
            AtlasFunction {
                id: function.id.clone(),
                name: function.name.clone(),
                line: function.span.start_line,
                cost,
                scale,
                ratio,
                rank,
                population,
                median: ordering.median(),
                top_pct: top_pct(rank, population),
                fan_in: function_fan_in(function),
                loop_depth: function.max_loop_depth,
                trip_source: function.trip_count_source.clone(),
                unknown_trip_loops: function.unknown_trip_loops,
                fan_out: function.call_fanout.unwrap_or(0),
                recursive: function.is_recursive.unwrap_or(false),
            }
        })
        .collect()
}

fn coverage(
    doc: &MetricsDocument,
    depth: &BTreeMap<String, u32>,
    source_paths: &BTreeSet<String>,
    files: &BTreeMap<String, FileInfo>,
    statuses: &BTreeMap<String, ParseStatus>,
) -> AtlasCoverage {
    let (parse_ok, parse_partial, parse_failed) = match doc.parse_health_summary {
        Some(summary) => (summary.ok, summary.partial, summary.failed),
        None => {
            let mut ok = 0_u32;
            let mut partial = 0_u32;
            let mut failed = 0_u32;
            for file in &doc.files {
                match file.parse_health.status {
                    ParseStatus::Ok => ok += 1,
                    ParseStatus::Partial => partial += 1,
                    ParseStatus::Failed => failed += 1,
                }
            }
            (ok, partial, failed)
        }
    };

    let mut unresolved_includes = 0_usize;
    if let Some(graphs) = &doc.graphs {
        unresolved_includes = graphs
            .include_graph
            .edges
            .iter()
            .filter(|edge| edge.to.is_none() || !edge.resolved)
            .count();
    }

    let demoted_files = source_paths
        .iter()
        .filter(|path| files.get(*path).is_some_and(|info| !info.production))
        .count();
    let unreached: Vec<&String> = source_paths
        .iter()
        .filter(|path| !depth.contains_key(*path))
        .collect();
    let unreached_production = unreached
        .iter()
        .filter(|path| files.get(**path).is_some_and(|info| info.production))
        .count();
    let unreached_nonproduction = unreached.len() - unreached_production;
    let files_without_functions = doc.files.len().saturating_sub(source_paths.len());
    let partial_parse_files = source_paths
        .iter()
        .filter(|path| {
            statuses
                .get(*path)
                .is_some_and(|status| matches!(status, ParseStatus::Partial | ParseStatus::Failed))
        })
        .count();

    AtlasCoverage {
        parse_ok,
        parse_partial,
        parse_failed,
        unresolved_includes,
        demoted_files,
        unreached_production,
        unreached_nonproduction,
        files_without_functions,
        partial_parse_files,
    }
}

fn components(files: &BTreeMap<String, FileInfo>) -> Vec<AtlasComponent> {
    struct Accum {
        cost: u64,
        production_cost: u64,
        file_count: usize,
        production_file_count: usize,
        members: Vec<String>,
    }
    let mut grouped: BTreeMap<String, Accum> = BTreeMap::new();
    for info in files.values() {
        let prefix = domain_of(&info.path);
        let entry = grouped.entry(prefix).or_insert_with(|| Accum {
            cost: 0,
            production_cost: 0,
            file_count: 0,
            production_file_count: 0,
            members: Vec::new(),
        });
        entry.cost = entry.cost.saturating_add(info.cost);
        entry.file_count += 1;
        if info.production {
            entry.production_cost = entry.production_cost.saturating_add(info.cost);
            entry.production_file_count += 1;
        }
        entry.members.push(info.path.clone());
    }
    let mut result: Vec<AtlasComponent> = grouped
        .into_iter()
        .map(|(prefix, accum)| AtlasComponent {
            prefix,
            cost: accum.cost,
            production_cost: accum.production_cost,
            file_count: accum.file_count,
            production_file_count: accum.production_file_count,
            members: accum.members,
        })
        .collect();
    result.sort_by(|a, b| {
        b.production_cost
            .cmp(&a.production_cost)
            .then_with(|| b.cost.cmp(&a.cost))
            .then_with(|| a.prefix.cmp(&b.prefix))
    });
    result
}

fn blast_radius(
    doc: &MetricsDocument,
    target: Option<&str>,
    structural: &Structural,
    source_paths: &BTreeSet<String>,
) -> AtlasBlastRadius {
    let mut include_edges_total = 0_usize;
    let mut include_edges_resolved = 0_usize;
    // Reverse-include sets over EVERY source file (zeros included) feed the
    // M-08 median; per-from occurrence counts feed the top-5 list. `from` may
    // be any scanned file (M-08's `from in files`), not only a source file.
    let scanned: BTreeSet<&str> = doc.files.iter().map(|file| file.path.as_str()).collect();
    let mut reverse: BTreeMap<&str, BTreeSet<&str>> = source_paths
        .iter()
        .map(|path| (path.as_str(), BTreeSet::new()))
        .collect();
    let mut target_edge_counts: BTreeMap<String, usize> = BTreeMap::new();
    if let Some(graphs) = &doc.graphs {
        include_edges_total = graphs.include_graph.edges.len();
        for edge in &graphs.include_graph.edges {
            if edge.resolved {
                include_edges_resolved += 1;
            }
            if !edge.resolved {
                continue;
            }
            let Some(to) = edge.to.as_deref() else {
                continue;
            };
            if edge.from == to
                || !scanned.contains(edge.from.as_str())
                || !source_paths.contains(to)
            {
                continue;
            }
            reverse.entry(to).or_default().insert(edge.from.as_str());
            if target == Some(to) {
                *target_edge_counts.entry(edge.from.clone()).or_insert(0) += 1;
            }
        }
    }

    let dependent_count = target
        .and_then(|target| reverse.get(target))
        .map_or(0, BTreeSet::len);
    let mut top_dependents: Vec<(String, usize)> = target_edge_counts.into_iter().collect();
    top_dependents.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_dependents.truncate(5);

    let median_values: Vec<f64> = reverse
        .values()
        .map(|sources| sources.len() as f64)
        .collect();
    let median_dependents = crate::reporter::views::median(&median_values);
    let dependent_multiple = match median_dependents {
        Some(median) if target.is_some() && median > 0.0 => Some(dependent_count as f64 / median),
        _ => None,
    };

    let mut ranked: Vec<(&str, usize)> = reverse
        .iter()
        .map(|(path, sources)| (*path, sources.len()))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let dependent_population = source_paths.len();
    let dependent_rank = target.and_then(|target| {
        ranked
            .iter()
            .position(|(path, _)| *path == target)
            .map(|index| index + 1)
    });
    let dependent_top_pct = dependent_rank.map(|rank| top_pct(rank, dependent_population));

    let (caller_files, callee_files) = match target {
        Some(target) => {
            let callers: BTreeSet<&str> = structural
                .pairs
                .keys()
                .filter(|(_, to)| to == target)
                .map(|(from, _)| from.as_str())
                .collect();
            let callees: BTreeSet<&str> = structural
                .pairs
                .keys()
                .filter(|(from, _)| from == target)
                .map(|(_, to)| to.as_str())
                .collect();
            (callers.len(), callees.len())
        }
        None => (0, 0),
    };

    AtlasBlastRadius {
        target: target.map(str::to_string),
        dependents: dependent_count,
        top_dependents,
        caller_files,
        callee_files,
        include_edges_total,
        include_edges_resolved,
        median_dependents,
        dependent_multiple,
        dependent_rank,
        dependent_population,
        dependent_top_pct,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

struct NodeContext<'a> {
    doc: &'a MetricsDocument,
    depth: &'a BTreeMap<String, u32>,
    statuses: &'a BTreeMap<String, ParseStatus>,
    findings_by_path: &'a BTreeMap<&'a str, Vec<&'a crate::schema::FindingEntry>>,
    functions_by_path: &'a BTreeMap<String, Vec<usize>>,
    v2: &'a ScaleOrdering,
    v1: &'a ScaleOrdering,
    unreached_depth: u32,
    unreached_column_shown: bool,
    entry_path: Option<&'a str>,
    hottest_path: Option<&'a str>,
}

impl NodeContext<'_> {
    fn build(&self, info: &FileInfo) -> AtlasNode {
        let path = info.path.as_str();
        let file_depth = self.depth.get(path).copied();
        let drawn_depth = match file_depth {
            Some(value) => Some(value.min(DEPTH_COLUMN_CAP)),
            None if self.unreached_column_shown => Some(self.unreached_depth),
            None => None,
        };
        let indices = self
            .functions_by_path
            .get(path)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let empty_findings: Vec<&crate::schema::FindingEntry> = Vec::new();
        let node_findings = self
            .findings_by_path
            .get(path)
            .map(Vec::as_slice)
            .unwrap_or(&empty_findings);
        AtlasNode {
            path: info.path.clone(),
            file_name: final_segment(&info.path).to_string(),
            path_class: info.class.clone(),
            production: info.production,
            component: domain_of(&info.path),
            depth: file_depth,
            drawn_depth,
            cost: info.cost,
            scale: info.scale,
            file_rank: info.rank,
            file_population: info.population,
            top_pct: info.top_pct,
            ratio: info.ratio,
            entry: self.entry_path == Some(path),
            hottest: self.hottest_path == Some(path),
            unreached: file_depth.is_none(),
            partial_parse: self
                .statuses
                .get(path)
                .is_some_and(|status| matches!(status, ParseStatus::Partial | ParseStatus::Failed)),
            finding: dominant_finding(node_findings),
            findings: finding_counts(node_findings),
            top_functions: top_functions(self.doc, indices, self.v2, self.v1),
        }
    }

    /// All-file summary for the below-graph brief (weight strip, cards, drill).
    fn build_file(&self, info: &FileInfo) -> AtlasFile {
        let path = info.path.as_str();
        let indices = self
            .functions_by_path
            .get(path)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let empty_findings: Vec<&crate::schema::FindingEntry> = Vec::new();
        AtlasFile {
            path: info.path.clone(),
            file_name: final_segment(&info.path).to_string(),
            path_class: info.class.clone(),
            production: info.production,
            component: domain_of(&info.path),
            cost: info.cost,
            scale: info.scale,
            file_rank: info.rank,
            file_population: info.population,
            top_pct: info.top_pct,
            ratio: info.ratio,
            median: info.median,
            fan_in: u32::try_from(info.fan_in).unwrap_or(u32::MAX),
            finding: dominant_finding(
                self.findings_by_path
                    .get(path)
                    .map(Vec::as_slice)
                    .unwrap_or(&empty_findings),
            ),
            top_functions: top_functions(self.doc, indices, self.v2, self.v1),
        }
    }
}

/// Build the complete deterministic Atlas view model.
#[must_use]
pub fn build_atlas_view(doc: &MetricsDocument) -> AtlasView {
    let source_paths: BTreeSet<String> = doc
        .files
        .iter()
        .filter(|file| file.function_count > 0)
        .map(|file| file.path.clone())
        .collect();
    let statuses: BTreeMap<String, ParseStatus> = doc
        .files
        .iter()
        .map(|file| (file.path.clone(), file.parse_health.status))
        .collect();
    let mut functions_by_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, function) in doc.stats.functions.iter().enumerate() {
        functions_by_path
            .entry(function.path.clone())
            .or_default()
            .push(index);
    }

    let v2 = ScaleOrdering::build(&doc.stats.functions, CostScale::V2);
    let v1 = ScaleOrdering::build(&doc.stats.functions, CostScale::V1);
    let structural = build_structural(doc, &source_paths);
    let files = build_file_infos(doc, &functions_by_path, &structural, &v2, &v1);
    let entry = elect_entry(doc, &files);

    let costs: BTreeMap<String, u64> = files
        .iter()
        .map(|(path, info)| (path.clone(), info.cost))
        .collect();
    let adjacency = build_adjacency(&structural.pairs);
    let (depth, parent) = match entry.point.as_ref() {
        Some(point) => bfs(&point.path, &adjacency, &costs),
        None => (BTreeMap::new(), BTreeMap::new()),
    };

    // M-04 rank 1 is global: it exists even when no entry is electable.
    let hottest = files
        .values()
        .min_by(|a, b| {
            b.cost
                .cmp(&a.cost)
                .then_with(|| b.fan_in.cmp(&a.fan_in))
                .then_with(|| a.path.cmp(&b.path))
        })
        .map(|info| info.path.clone());

    let path_files = match entry.point.as_ref() {
        Some(point) if !depth.is_empty() => {
            representative_path(&point.path, &depth, &parent, &costs)
        }
        _ => Vec::new(),
    };

    let mut selected: BTreeSet<String> = BTreeSet::new();
    let mut max_depth = 0_u32;
    if let Some(point) = &entry.point {
        selected.insert(point.path.clone());
        for path in &path_files {
            selected.insert(path.clone());
        }
        if let Some(path) = &hottest {
            selected.insert(path.clone());
        }
        max_depth = depth.values().copied().max().unwrap_or(0);

        let mut reachable: Vec<&String> = depth.keys().collect();
        reachable.sort_by(|a, b| {
            let left = costs.get(a.as_str()).copied().unwrap_or(0);
            let right = costs.get(b.as_str()).copied().unwrap_or(0);
            right
                .cmp(&left)
                .then_with(|| depth[a.as_str()].cmp(&depth[b.as_str()]))
                .then_with(|| a.cmp(b))
        });
        for path in reachable {
            if selected.len() >= NODE_CAP {
                break;
            }
            selected.insert(path.clone());
        }

        if max_depth <= UNREACHED_COLUMN_MAX_DEPTH {
            let mut unreached: Vec<&String> = files
                .values()
                .filter(|info| info.production && !depth.contains_key(&info.path))
                .map(|info| &info.path)
                .collect();
            unreached.sort_by(|a, b| {
                let left = costs.get(a.as_str()).copied().unwrap_or(0);
                let right = costs.get(b.as_str()).copied().unwrap_or(0);
                right.cmp(&left).then_with(|| a.cmp(b))
            });
            for path in unreached {
                if selected.len() >= NODE_CAP {
                    break;
                }
                selected.insert(path.clone());
            }
        }
    }

    let mut shown_unreached: Vec<String> = selected
        .iter()
        .filter(|path| !depth.contains_key(*path))
        .cloned()
        .collect();
    shown_unreached.sort_by(|a, b| {
        let left = costs.get(a).copied().unwrap_or(0);
        let right = costs.get(b).copied().unwrap_or(0);
        right.cmp(&left).then_with(|| a.cmp(b))
    });
    let last_reached_column = max_depth.min(DEPTH_COLUMN_CAP);
    let unreached_depth = last_reached_column + 1;
    let unreachable_production = sorted_unreachable(&files, &depth, true);
    let unreachable_nonproduction = sorted_unreachable(&files, &depth, false);
    let unreached_column_shown =
        !shown_unreached.is_empty() || !unreachable_nonproduction.is_empty();

    let mut findings_by_path: BTreeMap<&str, Vec<&crate::schema::FindingEntry>> = BTreeMap::new();
    for finding in &doc.findings {
        findings_by_path
            .entry(finding.path.as_str())
            .or_default()
            .push(finding);
    }

    let context = NodeContext {
        doc,
        depth: &depth,
        statuses: &statuses,
        findings_by_path: &findings_by_path,
        functions_by_path: &functions_by_path,
        v2: &v2,
        v1: &v1,
        unreached_depth,
        // No entry means no drawn column, so the undrawn HOTTEST node must not
        // pretend to occupy the unreached column.
        unreached_column_shown: entry.point.is_some() && unreached_column_shown,
        entry_path: entry.point.as_ref().map(|point| point.path.as_str()),
        hottest_path: hottest.as_deref(),
    };

    let mut file_summaries: Vec<AtlasFile> = files
        .values()
        .map(|info| context.build_file(info))
        .collect();
    file_summaries.sort_by(|a, b| {
        b.production
            .cmp(&a.production)
            .then_with(|| b.cost.cmp(&a.cost))
            .then_with(|| b.fan_in.cmp(&a.fan_in))
            .then_with(|| a.path.cmp(&b.path))
    });

    let mut nodes: Vec<AtlasNode> = Vec::new();
    for path in &selected {
        if let Some(info) = files.get(path) {
            nodes.push(context.build(info));
        }
    }
    nodes.sort_by(|a, b| {
        a.drawn_depth
            .unwrap_or(u32::MAX)
            .cmp(&b.drawn_depth.unwrap_or(u32::MAX))
            .then_with(|| b.cost.cmp(&a.cost))
            .then_with(|| a.path.cmp(&b.path))
    });
    let hottest_node = hottest
        .as_ref()
        .and_then(|path| files.get(path))
        .map(|info| context.build(info));

    // Columns (reached depth buckets + the trailing unreached column).
    let mut columns: Vec<AtlasColumn> = Vec::new();
    if entry.point.is_some() {
        for column_depth in 0..=last_reached_column {
            let folded = column_depth == DEPTH_COLUMN_CAP && max_depth > DEPTH_COLUMN_CAP;
            let in_bucket = |value: u32| {
                if folded {
                    value >= DEPTH_COLUMN_CAP
                } else {
                    value == column_depth
                }
            };
            let mut files_in_column: Vec<String> = selected
                .iter()
                .filter(|path| depth.get(*path).is_some_and(|value| in_bucket(*value)))
                .cloned()
                .collect();
            files_in_column.sort_by(|a, b| {
                let left = costs.get(a).copied().unwrap_or(0);
                let right = costs.get(b).copied().unwrap_or(0);
                right.cmp(&left).then_with(|| a.cmp(b))
            });

            let omitted: BTreeSet<&str> = depth
                .iter()
                .filter(|(path, value)| in_bucket(**value) && !selected.contains(*path))
                .map(|(path, _)| path.as_str())
                .collect();
            let omitted_count = omitted.len();
            let mut aggregated: Vec<AtlasAggregate> = Vec::new();
            if omitted_count > 0 {
                let incoming_calls = structural
                    .pairs
                    .iter()
                    .filter(|((from, to), _)| {
                        selected.contains(from) && omitted.contains(to.as_str())
                    })
                    .map(|(_, count)| *count)
                    .sum();
                aggregated.push(AtlasAggregate {
                    depth: column_depth,
                    count: omitted_count,
                    incoming_calls,
                });
            }
            columns.push(AtlasColumn {
                depth: column_depth,
                folded,
                unreached: false,
                file_count: files_in_column.len() + omitted_count,
                files: files_in_column,
                aggregated,
                nonproduction_unreached: 0,
            });
        }
        if unreached_column_shown {
            let nonproduction_unreached = unreachable_nonproduction
                .iter()
                .filter(|path| !shown_unreached.contains(path))
                .count();
            columns.push(AtlasColumn {
                depth: unreached_depth,
                folded: false,
                unreached: true,
                file_count: shown_unreached.len() + nonproduction_unreached,
                files: shown_unreached.clone(),
                aggregated: Vec::new(),
                nonproduction_unreached,
            });
        }
    }

    // Edges: forced path pairs first, then count-desc pairs up to the cap.
    let mut path_pairs: Vec<(String, String)> = Vec::new();
    for window in path_files.windows(2) {
        path_pairs.push((window[0].clone(), window[1].clone()));
    }
    let path_set: BTreeSet<(String, String)> = path_pairs.iter().cloned().collect();
    let edge_pairs_total = structural
        .pairs
        .keys()
        .filter(|(from, to)| selected.contains(from) && selected.contains(to))
        .count();
    let mut non_path: Vec<((String, String), usize)> = structural
        .pairs
        .iter()
        .filter(|((from, to), _)| {
            selected.contains(from)
                && selected.contains(to)
                && !path_set.contains(&(from.clone(), to.clone()))
        })
        .map(|(pair, count)| (pair.clone(), *count))
        .collect();
    non_path.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.0.cmp(&b.0.0))
            .then_with(|| a.0.1.cmp(&b.0.1))
    });
    let mut drawn_pairs: Vec<((String, String), usize)> = path_pairs
        .iter()
        .map(|pair| {
            (
                pair.clone(),
                structural.pairs.get(pair).copied().unwrap_or(1),
            )
        })
        .collect();
    for pair in non_path {
        if drawn_pairs.len() >= EDGE_CAP {
            break;
        }
        drawn_pairs.push(pair);
    }

    let drawn_depth_by_path: BTreeMap<&str, u32> = nodes
        .iter()
        .filter_map(|node| node.drawn_depth.map(|depth| (node.path.as_str(), depth)))
        .collect();
    let edges: Vec<AtlasEdge> = drawn_pairs
        .into_iter()
        .map(|((from, to), count)| {
            let back = match (
                drawn_depth_by_path.get(from.as_str()),
                drawn_depth_by_path.get(to.as_str()),
            ) {
                (Some(source), Some(target)) => target <= source,
                _ => false,
            };
            AtlasEdge {
                on_path: path_set.contains(&(from.clone(), to.clone())),
                from,
                to,
                count,
                back,
            }
        })
        .collect();

    let path: Vec<AtlasPathStep> = path_files
        .iter()
        .enumerate()
        .map(|(index, file)| AtlasPathStep {
            step: index + 1,
            path: file.clone(),
            depth: depth.get(file).copied().unwrap_or(0),
            cost: costs.get(file).copied().unwrap_or(0),
        })
        .collect();

    let coverage = coverage(doc, &depth, &source_paths, &files, &statuses);
    let components = components(&files);
    let blast_radius = blast_radius(
        doc,
        entry.point.as_ref().map(|point| point.path.as_str()),
        &structural,
        &source_paths,
    );

    let source_count = source_paths.len();
    let production_source_count = files.values().filter(|info| info.production).count();
    let reachable_files = depth.len();
    let reachable_selected = selected
        .iter()
        .filter(|path| depth.contains_key(*path))
        .count();
    let aggregated_files = reachable_files.saturating_sub(reachable_selected);
    let depth_folded = depth
        .values()
        .filter(|value| **value > DEPTH_COLUMN_CAP)
        .count();
    let total_cost = files
        .values()
        .map(|info| info.cost)
        .fold(0_u64, u64::saturating_add);
    let median_function_cost = if v2.total() > 0 {
        v2.median()
    } else {
        v1.median()
    };

    let caps = AtlasCaps {
        nodes_shown: nodes.len(),
        reachable_files,
        aggregated_files,
        edges_drawn: edges.len(),
        edge_pairs_total,
        depth_folded,
        reachable_shown: reachable_selected,
    };
    let state = AtlasState {
        empty: source_count == 0,
        no_entry: entry.policy == EntryPolicy::NoEntry,
        library: entry.policy == EntryPolicy::LibraryRoot,
        all_test: source_count > 0 && production_source_count == 0,
        capped: caps.aggregated_files > 0
            || caps.edges_drawn < caps.edge_pairs_total
            || caps.depth_folded > 0,
        partial_parse: coverage.parse_partial + coverage.parse_failed > 0,
        single_file: source_count == 1,
    };

    AtlasView {
        entry,
        hottest,
        hottest_node,
        nodes,
        files: file_summaries,
        edges,
        columns,
        unreachable_production,
        unreachable_nonproduction,
        path,
        components,
        blast_radius,
        coverage,
        caps,
        reachability: AtlasReachability {
            reachable_files,
            source_files: source_count,
        },
        edge_mix: structural.mix,
        totals: AtlasTotals {
            total_cost,
            median_function_cost,
            function_count: doc.stats.functions.len(),
            file_count: doc.files.len(),
            finding_count: doc.findings.len(),
        },
        state,
    }
}

fn sorted_unreachable(
    files: &BTreeMap<String, FileInfo>,
    depth: &BTreeMap<String, u32>,
    production: bool,
) -> Vec<String> {
    let mut paths: Vec<&FileInfo> = files
        .values()
        .filter(|info| info.production == production && !depth.contains_key(&info.path))
        .collect();
    paths.sort_by(|a, b| b.cost.cmp(&a.cost).then_with(|| a.path.cmp(&b.path)));
    paths.into_iter().map(|info| info.path.clone()).collect()
}

// ---------------------------------------------------------------------------
// Test support — deterministic fixtures reusable by renderer tests (step 4)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod test_support {
    use std::collections::BTreeMap;

    use crate::analyzer::{Confidence, Severity};
    use crate::ir::{IncludeCounts, IncludeForm, LocCounts, ParseStatus, ResolverTier, Span};
    use crate::schema::{
        CallEdge, CallGraphSection, ComponentEntry, ComponentMetrics, FileEntry, FindingEntry,
        GraphsSection, IncludeEdgeDto, IncludeGraph, MetricsDocument, ParseHealthEntry,
        ParseHealthSummary, ScanInfo, StatsSection, ToolInfo,
    };

    use super::FunctionStats;

    fn health(status: ParseStatus) -> ParseHealthEntry {
        ParseHealthEntry {
            status,
            error_regions: None,
            error_node_count: 0,
            resolver: None,
        }
    }

    /// One scanned file (parse status `Ok`).
    pub(crate) fn file(path: &str, class: &str, function_count: u32) -> FileEntry {
        file_with_status(path, class, function_count, ParseStatus::Ok)
    }

    /// One scanned file with an explicit parse status.
    pub(crate) fn file_with_status(
        path: &str,
        class: &str,
        function_count: u32,
        status: ParseStatus,
    ) -> FileEntry {
        FileEntry {
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
            parse_health: health(status),
            path_class: Some(class.to_string()),
        }
    }

    /// One function; `cost: None` means v1-only (pre-I2 document).
    pub(crate) fn function(
        path: &str,
        name: &str,
        line: u32,
        cost: Option<u64>,
        fan_in: u32,
        entry: Option<&str>,
    ) -> FunctionStats {
        let entry_confidence = match entry {
            Some("main" | "script") => Some("HIGH".to_string()),
            Some("lib-root" | "handler") => Some("MEDIUM".to_string()),
            Some("unknown-entry") => Some("LOW".to_string()),
            _ => None,
        };
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
            max_loop_depth: 0,
            alloc_site_count: 0,
            length_lines: 6,
            call_fanout: Some(0),
            call_fanin: Some(fan_in),
            call_depth: Some(0),
            is_recursive: Some(false),
            idiom_hash: None,
            error_path: Some(false),
            alloc_family: None,
            cfg_gated: None,
            entry_kind: entry.map(str::to_string),
            entry_confidence,
            reachable_from_entry: None,
            function_cost_v2: cost,
            unknown_trip_loops: 0,
            trip_count_source: None,
            known_trip_value: None,
        }
    }

    /// One finding.
    pub(crate) fn finding(
        rule_id: &str,
        path: &str,
        line: u32,
        severity: Severity,
        title: &str,
    ) -> FindingEntry {
        FindingEntry {
            finding_id: format!("{rule_id}:{path}:{line}:1"),
            rule_id: rule_id.to_string(),
            title: title.to_string(),
            path: path.to_string(),
            symbol: None,
            span: Span::point(line, 1),
            confidence: Confidence::High,
            severity,
            resolver_tier: ResolverTier::Syntactic,
            macro_suspect: false,
            error_node_overlap: false,
            threshold: None,
            message: format!("Message {rule_id}"),
            evidence: BTreeMap::new(),
        }
    }

    /// One structural call edge.
    pub(crate) fn call_edge(
        from_function: &str,
        to_name: &str,
        from_path: &str,
        to_path: Option<&str>,
        confidence: Confidence,
    ) -> CallEdge {
        CallEdge::with_confidence(
            from_function,
            to_name,
            from_path,
            to_path.map(str::to_string),
            confidence,
        )
    }

    /// One include edge.
    pub(crate) fn include_edge(from: &str, to: Option<&str>, resolved: bool) -> IncludeEdgeDto {
        IncludeEdgeDto {
            from: from.to_string(),
            to: to.map(str::to_string),
            form: IncludeForm::Quoted,
            resolved,
            span: Span::point(1, 1),
        }
    }

    /// One `components[]` entry (metrics are not read by the Atlas view).
    pub(crate) fn component(prefix: &str) -> ComponentEntry {
        ComponentEntry {
            component_id: prefix.to_string(),
            path_prefix: prefix.to_string(),
            rank: 1,
            metrics: ComponentMetrics {
                complexity_sum: 0.0,
                alloc_site_density: 0.0,
                max_loop_depth: 0,
                finding_density: 0.0,
            },
            finding_count: None,
        }
    }

    /// Assemble a document from the given sections.
    pub(crate) fn document(
        files: Vec<FileEntry>,
        functions: Vec<FunctionStats>,
        findings: Vec<FindingEntry>,
        call_edges: Vec<CallEdge>,
        include_edges: Vec<IncludeEdgeDto>,
    ) -> MetricsDocument {
        DocBuilder {
            files,
            functions,
            findings,
            call_edges,
            include_edges,
            ..DocBuilder::default()
        }
        .build()
    }

    /// Minimal deterministic `MetricsDocument` builder.
    #[derive(Default)]
    pub(crate) struct DocBuilder {
        files: Vec<FileEntry>,
        functions: Vec<FunctionStats>,
        findings: Vec<FindingEntry>,
        call_edges: Vec<CallEdge>,
        include_edges: Vec<IncludeEdgeDto>,
        parse_summary: Option<ParseHealthSummary>,
        components: Option<Vec<ComponentEntry>>,
    }

    impl DocBuilder {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn file(mut self, file: FileEntry) -> Self {
            self.files.push(file);
            self
        }

        pub(crate) fn function(mut self, function: FunctionStats) -> Self {
            self.functions.push(function);
            self
        }

        pub(crate) fn finding(mut self, finding: FindingEntry) -> Self {
            self.findings.push(finding);
            self
        }

        pub(crate) fn call_edge(mut self, edge: CallEdge) -> Self {
            self.call_edges.push(edge);
            self
        }

        pub(crate) fn include_edge(mut self, edge: IncludeEdgeDto) -> Self {
            self.include_edges.push(edge);
            self
        }

        pub(crate) fn parse_summary(mut self, summary: ParseHealthSummary) -> Self {
            self.parse_summary = Some(summary);
            self
        }

        pub(crate) fn components(mut self, components: Vec<ComponentEntry>) -> Self {
            self.components = Some(components);
            self
        }

        pub(crate) fn build(self) -> MetricsDocument {
            MetricsDocument {
                schema_version: "1.7".to_string(),
                tool: ToolInfo {
                    name: "perfpal".to_string(),
                    version: "test".to_string(),
                },
                scan: ScanInfo {
                    root_path: ".".to_string(),
                    started_at: "2026-10-23T00:00:00Z".to_string(),
                    duration_ms: None,
                    languages: vec!["c".to_string()],
                    file_count: self.files.len() as u32,
                    loc_total: None,
                },
                parse_health_summary: self.parse_summary,
                files: self.files,
                stats: StatsSection {
                    functions: self.functions,
                    distributions: None,
                },
                findings: self.findings,
                graphs: Some(GraphsSection {
                    include_graph: IncludeGraph {
                        nodes: Vec::new(),
                        edges: self.include_edges,
                    },
                    call_graph: Some(CallGraphSection {
                        edges: self.call_edges,
                    }),
                }),
                components: self.components,
            }
        }
    }

    // -----------------------------------------------------------------------
    // Fixtures (state matrix + core derivations)
    // -----------------------------------------------------------------------

    /// Application: production main, two forward hops, one unreachable file.
    pub(crate) fn application_fixture() -> MetricsDocument {
        document(
            vec![
                file("src/main.c", "production", 2),
                file("src/a.c", "production", 2),
                file("src/b.c", "production", 1),
                file("src/c.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(500), 0, Some("main")),
                function("src/main.c", "init", 20, Some(50), 1, None),
                function("src/a.c", "work", 1, Some(300), 1, None),
                function("src/a.c", "helper", 30, Some(20), 2, None),
                function("src/b.c", "leaf", 1, Some(100), 1, None),
                function("src/c.c", "island", 1, Some(900), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "work",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/a.c#work#1:1",
                    "leaf",
                    "src/a.c",
                    Some("src/b.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/b.c#leaf#1:1",
                    "main",
                    "src/b.c",
                    Some("src/main.c"),
                    Confidence::Low,
                ),
            ],
            vec![
                include_edge("src/b.c", Some("src/a.c"), true),
                include_edge("src/c.c", Some("src/a.c"), true),
            ],
        )
    }

    /// Python script entry with a non-production test main.
    pub(crate) fn script_fixture() -> MetricsDocument {
        document(
            vec![
                file("app.py", "production", 2),
                file("tests/test_app.py", "test", 1),
            ],
            vec![
                function("app.py", "<module>", 1, Some(200), 0, Some("script")),
                function("app.py", "helper", 10, Some(50), 1, None),
                function(
                    "tests/test_app.py",
                    "test_main",
                    1,
                    Some(900),
                    0,
                    Some("main"),
                ),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// Library: highest-fan-in production lib-root, unreachable callers.
    pub(crate) fn library_fixture() -> MetricsDocument {
        document(
            vec![
                file("src/lib.c", "production", 1),
                file("src/caller1.c", "production", 1),
                file("src/caller2.c", "production", 1),
                file("src/root2.c", "production", 1),
                file("src/root3.c", "production", 1),
            ],
            vec![
                function("src/lib.c", "lib_root", 1, Some(100), 10, Some("lib-root")),
                function("src/caller1.c", "caller_one", 1, Some(200), 0, None),
                function("src/caller2.c", "caller_two", 1, Some(50), 0, None),
                function("src/root2.c", "root_two", 1, Some(300), 4, Some("lib-root")),
                function(
                    "src/root3.c",
                    "root_three",
                    1,
                    Some(400),
                    0,
                    Some("lib-root"),
                ),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/caller1.c#caller_one#1:1",
                    "lib_root",
                    "src/caller1.c",
                    Some("src/lib.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/caller2.c#caller_two#1:1",
                    "lib_root",
                    "src/caller2.c",
                    Some("src/lib.c"),
                    Confidence::Low,
                ),
            ],
            Vec::new(),
        )
    }

    /// Production handler only: no electable entry.
    pub(crate) fn no_entry_fixture() -> MetricsDocument {
        document(
            vec![file("src/handler.c", "production", 1)],
            vec![function(
                "src/handler.c",
                "handle",
                1,
                Some(100),
                0,
                Some("handler"),
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// Empty scan: no files at all.
    pub(crate) fn empty_fixture() -> MetricsDocument {
        document(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new())
    }

    /// Test-only corpus (no production source files).
    pub(crate) fn all_test_fixture() -> MetricsDocument {
        document(
            vec![file("tests/test_main.c", "test", 1)],
            vec![function(
                "tests/test_main.c",
                "test_main",
                1,
                Some(100),
                0,
                Some("main"),
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// 26 reachable files with one depth-2 file; node cap bites.
    pub(crate) fn capped_fixture() -> MetricsDocument {
        let mut files = vec![file("src/entry.c", "production", 1)];
        let mut functions = vec![function(
            "src/entry.c",
            "main",
            1,
            Some(10),
            0,
            Some("main"),
        )];
        let mut edges = Vec::new();
        for index in 0..24_u64 {
            let path = format!("src/f{index}.c");
            files.push(file(&path, "production", 1));
            functions.push(function(
                &path,
                &format!("f{index}"),
                1,
                Some(1000 - index),
                0,
                None,
            ));
            edges.push(call_edge(
                "src/entry.c#main#1:1",
                &format!("f{index}"),
                "src/entry.c",
                Some(&path),
                Confidence::Low,
            ));
        }
        files.push(file("src/deep.c", "production", 1));
        functions.push(function("src/deep.c", "deep", 1, Some(5), 0, None));
        edges.push(call_edge(
            "src/f0.c#f0#1:1",
            "deep",
            "src/f0.c",
            Some("src/deep.c"),
            Confidence::Low,
        ));
        document(files, functions, Vec::new(), edges, Vec::new())
    }

    /// 21 reachable files with a dense cross-edge set (> EDGE_CAP pairs).
    pub(crate) fn edge_cap_fixture() -> MetricsDocument {
        let mut files = vec![file("src/entry.c", "production", 1)];
        let mut functions = vec![function(
            "src/entry.c",
            "main",
            1,
            Some(10),
            0,
            Some("main"),
        )];
        let mut edges = Vec::new();
        for index in 0..20_u64 {
            let path = format!("src/f{index}.c");
            files.push(file(&path, "production", 1));
            functions.push(function(
                &path,
                &format!("f{index}"),
                1,
                Some(1000 - index),
                0,
                None,
            ));
            edges.push(call_edge(
                "src/entry.c#main#1:1",
                &format!("f{index}"),
                "src/entry.c",
                Some(&path),
                Confidence::Low,
            ));
        }
        for from in 0..20_u64 {
            for to in (from + 1)..20_u64 {
                edges.push(call_edge(
                    &format!("src/f{from}.c#f{from}#1:1"),
                    &format!("f{to}"),
                    &format!("src/f{from}.c"),
                    Some(&format!("src/f{to}.c")),
                    Confidence::Low,
                ));
            }
        }
        document(files, functions, Vec::new(), edges, Vec::new())
    }

    /// Application shape with one partially parsed source file.
    pub(crate) fn partial_parse_fixture() -> MetricsDocument {
        document(
            vec![
                file("src/main.c", "production", 2),
                file_with_status("src/broken.c", "production", 1, ParseStatus::Partial),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/main.c", "helper", 10, Some(20), 1, None),
                function("src/broken.c", "broken", 1, Some(50), 0, None),
            ],
            Vec::new(),
            vec![call_edge(
                "src/main.c#main#1:1",
                "broken",
                "src/main.c",
                Some("src/broken.c"),
                Confidence::Low,
            )],
            Vec::new(),
        )
    }

    /// One production file with a main function and no edges.
    pub(crate) fn single_file_fixture() -> MetricsDocument {
        document(
            vec![file("src/solo.c", "production", 1)],
            vec![function("src/solo.c", "main", 1, Some(42), 0, Some("main"))],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// Pre-I2 document: every function is v1-only.
    pub(crate) fn v1_only_fixture() -> MetricsDocument {
        document(
            vec![file("src/v1.c", "production", 2)],
            vec![
                function("src/v1.c", "main", 1, None, 0, Some("main")),
                function("src/v1.c", "helper", 10, None, 0, None),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::analyzer::Confidence;

    fn node<'a>(view: &'a AtlasView, path: &str) -> &'a AtlasNode {
        view.nodes
            .iter()
            .find(|node| node.path == path)
            .unwrap_or_else(|| panic!("missing node {path}"))
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.unwrap_or_else(|| panic!("missing ratio"));
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    // -- entry election -----------------------------------------------------

    #[test]
    fn entry_prefers_production_main_and_excludes_build_and_test() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("build.rs", "production", 1),
                file("tests/t.c", "test", 1),
                file("app.py", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("build.rs", "main", 1, Some(900), 0, Some("main")),
                function("tests/t.c", "main", 1, Some(800), 0, Some("main")),
                function("app.py", "<module>", 1, Some(700), 0, Some("script")),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(view.entry.policy, EntryPolicy::ProductionMain);
        assert_eq!(view.entry.policy.label(), "production main");
        let point = view.entry.point.expect("elected main");
        assert_eq!(point.path, "src/main.c");
        assert_eq!(point.name, "main");
        assert_eq!(point.line, 1);
        assert_eq!(point.cost, 100);
        assert_eq!(view.entry.confidence, "HIGH");
        // build.rs and the test main are never alternatives.
        assert!(view.entry.alternatives.is_empty());
    }

    #[test]
    fn entry_ties_use_cost_fan_in_path_name_id_order() {
        let doc = document(
            vec![
                file("src/b.c", "production", 1),
                file("src/a.c", "production", 1),
            ],
            vec![
                function("src/b.c", "main", 1, Some(100), 5, Some("main")),
                function("src/a.c", "main", 1, Some(100), 5, Some("main")),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(
            view.entry.point.as_ref().map(|point| point.path.as_str()),
            Some("src/a.c")
        );
        assert_eq!(view.entry.alternatives.len(), 1);
        assert_eq!(view.entry.alternatives[0].path, "src/b.c");
    }

    #[test]
    fn entry_script_when_no_eligible_main_and_beats_lib_root() {
        let doc = document(
            vec![
                file("app.py", "production", 1),
                file("src/lib.c", "production", 1),
            ],
            vec![
                function("app.py", "<module>", 1, Some(100), 0, Some("script")),
                function("src/lib.c", "lib_root", 1, Some(900), 50, Some("lib-root")),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(view.entry.policy, EntryPolicy::ProductionScript);
        assert_eq!(
            view.entry.point.as_ref().map(|point| point.path.as_str()),
            Some("app.py")
        );

        let script = build_atlas_view(&script_fixture());
        assert_eq!(script.entry.policy, EntryPolicy::ProductionScript);
        assert_eq!(script.entry.confidence, "HIGH");
        assert!(script.entry.alternatives.is_empty());
    }

    #[test]
    fn entry_library_root_threshold_and_five_alternatives() {
        let mut builder = DocBuilder::new();
        let mut functions = vec![function(
            "src/lib.c",
            "lib_root",
            1,
            Some(100),
            10,
            Some("lib-root"),
        )];
        for index in 0..7_u32 {
            let path = format!("src/root{index}.c");
            let fan_in = 9 - index;
            functions.push(function(
                &path,
                &format!("root{index}"),
                1,
                Some(50),
                fan_in,
                Some("lib-root"),
            ));
        }
        builder = builder.file(file("src/lib.c", "production", 1));
        for index in 0..7_u32 {
            builder = builder.file(file(&format!("src/root{index}.c"), "production", 1));
        }
        for function in functions {
            builder = builder.function(function);
        }
        let view = build_atlas_view(&builder.build());
        assert_eq!(view.entry.policy, EntryPolicy::LibraryRoot);
        assert_eq!(
            view.entry.policy.label(),
            "library root (no production main)"
        );
        // 20% of fan-in 10 = 2.0: roots 9..=4 qualify (six), capped at five.
        assert_eq!(view.entry.alternatives.len(), 5);
        assert_eq!(view.entry.alternatives[0].fan_in, 9);
        assert_eq!(view.entry.alternatives[4].fan_in, 5);
        assert!(view.state.library);
    }

    #[test]
    fn entry_none_stub_never_panics() {
        let view = build_atlas_view(&no_entry_fixture());
        assert_eq!(view.entry.policy, EntryPolicy::NoEntry);
        assert_eq!(view.entry.policy.label(), "no entry");
        assert!(view.entry.point.is_none());
        assert_eq!(view.entry.confidence, "UNKNOWN");
        assert!(view.entry.alternatives.is_empty());
        assert!(view.nodes.is_empty());
        assert!(view.path.is_empty());
        // M-04 rank 1 exists globally even without an entry.
        assert_eq!(view.hottest.as_deref(), Some("src/handler.c"));
        let hottest_node = view.hottest_node.as_ref().expect("hottest node");
        assert_eq!(hottest_node.path, "src/handler.c");
        assert!(hottest_node.hottest);
        assert!(hottest_node.unreached);
        assert!(!hottest_node.entry);
        assert_eq!(hottest_node.drawn_depth, None);
        assert_eq!(view.totals.function_count, 1);
    }

    // -- BFS / structural edges --------------------------------------------

    #[test]
    fn bfs_uses_structural_edges_and_restricts_to_source_files() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/not_source.c", "production", 0),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a.c", "a", 1, Some(50), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "a",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                // Non-source target: ignored.
                call_edge(
                    "src/main.c#main#1:1",
                    "not_source",
                    "src/main.c",
                    Some("src/not_source.c"),
                    Confidence::Low,
                ),
                // Unresolved target: ignored.
                call_edge(
                    "src/main.c#main#1:1",
                    "missing",
                    "src/main.c",
                    None,
                    Confidence::Low,
                ),
            ],
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(view.reachability.source_files, 2);
        assert_eq!(view.reachability.reachable_files, 2);
        assert_eq!(node(&view, "src/a.c").depth, Some(1));
        assert!(
            view.nodes
                .iter()
                .all(|node| node.path != "src/not_source.c")
        );
        assert_eq!(view.edge_mix.low, 1);
        assert_eq!(view.edge_mix.medium, 0);
        assert_eq!(
            view.path
                .iter()
                .map(|step| step.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/main.c", "src/a.c"]
        );
    }

    #[test]
    fn bfs_parent_tie_break_prefers_the_higher_cost_parent() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/high.c", "production", 1),
                file("src/low.c", "production", 1),
                file("src/target.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/high.c", "high", 1, Some(100), 0, None),
                function("src/low.c", "low", 1, Some(10), 0, None),
                function("src/target.c", "target", 1, Some(50), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "high",
                    "src/main.c",
                    Some("src/high.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/main.c#main#1:1",
                    "low",
                    "src/main.c",
                    Some("src/low.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/high.c#high#1:1",
                    "target",
                    "src/high.c",
                    Some("src/target.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/low.c#low#1:1",
                    "target",
                    "src/low.c",
                    Some("src/target.c"),
                    Confidence::Low,
                ),
            ],
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(
            view.path
                .iter()
                .map(|step| step.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/main.c", "src/high.c", "src/target.c"]
        );
    }

    #[test]
    fn edge_confidence_mix_counts_distinct_structural_edges() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
                file("src/c.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a.c", "a", 1, Some(10), 0, None),
                function("src/b.c", "b", 1, Some(10), 0, None),
                function("src/c.c", "c", 1, Some(10), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "a",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                // Exact duplicate triple: not a distinct structural edge.
                call_edge(
                    "src/main.c#main#1:1",
                    "a",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                // Different callee name: distinct edge.
                call_edge(
                    "src/main.c#main#1:1",
                    "a_alias",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/main.c#main#1:1",
                    "b",
                    "src/main.c",
                    Some("src/b.c"),
                    Confidence::Medium,
                ),
                call_edge(
                    "src/main.c#main#1:1",
                    "c",
                    "src/main.c",
                    Some("src/c.c"),
                    Confidence::High,
                ),
            ],
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(view.edge_mix.low, 2);
        assert_eq!(view.edge_mix.medium, 1);
        assert_eq!(view.edge_mix.high, 1);
        let pair = view
            .edges
            .iter()
            .find(|edge| edge.from == "src/main.c" && edge.to == "src/a.c")
            .expect("aggregated pair");
        assert_eq!(pair.count, 2);
    }

    // -- caps / aggregation ------------------------------------------------

    #[test]
    fn node_cap_aggregates_omitted_files_and_marks_capped() {
        let view = build_atlas_view(&capped_fixture());
        assert_eq!(view.caps.nodes_shown, NODE_CAP);
        assert_eq!(view.caps.reachable_files, 26);
        assert_eq!(view.caps.aggregated_files, 6);
        assert_eq!(view.caps.reachable_shown, 20);
        assert!(view.state.capped);
        assert!(view.nodes.iter().all(|node| node.path != "src/deep.c"));

        let depth_zero = view
            .columns
            .iter()
            .find(|column| column.depth == 0)
            .expect("depth 0");
        assert_eq!(depth_zero.file_count, 1);
        assert!(depth_zero.aggregated.is_empty());

        let depth_one = view
            .columns
            .iter()
            .find(|column| column.depth == 1)
            .expect("depth 1");
        assert_eq!(depth_one.files.len(), 19);
        assert_eq!(depth_one.file_count, 24);
        assert_eq!(depth_one.aggregated.len(), 1);
        assert_eq!(depth_one.aggregated[0].count, 5);
        assert_eq!(depth_one.aggregated[0].incoming_calls, 5);

        let depth_two = view
            .columns
            .iter()
            .find(|column| column.depth == 2)
            .expect("depth 2");
        assert!(depth_two.files.is_empty());
        assert_eq!(depth_two.file_count, 1);
        assert_eq!(depth_two.aggregated[0].count, 1);
        assert_eq!(depth_two.aggregated[0].incoming_calls, 1);
    }

    #[test]
    fn capped_flag_uses_aggregated_count_not_nodes_shown() {
        // 20 reachable files + one unreachable HOTTEST. The mandatory hottest
        // consumes the cap slot, leaving one reachable file aggregated:
        // nodes_shown (20) is NOT less than reachable_files (20), so the flag
        // must come from aggregated_files.
        let mut builder = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .function(function("src/main.c", "main", 1, Some(10), 0, Some("main")));
        let mut call_edges = Vec::new();
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
            call_edges.push(call_edge(
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
        for edge in call_edges {
            builder = builder.call_edge(edge);
        }

        let view = build_atlas_view(&builder.build());
        assert_eq!(view.caps.reachable_files, 20);
        assert_eq!(view.caps.nodes_shown, 20);
        assert_eq!(view.caps.reachable_shown, 19);
        assert_eq!(view.caps.aggregated_files, 1);
        assert!(view.state.capped);
        assert!(node(&view, "src/hot.c").unreached);
        assert!(node(&view, "src/hot.c").hottest);
    }

    #[test]
    fn edge_cap_keeps_forced_path_edges() {
        let view = build_atlas_view(&edge_cap_fixture());
        assert_eq!(view.reachability.reachable_files, 21);
        assert_eq!(view.caps.edges_drawn, EDGE_CAP);
        assert_eq!(view.caps.edge_pairs_total, 190);
        assert_eq!(view.edges.len(), EDGE_CAP);
        assert!(view.state.capped);
        assert!(view.edges[0].on_path);
        assert_eq!(view.edges[0].from, "src/entry.c");
        assert_eq!(view.edges[0].to, "src/f0.c");
        // Same-depth structural edges are back-edges by the drawn-column rule.
        assert!(view.edges.iter().any(|edge| edge.back));
    }

    #[test]
    fn path_edges_over_the_cap_are_still_drawn() {
        let mut builder = DocBuilder::new().file(file("src/f0.c", "production", 1));
        let mut functions = vec![function("src/f0.c", "main", 1, Some(10), 0, Some("main"))];
        let mut edges = Vec::new();
        for index in 1..60_u64 {
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
        assert_eq!(view.path.len(), 60);
        assert_eq!(view.edges.len(), 59);
        assert!(view.edges.len() > EDGE_CAP);
        assert!(view.edges.iter().all(|edge| edge.on_path));
    }

    // -- hottest / columns -------------------------------------------------

    #[test]
    fn hottest_is_global_rank_one_and_forced_when_unreachable() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
                file("src/c.c", "production", 1),
                file("src/hot.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(10), 0, Some("main")),
                function("src/a.c", "a", 1, Some(100), 0, None),
                function("src/b.c", "b", 1, Some(50), 0, None),
                function("src/c.c", "c", 1, Some(20), 0, None),
                function("src/hot.c", "hot", 1, Some(9000), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "a",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/a.c#a#1:1",
                    "b",
                    "src/a.c",
                    Some("src/b.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/b.c#b#1:1",
                    "c",
                    "src/b.c",
                    Some("src/c.c"),
                    Confidence::Low,
                ),
            ],
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(view.hottest.as_deref(), Some("src/hot.c"));
        let hot = node(&view, "src/hot.c");
        assert!(hot.hottest);
        assert!(hot.unreached);
        assert_eq!(hot.depth, None);
        assert_eq!(view.unreachable_production, vec!["src/hot.c"]);
        // Depth 3 exceeds the unreached-fill rule, but the hottest still shows.
        let last = view.columns.last().expect("unreached column");
        assert!(last.unreached);
        assert_eq!(last.depth, 4);
        assert_eq!(last.files, vec!["src/hot.c"]);
    }

    #[test]
    fn hottest_may_also_be_the_entry() {
        let view = build_atlas_view(&single_file_fixture());
        assert_eq!(view.hottest.as_deref(), Some("src/solo.c"));
        let solo = node(&view, "src/solo.c");
        assert!(solo.entry);
        assert!(solo.hottest);
        assert!(!solo.unreached);
    }

    // -- file cost / ranks -------------------------------------------------

    #[test]
    fn file_cost_sums_v2_and_falls_back_to_v1() {
        let doc = document(
            vec![
                file("src/main.c", "production", 3),
                file("src/mixed.c", "production", 2),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/main.c", "helper", 10, Some(50), 1, None),
                function("src/main.c", "extra", 20, Some(10), 0, None),
                function("src/mixed.c", "v2fn", 1, Some(100), 0, None),
                function("src/mixed.c", "v1fn", 10, None, 2, None),
            ],
            Vec::new(),
            vec![call_edge(
                "src/main.c#main#1:1",
                "v2fn",
                "src/main.c",
                Some("src/mixed.c"),
                Confidence::Low,
            )],
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        let main = node(&view, "src/main.c");
        assert_eq!(main.cost, 160);
        assert_eq!(main.scale, CostScale::V2);
        assert_eq!(main.file_population, 1);
        assert_eq!(main.file_rank, 1);
        assert_eq!(main.top_pct, 100);
        assert_close(main.ratio, 160.0 / 50.0);

        let mixed = node(&view, "src/mixed.c");
        // v1 fallback sums v1 cost for EVERY function in the file:
        // v2fn -> 1, v1fn -> 3.
        assert_eq!(mixed.cost, 4);
        assert_eq!(mixed.scale, CostScale::V1);
        assert_eq!(mixed.file_population, 1);
        assert_close(mixed.ratio, 4.0 / 3.0);

        // A mixed document uses the v2 median for the page-level anchor.
        assert_close(view.totals.median_function_cost, 50.0);
    }

    #[test]
    fn v1_only_document_uses_v1_median_and_scale() {
        let view = build_atlas_view(&v1_only_fixture());
        let solo = node(&view, "src/v1.c");
        assert_eq!(solo.scale, CostScale::V1);
        assert_eq!(solo.cost, 2);
        assert_close(view.totals.median_function_cost, 1.0);
        assert_close(solo.ratio, 2.0);
    }

    #[test]
    fn median_even_n_uses_lower_middle() {
        let doc = document(
            vec![file("src/a.c", "production", 4)],
            vec![
                function("src/a.c", "main", 1, Some(10), 0, Some("main")),
                function("src/a.c", "b", 10, Some(20), 0, None),
                function("src/a.c", "c", 20, Some(30), 0, None),
                function("src/a.c", "d", 30, Some(40), 0, None),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_close(view.totals.median_function_cost, 20.0);
        let file = node(&view, "src/a.c");
        assert_eq!(file.cost, 100);
        assert_close(file.ratio, 5.0);
    }

    // -- components --------------------------------------------------------

    #[test]
    fn domain_of_skips_empty_leading_segments() {
        assert_eq!(domain_of("_private.py"), "private");
        assert_eq!(domain_of("-dash.c"), "dash");
        assert_eq!(domain_of("src/a/b.c"), "src/a");
        assert_eq!(domain_of("pkg/mod.py"), "pkg");
    }

    #[test]
    fn components_group_by_prefix_and_track_production_cost() {
        let doc = document(
            vec![
                file("src/a/x.c", "production", 1),
                file("src/a/y.c", "test", 1),
                file("src/b/z.c", "production", 1),
                file("root.c", "production", 1),
                file("tests/heavy.c", "test", 1),
            ],
            vec![
                function("src/a/x.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a/y.c", "test_helper", 1, Some(50), 0, None),
                function("src/b/z.c", "z", 1, Some(200), 0, None),
                function("root.c", "root_fn", 1, Some(10), 0, None),
                function("tests/heavy.c", "heavy_test", 1, Some(500), 0, None),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(view.components.len(), 4);
        // Production-first: a 500-cost test component never outranks a
        // 200-cost production component.
        assert_eq!(view.components[0].prefix, "src/b");
        assert_eq!(view.components[0].cost, 200);
        assert_eq!(view.components[0].production_cost, 200);
        assert_eq!(view.components[0].file_count, 1);
        assert_eq!(view.components[0].production_file_count, 1);
        assert_eq!(view.components[1].prefix, "src/a");
        assert_eq!(view.components[1].cost, 150);
        assert_eq!(view.components[1].production_cost, 100);
        assert_eq!(view.components[1].file_count, 2);
        assert_eq!(view.components[1].production_file_count, 1);
        assert_eq!(view.components[1].members, vec!["src/a/x.c", "src/a/y.c"]);
        assert_eq!(view.components[2].prefix, "root");
        assert_eq!(view.components[2].cost, 10);
        assert_eq!(view.components[3].prefix, "tests");
        assert_eq!(view.components[3].cost, 500);
        assert_eq!(view.components[3].production_cost, 0);
        assert_eq!(view.components[3].file_count, 1);
        assert_eq!(view.components[3].production_file_count, 0);
    }

    // -- findings ----------------------------------------------------------

    #[test]
    fn dominant_finding_prefers_count_then_severity_then_rule() {
        let doc = document(
            vec![
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
                file("src/c.c", "production", 1),
            ],
            vec![
                function("src/a.c", "main", 1, Some(100), 0, Some("main")),
                function("src/b.c", "b", 1, Some(10), 0, None),
                function("src/c.c", "c", 1, Some(9000), 0, None),
            ],
            vec![
                finding("PERF-B-001", "src/a.c", 10, Severity::Hot, "B hot"),
                finding("PERF-B-001", "src/a.c", 11, Severity::Warn, "B warn"),
                finding("PERF-A-001", "src/a.c", 20, Severity::Warn, "A warn"),
                finding("PERF-A-001", "src/a.c", 5, Severity::Hot, "A hot"),
                finding("PERF-C-001", "src/b.c", 1, Severity::Info, "C 1"),
                finding("PERF-C-001", "src/b.c", 2, Severity::Info, "C 2"),
                finding("PERF-C-001", "src/b.c", 3, Severity::Info, "C 3"),
                finding("PERF-D-001", "src/b.c", 4, Severity::Hot, "D 1"),
                finding("PERF-D-001", "src/b.c", 5, Severity::Hot, "D 2"),
                finding("PERF-E-001", "src/c.c", 10, Severity::Hot, "E 10"),
                finding("PERF-E-001", "src/c.c", 5, Severity::Hot, "E 5"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);

        // a: equal count + equal max severity -> rule id ascending.
        let finding_a = node(&view, "src/a.c").finding.clone().expect("a finding");
        assert_eq!(finding_a.rule_id, "PERF-A-001");
        assert_eq!(finding_a.count, 2);
        assert_eq!(finding_a.severity, Severity::Hot);
        assert_eq!(finding_a.title, "A hot");

        // b: count beats severity.
        let finding_b = node(&view, "src/b.c").finding.clone().expect("b finding");
        assert_eq!(finding_b.rule_id, "PERF-C-001");
        assert_eq!(finding_b.count, 3);
        assert_eq!(finding_b.severity, Severity::Info);

        // c: title comes from the highest-severity finding, finding_id asc.
        let finding_c = node(&view, "src/c.c").finding.clone().expect("c finding");
        // `finding_id` is compared lexicographically, so line 10 sorts before
        // line 5 ("…:10:1" < "…:5:1") — the pinned tie-break is by id.
        assert_eq!(finding_c.title, "E 10");
        assert_eq!(finding_c.count, 2);
    }

    // -- functions ---------------------------------------------------------

    #[test]
    fn top_functions_order_and_anchors() {
        let doc = document(
            vec![file("src/a.c", "production", 4)],
            vec![
                function("src/a.c", "high", 1, Some(100), 1, Some("main")),
                function("src/a.c", "tie_b", 10, Some(50), 5, None),
                function("src/a.c", "tie_a", 20, Some(50), 5, None),
                function("src/a.c", "low", 30, Some(10), 0, None),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        let file = node(&view, "src/a.c");
        let names: Vec<&str> = file
            .top_functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, vec!["high", "tie_a", "tie_b"]);
        assert_eq!(file.top_functions[0].top_pct, 25);
        assert_eq!(file.top_functions[1].top_pct, 50);
        assert_eq!(file.top_functions[2].top_pct, 75);
        assert_eq!(file.top_functions[0].scale, CostScale::V2);
        assert_close(file.top_functions[0].ratio, 2.0);
        assert_eq!(file.top_functions[0].fan_in, 1);
        assert_eq!(file.top_functions[1].fan_in, 5);
    }

    // -- blast radius ------------------------------------------------------

    #[test]
    fn blast_radius_counts_includes_and_structural_calls() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
                file("src/d.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a.c", "a", 1, Some(10), 0, None),
                function("src/b.c", "b", 1, Some(10), 0, None),
                function("src/d.c", "d", 1, Some(10), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "a",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/main.c#main#1:1",
                    "b",
                    "src/main.c",
                    Some("src/b.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/a.c#a#1:1",
                    "main",
                    "src/a.c",
                    Some("src/main.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/b.c#b#1:1",
                    "main",
                    "src/b.c",
                    Some("src/main.c"),
                    Confidence::Low,
                ),
            ],
            vec![
                include_edge("src/a.c", Some("src/main.c"), true),
                include_edge("src/a.c", Some("src/main.c"), true),
                include_edge("src/b.c", Some("src/main.c"), true),
                include_edge("src/d.c", Some("src/main.c"), false),
                include_edge("src/main.c", Some("src/main.c"), true),
            ],
        );
        let view = build_atlas_view(&doc);
        let blast = &view.blast_radius;
        assert_eq!(blast.target.as_deref(), Some("src/main.c"));
        assert_eq!(blast.dependents, 2);
        assert_eq!(
            blast.top_dependents,
            vec![("src/a.c".to_string(), 2), ("src/b.c".to_string(), 1)]
        );
        assert_eq!(blast.caller_files, 2);
        assert_eq!(blast.callee_files, 2);
        assert_eq!(blast.include_edges_total, 5);
        assert_eq!(blast.include_edges_resolved, 4);
        assert_eq!(blast.median_dependents, Some(0.0));
        assert!(blast.dependent_multiple.is_none());
        assert_eq!(blast.dependent_rank, Some(1));
        assert_eq!(blast.dependent_population, 4);
        assert_eq!(blast.dependent_top_pct, Some(25));
    }

    #[test]
    fn blast_radius_median_dependents_sets_the_multiple() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
                file("src/c.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a.c", "a", 1, Some(10), 0, None),
                function("src/b.c", "b", 1, Some(10), 0, None),
                function("src/c.c", "c", 1, Some(10), 0, None),
            ],
            Vec::new(),
            Vec::new(),
            vec![
                include_edge("src/a.c", Some("src/main.c"), true),
                include_edge("src/b.c", Some("src/main.c"), true),
                include_edge("src/c.c", Some("src/a.c"), true),
                include_edge("src/c.c", Some("src/b.c"), true),
            ],
        );
        let view = build_atlas_view(&doc);
        let blast = &view.blast_radius;
        assert_eq!(blast.dependents, 2);
        // Reverse counts: main 2, a 1, b 1, c 0 -> lower-middle median 1.0.
        assert_eq!(blast.median_dependents, Some(1.0));
        assert_close(blast.dependent_multiple, 2.0);
        assert_eq!(blast.dependent_rank, Some(1));
        assert_eq!(blast.dependent_population, 4);
        assert_eq!(blast.dependent_top_pct, Some(25));
    }

    #[test]
    fn blast_radius_ranks_the_target_for_m08_top_pct() {
        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a.c", "a", 1, Some(10), 0, None),
                function("src/b.c", "b", 1, Some(10), 0, None),
            ],
            Vec::new(),
            Vec::new(),
            vec![
                include_edge("src/a.c", Some("src/main.c"), true),
                include_edge("src/b.c", Some("src/main.c"), true),
                include_edge("src/b.c", Some("src/a.c"), true),
            ],
        );
        let view = build_atlas_view(&doc);
        let blast = &view.blast_radius;
        assert_eq!(blast.dependents, 2);
        assert_eq!(blast.dependent_rank, Some(1));
        assert_eq!(blast.dependent_population, 3);
        assert_eq!(blast.dependent_top_pct, Some(33));
    }

    // -- path --------------------------------------------------------------

    #[test]
    fn path_leaf_rule_and_single_entry() {
        let single = build_atlas_view(&single_file_fixture());
        assert_eq!(single.path.len(), 1);
        assert_eq!(single.path[0].step, 1);
        assert_eq!(single.path[0].path, "src/solo.c");
        assert_eq!(single.path[0].depth, 0);
        assert_eq!(single.path[0].cost, 42);

        let doc = document(
            vec![
                file("src/main.c", "production", 1),
                file("src/a.c", "production", 1),
                file("src/b.c", "production", 1),
                file("src/c.c", "production", 1),
            ],
            vec![
                function("src/main.c", "main", 1, Some(100), 0, Some("main")),
                function("src/a.c", "a", 1, Some(10), 0, None),
                function("src/b.c", "b", 1, Some(5), 0, None),
                function("src/c.c", "c", 1, Some(200), 0, None),
            ],
            Vec::new(),
            vec![
                call_edge(
                    "src/main.c#main#1:1",
                    "a",
                    "src/main.c",
                    Some("src/a.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/a.c#a#1:1",
                    "b",
                    "src/a.c",
                    Some("src/b.c"),
                    Confidence::Low,
                ),
                call_edge(
                    "src/main.c#main#1:1",
                    "c",
                    "src/main.c",
                    Some("src/c.c"),
                    Confidence::Low,
                ),
            ],
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        assert_eq!(
            view.path
                .iter()
                .map(|step| step.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/main.c", "src/c.c"]
        );
        assert_eq!(view.path[1].depth, 1);
        assert_eq!(view.path[1].cost, 200);
    }

    #[test]
    fn application_path_and_edge_flags_are_deterministic() {
        let view = build_atlas_view(&application_fixture());
        assert_eq!(
            view.path
                .iter()
                .map(|step| step.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/main.c", "src/a.c", "src/b.c"]
        );
        let on_path: Vec<(&str, &str)> = view
            .edges
            .iter()
            .filter(|edge| edge.on_path)
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();
        assert_eq!(
            on_path,
            vec![("src/main.c", "src/a.c"), ("src/a.c", "src/b.c")]
        );
        let back = view
            .edges
            .iter()
            .find(|edge| edge.from == "src/b.c" && edge.to == "src/main.c")
            .expect("back edge");
        assert!(back.back);
        assert!(!back.on_path);
    }

    // -- coverage / state --------------------------------------------------

    #[test]
    fn coverage_and_noise_counts() {
        let doc = document(
            vec![
                file("src/a.c", "production", 1),
                file_with_status("tests/t.c", "test", 1, ParseStatus::Partial),
                file_with_status("src/broken.c", "production", 1, ParseStatus::Failed),
                file("empty.c", "production", 0),
            ],
            vec![
                function("src/a.c", "main", 1, Some(100), 0, Some("main")),
                function("tests/t.c", "helper", 1, Some(50), 0, None),
                function("src/broken.c", "broken", 1, Some(10), 0, None),
            ],
            Vec::new(),
            Vec::new(),
            vec![
                include_edge("src/a.c", None, false),
                include_edge("tests/t.c", Some("src/a.c"), true),
                include_edge("empty.c", Some("src/a.c"), false),
            ],
        );
        let view = build_atlas_view(&doc);
        let coverage = view.coverage;
        assert_eq!(coverage.parse_ok, 2);
        assert_eq!(coverage.parse_partial, 1);
        assert_eq!(coverage.parse_failed, 1);
        assert_eq!(coverage.unresolved_includes, 2);
        assert_eq!(coverage.demoted_files, 1);
        assert_eq!(coverage.unreached_production, 1);
        assert_eq!(coverage.unreached_nonproduction, 1);
        assert_eq!(coverage.files_without_functions, 1);
        assert_eq!(coverage.partial_parse_files, 2);
        assert!(view.state.partial_parse);
        assert_eq!(view.unreachable_production, vec!["src/broken.c"]);
        assert_eq!(view.unreachable_nonproduction, vec!["tests/t.c"]);

        let summary_doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .function(function("src/a.c", "main", 1, Some(100), 0, Some("main")))
            .file(file_with_status(
                "src/b.c",
                "production",
                1,
                ParseStatus::Failed,
            ))
            .parse_summary(crate::schema::ParseHealthSummary {
                ok: 9,
                partial: 0,
                failed: 0,
            })
            .build();
        let summary_view = build_atlas_view(&summary_doc);
        assert_eq!(summary_view.coverage.parse_ok, 9);
        assert_eq!(summary_view.coverage.parse_partial, 0);
        assert_eq!(summary_view.coverage.parse_failed, 0);
        // The summary drives state.partial_parse, not the per-file statuses.
        assert!(!summary_view.state.partial_parse);
    }

    #[test]
    fn state_flags_matrix() {
        let application = build_atlas_view(&application_fixture());
        assert!(!application.state.empty);
        assert!(!application.state.no_entry);
        assert!(!application.state.library);
        assert!(!application.state.all_test);
        assert!(!application.state.capped);
        assert!(!application.state.partial_parse);
        assert!(!application.state.single_file);

        let library = build_atlas_view(&library_fixture());
        assert!(library.state.library);
        assert!(!library.state.no_entry);
        assert_eq!(
            library
                .entry
                .point
                .as_ref()
                .map(|point| point.path.as_str()),
            Some("src/lib.c")
        );

        let no_entry = build_atlas_view(&no_entry_fixture());
        assert!(no_entry.state.no_entry);

        let empty = build_atlas_view(&empty_fixture());
        assert!(empty.state.empty);
        assert!(empty.state.no_entry);
        assert!(empty.nodes.is_empty());
        assert_eq!(empty.totals.total_cost, 0);
        assert!(empty.totals.median_function_cost.is_none());

        let all_test = build_atlas_view(&all_test_fixture());
        assert!(all_test.state.all_test);
        assert!(all_test.state.no_entry);
        assert_eq!(all_test.hottest.as_deref(), Some("tests/test_main.c"));
        assert!(all_test.hottest_node.is_some());
        assert!(all_test.nodes.is_empty());

        let capped = build_atlas_view(&capped_fixture());
        assert!(capped.state.capped);

        let partial = build_atlas_view(&partial_parse_fixture());
        assert!(partial.state.partial_parse);

        let single = build_atlas_view(&single_file_fixture());
        assert!(single.state.single_file);
    }

    #[test]
    fn depth_columns_fold_past_the_cap() {
        let mut builder = DocBuilder::new().file(file("src/f0.c", "production", 1));
        let mut functions = vec![function("src/f0.c", "main", 1, Some(100), 0, Some("main"))];
        let mut edges = Vec::new();
        for index in 1..12_u64 {
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
        assert_eq!(view.caps.depth_folded, 3);
        assert!(view.state.capped);
        let folded = view
            .columns
            .iter()
            .find(|column| column.depth == DEPTH_COLUMN_CAP)
            .expect("folded column");
        assert!(folded.folded);
        assert_eq!(folded.files.len(), 4);
        assert_eq!(folded.file_count, 4);
        assert!(
            view.columns
                .iter()
                .all(|column| column.depth <= DEPTH_COLUMN_CAP)
        );
    }

    #[test]
    fn deterministic_double_build() {
        let doc = application_fixture();
        let first = build_atlas_view(&doc);
        let second = build_atlas_view(&doc);
        assert_eq!(first, second);
        assert_eq!(
            format!("{first:?}"),
            format!("{second:?}"),
            "debug render must be byte-identical"
        );
    }
}
