//! Python architecture-family heuristics (plan `python-backend` W3):
//! PY-ARCH-001/002/003.
//!
//! The W0 canary did NOT make any `ARCH-*` threshold decision-grade on Python
//! (docs/architecture/python-backend.md §9.2/§9.4): these ship conservative
//! Info findings and are measured in W4, not asserted. PY-ARCH-001 only sees
//! RESOLVED import edges — unresolved internal imports stay data-only with no
//! finding (§5), which gates the cycle rule against the FP stream the Rust N1
//! decision killed.

use std::collections::BTreeMap;

use crate::analyzer::finding::{PY_ARCH_001_RULE_ID, PY_ARCH_002_RULE_ID, PY_ARCH_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::graph::break_cycles;
use crate::ir::{ResolverTier, Span};
use crate::scanner::FileUnit;

use super::super::cross_file_call_edges;
use super::sort_findings;

/// Conservative hot-component complexity floor (W4-measured, not asserted).
const PY_ARCH_002_MIN_COMPLEXITY_SUM: u64 = 40;

/// PY-ARCH-001 — Import cycle (transferred mirror of PERF-C-ARCH-001), Info.
///
/// Over RESOLVED internal import edges only: one finding per
/// [`break_cycles`]-broken edge, anchored at that pair's earliest retained
/// directive span. A Python import cycle is a partial-initialisation /
/// import-order hazard and a readability cost. Evidence `kind="cycle"`,
/// `coupled_with`. Info/MEDIUM.
pub(crate) fn python_arch_001(units: &[FileUnit]) -> Vec<Finding> {
    // (from, to) → earliest span.
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
    let pairs: Vec<(String, String)> = earliest.keys().cloned().collect();
    let result = break_cycles(&pairs);
    let mut findings = Vec::new();
    for (from, to) in &result.broken {
        let span = earliest
            .get(&(from.clone(), to.clone()))
            .copied()
            .unwrap_or_default();
        findings.push(Finding {
            threshold: None,
            rule_id: PY_ARCH_001_RULE_ID.to_string(),
            title: "Import cycle".to_string(),
            message: format!(
                "'{from}' and '{to}' import each other, forming a cycle — \
                 CPython executes a partially-initialised module during the \
                 cycle, and the coupling resists testing; breaking the \
                 lowest-sorted edge into a shared leaf module is a candidate \
                 optimization"
            ),
            severity: Severity::Info,
            confidence: Confidence::Medium,
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
    }
    sort_findings(&mut findings);
    findings
}

/// Parent directory of a repo-relative POSIX path; root-level files map to
/// `"."`.
fn parent_dir(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => ".".to_string(),
    }
}

/// PY-ARCH-002 — Hot-component aggregation (transferred mirror of
/// PERF-C-ARCH-002), Info.
///
/// Ranks directories by aggregated cyclomatic complexity (then max loop
/// depth, then path); only the rank-1 component fires and only when its
/// complexity sum reaches the conservative floor. Thresholds are NOT
/// decision-grade on Python and are measured in W4 — the caveat is carried in
/// the rule message. Evidence `path_prefix`, `complexity_sum`,
/// `max_loop_depth`, `function_count`. Info/MEDIUM.
pub(crate) fn python_arch_002(units: &[FileUnit]) -> Vec<Finding> {
    struct Stat {
        complexity_sum: u64,
        max_loop_depth: u32,
        function_count: u32,
    }
    let mut stats: BTreeMap<String, Stat> = BTreeMap::new();
    // dir → (complexity, span, record path) best anchor.
    let mut anchors: BTreeMap<String, (u32, Span, String)> = BTreeMap::new();
    for unit in units {
        for func in &unit.functions {
            let dir = parent_dir(&func.record.path);
            let entry = stats.entry(dir.clone()).or_insert(Stat {
                complexity_sum: 0,
                max_loop_depth: 0,
                function_count: 0,
            });
            entry.complexity_sum += u64::from(func.record.cyclomatic_complexity);
            entry.max_loop_depth = entry.max_loop_depth.max(func.record.max_loop_depth);
            entry.function_count += 1;
            let slot =
                anchors
                    .entry(dir)
                    .or_insert((0, func.record.span, func.record.path.clone()));
            if func.record.cyclomatic_complexity > slot.0
                || (func.record.cyclomatic_complexity == slot.0 && func.record.span < slot.1)
            {
                *slot = (
                    func.record.cyclomatic_complexity,
                    func.record.span,
                    func.record.path.clone(),
                );
            }
        }
    }
    let mut ranked: Vec<(&String, &Stat)> = stats.iter().collect();
    ranked.sort_by(|(pa, a), (pb, b)| {
        b.complexity_sum
            .cmp(&a.complexity_sum)
            .then(b.max_loop_depth.cmp(&a.max_loop_depth))
            .then(pa.cmp(pb))
    });
    let Some((best_dir, best)) = ranked.first().copied() else {
        return Vec::new();
    };
    if best.complexity_sum < PY_ARCH_002_MIN_COMPLEXITY_SUM {
        return Vec::new();
    }
    let (_, span, record_path) = anchors
        .get(best_dir)
        .expect("component with functions has an anchor");
    let finding = Finding {
        threshold: None,
        rule_id: PY_ARCH_002_RULE_ID.to_string(),
        title: "Hot component".to_string(),
        message: format!(
            "directory '{}' aggregates complexity {} across {} function(s) — \
             the hottest component in this scan (thresholds are conservative \
             and not yet measured on Python); splitting or simplifying its \
             hottest functions is a candidate optimization",
            best_dir, best.complexity_sum, best.function_count
        ),
        severity: Severity::Info,
        confidence: Confidence::Medium,
        resolver_tier: ResolverTier::Syntactic,
        span: *span,
        path: record_path.clone(),
        evidence: [
            ("path_prefix".to_string(), best_dir.clone()),
            (
                "complexity_sum".to_string(),
                best.complexity_sum.to_string(),
            ),
            (
                "max_loop_depth".to_string(),
                best.max_loop_depth.to_string(),
            ),
            (
                "function_count".to_string(),
                best.function_count.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        macro_suspect: false,
        error_node_overlap: false,
    };
    vec![finding]
}

/// PY-ARCH-003 — Name-based cross-module call coupling (transferred mirror of
/// PERF-C-ARCH-003), Info.
///
/// Reuses the shared [`cross_file_call_edges`] producer (name-based fallback:
/// a callee name with EXACTLY ONE other-file definition). Python duck typing
/// and ubiquitous names (`get`/`save`/`run`) make this noisier than in C/Rust,
/// so the finding ships LOW-confidence Info; the caveat is in the message.
/// Evidence `callee_name`, `matched_path`. Info/LOW.
///
/// The Python catalog is **syntactic-only unconditionally** (the invariant in
/// `python/mod.rs`): even a resolver-confirmed name match reports
/// [`ResolverTier::Syntactic`] — there is no `Semantic` assignment path left for
/// Python (the central `apply_upgrade_hooks` gate used to revert an unreachable
/// `Semantic` claim here; the dead branch is removed).
pub(crate) fn python_arch_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings: Vec<Finding> = cross_file_call_edges(units)
        .into_iter()
        .map(|edge| {
            let message = if edge.resolved {
                format!(
                    "call to '{}' resolves to its definition in '{}' — \
                     cross-file coupling confirmed by the resolver; \
                     co-locating hot callees remains a candidate \
                     optimization",
                    edge.callee_name, edge.to_path
                )
            } else {
                format!(
                    "call to '{}' matches a definition in '{}' by name only \
                     — LOW-confidence hypothesis (Python duck typing makes \
                     name coupling noisy); confirming the match or \
                     co-locating hot callees is a candidate optimization",
                    edge.callee_name, edge.to_path
                )
            };
            Finding {
                threshold: None,
                rule_id: PY_ARCH_003_RULE_ID.to_string(),
                title: "Cross-file call coupling".to_string(),
                message,
                severity: Severity::Info,
                confidence: edge.confidence,
                resolver_tier: ResolverTier::Syntactic,
                span: edge.call_span,
                path: edge.from_path,
                evidence: [
                    ("callee_name".to_string(), edge.callee_name),
                    ("matched_path".to_string(), edge.to_path),
                    ("resolved".to_string(), edge.resolved.to_string()),
                ]
                .into_iter()
                .collect(),
                macro_suspect: false,
                error_node_overlap: false,
            }
        })
        .collect();
    sort_findings(&mut findings);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::heuristics::python::test_support::{call, func, point};
    use crate::ir::{FileRecord, IncludeEdge, IncludeForm, LocCounts};
    use crate::scanner::FileUnit;

    fn edge(from: &str, to: &str) -> IncludeEdge {
        IncludeEdge {
            from_path: from.to_string(),
            to_path: Some(to.to_string()),
            form: IncludeForm::Quoted,
            resolved: true,
            is_mod: false,
            span: point(1),
        }
    }

    fn file_unit(
        path: &str,
        includes: Vec<IncludeEdge>,
        functions: Vec<crate::scanner::FunctionUnit>,
    ) -> FileUnit {
        FileUnit {
            file: FileRecord {
                path: path.to_string(),
                language: "python".to_string(),
                loc: LocCounts::default(),
                function_count: functions.len() as u32,
                global_var_count: 0,
                include_counts: Default::default(),
            },
            parse_health: Default::default(),
            includes,
            global_vars: Vec::new(),
            structs: Vec::new(),
            macros: Vec::new(),
            functions,
        }
    }

    #[test]
    fn arch_001_fires_on_import_cycle() {
        let a = file_unit("a.py", vec![edge("a.py", "b.py")], vec![]);
        let b = file_unit("b.py", vec![edge("b.py", "a.py")], vec![]);
        assert_eq!(python_arch_001(&[a, b]).len(), 1);
    }

    #[test]
    fn arch_002_fires_only_above_floor() {
        let mut hot = func(
            "hot",
            point(2),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        hot.record.cyclomatic_complexity = 50;
        hot.record.path = "pkg/a.py".to_string();
        let hot_unit = file_unit("pkg/a.py", vec![], vec![hot]);
        assert_eq!(python_arch_002(&[hot_unit]).len(), 1);

        let mut cool = func(
            "cool",
            point(2),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        cool.record.cyclomatic_complexity = 5;
        let cool_unit = file_unit("pkg/b.py", vec![], vec![cool]);
        assert!(python_arch_002(&[cool_unit]).is_empty());
    }

    #[test]
    fn arch_003_emits_low_confidence_name_edges() {
        // a.py calls `helper`, defined only in b.py → one LOW edge.
        let caller = func(
            "caller",
            point(2),
            vec![call("helper", None, None, point(3), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        let a = file_unit("a.py", vec![], vec![caller]);
        let mut def = func(
            "helper",
            point(2),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        def.record.path = "b.py".to_string();
        let b = file_unit("b.py", vec![], vec![def]);
        let out = python_arch_003(&[a, b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].confidence, Confidence::Low);
        assert_eq!(out[0].resolver_tier, ResolverTier::Syntactic);
    }
}
