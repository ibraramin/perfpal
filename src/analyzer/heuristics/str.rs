//! String-family heuristics (catalog-v1 §3 Tier 1):
//! PERF-C-STR-001/002/003.

use crate::analyzer::finding::{STR_001_RULE_ID, STR_002_RULE_ID, STR_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, ShapeKind};
use crate::scanner::FileUnit;

use super::{first_arg_identifier, span_contains};

/// Accumulation targets (STR-001): quadratic rescan per concatenation.
const ACCUM_FAMILY: [&str; 2] = ["strcat", "strncat"];

/// Unbounded copy family (STR-003), fires anywhere — not loop-gated.
const UNBOUNDED_COPY_FAMILY: [&str; 4] = ["strcpy", "strcat", "sprintf", "gets"];

/// Formatted-input family (STR-003) — fires only with an unbounded `%s`
/// conversion in a string-literal format argument.
const SCANF_FAMILY: [&str; 3] = ["scanf", "fscanf", "sscanf"];

/// PERF-C-STR-001 — String accumulation in loop (catalog-v1 §3 Tier 1).
///
/// Detects `strcat`/`strncat` into an identifier destination at
/// `in_loop_depth ≥ 1`: each call rescans the destination from the start,
/// so accumulation inside a loop is quadratic in the final length.
pub(crate) fn str_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !ACCUM_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let Some(dest) = first_arg_identifier(&call.args) else {
                    continue;
                };
                findings.push(Finding {
                    threshold: None,
                    rule_id: STR_001_RULE_ID.to_string(),
                    title: "String accumulation in loop".to_string(),
                    message: format!(
                        "'{}' accumulates into '{dest}' at loop depth {} — \
                         each call rescans the destination from the start; \
                         tracking the running length or sizing the buffer \
                         up front is a candidate optimization",
                        call.callee_name, call.meta.in_loop_depth
                    ),
                    severity: Severity::Hot,
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), call.callee_name.clone()),
                        ("dest".to_string(), dest.to_string()),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
                        ),
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

