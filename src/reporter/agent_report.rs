//! Agent-facing depth artifacts (Track C, contract version 1).
//!
//! Two pure renderers over a finished [`MetricsDocument`]:
//! - [`render_actions_json`] — the complete, uncapped machine action list plus
//!   the top-100 profile queue (`agent-actions.json`);
//! - [`render_markdown`] — the bounded, fixed seven-section LLM triage view
//!   (`agent-report.md`).
//!
//! Both are pure functions of the document: no I/O, clock, environment,
//! locale, network or randomness. Ordering is the shipped key-view ordering
//! ([`crate::reporter::views::complete_actions`] / `complete_profile_queue`);
//! cost anchors are reused verbatim so the artifacts can never disagree with
//! the human report. Contract: `docs/specs/agent-report.md`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::analyzer::{Confidence, Severity};
use crate::ir::{LocCounts, ResolverTier};
use crate::reporter::finding_anchor_assignments;
use crate::reporter::format::{md_cell, md_code, md_text};
use crate::reporter::views::{
    self, CostScale, Kv1Row, Kv3Row, complete_actions, complete_profile_queue,
};
use crate::schema::{MetricsDocument, ParseHealthSummary, Threshold};

/// Contract version carried by the JSON envelope (§6). Bumped only by a
/// contract change — never by a renderer refactor.
pub const CONTRACT_VERSION: u32 = 1;
/// Markdown detail cap (§7 / §10 `caps.actions_detail`).
pub const ACTIONS_DETAIL_CAP: usize = 25;
/// Markdown inline profile cap (§7 / §10 `caps.profile_inline`).
pub const PROFILE_INLINE_CAP: usize = 25;
/// JSON profile-queue cap (the authoritative machine cap, named in-file).
pub const PROFILE_QUEUE_CAP: usize = 100;
/// Hard Markdown byte budget on the named fixtures and the §12 envelope
/// (80 KB, amended at the C2 kickoff).
pub const MD_SIZE_BUDGET: usize = 80 * 1024;

/// Named stubs (§7). Absent data never becomes a guessed value.
pub const STUB_NO_FINDINGS: &str = "no findings — nothing to rank on this scan.";
pub const STUB_NO_FUNCTIONS: &str = "no functions recorded — nothing to queue.";
pub const STUB_NO_RULES: &str = "no rules fired — nothing to index.";
pub const STUB_NO_FUNCTION: &str = "no enclosing function recorded for this location";
pub const STUB_NO_HINT: &str = "no curated hint for this rule yet";
pub const STUB_NO_PARSE: &str = "parse: not recorded";

/// Machine-readable action rank key (§5.1), emitted verbatim in the envelope.
pub const ACTION_RANK_KEY: [&str; 5] = [
    "path_class:production-first",
    "severity:hot>warn>info",
    "confidence:HIGH>MEDIUM>LOW",
    "function_cost_v2:desc",
    "finding_id:asc",
];

/// Machine-readable profile-queue rank key (§5.2), emitted verbatim.
pub const PROFILE_RANK_KEY: [&str; 7] = [
    "path_class:production-first",
    "max_loop_depth>0:first",
    "entry_kind:non-startup-first",
    "cost:desc",
    "path:asc",
    "function_name:asc",
    "function_id:asc",
];

/// Curated one-sentence fix hint per catalog rule (spec §8).
///
/// Data, not logic: the renderer never invents a fix. A completeness test
/// locks this key set against `HEURISTIC_CATALOG_V1` so a new catalog rule
/// without a hint fails the build. `PERF-RS-STR-001` is retired and absent.
pub const AGENT_FIX_HINTS: &[(&str, &str)] = &[
    (
        "PERF-C-ALLOC-001",
        "Hoist the allocation out of the loop, reuse a buffer across iterations, or reserve once before the loop.",
    ),
    (
        "PERF-C-ALLOC-002",
        "Move the free outside the loop or reuse a single allocation per iteration instead of freeing each pass.",
    ),
    (
        "PERF-C-ALLOC-003",
        "Replace additive realloc growth with geometric growth (double capacity) or one pre-sized allocation.",
    ),
    (
        "PERF-C-STR-001",
        "Track the destination length and append with bounded writes instead of rescanning the string each pass.",
    ),
    (
        "PERF-C-STR-002",
        "Compute the length once before the loop and reuse it while the receiver is unchanged.",
    ),
    (
        "PERF-C-STR-003",
        "Bound every copy with an explicit destination size (snprintf/strlcpy) and pre-size the buffer once.",
    ),
    (
        "PERF-C-LOOP-001",
        "Flatten the nesting, move loop-invariant work outward, or replace the innermost scan with a lookup table.",
    ),
    (
        "PERF-C-LOOP-002",
        "Hoist handle or regex setup out of the loop, or batch element-wise transfers into block reads and writes.",
    ),
    (
        "PERF-C-LOOP-003",
        "Read or write in blocks with fread/fwrite instead of one character per call.",
    ),
    (
        "PERF-C-LOOP-004",
        "Hoist the divisor when it is loop-invariant, or precompute its reciprocal once.",
    ),
    (
        "PERF-C-CALL-001",
        "Add an iterative form or memoise results so the same inputs are not recomputed down the call chain.",
    ),
    (
        "PERF-C-CALL-002",
        "Extract branches into helpers, replace long dispatch chains with a table, and return early.",
    ),
    (
        "PERF-C-ARCH-001",
        "Break the cycle by moving shared declarations into a leaf header and reduce fan-out through a facade.",
    ),
    (
        "PERF-C-ARCH-002",
        "Treat the hottest component as the first refactor target: split its costliest files and narrow its interface.",
    ),
    (
        "PERF-C-ARCH-003",
        "Confirm the coupled edge with a semantic check, then narrow the interface or pass the dependency in.",
    ),
    (
        "PERF-C-TYPE-001",
        "Pass large structs by pointer (or const reference in C++) instead of by value.",
    ),
    (
        "PERF-C-TYPE-002",
        "Reorder struct members large-to-small to reduce padding, and keep the declared layout when it is an ABI contract.",
    ),
    (
        "PERF-C-MEM-001",
        "Size allocations as element count times element size, and use the pointee type inside sizeof.",
    ),
    (
        "PERF-C-MEM-002",
        "Hoist constant-size block copies out of the loop, or copy whole blocks instead of per-iteration slices.",
    ),
    (
        "PERF-C-LOOP-005",
        "Accumulate into a local and write the global once after the loop.",
    ),
    (
        "PERF-C-LOOP-006",
        "Hoist the function-pointer lookup out of the loop and prefer a direct call in the hot body.",
    ),
    (
        "PERF-C-LOOP-007",
        "Replace the linear scan with a hash set, a sorted lookup, or a prebuilt index.",
    ),
    (
        "PERF-C-LOOP-008",
        "Cache the dereferenced pointer in a local and keep the hot chain contiguous in memory.",
    ),
    (
        "PERF-PY-ALLOC-001",
        "Build the loop-invariant container, string, or object once before the loop and reuse it.",
    ),
    (
        "PERF-PY-ALLOC-003",
        "Append to one accumulator, or extend from a generator, instead of rebuilding and copying the list each pass.",
    ),
    (
        "PERF-PY-ALLOC-004",
        "Keep the iterable lazy (generator or iterator) and materialise it once outside the loop.",
    ),
    (
        "PERF-PY-ARCH-001",
        "Break the import cycle by moving shared symbols into a lower module both sides can import.",
    ),
    (
        "PERF-PY-ARCH-002",
        "Start with the hottest package: split its largest modules and narrow their interfaces.",
    ),
    (
        "PERF-PY-ARCH-003",
        "Add type annotations or a semantic check to confirm the coupled name, then inject the dependency.",
    ),
    (
        "PERF-PY-ASYNC-001",
        "Offload the blocking call with asyncio.to_thread or switch to the client's async API.",
    ),
    (
        "PERF-PY-ASYNC-002",
        "Collect the awaitables and await them together with asyncio.gather when order is not required.",
    ),
    (
        "PERF-PY-CALL-001",
        "Rewrite the recursion iteratively or memoise results to bound recursion depth and call cost.",
    ),
    (
        "PERF-PY-CALL-002",
        "Split branches into helpers, use dispatch tables, and return early on the common path.",
    ),
    (
        "PERF-PY-COMPREHENSION-001",
        "Replace the append loop with a list comprehension.",
    ),
    (
        "PERF-PY-EXECUTOR-001",
        "Create the executor or event loop once outside the loop and submit work to it.",
    ),
    (
        "PERF-PY-LOOP-001",
        "Flatten the nesting, hoist invariant work, or replace the innermost scan with a set or dict.",
    ),
    (
        "PERF-PY-LOOP-002",
        "Hoist handle setup out of the loop and batch element-wise reads and writes; keep files open across iterations.",
    ),
    (
        "PERF-PY-LOOP-003",
        "Iterate the sequence directly or use enumerate instead of range(len(seq)).",
    ),
    (
        "PERF-PY-LOOP-007",
        "Use a set or dict for membership, or build an index once instead of scanning the list per iteration.",
    ),
    (
        "PERF-PY-LOOP-009",
        "Move the try/except outside the loop when the failure is not per-element, or isolate the risky step.",
    ),
    (
        "PERF-PY-MEM-002",
        "Copy once before the loop, or mutate in place through a view instead of copying per iteration.",
    ),
    (
        "PERF-PY-N1-001",
        "Batch the per-iteration round trip into one query with IN (...) or a JOIN, or one bulk HTTP request.",
    ),
    (
        "PERF-PY-PANDAS-001",
        "Vectorise the row operation with column arithmetic or numpy.where instead of iterrows or apply.",
    ),
    (
        "PERF-PY-PANDAS-002",
        "Collect the frames in a list and call pandas.concat once after the loop.",
    ),
    (
        "PERF-PY-STR-001",
        "Collect the pieces in a list and join once after the loop.",
    ),
    (
        "PERF-RS-ALLOC-001",
        "Hoist construction out of the loop, reuse one buffer, or reserve capacity once before the loop.",
    ),
    (
        "PERF-RS-ALLOC-002",
        "Drop once outside the loop, or reuse the value instead of dropping it each iteration.",
    ),
    (
        "PERF-RS-ALLOC-003",
        "Reserve capacity once before the loop, or grow it multiplicatively, instead of reserving per pass.",
    ),
    (
        "PERF-RS-ARCH-001",
        "Split the module chain, move shared items into a leaf module, and break the cycle.",
    ),
    (
        "PERF-RS-ARCH-002",
        "Refactor the hottest component first: split its costliest modules and reduce cross-module calls.",
    ),
    (
        "PERF-RS-ARCH-003",
        "Resolve the callee explicitly (direct import or trait bound) and narrow the cross-module call.",
    ),
    (
        "PERF-RS-CALL-001",
        "Use an explicit worklist or memoise to remove the recursive call from the hot path.",
    ),
    (
        "PERF-RS-CALL-002",
        "Split the function, replace match and if chains with dispatch, and return early.",
    ),
    (
        "PERF-RS-LOOP-001",
        "Flatten the nesting, hoist invariant work, or replace the innermost scan with a set.",
    ),
    (
        "PERF-RS-LOOP-002",
        "Hoist setup (File, Command, env) out of the loop and batch streaming calls into block reads or writes.",
    ),
    (
        "PERF-RS-LOOP-003",
        "Read into a buffer and process chunks instead of one character or byte per iteration.",
    ),
    (
        "PERF-RS-LOOP-004",
        "Hoist the divisor when it is loop-invariant, or precompute its reciprocal once.",
    ),
    (
        "PERF-RS-LOOP-005",
        "Accumulate in a local and write the static once after the loop.",
    ),
    (
        "PERF-RS-LOOP-006",
        "Hoist the function-pointer or trait-object dispatch out of the loop, or monomorphise the hot call.",
    ),
    (
        "PERF-RS-LOOP-007",
        "Use a HashSet or BTreeSet for membership, or build an index once before the loop.",
    ),
    (
        "PERF-RS-LOOP-008",
        "Cache the dereferenced field in a local and keep the hot data contiguous (Vec over linked nodes).",
    ),
    (
        "PERF-RS-MEM-001",
        "Divide the byte count by size_of::<T>() when passing it as an element count.",
    ),
    (
        "PERF-RS-MEM-002",
        "Copy the block once before the loop, or extend_from_slice into a pre-sized target.",
    ),
    (
        "PERF-RS-STR-002",
        "Compute len, is_empty, or capacity once before the loop and reuse the value.",
    ),
    (
        "PERF-RS-STR-003",
        "Replace the unsafe unbounded operation with a checked slice API, or assert the length at the boundary.",
    ),
    (
        "PERF-RS-TYPE-001",
        "Pass large structs by reference (&T) instead of by value.",
    ),
    (
        "PERF-RS-TYPE-002",
        "Order fields large-to-small, or use #[repr(C)] when the layout is part of an FFI contract.",
    ),
    (
        "PERF-JS-ALLOC-001",
        "Hoist the loop-invariant literal outside the loop, or reuse one object and reset its fields per pass.",
    ),
    (
        "PERF-JS-ALLOC-003",
        "Replace repeated concat or spread rebuilding with one accumulator and a single join at the end.",
    ),
    (
        "PERF-JS-STR-001",
        "Collect parts in an array and join once, or use a pre-sized buffer outside the loop.",
    ),
    (
        "PERF-JS-LOOP-001",
        "Flatten the nesting, hoist invariant work, or replace the inner scan with a lookup map.",
    ),
    (
        "PERF-JS-LOOP-002",
        "Batch the I/O outside the loop, or use the asynchronous API with bounded concurrency.",
    ),
    (
        "PERF-JS-LOOP-007",
        "Use a Set or Map for membership, or build an index once before the loop.",
    ),
    (
        "PERF-JS-ASYNC-001",
        "Collect the promises and await Promise.all once, or batch the iterations deliberately.",
    ),
    (
        "PERF-JS-ASYNC-002",
        "Replace the synchronous Node API with its asynchronous counterpart, or move it off the hot path.",
    ),
    (
        "PERF-JS-CALL-001",
        "Add an explicit base case or reduce the input before the recursive call.",
    ),
    (
        "PERF-JS-ARCH-001",
        "Break the cycle by extracting the shared module, or invert one edge behind an interface.",
    ),
    (
        "PERF-JS-TYPE-001",
        "Set the property to undefined only if read semantics allow, or rebuild the object without it.",
    ),
];

