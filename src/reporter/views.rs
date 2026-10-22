//! Deterministic key-view model (map-intelligibility V4).
//!
//! Pure functions over a finished [`MetricsDocument`]: no I/O, no clock, no
//! hashing. Every ordering has a total deterministic tie-break so
//! `report.md`, `report.html`, `findings-detail.*` and `profile-queue.*` all
//! derive the exact same rows from the same document.
//!
//! The three locked slots (V2, `docs/specs/map-intelligibility.md`):
//! - **KV1** — ranked maintainer brief (severity/rank), component-level
//!   `ARCH-00x` aggregates suppressed and counted.
//! - **KV2** — reverse include in-degree bars (coupling), **no**
//!   production-first demotion (V2 lock; rows label their path class).
//! - **KV3** — static-cost bars with production-first, startup and no-loop
//!   demotion (never exclusion).
//!
//! Units + anchors are mandatory for measured values: cost anchors carry
//! `static cost units (v2|v1)`, `× median` (only when the median is > 0) and
//! a `top P%` position inside the **same scale's** full cost ordering;
//! KV2 carries `N dependents`, `% of scanned files`, `top P%` and
//! `× median` (only when the median in-degree is > 0).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::{Confidence, Severity};
use crate::schema::{FunctionStats, MetricsDocument, format_fixed_point};

/// Cost scale of one row: v2 where `function_cost_v2` is present, v1
/// ([`crate::charts::function_cost`]) otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostScale {
    V2,
    V1,
}

impl CostScale {
    pub(crate) fn unit(self) -> &'static str {
        match self {
            CostScale::V2 => "static cost units (v2)",
            CostScale::V1 => "static cost units (v1)",
        }
    }
}

/// One ranked action in the KV1 brief / complete action list.
#[derive(Debug, Clone, PartialEq)]
pub struct Kv1Row {
    /// 1-based, contiguous over the action list (component aggregates are
    /// excluded from the list, so ranks never skip).
    pub rank: usize,
    /// Index into `doc.findings`.
    pub finding_index: usize,
    /// Index into `doc.stats.functions` for the narrowest containing function
    /// (`None` when no span contains the finding). The Atlas brief reuses this
    /// lookup instead of re-deriving it.
    pub function_index: Option<usize>,
    /// Containing function name (`None` when no span contains the finding).
    pub function: Option<String>,
    /// `files[].path_class` label (`unknown` when absent).
    pub path_class: String,
    pub production: bool,
    /// v2 cost of the containing function (the row's ranking anchor).
    pub cost: Option<u64>,
    /// `Some` exactly when a containing function exists.
    pub cost_scale: Option<CostScale>,
    /// Rendered unit + ratio + top-percentile anchor.
    pub cost_anchor: Option<String>,
}

/// KV1 brief: ranked actions (inline slice) plus honest counts.
#[derive(Debug, Clone, PartialEq)]
pub struct Kv1View {
    pub rows: Vec<Kv1Row>,
    /// Function-granularity findings only (component aggregates excluded).
    pub total_actions: usize,
    /// `-ARCH-002` hot-component aggregates suppressed from the ranking.
    pub component_findings: usize,
    /// Scanned files whose `path_class != production`.
    pub nonprod_file_count: usize,
}

/// One KV2 dependents bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Kv2Row {
    pub path: String,
    pub path_class: String,
    pub production: bool,
    /// Non-production row greyed and shown after the ranked production rows.
    pub demoted: bool,
    /// Distinct resolved includers (`from`) for this file.
    pub dependents: usize,
    /// Integer percentage of the largest shown row.
    pub width_pct: u32,
    /// Rendered `N dependent(s) · % of scanned files · top P% [· × median]`.
    pub value_label: String,
}

/// KV2 blast-radius view.
#[derive(Debug, Clone, PartialEq)]
pub struct Kv2View {
    pub rows: Vec<Kv2Row>,
    pub scanned_files: usize,
    pub resolved_edges: usize,
    pub total_edges: usize,
    /// Nearest-rank median over every scanned file, zeros included.
    pub median_in_degree: u64,
    /// Files with at least one resolved dependent (the inline cap's N).
    pub total_with_dependents: usize,
    pub nonprod_file_count: usize,
}

/// One KV3 static-cost row.
#[derive(Debug, Clone, PartialEq)]
pub struct Kv3Row {
    /// 1-based position in the full cost ordering (demoted rows included).
    pub rank: usize,
    /// Index into `doc.stats.functions`.
    pub function_index: usize,
    pub path: String,
    pub function: String,
    pub path_class: String,
    pub production: bool,
    pub cost: f64,
    pub cost_scale: CostScale,
    /// Rendered unit + ratio + top-percentile anchor.
    pub cost_anchor: String,
    /// Integer percentage of the largest cost in the row's own block
    /// (non-demoted vs demoted; clamped to 100).
    pub width_pct: u32,
    pub startup: bool,
    pub no_loop: bool,
    /// `non-production` / `startup` / `no-loop`, in that order.
    pub demotions: Vec<&'static str>,
    pub unknown_trip_loops: u32,
    pub known_trip_value: Option<u64>,
}

impl Kv3Row {
    /// True when any production/startup/no-loop demotion applies.
    #[must_use]
    pub fn is_demoted(&self) -> bool {
        !self.demotions.is_empty()
    }

