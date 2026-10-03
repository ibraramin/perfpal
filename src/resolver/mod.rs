//! `resolver` — semantic enrichment seam (modules.md §4.3, plan
//! `semantic-resolver` Wave A).
//!
//! Resolvers sit between scanner and analyzer: they consume the IR stream and
//! enrich it. The seam's inviolable contracts (modules.md §4.3, ADR-0001):
//!
//! - **Identity is never touched.** `(path, name, span)` keys, IR record
//!   shapes, and — in Wave A — every heuristic-visible field are unchanged by
//!   [`resolve`]. The only output this stage writes in Wave A is the additive
//!   `ParseHealth.resolver` honesty field (ir-v1.md additive note).
//! - **Conservative-upgrade rule:** a resolver tier is raised ONLY on
//!   unambiguous resolution. Ambiguous same-name targets stay on the
//!   syntactic tier. Waves B1+B2 consume the internal maps below to apply
//!   such upgrades (B1: cross-file USR resolution; B2: fopen-handle and
//!   string-literal facts); Wave A built them and deliberately emitted
//!   nothing beyond resolver health.
//! - **USRs are internal-only.** Clang USRs are used exclusively for decl↔def
//!   pairing inside this module's maps. Per ADR-0001's identity rule
//!   ("entity identity never uses engine-specific keys — a future semantic
//!   pass upgrades confidence; it never changes schema or identity"), a USR
//!   must NEVER appear in IR identity, findings, metrics.json, or any output
//!   artifact. [`DefinitionSite`]/[`CallRefMap`] deliberately do NOT
//!   implement `Serialize`.
//! - **Musl constraint (by design):** static musl targets cannot `dlopen`;
//!   the `semantic-clang` cargo feature stays OFF for musl release targets.
//!   The feature uses clang-sys `runtime` mode (lazy `dlopen` via libloading),
//!   so even feature-enabled builds run WITHOUT libclang present: absence is
//!   graceful and first-class — every file reports
//!   [`crate::ir::ResolverHealth::Absent`].
//!
//! Determinism: files are processed sorted by repo-relative path; all maps
//! are `BTreeMap`s keyed by `(path, start_line, start_col)` (the definition
//! map is keyed by USR with first-wins on the lexicographically smallest
//! `(path, span)` for duplicate USRs — deterministic regardless of input
//! order).
//!
//! Explicit-root contract: [`resolve`] takes the canonicalized scan root as
//! a parameter because [`crate::scanner::FileUnit`] deliberately carries no
//! bytes and IR paths are repo-relative POSIX, meaningless without the
//! anchor for disk re-reads. The caller threads its own root (the CLI passes
//! its scan target's canonicalization; tests pass theirs) — no
//! process-global state, so concurrent in-process scans resolve against
//! their own roots. There is no shared mutable state anywhere on this path:
//! `clang_sys::load()` installs the libclang handle in thread-local storage
//! (per-thread, idempotent) and every translation unit is parsed under a
//! fresh per-call `CXIndex` (created and disposed inside
//! [`parse_unit_maps`]), so parallel `resolve` calls are independent.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    CallSemantic, CalleeKind, Col, Line, ParseStatus, ResolvedSite, ResolverHealth, ResolverTier,
    ShapeKind, Span,
};
use crate::scanner::FileUnit;

// ---------------------------------------------------------------------------
// Pure map-building core (NO clang-sys dependency — parser-free testable,
// ADR-0001 guardrail 2 in spirit: the seam's data structures are engine-free)
// ---------------------------------------------------------------------------

/// A function-definition site observed by the parser: internal only, keyed by
/// USR in [`DefinitionMap`]. Never serialized (USR rule, module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionCandidate {
    /// Clang USR — INTERNAL pairing key only (ADR-0001 identity rule).
    pub usr: String,
    /// Repo-relative POSIX path of the DEFINING file.
    pub path: String,
    /// Point span at the definition's start (1-based line/col, IR convention
    /// ir-v1.md §2).
    pub span: Span,
}

/// A call expression's resolution: internal only. Never serialized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallReference {
    /// Repo-relative POSIX path of the file containing the call.
    pub path: String,
    /// 1-based line of the call expression's start.
    pub start_line: Line,
    /// 1-based byte column of the call expression's start.
    pub start_col: Col,
    /// USR of the referenced cursor — INTERNAL pairing key only.
    pub usr: String,
}

/// Resolved definition site carried in [`DefinitionMap`]. Deliberately NOT
/// `Serialize` (USR-keyed map must never reach the wire).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionSite {
    pub path: String,
    pub span: Span,
}

/// USR → definition site. USR is an INTERNAL key (ADR-0001 identity rule):
/// never output. Duplicate USRs (e.g. inline definitions in headers across
/// TUs) keep the lexicographically smallest `(path, span)` — deterministic.
pub type DefinitionMap = BTreeMap<String, DefinitionSite>;

/// Call-expression location → referenced-cursor USR. Keyed by
/// `(path, start_line, start_col)` (IR 1-based convention); the USR value is
/// INTERNAL (never output). Deliberately NOT `Serialize`.
pub type CallRefMap = BTreeMap<(String, Line, Col), String>;

/// Build the definition map from raw candidates. Deterministic: candidates
/// are sorted by `(usr, path, span)` and the smallest `(path, span)` wins per
/// USR regardless of input order.
#[must_use]
pub fn build_definition_map(candidates: &[DefinitionCandidate]) -> DefinitionMap {
    let mut sorted: Vec<&DefinitionCandidate> = candidates.iter().collect();
    sorted.sort_by(|a, b| (&a.usr, &a.path, &a.span).cmp(&(&b.usr, &b.path, &b.span)));
    let mut map = DefinitionMap::new();
    for candidate in sorted {
        map.entry(candidate.usr.clone())
            .or_insert_with(|| DefinitionSite {
                path: candidate.path.clone(),
                span: candidate.span,
            });
    }
    map
}

/// Build the call-reference map from raw references. Deterministic: the key
/// `(path, start_line, start_col)` is unique per call site; re-insertion of
/// an identical reference is idempotent.
#[must_use]
pub fn build_call_reference_map(references: &[CallReference]) -> CallRefMap {
    let mut map = CallRefMap::new();
    for reference in references {
        map.insert(
            (
                reference.path.clone(),
                reference.start_line,
                reference.start_col,
            ),
            reference.usr.clone(),
        );
    }
    map
}

// ---------------------------------------------------------------------------
// Wave B2 — per-function fact maps + extended MUTATION pass (pure,
// parser-free, deterministic)
// ---------------------------------------------------------------------------

/// The fopen family (Wave B2): constructors proving heap-backed `FILE *`
/// handles. Single source of truth for the resolver's FOPEN-SET collection
/// (consumed by the `semantic-clang` visitor's fopen-call probe; unused
/// without the feature — hence the scoped `allow`).
#[cfg_attr(not(feature = "semantic-clang"), allow(dead_code))]
pub(crate) const FOPEN_FAMILY: [&str; 3] = ["fopen", "freopen", "fdopen"];

/// Per-function semantic facts proven by the libclang walk (Wave B2).
/// Internal only — deliberately NOT `Serialize` (same hygiene as the B1
/// maps: facts reach output exclusively through heuristic findings and
/// their evidence, never as raw maps).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FunctionFacts {
    /// Local names proven fopen-assigned: a Direct fopen-family call sits on
    /// the RHS of the variable's initializer (`= fopen(...)`) or of a plain
    /// `=` assignment (`x = fopen(...)`) in the same function body, with the
    /// belt-and-braces FILE-type rule applied at collection (see the
    /// visitor docs).
    pub fopen_vars: BTreeSet<String>,
    /// Local name → resolved string-literal VALUE (quotes already removed):
    /// names initialized with a SINGLE string-literal token and proven
    /// single-write (exactly one DeclStmt-with-init or `=` assignment in
    /// the function body).
    pub literals: BTreeMap<String, String>,
}

