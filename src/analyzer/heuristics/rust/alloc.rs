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

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::finding::{RS_ALLOC_001_RULE_ID, RS_ALLOC_002_RULE_ID, RS_ALLOC_003_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{CallEvent, CalleeKind, RUST_ALLOC_FAMILY, ResolverTier, ShapeKind, Span};
use crate::scanner::FileUnit;

use super::{callee_token, innermost_enclosing_loop, is_bare_identifier};

/// PERF-RS-ALLOC-001 — Heap/growable-buffer allocation inside loop
/// (mirrors PERF-C-ALLOC-001).
///
/// Intent: per-iteration allocation is allocator churn that dominates runtime;
/// hoisting the allocation or reusing a buffer is the optimization candidate.
/// Detection: any direct or macro [`crate::ir::CallEvent`] at
/// `in_loop_depth ≥ 1` AND `is_innermost` for which
/// [`RUST_ALLOC_FAMILY::matches_site`](crate::ir::RustAllocFamily::matches_site)
/// holds — a qualified constructor (`Vec::new`), a bare method (`to_vec`,
/// `collect`, `with_capacity`), or a macro (`vec!`, `format!`) when
/// `is_macro`.
///
/// M1 Precision Gate (plan `evidence-beta`): only the INNERMOST loop is hot
/// (an outer loop's allocation is one call per outer iteration, not per inner
/// element), so surviving sites are GROUPED by their innermost enclosing loop
/// and ONE finding is emitted per group — span = the group loop's span,
/// `resolver_tier` from that loop, evidence `alloc_count`, `callees`
/// (sorted-unique SHORT callee tokens joined by `", "`, capped at 8 with a
/// `+N more` suffix), `loop_depth` (the group loop's own `depth`; only the
/// defensive fallback, below, uses the call's derived `in_loop_depth`), and
/// `callee` (the first sorted token, kept for evidence-key compatibility). A
/// defensive fallback (no enclosing loop modelled) emits a singleton group
/// keyed by the call span. Groups sort deterministically by `(start_line,
/// start_col, function id)`; Hot/HIGH.
///
/// M1 also SUPPRESSES the literal-receiver string idioms: `"…".to_string()`
/// / `"…".to_owned()` build a `String` from a string literal (often
/// foldable), recognised from the base before the FINAL `.` of `callee_path`
/// after `trim_start()` starting with a string-literal prefix — plain `"`,
/// raw `r"`/`r#`, byte `b"`, or raw byte `br"`/`br#` — or — fixture form, no
/// `callee_path` — a first argument whose shape is a
/// [`ShapeKind::Literal`].
///
/// Accepted FP classes / honest limitations: `clone` is deliberately NOT in
/// the family (ubiquitous, not allocation pressure by itself — an accepted
/// false negative); a `Vec::new()` whose capacity is later reserved once
/// still fires (the syntactic tier cannot prove the buffer is reused); a
/// macro-generated allocation whose expansion is invisible under the
/// `token_tree` blind spot is not seen beyond the macro-invocation event
/// itself.
///
/// Precision FP-class fix 3: zero-alloc constructors never fire here (see
/// [`is_empty_ctor_site`]) — a `String::new()` / `Vec::new()` / map-set
/// `::new` call performs NO allocation at the site, so per-iteration
/// "allocator churn" on it is a false positive.
pub(crate) fn rust_alloc_001(units: &[FileUnit]) -> Vec<Finding> {
    // (function id, anchor line, anchor col) → aggregated group. BTreeMap
    // keeps construction deterministic; the final sort re-anchors by
    // (start_line, start_col, function id) as specified.
    let mut groups: BTreeMap<(String, u32, u32), Alloc001Group> = BTreeMap::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if !RUST_ALLOC_FAMILY.matches_site(
                    call.callee_path.as_deref(),
                    &call.callee_name,
                    call.is_macro,
                ) {
                    continue;
                }
                if call.meta.in_loop_depth == 0 || !call.meta.is_innermost {
                    continue;
                }
                if is_empty_ctor_site(call, func.record.alloc_family.as_deref()) {
                    continue;
                }
                if is_literal_receiver_string_alloc(call) {
                    continue;
                }
                // When the group anchors at a found loop, the finding's
                // `loop_depth` is the LOOP's own `depth` (not the call's
                // derived `in_loop_depth`) — the aggregation gate trusts the
                // modelled loop; only the defensive singleton fallback keeps
                // the call-derived depth.
                let (span, resolver_tier, loop_depth) =
                    match innermost_enclosing_loop(&func.loops, &call.meta.span) {
                        Some(loop_node) => {
                            (loop_node.span, loop_node.resolver_tier, loop_node.depth)
                        }
                        // Defensive fallback: no modelled enclosing loop, so
                        // anchor a singleton group at the call site.
                        None => (
                            call.meta.span,
                            call.meta.resolver_tier,
                            call.meta.in_loop_depth,
                        ),
                    };
                let key = (func.record.id.clone(), span.start_line, span.start_col);
                let group = groups.entry(key).or_insert_with(|| Alloc001Group {
                    function_id: func.record.id.clone(),
                    path: unit.file.path.clone(),
                    span,
                    resolver_tier,
                    loop_depth,
                    alloc_count: 0,
                    callees: BTreeSet::new(),
                });
                group.callees.insert(short_alloc_token(call));
                group.alloc_count += 1;
            }
        }
    }

    let mut groups: Vec<Alloc001Group> = groups.into_values().collect();
    groups.sort_by(|a, b| {
        (a.span.start_line, a.span.start_col, a.function_id.as_str()).cmp(&(
            b.span.start_line,
            b.span.start_col,
            b.function_id.as_str(),
        ))
    });

    let mut findings = Vec::with_capacity(groups.len());
    for group in groups {
        let callee_tokens: Vec<String> = group.callees.into_iter().collect();
        // Cap the displayed list so an aggregation over many distinct idioms
        // cannot produce an unbounded message; the remainder is summarized.
        // The list is sorted-unique (BTreeSet) so the cap is deterministic.
        const MAX_CALLEES: usize = 8;
        let total_callees = callee_tokens.len();
        let callees = if total_callees > MAX_CALLEES {
            format!(
                "{}, +{} more",
                callee_tokens[..MAX_CALLEES].join(", "),
                total_callees - MAX_CALLEES
            )
        } else {
            callee_tokens.join(", ")
        };
        let first_callee = callee_tokens.first().cloned().unwrap_or_default();
        findings.push(Finding {
            threshold: None,
            rule_id: RS_ALLOC_001_RULE_ID.to_string(),
            title: "Heap allocation inside loop (Rust)".to_string(),
            message: format!(
                "loop performs {} per-iteration allocation(s) ({callees}) at \
                 loop depth {} — allocator churn dominates runtime; hoisting \
                 the allocation or reusing a buffer is a candidate \
                 optimization",
                group.alloc_count, group.loop_depth
            ),
            severity: Severity::Hot,
            confidence: Confidence::High,
            resolver_tier: group.resolver_tier,
            span: group.span,
            path: group.path,
            evidence: [
                ("alloc_count".to_string(), group.alloc_count.to_string()),
                ("callees".to_string(), callees),
                ("loop_depth".to_string(), group.loop_depth.to_string()),
                ("callee".to_string(), first_callee),
            ]
            .into_iter()
            .collect(),
            macro_suspect: false,
            error_node_overlap: false,
        });
    }
    findings
}