/// Curated hint for `rule_id`, or `None` when the rule is absent from the
/// table (JSON `fix_hint: null`; Markdown prints [`STUB_NO_HINT`]).
#[must_use]
pub fn fix_hint(rule_id: &str) -> Option<&'static str> {
    AGENT_FIX_HINTS
        .iter()
        .find_map(|(id, hint)| (*id == rule_id).then_some(*hint))
}

// ---------------------------------------------------------------------------
// JSON number handling
// ---------------------------------------------------------------------------

/// Cost serialized as a JSON number: integral values print without a
/// fractional part (`15`, never `15.0`), matching the committed sample.
struct CostNumber(f64);

impl Serialize for CostNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0.fract() == 0.0 && self.0 >= 0.0 && self.0 <= u64::MAX as f64 {
            serde_json::Number::from(self.0 as u64).serialize(serializer)
        } else {
            serializer.serialize_f64(self.0)
        }
    }
}

// ---------------------------------------------------------------------------
// Data quality (§10)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize)]
struct ParseHealthView {
    ok: u32,
    partial: u32,
    failed: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct IncludeQuality {
    unresolved: usize,
    total: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct CallQuality {
    total: usize,
    high: usize,
    medium: usize,
    low: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct CostScaleView {
    v2_functions: usize,
    v1_functions: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct CapsView {
    actions_detail: usize,
    profile_inline: usize,
}

/// The §10 `data_quality` object, shared by both artifacts so the Markdown
/// and JSON can never disagree.
#[derive(Debug, Clone, Serialize)]
struct DataQuality {
    parse_health: Option<ParseHealthView>,
    unresolved_includes: IncludeQuality,
    structural_call_edges: CallQuality,
    non_production_files: usize,
    startup_functions: usize,
    no_loop_functions: usize,
    cost_scale: CostScaleView,
    unknown_trip_functions: usize,
    component_findings_excluded: usize,
    resolver_tiers: Vec<String>,
    caps: CapsView,
}

fn resolver_tier_token(tier: ResolverTier) -> &'static str {
    match tier {
        ResolverTier::Syntactic => "syntactic",
        ResolverTier::Semantic => "semantic",
        ResolverTier::Dynamic => "dynamic",
    }
}

fn severity_token(severity: Severity) -> &'static str {
    match severity {
        Severity::Hot => "hot",
        Severity::Warn => "warn",
        Severity::Info => "info",
    }
}

fn confidence_token(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "HIGH",
        Confidence::Medium => "MEDIUM",
        Confidence::Low => "LOW",
    }
}

fn scale_token(scale: CostScale) -> &'static str {
    match scale {
        CostScale::V2 => "v2",
        CostScale::V1 => "v1",
    }
}

fn build_data_quality(doc: &MetricsDocument) -> DataQuality {
    let parse_health = doc.parse_health_summary.map(|summary| {
        let ParseHealthSummary {
            ok,
            partial,
            failed,
        } = summary;
        ParseHealthView {
            ok,
            partial,
            failed,
        }
    });

    let (unresolved_includes, structural_call_edges) = match &doc.graphs {
        Some(graphs) => {
            let include_edges = &graphs.include_graph.edges;
            let unresolved = include_edges.iter().filter(|edge| !edge.resolved).count();
            let mut calls = CallQuality {
                total: 0,
                high: 0,
                medium: 0,
                low: 0,
            };
            if let Some(call_graph) = &graphs.call_graph {
                for edge in &call_graph.edges {
                    calls.total += 1;
                    match edge.confidence.as_str() {
                        "HIGH" => calls.high += 1,
                        "MEDIUM" => calls.medium += 1,
                        _ => calls.low += 1,
                    }
                }
            }
            (
                IncludeQuality {
                    unresolved,
                    total: include_edges.len(),
                },
                calls,
            )
        }
        None => (
            IncludeQuality {
                unresolved: 0,
                total: 0,
            },
            CallQuality {
                total: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
        ),
    };

    let functions = &doc.stats.functions;
    let v2_functions = functions
        .iter()
        .filter(|function| function.function_cost_v2.is_some())
        .count();
    let non_production_files = doc
        .files
        .iter()
        .filter(|file| file.path_class.as_deref() != Some("production"))
        .count();
    let startup_functions = functions
        .iter()
        .filter(|function| matches!(function.entry_kind.as_deref(), Some("main" | "script")))
        .count();
    let no_loop_functions = functions
        .iter()
        .filter(|function| function.max_loop_depth == 0)
        .count();
    let unknown_trip_functions = functions
        .iter()
        .filter(|function| function.unknown_trip_loops > 0)
        .count();
    let component_findings_excluded = doc
        .findings
        .iter()
        .filter(|finding| finding.rule_id.ends_with("-ARCH-002"))
        .count();

    let mut tiers: BTreeSet<&'static str> = doc
        .findings
        .iter()
        .map(|finding| resolver_tier_token(finding.resolver_tier))
        .collect();
    if tiers.is_empty() {
        tiers.insert("syntactic");
    }

    DataQuality {
        parse_health,
        unresolved_includes,
        structural_call_edges,
        non_production_files,
        startup_functions,
        no_loop_functions,
        cost_scale: CostScaleView {
            v2_functions,
            v1_functions: functions.len() - v2_functions,
        },
        unknown_trip_functions,
        component_findings_excluded,
        resolver_tiers: tiers.into_iter().map(str::to_string).collect(),
        caps: CapsView {
            actions_detail: ACTIONS_DETAIL_CAP,
            profile_inline: PROFILE_INLINE_CAP,
        },
    }
}

// ---------------------------------------------------------------------------
// Prepared view (shared by both renderers)
// ---------------------------------------------------------------------------

struct Prepared {
    actions: Vec<Kv1Row>,
    profile: Vec<Kv3Row>,
    /// Detail anchors indexed by `doc.findings` position (the shipped
    /// `findings-detail` assignment, never a second slug implementation).
    anchors: Vec<String>,
    data_quality: DataQuality,
}

impl Prepared {
    fn new(doc: &MetricsDocument) -> Self {
        let actions = complete_actions(doc);
        let profile = complete_profile_queue(doc);
        let (_, anchors) = finding_anchor_assignments(doc);
        Self {
            actions,
            profile,
            anchors,
            data_quality: build_data_quality(doc),
        }
    }
}

fn finding_function_object(doc: &MetricsDocument, row: &Kv1Row) -> Option<FunctionObject> {
    let index = row.function_index?;
    let function = &doc.stats.functions[index];
    let scale = views::scale_of(function);
    Some(FunctionObject {
        id: function.id.clone(),
        name: function.name.clone(),
        path: function.path.clone(),
        span: function.span,
        cost: CostNumber(views::function_value(function)),
        cost_scale: scale_token(scale).to_string(),
        cost_anchor: row
            .cost_anchor
            .clone()
            .expect("views row with a function always carries an anchor"),
    })
}

// ---------------------------------------------------------------------------
// JSON model (§6)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ToolObject {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct ScanObject {
    root_path: String,
    languages: Vec<String>,
    file_count: u32,
    loc_total: Option<LocCounts>,
}

