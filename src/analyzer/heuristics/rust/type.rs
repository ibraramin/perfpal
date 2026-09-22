//! Rust type-family heuristics (plan `rust-backend` R3 workstream C1):
//! PERF-RS-TYPE-001/002, mirroring PERF-C-TYPE-001/002.
//!
//! Both rules reuse the SHARED language-parameterized LP64 layout model
//! ([`estimate_struct_size_with`]) so Rust and C size arithmetic stay
//! byte-identical; only the per-member byte mapping differs (Rust type text:
//! `i32`/`f32` = 4, `i64`/`f64`/`usize` = 8, `i8`/`u8`/`bool` = 1, `char` =
//! 4, references/pointers = 8, …). LOW confidence is mandatory — no real
//! layout at MVP.
//!
//! Documented FP classes (shared with the C model): tuple-struct positional
//! members are name-bearing synthetics; nested-struct sizing is unmodeled (a
//! struct-typed member counts conservatively as 8 bytes); generic/unknown
//! member types (`Vec<T>`, named types) also count as the conservative 8.

use std::collections::BTreeMap;

use crate::analyzer::finding::{RS_TYPE_001_RULE_ID, RS_TYPE_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{ResolverTier, StructDef, StructKind, StructMember};
use crate::scanner::FileUnit;

use super::super::{
    TYPE_001_MIN_SIZE_BYTES, TYPE_002_MIN_PADDING_BYTES, estimate_struct_size_with,
};

/// PERF-RS-TYPE-001 — Large struct passed by value (mirrors
/// PERF-C-TYPE-001).
///
/// Intent: a large `struct` copied by value on every call is cheaper to pass
/// by reference. Detection: a Rust [`crate::ir::FunctionRecord::param_types`]
/// entry that is a BARE named type (no `&`, `*`, `<`, `[`, `(`, `::`, or
/// whitespace) resolving to a same-crate [`StructDef`] with
/// `kind == Struct` whose LP64 estimate ≥ [`TYPE_001_MIN_SIZE_BYTES`] (32).
/// ONE finding PER FUNCTION aggregating every qualifying parameter (parameter
/// spans are not modeled, so per-parameter findings would share the function
/// record span and collapse under deterministic dedup). Warn/LOW; evidence
/// `params`, `structs`, `estimated_sizes`, `threshold`.
///
/// Definition lookup: a `StructDef` in the function's OWN file wins;
/// otherwise the lexicographically smallest defining path (deterministic
/// across the scan). Accepted FP classes: `Copy`-cheap large arrays and
/// intentionally-owned values still surface (Warn/LOW, candidate-grade);
/// tuple structs and re-exported aliases are not resolved.
pub(crate) fn rust_type_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut defs: BTreeMap<&str, Vec<&StructDef>> = BTreeMap::new();
    for unit in units {
        for def in &unit.structs {
            if def.kind == StructKind::Struct {
                defs.entry(def.name.as_str()).or_default().push(def);
            }
        }
    }

    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let mut hits: Vec<(&str, &str, u64)> = Vec::new();
            for param in &func.record.param_types {
                let Some(tag) = bare_named_type(param) else {
                    continue;
                };
                let Some(def) = find_struct(&defs, tag, &unit.file.path) else {
                    continue;
                };
                let estimated = rust_estimate_struct_size(def);
                if estimated < TYPE_001_MIN_SIZE_BYTES {
                    continue;
                }
                hits.push((param.as_str(), tag, estimated));
            }
            if hits.is_empty() {
                continue;
            }
            let listed = hits
                .iter()
                .map(|(param, tag, est)| format!("'{param}' (struct '{tag}', ~{est} bytes)"))
                .collect::<Vec<_>>()
                .join(", ");
            findings.push(Finding {
                threshold: None,
                rule_id: RS_TYPE_001_RULE_ID.to_string(),
                title: "Large struct passed by value (Rust)".to_string(),
                message: format!(
                    "parameters {listed} are copied by value on every \
                     call under the LP64 estimate — taking the structs by \
                     reference (`&T`) or `Box`-ing them is a candidate \
                     optimization"
                ),
                severity: Severity::Warn,
                confidence: Confidence::Low,
                resolver_tier: ResolverTier::Syntactic,
                span: func.record.span,
                path: unit.file.path.clone(),
                evidence: [
                    (
                        "params".to_string(),
                        hits.iter()
                            .map(|(param, _, _)| *param)
                            .collect::<Vec<_>>()
                            .join("; "),
                    ),
                    (
                        "structs".to_string(),
                        hits.iter()
                            .map(|(_, tag, _)| *tag)
                            .collect::<Vec<_>>()
                            .join(";"),
                    ),
                    (
                        "estimated_sizes".to_string(),
                        hits.iter()
                            .map(|(_, _, est)| est.to_string())
                            .collect::<Vec<_>>()
                            .join(";"),
                    ),
                    ("threshold".to_string(), TYPE_001_MIN_SIZE_BYTES.to_string()),
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

/// PERF-RS-TYPE-002 — struct field-order padding candidate (mirrors
/// PERF-C-TYPE-002).
///
/// Detection: per [`StructDef`] with `kind == Struct`, `members.len() ≥ 2`,
/// and (aligned estimate − packed member sum) ≥
/// [`TYPE_002_MIN_PADDING_BYTES`] (8). Info/LOW, anchored at the struct
/// definition span; evidence `struct`, `padding_bytes`, `packed_sum`,
/// `aligned_estimate`, `threshold`.
///
/// HONEST CAVEAT (founder brief, mandatory): Rust's default `repr(Rust)` is
/// free to REORDER fields automatically, so the declared-order padding waste
/// measured here is not necessarily real. This makes the rule a
/// lower-confidence candidate than the C analogue; it is emitted (mirroring
/// all 23 catalog ids) but deliberately framed as a signal to check whether
/// the type is `#[repr(C)]` / FFI-sensitive, where declared order IS the
/// layout. Single-member structs never fire (nothing to reorder).
pub(crate) fn rust_type_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for def in &unit.structs {
            if def.kind != StructKind::Struct || def.members.len() < 2 {
                continue;
            }
            let aligned = rust_estimate_struct_size(def);
            let packed: u64 = def.members.iter().map(rust_member_size).sum();
            let padding = aligned.saturating_sub(packed);
            if padding < TYPE_002_MIN_PADDING_BYTES {
                continue;
            }
            findings.push(Finding {
                threshold: None,
                rule_id: RS_TYPE_002_RULE_ID.to_string(),
                title: "Struct field ordering (padding candidate, Rust)".to_string(),
                message: format!(
                    "struct '{}' shows ~{padding} bytes of declared-order \
                     padding (aligned estimate {aligned} vs packed sum \
                     {packed}) — if the type is `#[repr(C)]`/FFI-sensitive, \
                     ordering fields largest-alignment-first is a candidate \
                     optimization; default `repr(Rust)` may already reorder \
                     fields automatically, so treat this as lower-confidence",
                    def.name
                ),
                severity: Severity::Info,
                confidence: Confidence::Low,
                resolver_tier: ResolverTier::Syntactic,
                span: def.span,
                path: unit.file.path.clone(),
                evidence: [
                    ("struct".to_string(), def.name.clone()),
                    ("padding_bytes".to_string(), padding.to_string()),
                    ("packed_sum".to_string(), packed.to_string()),
                    ("aligned_estimate".to_string(), aligned.to_string()),
                    (
                        "threshold".to_string(),
                        TYPE_002_MIN_PADDING_BYTES.to_string(),
                    ),
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

/// A BARE named type: trimmed, non-empty, and consisting solely of ASCII
/// alphanumerics/underscore. This excludes references/pointers (`&T`, `*T`),
/// generics (`Vec<T>`), arrays (`[T; N]`), tuples (`(A, B)`), paths
/// (`a::T`), slices, and any whitespace — the "by-value named struct" shape
/// only. Returns the identifier slice.
fn bare_named_type(param: &str) -> Option<&str> {
    let trimmed = param.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Some(trimmed)
    } else {
        None
    }
}

/// Resolve a bare struct name to a same-crate definition: the function's own
/// file wins, else the lexicographically smallest defining path (ties by
/// span) for determinism.
fn find_struct<'a>(
    defs: &'a BTreeMap<&'a str, Vec<&'a StructDef>>,
    name: &str,
    unit_path: &str,
) -> Option<&'a StructDef> {
    let candidates = defs.get(name)?;
    candidates
        .iter()
        .find(|d| d.path == unit_path)
        .or_else(|| {
            candidates
                .iter()
                .min_by_key(|d| (d.path.as_str(), d.span.start_line, d.span.start_col))
        })
        .copied()
}

/// Naive aligned-layout estimate over the shared model with the Rust member
/// sizer.
fn rust_estimate_struct_size(def: &StructDef) -> u64 {
    estimate_struct_size_with(def, rust_member_size)
}

/// Member byte size under the LP64 model for Rust type text. References and
/// raw pointers are 8 bytes (recorded as [`StructMember::is_pointer`]).
/// Arrays multiply by the literal dim. Unknown/generic/named member types
/// count conservatively as 8 bytes (the LP64 maximum primitive size/
/// alignment) so suspected padding waste is never understated.
fn rust_member_size(member: &StructMember) -> u64 {
    if member.is_pointer {
        return 8;
    }
    let base = rust_type_size(&member.type_text).unwrap_or(8);
    match member.array_dim_literal {
        Some(dim) if dim > 0 => base.saturating_mul(dim),
        _ => base,
    }
}

/// LP64 primitive table over raw Rust type text: `i64`/`u64`/`isize`/
/// `usize`/`f64` = 8, `i32`/`u32`/`f32`/`char` = 4, `i16`/`u16` = 2,
/// `i8`/`u8`/`bool` = 1, `i128`/`u128` = 16. Unknown text ⇒ `None` (caller
/// applies the conservative 8).
fn rust_type_size(type_text: &str) -> Option<u64> {
    let t = type_text.trim();
    if t.contains('&') || t.contains('*') {
        return Some(8);
    }
    match t {
        "i8" | "u8" | "bool" => Some(1),
        "i16" | "u16" => Some(2),
        "i32" | "u32" | "f32" | "char" => Some(4),
        "i64" | "u64" | "isize" | "usize" | "f64" => Some(8),
        "i128" | "u128" => Some(16),
        _ => None,
    }
}
