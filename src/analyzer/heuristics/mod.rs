//! Catalog heuristics (catalog-v1.md). One module-level function each,
//! pure over `[FileUnit]` — no I/O, no clock, no global state, no
//! sibling-order dependence (modules.md §4.1).
//!
//! Family layout: [`alloc`] (ALLOC-001/002/003), [`arch`] (ARCH-001/002/003),
//! [`call`] (CALL-001/002), `r#loop` (LOOP-001..008), [`mem`]
//! (MEM-001/002), [`str`] (STR-001/002/003), `r#type` (TYPE-001/002).
//! Shared candidate collection and span helpers live here so families stay
//! consistent.
//!
//! [`rust`] mirrors the 22-rule Rust catalog onto Rust idioms (R3 waves C1
//! structural + C2 API-name families; PERF-RS-STR-001 retired by the M1
//! Precision Gate, plan `evidence-beta`).

mod alloc;
mod arch;
mod call;
mod r#loop;
mod mem;
mod r#str;
mod r#type;

/// Rust (`PERF-RS-*`) heuristic families (plan `rust-backend` R3 workstreams
/// C1 + C2): the 22-rule Rust catalog mirrored onto Rust idioms (structural
/// subset in C1, API-name families in C2; STR-001 retired by the M1 Precision
/// Gate). The registry runs these only over `language == "rust"` units.
pub(super) mod rust;

pub(super) use alloc::{alloc_001, alloc_002, alloc_003};
pub(super) use arch::{arch_001, arch_002, arch_003};
pub(super) use call::{call_001, call_002};
pub(super) use r#loop::{
    loop_001, loop_002, loop_003, loop_004, loop_005, loop_006, loop_007, loop_008,
};
pub(super) use mem::{mem_001, mem_002};
pub(super) use rust::{
    rust_alloc_001, rust_alloc_002, rust_alloc_003, rust_arch_001, rust_arch_002, rust_arch_003,
    rust_call_001, rust_call_002, rust_loop_001, rust_loop_002, rust_loop_003, rust_loop_004,
    rust_loop_005, rust_loop_006, rust_loop_007, rust_loop_008, rust_mem_001, rust_mem_002,
    rust_str_002, rust_str_003, rust_type_001, rust_type_002,
};
/// Wave B2 (plan `semantic-resolver`): the resolver's literal-args mutation
/// pass reuses the scanf family/position contract — single source of truth,
/// never duplicated.
pub(crate) use r#str::scanf_format_index;
pub(super) use r#str::{str_001, str_002, str_003};
/// Shared language-parameterized LP64 layout model — reused by the Rust
/// TYPE family so both backends' size arithmetic is byte-identical.
pub(super) use r#type::{estimate_struct_size_with, type_001, type_002};

/// Cross-file name-matched call edges (ADR-0001 Syntactic tier): produced by
/// [`arch_003`] for findings and consumed verbatim by the aggregator for
/// `graphs.call_graph` (P4 Wave 2b).
pub(crate) use arch::{CrossFileCallEdge, cross_file_call_edges};

use crate::ir::{
    ALLOC_WRAPPER_PATTERN_ID, CallEvent, CalleeKind, MacroEventKind, ResolverTier, ShapeKind, Span,
};
use crate::scanner::FileUnit;

/// PERF-C-LOOP-001 nesting threshold (catalog default ≥ 3).
pub(crate) const LOOP_001_MIN_DEPTH: u32 = 3;

/// PERF-C-CALL-002 complexity threshold (catalog default ≥ 15).
pub(crate) const CALL_002_MIN_COMPLEXITY: u32 = 15;

/// PERF-C-ARCH-001 deep-chain threshold: transitive include depth at or
/// beyond this fires (catalog default ≥ 8).
pub(crate) const ARCH_001_MAX_INCLUDE_DEPTH: u32 = 8;

/// PERF-C-ARCH-001 hub threshold: fan-out OR fan-in at or beyond this fires
/// (catalog default ≥ 12).
pub(crate) const ARCH_001_HUB_FANOUT: usize = 12;

/// PERF-C-ARCH-002 component floor: the rank-1 component fires only when
/// its complexity_sum reaches this (catalog default ≥ 15; keeps hello-world
/// scans clean).
pub(crate) const ARCH_002_MIN_COMPLEXITY_SUM: u32 = 15;

/// PERF-C-TYPE-001 by-value struct size floor (LP64 estimated bytes): a
/// passed-by-value struct whose TU-defined estimate reaches this fires
/// (founder-approved floor of 32 — four 8-byte words; smaller structs are
/// cheaper to copy than to reference).
pub(crate) const TYPE_001_MIN_SIZE_BYTES: u64 = 32;

