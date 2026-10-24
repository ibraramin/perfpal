//! `analyzer` — heuristic registry execution (modules.md §3.3).
//!
//! Consumes: IR stream (read-only). Produces: [`Finding`]s in the report §4
//! total order (severity → confidence → rule_id → path → start_line →
//! start_col) so document order equals report order.
//!
//! Responsibilities: execute every registered heuristic over the IR; enforce
//! ParseHealth gating and the downgrade hooks (macro origin, error-region
//! overlap — ir-v1.md §5; cross-file name-based edges are ARCH-003's own LOW
//! tier and arrive with that rule); construct `finding_id` values; enforce
//! uniqueness — identical `(rule_id, path, start_line, start_col)` MUST NOT
//! exist; the registry collapses duplicates deterministically (first in
//! execution order; heuristics run in ascending rule_id).
//!
//! Purity rules (modules.md §4.1): heuristics are pure IR-view functions —
//! no I/O, no clock, no global state, no sibling-order dependence. ParseHealth
//! gating is applied CENTRALLY after candidate collection so no heuristic can
//! forget it: findings overlapping an error region are capped at MEDIUM and
//! flagged `error_node_overlap` — downgraded, never silently dropped
//! (modules.md Principle 3). Symmetrically, semantic upgrades pass through a
//! central consistency gate ([`apply_upgrade_hooks`], plan
//! `semantic-resolver` Wave B1): a finding may claim
//! `resolver_tier: semantic` only when the IR stream carries backing facts at
//! its source. Parser-free testing against tests/fixtures/ir/ per ADR-0001
//! guardrail 2.
//!
//! Hard seam rule (ADR-0001 guardrail 1): this module imports ONLY
//! `crate::ir` (+ `crate::scanner`'s IR-stream container types) — zero
//! tree-sitter types.
//!
//! Non-goals: graph/component building; stats assembly; rendering; runtime
//! threshold configuration.

use std::collections::BTreeMap;

use crate::ir::{MacroEventKind, ParseStatus, ResolverTier};
use crate::scanner::FileUnit;

pub mod finding;
use finding::{ALLOC_002_RULE_ID, ARCH_003_RULE_ID, RS_ARCH_003_RULE_ID, STR_003_RULE_ID};
pub use finding::{Confidence, Finding, Severity};

/// Registry of heuristics, executed in ascending `rule_id` order
/// (modules.md §3.3). Each entry is a pure function over read-only IR views.
type HeuristicFn = fn(&[FileUnit]) -> Vec<Finding>;

/// Language an entry runs on (plan `rust-backend` R3 workstream C1):
/// `PERF-C-*` rules must never run on Rust files and vice versa — a
/// name-based C rule (ALLOC/STR/ARCH) would otherwise fire on same-named Rust
/// constructs in a mixed scan. Grouping is by `FileRecord.language`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleLanguage {
    C,
    Python,
    Rust,
}

/// One registry entry: the language slice it is handed, its rule id (the
/// single spelling every catalog/Appendix-A lookup keys on), and its function.
pub(crate) struct RegistryEntry {
    language: RuleLanguage,
    /// Read by [`registered_rule_ids`] — the registry→catalog completeness
    /// seam consumed by the reporter lock (test-only consumer today).
    #[cfg_attr(not(test), allow(dead_code))]
    rule_id: &'static str,
    f: HeuristicFn,
}