/// RS-ALLOC-001 aggregation group: one innermost loop (or a defensive
/// fallback singleton keyed by a call span) plus its alloc-family sites.
struct Alloc001Group {
    function_id: String,
    path: String,
    span: Span,
    resolver_tier: ResolverTier,
    loop_depth: u32,
    alloc_count: u32,
    callees: BTreeSet<String>,
}

/// M1 short alloc token for message/evidence: a QUALIFIED name keeps its `::`
/// path (`Vec::with_capacity`, `HashMap::<K, V>::new`); methods and macros use
/// the bare `callee_name` (`format`, `to_string`, `collect`). Deliberately NOT
/// [`callee_token`], which returns the full receiver expression text for
/// method calls (e.g. `items.iter().collect`).
///
/// A path is trusted ONLY when it is a qualified name: it contains `::`,
/// contains no `(`, `)` or `.` (so a method call's embedded receiver
/// expression never leaks — `Mod::make().collect()` collapses to `collect`),
/// and its LAST `::` segment is a bare identifier (so `HashMap::<K, V>::new`
/// stays informative, while a trailing non-identifier segment is rejected).
/// Otherwise the bare `callee_name` is used.
fn short_alloc_token(call: &CallEvent) -> String {
    if call.is_macro {
        return call.callee_name.clone();
    }
    match call.callee_path.as_deref() {
        Some(path)
            if path.contains("::")
                && !path.contains('(')
                && !path.contains(')')
                && !path.contains('.')
                && path.rsplit("::").next().is_some_and(is_bare_identifier) =>
        {
            path.to_string()
        }
        _ => call.callee_name.clone(),
    }
}