/// PERF-C-STR-002 — strlen() in loop without reassignment (catalog-v1 §3).
///
/// Detects DIRECT `strlen(S)` on identifier `S` at `in_loop_depth ≥ 1`.
/// Syntactic suppression: when some assignment to `S` lies inside the
/// innermost enclosing loop (max-depth LoopNode containing the call span;
/// tie-break earliest span), the length may legitimately change per
/// iteration and no finding is emitted. Defensive fallback (no enclosing
/// LoopNode despite `in_loop_depth ≥ 1`): suppress only when an assignment
/// to `S` exists anywhere in the same function span.
pub(crate) fn str_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if !is_strlen_in_loop(call) {
                    continue;
                }
                // first_arg_identifier already proved kind == Identifier;
                // the name is what the reassignment check keys on.
                let Some(ident) = first_arg_identifier(&call.args) else {
                    continue;
                };
                if strlen_reassigned(func, call, ident) {
                    continue;
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: STR_002_RULE_ID.to_string(),
                    title: "strlen() recomputed in loop".to_string(),
                    message: format!(
                        "strlen('{ident}') is recomputed at loop depth {} \
                         without a reassignment of '{ident}' inside the \
                         loop; hoisting the length before the loop is a \
                         candidate optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("identifier".to_string(), ident.to_string()),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
                        ),
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

fn is_strlen_in_loop(call: &CallEvent) -> bool {
    call.callee_kind == CalleeKind::Direct
        && call.callee_name == "strlen"
        && call.meta.in_loop_depth >= 1
        && call
            .args
            .first()
            .is_some_and(|a| a.kind == ShapeKind::Identifier)
}

/// Reassignment check (syntactic only): find the innermost enclosing
/// LoopNode = max depth among loops whose span contains the strlen span
/// (tie-break earliest span); suppress iff an assignment to `ident` lands
/// inside that loop's span. No enclosing loop found (defensive) ⇒ fall back
/// to any assignment to `ident` within the same function span.
fn strlen_reassigned(func: &crate::scanner::FunctionUnit, call: &CallEvent, ident: &str) -> bool {
    let enclosing = func
        .loops
        .iter()
        .filter(|l| span_contains(&l.span, &call.meta.span))
        .max_by_key(|l| (l.depth, std::cmp::Reverse(l.span)));
    let assigns_to_ident = |span_filter: &dyn Fn(&crate::ir::Span) -> bool| {
        func.assignments.iter().any(|a| {
            a.target.kind == ShapeKind::Identifier
                && a.target.name.as_deref() == Some(ident)
                && span_filter(&a.meta.span)
        })
    };
    match enclosing {
        Some(loop_node) => assigns_to_ident(&|s| span_contains(&loop_node.span, s)),
        None => assigns_to_ident(&|s| span_contains(&func.record.span, s)),
    }
}

/// PERF-C-STR-003 — Unbounded string operation (catalog-v1 §3 Tier 1).
///
/// Fires ANYWHERE (not loop-gated): DIRECT calls to
/// `strcpy`/`strcat`/`sprintf`/`gets`, or `scanf`/`fscanf`/`sscanf` whose
/// string-literal format argument carries an unbounded `%s` conversion.
/// Framing rule (catalog): optimization candidate, never bug/unsafe.
pub(crate) fn str_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let name = call.callee_name.as_str();
                if UNBOUNDED_COPY_FAMILY.contains(&name) {
                    emit_unbounded(&mut findings, unit, call, None);
                } else if SCANF_FAMILY.contains(&name) {
                    // Format-POSITION contract (catalog): only the format
                    // argument is inspected — `scanf` at index 0, `fscanf`
                    // and `sscanf` at index 1 (the leading FILE*/buffer
                    // shifts the format one slot). A `%s` riding in a
                    // non-format argument never fires.
                    let format_index: usize = if name == "scanf" { 0 } else { 1 };
                    let format_literal = call
                        .args
                        .get(format_index)
                        .filter(|a| a.kind == ShapeKind::Literal)
                        .and_then(|a| {
                            let raw = a.value.as_deref()?;
                            strip_quotes(raw).contains("%s").then(|| strip_quotes(raw))
                        });
                    if let Some(format_text) = format_literal {
                        emit_unbounded(&mut findings, unit, call, Some(format_text));
                    }
                }
            }
        }
    }
    findings
}

/// Push one STR-003 finding; `format_text` carries the matched `%s`
/// literal for the scanf family.
fn emit_unbounded(
    findings: &mut Vec<Finding>,
    unit: &FileUnit,
    call: &CallEvent,
    format_text: Option<&str>,
) {
    let name = call.callee_name.as_str();
    let message = match format_text {
        Some(format_text) => format!(
            "unbounded '%s' conversion in '{name}' format \
             \"{format_text}\" — a width specifier on the conversion is a \
             candidate optimization"
        ),
        None => format!(
            "unbounded operation via '{name}' — a bounded variant or \
             explicit length tracking is a candidate optimization"
        ),
    };
    let mut evidence = vec![("callee".to_string(), name.to_string())];
    if let Some(format_text) = format_text {
        evidence.push(("format".to_string(), format_text.to_string()));
    }
    findings.push(Finding {
        threshold: None,
        rule_id: STR_003_RULE_ID.to_string(),
        title: "Unbounded string operation".to_string(),
        message,
        severity: Severity::Warn,
        confidence: Confidence::High,
        resolver_tier: call.meta.resolver_tier,
        span: call.meta.span,
        path: unit.file.path.clone(),
        evidence: evidence.into_iter().collect(),
        macro_suspect: false,
        error_node_overlap: false,
    });
}

/// Strip surrounding double quotes from a raw literal's source text
/// (`Shape::literal` values keep quotes, ir-v1.md §3.4).
fn strip_quotes(raw: &str) -> &str {
    raw.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw)
}
