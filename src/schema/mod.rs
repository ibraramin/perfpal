//! `schema` — metrics.json model & serializer (modules.md §3.5).
//!
//! Owns: the metrics.json document types ([`MetricsDocument`]); the
//! serializer ([`to_json`]); the embedded canonical JSON Schema artifact
//! ([`EMBEDDED_SCHEMA`], byte-copy source for the bundled
//! `metrics.schema.v<MAJOR>.<MINOR>.json`); the [`SCHEMA_VERSION`] constant.
//!
//! Contract: other modules depend on these types only — never on ad-hoc JSON
//! maps. Version bumps follow metrics-json-v1 policy: minor = additive only,
//! major = breaking.
//!
//! Field fidelity is line-by-line against `docs/schema/metrics.schema.v1.6.json`
//! (Draft 2020-12): a field absent from that schema's `required` list is
//! `Option<T>` with `skip_serializing_if`, so optional sections are OMITTED,
//! never emitted as `null` (metrics-json-v1.md §6: explicit nulls are not
//! valid for object-typed optionals). Enum tokens come from the shared IR /
//! analyzer enums (`hot|warn|info`, `HIGH|MEDIUM|LOW`, `syntactic`,
//! `ok|partial|failed`, `quoted|angle`) so there is exactly one source of
//! truth per token.
//!
//! Serializer decision (ONE canonical form, documented per the P1 task):
//! [`serde_json::to_string_pretty`] — 2-space indent, struct fields in
//! declaration order, map keys sorted because serde_json runs with default
//! features (`preserve_order` OFF, modules.md §6 row 3). Pretty output keeps
//! the artifact human/LLM-readable at no determinism cost; compact output
//! would be equally deterministic but strictly worse for consumers.
//! No timestamps are generated here — cli owns provenance values.
//!
//! P4 wave additions (Wave 1): the schema-optional sections gained typed
//! models matching the canonical schema of their version exactly:
//! - `graphs.call_graph` ([`CallGraphSection`] / [`CallEdge`], confidence
//!   locked to `"LOW"` per ADR-0001 name-based matching).
//! - `components[]` ([`ComponentEntry`] / [`ComponentMetrics`]).
//! - `stats.distributions` ([`DistributionEntry`]) — v1.4 step 1 (plan
//!   `divergence-revamp`): the aggregator now populates the
//!   `cyclomatic_complexity` + `max_loop_depth` histograms (fixed buckets per
//!   `docs/schema/stats-distributions-proposal.md`); the `(metric, bucket)`
//!   sort contract is enforced by `aggregator::sort_distributions`.
//!
//! The call_graph and components producers LANDED in P4 (aggregator); the
//! fields remain `Option<_>` with `skip_serializing_if` because ABSENT is
//! still the canonical empty form (omit-don't-null) — e.g. scans with no
//! cross-file edges or single-component repos — not because producers are
//! pending.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverHealth, ResolverTier, Span};

/// Schema version of emitted documents (metrics.schema.v1.6.json
/// `properties.schema_version.const`). MAJOR.MINOR: minor = additive-only;
/// major = breaking (metrics-json-v1.md §5).
///
/// v1.0 → v1.1 (2026-09-02, minor/additive): (a) `files[].parse_health`
/// gained the OPTIONAL `resolver` field (enum `absent|ok|failed`, omitted by
/// producers without resolver support — old outputs validate unchanged);
/// (b) `graphs.call_graph.edges[].confidence` widened from `const "LOW"` to
/// oneOf `["LOW","MEDIUM","HIGH"]` for the semantic tier (Wave B; widening is
/// additive — v1.0 documents still validate).
///
/// v1.1 → v1.2 (2026-09-22, minor/additive): `scan.languages` +
/// `files[].language` widened from const "c" to enum ["c","rust"] (language
/// addition; v1.1 documents still validate).
///
/// v1.2 → v1.3 (minor/additive): `scan.languages` +
/// `files[].language` widened from enum ["c","rust"] to
/// enum ["c","rust","python"] (language addition; v1.2 documents still
/// validate).
///
/// v1.3 → v1.4 (minor/additive, plan `divergence-revamp` step 1): seven
/// OPTIONAL `stats.functions[]` instrument fields (`call_fanout`,
/// `call_fanin`, `call_depth`, `is_recursive`, `idiom_hash`, `error_path`,
/// `alloc_family`) plus the first populated `stats.distributions[]`
/// histograms; v1.3 documents still validate.
///
/// v1.4 → v1.5 (minor/additive, plan `map-build` I1a C + I1b Rust + I1c
/// Python): three OPTIONAL `stats.functions[]` entry/reachability fields
/// (`entry_kind`, `entry_confidence`, `reachable_from_entry`, populated for
/// C, Rust and Python); v1.4 documents still validate.
///
/// v1.5 → v1.6 (minor/additive, plan `map-build` I2 cost-v2): two
/// `stats.functions[]` fields — OPTIONAL `function_cost_v2` (per-iteration
/// weighted cost; omitted only for pre-I2/parser-free documents) and
/// `unknown_trip_loops` (always emitted, default 0). Every v1.5 document
/// still validates (no required list grew).
pub const SCHEMA_VERSION: &str = "1.6";