/// Enclosing-function key → [`FunctionFacts`], keyed by
/// `(defining_file, function_name)` — the B1 matcher contract
/// ([`locate_def_record`]): C forbids two definitions of one name per file,
/// so the pair is unique per definition. Deterministic BTreeMap discipline
/// throughout; cross-TU merges union member-wise with literal values
/// first-wins in sorted-path visit order (see [`merge_function_facts`]).
/// Deliberately NOT `Serialize` (internal facts map).
pub type FunctionFactsMap = BTreeMap<(String, String), FunctionFacts>;

/// Merge one TU's function facts into the accumulator (deterministic:
/// `resolve` visits files in sorted-path order; member-wise union with
/// literal values first-wins keeps repeats — e.g. header-inline functions
/// parsed in several TUs — idempotent).
///
/// Public as a test seam (same precedent as [`build_definition_map`]).
pub fn merge_function_facts(into: &mut FunctionFactsMap, from: FunctionFactsMap) {
    for (key, facts) in from {
        let slot = into.entry(key).or_default();
        slot.fopen_vars.extend(facts.fopen_vars);
        for (name, value) in facts.literals {
            slot.literals.entry(name).or_insert(value);
        }
    }
}

// ---------------------------------------------------------------------------
// Wave B1 — conservative MUTATION pass (pure, parser-free, deterministic)
// ---------------------------------------------------------------------------

/// Number of call events enriched by [`apply_resolutions`] (test/diagnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EnrichmentStats {
    pub calls_enriched: u32,
    pub records_upgraded: u32,
    /// Wave B2: `fclose` calls proven fopen-handled.
    pub fopen_handles_proven: u32,
    /// Wave B2: scanf-family format args resolved to literals.
    pub literal_args_resolved: u32,
}

/// One planned mutation, decided under an immutable borrow (phase 1) and
/// applied afterwards (phase 2) — two-phase so lookups can read the whole
/// stream while mutations land.
struct PlannedMutation {
    unit: usize,
    function: usize,
    call: usize,
    /// Wave B1 resolved site (cross-file USR pairing), when proven.
    site: Option<ResolvedSite>,
    /// Wave B2: proven fopen-handle `fclose`.
    fopened: bool,
    /// Wave B2: resolved `(format_arg_index, literal_value)` pairs.
    literals: Vec<(u32, String)>,
    /// `(unit_index, function_index)` of the definition's FunctionRecord,
    /// when one was located by the documented matcher contract.
    def_record: Option<(usize, usize)>,
}

/// Locate a definition's FunctionRecord for a resolved site.
///
/// MATCHER CONTRACT (documented, plan `semantic-resolver` Wave B1): libclang
/// extents and tree-sitter spans are derived from different grammars and can
/// differ by whitespace/newlines, so span equality is NOT the primary key.
/// Primary key is `(site.path, callee_name)` — robust because C forbids two
/// definitions of the same function name in one file. The span is a SANITY
/// check only: among the (path, name) candidates (at most one definition per
/// file; declarations share the name but the resolver map holds definitions),
/// prefer one whose span CONTAINS the definition start point; if none
/// contains it (engine span drift), the (path, name) match still stands.
/// Returns the record index plus whether the record may be tier-upgraded
/// (its own span does not overlap its file's error regions).
fn locate_def_record(
    units: &[FileUnit],
    def_path: &str,
    callee_name: &str,
    def_start: Span,
) -> Option<(usize, usize, bool)> {
    let (unit_index, unit) = units
        .iter()
        .enumerate()
        .find(|(_, u)| u.file.path == def_path)?;
    for (function_index, f) in unit.functions.iter().enumerate() {
        if f.record.name != callee_name {
            continue;
        }
        let contains = (f.record.span.start_line, f.record.span.start_col)
            <= (def_start.start_line, def_start.start_col)
            && (def_start.start_line, def_start.start_col)
                <= (f.record.span.end_line, f.record.span.end_col);
        if !contains {
            continue;
        }
        let clean = !unit
            .parse_health
            .error_regions
            .iter()
            .any(|region| region.intersects(&f.record.span));
        return Some((unit_index, function_index, clean));
    }
    // No candidate contains the point (engine span drift): the (path, name)
    // match is still the contract — take the first, but mark it span-unverified
    // (still eligibility-checked against error regions).
    unit.functions.iter().enumerate().find_map(|(fi, f)| {
        (f.record.name == callee_name).then(|| {
            let clean = !unit
                .parse_health
                .error_regions
                .iter()
                .any(|region| region.intersects(&f.record.span));
            (unit_index, fi, clean)
        })
    })
}

/// Apply conservative semantic enrichment to the IR stream from prebuilt
/// maps (pure: no parser, no I/O — parser-free testable per ADR-0001
/// guardrail 2).
///
/// Shared gates, applied first to EVERY call (ALL conservative drops — a
/// gated call keeps its syntactic truth untouched):
/// 1. file `ParseHealth.status == Failed` → skip the whole file;
/// 2. `macro_origin` events → skip (macro-expanded spans can mis-attribute);
/// 3. call span overlapping the file's `error_regions` → skip (same overlap
///    semantics as the analyzer's `apply_downgrade_hooks`).
///
/// Wave B1 resolution (needs gates 4–5 on top):
/// 4. unresolved call site / unknown USR → skip;
/// 5. non-cross-file (same file) or non-internal site (system headers are
///    absolute paths outside the scanned set) → skip.
///
/// Wave B2 facts (same-file, per-enclosing-function — gates 1–3 already
/// passed, no cross-file requirement):
/// - FOPEN: a Direct `fclose` call at `in_loop_depth ≥ 1` whose arg0 is an
///   `Identifier` naming a member of the enclosing function's FOPEN SET →
///   `call.semantic.fopened_handle = true`;
/// - LITERAL: a Direct scanf-family call (family + format-arg index per
///   [`crate::analyzer::scanf_format_index`]) whose format arg is a
///   non-`Literal` `Identifier` naming a member of the enclosing function's
///   LITERAL MAP → `(format_idx, literal_value)` pushed into
///   `call.semantic.literal_args`.
///
/// Mutations, ONLY these field writes:
/// - `call.semantic = Some(...)` merging every proven part
///   (`resolved_definition`, `fopened_handle`, `literal_args`);
/// - the definition's `FunctionRecord.resolver_tier = Semantic`, ONLY when
///   the record's own span does not overlap its file's error regions.
///
/// INVARIANT (the analyzer's `apply_upgrade_hooks` gate depends on it):
/// `Some(CallSemantic)` is set ONLY when at least one fact is proven —
/// never an empty struct.
///
/// Determinism: units are visited in the given (scanner-sorted) order,
/// events in existing vec order; all lookups are BTreeMap-keyed; literal
/// pairs are pushed in ascending arg-index order.
#[must_use]
pub fn apply_resolutions(
    units: &mut [FileUnit],
    definitions: &DefinitionMap,
    call_refs: &CallRefMap,
    function_facts: &FunctionFactsMap,
) -> EnrichmentStats {
    let internal: BTreeSet<&str> = units.iter().map(|u| u.file.path.as_str()).collect();

    // Phase 1 — decide under an immutable borrow.
    let mut planned: Vec<PlannedMutation> = Vec::new();
    for (unit_index, unit) in units.iter().enumerate() {
        if unit.parse_health.status == ParseStatus::Failed {
            continue;
        }
        let file_path = unit.file.path.as_str();
        for (function_index, f) in unit.functions.iter().enumerate() {
            let fn_facts = function_facts.get(&(unit.file.path.clone(), f.record.name.clone()));
            for (call_index, call) in f.calls.iter().enumerate() {
                if call.meta.macro_origin {
                    continue;
                }
                if unit
                    .parse_health
                    .error_regions
                    .iter()
                    .any(|region| region.intersects(&call.meta.span))
                {
                    continue;
                }
                // Wave B1 resolution (gates 4–5).
                let mut site = None;
                let mut def_record = None;
                let key = (
                    unit.file.path.clone(),
                    call.meta.span.start_line,
                    call.meta.span.start_col,
                );
                if let Some(usr) = call_refs.get(&key)
                    && let Some(def) = definitions.get(usr)
                    && def.path != file_path
                    && internal.contains(def.path.as_str())
                {
                    def_record = locate_def_record(units, &def.path, &call.callee_name, def.span)
                        .filter(|(_, _, clean)| *clean)
                        .map(|(u, f, _)| (u, f));
                    site = Some(ResolvedSite {
                        path: def.path.clone(),
                        span: def.span,
                    });
                }
                // Wave B2 facts (same-file, per-enclosing-function).
                let mut fopened = false;
                let mut literals: Vec<(u32, String)> = Vec::new();
                if let Some(facts) = fn_facts {
                    if !facts.fopen_vars.is_empty()
                        && call.callee_kind == CalleeKind::Direct
                        && call.callee_name == "fclose"
                        && call.meta.in_loop_depth >= 1
                        && call.args.first().is_some_and(|a| {
                            a.kind == ShapeKind::Identifier
                                && a.name
                                    .as_deref()
                                    .is_some_and(|n| facts.fopen_vars.contains(n))
                        })
                    {
                        fopened = true;
                    }
                    if call.callee_kind == CalleeKind::Direct
                        && let Some(format_idx) =
                            crate::analyzer::scanf_format_index(&call.callee_name)
                        && let Some(arg) = call.args.get(format_idx)
                        && arg.kind == ShapeKind::Identifier
                        && let Some(name) = arg.name.as_deref()
                        && let Some(value) = facts.literals.get(name)
                    {
                        literals.push((format_idx as u32, value.clone()));
                    }
                }
                // INVARIANT: never an empty struct.
                if site.is_none() && !fopened && literals.is_empty() {
                    continue;
                }
                planned.push(PlannedMutation {
                    unit: unit_index,
                    function: function_index,
                    call: call_index,
                    site,
                    fopened,
                    literals,
                    def_record,
                });
            }
        }
    }

    // Phase 2 — apply.
    let mut stats = EnrichmentStats::default();
    for mutation in planned {
        let unit = &mut units[mutation.unit];
        let call = &mut unit.functions[mutation.function].calls[mutation.call];
        call.semantic = Some(CallSemantic {
            resolved_definition: mutation.site,
            fopened_handle: mutation.fopened,
            literal_args: mutation.literals.clone(),
        });
        stats.calls_enriched += 1;
        if mutation.fopened {
            stats.fopen_handles_proven += 1;
        }
        stats.literal_args_resolved += mutation.literals.len() as u32;
        if let Some((def_unit, def_function)) = mutation.def_record {
            units[def_unit].functions[def_function].record.resolver_tier = ResolverTier::Semantic;
            stats.records_upgraded += 1;
        }
    }
    stats
}

