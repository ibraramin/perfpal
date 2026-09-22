//! Rust (`PERF-RS-*`) heuristic families (plan `rust-backend` R3 workstreams
//! C1 + C2). Family layout mirrors `heuristics/`: [`alloc`]
//! (RS-ALLOC-001/002/003), [`arch`] (RS-ARCH-001/002/003), [`call`]
//! (RS-CALL-001/002), `r#loop` (RS-LOOP-001..008), [`mem`]
//! (RS-MEM-001/002), `r#str` (RS-STR-001/002/003), and `r#type`
//! (RS-TYPE-001/002).
//!
//! These functions are only EVER invoked with `language == "rust"` units
//! (registry language gating, `analyzer::analyze`); they are pure IR views
//! with no I/O, clock, global state, or sibling-order dependence
//! (modules.md §4.1).
//!
//! The C rules matched bare C library names on `callee_name`; Rust methods
//! and constructors carry a richer `callee_path` (e.g. `Vec::with_capacity`,
//! `s.push_str`), so the API-name families below match curated name lists
//! against `callee_path` (qualified/associ-ated functions) and
//! `callee_name` (methods) as appropriate — the family lists are
//! Rust-API-specific and deliberately NOT shared with the C backend.

mod alloc;
mod arch;
mod call;
mod r#loop;
mod mem;
mod r#str;
mod r#type;

use crate::ir::{CallEvent, ShapeKind};

pub(crate) use alloc::{rust_alloc_001, rust_alloc_002, rust_alloc_003};
pub(crate) use arch::{rust_arch_001, rust_arch_002, rust_arch_003};
pub(crate) use call::{rust_call_001, rust_call_002};
pub(crate) use r#loop::{
    rust_loop_001, rust_loop_002, rust_loop_003, rust_loop_004, rust_loop_005, rust_loop_006,
    rust_loop_007, rust_loop_008,
};
pub(crate) use mem::{rust_mem_001, rust_mem_002};
pub(crate) use r#str::{rust_str_001, rust_str_002, rust_str_003};
pub(crate) use r#type::{rust_type_001, rust_type_002};

/// Evidence-friendly callee token: the FULL `callee_path` when the Rust
/// backend populated it (`Vec::with_capacity`, `s.push_str`,
/// `std::slice::from_raw_parts`), else the bare `callee_name`. Deterministic
/// and identical across a rule's finding construction (ir-v1.md §3.3).
pub(super) fn callee_token(call: &CallEvent) -> String {
    call.callee_path
        .clone()
        .unwrap_or_else(|| call.callee_name.clone())
}

/// Bare Rust identifier text: non-empty, ASCII alphanumerics/underscore only,
/// no leading digit. Used to decide whether a call's receiver/base text is a
/// simple identifier (`s` in `s.push_str`) rather than an expression
/// (`items.iter()` in `items.iter().collect()`, `build()` in
/// `build().push_str`).
pub(super) fn is_bare_identifier(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !text.chars().next().is_some_and(|c| c.is_ascii_digit())
}

/// m1: a usable method receiver — a bare identifier (`s`) or a dotted chain of
/// bare identifiers (`self.buf`, `self.a.b`). A call or any other expression
/// (`build()`, `items.iter()`) is NOT a receiver we can name, and must never be
/// substituted by an argument.
fn is_simple_receiver(text: &str) -> bool {
    !text.is_empty() && text.split('.').all(is_bare_identifier)
}

/// Receiver identifier of a METHOD call.
///
/// Preferred source: the base of `callee_path` before the FINAL `.` when that
/// base is a simple receiver (`s.push_str` ⇒ `s`; `self.buf.push_str` ⇒
/// `self.buf`). This is what the Rust backend actually emits for
/// `recv.method(...)` — the receiver lives in the `field_expression` callee,
/// NOT in the call's argument list. Fallback: the first argument when it is an
/// `Identifier` shape, used for the UFCS/fixture form (`String::push_str`, no
/// dotted base) and for fixture-IR documents that model the receiver as arg 0.
///
/// m1 invariant: for a dotted `callee_path`, the ARGUMENT LIST IS NEVER the
/// receiver — a non-simple base (`build().push_str`, `self.buf` is simple but
/// `f().buf` is not) yields `None` rather than silently reporting the argument.
pub(super) fn method_receiver(call: &CallEvent) -> Option<&str> {
    match call.callee_path.as_deref() {
        Some(path) if path.contains('.') => path
            .rsplit_once('.')
            .map(|(base, _method)| base)
            .filter(|base| is_simple_receiver(base)),
        _ => super::first_arg_identifier(&call.args),
    }
}

/// Whether `arg` is a call shape to a Rust `size_of`/`size_of_val` intrinsic.
///
/// The backend records a `call_expression` shape's callee function text in
/// `Shape::name`, so `size_of::<u64>()` arrives as
/// `Call { name: Some("size_of::<u64>") }`, `std::mem::size_of::<u64>()` as
/// `Call { name: Some("std::mem::size_of::<u64>") }`, and `size_of_val(&x)`
/// as `Call { name: Some("size_of_val") }`. Matching the FINAL path segment
/// (turbofish stripped) against the `size_of` prefix covers all spellings —
/// `size_of_val` shares the prefix by design.
pub(super) fn is_size_of_call(arg: &crate::ir::Shape) -> bool {
    if arg.kind != ShapeKind::Call {
        return false;
    }
    let Some(name) = arg.name.as_deref() else {
        return false;
    };
    let without_turbofish = name.split("::<").next().unwrap_or(name);
    let segment = without_turbofish
        .rsplit("::")
        .next()
        .unwrap_or(without_turbofish);
    segment.starts_with("size_of")
}

/// Qualified-name matcher: true when `path` equals `entry` or ends with
/// `::entry` (so `std::fs::read` matches the curated `fs::read` and
/// `File::open` matches the curated `File::open`).
pub(super) fn qualified_match(path: Option<&str>, entry: &str) -> bool {
    match path {
        Some(p) => p == entry || p.ends_with(&format!("::{entry}")),
        None => false,
    }
}