/// Bundled-schema artifact name (output-contract §3): FULL version always —
/// `"1.6"` ⇒ `metrics.schema.v1.6.json`, never an elided `.0`. Kept as a
/// literal next to [`SCHEMA_VERSION`]; the drift test below asserts the two
/// stay in lockstep.
pub const SCHEMA_ARTIFACT_NAME: &str = "metrics.schema.v1.6.json";

/// Canonical JSON Schema artifact, embedded at compile time (modules.md §3.5).
/// Byte-copy of `docs/schema/metrics.schema.v1.6.json`; byte-equality with the
/// docs copy is asserted by [`tests::embedded_schema_matches_docs_copy`] so
/// the two cannot drift. The v1.5 + v1.4 + v1.3 + v1.2 + v1.1 + v1.0 docs
/// copies are kept as history.
pub const EMBEDDED_SCHEMA: &str = include_str!("metrics.schema.v1.6.json");

// ---------------------------------------------------------------------------
// Document model — nine sections, field-for-field vs the canonical schema
// ---------------------------------------------------------------------------

/// The complete metrics.json document (modules.md §3.4 aggregator output).
///
/// Required sections per the schema's top-level `required` list:
/// `schema_version`, `tool`, `scan`, `files`, `stats`, `findings`.
/// Optional: `parse_health_summary`, `graphs`, `components`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricsDocument {
    pub schema_version: String,
    pub tool: ToolInfo,
    pub scan: ScanInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_health_summary: Option<ParseHealthSummary>,
    pub files: Vec<FileEntry>,
    pub stats: StatsSection,
    pub findings: Vec<FindingEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphs: Option<GraphsSection>,
    /// Hot-component aggregation (PERF-C-ARCH-002). One entry per parent
    /// directory of the scanned paths; rank composite: finding_density ↓ →
    /// complexity_sum ↓ → max_loop_depth ↓ → alloc_site_density ↓ →
    /// path_prefix ↑ (1-based); emission order `(rank, path_prefix)`.
    /// ARCH-002 asymmetry note: `components[].rank` uses finding_density,
    /// while the ARCH-002 heuristic ranks IR-only (no density) — they may
    /// disagree by design.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ComponentEntry>>,
}

/// `tool` — provenance of the producing binary.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

/// `scan` — metadata about the scan run.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScanInfo {
    /// Scanned repository root VERBATIM as given to the CLI (may be absolute);
    /// rendered as provenance only (output-contract §2 paths rule).
    pub root_path: String,
    /// UTC RFC3339 instant captured by cli (provenance owner, modules.md §3.1).
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Sorted-unique languages of the scanned files (derived by the
    /// aggregator, modules.md §3.2/§3.4). On a zero-file scan this is the
    /// supported set `["c","python","rust"]` so schema `minItems: 1` holds
    /// (semantics: "languages scanned for").
    pub languages: Vec<String>,
    pub file_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loc_total: Option<LocCounts>,
}

/// `parse_health_summary` — aggregate parse quality (findings inside failed/
/// error regions are downgraded + flagged, never dropped — modules.md P3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ParseHealthSummary {
    pub ok: u32,
    pub partial: u32,
    pub failed: u32,
}

/// One element of `files[]` — per-file baseline stats, no severity/confidence
/// by design (canonical schema `files.items.description`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileEntry {
    /// Repo-relative POSIX path.
    pub path: String,
    /// `"c"`, `"rust"`, or `"python"` (schema 1.5 admits all three).
    pub language: String,
    pub loc: LocCounts,
    pub function_count: u32,
    pub global_var_count: u32,
    pub include_counts: IncludeCounts,
    pub parse_health: ParseHealthEntry,
}

/// Per-file `parse_health` object. `error_regions` is schema-optional ⇒
/// omitted when the file has none (the common case stays lean). `resolver`
/// is schema-optional (additive v1.1) ⇒ omitted when the producing binary
/// had no semantic-resolver outcome for the file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParseHealthEntry {
    pub status: ParseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_regions: Option<Vec<Span>>,
    pub error_node_count: u32,
    /// Additive v1.1: semantic-resolver outcome (`absent|ok|failed`),
    /// lifted verbatim from the IR `ParseHealth.resolver` honesty field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver: Option<ResolverHealth>,
}

/// `stats` — baseline measurements, always emitted, never findings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatsSection {
    pub functions: Vec<FunctionStats>,
    /// Histograms over function-level stats. Schema-optional ⇒ omitted on a
    /// zero-function scan; otherwise populated by the aggregator (v1.4 step
    /// 1: `cyclomatic_complexity` + `max_loop_depth`, fixed buckets, full
    /// label sets incl. zero-count buckets). Sort contract when populated:
    /// `(metric, bucket)` per modules.md §1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distributions: Option<Vec<DistributionEntry>>,
}

