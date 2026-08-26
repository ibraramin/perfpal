//! Type-family heuristics (catalog-v1 §4 Tier 2): PERF-C-TYPE-001/002.
//!
//! Both rules run on an APPROXIMATE LP64 size model (catalog: "member
//! counting + literal array dims within the TU"; LOW confidence is
//! mandatory — no real type layout at MVP). Documented FP classes:
//! - typedef'd struct params are invisible (no typedef IR) — only literal
//!   `struct <Tag>` parameter text matches;
//! - nested-struct sizing is unmodeled — a struct-typed member counts via
//!   the conservative unknown-type rule below;
//! - name-based scope classification is approximate (param text is raw
//!   source, so `struct Tag` in a comment-like position cannot occur here,
//!   but qualifiers/macros in the text are not resolved).

use crate::analyzer::finding::{TYPE_001_RULE_ID, TYPE_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{ResolverTier, StructDef, StructMember};
use crate::scanner::FileUnit;

use super::{TYPE_001_MIN_SIZE_BYTES, TYPE_002_MIN_PADDING_BYTES};

/// PERF-C-TYPE-001 — Large struct passed by value (catalog-v1 §4 Tier 2).
///
/// ONE finding PER FUNCTION aggregating EVERY parameter whose raw
/// `param_types` text is a `struct <Tag>` WITHOUT any pointer star AND
/// whose tag resolves to a struct defined in the SAME translation unit
/// with an LP64-model estimated size ≥ [`TYPE_001_MIN_SIZE_BYTES`].
/// Aggregation is REQUIRED, not stylistic: parameter spans are not
/// modeled, so per-parameter findings would all anchor at the function
/// record span and deterministic dedup (`collapse_duplicates` keys on
/// rule:path:line:col) would silently drop all but the first. Anchored at
/// the function record span; zero qualifying params ⇒ no finding.
///
/// `FunctionRecord` carries no resolver tier of its own; v1 emits only
/// [`ResolverTier::Syntactic`] (ir-v1.md §2), so that tier is used — same
/// precedent as CALL-002.
pub(crate) fn type_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            // Qualifying params in source order: (raw param text, tag,
            // LP64 estimate).
            let mut hits: Vec<(&str, &str, u64)> = Vec::new();
            for param in &func.record.param_types {
                let Some(tag) = by_value_struct_tag(param) else {
                    continue;
                };
                let Some(def) = unit.structs.iter().find(|s| s.name == tag) else {
                    continue;
                };
                let estimated = estimate_struct_size(def);
                if estimated < TYPE_001_MIN_SIZE_BYTES {
                    continue;
                }
                hits.push((param, tag, estimated));
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
                rule_id: TYPE_001_RULE_ID.to_string(),
                title: "Large struct passed by value".to_string(),
                message: format!(
                    "parameters {listed} are copied by value on every \
                     call under the LP64 estimate — passing pointers or \
                     taking the structs by reference is a candidate \
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

/// PERF-C-TYPE-002 — Struct member ordering / padding candidate
/// (catalog-v1 §4 Tier 2).
///
/// Fires per TU-defined struct whose naive aligned layout wastes at least
/// [`TYPE_002_MIN_PADDING_BYTES`] versus the packed member sum. Single-
/// member structs never fire — ordering cannot reclaim anything when
/// there is nothing to reorder.
pub(crate) fn type_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for def in &unit.structs {
            if def.members.len() < 2 {
                continue;
            }
            let aligned = estimate_struct_size(def);
            let packed: u64 = def.members.iter().map(member_size).sum();
            let padding = aligned.saturating_sub(packed);
            if padding < TYPE_002_MIN_PADDING_BYTES {
                continue;
            }
            findings.push(Finding {
                rule_id: TYPE_002_RULE_ID.to_string(),
                title: "Struct member ordering (padding candidate)".to_string(),
                message: format!(
                    "struct '{}' wastes ~{padding} bytes of padding in \
                     its declared member order (aligned estimate \
                     {aligned} vs packed sum {packed}) — ordering \
                     members largest-alignment-first is a candidate \
                     optimization",
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

/// `struct <Tag>` parameter text WITHOUT any pointer star ⇒ `Some(Tag)`.
/// Pointer params pass by reference and never fire (FP guard); anything
/// without a literal `struct` keyword is invisible to this heuristic
/// (typedef FP class, module doc).
fn by_value_struct_tag(param: &str) -> Option<&str> {
    if param.contains('*') {
        return None;
    }
    let lower = param.to_ascii_lowercase();
    let idx = lower.find("struct")?;
    // Keyword boundary: must not sit inside a larger identifier.
    if idx > 0
        && param[..idx]
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let rest = param[idx + "struct".len()..].trim_start();
    let tag = rest
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()?;
    (!tag.is_empty()).then_some(tag)
}

/// Naive aligned-layout size estimate (LP64): each member rounds the
/// offset up to its alignment (size clamped to 1..=8), then the total
/// rounds up to the maximum member alignment (tail padding).
fn estimate_struct_size(def: &StructDef) -> u64 {
    let mut offset = 0u64;
    let mut max_align = 1u64;
    for member in &def.members {
        let size = member_size(member);
        let align = size.clamp(1, 8);
        offset += (align - offset % align) % align;
        offset += size;
        max_align = max_align.max(align);
    }
    offset += (max_align - offset % max_align) % max_align;
    offset
}

/// Member byte size under the LP64 model. Pointer members are 8 bytes —
/// recorded explicitly as [`StructMember::is_pointer`] because declarator
/// stars never reach `type_text`. Arrays multiply by the literal dim.
/// UNKNOWN member types count conservatively as 8 bytes (the LP64 maximum
/// primitive size/alignment) so suspected padding waste is never
/// understated; nested-struct sizing is unmodeled (module doc FP class).
fn member_size(member: &StructMember) -> u64 {
    if member.is_pointer {
        return 8;
    }
    let base = lp64_type_size(&member.type_text).unwrap_or(8);
    match member.array_dim_literal {
        Some(dim) if dim > 0 => base.saturating_mul(dim),
        _ => base,
    }
}

/// LP64 primitive table over raw type text: pointers/long/long long/
/// double = 8, int/float = 4, short = 2, char/_Bool = 1. Unknown text ⇒
/// None (caller applies the conservative 8). Known-type caveat: `long
/// double` is 16 bytes on LP64 but models as 8 via the `long` word — a
/// KNOWN type whose size IS understated here (rare in perf-critical
/// structs; accepted).
fn lp64_type_size(type_text: &str) -> Option<u64> {
    let lower = type_text.to_ascii_lowercase();
    let has_word = |kw: &str| {
        lower
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|w| w == kw)
    };
    if type_text.contains('*') || has_word("long") || has_word("double") {
        return Some(8);
    }
    if has_word("short") {
        return Some(2);
    }
    if has_word("int") || has_word("float") {
        return Some(4);
    }
    if has_word("char") || has_word("_bool") || has_word("bool") {
        return Some(1);
    }
    None
}
