//! Alloc-family heuristics (catalog-v1 §3 Tier 1):
//! PERF-C-ALLOC-001/002/003.

use crate::analyzer::finding::{ALLOC_001_RULE_ID, ALLOC_002_RULE_ID, ALLOC_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, ShapeKind};
use crate::scanner::FileUnit;

use super::{AllocCandidate, alloc_candidates, span_contains};

/// PERF-C-ALLOC-001 — Heap allocation inside loop `(syntactic)`
/// (catalog-v1 §3 detail block).
///
/// Detects: `malloc`/`calloc`/`realloc`/`aligned_alloc`/`strdup` calls
/// nested anywhere inside a loop body (`in_loop_depth ≥ 1`). Default
/// HIGH/hot; macro-origin capping is applied centrally (ir-v1.md §5).
///
/// Macro policy (catalog detail block): a wrapper-macro USE inside a loop
/// IS the alloc site under tree-sitter's no-expansion parse, so calls
/// whose callee names a recognized alloc-wrapper macro
/// (`pattern_id = ALLOC_WRAPPER_PATTERN_ID` in the same file's MacroEvents)
/// also fire — the central hook then caps them at MEDIUM + `macro_suspect`.
pub(crate) fn alloc_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for f in alloc_candidates(units) {
        let message = format!(
            "per-iteration heap allocation via '{}' at loop depth {} — \
             allocator churn dominates runtime; hoisting or buffer reuse \
             is a candidate optimization",
            f.callee, f.loop_depth
        );
        findings.push(Finding {
            threshold: None,
            rule_id: ALLOC_001_RULE_ID.to_string(),
            title: "Heap allocation inside loop".to_string(),
            message,
            severity: Severity::Hot,
            confidence: Confidence::High,
            resolver_tier: f.resolver_tier,
            span: f.span,
            path: f.path.to_string(),
            evidence: [
                ("callee".to_string(), f.callee.to_string()),
                ("loop_depth".to_string(), f.loop_depth.to_string()),
                (
                    "alloc_site_index".to_string(),
                    f.alloc_site_index.to_string(),
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

/// PERF-C-ALLOC-002 — Deallocation inside loop (catalog-v1 §3 Tier 1).
///
/// Detects DIRECT `free()` calls at `in_loop_depth ≥ 1`. Default Warn/HIGH;
/// co-occurrence escalation: when the SAME function also contains an
/// ALLOC-001-shaped site (alloc-family or wrapper-macro call at
/// `in_loop_depth ≥ 1`, see [`alloc_candidates`]), the pairing
/// allocate-per-iteration + free-per-iteration escalates the finding to Hot
/// with evidence `alloc_cooccurrence=true`.
///
/// Deferral note: the catalog detail block also lists "`fclose` on
/// heap-backed handles" as detected, but heap-backedness of a `FILE *`
/// handle is not syntactically decidable at MVP (no type info on the
/// syntactic tier). Only `free` fires here; `fclose` detection is deferred
/// to the semantic resolver tier (post-MVP per the ADR-0001 extension path).
pub(crate) fn alloc_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        let candidates = alloc_candidates(std::slice::from_ref(unit));
        for func in &unit.functions {
            for call in &func.calls {
                if !is_free_in_loop(call) {
                    continue;
                }
                let cooccurrence = candidates
                    .iter()
                    .any(|c: &AllocCandidate<'_>| span_contains(&func.record.span, &c.span));
                let message = format!(
                    "deallocation via '{}' at loop depth {} — repeated \
                     frees per iteration; hoisting the free or \
                     restructuring buffer ownership is a candidate \
                     optimization",
                    call.callee_name, call.meta.in_loop_depth
                );
                let mut evidence = vec![
                    ("callee".to_string(), call.callee_name.clone()),
                    (
                        "loop_depth".to_string(),
                        call.meta.in_loop_depth.to_string(),
                    ),
                ];
                if cooccurrence {
                    evidence.push(("alloc_cooccurrence".to_string(), "true".to_string()));
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: ALLOC_002_RULE_ID.to_string(),
                    title: "Deallocation inside loop".to_string(),
                    message,
                    severity: if cooccurrence {
                        Severity::Hot
                    } else {
                        Severity::Warn
                    },
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: evidence.into_iter().collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    findings
}

fn is_free_in_loop(call: &CallEvent) -> bool {
    call.callee_kind == CalleeKind::Direct
        && call.callee_name == "free"
        && call.meta.in_loop_depth >= 1
}

/// PERF-C-ALLOC-003 — `realloc()` linear growth in loop `(syntactic)`
/// (catalog-v1 §3 detail block).
///
/// Detects DIRECT `realloc` calls at `in_loop_depth ≥ 1` whose size
/// argument grows ADDITIVELY (`realloc(p, n + k)` shapes ⇒ quadratic
/// memmove behavior per the catalog). Multiplicative growth (`n * 2`)
/// does NOT fire — doubling growth is the documented fix, not the finding.
///
/// Additivity proof (founder-approved), two parts:
/// 1. the LAST argument (the size) is itself a `Binary` shape — this kills
///    first-arg pointer arithmetic like `realloc(p + 1, n)`, where a `+`
///    lies inside the call span but the size never grows;
/// 2. some `+` BinaryOpEvent lies fully inside the realloc call span (the
///    additive expression). Exact at the syntactic tier because a
///    BinaryOpEvent carries no operators on its operand shapes, but its
///    own span covers the additive expression.
pub(crate) fn alloc_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        let candidates = alloc_candidates(std::slice::from_ref(unit));
        for func in &unit.functions {
            // Candidates of THIS function only (span containment against
            // the record keeps sibling functions from cross-matching).
            for f in candidates
                .iter()
                .filter(|c| span_contains(&func.record.span, &c.span))
            {
                if f.callee != "realloc" {
                    continue;
                }
                // Conjunct 1: the LAST argument (the size) must itself be a
                // Binary shape — first-arg pointer arithmetic like
                // `realloc(p + 1, n)` has a plain-identifier size and zero
                // growth, so it stays silent.
                let size_is_binary = func
                    .calls
                    .iter()
                    .find(|c| c.callee_name == "realloc" && c.meta.span == f.span)
                    .and_then(|c| c.args.last())
                    .is_some_and(|arg| arg.kind == ShapeKind::Binary);
                // Conjunct 2: a `+` BinaryOpEvent lies inside the call span
                // (the additive size expression).
                let additive = func
                    .binary_ops
                    .iter()
                    .any(|b| b.op == "+" && span_contains(&f.span, &b.meta.span));
                if !(size_is_binary && additive) {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: ALLOC_003_RULE_ID.to_string(),
                    title: "Realloc linear growth in loop".to_string(),
                    message: format!(
                        "'{}' grows by addition inside a loop at depth {} — \
                         each growth copies the whole buffer; doubling the \
                         capacity instead of adding a fixed amount is a \
                         candidate optimization",
                        f.callee, f.loop_depth
                    ),
                    severity: Severity::Hot,
                    confidence: Confidence::Medium,
                    resolver_tier: f.resolver_tier,
                    span: f.span,
                    path: f.path.to_string(),
                    evidence: [
                        ("callee".to_string(), f.callee.to_string()),
                        ("loop_depth".to_string(), f.loop_depth.to_string()),
                        (
                            "alloc_site_index".to_string(),
                            f.alloc_site_index.to_string(),
                        ),
                        ("growth".to_string(), "additive".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    findings
}
