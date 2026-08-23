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
//! Field fidelity is line-by-line against `docs/schema/metrics.schema.v1.json`
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
//! P1 scope notes (P4 follow-ups documented at point of use):
//! - `components[]` omitted (schema-optional; ARCH-002 aggregation lands P4).
//! - `stats.distributions` omitted (schema-optional; P4 completes it).
//! - `graphs.call_graph` omitted (schema-optional; ARCH-003 cross-file edges
//!   land with the P2 heuristic wave).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};

/// Schema version of emitted documents (metrics.schema.v1.json
/// `properties.schema_version.const`). MAJOR.MINOR: minor = additive-only;
/// major = breaking (metrics-json-v1.md §5).
pub const SCHEMA_VERSION: &str = "1.0";

/// Bundled-schema artifact name (output-contract §3): FULL version always —
/// `"1.0"` ⇒ `metrics.schema.v1.0.json`, never an elided `.0`. Kept as a
/// literal next to [`SCHEMA_VERSION`]; the drift test below asserts the two
/// stay in lockstep.
pub const SCHEMA_ARTIFACT_NAME: &str = "metrics.schema.v1.0.json";

/// Canonical JSON Schema artifact, embedded at compile time (modules.md §3.5).
/// Byte-copy of `docs/schema/metrics.schema.v1.json`; byte-equality with the
/// docs copy is asserted by [`tests::embedded_schema_matches_docs_copy`] so
/// the two cannot drift.
pub const EMBEDDED_SCHEMA: &str = include_str!("metrics.schema.v1.0.json");

// ---------------------------------------------------------------------------
// Document model — nine sections, field-for-field vs metrics.schema.v1.json
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
    // components[]: schema-OPTIONAL ⇒ omitted in P1. P4 follow-up:
    // ARCH-002 hot-component aggregation (rank by aggregated metrics,
    // sorted `(rank, path_prefix)` per modules.md §1) fills this in.
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
    /// Exactly `["c"]` in v1 (schema `minItems: 1` holds even on zero-C-file
    /// scans; modules.md §3.2).
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
/// by design (metrics.schema.v1.json `files.items.description`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FileEntry {
    /// Repo-relative POSIX path.
    pub path: String,
    /// `"c"` in v1.
    pub language: String,
    pub loc: LocCounts,
    pub function_count: u32,
    pub global_var_count: u32,
    pub include_counts: IncludeCounts,
    pub parse_health: ParseHealthEntry,
}

/// Per-file `parse_health` object. `error_regions` is schema-optional ⇒
/// omitted when the file has none (the common case stays lean).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParseHealthEntry {
    pub status: ParseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_regions: Option<Vec<Span>>,
    pub error_node_count: u32,
}

/// `stats` — baseline measurements, always emitted, never findings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatsSection {
    pub functions: Vec<FunctionStats>,
    // distributions: schema-OPTIONAL ⇒ omitted in P1. P4 follow-up completes
    // it; sort contract then: `(metric, bucket)` per modules.md §1.
}

/// One element of `stats.functions[]` — direct lift of the IR
/// FunctionRecord baseline metrics (catalog §2), minus IR-only fields
/// (`param_types`) that the external contract does not carry.
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
    /// Schema-optional; no v1 heuristic emits one yet ⇒ always omitted.
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
    // call_graph: schema-OPTIONAL ⇒ omitted in P1. P2/P4 follow-up:
    // ARCH-003 cross-file name-matched edges (confidence locked LOW per
    // ADR-0001) fill this in.
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

/// Include-graph edge — resolved internal dependencies only.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncludeEdgeDto {
    pub from: String,
    /// Resolved repo-relative POSIX path; `null` only for unresolvable edges,
    /// which are EXCLUDED from the graph in v1 (they live in `files[]`
    /// include_counts + ARCH-001 findings instead).
    pub to: String,
    pub form: crate::ir::IncludeForm,
    pub resolved: bool,
    /// Location of the `#include` directive (report drill-down). Always
    /// present because the IR edge always carries it.
    pub span: Span,
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
        threshold: None,
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
        let docs_copy = include_str!("../../docs/schema/metrics.schema.v1.json");
        assert_eq!(
            EMBEDDED_SCHEMA, docs_copy,
            "src/schema/metrics.schema.v1.0.json drifted from \
             docs/schema/metrics.schema.v1.json — re-copy the canonical artifact"
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
                version: "0.1.0".to_string(),
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
                }],
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
            }),
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
        ] {
            assert!(!out.contains(absent), "{absent} should be omitted:\n{out}");
        }
    }
}