#[derive(Serialize)]
struct RankKeyObject {
    actions: [&'static str; 5],
    profile_queue: [&'static str; 7],
}

#[derive(Serialize)]
struct FunctionObject {
    id: String,
    name: String,
    path: String,
    span: crate::ir::Span,
    cost: CostNumber,
    cost_scale: String,
    cost_anchor: String,
}

#[derive(Serialize)]
struct ActionObject {
    action_id: String,
    rank: usize,
    rule_id: String,
    title: String,
    path: String,
    symbol: Option<String>,
    span: crate::ir::Span,
    severity: Severity,
    confidence: Confidence,
    resolver_tier: ResolverTier,
    macro_suspect: bool,
    error_node_overlap: bool,
    threshold: Option<Threshold>,
    path_class: String,
    function: Option<FunctionObject>,
    why: String,
    evidence: BTreeMap<String, String>,
    fix_hint: Option<&'static str>,
    detail_anchor: String,
}

#[derive(Serialize)]
struct ProfileObject {
    rank: usize,
    function_id: String,
    path: String,
    function_name: String,
    path_class: String,
    cost: CostNumber,
    cost_scale: String,
    cost_anchor: String,
    unknown_trip_loops: u32,
    known_trip_value: Option<u64>,
}

#[derive(Serialize)]
struct Envelope {
    artifact: &'static str,
    contract_version: u32,
    schema_version: String,
    tool: ToolObject,
    scan: ScanObject,
    rank_key: RankKeyObject,
    data_quality: DataQuality,
    actions: Vec<ActionObject>,
    complete_action_count: usize,
    profile_queue: Vec<ProfileObject>,
    profile_queue_cap: usize,
    complete_profile_count: usize,
}

fn build_action_object(doc: &MetricsDocument, prepared: &Prepared, row: &Kv1Row) -> ActionObject {
    let finding = &doc.findings[row.finding_index];
    ActionObject {
        action_id: finding.finding_id.clone(),
        rank: row.rank,
        rule_id: finding.rule_id.clone(),
        title: finding.title.clone(),
        path: finding.path.clone(),
        symbol: finding.symbol.clone(),
        span: finding.span,
        severity: finding.severity,
        confidence: finding.confidence,
        resolver_tier: finding.resolver_tier,
        macro_suspect: finding.macro_suspect,
        error_node_overlap: finding.error_node_overlap,
        threshold: finding.threshold.clone(),
        path_class: row.path_class.clone(),
        function: finding_function_object(doc, row),
        why: finding.message.clone(),
        evidence: finding.evidence.clone(),
        fix_hint: fix_hint(&finding.rule_id),
        detail_anchor: prepared.anchors[row.finding_index].clone(),
    }
}

fn build_profile_object(doc: &MetricsDocument, row: &Kv3Row) -> ProfileObject {
    let function = &doc.stats.functions[row.function_index];
    ProfileObject {
        rank: row.rank,
        function_id: function.id.clone(),
        path: function.path.clone(),
        function_name: function.name.clone(),
        path_class: row.path_class.clone(),
        cost: CostNumber(views::function_value(function)),
        cost_scale: scale_token(views::scale_of(function)).to_string(),
        cost_anchor: row.cost_anchor.clone(),
        unknown_trip_loops: function.unknown_trip_loops,
        known_trip_value: function.known_trip_value,
    }
}

/// Render the complete `agent-actions.json` envelope (§6) as pretty-printed
/// 2-space JSON terminated by one `\n`. Keys follow declaration order;
/// evidence keys are sorted (`BTreeMap`); no clock provenance.
#[must_use]
pub fn render_actions_json(doc: &MetricsDocument) -> String {
    let prepared = Prepared::new(doc);
    let actions: Vec<ActionObject> = prepared
        .actions
        .iter()
        .map(|row| build_action_object(doc, &prepared, row))
        .collect();
    let profile_queue: Vec<ProfileObject> = prepared
        .profile
        .iter()
        .take(PROFILE_QUEUE_CAP)
        .map(|row| build_profile_object(doc, row))
        .collect();
    let envelope = Envelope {
        artifact: "agent-actions",
        contract_version: CONTRACT_VERSION,
        schema_version: doc.schema_version.clone(),
        tool: ToolObject {
            name: doc.tool.name.clone(),
            version: doc.tool.version.clone(),
        },
        scan: ScanObject {
            root_path: doc.scan.root_path.clone(),
            languages: doc.scan.languages.clone(),
            file_count: doc.scan.file_count,
            loc_total: doc.scan.loc_total,
        },
        rank_key: RankKeyObject {
            actions: ACTION_RANK_KEY,
            profile_queue: PROFILE_RANK_KEY,
        },
        data_quality: prepared.data_quality.clone(),
        complete_action_count: actions.len(),
        actions,
        profile_queue,
        profile_queue_cap: PROFILE_QUEUE_CAP,
        complete_profile_count: prepared.profile.len(),
    };
    let mut rendered = serde_json::to_string_pretty(&envelope)
        .expect("the agent-actions envelope is serializable");
    rendered.push('\n');
    rendered
}

// ---------------------------------------------------------------------------
// Markdown renderer (§7)
// ---------------------------------------------------------------------------

fn push_blank(lines: &mut Vec<String>) {
    lines.push(String::new());
}

/// Render `agent-report.md` — the bounded fixed seven-section skeleton.
#[must_use]
pub fn render_markdown(doc: &MetricsDocument) -> String {
    let prepared = Prepared::new(doc);
    let quality = &prepared.data_quality;

    let complete_actions = prepared.actions.len();
    let profile_total = prepared.profile.len();
    let total_findings = doc.findings.len();
    let machine_rows = PROFILE_QUEUE_CAP.min(profile_total);
    let profile_rows = PROFILE_INLINE_CAP.min(profile_total);
    let tiers = quality.resolver_tiers.join(", ");
    let languages = doc.scan.languages.join(", ");
    let parse_line = quality.parse_health.map_or_else(
        || STUB_NO_PARSE.to_string(),
        |health| {
            format!(
                "parse: {} ok / {} partial / {} failed",
                health.ok, health.partial, health.failed
            )
        },
    );

    let mut lines: Vec<String> = Vec::new();
    lines.push("# Perfpal Agent Report".to_string());
    push_blank(&mut lines);
    lines.push(format!(
        "> contract {} · schema {} · {} {} · scan root: `{}`",
        CONTRACT_VERSION,
        md_text(&doc.schema_version),
        md_text(&doc.tool.name),
        md_text(&doc.tool.version),
        md_code(&doc.scan.root_path)
    ));
    lines.push(format!(
        "> {} files · {} functions · {} findings · languages: {}",
        doc.scan.file_count,
        doc.stats.functions.len(),
        total_findings,
        md_code(&languages)
    ));
    push_blank(&mut lines);

    lines.push("## How to use this report".to_string());
    push_blank(&mut lines);
    lines.push(
        "- Every row is an **optimization candidate**, not a defect verdict; confirm it with a profiler."
            .to_string(),
    );
    lines.push("- Cost is **static cost units**, a heuristic; it is never a duration.".to_string());
    lines.push(
        "- Rank key: production code first, then severity (hot > warn > info), then confidence (HIGH > MEDIUM > LOW), then containing-function static cost descending, then finding_id ascending."
            .to_string(),
    );
    lines.push(format!(
        "- Machine contract: `agent-actions.json` holds all {complete_actions} actions and the top {machine_rows} of {profile_total} profile rows."
    ));
    push_blank(&mut lines);

    lines.push("## Data quality".to_string());
    push_blank(&mut lines);
    lines.push(format!("- {parse_line}"));
    lines.push(format!(
        "- unresolved includes: {} of {} · structural call edges: {} ({} HIGH / {} MEDIUM / {} LOW)",
        quality.unresolved_includes.unresolved,
        quality.unresolved_includes.total,
        quality.structural_call_edges.total,
        quality.structural_call_edges.high,
        quality.structural_call_edges.medium,
        quality.structural_call_edges.low
    ));
    lines.push(format!(
        "- demotions: {} non-production files · {} startup functions · {} no-loop functions (counted, never dropped)",
        quality.non_production_files, quality.startup_functions, quality.no_loop_functions
    ));
    lines.push(format!(
        "- cost scale: {} v2 / {} v1 functions",
        quality.cost_scale.v2_functions, quality.cost_scale.v1_functions
    ));
    lines.push(format!("- resolver tier(s): {}", md_code(&tiers)));
    lines.push(format!(
        "- functions with unknown trip counts: {} — their cost estimate is incomplete",
        quality.unknown_trip_functions
    ));
    lines.push(format!(
        "- component-level findings excluded from actions: {} (see the human Explorer)",
        quality.component_findings_excluded
    ));
    lines.push(format!(
        "- caps: top {} actions and {} profile rows here; all actions + top {} profile rows in `agent-actions.json`",
        quality.caps.actions_detail, quality.caps.profile_inline, PROFILE_QUEUE_CAP
    ));
    push_blank(&mut lines);

    lines.push("## Start here — ranked actions".to_string());
    push_blank(&mut lines);
    lines.push(format!(
        "showing top {} of {} actions",
        ACTIONS_DETAIL_CAP.min(complete_actions),
        complete_actions
    ));
    push_blank(&mut lines);

    if prepared.actions.is_empty() {
        lines.push(STUB_NO_FINDINGS.to_string());
        push_blank(&mut lines);
    }
    for row in prepared.actions.iter().take(ACTIONS_DETAIL_CAP) {
        let finding = &doc.findings[row.finding_index];
        lines.push(format!(
            "### {}. {} · `{}:{}` · {}/{} · {}",
            row.rank,
            md_text(&finding.rule_id),
            md_code(&finding.path),
            finding.span.start_line,
            severity_token(finding.severity),
            confidence_token(finding.confidence),
            md_text(&row.path_class)
        ));
        lines.push(format!("- what: {}", md_text(&finding.title)));
        match row.function_index {
            Some(index) => {
                let function = &doc.stats.functions[index];
                lines.push(format!(
                    "- function: `{}` (`{}:{}`) · {}",
                    md_code(&function.name),
                    md_code(&function.path),
                    function.span.start_line,
                    row.cost_anchor.as_deref().unwrap_or_default()
                ));
            }
            None => lines.push(format!("- function: {STUB_NO_FUNCTION}")),
        }
        lines.push(format!("- why: {}", md_text(&finding.message)));
        if !finding.evidence.is_empty() {
            let evidence = finding
                .evidence
                .iter()
                .map(|(key, value)| format!("{}={}", md_code(key), md_code(value)))
                .collect::<Vec<_>>()
                .join(" · ");
            lines.push(format!("- evidence: `{evidence}`"));
        }
        lines.push(format!(
            "- fix: {}",
            fix_hint(&finding.rule_id).unwrap_or(STUB_NO_HINT)
        ));
        lines.push(format!(
            "- anchor: `findings-detail.html#{}` · `{}`",
            prepared.anchors[row.finding_index],
            md_code(&finding.finding_id)
        ));
        push_blank(&mut lines);
    }
    if complete_actions > ACTIONS_DETAIL_CAP {
        lines.push(format!(
            "showing the top {ACTIONS_DETAIL_CAP} of {complete_actions} actions; the complete list is `agent-actions.json`."
        ));
        push_blank(&mut lines);
    }

    lines.push("## Action index by rule".to_string());
    push_blank(&mut lines);
    let mut rules: BTreeMap<String, RuleRollup> = BTreeMap::new();
    for row in &prepared.actions {
        let finding = &doc.findings[row.finding_index];
        let entry = rules
            .entry(finding.rule_id.clone())
            .or_insert_with(|| RuleRollup {
                title: finding.title.clone(),
                count: 0,
                hot: 0,
                warn: 0,
                info: 0,
                top_rank: row.rank,
            });
        entry.count += 1;
        match finding.severity {
            Severity::Hot => entry.hot += 1,
            Severity::Warn => entry.warn += 1,
            Severity::Info => entry.info += 1,
        }
        entry.top_rank = entry.top_rank.min(row.rank);
    }
    if rules.is_empty() {
        lines.push(STUB_NO_RULES.to_string());
        push_blank(&mut lines);
    } else {
        lines.push("| rule | title | actions | hot | warn | info | top rank |".to_string());
        lines.push("|---|---|---:|---:|---:|---:|---:|".to_string());
        for (rule_id, entry) in &rules {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} |",
                md_cell(rule_id),
                md_cell(&entry.title),
                entry.count,
                entry.hot,
                entry.warn,
                entry.info,
                entry.top_rank
            ));
        }
        push_blank(&mut lines);
    }

    lines.push("## Profile queue".to_string());
    push_blank(&mut lines);
    lines.push(format!(
        "showing top {profile_rows} of {profile_total} functions"
    ));
    push_blank(&mut lines);
    if prepared.profile.is_empty() {
        lines.push(STUB_NO_FUNCTIONS.to_string());
    }
    for row in prepared.profile.iter().take(PROFILE_INLINE_CAP) {
        let suffix = if row.unknown_trip_loops > 0 {
            format!(
                " · unknown-trip ×{} — estimate incomplete",
                row.unknown_trip_loops
            )
        } else {
            String::new()
        };
        lines.push(format!(
            "{}. `{}` — `{}` — {} · {}{}",
            row.rank,
            md_code(&row.function),
            md_code(&row.path),
            row.cost_anchor,
            md_text(&row.path_class),
            suffix
        ));
    }
    if profile_total > PROFILE_INLINE_CAP {
        push_blank(&mut lines);
        lines.push(format!(
            "showing the top {PROFILE_INLINE_CAP} of {profile_total} functions; complete human list in `profile-queue.md`, machine rows in `agent-actions.json`."
        ));
    }

    push_blank(&mut lines);
    lines.push("## What this scan cannot see".to_string());
    push_blank(&mut lines);
    if quality.resolver_tiers.iter().any(|tier| tier == "semantic") {
        lines.push(format!(
            "- **Resolver tiers** (`resolver_tier: {}`): semantic resolution applies where libclang was available; unresolved sites still fall back to syntactic analysis (no macro expansion, no type inference).",
            md_code(&tiers)
        ));
    } else {
        lines.push(format!(
            "- **Syntactic tier only** (`resolver_tier: {}`): no macro expansion, no type resolution.",
            md_code(&tiers)
        ));
    }
    lines.push(
        "- **Structural call edges** are name-based/semantic and may over/under-approximate; not resolved."
            .to_string(),
    );
    lines.push(
        "- **Static cost is a heuristic**, not a duration; a high rank is a hypothesis to profile."
            .to_string(),
    );
    lines.push(
        "- **Partial or failed parses** and unresolved includes reduce coverage; the counts above say by how much."
            .to_string(),
    );
    lines.push(
        "- **Findings are candidates** for human or agent review, never defect claims.".to_string(),
    );
    push_blank(&mut lines);

    let mut rendered = lines.join("\n");
    // `lines` ends with an empty element, so `join` already yields exactly one
    // trailing newline. Guard the invariant explicitly for future editors.
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