// ---------------------------------------------------------------------------
// Pipeline entry — thin I/O wrapper; all decisions live in the layers above
// ---------------------------------------------------------------------------

#[cfg(not(feature = "semantic-clang"))]
pub fn resolve(units: &mut [crate::scanner::FileUnit], root: &std::path::Path) {
    // Honest tier reporting in ALL builds (modules.md §4.3): without the
    // `semantic-clang` feature the resolver is absent by definition. The
    // explicit root is unused on this path (no disk re-reads happen).
    let _ = root;
    for unit in units {
        unit.parse_health.resolver = Some(ResolverHealth::Absent);
    }
}

#[cfg(feature = "semantic-clang")]
pub fn resolve(units: &mut [crate::scanner::FileUnit], root: &std::path::Path) {
    // Lazy dlopen (clang-sys `runtime` mode): graceful absence is a
    // first-class path — never a crash, never a degraded parse. Pipe-clean
    // stderr, no color (cli conventions §8).
    if let Err(err) = clang_sys::load() {
        eprintln!("perfpal: warning: libclang unavailable, semantic resolver disabled ({err})");
        for unit in units {
            unit.parse_health.resolver = Some(ResolverHealth::Absent);
        }
        return;
    }

    // Determinism (modules.md §4.3): process files sorted by repo-relative
    // path. The scanner already emits lexicographic order; this is the
    // resolver's own guarantee, independent of caller input order.
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by(|&a, &b| units[a].file.path.cmp(&units[b].file.path));

    // Pass 1 — parse every TU, collect health + per-file maps (immutable
    // over the units' heuristic-visible state). Maps merge across files:
    // files are visited in sorted-path order, so a duplicate USR (inline
    // definitions in headers across TUs) keeps the smallest defining path —
    // consistent with [`build_definition_map`]'s ambiguity rule — and
    // function facts union member-wise ([`merge_function_facts`]).
    let mut definitions: DefinitionMap = DefinitionMap::new();
    let mut call_refs: CallRefMap = CallRefMap::new();
    let mut function_facts: FunctionFactsMap = FunctionFactsMap::new();
    for &index in &order {
        let unit = &mut units[index];
        let rel = unit.file.path.clone();
        let abs = root.join(&rel);

        let health = match std::fs::read(&abs) {
            // Re-read from disk (FileUnit carries no bytes; the caller
            // threads the explicit scan root — module docs).
            Ok(source) => match parse_unit_maps(&rel, &abs, &source, root) {
                Some(maps) => {
                    for (usr, site) in maps.definitions {
                        definitions.entry(usr).or_insert(site);
                    }
                    call_refs.extend(maps.call_refs);
                    merge_function_facts(&mut function_facts, maps.function_facts);
                    Some(ResolverHealth::Ok)
                }
                None => Some(ResolverHealth::Failed),
            },
            // The file went unreadable between scan and resolve: honest
            // failure — never silently Absent, never an abort. Syntactic
            // results are untouched either way (enrich-only, never delete).
            Err(_) => Some(ResolverHealth::Failed),
        };
        unit.parse_health.resolver = health;
    }

    // Pass 2 — conservative mutation of the merged IR stream (Waves B1+B2):
    // call facts + definition-record tier upgrades, gated exactly as
    // documented on [`apply_resolutions`].
    let _stats = apply_resolutions(units, &definitions, &call_refs, &function_facts);
}

// ---------------------------------------------------------------------------
// libclang backend (cfg(feature)) — thin FFI wrapper over the pure core
// ---------------------------------------------------------------------------

/// One translation unit's parsed maps (feature `semantic-clang` only).
/// Deliberately NOT `Serialize` (it carries the internal USR-keyed maps —
/// ADR-0001 identity rule).
#[cfg(feature = "semantic-clang")]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedMaps {
    pub definitions: DefinitionMap,
    pub call_refs: CallRefMap,
    pub function_facts: FunctionFactsMap,
}

