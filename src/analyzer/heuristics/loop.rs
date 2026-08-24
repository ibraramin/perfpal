//! Loop-family heuristics (catalog-v1 §3 Tier 1): PERF-C-LOOP-001/003.

use crate::analyzer::finding::{LOOP_001_RULE_ID, LOOP_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CalleeKind, LoopNode, StatementRole};
use crate::scanner::FileUnit;

use super::{LOOP_001_MIN_DEPTH, span_contains};

/// Character-wise stdio family (LOOP-003).
const CHAR_IO_FAMILY: [&str; 5] = ["fgetc", "getc", "getchar", "fputc", "putc"];

/// PERF-C-LOOP-001 — Deeply nested loops (catalog-v1 §3 Tier 1).
///
/// Fires on every LoopNode whose depth is EXACTLY [`LOOP_001_MIN_DEPTH`] —
/// one finding per maximal nest entry point (a depth-5 chain yields one
/// finding at its depth-3 ancestor, not three). `max_depth` evidence
/// reports how deep the chain under this entry point actually reaches.
pub(crate) fn loop_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for loop_node in &func.loops {
                if loop_node.depth != LOOP_001_MIN_DEPTH {
                    continue;
                }
                // Max depth among this loop's inclusive descendants; the
                // loop contains itself, so the floor is its own depth.
                let max_depth = func
                    .loops
                    .iter()
                    .filter(|l| span_contains(&loop_node.span, &l.span))
                    .map(|l: &LoopNode| l.depth)
                    .max()
                    .unwrap_or(LOOP_001_MIN_DEPTH)
                    .max(LOOP_001_MIN_DEPTH);
                findings.push(Finding {
                    rule_id: LOOP_001_RULE_ID.to_string(),
                    title: "Deeply nested loops".to_string(),
                    message: format!(
                        "loops nest {} deep here (chain reaches depth \
                         {max_depth}) — flattening the nesting or adding \
                         early exits is a candidate optimization",
                        LOOP_001_MIN_DEPTH
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::High,
                    resolver_tier: loop_node.resolver_tier,
                    span: loop_node.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("nest_depth".to_string(), LOOP_001_MIN_DEPTH.to_string()),
                        ("max_depth".to_string(), max_depth.to_string()),
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

/// PERF-C-LOOP-003 — Character-wise stdio as primary loop-body statement
/// (catalog-v1 §3 Tier 1). Byte-at-a-time stdio calls that ARE the loop
/// body's primary statement signal a per-character processing pattern;
/// buffered block reads are the optimization candidate.
pub(crate) fn loop_003(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || !CHAR_IO_FAMILY.contains(&call.callee_name.as_str())
                    || call.statement_role != StatementRole::Primary
                {
                    continue;
                }
                findings.push(Finding {
                    rule_id: LOOP_003_RULE_ID.to_string(),
                    title: "Character-wise stdio in loop".to_string(),
                    message: format!(
                        "character-wise I/O via '{}' as the loop body's \
                         primary statement — buffered block reads or \
                         writes are a candidate optimization",
                        call.callee_name
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), call.callee_name.clone()),
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
