//! Rust memory-op-family heuristics (plan `rust-backend` R3 workstream C2):
//! PERF-RS-MEM-001/002 — the API-name mirror of the C memory family
//! (`heuristics::mem`) onto Rust idioms.
//!
//! Purity (modules.md §4.1): pure IR views, no I/O/clock/globals. Messages use
//! the mandatory "candidate optimization" framing.

use std::collections::BTreeSet;

use crate::analyzer::finding::{RS_MEM_001_RULE_ID, RS_MEM_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CalleeKind, ShapeKind};
use crate::scanner::FileUnit;

use super::{callee_token, is_size_of_call};

/// Capacity constructor method (MEM-001). `Vec::with_capacity` and
/// `String::with_capacity` both surface `callee_name == "with_capacity"`.
const WITH_CAPACITY_METHOD: &str = "with_capacity";

/// Block-copy method family (MEM-002): Rust's `memcpy`/`memmove` family —
/// bulk per-element or whole-buffer copies. M1 Precision Gate removed
/// `clone`/`to_vec`: borrow-checker clones and cheap `Copy` type copies are
/// not block copies, and without receiver-type evidence the collision is
/// noise (they require receiver-type evidence before returning).
const BLOCK_COPY_METHODS: [&str; 3] = [
    "copy_from_slice",
    "copy_nonoverlapping",
    "extend_from_slice",
];

/// PERF-RS-MEM-001 — `size_of` passed as an element count
/// (REFRAMED mirror of PERF-C-MEM-001).
///
/// REFRAMING (mandatory, documented): C's `sizeof` misuse shapes
/// (`malloc(n * sizeof(p))` sizing the pointer) have no byte-count analogue
/// in Rust because allocation goes through typed constructors. The real
/// misuse is passing a BYTE count where an ELEMENT count is expected:
/// `Vec::with_capacity(size_of::<T>())` reserves `size_of::<T>()` ELEMENTS
/// (each `size_of::<T>()` bytes) — `size_of::<T>()` squared bytes, often
/// enormous. Detection: a DIRECT `with_capacity` call (covers
/// `Vec::with_capacity` / `String::with_capacity`) whose argument list
/// contains either
/// - a `Call` shape whose callee name starts with `size_of`/`size_of_val`
///   (the Rust intrinsic — the backend records the nested callee text in
///   `Shape::name`, so the nested call IS expressible in the IR), or
/// - a `Sizeof` shape (a fixture/C-style representation) carrying
///   `sizeof_operand`.
///
/// Warn/MEDIUM, anchored at the call; evidence `callee`, `misuse`. Multiple
/// classes join into sorted evidence (mirroring the C rule).
///
/// HONEST LIMITATION: the nested-callee match is name-prefix based, so a
/// user function named `size_of_thing` also matches (documented FP class);
/// conversely a computed byte count (`let n = size_of::<T>(); with_capacity(n)`)
/// whose `size_of` call is hoisted out of the argument is NOT detectable
/// (accepted false negative).
pub(crate) fn rust_mem_001(units: &[FileUnit]) -> Vec<Finding> {
    /// Misuse class inserted when a `size_of`/`size_of_val` call (or a
    /// C-style `Sizeof` shape) appears as a capacity argument.
    const SIZE_OF_AS_ELEMENT_COUNT: &str = "size_of_as_element_count";

    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.is_macro
                    || call.callee_name != WITH_CAPACITY_METHOD
                {
                    continue;
                }
                let mut misuses: BTreeSet<&'static str> = BTreeSet::new();
                for arg in &call.args {
                    if is_size_of_call(arg)
                        || (arg.kind == ShapeKind::Sizeof && arg.sizeof_operand.is_some())
                    {
                        misuses.insert(SIZE_OF_AS_ELEMENT_COUNT);
                    }
                }
                if misuses.is_empty() {
                    continue;
                }
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_MEM_001_RULE_ID.to_string(),
                    title: "size_of passed as an element count (Rust)".to_string(),
                    message: format!(
                        "'{callee}' receives a `size_of` byte count where an \
                         element COUNT is expected — that over-reserves by a \
                         factor of the element size; passing the element count \
                         is a candidate optimization",
                    ),
                    severity: Severity::Warn,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
                        (
                            "misuse".to_string(),
                            misuses.iter().copied().collect::<Vec<_>>().join("+"),
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

/// PERF-RS-MEM-002 — Block copy/clone in loop
/// (REFRAMED mirror of PERF-C-MEM-002).
///
/// REFRAMING (mandatory, documented): Rust block-copy calls
/// (`copy_from_slice`, `copy_nonoverlapping`, `extend_from_slice`) carry NO
/// explicit size argument, so the C rule's "non-literal size" guard collapses
/// to presence-in-loop. Detection: a DIRECT method call in the block-copy
/// family at `in_loop_depth ≥ 1`.
/// Info/MEDIUM by default; escalated to Warn/MEDIUM when the call sits in
/// the INNERMOST loop (`is_innermost`), where per-element small copies are
/// most repetitive. Evidence `callee`, `loop_depth`, `innermost`.
///
/// M1 Precision Gate (plan `evidence-beta`) REMOVED `clone` and `to_vec` from
/// the family: a borrow-checker clone is required by ownership and a `Copy`
/// type's clone is a bit-copy, neither is a block copy — and the syntactic
/// tier has no receiver-type evidence to distinguish them, so they were the
/// dominant false-positive class. Re-admitting them requires receiver-type
/// evidence.
///
/// Accepted FP classes: per-element small-struct copies are idiomatic and
/// cheap (the Info band keeps them candidate-grade).
pub(crate) fn rust_mem_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct
                    || call.is_macro
                    || !BLOCK_COPY_METHODS.contains(&call.callee_name.as_str())
                    || call.meta.in_loop_depth == 0
                {
                    continue;
                }
                let innermost = call.meta.is_innermost;
                let callee = callee_token(call);
                findings.push(Finding {
                    threshold: None,
                    rule_id: RS_MEM_002_RULE_ID.to_string(),
                    title: "Block copy/clone in loop (Rust)".to_string(),
                    message: format!(
                        "'{callee}' copies a block every iteration at loop \
                         depth {} — hoisting the copy, batching the copies, or \
                         reusing a buffer is a candidate optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: if innermost {
                        Severity::Warn
                    } else {
                        Severity::Info
                    },
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
                        ("innermost".to_string(), innermost.to_string()),
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