/// Parse one translation unit and build the internal maps (feature
/// `semantic-clang` only).
///
/// - Parse options: `CXTranslationUnit_KeepGoing` — a non-compiling TU (our
///   core case, ADR-0001) still yields a best-effort AST; TU-level failure
///   codes (`CXError_Failure`/`Crashed`/`ASTReadError`/…) map to `None` →
///   [`crate::ir::ResolverHealth::Failed`]. Syntactic results are NEVER
///   deleted or degraded on that path (conservative direction).
/// - `source` is fed via `CXUnsavedFile` so the parse sees EXACTLY the bytes
///   re-read from disk (byte-determinism, modules.md §6).
/// - Default (empty) compile args; no compile DB is assumed (ADR-0001).
/// - `rel_path` is the repo-relative POSIX path used as the map key for the
///   main file; locations inside other files (headers) are relativized
///   against the scan root the same way.
///
/// Public as a test seam; Wave B consumers should go through `resolve`.
#[cfg(feature = "semantic-clang")]
pub fn parse_unit_maps(
    rel_path: &str,
    abs_path: &std::path::Path,
    source: &[u8],
    root: &std::path::Path,
) -> Option<ParsedMaps> {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_ulong};

    // `rel_path` names the main file for map keys; locations inside the main
    // file relativize to it via the root prefix strip below. Header locations
    // relativize against `root` directly.
    let _ = rel_path;

    // SAFETY: clang_sys::load() must have succeeded before this runs (the
    // contract of `resolve`; tests assert it via `load()` first). All clang
    // strings/pointers passed here are pinned for each call's duration; the
    // TU handle (if any) is disposed before returning.
    unsafe {
        let index = clang_sys::clang_createIndex(0, 0);
        if index.is_null() {
            return None;
        }

        let parsed = (|| {
            {
                let Ok(filename) = CString::new(abs_path.to_string_lossy().as_bytes()) else {
                    return None;
                };

                // CXUnsavedFile: contents must be NUL-terminated; Length
                // EXCLUDES the terminator (libclang requirement).
                let mut contents = source.to_vec();
                contents.push(0);
                let mut unsaved = clang_sys::CXUnsavedFile {
                    Filename: filename.as_ptr(),
                    Contents: contents.as_ptr().cast::<c_char>(),
                    Length: source.len() as c_ulong,
                };

                let mut tu: clang_sys::CXTranslationUnit = std::ptr::null_mut();
                // CXTranslationUnit_KeepGoing = 0x200 (libclang ≥ 3.9):
                // continue parsing past TU-level errors so non-compiling TUs
                // still yield a best-effort AST (ADR-0001 core case). The
                // constant lives behind clang-sys's optional `clang_3_9`
                // cfg-feature, which the pinned dependency deliberately does
                // NOT enable — the raw value is ABI-stable, so it is pinned
                // here with its provenance instead.
                const CX_TRANSLATION_UNIT_KEEP_GOING: std::os::raw::c_int = 0x200;
                let code = clang_sys::clang_parseTranslationUnit2(
                    index,
                    filename.as_ptr(),
                    std::ptr::null(), // no compile args: default, no compile DB (ADR-0001)
                    0,
                    &mut unsaved,
                    1,
                    CX_TRANSLATION_UNIT_KEEP_GOING,
                    &mut tu,
                );
                if code != clang_sys::CXError_Success || tu.is_null() {
                    return None;
                }

                let mut collector = Collector {
                    root: root.to_path_buf(),
                    tu,
                    definitions: Vec::new(),
                    references: Vec::new(),
                    fn_stack: Vec::new(),
                    function_facts: FunctionFactsMap::new(),
                };
                let cursor = clang_sys::clang_getTranslationUnitCursor(tu);
                let data = std::ptr::addr_of_mut!(collector).cast();
                clang_sys::clang_visitChildren(cursor, visit, data);

                clang_sys::clang_disposeTranslationUnit(tu);
                Some(ParsedMaps {
                    definitions: build_definition_map(&collector.definitions),
                    call_refs: build_call_reference_map(&collector.references),
                    function_facts: collector.function_facts,
                })
            }
        })();

        clang_sys::clang_disposeIndex(index);
        parsed
    }
}

#[cfg(feature = "semantic-clang")]
struct Collector {
    root: std::path::PathBuf,
    /// Active TU handle — token/type queries need it; valid for the whole
    /// traversal (disposed after).
    tu: clang_sys::CXTranslationUnit,
    definitions: Vec<DefinitionCandidate>,
    references: Vec<CallReference>,
    /// Enclosing-function scope stack (Wave B2): pushed on function
    /// definitions, popped after their subtrees. Empty at file scope —
    /// file-scope `VarDecl`s are never body locals.
    fn_stack: Vec<FnScope>,
    function_facts: FunctionFactsMap,
}

/// Per-function collection state: one scope per enclosing function
/// definition on the visitor's scope stack (Wave B2). Name-granular (blocks
/// are NOT distinguished): a shadowed re-declaration simply raises the
/// write count, which fails the single-write proof conservatively.
#[cfg(feature = "semantic-clang")]
#[derive(Debug, Default)]
struct FnScope {
    /// Function name (cursor spelling) — map key with the defining path.
    name: String,
    /// Repo-relative POSIX path of the defining file — map key.
    path: String,
    /// Declared local name → libclang type spelling (`VarDecl` only;
    /// parameters are `ParmDecl` and never enter this table).
    var_types: BTreeMap<String, String>,
    /// Name → total write count: DeclStmt-with-init counts 1,
    /// declaration-without-init 0, plain `=` assignment 1, compound
    /// `=`-family assignment 1, prefix/postfix `++`/`--` 1 (the last two
    /// are a conservative extension of the "one DeclStmt-with-init or `=`
    /// assignment" rule — any second write shape fails the proof).
    writes: BTreeMap<String, u32>,
    /// Name → candidate literal value from the write, when that write was
    /// a single-string-literal write (decl-init or plain-`=` RHS
    /// tokenizing to exactly one string-literal token, no calls inside).
    /// Admitted to the LITERAL MAP only when `writes[name] == 1`.
    write_values: BTreeMap<String, String>,
    /// Names with at least one Direct fopen-family call on a write RHS
    /// (decl-init or plain-`=` assignment). Admitted to the FOPEN SET
    /// subject to the FILE-type rule (see [`Collector::leave_function`]).
    fopen_writes: BTreeSet<String>,
}

/// Recursive visitor: collects function-definition candidates and call
/// references, then recurses. `CXChildVisit_Continue` after self-recursion
/// walks the full AST deterministically (clang visits children in source
/// order).
///
/// Wave B2 scoping: function definitions push an [`FnScope`] before their
/// subtree is visited and pop it after, so per-cursor facts
/// (`VarDecl`/assignments) attribute to the enclosing function body.
#[cfg(feature = "semantic-clang")]
extern "C" fn visit(
    cursor: clang_sys::CXCursor,
    _parent: clang_sys::CXCursor,
    data: clang_sys::CXClientData,
) -> clang_sys::CXChildVisitResult {
    // SAFETY: `data` always points at the `Collector` boxed by the caller for
    // the duration of this traversal.
    let collector = unsafe { &mut *(data as *mut Collector) };
    // SAFETY: `cursor` is a live cursor handed out by clang's own visitor.
    let is_fn_def = unsafe {
        clang_sys::clang_getCursorKind(cursor) == clang_sys::CXCursor_FunctionDecl
            && clang_sys::clang_isCursorDefinition(cursor) != 0
    };
    if is_fn_def {
        unsafe { collector.enter_function(cursor) };
    } else {
        unsafe { collector.record(cursor) };
    }
    unsafe { clang_sys::clang_visitChildren(cursor, visit, data) };
    if is_fn_def {
        collector.leave_function();
    }
    clang_sys::CXChildVisit_Continue
}

#[cfg(feature = "semantic-clang")]
impl Collector {
    /// Begin a function-definition scope: records the B1 definition
    /// candidate and pushes a fresh [`FnScope`] for the body's facts.
    unsafe fn enter_function(&mut self, cursor: clang_sys::CXCursor) {
        // `enter_function` is `unsafe fn` for call-site uniformity with the
        // visitor; the location query is the one raw-FFI call here.
        let usr = usr_of(cursor);
        // SAFETY: live cursor handed out by clang's own visitor.
        let (path, line, col) = unsafe { self.location(cursor) };
        let name = spelling_of(cursor);
        if !usr.is_empty() {
            self.definitions.push(DefinitionCandidate {
                usr,
                path: path.clone(),
                span: Span::point(line, col),
            });
        }
        self.fn_stack.push(FnScope {
            name,
            path,
            ..FnScope::default()
        });
    }