/// The registry, DECLARED IN ASCENDING `rule_id` ORDER ACROSS ALL THREE
/// LANGUAGES (all `PERF-C-*` ascending, then all `PERF-PY-*` ascending, then
/// all `PERF-RS-*` ascending) — execution order equals declaration order and
/// duplicate collapse is first-wins, so declaration order is load-bearing
/// (modules.md §3.3). Adding a heuristic = inserting a new entry whose rule_id
/// sorts into place (open pattern, minor-bump schema policy).
///
/// C: alloc_001, alloc_002, alloc_003, arch_001, arch_002, arch_003, call_001,
/// call_002, loop_001, loop_002, loop_003, loop_004, loop_005, loop_006,
/// loop_007, loop_008, mem_001, mem_002, str_001, str_002, str_003,
/// type_001, type_002.
/// Python (plan `python-backend` W3 — 13 reframed mirrors + 9 native): the 22
/// `PERF-PY-*` rules from docs/architecture/python-backend.md §9, ascending.
/// Rust (waves C1 structural + C2 API-name) — the 22-rule Rust catalog
/// (STR-001 retired by the M1 Precision Gate), ascending: alloc_001,
/// alloc_002, alloc_003, arch_001, arch_002, arch_003, call_001, call_002,
/// loop_001, loop_002, loop_003, loop_004, loop_005, loop_006, loop_007,
/// loop_008, mem_001, mem_002, str_002, str_003, type_001, type_002.
const REGISTRY: &[RegistryEntry] = &[
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::ALLOC_001_RULE_ID,
        f: heuristics::alloc_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::ALLOC_002_RULE_ID,
        f: heuristics::alloc_002,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::ALLOC_003_RULE_ID,
        f: heuristics::alloc_003,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::ARCH_001_RULE_ID,
        f: heuristics::arch_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::ARCH_002_RULE_ID,
        f: heuristics::arch_002,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::ARCH_003_RULE_ID,
        f: heuristics::arch_003,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::CALL_001_RULE_ID,
        f: heuristics::call_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::CALL_002_RULE_ID,
        f: heuristics::call_002,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_001_RULE_ID,
        f: heuristics::loop_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_002_RULE_ID,
        f: heuristics::loop_002,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_003_RULE_ID,
        f: heuristics::loop_003,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_004_RULE_ID,
        f: heuristics::loop_004,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_005_RULE_ID,
        f: heuristics::loop_005,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_006_RULE_ID,
        f: heuristics::loop_006,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_007_RULE_ID,
        f: heuristics::loop_007,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::LOOP_008_RULE_ID,
        f: heuristics::loop_008,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::MEM_001_RULE_ID,
        f: heuristics::mem_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::MEM_002_RULE_ID,
        f: heuristics::mem_002,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::STR_001_RULE_ID,
        f: heuristics::str_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::STR_002_RULE_ID,
        f: heuristics::str_002,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::STR_003_RULE_ID,
        f: heuristics::str_003,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::TYPE_001_RULE_ID,
        f: heuristics::type_001,
    },
    RegistryEntry {
        language: RuleLanguage::C,
        rule_id: finding::TYPE_002_RULE_ID,
        f: heuristics::type_002,
    },
    // ---- Python (`PERF-PY-*`) block — ascending rule_id (W3). ----
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ALLOC_001_RULE_ID,
        f: heuristics::python_alloc_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ALLOC_003_RULE_ID,
        f: heuristics::python_alloc_003,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ALLOC_004_RULE_ID,
        f: heuristics::python_alloc_004,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ARCH_001_RULE_ID,
        f: heuristics::python_arch_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ARCH_002_RULE_ID,
        f: heuristics::python_arch_002,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ARCH_003_RULE_ID,
        f: heuristics::python_arch_003,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ASYNC_001_RULE_ID,
        f: heuristics::python_async_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_ASYNC_002_RULE_ID,
        f: heuristics::python_async_002,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_CALL_001_RULE_ID,
        f: heuristics::python_call_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_CALL_002_RULE_ID,
        f: heuristics::python_call_002,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_COMPREHENSION_001_RULE_ID,
        f: heuristics::python_comprehension_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_EXECUTOR_001_RULE_ID,
        f: heuristics::python_executor_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_LOOP_001_RULE_ID,
        f: heuristics::python_loop_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_LOOP_002_RULE_ID,
        f: heuristics::python_loop_002,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_LOOP_003_RULE_ID,
        f: heuristics::python_loop_003,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_LOOP_007_RULE_ID,
        f: heuristics::python_loop_007,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_LOOP_009_RULE_ID,
        f: heuristics::python_loop_009,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_MEM_002_RULE_ID,
        f: heuristics::python_mem_002,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_N1_001_RULE_ID,
        f: heuristics::python_n1_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_PANDAS_001_RULE_ID,
        f: heuristics::python_pandas_001,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_PANDAS_002_RULE_ID,
        f: heuristics::python_pandas_002,
    },
    RegistryEntry {
        language: RuleLanguage::Python,
        rule_id: finding::PY_STR_001_RULE_ID,
        f: heuristics::python_str_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_ALLOC_001_RULE_ID,
        f: heuristics::rust_alloc_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_ALLOC_002_RULE_ID,
        f: heuristics::rust_alloc_002,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_ALLOC_003_RULE_ID,
        f: heuristics::rust_alloc_003,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_ARCH_001_RULE_ID,
        f: heuristics::rust_arch_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_ARCH_002_RULE_ID,
        f: heuristics::rust_arch_002,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_ARCH_003_RULE_ID,
        f: heuristics::rust_arch_003,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_CALL_001_RULE_ID,
        f: heuristics::rust_call_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_CALL_002_RULE_ID,
        f: heuristics::rust_call_002,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_001_RULE_ID,
        f: heuristics::rust_loop_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_002_RULE_ID,
        f: heuristics::rust_loop_002,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_003_RULE_ID,
        f: heuristics::rust_loop_003,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_004_RULE_ID,
        f: heuristics::rust_loop_004,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_005_RULE_ID,
        f: heuristics::rust_loop_005,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_006_RULE_ID,
        f: heuristics::rust_loop_006,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_007_RULE_ID,
        f: heuristics::rust_loop_007,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_LOOP_008_RULE_ID,
        f: heuristics::rust_loop_008,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_MEM_001_RULE_ID,
        f: heuristics::rust_mem_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_MEM_002_RULE_ID,
        f: heuristics::rust_mem_002,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_STR_002_RULE_ID,
        f: heuristics::rust_str_002,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_STR_003_RULE_ID,
        f: heuristics::rust_str_003,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_TYPE_001_RULE_ID,
        f: heuristics::rust_type_001,
    },
    RegistryEntry {
        language: RuleLanguage::Rust,
        rule_id: finding::RS_TYPE_002_RULE_ID,
        f: heuristics::rust_type_002,
    },
];