    /// `startup + no-loop — demoted` (or `None` when not demoted).
    #[must_use]
    pub fn demotion_label(&self) -> Option<String> {
        if self.demotions.is_empty() {
            None
        } else {
            Some(format!("{} — demoted", self.demotions.join(" + ")))
        }
    }

    /// `unknown-trip ×N — estimate incomplete` when loops have no trip count.
    #[must_use]
    pub fn unknown_trip_badge(&self) -> Option<String> {
        if self.unknown_trip_loops > 0 {
            Some(format!(
                "unknown-trip ×{} — estimate incomplete",
                self.unknown_trip_loops
            ))
        } else {
            None
        }
    }

    /// `known trip ×N — verify it actually runs` for a known literal trip.
    #[must_use]
    pub fn known_trip_badge(&self) -> Option<String> {
        self.known_trip_value
            .map(|trip| format!("known trip ×{trip} — verify it actually runs"))
    }
}

/// KV3 cost view: inline rows plus honest demotion counts.
#[derive(Debug, Clone, PartialEq)]
pub struct Kv3View {
    pub rows: Vec<Kv3Row>,
    pub startup_count: usize,
    pub no_loop_count: usize,
    pub nonprod_file_count: usize,
    /// Complete cost-ranked list length (every function, demoted included).
    pub total_functions: usize,
}

// ---------------------------------------------------------------------------
// Deterministic helpers (single implementation shared by every renderer)
// ---------------------------------------------------------------------------

/// Nearest-rank median (`ceil(0.5·N)`-th smallest, 1-based) of `values`.
/// `None` for an empty slice; the caller decides whether a zero median is
/// meaningful (KV2/KV3 print `× median` only when it is > 0).
#[must_use]
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    Some(sorted[(sorted.len() - 1) / 2])
}

/// `2.1×` when `r < 10`, `12×` otherwise (no trailing decimal).
#[must_use]
pub fn ratio_label(r: f64) -> String {
    if r < 10.0 {
        format!("{}×", format_fixed_point(r, 1))
    } else {
        format!("{}×", format_fixed_point(r, 0))
    }
}

/// `1 dependent` / `n dependents` (regular `+s` nouns only — every count
/// noun in the key views is regular).
#[must_use]
pub fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

/// Noun-only plural form for headers that already render the number.
#[must_use]
pub fn plural_word(count: usize, singular: &'static str, many: &'static str) -> &'static str {
    if count == 1 { singular } else { many }
}

/// 1-based rank `k` of `n` → exact scan-relative `top {pct}%`
/// (`pct = max(1, round(100·k/n))`, capped at 100). No bucket floor: rank
/// `n` of `n` is honestly `top 100%`, not a misleading `top 25%`.
#[must_use]
pub fn top_label(k: usize, n: usize) -> String {
    if n == 0 {
        return "top 1%".to_string();
    }
    let pct = ((100 * k + n / 2) / n).clamp(1, 100);
    format!("top {pct}%")
}

/// Top-percentile anchor for a position inside a full cost ordering.
#[must_use]
pub fn cost_rank_label(rank: usize, total: usize) -> String {
    top_label(rank, total)
}

/// Only `-ARCH-002` ("Hot-component aggregation") is a directory/component
/// aggregate. `-ARCH-001` (include fan-out/cycles) and `-ARCH-003`
/// (cross-file call coupling) are FILE-level actions and stay in KV1.
fn is_hot_component_rule(rule_id: &str) -> bool {
    rule_id.ends_with("-ARCH-002")
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Hot => 0,
        Severity::Warn => 1,
        Severity::Info => 2,
    }
}

fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::High => 0,
        Confidence::Medium => 1,
        Confidence::Low => 2,
    }
}

pub(crate) fn scale_of(function: &FunctionStats) -> CostScale {
    if function.function_cost_v2.is_some() {
        CostScale::V2
    } else {
        CostScale::V1
    }
}

pub(crate) fn function_value(function: &FunctionStats) -> f64 {
    match function.function_cost_v2 {
        Some(cost) => cost as f64,
        None => crate::charts::function_cost(function),
    }
}

fn format_cost(value: f64, _scale: CostScale) -> String {
    format_fixed_point(value, 0)
}

/// Full ordering for one cost scale: cost desc, then path/name/id asc.
pub(crate) struct ScaleOrdering {
    median: Option<f64>,
    rank_by_id: BTreeMap<String, usize>,
    total: usize,
}

impl ScaleOrdering {
    pub(crate) fn build(functions: &[FunctionStats], scale: CostScale) -> Self {
        let mut members: Vec<&FunctionStats> =
            functions.iter().filter(|f| scale_of(f) == scale).collect();
        members.sort_by(|a, b| {
            function_value(b)
                .partial_cmp(&function_value(a))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.id.cmp(&b.id))
        });
        let values: Vec<f64> = members.iter().map(|f| function_value(f)).collect();
        let mut rank_by_id = BTreeMap::new();
        for (index, function) in members.iter().enumerate() {
            rank_by_id.insert(function.id.clone(), index + 1);
        }
        Self {
            median: median(&values),
            rank_by_id,
            total: members.len(),
        }
    }

    /// Nearest-rank median over this scale's members (zeros included).
    pub(crate) fn median(&self) -> Option<f64> {
        self.median
    }

    /// 1-based rank of `id` inside this scale's full cost ordering.
    pub(crate) fn rank(&self, id: &str) -> Option<usize> {
        self.rank_by_id.get(id).copied()
    }

    /// Number of functions in this scale's population.
    pub(crate) fn total(&self) -> usize {
        self.total
    }

    fn anchor(&self, function: &FunctionStats, scale: CostScale) -> String {
        let value = function_value(function);
        let mut parts = vec![format!("{} {}", format_cost(value, scale), scale.unit())];
        if let Some(median) = self.median
            && median > 0.0
        {
            parts.push(format!("{} median", ratio_label(value / median)));
        }
        if let Some(rank) = self.rank_by_id.get(&function.id) {
            parts.push(cost_rank_label(*rank, self.total));
        }
        parts.join(" · ")
    }
}

