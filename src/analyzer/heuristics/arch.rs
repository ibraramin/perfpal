//! Architecture-family heuristics (catalog-v1 §3 Tier 1):
//! PERF-C-ARCH-001/002/003.
//!
//! Purity (modules.md §4.1): pure IR-view functions — no I/O, no clock, no
//! global state, no sibling-order dependence. ParseHealth is consulted
//! CENTRALLY (`analyzer::apply_downgrade_hooks`) exactly as every other
//! family does; no per-rule gating lives here. Messages use the
//! "candidate optimization" framing throughout.

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::finding::{ARCH_001_RULE_ID, ARCH_002_RULE_ID, ARCH_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::graph::break_cycles;
use crate::ir::{CalleeKind, IncludeForm, ResolverTier, Span};
use crate::scanner::FileUnit;

use super::{ARCH_001_HUB_FANOUT, ARCH_001_MAX_INCLUDE_DEPTH, ARCH_002_MIN_COMPLEXITY_SUM};

// ===========================================================================
// PERF-C-ARCH-001 — include-graph fan-out / cycles
// ===========================================================================

/// One resolved INTERNAL include edge (deduped to its earliest-span
/// instance): `(from_path, to_path)` plus the retained edge's span.
struct InternalEdge {
    from: String,
    to: String,
    /// Span of the RETAINED (earliest) directive for this `(from, to)` pair;
    /// duplicate pairs collapse onto it (see [`break_cycles`] contract).
    span: Span,
}

/// Collect resolved internal include edges across ALL units (whole-program
/// view), deduplicating identical `(from, to)` pairs and keeping the
/// EARLIEST-span instance — callers anchor findings there.
fn internal_edges(units: &[FileUnit]) -> Vec<InternalEdge> {
    // (from, to) → earliest span seen so far.
    let mut earliest: BTreeMap<(String, String), Span> = BTreeMap::new();
    for unit in units {
        for edge in &unit.includes {
            if !edge.resolved {
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

/// PERF-C-ARCH-001 — include-graph fan-out/cycles (catalog-v1 §3 Tier 1).
///
/// Over INTERNAL edges only (`resolved && to_path.is_some()`), across all
/// units:
/// 1. cycles — one finding per [`break_cycles`] broken edge, anchored at
///    that pair's earliest retained directive span;
/// 2. deep chains — transitive include depth over the kept DAG; EACH
///    qualifying file (depth ≥ [`ARCH_001_MAX_INCLUDE_DEPTH`]) emits EXACTLY
///    ONE finding — the contract is once-per-file, not once-per-out-edge —
///    anchored deterministically at THE include edge targeting its
///    max-depth successor (max successor depth, tie smallest `to_path`);
/// 3. hubs — fan-out (distinct resolved targets) or fan-in (distinct
///    includers) at or beyond [`ARCH_001_HUB_FANOUT`]; anchored at the
///    file's FIRST include directive when fan-out triggered — ADVANCED to
///    the next directive in ascending span order when that first-directive
///    span already anchors an earlier ARCH-001 candidate in this run (the
///    duplicate-collapse key `(rule_id, path, line, col)` is sub-kind-blind,
///    so an unadvanced collision would silently drop the hub) — else at the
///    synthetic `Span{line 1, col 1}` (a pure includer has no natural
///    out-edge to anchor on);
/// 4. unresolved quoted includes — data, not error (modules.md Principle 4):
///    one Info finding per edge. Raw target text is unavailable (the
///    resolver nulls `to_path`) — accepted limitation, documented in the
///    catalog.
pub(crate) fn arch_001(units: &[FileUnit]) -> Vec<Finding> {
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
    // Anchor-collision set: (path, line, col) of every ARCH-001 candidate
    // already emitted in THIS heuristic run. The registry's duplicate
    // collapse keys on `(rule_id, path, start_line, start_col)` — it cannot
    // see sub-kinds — so a later cycle/deep-chain/hub candidate anchoring on
    // an already-used span would be SILENTLY DISCARDED. Every emitter below
    // registers its anchor here; hubs consult it to advance theirs.
    let mut emitted_anchors: BTreeSet<(String, u32, u32)> = BTreeSet::new();
    for (from, to) in &result.broken {
        let span = spans
            .get(&(from.as_str(), to.as_str()))
            .copied()
            .unwrap_or_default();
        findings.push(Finding {
            threshold: None,
            rule_id: ARCH_001_RULE_ID.to_string(),
            title: "Include-graph cycle".to_string(),
            message: format!(
                "'{from}' and '{to}' include each other, forming a cycle — \
                 breaking the lowest-sorted edge (here '{from}' → '{to}') \
                 into a shared leaf header is a candidate optimization",
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
    // Transitive include depth over the KEPT DAG: depth(f) = 0 for a leaf,
    // else 1 + max(depth(successors)). Memoized DFS (the kept set is
    // acyclic by construction).
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
    // ONCE-PER-FILE contract: qualify FILES (ascending path via BTreeSet),
    // not out-edges. The historical per-edge loop emitted one identical
    // candidate per out-edge (all anchoring at the same max-depth-successor
    // span) and only looked once-per-file because duplicate collapse happened
    // to merge them; the anchor selection below is now explicit and
    // deterministic instead of emergent.
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
        // THE include edge targeting the max-depth successor (max successor
        // depth, tie smallest to_path): one deterministic anchor per file.
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
            rule_id: ARCH_001_RULE_ID.to_string(),
            title: "Deep include chain".to_string(),
            message: format!(
                "'{}' sits {} includes deep — every transitive reopen pays \
                 parse and preprocessing cost; flattening the chain toward \
                 the leaves is a candidate optimization",
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
    // fan_out = distinct resolved targets, fan_in = distinct includers.
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
        // Fan-out trigger ⇒ anchor at an include directive of the file.
        // If the FIRST directive span already anchors an earlier ARCH-001
        // candidate in this run (e.g. a deep-chain anchor on the same span),
        // ADVANCE to the next directive in ascending span order — the
        // duplicate-collapse key is sub-kind-blind and would silently drop
        // the hub otherwise. Fan-in-only ⇒ synthetic line-1/col-1 span
        // (documented catalog behavior; a pure includer has no directive
        // worth anchoring on).
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
                // Every directive collides (unreachable while a fan-out hub
                // has ≥ ARCH_001_HUB_FANOUT distinct directive spans) — keep
                // the earliest rather than dropping the finding silently.
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
            rule_id: ARCH_001_RULE_ID.to_string(),
            title: "Include hub".to_string(),
            message: format!(
                "'{path}' couples {out} downstream target(s) to {inc} \
                 upstream includer(s) — splitting the hub into focused \
                 headers is a candidate optimization"
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

    // --- 4. Unresolved quoted includes -------------------------------------
    for unit in units {
        for edge in &unit.includes {
            if edge.form != IncludeForm::Quoted || edge.resolved {
                continue;
            }
            findings.push(Finding {
                threshold: None,
                rule_id: ARCH_001_RULE_ID.to_string(),
                title: "Unresolved quoted include".to_string(),
                message: format!(
                    "'{}' carries a quoted include the scanner could not \
                     resolve in-repo — an external or build-generated \
                     dependency worth surfacing; pinning it into the repo \
                     is a candidate optimization",
                    unit.file.path
                ),
                severity: Severity::Info,
                confidence: Confidence::Medium,
                resolver_tier: ResolverTier::Syntactic,
                span: edge.span,
                path: edge.from_path.clone(),
                evidence: [
                    ("kind".to_string(), "unresolved_include".to_string()),
                    ("form".to_string(), "quoted".to_string()),
                ]
                .into_iter()
                .collect(),
                macro_suspect: false,
                error_node_overlap: false,
            });
        }
    }

    findings
}

// ===========================================================================
// PERF-C-ARCH-002 — hot-component aggregation
// ===========================================================================

/// Per-directory aggregate feeding the ARCH-002 ranking.
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

/// PERF-C-ARCH-002 — hot-component aggregation (catalog-v1 §3 Tier 1).
///
/// Pure aggregation over FunctionRecords + FileRecords ONLY — deliberately
/// NO access to other heuristics' findings: `finding_density` would couple
/// this rule to sibling emission order, breaking the no-sibling-order-
/// dependence purity rule (modules.md §4.1). The composite therefore ranks
/// on IR facts alone: complexity_sum ↓ → max_loop_depth ↓ →
/// alloc_site_density ↓ → path_prefix ↑.
///
/// Only the rank-1 component fires, and only when its complexity_sum
/// reaches [`ARCH_002_MIN_COMPLEXITY_SUM`] (floor keeps hello-world scans
/// clean). Full-composite ties all fire; because `path_prefix` participates
/// in the composite and is unique per component (it IS the grouping key),
/// such ties cannot occur in practice — the clause is honored structurally.
pub(crate) fn arch_002(units: &[FileUnit]) -> Vec<Finding> {
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

            // Anchor candidate: highest complexity, tie earliest
            // (start_line, start_col). Key encodes the ordering: lower is
            // better for (−complexity, line, col).
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

    // Rank composite: complexity_sum ↓ → max_loop_depth ↓ → density ↓ →
    // path_prefix ↑ (path_prefix breaks remaining ties deterministically).
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
    // Full-composite ties fire together (unreachable while path_prefix is
    // unique — see rustdoc).
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
            rule_id: ARCH_002_RULE_ID.to_string(),
            title: "Hot component".to_string(),
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
                    // Integer arithmetic upstream (u64) — plain integer text,
                    // never an f64 round-trip ("18", not "18.0"-style drift).
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
// PERF-C-ARCH-003 — cross-file call coupling
// ===========================================================================

/// One name-matched cross-file call edge. The aggregator consumes
/// [`cross_file_call_edges`] verbatim for `graphs.call_graph`, so this
/// shape is contract: plain data, deterministic construction, no
/// finding-specific fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrossFileCallEdge {
    /// Enclosing FunctionRecord identity (`path#name#line:col`).
    pub from_function_id: String,
    /// Caller file (repo-relative POSIX).
    pub from_path: String,
    /// Span of the (earliest) matching call site.
    pub call_span: Span,
    /// Callee identifier text (Direct calls only).
    pub callee_name: String,
    /// Matched definition file (lexicographically smallest non-local).
    pub to_path: String,
}

/// Build cross-file name-matched call edges (ADR-0001 resolver tiers: v1 is
/// name-based Syntactic; file-local-first).
///
/// Algorithm: map every FunctionRecord name → its definition paths across
/// ALL units; per unit, collect the same-file name set. Walk each function's
/// DIRECT calls in span order; a call whose callee is defined in the SAME
/// file is skipped (file-local-first per ADR-0001). Otherwise the target is
/// the lexicographically SMALLEST other-file definition path — ambiguity is
/// resolved deterministically rather than guessed semantically (documented
/// catalog limitation; semantic resolution is a later tier). Duplicates
/// collapse per `(from_function_id, callee_name)` keeping the EARLIEST call
/// span. Output is sorted `(from_function_id, callee_name, call_span)`.
pub(crate) fn cross_file_call_edges(units: &[FileUnit]) -> Vec<CrossFileCallEdge> {
    // name → all definition paths (sorted sets ⇒ deterministic min).
    let mut defs: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for unit in units {
        for func in &unit.functions {
            defs.entry(func.record.name.as_str())
                .or_default()
                .insert(func.record.path.as_str());
        }
    }

    let mut edges: Vec<CrossFileCallEdge> = Vec::new();
    for unit in units {
        let local_names: BTreeSet<&str> = unit
            .functions
            .iter()
            .map(|f| f.record.name.as_str())
            .collect();
        for func in &unit.functions {
            let mut calls: Vec<&crate::ir::CallEvent> = func.calls.iter().collect();
            // Span order makes "earliest wins" well-defined regardless of
            // backend emission order.
            calls.sort_by_key(|c| (c.meta.span.start_line, c.meta.span.start_col));
            let mut seen_pairs: BTreeSet<&str> = BTreeSet::new();
            for call in calls {
                if call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let name = call.callee_name.as_str();
                if local_names.contains(name) {
                    continue; // file-local-first (ADR-0001)
                }
                let Some(targets) = defs.get(name) else {
                    continue;
                };
                let Some(target) = targets.iter().find(|p| **p != func.record.path) else {
                    continue;
                };
                if seen_pairs.insert(name) {
                    edges.push(CrossFileCallEdge {
                        from_function_id: func.record.id.clone(),
                        from_path: func.record.path.clone(),
                        call_span: call.meta.span,
                        callee_name: name.to_string(),
                        to_path: (*target).to_string(),
                    });
                }
            }
        }
    }
    edges.sort_by(|a, b| {
        a.from_function_id
            .cmp(&b.from_function_id)
            .then(a.callee_name.cmp(&b.callee_name))
            .then(a.call_span.cmp(&b.call_span))
    });
    edges
}

/// PERF-C-ARCH-003 — cross-file call coupling (catalog-v1 §3 Tier 1).
///
/// ONE Info finding per [`cross_file_call_edge`], anchored at the call
/// site. Confidence is LOCKED LOW by schema policy (cross-file name matches
/// are hypotheses; the central hooks may only cap downward from there).
pub(crate) fn arch_003(units: &[FileUnit]) -> Vec<Finding> {
    cross_file_call_edges(units)
        .into_iter()
        .map(|edge| Finding {
            threshold: None,
            rule_id: ARCH_003_RULE_ID.to_string(),
            title: "Cross-file call coupling".to_string(),
            message: format!(
                "call to '{}' matches a definition in '{}' by name only — \
                 LOW-confidence hypothesis pending semantic resolution; \
                 confirming the match (or co-locating hot callees) is a \
                 candidate optimization",
                edge.callee_name, edge.to_path
            ),
            severity: Severity::Info,
            confidence: Confidence::Low,
            resolver_tier: ResolverTier::Syntactic,
            span: edge.call_span,
            path: edge.from_path,
            evidence: [
                ("callee_name".to_string(), edge.callee_name),
                ("matched_path".to_string(), edge.to_path),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        })
        .collect()
}