/// PERF-C-TYPE-002 padding-waste floor (bytes): the naive aligned layout
/// must waste at least this much versus the packed member sum before
/// reordering is worth surfacing. Derived from the catalog's "padding
/// candidate" wording with no numeric default: one full 8-byte LP64 word —
/// anything smaller is not actionable member reordering.
pub(crate) const TYPE_002_MIN_PADDING_BYTES: u64 = 8;

/// PERF-C-LOOP-007 nesting floor: membership-scan comparisons below this
/// depth lack the outer×inner iteration product that makes O(n²) lookups
/// hurt (catalog: "nested in loop").
pub(crate) const LOOP_007_MIN_DEPTH: u32 = 2;

/// PERF-C-LOOP-008 chain-depth floor: member/subscript chains shorter than
/// this rarely cross enough pointers to be cache-miss candidates (catalog:
/// depth ≥ 3, `a->b->c->d`).
pub(crate) const LOOP_008_MIN_CHAIN_DEPTH: usize = 3;

/// Inclusive containment: `inner` lies fully within `outer`, endpoints
/// allowed to touch (spans are inclusive on both ends, ir-v1.md §2).
pub(super) fn span_contains(outer: &Span, inner: &Span) -> bool {
    (outer.start_line, outer.start_col) <= (inner.start_line, inner.start_col)
        && (inner.end_line, inner.end_col) <= (outer.end_line, outer.end_col)
}

/// Internal candidate shape shared by alloc-family heuristics.
pub(super) struct AllocCandidate<'a> {
    pub path: &'a str,
    pub span: Span,
    pub callee: &'a str,
    pub loop_depth: u32,
    /// 1-based index of this call among the function's alloc-family
    /// calls (evidence for ALLOC-001/002 pairing downstream).
    pub alloc_site_index: u32,
    /// Resolver tier carried by the source event (ir-v1.md §2) — findings
    /// built from this candidate inherit it instead of hardcoding a tier.
    pub resolver_tier: ResolverTier,
}

/// Walk every CallEvent that is an alloc site: a DIRECT alloc-family
/// call, or a call whose callee names a recognized alloc-wrapper macro in
/// the same file (see [`alloc::alloc_001`] doc). Only sites with
/// `in_loop_depth ≥ 1` become candidates.
pub(super) fn alloc_candidates(units: &[FileUnit]) -> Vec<AllocCandidate<'_>> {
    let mut out = Vec::new();
    for unit in units {
        let wrapper_names: Vec<&str> = unit
            .macros
            .iter()
            .filter(|m| {
                m.kind == MacroEventKind::RecognizedPattern
                    && m.pattern_id.as_deref() == Some(ALLOC_WRAPPER_PATTERN_ID)
            })
            .map(|m| m.name.as_str())
            .collect();
        for func in &unit.functions {
            let mut site_index = 0u32;
            for call in &func.calls {
                if !is_alloc_call(call, &wrapper_names) {
                    continue;
                }
                site_index += 1;
                if call.meta.in_loop_depth >= 1 {
                    out.push(AllocCandidate {
                        path: &unit.file.path,
                        span: call.meta.span,
                        callee: &call.callee_name,
                        loop_depth: call.meta.in_loop_depth,
                        alloc_site_index: site_index,
                        resolver_tier: call.meta.resolver_tier,
                    });
                }
            }
        }
    }
    out
}

const ALLOC_FAMILY: [&str; 5] = ["malloc", "calloc", "realloc", "aligned_alloc", "strdup"];

fn is_alloc_call(call: &CallEvent, wrapper_names: &[&str]) -> bool {
    if call.callee_kind != CalleeKind::Direct {
        return false;
    }
    ALLOC_FAMILY.contains(&call.callee_name.as_str())
        || wrapper_names.contains(&call.callee_name.as_str())
}

/// Shared guard for arg-shape-sensitive heuristics: the first argument must
/// be an identifier shape; returns its name.
pub(super) fn first_arg_identifier(args: &[crate::ir::Shape]) -> Option<&str> {
    let first = args.first()?;
    if first.kind != ShapeKind::Identifier {
        return None;
    }
    first.name.as_deref()
}

/// Schema token for a [`ShapeKind`] (serde snake_case spelling, ir-v1.md
/// §3.4) — used as evidence text so findings stay byte-deterministic.
pub(super) fn shape_kind_token(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Literal => "literal",
        ShapeKind::Identifier => "identifier",
        ShapeKind::Call => "call",
        ShapeKind::Subscript => "subscript",
        ShapeKind::MemberAccess => "member_access",
        ShapeKind::Unary => "unary",
        ShapeKind::Binary => "binary",
        ShapeKind::Sizeof => "sizeof",
        ShapeKind::Other => "other",
    }
}