    /// Close the current function scope, finalizing its facts.
    ///
    /// - FOPEN SET = fopen-written names whose DECLARED type spelling
    ///   contains `"FILE"`, or whose type is unobtainable (no in-scope
    ///   declaration — parameters, globals, undeclared names — where
    ///   assignment-from-fopen alone suffices, per contract). A declared
    ///   NON-`FILE` type vetoes (belt-and-braces precedence: known type
    ///   beats the call shape).
    /// - LITERAL MAP = single-write names (`writes[name] == 1`) with a
    ///   proven single-literal value.
    ///
    /// Scopes proving nothing store nothing — the map holds proven facts
    /// only (the mutation pass's `Some`-invariant starts here).
    fn leave_function(&mut self) {
        let Some(scope) = self.fn_stack.pop() else {
            return;
        };
        let mut facts = FunctionFacts::default();
        for name in &scope.fopen_writes {
            let vetoed = scope
                .var_types
                .get(name)
                .is_some_and(|t| !t.contains("FILE"));
            if !vetoed {
                facts.fopen_vars.insert(name.clone());
            }
        }
        for (name, value) in &scope.write_values {
            if scope.writes.get(name).copied().unwrap_or(0) == 1 {
                facts.literals.insert(name.clone(), value.clone());
            }
        }
        if facts.fopen_vars.is_empty() && facts.literals.is_empty() {
            return;
        }
        let slot = self
            .function_facts
            .entry((scope.path, scope.name))
            .or_default();
        slot.fopen_vars.extend(facts.fopen_vars);
        for (name, value) in facts.literals {
            slot.literals.entry(name).or_insert(value);
        }
    }

    unsafe fn record(&mut self, cursor: clang_sys::CXCursor) {
        // SAFETY: cursor is a live cursor handed out by clang's own visitor.
        unsafe {
            let kind = clang_sys::clang_getCursorKind(cursor);
            if kind == clang_sys::CXCursor_CallExpr {
                // B1 call-reference collection (unchanged).
                let referenced = clang_sys::clang_getCursorReferenced(cursor);
                if clang_sys::clang_Cursor_isNull(referenced) == 0 {
                    let usr = usr_of(referenced);
                    if !usr.is_empty() {
                        let (path, line, col) = self.location(cursor);
                        self.references.push(CallReference {
                            path,
                            start_line: line,
                            start_col: col,
                            usr,
                        });
                    }
                }
            } else if !self.fn_stack.is_empty() {
                // Wave B2 per-function facts (file-scope cursors — empty
                // stack — are never body locals).
                if kind == clang_sys::CXCursor_VarDecl {
                    self.record_var_decl(cursor);
                } else if kind == clang_sys::CXCursor_BinaryOperator {
                    self.record_assignment(cursor);
                } else if kind == clang_sys::CXCursor_UnaryOperator {
                    self.record_incdec(cursor);
                }
            }
        }
    }

    /// A local declaration: records the declared type and, when an
    /// initializer is present, counts one write plus its fopen/literal
    /// candidacy. The initializer is the LAST direct child (attributes, if
    /// any, precede it in visitation order); childless declarations are
    /// bare (`int x;`) — not writes.
    unsafe fn record_var_decl(&mut self, cursor: clang_sys::CXCursor) {
        // Safe wrappers only (see `enter_function`).
        let name = spelling_of(cursor);
        if name.is_empty() {
            return;
        }
        let tu = self.tu;
        if let Some(type_spelling) = type_spelling_of(cursor)
            && let Some(scope) = self.fn_stack.last_mut()
        {
            scope.var_types.insert(name.clone(), type_spelling);
        }
        let probe = probe_subtree(tu, cursor);
        let Some((_, init_range)) = probe.direct.last().copied() else {
            return;
        };
        let Some(scope) = self.fn_stack.last_mut() else {
            return;
        };
        *scope.writes.entry(name.clone()).or_default() += 1;
        if probe.has_fopen_call {
            scope.fopen_writes.insert(name.clone());
        }
        if !probe.has_call
            && let Some(value) = single_string_literal(tu, init_range)
        {
            scope.write_values.insert(name.clone(), value);
        }
    }

    /// A binary-operator expression: only `=`-family operators write.
    /// Plain `=` on a bare-identifier LHS (`x = <rhs>`) counts one write
    /// with fopen/literal candidacy from the RHS subtree; compound
    /// `=`-family operators invalidate without proving. Comparisons
    /// (`==`, `!=`, …) and member/subscript/deref LHS shapes (`p->x`,
    /// `a[i]`, `*p`) are neither proven nor invalidated (documented
    /// limitation — the proof stays name-granular).
    ///
    /// Operator detection tokenizes the WHOLE operator extent: the LHS gate
    /// above proves the first token is the bare target name, so the second
    /// token IS the operator. (The gap-range alternative — LHS-end to
    /// RHS-start — overshoots into implicit-cast RHS extents, which
    /// libclang reports degenerately; whole-extent tokens are exact here.)
    unsafe fn record_assignment(&mut self, cursor: clang_sys::CXCursor) {
        // Safe wrappers only (see `enter_function`).
        let tu = self.tu;
        let probe = probe_subtree(tu, cursor);
        if probe.direct.len() < 2 {
            return;
        }
        let (lhs_cursor, _) = probe.direct[0];
        let (rhs_cursor, _) = probe.direct[probe.direct.len() - 1];
        // SAFETY: live cursor handed out by clang's own traversal.
        let lhs_kind = unsafe { clang_sys::clang_getCursorKind(lhs_cursor) };
        if lhs_kind != clang_sys::CXCursor_DeclRefExpr {
            return;
        }
        let target = spelling_of(lhs_cursor);
        if target.is_empty() {
            return;
        }
        let tokens = tokenize(tu, extent_of(cursor));
        if tokens.len() < 3 || tokens[0].1 != target {
            return;
        }
        let is_plain = tokens[1].1 == "=";
        if !is_plain && !is_compound_assign(Some(tokens[1].1.as_str())) {
            return;
        }
        let Some(scope) = self.fn_stack.last_mut() else {
            return;
        };
        *scope.writes.entry(target.clone()).or_default() += 1;
        if !is_plain {
            return;
        }
        let rhs_probe = probe_subtree(tu, rhs_cursor);
        if rhs_probe.has_fopen_call {
            scope.fopen_writes.insert(target.clone());
        }
        if !rhs_probe.has_call
            && let Some(value) = unquote_single_token(&tokens[2..])
        {
            scope.write_values.insert(target.clone(), value);
        }
    }

    /// Prefix/postfix `++`/`--` on a bare identifier: a write with no
    /// provable value (invalidates the single-write proof). Other unary
    /// shapes (`*p`, `&x`, `-x`, `!c`) never reach the counter.
    unsafe fn record_incdec(&mut self, cursor: clang_sys::CXCursor) {
        // SAFETY: live cursor; nested traversals pin their own data.
        unsafe {
            let tu = self.tu;
            let tokens = tokenize(tu, extent_of(cursor));
            let edge = tokens
                .first()
                .map(|(_, s)| s.as_str())
                .or(tokens.last().map(|(_, s)| s.as_str()));
            if !matches!(edge, Some("++" | "--")) {
                return;
            }
            let probe = probe_subtree(tu, cursor);
            let Some(target) = probe
                .direct
                .iter()
                .find(|(c, _)| {
                    clang_sys::clang_getCursorKind(*c) == clang_sys::CXCursor_DeclRefExpr
                })
                .map(|(c, _)| spelling_of(*c))
            else {
                return;
            };
            if target.is_empty() {
                return;
            }
            if let Some(scope) = self.fn_stack.last_mut() {
                *scope.writes.entry(target).or_default() += 1;
            }
        }
    }