/// Shared per-document lookup tables (built once per view call).
struct Context<'a> {
    doc: &'a MetricsDocument,
    classes: BTreeMap<&'a str, &'a str>,
    functions_by_path: BTreeMap<&'a str, Vec<usize>>,
    v2: ScaleOrdering,
    v1: ScaleOrdering,
}

impl<'a> Context<'a> {
    fn new(doc: &'a MetricsDocument) -> Self {
        let mut classes = BTreeMap::new();
        for file in &doc.files {
            classes.insert(
                file.path.as_str(),
                file.path_class.as_deref().unwrap_or("unknown"),
            );
        }
        let mut functions_by_path: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (index, function) in doc.stats.functions.iter().enumerate() {
            functions_by_path
                .entry(function.path.as_str())
                .or_default()
                .push(index);
        }
        for bucket in functions_by_path.values_mut() {
            bucket.sort_by_key(|&index| {
                let function = &doc.stats.functions[index];
                (
                    function.span.start_line,
                    function.span.end_line,
                    function.id.clone(),
                )
            });
        }
        Self {
            classes,
            functions_by_path,
            v2: ScaleOrdering::build(&doc.stats.functions, CostScale::V2),
            v1: ScaleOrdering::build(&doc.stats.functions, CostScale::V1),
            doc,
        }
    }

    fn path_class(&self, path: &str) -> (String, bool) {
        let class = self.classes.get(path).copied().unwrap_or("unknown");
        (class.to_string(), class == "production")
    }

    /// Non-allocating production check for sort comparators.
    fn is_production(&self, path: &str) -> bool {
        self.classes.get(path).copied() == Some("production")
    }

    fn nonprod_file_count(&self) -> usize {
        self.doc
            .files
            .iter()
            .filter(|file| file.path_class.as_deref() != Some("production"))
            .count()
    }

    /// Narrowest function span containing the finding (ties: `id` asc).
    fn containing_function(&self, finding_index: usize) -> Option<usize> {
        let finding = &self.doc.findings[finding_index];
        let line = finding.span.start_line;
        self.functions_by_path
            .get(finding.path.as_str())?
            .iter()
            .copied()
            .filter(|&index| {
                let span = &self.doc.stats.functions[index].span;
                span.start_line <= line && line <= span.end_line
            })
            .min_by(|&a, &b| {
                let fa = &self.doc.stats.functions[a];
                let fb = &self.doc.stats.functions[b];
                (fa.span.end_line - fa.span.start_line)
                    .cmp(&(fb.span.end_line - fb.span.start_line))
                    .then_with(|| fa.id.cmp(&fb.id))
            })
    }

    fn ordering_for(&self, scale: CostScale) -> &ScaleOrdering {
        match scale {
            CostScale::V2 => &self.v2,
            CostScale::V1 => &self.v1,
        }
    }
}

// ---------------------------------------------------------------------------
// KV1 — ranked maintainer brief
// ---------------------------------------------------------------------------