/// The registered rule ids in DECLARATION ORDER (ascending rule_id across both
/// languages: the C block then the `PERF-RS-*` block). Exposed so the reporter's
/// catalog-completeness lock can derive its expected set FROM the registry
/// instead of a hand-maintained duplicate; a registry entry whose rule id is
/// not cataloged then fails the lock rather than reaching Appendix A.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn registered_rule_ids() -> impl Iterator<Item = &'static str> {
    REGISTRY.iter().map(|entry| entry.rule_id)
}

mod heuristics;

/// Cross-file call-edge producer shared with the aggregator's
/// `graphs.call_graph` assembly (P4 Wave 2b). Same data feeds ARCH-003
/// findings and the document — one producer, two consumers.
/// (`scanf_format_index` is additionally re-exported for the semantic
/// resolver's Wave B2 literal-args pass — one contract, two consumers.)
pub(crate) use heuristics::{CrossFileCallEdge, cross_file_call_edges, scanf_format_index};

/// Run the full registry over the IR stream and return findings in the
/// report §4 total order (modules.md §3.3 / report §4):
/// severity → confidence → rule_id → path → start_line → start_col.
#[must_use]
pub fn analyze(units: &[FileUnit]) -> Vec<Finding> {
    // Language gating (plan `rust-backend` R3 workstream C1; extended by plan
    // `python-backend` W3). The registry is language-tagged, so each entry must
    // receive ONLY the units for its own language — otherwise a name-based C
    // rule (ALLOC/STR/ARCH) would fire on same-named constructs in another
    // language. The units are partitioned ONCE here into three owned slices (a
    // single deep clone of every `FileUnit`); the alternative — handing each
    // rule the full borrowed set and repeating the language filter inside all
    // 67 rules — trades that one upfront clone for 67 per-rule passes over the
    // units. The upfront split is paid once per `analyze` call and keeps every
    // rule a pure, language-homogeneous IR view. `"c"`, `"python"` and `"rust"`
    // are bucketed explicitly; any OTHER language is dropped — it has no
    // registry rules and MUST NOT be scored by another language's heuristics.
    let mut c_units: Vec<FileUnit> = Vec::new();
    let mut python_units: Vec<FileUnit> = Vec::new();
    let mut rust_units: Vec<FileUnit> = Vec::new();
    for unit in units {
        match unit.file.language.as_str() {
            "rust" => rust_units.push(unit.clone()),
            "c" => c_units.push(unit.clone()),
            "python" => python_units.push(unit.clone()),
            _ => {}
        }
    }

    // Heuristics run in ascending rule_id = REGISTRY declaration order;
    // candidates keep emission order for deterministic duplicate collapse.
    let mut findings: Vec<Finding> = Vec::new();
    for entry in REGISTRY {
        let slice: &[FileUnit] = match entry.language {
            RuleLanguage::C => &c_units,
            RuleLanguage::Python => &python_units,
            RuleLanguage::Rust => &rust_units,
        };
        if slice.is_empty() {
            continue;
        }
        findings.extend((entry.f)(slice));
    }

    // Downgrade/upgrade hooks and duplicate collapse keep the FULL unit set
    // and the full findings (they look up a finding's file by path, so the
    // language split must not narrow their view).
    apply_cfg_gating(units, &mut findings);
    apply_downgrade_hooks(units, &mut findings);
    apply_upgrade_hooks(units, &mut findings);
    collapse_duplicates(&mut findings);
    findings.sort_by(total_order);
    findings
}