/// One element of `stats.functions[]` — direct lift of the IR
/// FunctionRecord baseline metrics (catalog §2), minus IR-only fields
/// (`param_types`) that the external contract does not carry, plus the v1.4
/// additive instrument fields (all `Option` + omitted when uncomputed, so
/// v1.3 documents validate unchanged).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FunctionStats {
    /// Stable IR identity: `path#name#start_line:start_col` (ir-v1.md §2).
    pub id: String,
    pub name: String,
    pub path: String,
    pub span: Span,
    pub param_count: u32,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub max_loop_depth: u32,
    pub alloc_site_count: u32,
    pub length_lines: u32,
    /// ADDITIVE v1.4 (plan `divergence-revamp` step 1, specs
    /// `divergence-study`/`divergence-paper`): per-function call-shape —
    /// distinct direct-callee count. Skeleton: `None` until a producer with a
    /// whole-program view computes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_fanout: Option<u32>,
    /// ADDITIVE v1.4: per-function call-shape — distinct caller count.
    /// Skeleton: `None` until computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_fanin: Option<u32>,
    /// ADDITIVE v1.4: per-function call-shape — longest static call-chain
    /// depth rooted at this function. Skeleton: `None` until computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_depth: Option<u32>,
    /// ADDITIVE v1.4: per-function call-shape — true when the function can
    /// reach itself statically. Skeleton: `None` until computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_recursive: Option<bool>,
    /// ADDITIVE v1.4: token-normalized idiom hash of the function body
    /// (clone/idiom-rate bucketing). Skeleton: `None` until computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idiom_hash: Option<String>,
    /// ADDITIVE v1.4: true when the body contains an error-handling path
    /// (`try`/`except`, `?`/`Err`-return, `do-while(0)`, `#if 0` /
    /// `cfg(test)` gating, error-branch `free`). Skeleton: `None` until
    /// computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_path: Option<bool>,
    /// ADDITIVE v1.4: dominant allocation family (`real_alloc` vs
    /// `empty_ctor` vs `serialize/format`). Skeleton: `None` until computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alloc_family: Option<String>,
    /// ADDITIVE precision-gate flag (precision FP-class fix 2): `Some(true)`
    /// only for functions defined inside a provably-dead configuration
    /// region (C `#if 0`). Omitted when uncomputed or live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg_gated: Option<bool>,
    /// ADDITIVE v1.5 (plan `map-build` I1a C + I1b Rust + I1c Python):
    /// entry kind — closed set `main` | `script` | `handler` | `lib-root` |
    /// `unknown-entry` (`script` is Python-only: a guarded `<module>`
    /// record). `Some` only for entries (the aggregator's entry passes
    /// cover every function id of each language, so a function with no
    /// entry role carries `None`/omitted, NOT a stub — the `unknown-entry`
    /// stub fires only when the whole per-language corpus yields zero
    /// entries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_kind: Option<String>,
    /// ADDITIVE v1.5: entry confidence — closed set `HIGH` (`main`,
    /// `script`) | `MEDIUM` (`handler`, `lib-root`) | `LOW`
    /// (`unknown-entry` stub). `Some` exactly when
    /// [`FunctionStats::entry_kind`] is `Some`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_confidence: Option<String>,
    /// ADDITIVE v1.5: true when the function is reachable from an entry of
    /// its own language over RESOLVED call edges only (semantic-fact-backed,
    /// MEDIUM+ — the LOW-confidence name-based fallback is EXCLUDED, so a
    /// uniquely-named callee with no semantic fact still reports `false`).
    /// `Some` for every function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reachable_from_entry: Option<bool>,
    /// ADDITIVE v1.6 (plan `map-build` I2): per-iteration weighted cost.
    /// EXACT formula (aggregator-owned, recomputed from IR with the v1.5
    /// coefficients so it can never drift from the retained v1 formula):
    ///
    /// `cost = cyclomatic + cognitive + 2*max_loop_depth
    ///         + 3*alloc_site_count + call_fanout + call_fanin
    ///         + call_depth + (2 if recursive) + (1 if error_path)
    ///         + (1 if alloc_family is Some)
    ///         + SUM over loops of trip_weight`, where
    /// - known trip `t`: `trip_weight = min(t, MAX_TRIP) * (1 + loop.depth)`;
    /// - unknown trip:   `trip_weight = 1 * (1 + loop.depth)` AND the loop
    ///   increments [`FunctionStats::unknown_trip_loops`].
    ///
    /// All arithmetic is saturating (`u64`). v1 [`crate::charts::function_cost`]
    /// stays available until cutover; heat consumers prefer this field when
    /// present and fall back to v1 otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_cost_v2: Option<u64>,
    /// ADDITIVE v1.6: number of loops in the function whose trip count is
    /// genuinely unknown (`LoopNode.trip_count == None`). `0` for
    /// loop-free/canonical-counted functions; always emitted.
    #[serde(default)]
    pub unknown_trip_loops: u32,
}