    /// File path (repo-relative POSIX), 1-based line and 1-based byte column
    /// of the cursor's extent start (IR convention, ir-v1.md §2 — libclang
    /// lines/columns are 1-based byte-based already).
    unsafe fn location(&self, cursor: clang_sys::CXCursor) -> (String, u32, u32) {
        // SAFETY: live cursor; out-params are pinned.
        unsafe {
            let range = clang_sys::clang_getCursorExtent(cursor);
            let location = clang_sys::clang_getRangeStart(range);
            let mut file: clang_sys::CXFile = std::ptr::null_mut();
            let mut line: std::os::raw::c_uint = 0;
            let mut col: std::os::raw::c_uint = 0;
            let mut offset: std::os::raw::c_uint = 0;
            clang_sys::clang_getFileLocation(location, &mut file, &mut line, &mut col, &mut offset);
            let raw = if file.is_null() {
                String::new()
            } else {
                cxstring_to_string(clang_sys::clang_getFileName(file)).unwrap_or_default()
            };
            let path = std::path::Path::new(&raw)
                .strip_prefix(&self.root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or(raw);
            (path, line, col)
        }
    }
}

// ---------------------------------------------------------------------------
// Wave B2 libclang probes (feature `semantic-clang` only): subtree walks,
// token queries, and spelling/type helpers feeding FnScope collection.
// ---------------------------------------------------------------------------

/// One nested-subtree walk result: call flags over the root itself plus
/// all descendants, and the root's DIRECT children (cursor + extent) for
/// initializer/RHS/operand slicing.
#[cfg(feature = "semantic-clang")]
struct SubtreeProbe {
    root: clang_sys::CXCursor,
    has_call: bool,
    has_fopen_call: bool,
    direct: Vec<(clang_sys::CXCursor, clang_sys::CXSourceRange)>,
}

/// Nested visitor feeding [`probe_subtree`]: records direct children
/// (`parent == root` via cursor equality) and flags every descendant
/// `CallExpr`. Returns `Recurse` so the whole subtree is covered.
#[cfg(feature = "semantic-clang")]
extern "C" fn probe_visit(
    cursor: clang_sys::CXCursor,
    parent: clang_sys::CXCursor,
    data: clang_sys::CXClientData,
) -> clang_sys::CXChildVisitResult {
    // SAFETY: `data` points at the caller's `SubtreeProbe` for the
    // duration of this nested traversal.
    let probe = unsafe { &mut *(data as *mut SubtreeProbe) };
    // SAFETY: live cursors from clang's own traversal.
    unsafe {
        if clang_sys::clang_equalCursors(parent, probe.root) != 0 {
            probe.direct.push((cursor, extent_of(cursor)));
        }
        if clang_sys::clang_getCursorKind(cursor) == clang_sys::CXCursor_CallExpr {
            probe.has_call = true;
            if is_fopen_call(cursor) {
                probe.has_fopen_call = true;
            }
        }
    }
    clang_sys::CXChildVisit_Recurse
}

/// Walk `root` (itself + all descendants) collecting call flags and direct
/// children in source order (deterministic: clang visits in order).
#[cfg(feature = "semantic-clang")]
fn probe_subtree(tu: clang_sys::CXTranslationUnit, root: clang_sys::CXCursor) -> SubtreeProbe {
    let _ = tu;
    let mut probe = SubtreeProbe {
        root,
        has_call: false,
        has_fopen_call: false,
        direct: Vec::new(),
    };
    // SAFETY: `root` is a live cursor; `probe` outlives the traversal.
    unsafe {
        if clang_sys::clang_getCursorKind(root) == clang_sys::CXCursor_CallExpr {
            probe.has_call = true;
            if is_fopen_call(root) {
                probe.has_fopen_call = true;
            }
        }
        let data = std::ptr::addr_of_mut!(probe).cast();
        clang_sys::clang_visitChildren(root, probe_visit, data);
    }
    probe
}

/// Extent of a cursor (caller-owned `Copy` range).
#[cfg(feature = "semantic-clang")]
fn extent_of(cursor: clang_sys::CXCursor) -> clang_sys::CXSourceRange {
    // SAFETY: live cursor.
    unsafe { clang_sys::clang_getCursorExtent(cursor) }
}

/// Spelling of a cursor (name text), disposed safely; empty on failure.
#[cfg(feature = "semantic-clang")]
fn spelling_of(cursor: clang_sys::CXCursor) -> String {
    // SAFETY: live cursor; the CXString is disposed before returning.
    unsafe { cxstring_to_string(clang_sys::clang_getCursorSpelling(cursor)).unwrap_or_default() }
}

/// Type spelling of a cursor (`None` when unobtainable — the contract's
/// "assignment-from-fopen alone suffices" path). `FILE *` variables spell
/// with a `FILE` token (e.g. `"FILE *"`), which is what the FOPEN-SET
/// FILE-type rule matches.
#[cfg(feature = "semantic-clang")]
fn type_spelling_of(cursor: clang_sys::CXCursor) -> Option<String> {
    // SAFETY: live cursor; the CXString is disposed before returning.
    unsafe {
        let ty = clang_sys::clang_getCursorType(cursor);
        cxstring_to_string(clang_sys::clang_getTypeSpelling(ty)).filter(|s| !s.is_empty())
    }
}

/// A Direct fopen-family call: the REFERENCED cursor is a function
/// declaration whose spelling is exactly `fopen`/`freopen`/`fdopen`
/// ([`FOPEN_FAMILY`]). Function-pointer and macro-textual calls cannot
/// satisfy the referenced-`FunctionDecl` shape — conservative by
/// construction.
#[cfg(feature = "semantic-clang")]
fn is_fopen_call(cursor: clang_sys::CXCursor) -> bool {
    // SAFETY: live cursor.
    unsafe {
        if clang_sys::clang_getCursorKind(cursor) != clang_sys::CXCursor_CallExpr {
            return false;
        }
        let referenced = clang_sys::clang_getCursorReferenced(cursor);
        if clang_sys::clang_Cursor_isNull(referenced) != 0 {
            return false;
        }
        if clang_sys::clang_getCursorKind(referenced) != clang_sys::CXCursor_FunctionDecl {
            return false;
        }
        FOPEN_FAMILY.contains(&spelling_of(referenced).as_str())
    }
}

/// Tokenize a range into `(kind, spelling)` pairs in source order.
/// Tokens are disposed before returning; API failure yields an empty vec
/// (callers treat "no tokens" as "no proof" — conservative).
#[cfg(feature = "semantic-clang")]
fn tokenize(
    tu: clang_sys::CXTranslationUnit,
    range: clang_sys::CXSourceRange,
) -> Vec<(clang_sys::CXTokenKind, String)> {
    // SAFETY: `tu` is live for the whole traversal; `tokens` is disposed
    // before returning.
    unsafe {
        let mut tokens: *mut clang_sys::CXToken = std::ptr::null_mut();
        let mut count: std::os::raw::c_uint = 0;
        clang_sys::clang_tokenize(tu, range, &mut tokens, &mut count);
        if tokens.is_null() {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let token = *tokens.add(i as usize);
            let kind = clang_sys::clang_getTokenKind(token);
            let spelling = cxstring_to_string(clang_sys::clang_getTokenSpelling(tu, token))
                .unwrap_or_default();
            out.push((kind, spelling));
        }
        clang_sys::clang_disposeTokens(tu, tokens, count);
        out
    }
}

/// Resolve an initializer extent to its string-literal VALUE: the range
/// must tokenize to EXACTLY one token which is a `"..."` literal
/// ([`unquote_single_token`]). Conservative strengthening of the "one
/// STRING_LITERAL token" rule: the single-token requirement additionally
/// rejects `sizeof("a")`, parenthesized multi-token shapes, and any
/// concatenated/commented form — only a lone literal proves a value.
/// Prefixed literals (`L"…"` spell with the prefix, so they never match
/// the `"` pair either. Quotes are stripped (outermost pair only); escapes
/// are NOT processed.
#[cfg(feature = "semantic-clang")]
fn single_string_literal(
    tu: clang_sys::CXTranslationUnit,
    range: clang_sys::CXSourceRange,
) -> Option<String> {
    unquote_single_token(&tokenize(tu, range))
}

/// The shared single-token rule: EXACTLY one token which is a `"..."`
/// literal → its unquoted VALUE (outermost quote pair stripped, escapes
/// unprocessed). Anything else (empty, multi-token, non-literal,
/// unquoted) proves no value.
#[cfg(feature = "semantic-clang")]
fn unquote_single_token(tokens: &[(clang_sys::CXTokenKind, String)]) -> Option<String> {
    if tokens.len() != 1 {
        return None;
    }
    let (kind, spelling) = &tokens[0];
    if *kind != clang_sys::CXToken_Literal {
        return None;
    }
    if spelling.len() >= 2 && spelling.starts_with('"') && spelling.ends_with('"') {
        Some(spelling[1..spelling.len() - 1].to_string())
    } else {
        None
    }
}

/// Compound `=`-family operators: writes that invalidate the
/// single-assignment proof without ever proving a value.
#[cfg(feature = "semantic-clang")]
fn is_compound_assign(op: Option<&str>) -> bool {
    matches!(
        op,
        Some("+=" | "-=" | "*=" | "/=" | "%=" | "<<=" | ">>=" | "&=" | "|=" | "^=")
    )
}

/// USR of a cursor, disposed safely; empty on failure (callers skip empty).
#[cfg(feature = "semantic-clang")]
fn usr_of(cursor: clang_sys::CXCursor) -> String {
    // SAFETY: live cursor; the CXString is disposed before returning.
    unsafe { cxstring_to_string(clang_sys::clang_getCursorUSR(cursor)).unwrap_or_default() }
}

/// Convert + dispose a `CXString` (never leaks the clang allocation).
#[cfg(feature = "semantic-clang")]
unsafe fn cxstring_to_string(string: clang_sys::CXString) -> Option<String> {
    // SAFETY: `string` is a valid CXString from a clang getter; disposed here.
    unsafe {
        let raw = clang_sys::clang_getCString(string);
        let out = if raw.is_null() {
            None
        } else {
            Some(std::ffi::CStr::from_ptr(raw).to_string_lossy().into_owned())
        };
        clang_sys::clang_disposeString(string);
        out
    }
}

// ---------------------------------------------------------------------------
// Tests — pure core is parser-free (default gate); libclang-backed tests live
// in tests/resolver_health.rs behind `#[cfg(feature)]` + PERFPAL_SEMANTIC_TESTS
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_map_is_deterministic_and_ambiguity_minimal() {
        let candidates = |reverse: bool| {
            let mut v = vec![
                DefinitionCandidate {
                    usr: "c:@F#f#".to_string(),
                    path: "src/b.c".to_string(),
                    span: Span::point(10, 1),
                },
                DefinitionCandidate {
                    usr: "c:@F#f#".to_string(),
                    path: "src/a.c".to_string(),
                    span: Span::point(2, 1),
                },
                DefinitionCandidate {
                    usr: "c:@F#g#".to_string(),
                    path: "src/a.c".to_string(),
                    span: Span::point(5, 1),
                },
            ];
            if reverse {
                v.reverse();
            }
            v
        };

        let forward = build_definition_map(&candidates(false));
        let reverse = build_definition_map(&candidates(true));
        assert_eq!(forward, reverse, "map must be input-order independent");

        assert_eq!(forward.len(), 2);
        // Duplicate USR keeps the lexicographically smallest (path, span).
        assert_eq!(forward["c:@F#f#"].path, "src/a.c");
        assert_eq!(forward["c:@F#f#"].span, Span::point(2, 1));
        // Keys iterate sorted (BTreeMap contract).
        let keys: Vec<&String> = forward.keys().collect();
        assert!(keys.is_sorted());
    }