/// Central precision SUPPRESSION GATE (precision FP-class fix 2): findings
/// anchored inside a provably-dead configuration region (C `#if 0` —
/// [`FunctionRecord::cfg_gated`](crate::ir::FunctionRecord::cfg_gated)) are
/// false positives — the code never compiles — and are REMOVED, not
/// downgraded. Conservative by construction: only `Some(true)` suppresses;
/// `None` (uncomputed, parser-free, or a live region) and `Some(false)` keep
/// every finding. Runs before the downgrade hooks so dead findings never
/// reach confidence capping or duplicate collapse.
fn apply_cfg_gating(units: &[FileUnit], findings: &mut Vec<Finding>) {
    findings.retain(|finding| {
        !units
            .iter()
            .find(|u| u.file.path == finding.path)
            .is_some_and(|unit| {
                unit.functions.iter().any(|f| {
                    f.record.cfg_gated == Some(true)
                        && span_contains_point(&f.record.span, &finding.span)
                })
            })
    });
}

/// Central semantic CONSISTENCY GATE (plan `semantic-resolver` Wave B1),
/// mirroring [`apply_downgrade_hooks`] placement (before dedup + sort), but
/// conservative in the upgrade direction: any finding that claims
/// `resolver_tier == Semantic` must be BACKED by a kind-matched semantic
/// fact at its source — a `CallEvent.semantic` fact on a call whose span
/// CONTAINS the finding span's start point, in the finding's file, where the
/// fact kind matches the finding's rule class:
/// - `PERF-C-ARCH-003` / `PERF-RS-ARCH-003` require
///   `resolved_definition.is_some()`;
/// - `PERF-C-ALLOC-002` requires `fopened_handle == true`;
/// - `PERF-C-STR-003` requires `!literal_args.is_empty()`.
///   Any other rule_id claiming Semantic is UNBACKED by construction
///   (default-deny: future rules emitting semantic findings must extend this
///   match). Unbacked findings are REVERTED to `Syntactic` with a confidence
///   floor of LOW.
///
/// The `FunctionRecord.resolver_tier` is deliberately NOT consulted here: it
/// reports the definition-side contract honestly (ir-v1) but never backs a
/// finding on its own. The downgrade hooks ([`apply_downgrade_hooks`]) and
/// the resolver's error-region gating keep the weaker `intersects()`
/// overlap check (conservative direction) — only this gate uses the
/// strictly stronger start-point containment.
///
/// Heuristics are expected to emit only backed findings; this single choke
/// point makes all future upgrade logic safe — a heuristic can never claim
/// semantic authority the IR stream does not carry (defensive, never a
/// behavior change for well-formed candidates).
fn apply_upgrade_hooks(units: &[FileUnit], findings: &mut [Finding]) {
    for finding in findings.iter_mut() {
        if finding.resolver_tier != ResolverTier::Semantic {
            continue;
        }
        let backed = units
            .iter()
            .find(|u| u.file.path == finding.path)
            .is_some_and(|unit| {
                unit.functions.iter().any(|f| {
                    f.calls.iter().any(|c| {
                        let Some(semantic) = c.semantic.as_ref() else {
                            return false;
                        };
                        let kind_matches = match finding.rule_id.as_str() {
                            ARCH_003_RULE_ID => semantic.resolved_definition.is_some(),
                            // Same cross-file-resolution contract mirrored for
                            // Rust (no Rust resolver exists yet, so no Rust
                            // edge can currently claim Semantic — defensively
                            // kind-matched rather than default-denied).
                            RS_ARCH_003_RULE_ID => semantic.resolved_definition.is_some(),
                            ALLOC_002_RULE_ID => semantic.fopened_handle,
                            STR_003_RULE_ID => !semantic.literal_args.is_empty(),
                            // Default-deny: no other rule may claim
                            // Semantic without extending this match.
                            _ => false,
                        };
                        kind_matches && span_contains_point(&c.meta.span, &finding.span)
                    })
                })
            });
        if !backed {
            finding.resolver_tier = ResolverTier::Syntactic;
            finding.confidence = Confidence::Low;
        }
    }
}