/// One element of `findings[]` — the analyzer's [`Finding`] lifted into its
/// serialized contract shape. `finding_id` is derived
/// (`rule:path:start_line:start_col`, modules.md §1 analyzer row) and made
/// explicit here because the schema requires it on every finding.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FindingEntry {
    pub finding_id: String,
    pub rule_id: String,
    pub title: String,
    pub path: String,
    /// Enclosing function name when resolvable; schema-optional ⇒ omitted
    /// otherwise (metrics-json-v1.md §6 omit-don't-null rule).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    pub span: Span,
    pub confidence: Confidence,
    pub severity: Severity,
    pub resolver_tier: ResolverTier,
    /// ir-v1.md §5 hook 1: macro origin caps confidence ≤ MEDIUM.
    pub macro_suspect: bool,
    /// ir-v1.md §5 hook 3: error-region overlap caps confidence ≤ MEDIUM.
    pub error_node_overlap: bool,
    /// Schema-optional; LOOP-001 (`min_nesting_depth`) and CALL-002
    /// (`min_complexity`) populate it — rules without a threshold omit it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<Threshold>,
    pub message: String,
    /// Rule-specific machine-readable facts; BTreeMap ⇒ keys serialize
    /// sorted regardless of insertion order (modules.md §6 row 3).
    /// Schema-optional ⇒ omitted when empty.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub evidence: BTreeMap<String, String>,
}

/// Configurable threshold that fired (schema `$defs.threshold`). The value
/// mirrors the schema's `["number", "string"]` union.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Threshold {
    pub name: String,
    pub value: ThresholdValue,
}

/// `threshold.value` union (schema: `type: ["number", "string"]`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ThresholdValue {
    Number(f64),
    Text(String),
}

/// `graphs` — architecture-level structure feeding pillar-1b and ARCH-*.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GraphsSection {
    pub include_graph: IncludeGraph,
    /// Cross-file call edges (ARCH-003, confidence locked `"LOW"` per
    /// ADR-0001). ALWAYS `Some` once assembled — even when `edges` is empty —
    /// so consumers can rely on the `["edges"]` shape (graphs-always-Some
    /// precedent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_graph: Option<CallGraphSection>,
}

/// `graphs.call_graph` — cross-file call edges only (intra-file edges
/// omitted in v1); name-based matching ⇒ every edge confidence LOW until a
/// semantic resolver exists (canonical schema `call_graph`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CallGraphSection {
    pub edges: Vec<CallEdge>,
}

/// One element of `call_graph.edges[]`. Field declaration order is
/// load-bearing: it must match the schema's property order (`from_function`,
/// `to_name`, `from_path`, `to_path`, `confidence`) because serde serializes
/// structs in declaration order. `to_path` is REQUIRED by the schema (may be
/// explicit `null` when unresolved) ⇒ serialized without
/// `skip_serializing_if`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CallEdge {
    /// Caller function id (stable IR identity).
    pub from_function: String,
    /// Callee name as written.
    pub to_name: String,
    pub from_path: String,
    /// Best-match file or `null` when unresolved.
    pub to_path: Option<String>,
    /// Schema v1.1 widened to oneOf `["LOW","MEDIUM","HIGH"]` (additive —
    /// v1.0 `const "LOW"` documents still validate). In Wave A the value is
    /// always `"LOW"`: name-based matching (ADR-0001); the semantic resolver
    /// raises it in Wave B. Construct via [`CallEdge::new`].
    pub confidence: String,
}

impl CallEdge {
    /// Constructor hardcoding the schema-const confidence (`"LOW"`): the
    /// name-based fallback (ADR-0001). Resolved edges use
    /// [`CallEdge::with_confidence`].
    pub fn new(
        from_function: impl Into<String>,
        to_name: impl Into<String>,
        from_path: impl Into<String>,
        to_path: Option<String>,
    ) -> Self {
        Self::with_confidence(from_function, to_name, from_path, to_path, Confidence::Low)
    }

    /// Constructor carrying the edge's own confidence (semantic-resolver
    /// Wave B1): MEDIUM for resolved (USR-backed) edges, LOW for the
    /// name-based fallback — serialized as its schema token.
    pub fn with_confidence(
        from_function: impl Into<String>,
        to_name: impl Into<String>,
        from_path: impl Into<String>,
        to_path: Option<String>,
        confidence: Confidence,
    ) -> Self {
        let token = match confidence {
            Confidence::High => "HIGH",
            Confidence::Medium => "MEDIUM",
            Confidence::Low => "LOW",
        };
        Self {
            from_function: from_function.into(),
            to_name: to_name.into(),
            from_path: from_path.into(),
            to_path,
            confidence: token.to_string(),
        }
    }
}

/// Hot-component aggregation metrics (PERF-C-ARCH-002). Densities are per
/// KLOC and schema-typed `number` ⇒ f64; `complexity_sum` is also `number`
/// in the schema (f64 here), while `max_loop_depth` is `integer` (u32).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentMetrics {
    pub complexity_sum: f64,
    /// Alloc sites per KLOC.
    pub alloc_site_density: f64,
    pub max_loop_depth: u32,
    /// Findings per KLOC.
    pub finding_density: f64,
}