    #[test]
    fn call_reference_map_keys_are_path_ordered_points() {
        let refs = vec![
            CallReference {
                path: "src/main.c".to_string(),
                start_line: 12,
                start_col: 5,
                usr: "c:@F#g#".to_string(),
            },
            CallReference {
                path: "src/main.c".to_string(),
                start_line: 3,
                start_col: 9,
                usr: "c:@F#f#".to_string(),
            },
        ];
        let map = build_call_reference_map(&refs);
        let again = build_call_reference_map(&refs);
        assert_eq!(map, again, "map must be deterministic across builds");
        let keys: Vec<&(String, u32, u32)> = map.keys().collect();
        assert!(keys.is_sorted(), "keys must be (path,line,col) ordered");
        assert_eq!(map[&("src/main.c".to_string(), 3, 9)], "c:@F#f#");
    }

    #[test]
    fn maps_are_deliberately_not_serializable() {
        // Compile-time guard of the ADR-0001 USR rule: neither map type may
        // implement Serialize. (No test body needed — the bound check fails
        // compilation if someone derives Serialize.)
        fn assert_not_serialize<T>() {}
        assert_not_serialize::<DefinitionMap>();
        assert_not_serialize::<CallRefMap>();
        assert_not_serialize::<DefinitionSite>();
        // Wave B2: the per-function facts map is internal for the same
        // reason (facts reach output only via findings + evidence).
        assert_not_serialize::<FunctionFactsMap>();
        assert_not_serialize::<FunctionFacts>();
    }

    // -----------------------------------------------------------------------
    // Wave B2 pure-core tests (parser-free — run in BOTH builds)
    // -----------------------------------------------------------------------

    use crate::ir::{CallEvent, EventMeta, FunctionRecord, Shape, StatementRole};
    use crate::scanner::{FileUnit, FunctionUnit};

    fn b2_span(line: u32) -> Span {
        Span {
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: 20,
        }
    }

    fn b2_ident(name: &str) -> Shape {
        Shape {
            kind: ShapeKind::Identifier,
            name: Some(name.to_string()),
            ..Shape::default()
        }
    }

    fn b2_literal(value: &str) -> Shape {
        Shape {
            kind: ShapeKind::Literal,
            value: Some(value.to_string()),
            ..Shape::default()
        }
    }

    fn b2_call(callee: &str, args: Vec<Shape>, depth: u32, line: u32) -> CallEvent {
        CallEvent {
            meta: EventMeta {
                span: b2_span(line),
                in_loop_depth: depth,
                is_innermost: depth >= 1,
                resolver_tier: ResolverTier::Syntactic,
                macro_origin: false,
            },
            callee_name: callee.to_string(),
            callee_kind: CalleeKind::Direct,
            callee_path: None,
            is_macro: false,
            args,
            statement_role: StatementRole::Nested,
            conditional_guarded: false,
            semantic: None,
            receiver_root: None,
        }
    }

    fn b2_unit(path: &str, name: &str, calls: Vec<CallEvent>) -> FileUnit {
        let span = Span {
            start_line: 1,
            start_col: 1,
            end_line: 50,
            end_col: 1,
        };
        FileUnit {
            file: crate::ir::FileRecord {
                path: path.to_string(),
                language: "c".to_string(),
                ..crate::ir::FileRecord::default()
            },
            functions: vec![FunctionUnit {
                record: FunctionRecord {
                    id: format!("{path}#{name}#1:1"),
                    name: name.to_string(),
                    path: path.to_string(),
                    span,
                    ..FunctionRecord::default()
                },
                calls,
                ..FunctionUnit::default()
            }],
            ..FileUnit::default()
        }
    }

    fn b2_facts(
        path: &str,
        name: &str,
        fopen: &[&str],
        literals: &[(&str, &str)],
    ) -> FunctionFactsMap {
        let mut map = FunctionFactsMap::new();
        map.insert(
            (path.to_string(), name.to_string()),
            FunctionFacts {
                fopen_vars: fopen.iter().map(|s| (*s).to_string()).collect(),
                literals: literals
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            },
        );
        map
    }