/// Start-point containment for the upgrade gate: the backing call's span
/// must CONTAIN the finding span's start point (strictly stronger than
/// [`crate::ir::Span::intersects`]: a call span merely touching the finding
/// span's edge does not back it).
fn span_contains_point(outer: &crate::ir::Span, inner: &crate::ir::Span) -> bool {
    (outer.start_line, outer.start_col) <= (inner.start_line, inner.start_col)
        && (inner.start_line, inner.start_col) <= (outer.end_line, outer.end_col)
}

/// Central downgrade hooks (ir-v1.md §5), applied uniformly:
/// 1. macro origin — finding span intersects a `recognized_pattern`
///    MacroEvent ⇒ cap MEDIUM + `macro_suspect=true`;
/// 2. ParseHealth gating — finding span overlaps an error region ⇒ cap
///    MEDIUM + `error_node_overlap=true` (never silently dropped).
fn apply_downgrade_hooks(units: &[FileUnit], findings: &mut [Finding]) {
    for finding in findings.iter_mut() {
        let Some(unit) = units.iter().find(|u| u.file.path == finding.path) else {
            continue;
        };
        let macro_hit = unit.macros.iter().any(|m| {
            m.kind == MacroEventKind::RecognizedPattern && m.span.intersects(&finding.span)
        }) || event_macro_origin(unit, &finding.span);
        if macro_hit {
            finding.macro_suspect = true;
            finding.confidence = Confidence::Medium;
        }
        if unit.parse_health.status != ParseStatus::Ok
            && unit
                .parse_health
                .error_regions
                .iter()
                .any(|region| region.intersects(&finding.span))
        {
            finding.error_node_overlap = true;
            finding.confidence = Confidence::Medium;
        }
    }
}

/// True when any event overlapping `span` carries `macro_origin=true`
/// (wrapper USE sites — the recognizer's definition-span intersection alone
/// cannot see them, since the pattern event anchors at the `#define`).
fn event_macro_origin(unit: &FileUnit, span: &crate::ir::Span) -> bool {
    unit.functions.iter().any(|f| {
        f.calls
            .iter()
            .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.assignments
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.binary_ops
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.comparisons
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
            || f.access_chains
                .iter()
                .any(|e| e.meta.macro_origin && e.meta.span.intersects(span))
    })
}

