//! Rust alloc-family heuristics (plan `rust-backend` R3 workstream C2):
//! PERF-RS-ALLOC-001/002/003 — the API-name mirror of the C alloc family
//! (`heuristics::alloc`) onto Rust's allocation idioms.
//!
//! Where the C family matched bare library names (`malloc`/`free`/`realloc`)
//! on `callee_name`, Rust allocation is spread across qualified constructors
//! (`Vec::new`, `Box::new`, `String::with_capacity`), methods (`to_vec`,
//! `collect`, `with_capacity`) and macros (`vec!`, `format!`). The shared
//! curated source of truth is [`crate::ir::RUST_ALLOC_FAMILY`]
//! (`matches_site`), so the scanner's `alloc_site_count` and these rules
//! count the SAME sites.
//!
//! Purity (modules.md §4.1): pure IR views, no I/O/clock/globals. Messages use
//! the mandatory "candidate optimization" framing; macro-origin capping is
//! applied centrally (`analyzer::apply_downgrade_hooks`).

use crate::analyzer::finding::{RS_ALLOC_001_RULE_ID, RS_ALLOC_002_RULE_ID, RS_ALLOC_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CalleeKind, RUST_ALLOC_FAMILY};
use crate::scanner::FileUnit;

use super::callee_token;

/// PERF-RS-ALLOC-001 — Heap/growable-buffer allocation inside loop
/// (mirrors PERF-C-ALLOC-001).
///
/// Intent: per-iteration allocation is allocator churn that dominates runtime;
/// hoisting the allocation or reusing a buffer is the optimization candidate.
/// Detection: any direct or macro [`crate::ir::CallEvent`] at
/// `in_loop_depth ≥ 1` for which
/// [`RUST_ALLOC_FAMILY::matches_site`](crate::ir::RustAllocFamily::matches_site)
/// holds — a qualified constructor (`Vec::new`), a bare method (`to_vec`,
/// `collect`, `with_capacity`), or a macro (`vec!`, `format!`) when
/// `is_macro`. Hot/HIGH; `resolver_tier` is inherited from the call event.
/// Evidence `callee`, `loop_depth`, `alloc_site_index` (1-based index among
/// the function's alloc-family sites in declaration order, mirroring the C
/// rule's pairing key).
///
/// Accepted FP classes / honest limitations: `clone` is deliberately NOT in
/// the family (ubiquitous, not allocation pressure by itself — an accepted
/// false negative); a `Vec::new()` whose capacity is later reserved once
/// still fires (the syntactic tier cannot prove the buffer is reused); a
/// macro-generated allocation whose expansion is invisible under the
/// `token_tree` blind spot is not seen beyond the macro-invocation event
/// itself.
pub(crate) fn rust_alloc_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let mut site_index = 0u32;
            for call in &func.calls {
                if !RUST_ALLOC_FAMILY.matches_site(
                    call.callee_path.as_deref(),
                    &call.callee_name,
                    call.is_macro,
                ) {
                    continue;
                }
                site_index += 1;
                if call.meta.in_loop_depth == 0 {
                    continue;
                }
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_ALLOC_001_RULE_ID.to_string(),
                    title: "Heap allocation inside loop (Rust)".to_string(),
                    message: format!(
                        "per-iteration allocation via '{callee}' at loop depth \
                         {} — allocator churn dominates runtime; hoisting the \
                         allocation or reusing a buffer is a candidate \
                         optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Hot,
                    confidence: Confidence::High,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
                        ),
                        ("alloc_site_index".to_string(), site_index.to_string()),
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

/// PERF-RS-ALLOC-002 — Deallocation (`drop`) inside loop
/// (mirrors PERF-C-ALLOC-002).
///
/// Intent: repeated per-iteration teardown of an owned value is allocation
/// churn when paired with a per-iteration allocation. Detection: a DIRECT
/// `drop` call (non-macro) at `in_loop_depth ≥ 1`. Default Warn/HIGH;
/// CO-OCCURRENCE ESCALATION: when the SAME function also contains any
/// allocation site firing ALLOC-001 (an `RUST_ALLOC_FAMILY.matches_site`
/// call at `in_loop_depth ≥ 1`), the allocate-per-iteration +
/// drop-per-iteration pairing escalates the finding to Hot with additive
/// evidence `alloc_cooccurrence="true"` (mirroring the C rule's
/// malloc/free pairing).
///
/// Accepted FP classes / honest limitations: `drop` is also the explicit
/// destructor call on a legitimately scoped value; a balanced
/// allocate-per-iteration + drop-per-iteration is still flagged as churn, not
/// a bug verdict. Evidence `callee`, `loop_depth`[ , `alloc_cooccurrence` ].
pub(crate) fn rust_alloc_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let alloc_cooccurrence = func.calls.iter().any(|c| {
                c.meta.in_loop_depth >= 1
                    && RUST_ALLOC_FAMILY.matches_site(
                        c.callee_path.as_deref(),
                        &c.callee_name,
                        c.is_macro,
                    )
            });
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.is_macro
                    || call.callee_name != "drop"
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let callee = callee_token(call);
                let mut evidence = vec![
                    ("callee".to_string(), callee.clone()),
                    (
                        "loop_depth".to_string(),
                        call.meta.in_loop_depth.to_string(),
                    ),
                ];
                if alloc_cooccurrence {
                    evidence.push(("alloc_cooccurrence".to_string(), "true".to_string()));
                }
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_ALLOC_002_RULE_ID.to_string(),
                    title: "Deallocation inside loop (Rust)".to_string(),
                    message: format!(
                        "deallocation via 'drop' at loop depth {} — repeated \
                         drop-per-iteration teardown; hoisting the drop or \
                         restructuring value ownership is a candidate \
                         optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: if alloc_cooccurrence {
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

/// PERF-RS-ALLOC-003 — Growable-buffer growth in loop
/// (REFRAMED mirror of PERF-C-ALLOC-003).
///
/// REFRAMING (mandatory, documented): Rust has no `realloc`; the additive-
/// growth analogue of C's `realloc(p, n + k)` is a method call to
/// `reserve`/`reserve_exact`, which grows a growable buffer's capacity by an
/// ADDITIVE amount (`v.reserve(chunk)` per iteration ⇒ repeated reallocation
/// and copy). Multiplicative capacity doubling is the documented fix, so only
/// these additive-reserve methods fire. Detection: a DIRECT
/// `reserve`/`reserve_exact` call at `in_loop_depth ≥ 1`. Hot/MEDIUM
/// (MEDIUM because the syntactic tier cannot see whether the reserve is
/// amortized or a one-shot warm-up; MEDIUM also matches the C rule's
/// heuristic additivity proof). Evidence `callee`, `loop_depth`,
/// `growth="additive"`.
///
/// Accepted FP classes / honest limitations: a reserve whose argument is a
/// loop-invariant large constant still fires (it may be a deliberate
/// one-time pre-allocation); `with_capacity` growth is NOT this rule's shape
/// (it is an allocation, handled by ALLOC-001).
pub(crate) fn rust_alloc_003(units: &[FileUnit]) -> Vec<Finding> {
    /// Additive-reserve method family (the `realloc(p, n + k)` analogue).
    const ADDITIVE_GROWTH_FAMILY: [&str; 2] = ["reserve", "reserve_exact"];

    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.is_macro
                    || !ADDITIVE_GROWTH_FAMILY.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_ALLOC_003_RULE_ID.to_string(),
                    title: "Growable-buffer additive growth in loop (Rust)".to_string(),
                    message: format!(
                        "'{callee}' grows a buffer by addition inside a loop \
                         at depth {} — each reserve may reallocate and copy \
                         the whole buffer; reserving a larger capacity once \
                         up front is a candidate optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Hot,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
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
