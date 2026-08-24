//! Catalog heuristics (catalog-v1.md). One module-level function each,
//! pure over `[FileUnit]` — no I/O, no clock, no global state, no
//! sibling-order dependence (modules.md §4.1).
//!
//! Family layout: [`alloc`] (ALLOC-001/002), [`call`] (CALL-001),
//! `r#loop` (LOOP-001/003), [`str`] (STR-001/002/003). Shared candidate
//! collection and span helpers live here so families stay consistent.

mod alloc;
mod call;
mod r#loop;
mod r#str;

pub(super) use alloc::{alloc_001, alloc_002};
pub(super) use call::call_001;
pub(super) use r#loop::{loop_001, loop_003};
pub(super) use r#str::{str_001, str_002, str_003};

use crate::ir::{ALLOC_WRAPPER_PATTERN_ID, CallEvent, CalleeKind, MacroEventKind, ShapeKind, Span};
use crate::scanner::FileUnit;

/// PERF-C-LOOP-001 nesting threshold (catalog default ≥ 3).
pub(crate) const LOOP_001_MIN_DEPTH: u32 = 3;

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