/// One element of `components[]`. Field declaration order matches the
/// schema's property order; `finding_count` is schema-OPTIONAL ⇒ omitted
/// when absent.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComponentEntry {
    pub component_id: String,
    pub path_prefix: String,
    /// 1 = hottest (schema `minimum: 1`).
    pub rank: u32,
    pub metrics: ComponentMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finding_count: Option<u32>,
}

/// One element of `stats.distributions[]` — a histogram bucket over a
/// function-level stat field (e.g. metric `length_lines`, bucket `"1-10"`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DistributionEntry {
    /// Function stat field name being distributed.
    pub metric: String,
    pub bucket: String,
    pub count: u32,
}

/// Include dependency graph: every scanned file as a node plus resolved
/// internal edges (modules.md §3.4).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncludeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<IncludeEdgeDto>,
}

/// Include-graph node.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GraphNode {
    pub path: String,
}

/// Include-graph edge. Admission rule (m6, P4 Wave 2b): resolved internal
/// edges (quoted or angle-in-repo) PLUS unresolved QUOTED edges (`to: null`
/// — data, not error, modules.md Principle 4). Unresolved ANGLE edges stay
/// EXCLUDED upstream (external bucket).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncludeEdgeDto {
    pub from: String,
    /// Resolved repo-relative POSIX path; explicit `null` for unresolved
    /// quoted edges (schema `"type": ["string", "null"]`). Serialized
    /// WITHOUT `skip_serializing_if` — the null is load-bearing.
    pub to: Option<String>,
    pub form: crate::ir::IncludeForm,
    pub resolved: bool,
    /// Location of the `#include` directive (report drill-down). Always
    /// present because the IR edge always carries it.
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Shared numeric formatting — the SINGLE rounding implementation for every
// numeric text the tool emits (charts SVG labels/geometry, report.md
// densities). Both consumers consume schema DTOs, so this lives next to them
// without a layering violation; the former per-module copies (charts'
// binary-scaling floor, reporter's private digit method) were unified here.
// ---------------------------------------------------------------------------