fn build_kv1_rows(doc: &MetricsDocument) -> (Vec<Kv1Row>, usize, usize) {
    let context = Context::new(doc);
    let mut action_indices: Vec<usize> = Vec::new();
    let mut component_findings = 0;
    for (index, finding) in doc.findings.iter().enumerate() {
        if is_hot_component_rule(&finding.rule_id) {
            component_findings += 1;
        } else {
            action_indices.push(index);
        }
    }

    let cost_of = |index: usize| -> u64 {
        context
            .containing_function(index)
            .map(|function_index| {
                context.doc.stats.functions[function_index]
                    .function_cost_v2
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    };
    action_indices.sort_by(|&a, &b| {
        let fa = &doc.findings[a];
        let fb = &doc.findings[b];
        let (_, prod_a) = context.path_class(&fa.path);
        let (_, prod_b) = context.path_class(&fb.path);
        (!prod_a)
            .cmp(&(!prod_b))
            .then_with(|| severity_rank(fa.severity).cmp(&severity_rank(fb.severity)))
            .then_with(|| confidence_rank(fa.confidence).cmp(&confidence_rank(fb.confidence)))
            .then_with(|| cost_of(b).cmp(&cost_of(a)))
            .then_with(|| fa.finding_id.cmp(&fb.finding_id))
    });

    let mut rows = Vec::with_capacity(action_indices.len());
    for (rank, index) in action_indices.into_iter().enumerate() {
        let finding = &doc.findings[index];
        let (path_class, production) = context.path_class(&finding.path);
        let container = context.containing_function(index);
        let (function, cost, cost_scale, cost_anchor) = match container {
            Some(function_index) => {
                let function = &doc.stats.functions[function_index];
                let scale = scale_of(function);
                (
                    Some(function.name.clone()),
                    function.function_cost_v2,
                    Some(scale),
                    Some(context.ordering_for(scale).anchor(function, scale)),
                )
            }
            None => (None, None, None, None),
        };
        rows.push(Kv1Row {
            rank: rank + 1,
            finding_index: index,
            function_index: container,
            function,
            path_class,
            production,
            cost,
            cost_scale,
            cost_anchor,
        });
    }
    (rows, component_findings, context.nonprod_file_count())
}

/// KV1: ranked brief with at most five inline rows.
#[must_use]
pub fn kv1(doc: &MetricsDocument) -> Kv1View {
    let (mut rows, component_findings, nonprod_file_count) = build_kv1_rows(doc);
    let total_actions = rows.len();
    rows.truncate(5);
    Kv1View {
        rows,
        total_actions,
        component_findings,
        nonprod_file_count,
    }
}

/// Every function-granularity finding in KV1 order (drill artifact).
#[must_use]
pub fn complete_actions(doc: &MetricsDocument) -> Vec<Kv1Row> {
    build_kv1_rows(doc).0
}

// ---------------------------------------------------------------------------
// KV2 — reverse include in-degree (blast radius)
// ---------------------------------------------------------------------------

fn scale_width(value: f64, max: f64) -> u32 {
    if max <= 0.0 {
        0
    } else {
        ((100.0 * value / max) + 0.5).floor().clamp(0.0, 100.0) as u32
    }
}

/// KV2: dependents bars, production-first with demoted non-production rows.
#[must_use]
pub fn kv2(doc: &MetricsDocument) -> Kv2View {
    let context = Context::new(doc);
    let mut indegree: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut resolved_edges = 0usize;
    let mut total_edges = 0usize;
    if let Some(graphs) = &doc.graphs {
        total_edges = graphs.include_graph.edges.len();
        for edge in &graphs.include_graph.edges {
            let Some(to) = edge.to.as_deref() else {
                continue;
            };
            if edge.resolved {
                resolved_edges += 1;
                indegree.entry(to).or_default().insert(edge.from.as_str());
            }
        }
    }

    let scanned_files = doc.files.len();
    let mut ordering: Vec<(&str, usize)> = doc
        .files
        .iter()
        .map(|file| {
            (
                file.path.as_str(),
                indegree.get(file.path.as_str()).map_or(0, BTreeSet::len),
            )
        })
        .collect();
    ordering.sort_by(|a, b| {
        let prod_a = context.is_production(a.0);
        let prod_b = context.is_production(b.0);
        (!prod_a)
            .cmp(&(!prod_b))
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.0.cmp(b.0))
    });

    let ranks: BTreeMap<&str, usize> = ordering
        .iter()
        .enumerate()
        .map(|(index, (path, _))| (*path, index + 1))
        .collect();
    let all_values: Vec<f64> = ordering.iter().map(|(_, degree)| *degree as f64).collect();
    let median_in_degree = median(&all_values).unwrap_or(0.0) as u64;

    // Files with zero dependents are not a blast radius; omit them from the
    // inline bars (the median still counts them honestly).
    let with_dependents: Vec<(&str, usize)> = ordering
        .into_iter()
        .filter(|(_, degree)| *degree > 0)
        .collect();
    let total_with_dependents = with_dependents.len();
    // Up to eight production rows, then up to two demoted rows — the demoted
    // rows are never excluded from the ordering, only the inline slice.
    let mut shown: Vec<(&str, usize)> = with_dependents
        .iter()
        .filter(|(path, _)| context.is_production(path))
        .take(8)
        .copied()
        .collect();
    shown.extend(
        with_dependents
            .iter()
            .filter(|(path, _)| !context.is_production(path))
            .take(2)
            .copied(),
    );
    let max_degree = shown.iter().map(|(_, degree)| *degree).max().unwrap_or(0);
    let mut rows = Vec::with_capacity(shown.len());
    for (path, degree) in shown.drain(..) {
        let (path_class, production) = context.path_class(path);
        let percent = if scanned_files == 0 {
            0
        } else {
            (100 * degree + scanned_files / 2) / scanned_files
        };
        let mut parts = vec![
            plural(degree, "dependent"),
            format!("{percent}% of scanned files"),
            top_label(*ranks.get(path).unwrap_or(&1), scanned_files),
        ];
        if median_in_degree > 0 {
            parts.push(format!(
                "{} median",
                ratio_label(degree as f64 / median_in_degree as f64)
            ));
        }
        rows.push(Kv2Row {
            path: path.to_string(),
            path_class,
            production,
            demoted: !production,
            dependents: degree,
            width_pct: scale_width(degree as f64, max_degree as f64),
            value_label: parts.join(" · "),
        });
    }

    Kv2View {
        rows,
        scanned_files,
        resolved_edges,
        total_edges,
        median_in_degree,
        total_with_dependents,
        nonprod_file_count: context.nonprod_file_count(),
    }
}

// ---------------------------------------------------------------------------
// KV3 — static cost (profile queue)
// ---------------------------------------------------------------------------

fn build_kv3_rows(doc: &MetricsDocument) -> (Vec<Kv3Row>, usize, usize, usize) {
    let context = Context::new(doc);
    let mut indices: Vec<usize> = (0..doc.stats.functions.len()).collect();
    indices.sort_by(|&a, &b| {
        let fa = &doc.stats.functions[a];
        let fb = &doc.stats.functions[b];
        let (_, prod_a) = context.path_class(&fa.path);
        let (_, prod_b) = context.path_class(&fb.path);
        let no_loop_a = fa.max_loop_depth == 0;
        let no_loop_b = fb.max_loop_depth == 0;
        let startup_a = matches!(fa.entry_kind.as_deref(), Some("main" | "script"));
        let startup_b = matches!(fb.entry_kind.as_deref(), Some("main" | "script"));
        (!prod_a)
            .cmp(&(!prod_b))
            .then_with(|| no_loop_a.cmp(&no_loop_b))
            .then_with(|| startup_a.cmp(&startup_b))
            .then_with(|| {
                function_value(fb)
                    .partial_cmp(&function_value(fa))
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| fa.path.cmp(&fb.path))
            .then_with(|| fa.name.cmp(&fb.name))
            .then_with(|| fa.id.cmp(&fb.id))
    });

    // Dual bar scale: ranked and demoted rows are normalized within their
    // own block. A demoted outlier (test/startup/no-loop) must not flatten
    // every ranked bar to zero, and a demoted bar must not contradict its
    // printed cost.
    let mut max_non_demoted = 0.0_f64;
    let mut max_demoted = 0.0_f64;
    for &index in &indices {
        let function = &doc.stats.functions[index];
        let (_, production) = context.path_class(&function.path);
        let demoted = !production
            || function.max_loop_depth == 0
            || matches!(function.entry_kind.as_deref(), Some("main" | "script"));
        let cost = function_value(function);
        if demoted {
            max_demoted = max_demoted.max(cost);
        } else {
            max_non_demoted = max_non_demoted.max(cost);
        }
    }

    let mut rows = Vec::with_capacity(indices.len());
    for (position, index) in indices.into_iter().enumerate() {
        let function = &doc.stats.functions[index];
        let (path_class, production) = context.path_class(&function.path);
        let startup = matches!(function.entry_kind.as_deref(), Some("main" | "script"));
        let no_loop = function.max_loop_depth == 0;
        let mut demotions = Vec::new();
        if !production {
            demotions.push("non-production");
        }
        if startup {
            demotions.push("startup");
        }
        if no_loop {
            demotions.push("no-loop");
        }
        let scale = scale_of(function);
        let cost = function_value(function);
        let scale_max = if demotions.is_empty() {
            max_non_demoted
        } else {
            max_demoted
        };
        rows.push(Kv3Row {
            rank: position + 1,
            function_index: index,
            path: function.path.clone(),
            function: function.name.clone(),
            path_class,
            production,
            cost,
            cost_scale: scale,
            cost_anchor: context.ordering_for(scale).anchor(function, scale),
            width_pct: scale_width(cost, scale_max),
            startup,
            no_loop,
            demotions,
            unknown_trip_loops: function.unknown_trip_loops,
            known_trip_value: function.known_trip_value,
        });
    }

    let startup_count = doc
        .stats
        .functions
        .iter()
        .filter(|f| matches!(f.entry_kind.as_deref(), Some("main" | "script")))
        .count();
    let no_loop_count = doc
        .stats
        .functions
        .iter()
        .filter(|f| f.max_loop_depth == 0)
        .count();
    (
        rows,
        startup_count,
        no_loop_count,
        context.nonprod_file_count(),
    )
}

/// KV3: at most eight non-demoted rows plus up to two greyed demoted rows.
#[must_use]
pub fn kv3(doc: &MetricsDocument) -> Kv3View {
    let (all, startup_count, no_loop_count, nonprod_file_count) = build_kv3_rows(doc);
    let total_functions = all.len();
    let mut rows: Vec<Kv3Row> = all
        .iter()
        .filter(|row| !row.is_demoted())
        .take(8)
        .cloned()
        .collect();
    // The two greyed rows are the most expensive demoted functions (cost
    // desc), not simply the first two in the demotion tier.
    let mut demoted: Vec<Kv3Row> = all.iter().filter(|row| row.is_demoted()).cloned().collect();
    demoted.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.function.cmp(&b.function))
            .then_with(|| a.rank.cmp(&b.rank))
    });
    demoted.truncate(2);
    rows.extend(demoted);
    Kv3View {
        rows,
        startup_count,
        no_loop_count,
        nonprod_file_count,
        total_functions,
    }
}