/// Precision FP-class fix 3 — zero-alloc constructor paths: the qualified
/// `*::new` forms that never touch the allocator. Mirrors the
/// `AllocFamily::EmptyCtor` qualified arm of
/// `scanner::instrument::classify_alloc_call` (which the analyzer consults
/// via `record.alloc_family`); `Box`/`Rc`/`Arc` `::new` and
/// `with_capacity` are REAL allocation and stay out of this list.
const EMPTY_CTOR_QUALIFIED: [&str; 7] = [
    "String::new",
    "Vec::new",
    "VecDeque::new",
    "HashMap::new",
    "HashSet::new",
    "BTreeMap::new",
    "BTreeSet::new",
];

/// Precision FP-class fix 3 gate: true when this alloc-family site is a
/// zero-alloc constructor performing NO allocation at the call, so an
/// "allocator churn" finding on it would be a false positive.
///
/// Two arms (both conservative — anything unproven keeps firing):
/// - qualified `*::new` empty ctors ([`EMPTY_CTOR_QUALIFIED`]), provable
///   per-call from `callee_path` alone (exact match, mirroring the
///   scan-side classifier);
/// - the arg-less `vec![]` macro form: macro calls carry no args and no body
///   text analyzer-side, so emptiness is readable ONLY through the
///   scan-side dominant family — gated solely when
///   `record.alloc_family == "empty_ctor"` (computed WITH body text, where
///   available). A `vec![..]` with elements in an empty-dominant function is
///   the documented residual false negative.
fn is_empty_ctor_site(call: &CallEvent, alloc_family: Option<&str>) -> bool {
    if call
        .callee_path
        .as_deref()
        .is_some_and(|path| EMPTY_CTOR_QUALIFIED.contains(&path))
    {
        return true;
    }
    call.is_macro && call.callee_name == "vec" && alloc_family == Some("empty_ctor")
}

/// M1 literal-receiver suppression: `"…".to_string()` / `"…".to_owned()`.
/// The receiver is the base before the FINAL `.` of `callee_path` (leading
/// whitespace trimmed) when it starts with a string-literal prefix; for the
/// path-less fixture form the first argument is the receiver and must be a
/// literal shape.
fn is_literal_receiver_string_alloc(call: &CallEvent) -> bool {
    if call.callee_name != "to_string" && call.callee_name != "to_owned" {
        return false;
    }
    match call.callee_path.as_deref() {
        Some(path) => path
            .rsplit_once('.')
            .is_some_and(|(base, _method)| starts_with_string_literal(base.trim_start())),
        None => call
            .args
            .first()
            .is_some_and(|arg| arg.kind == ShapeKind::Literal),
    }
}

/// Whether `base` (a trimmed receiver expression) begins a string literal:
/// plain `"…"`, raw `r"…"`/`r#"…"#`, byte `b"…"`, or raw byte
/// `br"…"`/`br#"…"#`.
fn starts_with_string_literal(base: &str) -> bool {
    const PREFIXES: [&str; 6] = ["\"", "r\"", "r#", "b\"", "br\"", "br#"];
    PREFIXES.iter().any(|prefix| base.starts_with(prefix))
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
/// Precision FP-class fix 3: zero-alloc constructor sites (see
/// [`is_empty_ctor_site`]) do NOT count toward the co-occurrence escalation —
/// a `Vec::new()` + `drop()` pairing is not allocator churn.
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
                    && !is_empty_ctor_site(c, func.record.alloc_family.as_deref())
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