/// Round `value` to `scale` decimal places, HALF-UP **away from zero**
/// (pinned: -2.5 @0 → -3, 2.5 @0 → 3, 0.125 @2 → 13 i.e. "0.13").
///
/// Digit-inspection algorithm: render `|value|` at `scale + 1` decimals
/// (Rust's float formatting is correctly rounded), then round the trailing
/// digit half-up into the kept digits. This avoids the binary-representation
/// drift of naive scaling (`value * factor + 0.5` floors 2.675 @2 to 2.67
/// because 2.675 is not representable); decimal-representable borderline
/// values round half-up as pinned. A scaled value of zero never carries a
/// sign ("-0.00" cannot leak). Inputs whose magnitude fits `i64` after
/// scaling behave exactly; callers feed small counts/densities only.
#[must_use]
pub(crate) fn round_half_up(value: f64, scale: u32) -> i64 {
    debug_assert!(value.is_finite(), "only finite values reach output text");
    let width = scale as usize + 1;
    let rendered = format!("{:.width$}", value.abs(), width = width);
    let (int_part, frac_part) = rendered.split_once('.').unwrap_or((&rendered, ""));
    let mut digits = [0u32; 32]; // scale+1 ≤ 32 by contract (call sites use 0..=2)
    assert!(width <= digits.len(), "scale out of supported range");
    for (slot, byte) in frac_part.bytes().take(width).enumerate() {
        digits[slot] = u32::from(byte - b'0');
    }
    let mut int_value: i64 = int_part.parse().unwrap_or(0);
    let mut frac_value: i64 = digits[..width - 1]
        .iter()
        .fold(0_i64, |acc, &d| acc * 10 + i64::from(d));
    if digits[width - 1] >= 5 {
        frac_value += 1;
    }
    let factor = 10_i64.pow(scale);
    if frac_value >= factor {
        int_value += 1;
        frac_value -= factor;
    }
    let magnitude = int_value * factor + frac_value;
    if value.is_sign_negative() && magnitude != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// Format `value` fixed-point at `scale` decimals with HALF-UP rounding
/// ([`round_half_up`]) — the ONLY way non-integer numbers reach output text
/// in charts and report.md.
#[must_use]
pub(crate) fn format_fixed_point(value: f64, scale: u32) -> String {
    let scaled = round_half_up(value, scale);
    if scale == 0 {
        return scaled.to_string();
    }
    // Magnitude-based split: `div_euclid`/`rem_euclid` on a NEGATIVE scaled
    // value would produce non-canonical text ("-3.32" for -2.68).
    let factor = 10_i64.pow(scale);
    let sign = if scaled < 0 { "-" } else { "" };
    let magnitude = scaled.abs();
    let int_part = magnitude / factor;
    let frac = magnitude % factor;
    format!("{sign}{int_part}.{frac:0width$}", width = scale as usize)
}

// ---------------------------------------------------------------------------
// Serializer
// ---------------------------------------------------------------------------

/// Canonical serialization (see module docs: pretty, declaration-order
/// fields, sorted map keys, no generated timestamps).
///
/// # Errors
/// Propagates [`serde_json::Error`] — unreachable for the in-memory document
/// shape (no maps with non-string keys, no NaN floats), but kept typed so
/// callers can route failures to the internal-error exit path.
pub fn to_json(doc: &MetricsDocument) -> serde_json::Result<String> {
    serde_json::to_string_pretty(doc)
}

/// Lift an analyzer [`Finding`] into its schema-typed [`FindingEntry`].
///
/// `symbol` resolution (enclosing-function lookup) is the AGGREGATOR's job —
/// pass `None` when unknown. Splitting lift from assembly keeps this helper
/// usable by tests without a full IR stream.
#[must_use]
pub fn finding_entry(finding: &Finding, symbol: Option<String>) -> FindingEntry {
    FindingEntry {
        finding_id: finding.finding_id(),
        rule_id: finding.rule_id.clone(),
        title: finding.title.clone(),
        path: finding.path.clone(),
        symbol,
        span: finding.span,
        confidence: finding.confidence,
        severity: finding.severity,
        resolver_tier: finding.resolver_tier,
        macro_suspect: finding.macro_suspect,
        error_node_overlap: finding.error_node_overlap,
        threshold: finding.threshold.map(|(name, value)| Threshold {
            name: name.to_string(),
            value: ThresholdValue::Number(f64::from(value)),
        }),
        message: finding.message.clone(),
        evidence: finding.evidence.clone(),
    }
}

#[cfg(test)]
mod tests {
    //! Contract locks: schema-artifact drift guard, serializer determinism,
    //! required/optional fidelity.

    use super::*;

    /// DRIFT GUARD: the embedded bundled artifact must remain a byte-copy of
    /// the canonical schema under docs/ (output-contract §3). Any edit to one
    /// without the other fails here.
    #[test]
    fn embedded_schema_matches_docs_copy_byte_for_byte() {
        let docs_copy = include_str!("../../docs/schema/metrics.schema.v1.6.json");
        assert_eq!(
            EMBEDDED_SCHEMA, docs_copy,
            "src/schema/metrics.schema.v1.6.json drifted from \
             docs/schema/metrics.schema.v1.6.json — re-copy the canonical artifact"
        );
    }

    /// Artifact-name derivation lock (output-contract §3: full version, no
    /// elided `.0`): SCHEMA_ARTIFACT_NAME must be built from SCHEMA_VERSION.
    #[test]
    fn artifact_name_derives_from_schema_version() {
        assert_eq!(
            SCHEMA_ARTIFACT_NAME,
            format!("metrics.schema.v{SCHEMA_VERSION}.json")
        );
    }

    fn sample_doc() -> MetricsDocument {
        let mut evidence = BTreeMap::new();
        // Deliberately unsorted insertion order — output must still be sorted.
        evidence.insert("loop_depth".to_string(), "2".to_string());
        evidence.insert("alloc_site_index".to_string(), "1".to_string());
        evidence.insert("callee".to_string(), "malloc".to_string());

        MetricsDocument {
            schema_version: SCHEMA_VERSION.to_string(),
            tool: ToolInfo {
                name: "perfpal".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            scan: ScanInfo {
                root_path: ".".to_string(),
                started_at: "2026-08-24T00:00:00Z".to_string(),
                duration_ms: Some(12),
                languages: vec!["c".to_string()],
                file_count: 1,
                loc_total: Some(LocCounts {
                    code: 10,
                    comment: 2,
                    blank: 1,
                }),
            },
            parse_health_summary: Some(ParseHealthSummary {
                ok: 1,
                partial: 0,
                failed: 0,
            }),
            files: vec![FileEntry {
                path: "a.c".to_string(),
                language: "c".to_string(),
                loc: LocCounts {
                    code: 10,
                    comment: 2,
                    blank: 1,
                },
                function_count: 1,
                global_var_count: 0,
                include_counts: IncludeCounts {
                    quoted: 1,
                    angle: 1,
                },
                parse_health: ParseHealthEntry {
                    status: ParseStatus::Ok,
                    error_regions: None,
                    error_node_count: 0,
                    resolver: None,
                },
            }],
            stats: StatsSection {
                functions: vec![FunctionStats {
                    id: "a.c#fill#4:1".to_string(),
                    name: "fill".to_string(),
                    path: "a.c".to_string(),
                    span: Span::point(4, 1),
                    param_count: 1,
                    cyclomatic_complexity: 2,
                    cognitive_complexity: 1,
                    max_loop_depth: 1,
                    alloc_site_count: 1,
                    length_lines: 6,
                    // v1.4 skeleton: uncomputed instrument fields stay None
                    // (omitted on the wire).
                    call_fanout: None,
                    call_fanin: None,
                    call_depth: None,
                    is_recursive: None,
                    idiom_hash: None,
                    error_path: None,
                    alloc_family: None,
                    cfg_gated: None,
                    // v1.5: uncomputed entry/reachability fields stay None
                    // (omitted on the wire).
                    entry_kind: None,
                    entry_confidence: None,
                    reachable_from_entry: None,
                    function_cost_v2: None,
                    unknown_trip_loops: 0,
                }],
                distributions: None,
            },
            findings: vec![FindingEntry {
                finding_id: "PERF-C-ALLOC-001:a.c:6:9".to_string(),
                rule_id: "PERF-C-ALLOC-001".to_string(),
                title: "Heap allocation inside loop".to_string(),
                path: "a.c".to_string(),
                symbol: Some("fill".to_string()),
                span: Span::point(6, 9),
                confidence: Confidence::High,
                severity: Severity::Hot,
                resolver_tier: ResolverTier::Syntactic,
                macro_suspect: false,
                error_node_overlap: false,
                threshold: None,
                message: "candidate optimization".to_string(),
                evidence,
            }],
            graphs: Some(GraphsSection {
                include_graph: IncludeGraph {
                    nodes: vec![GraphNode {
                        path: "a.c".to_string(),
                    }],
                    edges: vec![],
                },
                call_graph: None,
            }),
            components: None,
        }
    }

    /// Same document serialized twice ⇒ identical bytes (byte-determinism,
    /// modules.md §6 row 3).
    #[test]
    fn serialization_is_byte_deterministic() {
        let doc = sample_doc();
        let first = to_json(&doc).expect("serialize pass 1");
        let second = to_json(&doc).expect("serialize pass 2");
        assert_eq!(first, second);
    }

    /// Map-key sorting regardless of insertion order: the finding's evidence
    /// BTreeMap was built z-order-ish; serialized keys MUST be alphabetical.
    #[test]
    fn evidence_keys_serialize_sorted() {
        let doc = sample_doc();
        let out = to_json(&doc).expect("serialize");
        let callee = out.find("\"callee\"").expect("callee key present");
        let loop_depth = out.find("\"loop_depth\"").expect("loop_depth key present");
        let alloc_idx = out
            .find("\"alloc_site_index\"")
            .expect("alloc_site_index key present");
        assert!(
            alloc_idx < callee && callee < loop_depth,
            "evidence keys not sorted:\n{out}"
        );
    }

    /// Optional-in-schema fields are OMITTED, never null (metrics-json-v1 §6):
    /// drop all optionals from the sample and confirm neither `null` nor the
    /// optional keys appear.
    #[test]
    fn optional_fields_are_omitted_not_null() {
        let mut doc = sample_doc();
        doc.scan.duration_ms = None;
        doc.scan.loc_total = None;
        doc.parse_health_summary = None;
        doc.graphs = None;
        doc.files[0].parse_health.error_regions = None;
        doc.findings[0].symbol = None;
        doc.findings[0].evidence.clear();
        let out = to_json(&doc).expect("serialize");
        assert!(
            !out.contains("null"),
            "explicit null leaked into output:\n{out}"
        );
        for absent in [
            "\"duration_ms\"",
            "\"loc_total\"",
            "\"parse_health_summary\"",
            "\"graphs\"",
            "\"error_regions\"",
            "\"symbol\"",
            "\"evidence\"",
            "\"threshold\"",
            "\"components\"",
            "\"distributions\"",
            "\"call_graph\"",
            // v1.4 skeleton: uncomputed instrument fields are omitted.
            "\"call_fanout\"",
            "\"call_fanin\"",
            "\"call_depth\"",
            "\"is_recursive\"",
            "\"idiom_hash\"",
            "\"error_path\"",
            "\"alloc_family\"",
            // v1.5: uncomputed entry/reachability fields are omitted.
            "\"entry_kind\"",
            "\"entry_confidence\"",
            "\"reachable_from_entry\"",
        ] {
            assert!(!out.contains(absent), "{absent} should be omitted:\n{out}");
        }
    }

    /// P4 Wave 1: a fully-populated document serializes with key order
    /// matching the canonical schema property order (declaration order is
    /// load-bearing), and round-trips through `serde_json::Value` with the
    /// expected structure — including an explicit `null` `to_path` and an
    /// omitted-when-absent `finding_count`.
    #[test]
    fn p4_sections_serialize_in_schema_field_order() {
        let mut doc = sample_doc();
        doc.stats.distributions = Some(vec![
            DistributionEntry {
                metric: "length_lines".to_string(),
                bucket: "1-10".to_string(),
                count: 3,
            },
            DistributionEntry {
                metric: "cyclomatic_complexity".to_string(),
                bucket: "1-5".to_string(),
                count: 7,
            },
        ]);
        if let Some(graphs) = doc.graphs.as_mut() {
            graphs.call_graph = Some(CallGraphSection {
                edges: vec![CallEdge::new("a.c#fill#4:1", "malloc", "a.c", None)],
            });
        }
        doc.components = Some(vec![
            ComponentEntry {
                component_id: "src/net".to_string(),
                path_prefix: "src/net".to_string(),
                rank: 1,
                metrics: ComponentMetrics {
                    complexity_sum: 42.0,
                    alloc_site_density: 12.5,
                    max_loop_depth: 3,
                    finding_density: 8.25,
                },
                finding_count: Some(5),
            },
            ComponentEntry {
                component_id: "src/io".to_string(),
                path_prefix: "src/io".to_string(),
                rank: 2,
                metrics: ComponentMetrics {
                    complexity_sum: 10.0,
                    alloc_site_density: 0.0,
                    max_loop_depth: 1,
                    finding_density: 2.0,
                },
                finding_count: None,
            },
        ]);

        let out = to_json(&doc).expect("serialize");

        // Key ORDER must be checked on the text: serde_json::Value sorts map
        // keys (preserve_order OFF), so it cannot observe declaration order.
        let pos = |needle: &str, from: usize| {
            out[from..]
                .find(needle)
                .unwrap_or_else(|| panic!("{needle} missing from output:\n{out}"))
                + from
        };
        let start = pos("\"schema_version\"", 0);
        let tool = pos("\"tool\"", start);
        let scan = pos("\"scan\"", tool);
        let files = pos("\"files\"", scan);
        let stats = pos("\"stats\"", files);
        let findings = pos("\"findings\"", stats);
        let graphs = pos("\"graphs\"", findings);
        let components = pos("\"components\"", graphs);
        assert!(start < tool && tool < scan && scan < files);
        assert!(files < stats && stats < findings && findings < graphs && graphs < components);

        // stats: functions before distributions.
        let functions = pos("\"functions\"", stats);
        let distributions = pos("\"distributions\"", functions);
        assert!(functions < distributions);

        // graphs: include_graph before call_graph.
        let include_graph = pos("\"include_graph\"", graphs);
        let call_graph = pos("\"call_graph\"", include_graph);
        assert!(include_graph < call_graph);

        // call edge: exact schema field order.
        let from_function = pos("\"from_function\"", call_graph);
        let to_name = pos("\"to_name\"", from_function);
        let from_path = pos("\"from_path\"", to_name);
        let to_path = pos("\"to_path\"", from_path);
        let confidence = pos("\"confidence\"", to_path);
        assert!(
            from_function < to_name
                && to_name < from_path
                && from_path < to_path
                && to_path < confidence
        );

        // Round-trip through serde_json::Value: structure + values.
        let value: serde_json::Value =
            serde_json::from_str(&out).expect("output must be valid JSON");
        let edge = &value["graphs"]["call_graph"]["edges"][0];
        assert_eq!(edge["from_function"], "a.c#fill#4:1");
        assert_eq!(edge["to_name"], "malloc");
        assert_eq!(edge["from_path"], "a.c");
        assert!(
            edge["to_path"].is_null(),
            "unresolved to_path serializes null"
        );
        assert_eq!(edge["confidence"], "LOW");
        assert_eq!(value["stats"]["distributions"][0]["metric"], "length_lines");
        assert_eq!(value["stats"]["distributions"][0]["bucket"], "1-10");
        assert_eq!(value["stats"]["distributions"][0]["count"], 3);
        assert_eq!(
            value["stats"]["distributions"][1]["metric"],
            "cyclomatic_complexity"
        );
        assert_eq!(value["components"][0]["component_id"], "src/net");
        assert_eq!(value["components"][0]["rank"], 1);
        assert_eq!(value["components"][0]["finding_count"], 5);
        assert_eq!(value["components"][0]["metrics"]["complexity_sum"], 42.0);
        assert_eq!(
            value["components"][0]["metrics"]["alloc_site_density"],
            12.5
        );
        assert_eq!(value["components"][0]["metrics"]["max_loop_depth"], 3);
        assert_eq!(value["components"][0]["metrics"]["finding_density"], 8.25);
        assert!(
            value["components"][1].get("finding_count").is_none(),
            "absent finding_count must be omitted, not null"
        );
    }

    /// Empty/minimal document: with every P4 section left `None`, output is
    /// valid JSON and none of the new optional keys appear at all.
    #[test]
    fn minimal_document_still_serializes_without_p4_sections() {
        let mut doc = sample_doc();
        doc.stats.distributions = None;
        if let Some(graphs) = doc.graphs.as_mut() {
            graphs.call_graph = None;
        }
        doc.components = None;
        let out = to_json(&doc).expect("serialize minimal document");
        for absent in ["\"call_graph\"", "\"components\"", "\"distributions\""] {
            assert!(!out.contains(absent), "{absent} should be absent:\n{out}");
        }
        let value: serde_json::Value =
            serde_json::from_str(&out).expect("minimal output must be valid JSON");
        assert!(value.get("components").is_none());
        assert!(value["stats"].get("distributions").is_none());
        assert!(value["graphs"].get("call_graph").is_none());
    }

    /// The call-edge confidence is the schema `const: "LOW"` — both on the
    /// struct (constructor hardcodes it) and in serialized form.
    #[test]
    fn call_edge_confidence_is_always_low() {
        let resolved = CallEdge::new("a.c#f#1:1", "g", "a.c", Some("b.c".to_string()));
        let unresolved = CallEdge::new("a.c#f#1:1", "h", "a.c", None);
        assert_eq!(resolved.confidence, "LOW");
        assert_eq!(unresolved.confidence, "LOW");
        for edge in [resolved, unresolved] {
            let value = serde_json::to_value(&edge).expect("serialize edge");
            assert_eq!(value["confidence"], "LOW");
        }
    }
}