/// Complete cost-ranked list (every function, demoted rows labelled).
#[must_use]
pub fn complete_profile_queue(doc: &MetricsDocument) -> Vec<Kv3Row> {
    build_kv3_rows(doc).0
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
    use crate::schema::{
        FileEntry, FindingEntry, GraphsSection, IncludeEdgeDto, IncludeGraph, ScanInfo,
        StatsSection, ToolInfo,
    };

    fn file(path: &str, class: &str) -> FileEntry {
        FileEntry {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts {
                code: 10,
                comment: 0,
                blank: 0,
            },
            function_count: 1,
            global_var_count: 0,
            include_counts: IncludeCounts {
                quoted: 0,
                angle: 0,
            },
            parse_health: crate::schema::ParseHealthEntry {
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
        start: u32,
        end: u32,
        cost: Option<u64>,
        loops: u32,
        entry: Option<&str>,
        unknown_trips: u32,
        known_trip: Option<u64>,
    ) -> FunctionStats {
        FunctionStats {
            id: format!("{path}#{name}#{start}:1"),
            name: name.to_string(),
            path: path.to_string(),
            span: Span {
                start_line: start,
                start_col: 1,
                end_line: end,
                end_col: 1,
            },
            param_count: 0,
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            max_loop_depth: loops,
            alloc_site_count: 0,
            length_lines: end - start + 1,
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
            function_cost_v2: cost,
            unknown_trip_loops: unknown_trips,
            trip_count_source: None,
            known_trip_value: known_trip,
        }
    }

    fn finding(
        rule: &str,
        path: &str,
        line: u32,
        severity: Severity,
        confidence: Confidence,
    ) -> FindingEntry {
        FindingEntry {
            finding_id: format!("{rule}:{path}:{line}:1"),
            rule_id: rule.to_string(),
            title: format!("Title {rule}"),
            path: path.to_string(),
            symbol: None,
            span: Span::point(line, 1),
            confidence,
            severity,
            resolver_tier: ResolverTier::Syntactic,
            macro_suspect: false,
            error_node_overlap: false,
            threshold: None,
            message: format!("Message {rule}"),
            evidence: BTreeMap::new(),
        }
    }

    fn edge(from: &str, to: Option<&str>, resolved: bool) -> IncludeEdgeDto {
        IncludeEdgeDto {
            from: from.to_string(),
            to: to.map(str::to_string),
            form: crate::ir::IncludeForm::Quoted,
            resolved,
            span: Span::point(1, 1),
        }
    }

    fn doc(
        files: Vec<FileEntry>,
        functions: Vec<FunctionStats>,
        findings: Vec<FindingEntry>,
        edges: Vec<IncludeEdgeDto>,
    ) -> MetricsDocument {
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
                file_count: files.len() as u32,
                loc_total: None,
            },
            parse_health_summary: None,
            files,
            stats: StatsSection {
                functions,
                distributions: None,
            },
            findings,
            graphs: Some(GraphsSection {
                include_graph: IncludeGraph {
                    nodes: Vec::new(),
                    edges,
                },
                call_graph: None,
            }),
            components: None,
        }
    }

    #[test]
    fn helpers_are_pinned() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[5.0, 1.0, 3.0, 9.0]), Some(3.0));
        assert_eq!(median(&[4.0, 1.0, 9.0]), Some(4.0));
        assert_eq!(ratio_label(2.125), "2.1×");
        assert_eq!(ratio_label(12.4), "12×");
        assert_eq!(top_label(1, 200), "top 1%");
        assert_eq!(top_label(1, 100), "top 1%");
        assert_eq!(top_label(9, 100), "top 9%");
        assert_eq!(top_label(50, 100), "top 50%");
        assert_eq!(top_label(12, 12), "top 100%");
        assert_eq!(cost_rank_label(1, 10), "top 10%");
        // Exact rank over the KV2 fixture: 1 of 3 ranks (incl. zero rows).
        assert_eq!(top_label(3, 3), "top 100%");
        assert_eq!(plural(0, "dependent"), "0 dependents");
        assert_eq!(plural(1, "dependent"), "1 dependent");
        assert_eq!(plural(2, "file"), "2 files");
        assert_eq!(plural_word(1, "finding", "findings"), "finding");
        assert_eq!(plural_word(2, "finding", "findings"), "findings");
    }

    #[test]
    fn kv1_ranks_production_first_then_severity_confidence_cost() {
        let files = vec![file("src/a.c", "production"), file("tests/t.c", "test")];
        let functions = vec![
            function("src/a.c", "hot_fn", 1, 20, Some(100), 1, None, 0, None),
            function("tests/t.c", "test_fn", 1, 20, Some(900), 1, None, 0, None),
        ];
        let findings = vec![
            finding(
                "PERF-C-ALLOC-001",
                "tests/t.c",
                5,
                Severity::Hot,
                Confidence::High,
            ),
            finding(
                "PERF-C-STR-001",
                "src/a.c",
                5,
                Severity::Warn,
                Confidence::High,
            ),
            finding(
                "PERF-C-ALLOC-002",
                "src/a.c",
                5,
                Severity::Hot,
                Confidence::High,
            ),
            finding(
                "PERF-C-ARCH-002",
                "src/a.c",
                5,
                Severity::Hot,
                Confidence::High,
            ),
            // File-level ARCH rules stay in KV1 — only ARCH-002 is the
            // directory/component aggregate.
            finding(
                "PERF-C-ARCH-001",
                "src/a.c",
                5,
                Severity::Info,
                Confidence::High,
            ),
            finding(
                "PERF-C-ARCH-003",
                "src/a.c",
                5,
                Severity::Warn,
                Confidence::High,
            ),
        ];
        let doc = doc(files, functions, findings, Vec::new());
        let view = kv1(&doc);
        assert_eq!(view.component_findings, 1);
        assert_eq!(view.total_actions, 5);
        assert_eq!(view.rows.len(), 5);
        // Production first, then hot → warn → info; only ARCH-002 suppressed.
        assert_eq!(
            view.rows
                .iter()
                .map(|r| r.finding_index)
                .collect::<Vec<_>>(),
            vec![2, 5, 1, 4, 0]
        );
        assert_eq!(
            view.rows.iter().map(|r| r.rank).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
        let action_ids: Vec<&str> = view
            .rows
            .iter()
            .map(|r| doc.findings[r.finding_index].rule_id.as_str())
            .collect();
        assert!(action_ids.contains(&"PERF-C-ARCH-001"));
        assert!(action_ids.contains(&"PERF-C-ARCH-003"));
        assert!(!action_ids.contains(&"PERF-C-ARCH-002"));
        assert_eq!(view.nonprod_file_count, 1);
        assert_eq!(view.rows[0].function.as_deref(), Some("hot_fn"));
        assert_eq!(view.rows[0].cost, Some(100));
        assert!(
            view.rows[0]
                .cost_anchor
                .as_deref()
                .unwrap()
                .contains("static cost units (v2)")
        );
    }

    #[test]
    fn kv3_dual_scale_keeps_ranked_bars_visible_with_demoted_outlier() {
        let files = vec![file("src/a.c", "production")];
        let functions = vec![
            function("src/a.c", "ranked_top", 1, 10, Some(100), 1, None, 0, None),
            function(
                "src/a.c",
                "ranked_second",
                20,
                30,
                Some(50),
                1,
                None,
                0,
                None,
            ),
            function(
                "src/a.c",
                "leaf_outlier",
                40,
                50,
                Some(1_000_000),
                0,
                None,
                0,
                None,
            ),
            function(
                "src/a.c",
                "main_outlier",
                60,
                70,
                Some(500_000),
                1,
                Some("main"),
                0,
                None,
            ),
        ];
        let view = kv3(&doc(files, functions, Vec::new(), Vec::new()));
        let width = |name: &str| {
            view.rows
                .iter()
                .find(|row| row.function == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .width_pct
        };
        // Ranked block: 100/50 against max 100.
        assert_eq!(width("ranked_top"), 100);
        assert_eq!(width("ranked_second"), 50);
        // Demoted block: 1,000,000 / 500,000 against max 1,000,000.
        assert_eq!(width("leaf_outlier"), 100);
        assert_eq!(width("main_outlier"), 50);
        // Demoted rows still cost-desc ordered.
        let demoted: Vec<&str> = view
            .rows
            .iter()
            .filter(|row| row.is_demoted())
            .map(|row| row.function.as_str())
            .collect();
        assert_eq!(demoted, vec!["leaf_outlier", "main_outlier"]);
    }

    #[test]
    fn kv1_containing_function_uses_narrowest_span_and_v1_fallback() {
        let files = vec![file("src/a.c", "production")];
        let functions = vec![
            function("src/a.c", "outer", 1, 50, Some(10), 1, None, 0, None),
            function("src/a.c", "inner", 10, 20, None, 1, None, 0, None),
        ];
        let findings = vec![finding(
            "PERF-C-ALLOC-001",
            "src/a.c",
            15,
            Severity::Hot,
            Confidence::High,
        )];
        let row = complete_actions(&doc(files, functions, findings, Vec::new())).remove(0);
        assert_eq!(row.function.as_deref(), Some("inner"));
        assert_eq!(row.cost, None);
        assert_eq!(row.cost_scale, Some(CostScale::V1));
        assert!(
            row.cost_anchor
                .as_deref()
                .unwrap()
                .contains("static cost units (v1)")
        );
    }

    #[test]
    fn kv1_without_containing_function_has_no_cost_anchor() {
        let files = vec![file("src/a.c", "production")];
        let findings = vec![finding(
            "PERF-C-ALLOC-001",
            "src/a.c",
            5,
            Severity::Hot,
            Confidence::High,
        )];
        let row = complete_actions(&doc(files, Vec::new(), findings, Vec::new())).remove(0);
        assert_eq!(row.function, None);
        assert_eq!(row.cost_scale, None);
        assert_eq!(row.cost_anchor, None);
    }

    #[test]
    fn kv2_demotes_non_production_without_excluding() {
        let files = vec![
            file("src/a.c", "production"),
            file("src/b.c", "production"),
            file("tests/t.c", "test"),
        ];
        let edges = vec![
            edge("tests/t.c", Some("src/a.c"), true),
            edge("src/b.c", Some("src/a.c"), true),
            edge("src/b.c", Some("src/a.c"), true), // duplicate pair: distinct
            edge("tests/t.c", Some("src/b.c"), true),
            edge("src/a.c", Some("tests/t.c"), true),
            edge("src/b.c", None, false),
        ];
        let view = kv2(&doc(files, Vec::new(), Vec::new(), edges));
        assert_eq!(view.total_edges, 6);
        assert_eq!(view.resolved_edges, 5);
        assert_eq!(view.scanned_files, 3);
        assert_eq!(view.median_in_degree, 1);
        assert_eq!(view.total_with_dependents, 3);
        // Production ranks first even at equal degree (path asc tie-break).
        assert_eq!(view.rows[0].path, "src/a.c");
        assert_eq!(view.rows[0].dependents, 2);
        assert!(!view.rows[0].demoted);
        assert_eq!(view.rows[1].path, "src/b.c");
        assert!(!view.rows[1].demoted);
        assert!(view.rows[0].value_label.contains("% of scanned files"));
        assert!(view.rows[0].value_label.contains("top "));
        // The test file is demoted but never excluded.
        let test_row = view
            .rows
            .iter()
            .find(|r| r.path == "tests/t.c")
            .expect("demoted row must stay in the inline slice");
        assert!(!test_row.production);
        assert!(test_row.demoted);
        assert_eq!(test_row.dependents, 1);
        assert_eq!(view.nonprod_file_count, 1);
    }

    #[test]
    fn kv2_test_file_with_more_dependents_ranks_below_production() {
        let files = vec![file("src/p.c", "production"), file("tests/t.c", "test")];
        let edges = vec![
            edge("src/a.c", Some("src/p.c"), true),
            edge("src/a.c", Some("tests/t.c"), true),
            edge("src/b.c", Some("tests/t.c"), true),
            edge("src/c.c", Some("tests/t.c"), true),
        ];
        let view = kv2(&doc(files, Vec::new(), Vec::new(), edges));
        assert_eq!(view.total_with_dependents, 2);
        assert_eq!(view.rows[0].path, "src/p.c");
        assert_eq!(view.rows[0].dependents, 1);
        assert!(view.rows[0].value_label.contains("top 50%"));
        assert_eq!(view.rows[1].path, "tests/t.c");
        assert_eq!(view.rows[1].dependents, 3);
        assert!(view.rows[1].demoted);
        // Ranked position follows the production-first ordering, not raw degree.
        assert!(view.rows[1].value_label.contains("top 100%"));
        // Median still counts every scanned file, zeros included.
        assert_eq!(view.median_in_degree, 1);
    }

    #[test]
    fn kv2_slots_eight_production_then_two_demoted() {
        let mut files: Vec<FileEntry> = (0..10)
            .map(|index| file(&format!("src/p{index}.c"), "production"))
            .collect();
        files.extend((0..3).map(|index| file(&format!("tests/t{index}.c"), "test")));
        let mut edges = Vec::new();
        for entry in &files {
            edges.push(edge("src/hub.c", Some(&entry.path), true));
        }
        let view = kv2(&doc(files, Vec::new(), Vec::new(), edges));
        assert_eq!(view.total_with_dependents, 13);
        assert_eq!(view.rows.len(), 10);
        let production: Vec<&str> = view
            .rows
            .iter()
            .take(8)
            .map(|row| row.path.as_str())
            .collect();
        assert_eq!(
            production,
            vec![
                "src/p0.c", "src/p1.c", "src/p2.c", "src/p3.c", "src/p4.c", "src/p5.c", "src/p6.c",
                "src/p7.c",
            ]
        );
        assert!(view.rows.iter().take(8).all(|row| !row.demoted));
        let demoted: Vec<&str> = view
            .rows
            .iter()
            .skip(8)
            .map(|row| row.path.as_str())
            .collect();
        assert_eq!(demoted, vec!["tests/t0.c", "tests/t1.c"]);
        assert!(view.rows.iter().skip(8).all(|row| row.demoted));
        assert_eq!(view.nonprod_file_count, 3);
    }

    #[test]
    fn kv3_demotes_without_excluding_and_anchors_v1_separately() {
        let files = vec![file("src/a.c", "production"), file("tests/t.c", "test")];
        let functions = vec![
            function("src/a.c", "looper", 1, 10, Some(100), 2, None, 1, Some(4)),
            function("src/a.c", "leaf", 20, 30, Some(900), 0, None, 0, None),
            function(
                "src/a.c",
                "main",
                40,
                50,
                Some(500),
                1,
                Some("main"),
                0,
                None,
            ),
            function("tests/t.c", "helper", 1, 10, Some(700), 1, None, 0, None),
            function("src/a.c", "v1only", 60, 70, None, 1, None, 0, None),
        ];
        let view = kv3(&doc(files, functions, Vec::new(), Vec::new()));
        assert_eq!(view.total_functions, 5);
        assert_eq!(view.startup_count, 1);
        assert_eq!(view.no_loop_count, 1);
        assert_eq!(view.nonprod_file_count, 1);
        // Production + loop-bearing first: looper, v1only, then demoted rows.
        assert_eq!(view.rows[0].function, "looper");
        assert!(!view.rows[0].is_demoted());
        assert_eq!(
            view.rows[0].unknown_trip_badge().as_deref(),
            Some("unknown-trip ×1 — estimate incomplete")
        );
        assert_eq!(
            view.rows[0].known_trip_badge().as_deref(),
            Some("known trip ×4 — verify it actually runs")
        );
        let demoted: Vec<&str> = view
            .rows
            .iter()
            .filter(|row| row.is_demoted())
            .map(|row| row.function.as_str())
            .collect();
        // The two most expensive demoted rows, cost desc: leaf 900 > helper 700.
        assert_eq!(demoted, vec!["leaf", "helper"]);
        let leaf = view
            .rows
            .iter()
            .find(|row| row.function == "leaf")
            .expect("leaf row");
        let helper = view
            .rows
            .iter()
            .find(|row| row.function == "helper")
            .expect("helper row");
        assert_eq!(leaf.width_pct, 100);
        assert_eq!(helper.width_pct, 78);
        assert!(leaf.is_demoted() && helper.is_demoted());
        let complete = complete_profile_queue(&doc(
            vec![file("src/a.c", "production"), file("tests/t.c", "test")],
            vec![
                function("src/a.c", "looper", 1, 10, Some(100), 2, None, 0, None),
                function("src/a.c", "leaf", 20, 30, Some(900), 0, None, 0, None),
                function(
                    "src/a.c",
                    "main",
                    40,
                    50,
                    Some(500),
                    1,
                    Some("main"),
                    0,
                    None,
                ),
                function("tests/t.c", "helper", 1, 10, Some(700), 1, None, 0, None),
            ],
            Vec::new(),
            Vec::new(),
        ));
        assert!(complete.iter().any(|row| row.function == "helper"));
        assert_eq!(complete.len(), 4);
        let v1 = complete_profile_queue(&doc(
            vec![file("src/a.c", "production")],
            vec![function("src/a.c", "v1only", 1, 10, None, 1, None, 0, None)],
            Vec::new(),
            Vec::new(),
        ));
        assert_eq!(v1[0].cost_scale, CostScale::V1);
        assert!(v1[0].cost_anchor.contains("static cost units (v1)"));
        assert!(v1[0].cost_anchor.contains("(v1)"));
    }
}