struct RuleRollup {
    title: String,
    count: usize,
    hot: usize,
    warn: usize,
    info: usize,
    top_rank: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    use serde_json::Value;

    use crate::analyzer::{Confidence, Severity};
    use crate::ir::{
        IncludeCounts, IncludeForm, LocCounts, ParseStatus, ResolverHealth, ResolverTier, Span,
    };
    use crate::schema::{
        CallEdge, CallGraphSection, FileEntry, FindingEntry, FunctionStats, GraphsSection,
        IncludeEdgeDto, IncludeGraph, ParseHealthEntry, ScanInfo, StatsSection, Threshold,
        ThresholdValue, ToolInfo,
    };

    const FIXTURES: [&str; 4] = [
        "tests/golden/report/metrics.json",
        "tests/golden/report-mixed/metrics.json",
        "tests/golden/report-python/metrics.json",
        "tests/golden/report-rust/metrics.json",
    ];

    // ---- fixture loading -------------------------------------------------
    //
    // `MetricsDocument` intentionally serializes only (the schema stays
    // untouched, per the C2 hard rules), so the unit tests build it from a
    // `serde_json::Value` with this test-local reader. Every field the
    // renderers/views read is lifted verbatim; unused optional sections
    // (`components`, `stats.distributions`) are left absent.

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
        value
            .get(key)
            .unwrap_or_else(|| panic!("fixture missing key {key}"))
    }

    fn req_str(value: &Value, key: &str) -> String {
        field(value, key)
            .as_str()
            .unwrap_or_else(|| panic!("{key} must be a string"))
            .to_string()
    }

    fn opt_str(value: &Value, key: &str) -> Option<String> {
        value.get(key).and_then(Value::as_str).map(str::to_string)
    }

    fn req_u32(value: &Value, key: &str) -> u32 {
        u32::try_from(
            field(value, key)
                .as_u64()
                .unwrap_or_else(|| panic!("{key} must be an unsigned integer")),
        )
        .expect("fixture integer fits u32")
    }

    fn opt_u64(value: &Value, key: &str) -> Option<u64> {
        value.get(key).and_then(Value::as_u64)
    }

    fn req_bool(value: &Value, key: &str) -> bool {
        field(value, key)
            .as_bool()
            .unwrap_or_else(|| panic!("{key} must be a boolean"))
    }

    fn opt_bool(value: &Value, key: &str) -> Option<bool> {
        value.get(key).and_then(Value::as_bool)
    }

    fn span_from(value: &Value) -> Span {
        Span {
            start_line: req_u32(value, "start_line"),
            start_col: req_u32(value, "start_col"),
            end_line: req_u32(value, "end_line"),
            end_col: req_u32(value, "end_col"),
        }
    }

    fn loc_from(value: &Value) -> LocCounts {
        LocCounts {
            code: req_u32(value, "code"),
            comment: req_u32(value, "comment"),
            blank: req_u32(value, "blank"),
        }
    }

    fn threshold_from(value: Option<&Value>) -> Option<Threshold> {
        let value = value?;
        if value.is_null() {
            return None;
        }
        let raw_value = field(value, "value");
        let parsed = if let Some(text) = raw_value.as_str() {
            ThresholdValue::Text(text.to_string())
        } else {
            ThresholdValue::Number(raw_value.as_f64().expect("threshold number"))
        };
        Some(Threshold {
            name: req_str(value, "name"),
            value: parsed,
        })
    }

    fn confidence_from(token: &str) -> Confidence {
        match token {
            "HIGH" => Confidence::High,
            "MEDIUM" => Confidence::Medium,
            "LOW" => Confidence::Low,
            other => panic!("unknown confidence {other}"),
        }
    }

    fn severity_from(token: &str) -> Severity {
        match token {
            "hot" => Severity::Hot,
            "warn" => Severity::Warn,
            "info" => Severity::Info,
            other => panic!("unknown severity {other}"),
        }
    }

    fn resolver_tier_from(token: &str) -> ResolverTier {
        match token {
            "syntactic" => ResolverTier::Syntactic,
            "semantic" => ResolverTier::Semantic,
            "dynamic" => ResolverTier::Dynamic,
            other => panic!("unknown resolver tier {other}"),
        }
    }

    fn parse_status_from(token: &str) -> ParseStatus {
        match token {
            "ok" => ParseStatus::Ok,
            "partial" => ParseStatus::Partial,
            "failed" => ParseStatus::Failed,
            other => panic!("unknown parse status {other}"),
        }
    }

    fn resolver_health_from(token: &str) -> ResolverHealth {
        match token {
            "absent" => ResolverHealth::Absent,
            "ok" => ResolverHealth::Ok,
            "failed" => ResolverHealth::Failed,
            other => panic!("unknown resolver health {other}"),
        }
    }

    fn parse_health_entry_from(value: &Value) -> ParseHealthEntry {
        ParseHealthEntry {
            status: parse_status_from(&req_str(value, "status")),
            error_regions: value
                .get("error_regions")
                .and_then(Value::as_array)
                .map(|spans| spans.iter().map(span_from).collect::<Vec<Span>>()),
            error_node_count: req_u32(value, "error_node_count"),
            resolver: opt_str(value, "resolver").map(|token| resolver_health_from(&token)),
        }
    }

    fn file_from(value: &Value) -> FileEntry {
        let include_counts = field(value, "include_counts");
        FileEntry {
            path: req_str(value, "path"),
            language: req_str(value, "language"),
            loc: loc_from(field(value, "loc")),
            function_count: req_u32(value, "function_count"),
            global_var_count: req_u32(value, "global_var_count"),
            include_counts: IncludeCounts {
                quoted: req_u32(include_counts, "quoted"),
                angle: req_u32(include_counts, "angle"),
            },
            parse_health: parse_health_entry_from(field(value, "parse_health")),
            path_class: opt_str(value, "path_class"),
        }
    }

    fn function_from(value: &Value) -> FunctionStats {
        FunctionStats {
            id: req_str(value, "id"),
            name: req_str(value, "name"),
            path: req_str(value, "path"),
            span: span_from(field(value, "span")),
            param_count: req_u32(value, "param_count"),
            cyclomatic_complexity: req_u32(value, "cyclomatic_complexity"),
            cognitive_complexity: req_u32(value, "cognitive_complexity"),
            max_loop_depth: req_u32(value, "max_loop_depth"),
            loop_count: None,
            loop_body_loc: None,
            parse_health: None,
            alloc_site_count: req_u32(value, "alloc_site_count"),
            length_lines: req_u32(value, "length_lines"),
            call_fanout: opt_u64(value, "call_fanout").map(|n| n as u32),
            call_fanin: opt_u64(value, "call_fanin").map(|n| n as u32),
            call_depth: opt_u64(value, "call_depth").map(|n| n as u32),
            is_recursive: opt_bool(value, "is_recursive"),
            idiom_hash: opt_str(value, "idiom_hash"),
            error_path: opt_bool(value, "error_path"),
            alloc_family: opt_str(value, "alloc_family"),
            cfg_gated: opt_bool(value, "cfg_gated"),
            entry_kind: opt_str(value, "entry_kind"),
            entry_confidence: opt_str(value, "entry_confidence"),
            reachable_from_entry: opt_bool(value, "reachable_from_entry"),
            function_cost_v2: opt_u64(value, "function_cost_v2"),
            unknown_trip_loops: value
                .get("unknown_trip_loops")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            trip_count_source: opt_str(value, "trip_count_source"),
            known_trip_value: opt_u64(value, "known_trip_value"),
        }
    }

    fn finding_from(value: &Value) -> FindingEntry {
        let evidence = value
            .get("evidence")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(key, value)| {
                        (
                            key.clone(),
                            value
                                .as_str()
                                .expect("evidence values are strings")
                                .to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        FindingEntry {
            finding_id: req_str(value, "finding_id"),
            rule_id: req_str(value, "rule_id"),
            title: req_str(value, "title"),
            path: req_str(value, "path"),
            symbol: opt_str(value, "symbol"),
            span: span_from(field(value, "span")),
            confidence: confidence_from(&req_str(value, "confidence")),
            severity: severity_from(&req_str(value, "severity")),
            resolver_tier: resolver_tier_from(&req_str(value, "resolver_tier")),
            macro_suspect: req_bool(value, "macro_suspect"),
            error_node_overlap: req_bool(value, "error_node_overlap"),
            threshold: threshold_from(value.get("threshold")),
            message: req_str(value, "message"),
            evidence,
        }
    }

    fn include_edge_from(value: &Value) -> IncludeEdgeDto {
        IncludeEdgeDto {
            from: req_str(value, "from"),
            to: opt_str(value, "to"),
            form: match req_str(value, "form").as_str() {
                "quoted" => IncludeForm::Quoted,
                "angle" => IncludeForm::Angle,
                other => panic!("unknown include form {other}"),
            },
            resolved: req_bool(value, "resolved"),
            span: span_from(field(value, "span")),
        }
    }

    fn call_edge_from(value: &Value) -> CallEdge {
        CallEdge {
            from_function: req_str(value, "from_function"),
            to_name: req_str(value, "to_name"),
            from_path: req_str(value, "from_path"),
            to_path: opt_str(value, "to_path"),
            confidence: req_str(value, "confidence"),
        }
    }

    fn graphs_from(value: &Value) -> GraphsSection {
        let include = field(value, "include_graph");
        let edges = include
            .get("edges")
            .and_then(Value::as_array)
            .map(|values| values.iter().map(include_edge_from).collect())
            .unwrap_or_default();
        let call_graph = value.get("call_graph").and_then(|call| {
            (!call.is_null()).then(|| CallGraphSection {
                edges: call
                    .get("edges")
                    .and_then(Value::as_array)
                    .map(|values| values.iter().map(call_edge_from).collect())
                    .unwrap_or_default(),
            })
        });
        GraphsSection {
            include_graph: IncludeGraph {
                nodes: Vec::new(),
                edges,
            },
            call_graph,
        }
    }

    fn load_metrics(relative: &str) -> MetricsDocument {
        let raw = fs::read_to_string(fixture_path(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"));
        let value: Value =
            serde_json::from_str(&raw).unwrap_or_else(|error| panic!("parse {relative}: {error}"));
        let scan = field(&value, "scan");
        let tool = field(&value, "tool");
        let stats = field(&value, "stats");
        MetricsDocument {
            schema_version: req_str(&value, "schema_version"),
            tool: ToolInfo {
                name: req_str(tool, "name"),
                version: req_str(tool, "version"),
            },
            scan: ScanInfo {
                root_path: req_str(scan, "root_path"),
                started_at: opt_str(scan, "started_at").unwrap_or_default(),
                duration_ms: opt_u64(scan, "duration_ms"),
                languages: scan
                    .get("languages")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .map(|value| value.as_str().expect("language").to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
                file_count: req_u32(scan, "file_count"),
                loc_total: scan
                    .get("loc_total")
                    .filter(|value| !value.is_null())
                    .map(loc_from),
            },
            parse_health_summary: value.get("parse_health_summary").and_then(|summary| {
                (!summary.is_null()).then(|| ParseHealthSummary {
                    ok: req_u32(summary, "ok"),
                    partial: req_u32(summary, "partial"),
                    failed: req_u32(summary, "failed"),
                })
            }),
            files: value
                .get("files")
                .and_then(Value::as_array)
                .map(|values| values.iter().map(file_from).collect())
                .unwrap_or_default(),
            stats: StatsSection {
                functions: stats
                    .get("functions")
                    .and_then(Value::as_array)
                    .map(|values| values.iter().map(function_from).collect())
                    .unwrap_or_default(),
                distributions: None,
            },
            findings: value
                .get("findings")
                .and_then(Value::as_array)
                .map(|values| values.iter().map(finding_from).collect())
                .unwrap_or_default(),
            graphs: value
                .get("graphs")
                .and_then(|graphs| (!graphs.is_null()).then(|| graphs_from(graphs))),
            components: None,
        }
    }

    // ---- document builders ----------------------------------------------

    fn empty_doc() -> MetricsDocument {
        MetricsDocument {
            schema_version: "1.7".to_string(),
            tool: ToolInfo {
                name: "perfpal".to_string(),
                version: "0.2.0-beta.1".to_string(),
            },
            scan: ScanInfo {
                root_path: "<ROOT>".to_string(),
                started_at: String::new(),
                duration_ms: None,
                languages: vec!["c".to_string(), "python".to_string(), "rust".to_string()],
                file_count: 0,
                loc_total: None,
            },
            parse_health_summary: None,
            files: Vec::new(),
            stats: StatsSection {
                functions: Vec::new(),
                distributions: None,
            },
            findings: Vec::new(),
            graphs: None,
            components: None,
        }
    }

    fn synthetic_function(path: &str, name: &str, cost: u64) -> FunctionStats {
        FunctionStats {
            id: format!("{path}#{name}#1:1"),
            name: name.to_string(),
            path: path.to_string(),
            span: Span {
                start_line: 1,
                start_col: 1,
                end_line: 500,
                end_col: 1,
            },
            param_count: 0,
            cyclomatic_complexity: 2,
            cognitive_complexity: 1,
            max_loop_depth: 1,
            loop_count: None,
            loop_body_loc: None,
            parse_health: None,
            alloc_site_count: 1,
            length_lines: 500,
            call_fanout: Some(0),
            call_fanin: Some(0),
            call_depth: Some(0),
            is_recursive: Some(false),
            idiom_hash: None,
            error_path: Some(false),
            alloc_family: None,
            cfg_gated: None,
            entry_kind: None,
            entry_confidence: None,
            reachable_from_entry: None,
            function_cost_v2: Some(cost),
            unknown_trip_loops: 1,
            trip_count_source: Some("literal".to_string()),
            known_trip_value: Some(1),
        }
    }

    /// The §12 at-envelope fixture: 25 detail actions (75 total actions =
    /// 75 rollup rules) and 25 profile rows, with paths 200 B at every
    /// rendering site, function names 64 B, titles 120 B, messages 500 B,
    /// three-key evidence 300 B/action (separator overhead included), symbols
    /// 64 B, and one unknown trip per function (every profile row carries the
    /// `unknown-trip ×1 — estimate incomplete` suffix).
    ///
    /// The literal width × cap combination measures 77,713 B against the
    /// amended §12 budget of 81,920 B (80 KB): 4,207 B headroom (5.1% of the
    /// budget).
    const SYNTHETIC_PATH_LEN: usize = 200;

    fn synthetic_envelope_doc() -> MetricsDocument {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let rules: Vec<String> = crate::reporter::HEURISTIC_CATALOG_V1
            .iter()
            .map(|(id, _, _)| (*id).to_string())
            .filter(|id| !id.ends_with("-ARCH-002"))
            .collect();
        assert_eq!(
            rules.len(),
            75,
            "catalog minus the three ARCH-002 aggregates"
        );

        let pad = SYNTHETIC_PATH_LEN - 2;
        let paths: Vec<String> = (0..25).map(|index| format!("p/{index:0>pad$}")).collect();
        let names: Vec<String> = (0..25).map(|index| format!("fn{index:0>62}")).collect();

        doc.files = paths
            .iter()
            .map(|path| FileEntry {
                path: path.clone(),
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
                parse_health: ParseHealthEntry {
                    status: ParseStatus::Ok,
                    error_regions: None,
                    error_node_count: 0,
                    resolver: Some(ResolverHealth::Absent),
                },
                path_class: Some("production".to_string()),
            })
            .collect();
        doc.stats.functions = paths
            .iter()
            .zip(&names)
            .enumerate()
            .map(|(index, (path, name))| synthetic_function(path, name, 100 - index as u64))
            .collect();
        doc.findings = rules
            .iter()
            .enumerate()
            .map(|(index, rule)| {
                let path = &paths[index % paths.len()];
                let line = (index + 1) as u32;
                // Three keys whose rendered `key=value · key=value · key=value`
                // form is exactly 300 UTF-8 bytes (separator overhead included).
                let mut evidence = BTreeMap::new();
                evidence.insert("e1".to_string(), "V".repeat(95));
                evidence.insert("e2".to_string(), "V".repeat(94));
                evidence.insert("e3".to_string(), "V".repeat(94));
                FindingEntry {
                    finding_id: format!("{rule}:{path}:{line}:1"),
                    rule_id: rule.clone(),
                    title: "T".repeat(120),
                    path: path.clone(),
                    symbol: Some(format!("{index:0>64}")),
                    span: Span {
                        start_line: line,
                        start_col: 1,
                        end_line: line,
                        end_col: 2,
                    },
                    confidence: Confidence::High,
                    severity: Severity::Hot,
                    resolver_tier: ResolverTier::Syntactic,
                    macro_suspect: false,
                    error_node_overlap: false,
                    threshold: None,
                    message: "M".repeat(500),
                    evidence,
                }
            })
            .collect();
        doc.parse_health_summary = Some(ParseHealthSummary {
            ok: 25,
            partial: 0,
            failed: 0,
        });
        doc.graphs = None;
        doc
    }

    fn render(doc: &MetricsDocument) -> (String, String) {
        (render_actions_json(doc), render_markdown(doc))
    }

    fn json_value(raw: &str) -> Value {
        serde_json::from_str(raw).expect("rendered JSON parses")
    }

    // ---- tests -----------------------------------------------------------

    #[test]
    fn agent_report_rank_parity_with_complete_actions() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            let raw = render_actions_json(&doc);
            let value = json_value(&raw);
            let rendered: Vec<&str> = value["actions"]
                .as_array()
                .expect("actions array")
                .iter()
                .map(|action| action["action_id"].as_str().expect("action_id"))
                .collect();
            let expected: Vec<&str> = views::complete_actions(&doc)
                .iter()
                .map(|row| doc.findings[row.finding_index].finding_id.as_str())
                .collect();
            assert_eq!(rendered, expected, "{relative}: action order");
        }
    }

    #[test]
    fn agent_report_profile_parity_with_complete_profile_queue() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            let raw = render_actions_json(&doc);
            let value = json_value(&raw);
            let rendered: Vec<&str> = value["profile_queue"]
                .as_array()
                .expect("profile_queue array")
                .iter()
                .map(|row| row["function_id"].as_str().expect("function_id"))
                .collect();
            let expected: Vec<&str> = views::complete_profile_queue(&doc)
                .iter()
                .take(PROFILE_QUEUE_CAP)
                .map(|row| doc.stats.functions[row.function_index].id.as_str())
                .collect();
            assert_eq!(rendered, expected, "{relative}: profile order");
        }
    }

    #[test]
    fn agent_report_completeness_and_contiguous_ranks() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            let raw = render_actions_json(&doc);
            let value = json_value(&raw);
            let actions = value["actions"].as_array().expect("actions");
            assert_eq!(
                actions.len() as u64,
                value["complete_action_count"].as_u64().expect("count"),
                "{relative}: complete_action_count"
            );
            for (index, action) in actions.iter().enumerate() {
                assert_eq!(
                    action["rank"].as_u64().expect("action rank") as usize,
                    index + 1,
                    "{relative}: action ranks are contiguous"
                );
            }
            let profile = value["profile_queue"].as_array().expect("profile");
            let complete = value["complete_profile_count"].as_u64().expect("complete") as usize;
            assert_eq!(profile.len(), PROFILE_QUEUE_CAP.min(complete));
            for (index, row) in profile.iter().enumerate() {
                assert_eq!(
                    row["rank"].as_u64().expect("profile rank") as usize,
                    index + 1,
                    "{relative}: profile ranks are contiguous"
                );
            }
        }
    }

    #[test]
    fn agent_report_hint_table_equals_catalog() {
        let hints: BTreeSet<&str> = AGENT_FIX_HINTS.iter().map(|(id, _)| *id).collect();
        assert_eq!(AGENT_FIX_HINTS.len(), hints.len(), "no duplicate hint ids");
        let catalog: BTreeSet<&str> = crate::reporter::HEURISTIC_CATALOG_V1
            .iter()
            .map(|(id, _, _)| *id)
            .collect();
        assert_eq!(hints, catalog, "hint table must cover the catalog exactly");
        assert_eq!(catalog.len(), 78);
        assert_eq!(fix_hint("PERF-RS-STR-001"), None);
        assert_eq!(fix_hint("PERF-NOT-A-RULE"), None);
    }

    #[test]
    fn agent_report_hint_hygiene() {
        let claims = [
            "ms",
            "µs",
            "seconds",
            "speedup",
            "latency",
            "throughput",
            "runtime",
        ];
        let certainty = ["will", "guarantees", "fixes", "must"];
        for (rule_id, hint) in AGENT_FIX_HINTS {
            assert!(
                hint.split_whitespace().count() <= 20,
                "{rule_id}: hint exceeds 20 words"
            );
            for claim in claims {
                assert!(
                    !contains_word(hint, claim),
                    "{rule_id}: forbidden time claim {claim:?} in hint"
                );
            }
            for word in certainty {
                assert!(
                    !contains_word(hint, word),
                    "{rule_id}: forbidden certainty word {word:?} in hint"
                );
            }
        }
    }

    fn contains_word(text: &str, word: &str) -> bool {
        let lower = text.to_lowercase();
        let mut start = 0;
        while let Some(offset) = lower[start..].find(word) {
            let index = start + offset;
            let before = lower[..index].chars().next_back();
            let after = lower[index + word.len()..].chars().next();
            let boundary = |ch: Option<char>| ch.is_none_or(|c| !c.is_alphanumeric() && c != '_');
            if boundary(before) && boundary(after) {
                return true;
            }
            start = index + word.len();
        }
        false
    }

    /// Assert every occurrence of `needle` on every MD line sits inside an
    /// inline code span (an odd number of backticks before it).
    fn expect_html_only_inside_code_spans(markdown: &str, needle: &str) {
        for line in markdown.lines().filter(|line| line.contains(needle)) {
            for (index, _) in line.match_indices(needle) {
                assert_eq!(
                    line[..index].matches('`').count() % 2,
                    1,
                    "{needle} must stay inside a code span: {line:?}"
                );
            }
        }
    }

    /// The fixed skeleton: exactly one H1 and the six `## ` sections in order.
    fn assert_seven_headings(markdown: &str) {
        let h1: Vec<&str> = markdown
            .lines()
            .filter(|line| line.starts_with("# "))
            .collect();
        assert_eq!(h1, vec!["# Perfpal Agent Report"], "exactly one H1");

        let sections: Vec<&str> = markdown
            .lines()
            .filter(|line| line.starts_with("## "))
            .collect();
        assert_eq!(
            sections,
            vec![
                "## How to use this report",
                "## Data quality",
                "## Start here — ranked actions",
                "## Action index by rule",
                "## Profile queue",
                "## What this scan cannot see",
            ],
            "fixed six `## ` sections in order"
        );
    }

    /// A numbered profile row (`1. `name` — …`): scanner data, exempt from the
    /// C-authored time-claim scan (spec §9.7).
    fn is_numbered_profile_row(line: &str) -> bool {
        line.chars().next().is_some_and(|c| c.is_ascii_digit()) && line.contains(". `")
    }

    /// Count table-column separators, ignoring pipes escaped by an odd number
    /// of preceding backslashes.
    fn structural_pipes(line: &str) -> usize {
        let mut pipes = 0;
        let mut backslashes = 0;
        for ch in line.chars() {
            if ch == '\\' {
                backslashes += 1;
            } else {
                if ch == '|' && backslashes % 2 == 0 {
                    pipes += 1;
                }
                backslashes = 0;
            }
        }
        pipes
    }

    #[test]
    fn agent_report_anchor_parity() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            let raw = render_actions_json(&doc);
            let value = json_value(&raw);
            let view_rows = views::complete_actions(&doc);
            for (row, action) in view_rows.iter().zip(value["actions"].as_array().unwrap()) {
                match row.function_index {
                    Some(function_index) => {
                        let function = &doc.stats.functions[function_index];
                        let function_json = &action["function"];
                        assert_eq!(
                            function_json["cost_anchor"].as_str(),
                            row.cost_anchor.as_deref(),
                            "{relative}: action anchor for {}",
                            function.id
                        );
                        assert_eq!(
                            function_json["cost_scale"].as_str(),
                            Some(scale_token(views::scale_of(function))),
                        );
                    }
                    None => assert!(action["function"].is_null()),
                }
            }
            let profile_rows = views::complete_profile_queue(&doc);
            for (row, profile) in profile_rows
                .iter()
                .take(PROFILE_QUEUE_CAP)
                .zip(value["profile_queue"].as_array().unwrap())
            {
                assert_eq!(
                    profile["cost_anchor"].as_str(),
                    Some(row.cost_anchor.as_str()),
                    "{relative}: profile anchor"
                );
            }
        }
    }

    #[test]
    fn agent_report_detail_anchor_matches_shipped_assignments() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            let value = json_value(&render_actions_json(&doc));
            let (_, anchors) = crate::reporter::finding_anchor_assignments(&doc);
            for row in views::complete_actions(&doc) {
                let finding = &doc.findings[row.finding_index];
                let action = value["actions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|action| {
                        action["action_id"].as_str() == Some(finding.finding_id.as_str())
                    })
                    .expect("action present");
                assert_eq!(
                    action["detail_anchor"].as_str(),
                    Some(anchors[row.finding_index].as_str()),
                    "{relative}: detail anchor for {}",
                    finding.finding_id
                );
            }
        }
    }

    #[test]
    fn agent_report_integral_cost_serialization() {
        let doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let raw = render_actions_json(&doc);
        assert!(raw.contains("\"cost\": 15,"), "integral cost prints as 15");
        assert!(!raw.contains("\"cost\": 15.0"), "never 15.0");
        let value = json_value(&raw);
        for action in value["actions"].as_array().unwrap() {
            if let Some(function) = action["function"].as_object() {
                let cost = function["cost"].as_f64().expect("numeric cost");
                assert_eq!(cost.fract(), 0.0);
            }
        }
        for row in value["profile_queue"].as_array().unwrap() {
            assert!(row["cost"].is_number());
        }
    }

    #[test]
    fn agent_report_omits_clock_provenance() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            let (json, markdown) = render(&doc);
            for artifact in [&json, &markdown] {
                assert!(!artifact.contains("started_at"), "{relative}: started_at");
                assert!(!artifact.contains("duration_ms"), "{relative}: duration_ms");
            }
        }
    }

    #[test]
    fn agent_report_double_render_is_byte_identical() {
        for relative in FIXTURES {
            let doc = load_metrics(relative);
            assert_eq!(render_actions_json(&doc), render_actions_json(&doc));
            assert_eq!(render_markdown(&doc), render_markdown(&doc));
        }
    }

    #[test]
    fn agent_report_empty_scan_uses_named_stubs() {
        let doc = empty_doc();
        let markdown = render_markdown(&doc);
        assert_seven_headings(&markdown);
        assert!(markdown.contains(STUB_NO_FINDINGS));
        assert!(markdown.contains(STUB_NO_FUNCTIONS));
        assert!(markdown.contains(STUB_NO_RULES));
        assert!(markdown.contains(STUB_NO_PARSE));
        assert!(
            markdown.contains(
                "no functions recorded — nothing to queue.\n\n## What this scan cannot see"
            ),
            "exactly one blank line before the final section"
        );
        assert!(
            !markdown.contains("nothing to queue.\n\n\n## What"),
            "no duplicate blank line after the empty profile stub"
        );
        for heading in [
            "# Perfpal Agent Report",
            "## How to use this report",
            "## Data quality",
            "## Start here — ranked actions",
            "## Action index by rule",
            "## Profile queue",
            "## What this scan cannot see",
        ] {
            assert!(markdown.contains(heading), "missing heading {heading}");
        }
        let value = json_value(&render_actions_json(&doc));
        assert!(value["data_quality"]["parse_health"].is_null());
        assert_eq!(value["actions"].as_array().unwrap().len(), 0);
        assert_eq!(value["profile_queue"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn agent_report_text_safety() {
        let mut doc = empty_doc();
        let hostile = "src/a\nb`c<script>";
        let hostile_rule = "PERF-<b>|x";
        doc.scan.file_count = 1;
        doc.files.push(FileEntry {
            path: hostile.to_string(),
            language: "c".to_string(),
            loc: LocCounts {
                code: 1,
                comment: 0,
                blank: 0,
            },
            function_count: 1,
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
            path_class: Some("prod|uction".to_string()),
        });
        doc.stats.functions.push(FunctionStats {
            id: format!("{hostile}#hostile#1:1"),
            name: "hostile".to_string(),
            path: hostile.to_string(),
            span: Span {
                start_line: 1,
                start_col: 1,
                end_line: 10,
                end_col: 1,
            },
            param_count: 0,
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            max_loop_depth: 1,
            loop_count: None,
            loop_body_loc: None,
            parse_health: None,
            alloc_site_count: 0,
            length_lines: 10,
            call_fanout: Some(0),
            call_fanin: Some(0),
            call_depth: Some(0),
            is_recursive: Some(false),
            idiom_hash: None,
            error_path: Some(false),
            alloc_family: None,
            cfg_gated: None,
            entry_kind: None,
            entry_confidence: None,
            reachable_from_entry: None,
            function_cost_v2: Some(5),
            unknown_trip_loops: 0,
            trip_count_source: None,
            known_trip_value: None,
        });
        doc.findings.push(FindingEntry {
            finding_id: format!("{hostile_rule}:{hostile}:3:1"),
            rule_id: hostile_rule.to_string(),
            title: "a\\|b <script>".to_string(),
            path: hostile.to_string(),
            symbol: None,
            span: Span {
                start_line: 3,
                start_col: 1,
                end_line: 3,
                end_col: 2,
            },
            confidence: Confidence::High,
            severity: Severity::Hot,
            resolver_tier: ResolverTier::Syntactic,
            macro_suspect: false,
            error_node_overlap: false,
            threshold: None,
            message: "why <script>".to_string(),
            evidence: BTreeMap::new(),
        });

        let (json, markdown) = render(&doc);

        // Fixed heading set: the H1 plus six `## ` sections (seven headings
        // total). The injected newline is escaped, so it cannot mint a heading.
        assert_eq!(
            markdown
                .lines()
                .filter(|line| line.starts_with("# ") || line.starts_with("## "))
                .count(),
            7,
            "fixed heading set is unchanged"
        );
        assert_eq!(
            markdown
                .lines()
                .filter(|line| line.starts_with("## "))
                .count(),
            6,
            "six fixed `## ` sections"
        );
        assert_eq!(
            markdown
                .lines()
                .filter(|line| line.starts_with("### "))
                .count(),
            1,
            "only the one action heading"
        );
        assert!(
            !markdown.lines().any(|line| line.starts_with("## <script>")),
            "no forged heading"
        );

        // Backtick: every scanner-controlled occurrence becomes U+02CB, so no
        // code span can be closed by hostile data. The hostile path renders at
        // four sites (action heading, function line, anchor, profile row).
        assert!(
            markdown.contains("\\nb\u{02cb}c"),
            "hostile newline and backtick are escaped"
        );
        assert!(
            !markdown.contains("\\nb`c"),
            "no raw backtick from scanner data"
        );
        assert!(
            markdown.matches('\u{02cb}').count() >= 4,
            "the backtick is substituted at every rendering site"
        );

        // H3 hardening: hostile `<script>` never appears raw anywhere in the
        // Markdown — code-span values are neutralized too (4 path sites plus
        // the prose title/message).
        assert!(
            !markdown.contains("<script>"),
            "no raw HTML token anywhere in agent-report.md"
        );
        assert!(
            !markdown.contains("<script"),
            "no raw script token anywhere in agent-report.md"
        );
        assert!(
            markdown.matches("&lt;script").count() >= 4,
            "every hostile script occurrence is neutralized"
        );

        // Prose (`title`, `why`) is HTML-neutralised.
        for prefix in ["- what:", "- why:"] {
            let line = markdown
                .lines()
                .find(|line| line.starts_with(prefix))
                .unwrap_or_else(|| panic!("missing {prefix} line"));
            assert!(
                line.contains("&lt;script&gt;"),
                "prose HTML is neutralized on {prefix}"
            );
            assert!(
                !line.contains("<script>"),
                "no raw HTML in prose on {prefix}"
            );
        }

        // The hostile path stays one physical line at each rendering site.
        let path_lines = markdown
            .lines()
            .filter(|line| line.contains("src/a\\nb"))
            .count();
        assert_eq!(path_lines, 4, "each rendering site stays one line");

        // The rollup is the only table: the hostile rule cell is escaped, so
        // the row still has exactly eight columns (nine structural pipes).
        let rollup = markdown
            .lines()
            .find(|line| line.starts_with("| ") && line.contains("PERF-&lt;b&gt;"))
            .expect("rollup row for the hostile rule");
        assert!(
            rollup.contains("\\|"),
            "the escaped pipe is present in the rule cell: {rollup:?}"
        );
        let header = markdown
            .lines()
            .find(|line| line.starts_with("| rule |"))
            .expect("rollup header");
        assert_eq!(
            structural_pipes(rollup),
            structural_pipes(header),
            "the hostile rule must not add a structural column: {rollup:?}"
        );
        assert_eq!(
            structural_pipes(rollup),
            8,
            "seven columns / eight structural pipes (rule, title, actions, hot, warn, info, top rank): {rollup:?}"
        );
        assert!(!rollup.contains("<b>"), "no raw <b> in the table cell");
        assert!(
            !rollup.contains("<script>"),
            "no raw <script> in the table cell"
        );
        expect_html_only_inside_code_spans(&markdown, "<b>");
        assert!(
            markdown.lines().any(|line| line.contains("<b>")),
            "the raw rule id remains verbatim inside the action-id code span"
        );

        // JSON keeps the raw scanner bytes: no Markdown substitution leaks in.
        let value = json_value(&json);
        assert_eq!(value["actions"][0]["path"].as_str(), Some(hostile));
        assert_eq!(value["actions"][0]["rule_id"].as_str(), Some(hostile_rule));
        assert_eq!(
            value["actions"][0]["path_class"].as_str(),
            Some("prod|uction")
        );
        assert_eq!(
            value["actions"][0]["title"].as_str(),
            Some("a\\|b <script>")
        );
        assert!(!json.contains('\u{02cb}'), "JSON has no ˋ substitution");
        assert!(!json.contains("&lt;"), "JSON has no HTML escaping");
    }

    #[test]
    fn agent_report_profile_row_schema() {
        let doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let value = json_value(&render_actions_json(&doc));
        let rows = value["profile_queue"].as_array().expect("profile rows");
        assert!(!rows.is_empty());
        for row in rows {
            let object = row.as_object().expect("row object");
            let keys: BTreeSet<&str> = object.keys().map(String::as_str).collect();
            let expected: BTreeSet<&str> = [
                "rank",
                "function_id",
                "path",
                "function_name",
                "path_class",
                "cost",
                "cost_scale",
                "cost_anchor",
                "unknown_trip_loops",
                "known_trip_value",
            ]
            .into_iter()
            .collect();
            assert_eq!(keys, expected);
            assert!(row["rank"].as_u64().is_some());
            assert!(row["function_id"].is_string());
            assert!(row["path"].is_string());
            assert!(
                row["function_name"].is_string(),
                "function_name is a string"
            );
            assert!(row["path_class"].is_string());
            assert!(row["cost"].is_number());
            assert!(row["cost_scale"].is_string());
            assert!(row["cost_anchor"].is_string());
            assert!(row["unknown_trip_loops"].as_u64().is_some());
            assert!(
                row["known_trip_value"].is_null() || row["known_trip_value"].as_u64().is_some()
            );
        }
    }

    #[test]
    fn agent_report_unknown_rule_and_missing_function_stubs() {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let target_id = {
            let finding = &mut doc.findings[0];
            finding.rule_id = "PERF-NOT-A-RULE-999".to_string();
            finding.symbol = None;
            finding.path = "src/missing.c".to_string();
            finding.span.start_line = 9_999;
            finding.span.end_line = 9_999;
            finding.finding_id.clone()
        };

        let (json, markdown) = render(&doc);
        let value = json_value(&json);
        let action = value["actions"]
            .as_array()
            .expect("actions")
            .iter()
            .find(|action| action["action_id"].as_str() == Some(target_id.as_str()))
            .expect("mutated action present");
        let object = action.as_object().expect("action object");
        assert!(object.contains_key("fix_hint"), "fix_hint key present");
        assert!(action["fix_hint"].is_null());
        assert!(object.contains_key("function"), "function key present");
        assert!(action["function"].is_null());
        assert!(object.contains_key("symbol"), "symbol key present");
        assert!(action["symbol"].is_null());
        assert_eq!(action["path_class"].as_str(), Some("unknown"));
        assert!(markdown.contains(STUB_NO_HINT));
        assert!(markdown.contains(STUB_NO_FUNCTION));
    }

    #[test]
    fn agent_report_v1_cost_scale() {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let index = 0;
        doc.stats.functions[index].function_cost_v2 = None;
        let function = doc.stats.functions[index].clone();
        let expected_cost = views::function_value(&doc.stats.functions[index]);
        assert_eq!(expected_cost.fract(), 0.0, "v1 chart cost is integral here");

        let (json, markdown) = render(&doc);
        let value = json_value(&json);
        let action = value["actions"]
            .as_array()
            .expect("actions")
            .iter()
            .find(|action| action["function"]["id"].as_str() == Some(function.id.as_str()))
            .expect("action for the v1 function");
        let profile = value["profile_queue"]
            .as_array()
            .expect("profile")
            .iter()
            .find(|row| row["function_id"].as_str() == Some(function.id.as_str()))
            .expect("profile row for the v1 function");
        let shipped = views::complete_profile_queue(&doc)
            .into_iter()
            .find(|row| row.function_index == index)
            .expect("shipped v1 row");

        assert_eq!(
            action["function"]["cost"].as_f64(),
            Some(expected_cost),
            "action cost is the v1 chart-side value"
        );
        assert_eq!(
            profile["cost"].as_f64(),
            Some(expected_cost),
            "profile cost is the v1 chart-side value"
        );
        assert_eq!(action["function"]["cost_scale"].as_str(), Some("v1"));
        assert_eq!(profile["cost_scale"].as_str(), Some("v1"));
        assert_eq!(
            action["function"]["cost_anchor"].as_str(),
            Some(shipped.cost_anchor.as_str()),
            "action anchor byte-equals the shipped Kv3 anchor"
        );
        assert_eq!(
            profile["cost_anchor"].as_str(),
            Some(shipped.cost_anchor.as_str())
        );
        assert!(shipped.cost_anchor.contains("static cost units (v1)"));
        assert!(markdown.contains("static cost units (v1)"));
        assert_eq!(
            value["data_quality"]["cost_scale"]["v1_functions"].as_u64(),
            Some(1)
        );
        assert!(
            profile["cost"].is_u64(),
            "integral v1 cost serializes as an unsigned JSON integer (no .0)"
        );
    }

    #[test]
    fn agent_report_optional_fields_and_threshold_shapes() {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        doc.findings[0].threshold = Some(Threshold {
            name: "min_nesting_depth".to_string(),
            value: ThresholdValue::Number(3.0),
        });
        doc.findings[1].threshold = Some(Threshold {
            name: "min_complexity".to_string(),
            value: ThresholdValue::Text("x".to_string()),
        });
        doc.findings[2].threshold = None;
        doc.findings[0].symbol = None;
        doc.stats.functions[0].known_trip_value = Some(7);
        let ids: Vec<String> = doc.findings[..3]
            .iter()
            .map(|finding| finding.finding_id.clone())
            .collect();

        let value = json_value(&render_actions_json(&doc));
        let actions = value["actions"].as_array().expect("actions");
        let action = |id: &str| {
            actions
                .iter()
                .find(|action| action["action_id"].as_str() == Some(id))
                .unwrap_or_else(|| panic!("missing action {id}"))
        };

        let numeric = action(&ids[0]);
        assert!(numeric["threshold"]["value"].is_number());
        assert_eq!(numeric["threshold"]["value"].as_f64(), Some(3.0));
        assert!(numeric.as_object().unwrap().contains_key("symbol"));
        assert!(numeric["symbol"].is_null());

        let textual = action(&ids[1]);
        assert!(textual["threshold"]["value"].is_string());
        assert_eq!(textual["threshold"]["value"].as_str(), Some("x"));

        assert!(action(&ids[2])["threshold"].is_null());

        let function_id = doc.stats.functions[0].id.clone();
        let profile = value["profile_queue"]
            .as_array()
            .expect("profile")
            .iter()
            .find(|row| row["function_id"].as_str() == Some(function_id.as_str()))
            .expect("profile row");
        assert_eq!(profile["known_trip_value"].as_u64(), Some(7));

        let empty = json_value(&render_actions_json(&empty_doc()));
        assert!(
            empty["scan"]
                .as_object()
                .expect("scan object")
                .contains_key("loc_total"),
            "loc_total key present"
        );
        assert!(empty["scan"]["loc_total"].is_null());
        assert!(empty["profile_queue"].as_array().unwrap().is_empty());
        assert_eq!(empty["complete_profile_count"].as_u64(), Some(0));
        assert_eq!(
            empty["data_quality"]["structural_call_edges"],
            serde_json::json!({"total": 0, "high": 0, "medium": 0, "low": 0})
        );
        assert_eq!(
            empty["data_quality"]["cost_scale"],
            serde_json::json!({"v2_functions": 0, "v1_functions": 0})
        );
        assert_eq!(
            empty["data_quality"]["resolver_tiers"],
            serde_json::json!(["syntactic"])
        );
        assert!(
            empty["data_quality"]
                .as_object()
                .expect("data_quality object")
                .contains_key("parse_health"),
            "parse_health key present"
        );
        assert!(empty["data_quality"]["parse_health"].is_null());
        assert_eq!(
            empty["data_quality"]["unresolved_includes"],
            serde_json::json!({"unresolved": 0, "total": 0})
        );
    }

    #[test]
    fn agent_report_capped_tails() {
        let mut doc = synthetic_envelope_doc();
        doc.stats
            .functions
            .push(synthetic_function("p/extra", "extra_fn", 1));

        let markdown = render_markdown(&doc);
        assert_seven_headings(&markdown);
        assert!(
            markdown.contains(
                "showing the top 25 of 75 actions; the complete list is `agent-actions.json`."
            ),
            "action tail"
        );
        assert!(
            markdown.contains(
                "showing the top 25 of 26 functions; complete human list in `profile-queue.md`, machine rows in `agent-actions.json`."
            ),
            "profile tail"
        );
    }

    #[test]
    fn agent_report_semantic_tier_caveat() {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let mut finding = doc.findings[0].clone();
        finding.resolver_tier = ResolverTier::Semantic;
        doc.findings = vec![finding];

        let (json, markdown) = render(&doc);
        assert_seven_headings(&markdown);
        let value = json_value(&json);
        assert_eq!(
            value["data_quality"]["resolver_tiers"],
            serde_json::json!(["semantic"])
        );
        assert!(
            markdown.contains(
                "- **Resolver tiers** (`resolver_tier: semantic`): semantic resolution applies where libclang was available; unresolved sites still fall back to syntactic analysis (no macro expansion, no type inference)."
            ),
            "semantic tier caveat"
        );
        assert!(!markdown.contains("Syntactic tier only"));

        let baseline = render_markdown(&load_metrics("tests/golden/report-mixed/metrics.json"));
        assert!(
            baseline.contains(
                "- **Syntactic tier only** (`resolver_tier: syntactic`): no macro expansion, no type resolution."
            ),
            "syntactic-only scans keep the byte-identical bullet"
        );
    }

    #[test]
    fn agent_report_data_quality_counts() {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        assert_eq!(
            doc.findings
                .iter()
                .filter(|finding| finding.rule_id.ends_with("-ARCH-002"))
                .count(),
            0,
            "fixture has no pre-existing component aggregate"
        );
        doc.files[0].path_class = Some("test".to_string());
        doc.stats.functions[0].entry_kind = Some("main".to_string());

        let mut component = doc.findings[0].clone();
        component.finding_id = "PERF-C-ARCH-002:src/main.c:99:1".to_string();
        component.rule_id = "PERF-C-ARCH-002".to_string();
        component.path = "src/main.c".to_string();
        component.span = Span::point(99, 1);
        doc.findings.push(component);

        doc.graphs = Some(GraphsSection {
            include_graph: IncludeGraph {
                nodes: Vec::new(),
                edges: vec![
                    IncludeEdgeDto {
                        from: "src/main.c".to_string(),
                        to: Some("src/util.h".to_string()),
                        form: IncludeForm::Quoted,
                        resolved: true,
                        span: Span::point(1, 1),
                    },
                    IncludeEdgeDto {
                        from: "src/lib.rs".to_string(),
                        to: None,
                        form: IncludeForm::Quoted,
                        resolved: false,
                        span: Span::point(2, 1),
                    },
                ],
            },
            call_graph: Some(CallGraphSection {
                edges: vec![
                    CallEdge {
                        from_function: "a".to_string(),
                        to_name: "b".to_string(),
                        from_path: "src/a.c".to_string(),
                        to_path: None,
                        confidence: "HIGH".to_string(),
                    },
                    CallEdge {
                        from_function: "b".to_string(),
                        to_name: "c".to_string(),
                        from_path: "src/b.c".to_string(),
                        to_path: None,
                        confidence: "MEDIUM".to_string(),
                    },
                    CallEdge {
                        from_function: "c".to_string(),
                        to_name: "d".to_string(),
                        from_path: "src/c.c".to_string(),
                        to_path: None,
                        confidence: "LOW".to_string(),
                    },
                ],
            }),
        });
        doc.parse_health_summary = Some(ParseHealthSummary {
            ok: 1,
            partial: 2,
            failed: 3,
        });

        let value = json_value(&render_actions_json(&doc));
        let quality = &value["data_quality"];
        assert_eq!(quality["parse_health"]["ok"].as_u64(), Some(1));
        assert_eq!(quality["parse_health"]["partial"].as_u64(), Some(2));
        assert_eq!(quality["parse_health"]["failed"].as_u64(), Some(3));
        assert_eq!(quality["non_production_files"].as_u64(), Some(1));
        assert_eq!(quality["startup_functions"].as_u64(), Some(1));
        assert_eq!(quality["component_findings_excluded"].as_u64(), Some(1));
        assert_eq!(
            quality["unresolved_includes"],
            serde_json::json!({"unresolved": 1, "total": 2})
        );
        assert_eq!(
            quality["structural_call_edges"],
            serde_json::json!({"total": 3, "high": 1, "medium": 1, "low": 1})
        );
        let actions = value["actions"].as_array().expect("actions");
        assert_eq!(
            actions.len() + quality["component_findings_excluded"].as_u64().unwrap() as usize,
            doc.findings.len(),
            "component aggregates are excluded from actions, never dropped"
        );

        let markdown = render_markdown(&doc);
        assert!(markdown.contains("- parse: 1 ok / 2 partial / 3 failed"));
        assert!(markdown.contains(
            "- unresolved includes: 1 of 2 · structural call edges: 3 (1 HIGH / 1 MEDIUM / 1 LOW)"
        ));
        assert!(
            markdown.contains("- demotions: 1 non-production files · 1 startup functions · "),
            "demotion counts"
        );
        assert!(markdown.contains(
            "- component-level findings excluded from actions: 1 (see the human Explorer)"
        ));
    }

    #[test]
    fn agent_report_profile_queue_cap() {
        let mut doc = empty_doc();
        doc.scan.file_count = 101;
        doc.stats.functions = (0..101)
            .map(|index| {
                synthetic_function(
                    &format!("p/{index:03}"),
                    &format!("fn{index:03}"),
                    101 - index,
                )
            })
            .collect();

        let value = json_value(&render_actions_json(&doc));
        let profile = value["profile_queue"].as_array().expect("profile queue");
        assert_eq!(profile.len(), 100, "JSON profile queue is capped at 100");
        assert_eq!(value["complete_profile_count"].as_u64(), Some(101));
        for (index, row) in profile.iter().enumerate() {
            assert_eq!(row["rank"].as_u64(), Some(index as u64 + 1));
        }

        let markdown = render_markdown(&doc);
        assert!(markdown.contains("showing top 25 of 101 functions"));
        assert!(markdown.contains(
            "showing the top 25 of 101 functions; complete human list in `profile-queue.md`, machine rows in `agent-actions.json`."
        ));
    }

    #[test]
    fn agent_report_all_component_scan() {
        let mut doc = load_metrics("tests/golden/report-mixed/metrics.json");
        for finding in &mut doc.findings {
            finding.rule_id = "PERF-C-ARCH-002".to_string();
        }
        let total = doc.findings.len();

        let (json, markdown) = render(&doc);
        assert_seven_headings(&markdown);
        let value = json_value(&json);
        assert!(
            value["actions"].as_array().expect("actions").is_empty(),
            "component aggregates never enter the action list"
        );
        assert_eq!(value["complete_action_count"].as_u64(), Some(0));
        assert_eq!(
            value["data_quality"]["component_findings_excluded"].as_u64(),
            Some(total as u64)
        );
        assert!(markdown.contains(STUB_NO_RULES));
        assert!(markdown.contains(STUB_NO_FINDINGS));
        assert!(markdown.contains(&format!(
            "- component-level findings excluded from actions: {total} (see the human Explorer)"
        )));
    }

    #[test]
    fn agent_report_size_budget_golden_fixtures() {
        // Exact pins: intentional renderer changes must update this table and
        // spec §12 (the pinned sizes are part of the measured contract).
        let pinned = [
            ("tests/golden/report/metrics.json", 13_490usize),
            ("tests/golden/report-mixed/metrics.json", 8_900),
            ("tests/golden/report-python/metrics.json", 26_932),
            ("tests/golden/report-rust/metrics.json", 25_891),
        ];
        for (relative, expected) in pinned {
            let doc = load_metrics(relative);
            let markdown = render_markdown(&doc);
            assert_eq!(
                markdown.len(),
                expected,
                "intentional renderer changes must update this table and spec §12: {relative}"
            );
            assert!(
                markdown.len() <= MD_SIZE_BUDGET,
                "{relative}: observed {} B exceeds budget {MD_SIZE_BUDGET} B",
                markdown.len()
            );
        }

        let synthetic = synthetic_envelope_doc();
        let markdown = render_markdown(&synthetic);
        // No truncation: the widest path and function name survive in full.
        let long_path = synthetic.files[0].path.clone();
        let long_name = synthetic.stats.functions[0].name.clone();
        assert_eq!(long_path.len(), 200, "path width");
        assert_eq!(long_name.len(), 64, "function-name width");
        assert!(
            markdown.contains(long_path.as_str()),
            "the full 200 B path is never truncated"
        );
        assert!(
            markdown.contains(long_name.as_str()),
            "the full 64 B function name is never truncated"
        );
        // The worst-case evidence uses three keys and exactly 300 B of
        // rendered `key=value · key=value · key=value` (separator included).
        let evidence_row = &synthetic.findings[0].evidence;
        assert_eq!(evidence_row.len(), 3, "three evidence keys");
        let evidence_bytes = evidence_row
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" · ");
        assert_eq!(
            evidence_bytes.len(),
            300,
            "three-key evidence totals exactly 300 B"
        );
        assert!(
            markdown
                .matches(" · unknown-trip ×1 — estimate incomplete")
                .count()
                == 25,
            "all 25 profile rows carry the unknown-trip suffix"
        );
        assert_eq!(markdown.matches("\n### ").count(), 25, "25 detail actions");
        assert_eq!(
            markdown
                .lines()
                .filter(|line| line.starts_with("| PERF-"))
                .count(),
            75,
            "75 rollup rules"
        );
        assert_eq!(
            markdown
                .lines()
                .filter(|line| {
                    let mut chars = line.chars();
                    chars.next().is_some_and(|c| c.is_ascii_digit()) && line.contains(". `")
                })
                .count(),
            25,
            "25 profile rows"
        );
        assert_eq!(
            markdown.len(),
            77_763,
            "intentional renderer changes must update this table and spec §12: synthetic"
        );
        assert!(
            markdown.len() <= MD_SIZE_BUDGET,
            "synthetic: observed {} B exceeds budget {MD_SIZE_BUDGET} B",
            markdown.len()
        );
    }

    #[test]
    fn agent_report_forbidden_time_claims() {
        let mut runtime_doc = empty_doc();
        runtime_doc
            .stats
            .functions
            .push(synthetic_function("runtime/loop.c", "runtime", 5));
        runtime_doc.findings.push(FindingEntry {
            finding_id: "PERF-C-LOOP-004:runtime/loop.c:3:1".to_string(),
            rule_id: "PERF-C-LOOP-004".to_string(),
            title: "Runtime loop".to_string(),
            path: "runtime/loop.c".to_string(),
            symbol: Some("runtime".to_string()),
            span: Span::point(3, 1),
            confidence: Confidence::High,
            severity: Severity::Hot,
            resolver_tier: ResolverTier::Syntactic,
            macro_suspect: false,
            error_node_overlap: false,
            threshold: None,
            message: "loop in runtime".to_string(),
            evidence: BTreeMap::new(),
        });
        let runtime_markdown = render_markdown(&runtime_doc);
        assert!(
            runtime_markdown.contains("`runtime` — `runtime/loop.c`"),
            "the profile row renders the exempt scanner-data name"
        );
        assert!(
            runtime_markdown
                .lines()
                .any(|line| line.starts_with("### ") && line.contains("runtime")),
            "the action-heading exemption is exercised"
        );
        assert!(
            runtime_markdown
                .lines()
                .any(|line| line.starts_with("- anchor:") && line.contains("runtime")),
            "the anchor exemption is exercised"
        );

        let mut semantic_doc = load_metrics("tests/golden/report-mixed/metrics.json");
        semantic_doc.findings[0].resolver_tier = ResolverTier::Semantic;

        let mut documents: Vec<MetricsDocument> = FIXTURES
            .iter()
            .map(|relative| load_metrics(relative))
            .collect();
        documents.push(synthetic_envelope_doc());
        documents.push(runtime_doc);
        documents.push(semantic_doc);

        for doc in documents {
            let markdown = render_markdown(&doc);
            for line in markdown.lines() {
                if line.starts_with("- why:")
                    || line.starts_with("- what:")
                    || line.starts_with("- function:")
                    || line.starts_with("- evidence:")
                    || line.starts_with("- anchor:")
                    || line.starts_with("### ")
                    || line.starts_with("| PERF-")
                    || line.starts_with("| rule")
                    || line.starts_with("|---")
                    || is_numbered_profile_row(line)
                {
                    continue;
                }
                for claim in [
                    "ms",
                    "µs",
                    "seconds",
                    "speedup",
                    "latency",
                    "throughput",
                    "runtime",
                ] {
                    assert!(
                        !contains_word(line, claim),
                        "forbidden time claim {claim:?} in C-authored line: {line:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn agent_report_sample_matches_committed_contract() {
        let doc = load_metrics("tests/golden/report-mixed/metrics.json");
        let (json, markdown) = render(&doc);
        let sample_dir = fixture_path("docs/evidence/agent-report/sample");
        let expected_json = fs::read_to_string(sample_dir.join("agent-actions.sample.json"))
            .expect("committed agent-actions sample");
        let expected_markdown = fs::read_to_string(sample_dir.join("agent-report.sample.md"))
            .expect("committed agent-report sample");
        assert_eq!(json, expected_json, "agent-actions sample parity");
        assert_eq!(markdown, expected_markdown, "agent-report sample parity");
    }
}
