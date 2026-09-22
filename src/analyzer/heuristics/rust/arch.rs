//! Rust architecture-family heuristics (plan `rust-backend` R3 workstream
//! C1): PERF-RS-ARCH-001/002/003, mirroring PERF-C-ARCH-001/002/003 over the
//! Rust module/`use` graph and Rust call graph.
//!
//! Purity (modules.md §4.1): pure IR-view functions — no I/O, no clock, no
//! global state, no sibling-order dependence. ParseHealth is consulted
//! CENTRALLY (`analyzer::apply_downgrade_hooks`) exactly as every other
//! family does; no per-rule gating lives here. Messages use the
//! "candidate optimization" framing throughout.
//!
//! The C backend's include graph maps onto Rust's `use` IMPORT graph:
//! `PERF-RS-ARCH-001` operates on resolved internal `use` edges only
//! (`resolved && to_path.is_some() && !is_mod`). `mod` declaration edges are
//! still emitted in `includes` (they drive module resolution and
//! `RustModuleMap`), but they are excluded from this rule — a `mod child;`
//! declaration plus the child's `use super::…` is ordinary parent/child
//! structure, not a cycle. Module structure lives in `include_graph` data,
//! not in this rule. External crate paths stay `Angle`; unresolved internal
//! paths normalize to `to_path=None`. The `token_tree` blind spot means
//! macro-generated modules/items are invisible (rust-backend.md §5).

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::finding::{RS_ARCH_001_RULE_ID, RS_ARCH_002_RULE_ID, RS_ARCH_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::graph::break_cycles;
use crate::ir::{ResolverTier, Span};
use crate::scanner::FileUnit;

use super::super::{
    ARCH_001_HUB_FANOUT, ARCH_001_MAX_INCLUDE_DEPTH, ARCH_002_MIN_COMPLEXITY_SUM,
    cross_file_call_edges,
};

// ===========================================================================
// PERF-RS-ARCH-001 — module-graph fan-out / cycles
// ===========================================================================

/// One resolved INTERNAL module edge (deduped to its earliest-span instance):
/// `(from_path, to_path)` plus the retained edge's span.
struct InternalEdge {
    from: String,
    to: String,
    /// Span of the RETAINED (earliest) directive for this `(from, to)` pair.
    span: Span,
}

/// Collect resolved internal IMPORT (`use`) edges across ALL Rust units
/// (whole-program view), deduplicating identical `(from, to)` pairs and
/// keeping the EARLIEST-span instance — callers anchor findings there.
///
/// `mod` declaration edges (`IncludeEdge::is_mod`) are EXCLUDED: a
/// `mod child;` declaration yields parent→child while the child's ordinary
/// `use super::…` yields child→parent, so counting declarations would report
/// every parent/child module pair as a cycle (N1). ARCH-001 is the `use`
/// import graph, mirroring C's `#include`-only graph; module structure
/// remains available in `include_graph` data.
fn internal_edges(units: &[FileUnit]) -> Vec<InternalEdge> {
    let mut earliest: BTreeMap<(String, String), Span> = BTreeMap::new();
    for unit in units {
        for edge in &unit.includes {
            if !edge.resolved || edge.is_mod {
                continue;
            }
            let Some(to) = &edge.to_path else {
                continue;
            };
            let key = (edge.from_path.clone(), to.clone());
            match earliest.get(&key) {
                Some(span) if *span <= edge.span => {}
                _ => {
                    earliest.insert(key, edge.span);
                }
            }
        }
    }
    earliest
        .into_iter()
        .map(|((from, to), span)| InternalEdge { from, to, span })
        .collect()
}

/// PERF-RS-ARCH-001 — module-graph fan-out/cycles (mirrors PERF-C-ARCH-001).
///
/// Over INTERNAL `use` IMPORT edges only (`resolved && to_path.is_some()` and
/// `!is_mod`), across all Rust units. `mod` declaration edges are excluded:
/// module structure lives in `include_graph` data, and counting declarations
/// alongside the child's `use super::…` made every parent/child module pair
/// look like a cycle (N1). Sub-cases:
/// 1. cycles — one finding per [`break_cycles`] broken edge (Warn/HIGH),
///    anchored at that pair's earliest retained directive span;
/// 2. deep chains — transitive module depth over the kept DAG; each
///    qualifying file (depth ≥ [`ARCH_001_MAX_INCLUDE_DEPTH`]) emits EXACTLY
///    ONE finding (Warn/MEDIUM) anchored at the edge targeting its
///    max-depth successor;
/// 3. hubs — fan-out/fan-in at or beyond [`ARCH_001_HUB_FANOUT`]
///    (Warn/MEDIUM), anchored at the file's first non-colliding directive
///    (advancing past spans already used by this run) or the synthetic
///    line-1/col-1 span for a pure importer.
///
/// The C rule's fourth sub-case (unresolved internal `mod`/`use` edges →
/// Info/MEDIUM `kind=unresolved_include`) is **intentionally NOT mirrored**.
/// The longest-file-backed-prefix resolver cannot see items defined at the
/// crate root (`use crate::Config;`), items in inline modules
/// (`mod helper { … }` + `use crate::helper::run;`), or modules behind the
/// raw-identifier class (C1) — so ordinary, rustc-resolvable Rust produced a
/// stream of false "external or build-generated dependency" findings. Rust
/// unresolved edges remain in `include_graph` as data (`to:null`,
/// `resolved=false`); they no longer produce findings.
///
/// Accepted FP classes: deep/hub thresholds are proxy signals for rebuild and
/// coupling cost, not runtime; macro-generated modules are invisible.
pub(crate) fn rust_arch_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let edges = internal_edges(units);

    // --- 1. Cycles ---------------------------------------------------------
    let pairs: Vec<(String, String)> = edges
        .iter()
        .map(|e| (e.from.clone(), e.to.clone()))
        .collect();
    let spans: BTreeMap<(&str, &str), Span> = edges
        .iter()
        .map(|e| ((e.from.as_str(), e.to.as_str()), e.span))
        .collect();
    let result = break_cycles(&pairs);
    // Anchor-collision set: (path, line, col) of every RS-ARCH-001 candidate
    // already emitted in THIS run. The registry's duplicate collapse keys on
    // `(rule_id, path, start_line, start_col)` — sub-kind-blind — so a later
    // candidate anchoring on an already-used span would be silently dropped.
    let mut emitted_anchors: BTreeSet<(String, u32, u32)> = BTreeSet::new();
    for (from, to) in &result.broken {
        let span = spans
            .get(&(from.as_str(), to.as_str()))
            .copied()
            .unwrap_or_default();
        findings.push(Finding {
            threshold: None,
            rule_id: RS_ARCH_001_RULE_ID.to_string(),
            title: "Module-graph cycle (Rust)".to_string(),
            message: format!(
                "'{from}' and '{to}' import each other, forming a module cycle \
                 — breaking the lowest-sorted edge (here '{from}' → '{to}') \
                 into a shared leaf module is a candidate optimization",
            ),
            severity: Severity::Warn,
            confidence: Confidence::High,
            resolver_tier: ResolverTier::Syntactic,
            span,
            path: from.clone(),
            evidence: [
                ("kind".to_string(), "cycle".to_string()),
                ("coupled_with".to_string(), to.clone()),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
        emitted_anchors.insert((from.clone(), span.start_line, span.start_col));
    }

    // --- 2. Deep chains ----------------------------------------------------
    let mut succs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to) in &result.kept {
        succs.entry(from.as_str()).or_default().push(to.as_str());
    }
    fn depth_of<'a>(
        node: &'a str,
        succs: &BTreeMap<&'a str, Vec<&'a str>>,
        memo: &mut BTreeMap<&'a str, u32>,
    ) -> u32 {
        if let Some(d) = memo.get(node) {
            return *d;
        }
        let d = succs.get(node).map_or(0, |ss| {
            ss.iter()
                .map(|s| depth_of(s, succs, memo))
                .max()
                .unwrap_or(0)
                + 1
        });
        memo.insert(node, d);
        d
    }
    let mut memo: BTreeMap<&str, u32> = BTreeMap::new();
    let qualifying: BTreeSet<&str> = edges
        .iter()
        .map(|e| e.from.as_str())
        .filter(|p| depth_of(p, &succs, &mut memo) >= ARCH_001_MAX_INCLUDE_DEPTH)
        .collect();
    for &path in qualifying.iter() {
        let depth = depth_of(path, &succs, &mut memo);
        let ss = succs
            .get(path)
            .expect("qualifying file has kept successors");
        let anchor_target = ss
            .iter()
            .max_by_key(|s| (depth_of(s, &succs, &mut memo), std::cmp::Reverse(**s)))
            .expect("non-empty successor list");
        let span = spans
            .get(&(path, *anchor_target))
            .copied()
            .unwrap_or_default();
        findings.push(Finding {
            threshold: None,
            rule_id: RS_ARCH_001_RULE_ID.to_string(),
            title: "Deep module chain (Rust)".to_string(),
            message: format!(
                "'{}' sits {} module imports deep — every transitive reopen \
                 pays parse cost and couples the build; flattening the chain \
                 toward the leaves is a candidate optimization",
                path, depth
            ),
            severity: Severity::Warn,
            confidence: Confidence::Medium,
            resolver_tier: ResolverTier::Syntactic,
            span,
            path: path.to_string(),
            evidence: [
                ("kind".to_string(), "deep_chain".to_string()),
                ("include_depth".to_string(), depth.to_string()),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
        emitted_anchors.insert((path.to_string(), span.start_line, span.start_col));
    }

    // --- 3. Hubs -----------------------------------------------------------
    let mut fan_out: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut fan_in: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in &edges {
        fan_out
            .entry(edge.from.as_str())
            .or_default()
            .insert(&edge.to);
        fan_in
            .entry(edge.to.as_str())
            .or_default()
            .insert(&edge.from);
    }
    let hub_paths: BTreeSet<&str> = fan_out
        .keys()
        .chain(fan_in.keys())
        .filter(|p| {
            fan_out.get(*p).map_or(0, |s| s.len()) >= ARCH_001_HUB_FANOUT
                || fan_in.get(*p).map_or(0, |s| s.len()) >= ARCH_001_HUB_FANOUT
        })
        .copied()
        .collect();
    for path in hub_paths {
        let out = fan_out.get(path).map_or(0, |s| s.len());
        let inc = fan_in.get(path).map_or(0, |s| s.len());
        let span = if out >= ARCH_001_HUB_FANOUT {
            let mut directives: Vec<Span> = units
                .iter()
                .find(|u| u.file.path == path)
                .map(|u| u.includes.iter().map(|e| e.span).collect())
                .unwrap_or_default();
            directives.sort();
            directives.dedup();
            let key = |s: &Span| (path.to_string(), s.start_line, s.start_col);
            match directives
                .iter()
                .find(|s| !emitted_anchors.contains(&key(s)))
            {
                Some(s) => *s,
                None => directives
                    .first()
                    .copied()
                    .unwrap_or_else(|| Span::point(1, 1)),
            }
        } else {
            Span::point(1, 1)
        };
        findings.push(Finding {
            threshold: None,
            rule_id: RS_ARCH_001_RULE_ID.to_string(),
            title: "Module hub (Rust)".to_string(),
            message: format!(
                "'{path}' couples {out} downstream module(s) to {inc} \
                 upstream importer(s) — splitting the hub into focused \
                 modules is a candidate optimization"
            ),
            severity: Severity::Warn,
            confidence: Confidence::Medium,
            resolver_tier: ResolverTier::Syntactic,
            span,
            path: path.to_string(),
            evidence: [
                ("kind".to_string(), "hub".to_string()),
                ("fan_out".to_string(), out.to_string()),
                ("fan_in".to_string(), inc.to_string()),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
        emitted_anchors.insert((path.to_string(), span.start_line, span.start_col));
    }

    findings
}

// ===========================================================================
// PERF-RS-ARCH-002 — hot-component aggregation
// ===========================================================================

/// Per-directory aggregate feeding the RS-ARCH-002 ranking.
struct ComponentStat {
    path_prefix: String,
    complexity_sum: u64,
    max_loop_depth: u32,
    alloc_site_count: u64,
    kloc: f64,
    function_count: u32,
}

impl ComponentStat {
    fn alloc_site_density(&self) -> f64 {
        if self.kloc == 0.0 {
            0.0
        } else {
            self.alloc_site_count as f64 / self.kloc
        }
    }
}

/// Parent directory of a repo-relative POSIX path; root-level files map to
/// `"."`.
fn parent_dir(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => ".".to_string(),
    }
}

/// PERF-RS-ARCH-002 — hot-component aggregation (mirrors PERF-C-ARCH-002).
///
/// Identical IR-only composite over Rust files: `complexity_sum ↓ →
/// max_loop_depth ↓ → alloc_site_density ↓ → path_prefix ↑`, rank-1 only,
/// floor [`ARCH_002_MIN_COMPLEXITY_SUM`] (15) keeps trivial scans clean.
/// Anchored at the highest-complexity function. Evidence keys mirror the C
/// rule (`path_prefix`, `complexity_sum`, `max_loop_depth`,
/// `alloc_site_density`, `function_count`).
///
/// Accepted limitations: `alloc_site_density` reflects the R3 Rust alloc
/// family (qualified/method/macro sites); `finding_density` is deliberately
/// excluded (heuristic purity).
pub(crate) fn rust_arch_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut stats: BTreeMap<String, ComponentStat> = BTreeMap::new();
    let mut anchors: BTreeMap<String, (u32, u32, u32, &crate::ir::FunctionRecord)> =
        BTreeMap::new();

    for unit in units {
        let dir = parent_dir(&unit.file.path);
        let entry = stats.entry(dir.clone()).or_insert_with(|| ComponentStat {
            path_prefix: dir,
            complexity_sum: 0,
            max_loop_depth: 0,
            alloc_site_count: 0,
            kloc: 0.0,
            function_count: 0,
        });
        entry.kloc += f64::from(unit.file.loc.code) / 1000.0;

        for func in &unit.functions {
            let dir = parent_dir(&func.record.path);
            let entry = stats.entry(dir.clone()).or_insert_with(|| ComponentStat {
                path_prefix: dir.clone(),
                complexity_sum: 0,
                max_loop_depth: 0,
                alloc_site_count: 0,
                kloc: 0.0,
                function_count: 0,
            });
            entry.complexity_sum += u64::from(func.record.cyclomatic_complexity);
            entry.max_loop_depth = entry.max_loop_depth.max(func.record.max_loop_depth);
            entry.alloc_site_count += u64::from(func.record.alloc_site_count);
            entry.function_count += 1;

            let anchor_key = (
                u32::MAX - func.record.cyclomatic_complexity,
                func.record.span.start_line,
                func.record.span.start_col,
            );
            let slot = anchors
                .entry(dir)
                .or_insert((u32::MAX, u32::MAX, u32::MAX, &func.record));
            if anchor_key < (slot.0, slot.1, slot.2) {
                *slot = (anchor_key.0, anchor_key.1, anchor_key.2, &func.record);
            }
        }
    }

    let mut ranked: Vec<&ComponentStat> = stats.values().collect();
    ranked.sort_by(|a, b| {
        b.complexity_sum
            .cmp(&a.complexity_sum)
            .then(b.max_loop_depth.cmp(&a.max_loop_depth))
            .then(b.alloc_site_density().total_cmp(&a.alloc_site_density()))
            .then(a.path_prefix.cmp(&b.path_prefix))
    });

    let Some(best) = ranked.first() else {
        return Vec::new();
    };
    if best.complexity_sum < u64::from(ARCH_002_MIN_COMPLEXITY_SUM) {
        return Vec::new();
    }
    // Rank-1 only. `path_prefix` is part of the equality test and directory
    // keys are unique, so this `take_while` can match AT MOST one component —
    // there is no multi-way tie to co-fire. (The C rank-1 note lives in the
    // C catalog; this iterator is deliberately a singleton.)
    let tied = ranked.iter().take_while(|c| {
        c.complexity_sum == best.complexity_sum
            && c.max_loop_depth == best.max_loop_depth
            && c.alloc_site_density().total_cmp(&best.alloc_site_density())
                == std::cmp::Ordering::Equal
            && c.path_prefix == best.path_prefix
    });

    let mut findings = Vec::new();
    for component in tied {
        let (_, _, _, anchor) = anchors
            .get(&component.path_prefix)
            .expect("component with functions has an anchor");
        findings.push(Finding {
            threshold: None,
            rule_id: RS_ARCH_002_RULE_ID.to_string(),
            title: "Hot component (Rust)".to_string(),
            message: format!(
                "directory '{}' aggregates complexity {} across {} \
                 function(s) — the hottest component in this scan; \
                 splitting or simplifying its hottest functions is a \
                 candidate optimization",
                component.path_prefix, component.complexity_sum, component.function_count
            ),
            severity: Severity::Hot,
            confidence: Confidence::High,
            resolver_tier: ResolverTier::Syntactic,
            span: anchor.span,
            path: anchor.path.clone(),
            evidence: [
                ("path_prefix".to_string(), component.path_prefix.clone()),
                (
                    "complexity_sum".to_string(),
                    component.complexity_sum.to_string(),
                ),
                (
                    "max_loop_depth".to_string(),
                    component.max_loop_depth.to_string(),
                ),
                (
                    "alloc_site_density".to_string(),
                    component.alloc_site_density().to_string(),
                ),
                (
                    "function_count".to_string(),
                    component.function_count.to_string(),
                ),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
    }
    findings
}

// ===========================================================================
// PERF-RS-ARCH-003 — cross-module call coupling
// ===========================================================================

/// PERF-RS-ARCH-003 — cross-file/module call coupling (mirrors
/// PERF-C-ARCH-003).
///
/// ONE Info finding per language-scoped [`cross_file_call_edges`] edge:
/// `cross_file_call_edges` is keyed by `(language, name)`, so a Rust unit
/// slice can never emit a C↔Rust cross-language edge. RESOLVED edges
/// (semantic fact) carry confidence MEDIUM and `resolver_tier: semantic`
/// with evidence `callee_name`, `matched_path`, `resolved`; the name-based
/// fallback keeps LOW/Syntactic with `callee_name`, `matched_path`.
///
/// Rust is syntactic-tier only in this milestone (`rust-backend.md §6`), so
/// in practice every Rust edge is the name-based fallback; the resolved
/// branch is kept for symmetry and remains covered by the central upgrade
/// gate (an unbacked Semantic claim is reverted there).
pub(crate) fn rust_arch_003(units: &[FileUnit]) -> Vec<Finding> {
    cross_file_call_edges(units)
        .into_iter()
        .map(|edge| {
            let (message, confidence, tier, evidence) = if edge.resolved {
                (
                    format!(
                        "call to '{}' resolves to its definition in '{}' via \
                         the semantic resolver — cross-file coupling confirmed \
                         at MEDIUM confidence; co-locating hot callees remains \
                         a candidate optimization",
                        edge.callee_name, edge.to_path
                    ),
                    edge.confidence,
                    ResolverTier::Semantic,
                    vec![
                        ("callee_name".to_string(), edge.callee_name.clone()),
                        ("matched_path".to_string(), edge.to_path.clone()),
                        ("resolved".to_string(), "true".to_string()),
                    ],
                )
            } else {
                (
                    format!(
                        "call to '{}' matches a definition in '{}' by name only \
                         — LOW-confidence hypothesis; confirming the match (or \
                         co-locating hot callees) is a candidate optimization",
                        edge.callee_name, edge.to_path
                    ),
                    edge.confidence,
                    ResolverTier::Syntactic,
                    vec![
                        ("callee_name".to_string(), edge.callee_name.clone()),
                        ("matched_path".to_string(), edge.to_path.clone()),
                    ],
                )
            };
            Finding {
                threshold: None,
                rule_id: RS_ARCH_003_RULE_ID.to_string(),
                title: "Cross-module call coupling (Rust)".to_string(),
                message,
                severity: Severity::Info,
                confidence,
                resolver_tier: tier,
                span: edge.call_span,
                path: edge.from_path,
                evidence: evidence.into_iter().collect(),
                macro_suspect: false,
                error_node_overlap: false,
            }
        })
        .collect()
}