/// Uniqueness enforcement (modules.md §3.3): collapse identical
/// `(rule_id, path, start_line, start_col)` keeping the FIRST in execution
/// order (registry order + emission order within a heuristic).
fn collapse_duplicates(findings: &mut Vec<Finding>) {
    let mut seen: BTreeMap<(String, String, u32, u32), usize> = BTreeMap::new();
    let mut kept = Vec::with_capacity(findings.len());
    for finding in findings.drain(..) {
        let key = (
            finding.rule_id.clone(),
            finding.path.clone(),
            finding.span.start_line,
            finding.span.start_col,
        );
        match seen.get(&key) {
            Some(_) => {} // duplicate: first-in-execution-order wins
            None => {
                seen.insert(key, kept.len());
                kept.push(finding);
            }
        }
    }
    *findings = kept;
}

/// Report §4 total order comparator (output-contract §4): severity
/// hot→warn→info and confidence HIGH→MEDIUM→LOW are the ASCENDING enum
/// orders ([`Severity`]/[`Confidence`] declare variants in report order),
/// so every key compares ascending.
fn total_order(a: &Finding, b: &Finding) -> std::cmp::Ordering {
    a.severity
        .cmp(&b.severity)
        .then(a.confidence.cmp(&b.confidence))
        .then(a.rule_id.cmp(&b.rule_id))
        .then(a.path.cmp(&b.path))
        .then(a.span.start_line.cmp(&b.span.start_line))
        .then(a.span.start_col.cmp(&b.span.start_col))
}

#[cfg(test)]
mod tests {
    //! Parser-free gate proofs (plan `semantic-resolver` Wave B2):
    //! [`apply_upgrade_hooks`] reverts `Semantic` findings without backing
    //! facts and keeps backed ones — the choke point every heuristic's
    //! semantic claims pass through.

    use super::*;
    use crate::ir::{
        CallEvent, CalleeKind, EventMeta, FileRecord, FunctionRecord, LocCounts, Span,
        StatementRole,
    };
    use crate::scanner::{FileUnit, FunctionUnit};

    fn gate_span() -> Span {
        Span {
            start_line: 10,
            start_col: 5,
            end_line: 10,
            end_col: 20,
        }
    }

    fn gate_call_at(span: Span, semantic: Option<crate::ir::CallSemantic>) -> CallEvent {
        CallEvent {
            meta: EventMeta {
                span,
                in_loop_depth: 1,
                is_innermost: true,
                resolver_tier: ResolverTier::Syntactic,
                macro_origin: false,
            },
            callee_name: "fclose".to_string(),
            callee_kind: CalleeKind::Direct,
            callee_path: None,
            is_macro: false,
            args: Vec::new(),
            statement_role: StatementRole::Nested,
            conditional_guarded: false,
            semantic,
            receiver_root: None,
        }
    }

    fn gate_unit(semantic: Option<crate::ir::CallSemantic>) -> FileUnit {
        gate_unit_with(semantic, gate_span())
    }

    fn gate_unit_with(semantic: Option<crate::ir::CallSemantic>, call_span: Span) -> FileUnit {
        let record_span = Span {
            start_line: 1,
            start_col: 1,
            end_line: 30,
            end_col: 1,
        };
        FileUnit {
            file: FileRecord {
                path: "src/a.c".to_string(),
                language: "c".to_string(),
                loc: LocCounts::default(),
                function_count: 1,
                global_var_count: 0,
                include_counts: Default::default(),
            },
            parse_health: Default::default(),
            includes: Vec::new(),
            global_vars: Vec::new(),
            structs: Vec::new(),
            macros: Vec::new(),
            functions: vec![FunctionUnit {
                record: FunctionRecord {
                    id: "src/a.c#drain#1:1".to_string(),
                    name: "drain".to_string(),
                    path: "src/a.c".to_string(),
                    span: record_span,
                    ..FunctionRecord::default()
                },
                calls: vec![gate_call_at(call_span, semantic)],
                assignments: Vec::new(),
                binary_ops: Vec::new(),
                comparisons: Vec::new(),
                access_chains: Vec::new(),
                loops: Vec::new(),
                awaits: Vec::new(),
                try_blocks: Vec::new(),
                allocations: Vec::new(),
                unaries: Vec::new(),
            }],
        }
    }