    #[test]
    fn fopen_fact_sets_fopened_handle_on_matching_fclose_in_loop() {
        let mut units = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("fclose", vec![b2_ident("fp")], 1, 10)],
        )];
        let facts = b2_facts("src/a.c", "drain", &["fp"], &[]);
        let stats = apply_resolutions(
            &mut units,
            &DefinitionMap::new(),
            &CallRefMap::new(),
            &facts,
        );
        let call = &units[0].functions[0].calls[0];
        assert_eq!(
            call.semantic,
            Some(CallSemantic {
                resolved_definition: None,
                fopened_handle: true,
                literal_args: Vec::new(),
            })
        );
        assert_eq!(stats.calls_enriched, 1);
        assert_eq!(stats.fopen_handles_proven, 1);
        assert_eq!(stats.literal_args_resolved, 0);
        assert_eq!(stats.records_upgraded, 0);
    }

    #[test]
    fn fopen_fact_ignored_without_depth_name_and_callee_match() {
        // depth-0 fclose (same name, proven var) — the loop gate holds in
        // the resolver, not just the heuristic.
        let mut shallow = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("fclose", vec![b2_ident("fp")], 0, 10)],
        )];
        // Unknown handle: no fact for `other`.
        let mut unknown = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("fclose", vec![b2_ident("other")], 1, 10)],
        )];
        // Wrong callee: `free(fp)` never reads the FOPEN SET.
        let mut free_call = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("free", vec![b2_ident("fp")], 1, 10)],
        )];
        // Non-identifier arg0: `fclose(get_fp())` has no name to match.
        let mut call_arg = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("fclose", vec![b2_literal("x")], 1, 10)],
        )];
        let facts = b2_facts("src/a.c", "drain", &["fp"], &[]);
        for units in [&mut shallow, &mut unknown, &mut free_call, &mut call_arg] {
            let stats = apply_resolutions(units, &DefinitionMap::new(), &CallRefMap::new(), &facts);
            assert!(
                units[0].functions[0].calls[0].semantic.is_none(),
                "no fact ⇒ no semantic struct (INVARIANT)"
            );
            assert_eq!(stats.calls_enriched, 0);
        }
    }

    #[test]
    fn shared_gates_still_drop_b2_facts() {
        let facts = b2_facts("src/a.c", "drain", &["fp"], &[("fmt", "%s")]);
        let mut macro_tainted = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![{
                let mut c = b2_call("fclose", vec![b2_ident("fp")], 1, 10);
                c.meta.macro_origin = true;
                c
            }],
        )];
        let mut error_overlapped = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("fclose", vec![b2_ident("fp")], 1, 10)],
        )];
        error_overlapped[0].parse_health.error_regions = vec![b2_span(10)];
        let mut failed = vec![b2_unit(
            "src/a.c",
            "drain",
            vec![b2_call("fclose", vec![b2_ident("fp")], 1, 10)],
        )];
        failed[0].parse_health.status = ParseStatus::Failed;
        for units in [&mut macro_tainted, &mut error_overlapped, &mut failed] {
            let _ = apply_resolutions(units, &DefinitionMap::new(), &CallRefMap::new(), &facts);
            assert!(
                units[0].functions[0].calls[0].semantic.is_none(),
                "gated call keeps its syntactic truth untouched"
            );
        }
    }

    #[test]
    fn literal_fact_resolves_identifier_format_arg() {
        let mut units = vec![b2_unit(
            "src/a.c",
            "read",
            vec![
                b2_call("scanf", vec![b2_ident("fmt"), b2_ident("buf")], 0, 10),
                b2_call(
                    "fscanf",
                    vec![b2_ident("fp"), b2_ident("fmt2"), b2_ident("buf")],
                    0,
                    12,
                ),
            ],
        )];
        let facts = b2_facts("src/a.c", "read", &[], &[("fmt", "%s"), ("fmt2", "x%d")]);
        let stats = apply_resolutions(
            &mut units,
            &DefinitionMap::new(),
            &CallRefMap::new(),
            &facts,
        );
        assert_eq!(
            units[0].functions[0].calls[0]
                .semantic
                .as_ref()
                .map(|s| &s.literal_args),
            Some(&vec![(0, "%s".to_string())])
        );
        assert_eq!(
            units[0].functions[0].calls[1]
                .semantic
                .as_ref()
                .map(|s| &s.literal_args),
            Some(&vec![(1, "x%d".to_string())])
        );
        assert_eq!(stats.literal_args_resolved, 2);
        assert_eq!(stats.calls_enriched, 2);
    }

    #[test]
    fn literal_fact_ignores_literal_format_and_unmapped_names() {
        // Literal format args never consult the map (syntactic path owns
        // them); unmapped identifiers stay silent (never invents content).
        let mut units = vec![b2_unit(
            "src/a.c",
            "read",
            vec![
                b2_call("scanf", vec![b2_literal("\"%s\""), b2_ident("buf")], 0, 10),
                b2_call("scanf", vec![b2_ident("other"), b2_ident("buf")], 0, 12),
            ],
        )];
        let facts = b2_facts("src/a.c", "read", &[], &[("fmt", "%s")]);
        let stats = apply_resolutions(
            &mut units,
            &DefinitionMap::new(),
            &CallRefMap::new(),
            &facts,
        );
        for call in &units[0].functions[0].calls {
            assert!(call.semantic.is_none(), "no proof ⇒ no struct");
        }
        assert_eq!(stats.calls_enriched, 0);
    }

    #[test]
    fn b1_and_b2_facts_merge_on_one_call() {
        // A cross-file-resolved `fclose` that is ALSO fopen-proven carries
        // both facts in ONE struct (no overwrite between tiers).
        let mut units = vec![
            b2_unit(
                "src/a.c",
                "drain",
                vec![b2_call("fclose", vec![b2_ident("fp")], 1, 10)],
            ),
            b2_unit("src/b.c", "close_all", vec![]),
        ];
        let definitions = build_definition_map(&[DefinitionCandidate {
            usr: "c:@F@close_all#".to_string(),
            path: "src/b.c".to_string(),
            span: Span::point(1, 1),
        }]);
        let call_refs = build_call_reference_map(&[CallReference {
            path: "src/a.c".to_string(),
            start_line: 10,
            start_col: 1,
            usr: "c:@F@close_all#".to_string(),
        }]);
        // NOTE: the USR pairing here is synthetic (a `fclose` resolving to
        // `close_all` cannot happen from a real parse) — the test pins the
        // MERGE mechanics, not a real shape.
        let facts = b2_facts("src/a.c", "drain", &["fp"], &[]);
        let stats = apply_resolutions(&mut units, &definitions, &call_refs, &facts);
        let semantic = units[0].functions[0].calls[0]
            .semantic
            .clone()
            .expect("merged facts present");
        assert!(semantic.resolved_definition.is_some(), "B1 part kept");
        assert!(semantic.fopened_handle, "B2 part kept");
        assert_eq!(stats.calls_enriched, 1);
        assert_eq!(stats.fopen_handles_proven, 1);
    }

    #[test]
    fn merge_function_facts_unions_with_literal_first_wins() {
        let mut into = b2_facts("src/a.c", "f", &["fp"], &[("fmt", "%s")]);
        let from = b2_facts("src/a.c", "f", &["other"], &[("fmt", "%d"), ("g", "x")]);
        merge_function_facts(&mut into, from);
        let facts = &into[&("src/a.c".to_string(), "f".to_string())];
        assert_eq!(
            facts.fopen_vars,
            ["fp".to_string(), "other".to_string()]
                .into_iter()
                .collect()
        );
        assert_eq!(facts.literals.get("fmt").map(String::as_str), Some("%s"));
        assert_eq!(facts.literals.get("g").map(String::as_str), Some("x"));
        // Re-merging is idempotent (header-inline repeat parses).
        let snapshot = into.clone();
        let repeat = snapshot.clone();
        merge_function_facts(&mut into, repeat);
        assert_eq!(into, snapshot);
    }
}