    fn semantic_finding() -> Finding {
        finding_for_rule("PERF-C-ALLOC-002")
    }

    fn finding_for_rule(rule_id: &str) -> Finding {
        Finding {
            threshold: None,
            rule_id: rule_id.to_string(),
            title: "probe".to_string(),
            message: "probe".to_string(),
            severity: Severity::Warn,
            confidence: Confidence::High,
            resolver_tier: ResolverTier::Semantic,
            span: gate_span(),
            path: "src/a.c".to_string(),
            evidence: Default::default(),
            macro_suspect: false,
            error_node_overlap: false,
        }
    }

    #[test]
    fn unbacked_semantic_finding_reverts_to_syntactic_low() {
        let units = vec![gate_unit(None)];
        let mut findings = vec![semantic_finding()];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::Low);
    }

    #[test]
    fn call_semantic_fact_backs_the_finding_at_its_span() {
        // KIND-MATCHED over fact kinds: a `fopened_handle` fact backs an
        // ALLOC-002 finding…
        let units = vec![gate_unit(Some(crate::ir::CallSemantic {
            resolved_definition: None,
            fopened_handle: true,
            literal_args: Vec::new(),
        }))];
        let mut findings = vec![semantic_finding()];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Semantic);
        assert_eq!(findings[0].confidence, Confidence::High);
    }

    #[test]
    fn resolved_definition_backs_arch003() {
        // …a `resolved_definition` fact backs an ARCH-003 finding…
        let units = vec![gate_unit(Some(crate::ir::CallSemantic {
            resolved_definition: Some(crate::ir::ResolvedSite {
                path: "src/b.c".to_string(),
                span: gate_span(),
            }),
            fopened_handle: false,
            literal_args: Vec::new(),
        }))];
        let mut findings = vec![finding_for_rule("PERF-C-ARCH-003")];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Semantic);
        assert_eq!(findings[0].confidence, Confidence::High);
    }

    #[test]
    fn literal_args_fact_backs_the_finding_at_its_span() {
        // …and a `literal_args` fact backs a STR-003 finding the same way.
        let units = vec![gate_unit(Some(crate::ir::CallSemantic {
            resolved_definition: None,
            fopened_handle: false,
            literal_args: vec![(0, "%s".to_string())],
        }))];
        let mut findings = vec![{
            let mut f = semantic_finding();
            f.rule_id = "PERF-C-STR-003".to_string();
            f
        }];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Semantic);
        assert_eq!(findings[0].confidence, Confidence::High);
    }

    #[test]
    fn semantic_record_tier_alone_never_backs() {
        // The definition-side record tier is honest contract reporting, not
        // gate backing: a Semantic ARCH-003 finding in a Semantic-record
        // function WITHOUT any call fact still reverts.
        let mut units = vec![gate_unit(None)];
        units[0].functions[0].record.resolver_tier = ResolverTier::Semantic;
        let mut findings = vec![finding_for_rule("PERF-C-ARCH-003")];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::Low);
    }

    #[test]
    fn wrong_kind_fact_does_not_back() {
        // A fact of the wrong kind is unbacked even at the exact span: an
        // ALLOC-002 finding backed only by `literal_args`…
        let units = vec![gate_unit(Some(crate::ir::CallSemantic {
            resolved_definition: None,
            fopened_handle: false,
            literal_args: vec![(0, "%s".to_string())],
        }))];
        let mut findings = vec![semantic_finding()];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::Low);

        // …and a STR-003 finding backed only by `fopened_handle`.
        let units = vec![gate_unit(Some(crate::ir::CallSemantic {
            resolved_definition: None,
            fopened_handle: true,
            literal_args: Vec::new(),
        }))];
        let mut findings = vec![finding_for_rule("PERF-C-STR-003")];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::Low);
    }

    #[test]
    fn unknown_rule_id_is_default_deny() {
        // No other rule may claim Semantic without extending the gate's
        // match — even a kind-plausible fact does not back it.
        let units = vec![gate_unit(Some(crate::ir::CallSemantic {
            resolved_definition: None,
            fopened_handle: true,
            literal_args: Vec::new(),
        }))];
        let mut findings = vec![finding_for_rule("PERF-C-LOOP-001")];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::Low);
    }

    #[test]
    fn merely_touching_fact_span_does_not_back() {
        // The backing call's span must CONTAIN the finding start: a call at
        // 10:20–10:30 touches the finding span (10:5–10:20) at its end edge —
        // `intersects()` is true — but does not contain the start point, so
        // the finding reverts.
        let touching = Span {
            start_line: 10,
            start_col: 20,
            end_line: 10,
            end_col: 30,
        };
        assert!(
            touching.intersects(&gate_span()),
            "test setup: the spans must still intersect (the old check)"
        );
        let units = vec![gate_unit_with(
            Some(crate::ir::CallSemantic {
                resolved_definition: None,
                fopened_handle: true,
                literal_args: Vec::new(),
            }),
            touching,
        )];
        let mut findings = vec![semantic_finding()];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::Low);
    }

    #[test]
    fn syntactic_findings_pass_through_untouched() {
        let units = vec![gate_unit(None)];
        let mut findings = vec![{
            let mut f = semantic_finding();
            f.resolver_tier = ResolverTier::Syntactic;
            f.confidence = Confidence::High;
            f
        }];
        apply_upgrade_hooks(&units, &mut findings);
        assert_eq!(findings[0].resolver_tier, ResolverTier::Syntactic);
        assert_eq!(findings[0].confidence, Confidence::High);
    }

    /// Precision FP-class fix 2 consumer locks: [`apply_cfg_gating`] removes
    /// findings anchored inside a `cfg_gated` (`#if 0`) function but keeps
    /// findings in live functions (`None`), explicitly-live functions
    /// (`Some(false)`), and findings outside any function span.
    #[test]
    fn cfg_gated_dead_findings_removed_live_kept() {
        fn func(name: &str, start: u32, end: u32, gated: Option<bool>) -> FunctionUnit {
            FunctionUnit {
                record: FunctionRecord {
                    id: format!("src/a.c#{name}#{start}:1"),
                    name: name.to_string(),
                    path: "src/a.c".to_string(),
                    span: Span {
                        start_line: start,
                        start_col: 1,
                        end_line: end,
                        end_col: 1,
                    },
                    cfg_gated: gated,
                    ..FunctionRecord::default()
                },
                ..FunctionUnit::default()
            }
        }
        fn finding_at(line: u32) -> Finding {
            let mut f = finding_for_rule("PERF-C-ALLOC-001");
            f.span = Span {
                start_line: line,
                start_col: 5,
                end_line: line,
                end_col: 20,
            };
            f
        }
        let units = vec![FileUnit {
            file: FileRecord {
                path: "src/a.c".to_string(),
                language: "c".to_string(),
                ..FileRecord::default()
            },
            functions: vec![
                func("dead", 1, 15, Some(true)),
                func("live", 20, 40, None),
                func("checked", 45, 60, Some(false)),
            ],
            ..FileUnit::default()
        }];
        let mut findings = vec![
            finding_at(10),
            finding_at(30),
            finding_at(50),
            finding_at(70),
        ];
        apply_cfg_gating(&units, &mut findings);
        let lines: Vec<u32> = findings.iter().map(|f| f.span.start_line).collect();
        assert_eq!(lines, [30, 50, 70]);
    }
}
