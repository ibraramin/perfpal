//! Shared fixture-IR harness (parser-free heuristic-test support).
//!
//! ADR-0001 guardrail 2: heuristics are tested against a parser-free fixture
//! corpus under `tests/fixtures/ir/` — no tree-sitter binding required. The
//! corpus holds serialized IR documents; the harness loads them as typed IR
//! (`Vec<FileUnit>` via [`load_units_fixture`]).
//!
//! Byte-determinism rule (modules.md §6 row 3): any JSON we emit must be
//! byte-identical across runs — object keys sorted, struct fields in
//! declaration order.

// Each integration test binary links this module and uses a subset of it.
#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use perfpal::analyzer::Finding;
use perfpal::ir::{
    AccessChainEvent, AssignmentEvent, BinaryOpEvent, CallEvent, CalleeKind, ComparisonEvent,
    EventMeta, FileRecord, FunctionRecord, GlobalVar, IncludeCounts, IncludeEdge, IncludeForm,
    LocCounts, LoopKind, LoopNode, ParseHealth, ResolverTier, ScopeClass, Shape, ShapeKind, Span,
    StatementRole, StorageClass, StructDef, StructKind, StructMember,
};
use perfpal::scanner::{FileUnit, FunctionUnit};

/// All findings of one rule, preserving document order.
#[must_use]
pub fn by_rule<'a>(findings: &'a [Finding], rule_id: &str) -> Vec<&'a Finding> {
    findings.iter().filter(|f| f.rule_id == rule_id).collect()
}

/// Load a fixture from `tests/fixtures/ir/<name>` and parse it as JSON.
///
/// # Panics
/// Panics with the fixture path in the message when missing or invalid —
/// a broken fixture is a test-authoring bug, not a runtime condition.
#[must_use]
pub fn load_ir_fixture(name: &str) -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ir")
        .join(name);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture unreadable: {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("fixture not valid JSON: {}: {e}", path.display()))
}

/// Load a fixture as TYPED IR (P1 upgrade of the P0 `serde_json::Value`
/// harness): an array of [`FileUnit`] documents, deserialized with no parser
/// bound (ADR-0001 guardrail 2).
///
/// # Panics
/// Same contract as [`load_ir_fixture`]; additionally panics when the JSON
/// does not match the IR v1 shapes.
#[must_use]
pub fn load_units_fixture(name: &str) -> Vec<FileUnit> {
    let value = load_ir_fixture(name);
    serde_json::from_value(value)
        .unwrap_or_else(|e| panic!("fixture not valid IR v1 document: {name}: {e}"))
}

/// Assert that serializing `value` twice yields identical bytes AND that every
/// object's keys are emitted in sorted order (the `preserve_order`-OFF
/// guarantee at the value level).
#[must_use]
pub fn deterministic_bytes(value: &serde_json::Value) -> String {
    let first = serde_json::to_string(value).expect("serialization failed");
    let second = serde_json::to_string(value).expect("serialization failed");
    assert_eq!(
        first, second,
        "two serialization passes differ — nondeterminism"
    );
    assert_keys_sorted(value);
    first
}

fn assert_keys_sorted(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let keys: Vec<&String> = map.keys().collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(
                keys, sorted,
                "object keys not emitted in sorted order — preserve_order leaked ON?"
            );
            for v in map.values() {
                assert_keys_sorted(v);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                assert_keys_sorted(v);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Hermetic corpus snapshots. `scanner::discover` does not skip Cargo's
// `target/` build directory, so scanning a live crate root would silently
// inflate the scan the moment someone runs `cargo build` in it. These helpers
// snapshot a corpus into a tempdir with `target/` excluded at every depth;
// relative paths are preserved exactly, so a snapshot scan is byte-identical
// to a live-tree scan modulo the (masked) absolute root.
// ---------------------------------------------------------------------------

/// Recursively copy `from` into `to`, skipping any directory whose name equals
/// `excluded_dir` at every depth.
///
/// # Panics
/// Panics if the source cannot be read or a file cannot be copied — a broken
/// corpus is a test-authoring bug, not a runtime condition.
pub fn copy_tree_excluding(from: &Path, to: &Path, excluded_dir: &str) {
    fs::create_dir_all(to).expect("create copy dir");
    for entry in fs::read_dir(from).expect("read source tree").flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            if entry.file_name().to_str() == Some(excluded_dir) {
                continue;
            }
            copy_tree_excluding(&src, &dst, excluded_dir);
        } else {
            fs::copy(&src, &dst).expect("copy tree file");
        }
    }
}

// ---------------------------------------------------------------------------
// m7 stale-sweep grace env: PERFPAL_STALE_GRACE_SECS is process-global, and
// cargo runs a test binary's tests in parallel threads, so every mutation is
// serialized through a global mutex. Call `set_stale_grace_env` and HOLD the
// returned guard for the whole assertion section; dropping the guard removes
// the variable again (restoring the built-in default grace). Test binaries
// that never touch the variable (e2e/pipeline) are unaffected: they only
// ever see own-pid staging dirs in their own unique temp parents, and the
// grace value never changes the own-pid always-sweep rule.
// ---------------------------------------------------------------------------

static GRACE_LOCK: Mutex<()> = Mutex::new(());

/// RAII handle: holds [`GRACE_LOCK`] and removes `PERFPAL_STALE_GRACE_SECS`
/// on drop, so a panicking test cannot leak the mutated environment.
pub struct GraceEnvGuard {
    _lock: MutexGuard<'static, ()>,
}

impl Drop for GraceEnvGuard {
    fn drop(&mut self) {
        // SAFETY: the lock is held for the whole lifetime of this guard, so
        // no other thread in this process mutates or reads the variable for
        // the purposes of the sweep tests; env mutation is otherwise not
        // thread-safe (Rust 2024 makes it explicitly unsafe).
        unsafe { std::env::remove_var("PERFPAL_STALE_GRACE_SECS") };
    }
}

/// Set `PERFPAL_STALE_GRACE_SECS` to `val` (or remove it on `None`) while
/// holding the global grace lock. Keep the returned guard alive for as long
/// as the value must hold; drop it to restore the default.
pub fn set_stale_grace_env(val: Option<&str>) -> GraceEnvGuard {
    let lock = GRACE_LOCK.lock().unwrap();
    // SAFETY: exclusive access guaranteed by GRACE_LOCK.
    unsafe {
        match val {
            Some(v) => std::env::set_var("PERFPAL_STALE_GRACE_SECS", v),
            None => std::env::remove_var("PERFPAL_STALE_GRACE_SECS"),
        }
    }
    GraceEnvGuard { _lock: lock }
}

// ---------------------------------------------------------------------------
// P4 integration determinism helpers (modules.md §7 "double-render modulo
// provenance"): mask the two provenance fields (`started_at`, `duration_ms`)
// wherever they surface — JSON in metrics.json, text in report.md.
// ---------------------------------------------------------------------------

/// Mask `scan.started_at` / `scan.duration_ms` in a metrics.json payload,
/// returning canonical JSON with both fields nulled.
#[must_use]
pub fn mask_metrics_provenance(raw: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(raw).expect("metrics.json parses");
    value["scan"]["started_at"] = serde_json::Value::Null;
    value["scan"]["duration_ms"] = serde_json::Value::Null;
    value.to_string()
}

/// Mask the provenance surfaces inside report.md: the header blockquote's
/// `started <RFC3339>` instant and `duration <n> ms` figure. Everything else
/// is compared verbatim. Hand-rolled (no regex dependency — Cargo.toml is
/// dependency-frozen). The v1 §5 template emits each marker at most once
/// (header blockquote), so first-occurrence masking is complete.
#[must_use]
pub fn mask_report_provenance(raw: &str) -> String {
    const STARTED: &str = "started ";
    const DURATION: &str = "duration ";
    raw.lines()
        .map(|line| {
            let mut masked = line.to_string();
            // RFC3339 UTC instant ends at the literal 'Z'.
            if let Some(i) = masked.find(STARTED) {
                let from = i + STARTED.len();
                if let Some(z) = masked[from..].find('Z') {
                    masked.replace_range(from..=from + z, "<MASKED>");
                }
            }
            if let Some(i) = masked.find(DURATION) {
                let from = i + DURATION.len();
                let digits = masked[from..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .count();
                if digits > 0 {
                    masked.replace_range(from..from + digits, "<MASKED>");
                }
            }
            masked
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// P6 golden-tree masking (tests/golden/report): the same provenance fields
// as above PLUS the absolute scan root, so the committed goldens stay valid
// on any machine/checkout path.
// ---------------------------------------------------------------------------

/// Golden metrics.json masking: `started_at`/`duration_ms` → null (as
/// [`mask_metrics_provenance`]) AND `scan.root_path` → the literal
/// `"<ROOT>"`. Everything else is byte-canonical JSON — INCLUDING
/// `parse_health.resolver` ("absent" on the default build), which is
/// neutralized only at COMPARISON time via [`mask_golden_resolver`] so the
/// committed goldens keep the honest default-build value.
#[must_use]
pub fn mask_golden_metrics(raw: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(raw).expect("metrics.json parses");
    value["scan"]["started_at"] = serde_json::Value::Null;
    value["scan"]["duration_ms"] = serde_json::Value::Null;
    value["scan"]["root_path"] = serde_json::Value::String("<ROOT>".to_string());
    value.to_string()
}

/// Comparison-time neutralizer for `files[].parse_health.resolver`: the
/// committed goldens are produced by the DEFAULT build (`"absent"`), while a
/// `semantic-clang` feature build legitimately produces `"ok"`. Both sides of
/// the golden comparison go through this so the feature build compares clean;
/// the exact default-build value stays locked by tests/resolver_health.rs.
#[must_use]
pub fn mask_golden_resolver(masked_metrics: &str) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(masked_metrics).expect("masked metrics.json parses");
    if let Some(files) = value["files"].as_array_mut() {
        for file in files {
            file["parse_health"]["resolver"] = serde_json::Value::String("<RESOLVER>".to_string());
        }
    }
    value.to_string()
}

/// Comparison-time neutralizer for Wave B1 resolved-edge deltas: the
/// committed goldens are produced by the DEFAULT build (name-based ARCH-003,
/// LOW), while a `semantic-clang` feature build legitimately emits a
/// MEDIUM-confidence resolved finding + MEDIUM call-graph edge for the same
/// call site. Both sides of the golden comparison go through this:
/// - `findings[]`: a `PERF-C-ARCH-003` finding carrying
///   `evidence.resolved == "true"` is rewritten to its name-based form
///   (confidence LOW, tier syntactic, message rebuilt byte-exact from
///   `callee_name`/`matched_path`, `resolved` key dropped);
/// - `findings[]` are then re-sorted by the §4 total order
///   (severity hot→warn→info, confidence HIGH→MEDIUM→LOW, rule_id → path →
///   start_line → start_col per output-contract.md §4), restoring the slot
///   the upgraded finding vacated — a stable no-op on the already-sorted
///   default side;
/// - `graphs.call_graph.edges[]`: confidence MEDIUM → LOW (default builds
///   never emit above LOW).
///
/// The exact default-build values stay locked by arch_fixtures.rs +
/// pipeline_e2e.rs; the resolved behavior by tests/resolver_health.rs.
#[must_use]
pub fn mask_golden_semantic(masked_metrics: &str) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(masked_metrics).expect("masked metrics.json parses");
    if let Some(findings) = value["findings"].as_array_mut() {
        for finding in findings.iter_mut() {
            let resolved_arch003 = finding["rule_id"] == "PERF-C-ARCH-003"
                && finding["evidence"]["resolved"] == "true";
            if resolved_arch003 {
                let callee = finding["evidence"]["callee_name"]
                    .as_str()
                    .unwrap_or("<unknown>")
                    .to_string();
                let matched = finding["evidence"]["matched_path"]
                    .as_str()
                    .unwrap_or("<unknown>")
                    .to_string();
                finding["confidence"] = serde_json::Value::String("LOW".to_string());
                finding["resolver_tier"] = serde_json::Value::String("syntactic".to_string());
                finding["message"] = serde_json::Value::String(format!(
                    "call to '{callee}' matches a definition in '{matched}' by name only — LOW-confidence hypothesis pending semantic resolution; confirming the match (or co-locating hot callees) is a candidate optimization"
                ));
                if let Some(evidence) = finding["evidence"].as_object_mut() {
                    evidence.remove("resolved");
                }
            }
        }
        let rank_severity = |s: &str| match s {
            "hot" => 0,
            "warn" => 1,
            _ => 2,
        };
        let rank_confidence = |c: &str| match c {
            "HIGH" => 0,
            "MEDIUM" => 1,
            _ => 2,
        };
        let sort_key = |f: &serde_json::Value| {
            (
                rank_severity(f["severity"].as_str().unwrap_or("")),
                rank_confidence(f["confidence"].as_str().unwrap_or("")),
                f["rule_id"].as_str().unwrap_or("").to_string(),
                f["path"].as_str().unwrap_or("").to_string(),
                f["span"]["start_line"].as_u64().unwrap_or(0),
                f["span"]["start_col"].as_u64().unwrap_or(0),
            )
        };
        findings.sort_by_key(|a| sort_key(a));
    }
    if let Some(edges) = value["graphs"]["call_graph"]["edges"].as_array_mut() {
        for edge in edges.iter_mut() {
            if edge["confidence"] == "MEDIUM" {
                edge["confidence"] = serde_json::Value::String("LOW".to_string());
            }
        }
    }
    value.to_string()
}

/// Markdown metacharacter set escaped by the reporter's `escape_md`
/// (`src/reporter/mod.rs::MD_ESCAPE_CHARS`). Duplicated here because the
/// helper is private to the crate and the golden masker must reproduce the
/// escaping to neutralize ABSOLUTE scan roots whose characters get
/// backslash-escaped in report.md (e.g. `-` in `e2e-mixed`, `examples-rust`).
/// If the reporter's pinned set ever changes, `regenerate_goldens`' `<ROOT>`
/// sentinel assertion fails loudly.
const MD_ESCAPE_CHARS: &[char] = &[
    '\\', '`', '*', '_', '{', '}', '[', ']', '(', ')', '#', '+', '-', '.', '!', '|', '<', '>', '&',
];

/// Reproduce `escape_md` for the absolute-root needle.
fn markdown_escaped(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    for c in input.chars() {
        if MD_ESCAPE_CHARS.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Golden report.md masking: the [`mask_report_provenance`] surfaces
/// (`started <RFC3339>`, `duration <n> ms`) PLUS every literal occurrence
/// of the absolute scan `root` → `"<ROOT>"` (the header blockquote and the
/// §2 "Scan root" line are the only surfaces, per the §5 template) PLUS the
/// Markdown-ESCAPED form of the root (the reporter runs every embedded path
/// through `escape_md`, so e.g. `…/e2e-mixed` appears as `…/e2e\-mixed`)
/// PLUS the Wave B1 ARCH-003 §4 block strip: a `semantic-clang` feature
/// build legitimately renders the resolved finding (MEDIUM/semantic/resolved
/// evidence) at a DIFFERENT §4 position than the default build's name-based
/// block, so string-level normalization cannot restore order — the whole
/// block (header `#### Cross\-file call coupling` through the next blank
/// line, or the next `#### ` finding header defensively) is dropped on BOTH
/// sides. The header terminator is explicit (`"#### "` with the trailing
/// space, pushed back onto the output) IN ADDITION to the blank-line rule —
/// a future evidence line containing a blank line ends the skip at that
/// blank line exactly as before, and only a real finding header (never a
/// `#`-starting evidence line) can end it otherwise, so neither
/// over-stripping nor under-stripping is possible.
/// ARCH-003 §4 rendering stays locked by arch_fixtures.rs + pipeline_e2e.rs
/// (name-based) and tests/resolver_health.rs (resolved variant).
/// Backslash→slash normalization is applied to the SEARCH PATH ONLY (never
/// to the document, which legitimately contains Markdown escapes), so the
/// comparison is separator-agnostic.
#[must_use]
pub fn mask_golden_report(raw: &str, root: &str) -> String {
    let forward = root.replace('\\', "/");
    let escaped = markdown_escaped(root);
    let escaped_forward = markdown_escaped(&forward);
    let mut masked = mask_report_provenance(raw);
    for needle in [root, &forward, &escaped, &escaped_forward] {
        if !needle.is_empty() {
            masked = masked.replace(needle, "<ROOT>");
        }
    }
    let mut kept: Vec<&str> = Vec::new();
    let mut skipping_arch003 = false;
    for line in masked.lines() {
        if line.starts_with("#### Cross\\-file call coupling") {
            skipping_arch003 = true;
            continue;
        }
        if skipping_arch003 {
            if line.trim().is_empty() || line.starts_with("#### ") {
                skipping_arch003 = false;
                // The terminating blank line belongs to the stripped block;
                // a finding header line belongs to the document.
                if line.starts_with("#### ") {
                    kept.push(line);
                }
            }
            continue;
        }
        kept.push(line);
    }
    kept.join("\n")
}

// ---------------------------------------------------------------------------
// Kitchen-sink synthetic unit set (moved from tests/wave_a_fixtures.rs so
// the P6 finding-id uniqueness gate can reuse the all-23-rule corpus).
// ---------------------------------------------------------------------------

/// Point-ish span helper (cols chosen so containment stays plausible).
#[must_use]
pub fn sp(line: u32, sc: u32, ec: u32) -> Span {
    Span {
        start_line: line,
        start_col: sc,
        end_line: line,
        end_col: ec,
    }
}

#[must_use]
fn meta(span: Span, depth: u32) -> EventMeta {
    EventMeta {
        span,
        in_loop_depth: depth,
        // Scanner contract (ir-v1.md §3.3): events outside any loop are
        // NEVER innermost.
        is_innermost: depth >= 1,
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
    }
}

#[must_use]
fn call(callee: &str, args: &[&str], span: Span, depth: u32, role: StatementRole) -> CallEvent {
    CallEvent {
        meta: meta(span, depth),
        callee_name: callee.to_string(),
        callee_kind: CalleeKind::Direct,
        // Synthetic C-fixture calls: no path/macro distinction (additive R3).
        callee_path: None,
        is_macro: false,
        args: args
            .iter()
            .map(|name| Shape {
                kind: ShapeKind::Identifier,
                name: Some((*name).to_string()),
                ..Shape::default()
            })
            .collect(),
        statement_role: role,
        conditional_guarded: false,
        semantic: None,
    }
}

#[must_use]
fn loop_node(kind: LoopKind, depth: u32, span: Span) -> LoopNode {
    LoopNode {
        kind,
        depth,
        induction_vars: Vec::new(),
        span,
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
    }
}

/// Binary-op event with an identifier right operand (LOOP-004 kitchen
/// input). `depth` must be ≥ 1 with the derived innermost flag — the
/// LOOP-004 gate consumes `is_innermost`.
#[must_use]
fn binop(op: &str, span: Span, divisor: &str, depth: u32) -> BinaryOpEvent {
    BinaryOpEvent {
        meta: meta(span, depth),
        op: op.to_string(),
        left: Shape::identifier("acc"),
        right: Shape::identifier(divisor),
    }
}

#[must_use]
fn func(
    name: &str,
    path: &str,
    span: Span,
    calls: Vec<CallEvent>,
    loops: Vec<LoopNode>,
) -> FunctionUnit {
    FunctionUnit {
        record: FunctionRecord {
            id: format!("{path}#{name}#{}:{}", span.start_line, span.start_col),
            name: name.to_string(),
            path: path.to_string(),
            span,
            param_count: 0,
            param_types: Vec::new(),
            cyclomatic_complexity: 1,
            cognitive_complexity: 0,
            max_loop_depth: loops.iter().map(|l| l.depth).max().unwrap_or(0),
            alloc_site_count: 0,
            length_lines: span.end_line - span.start_line + 1,
            resolver_tier: ResolverTier::Syntactic,
        },
        calls,
        assignments: Vec::new(),
        binary_ops: Vec::new(),
        comparisons: Vec::new(),
        access_chains: Vec::new(),
        loops,
    }
}

/// TYPE-001/002 inputs in the kitchen unit:
/// - `Big` (char[32] + long): by-value estimate 40 bytes ≥ the 32-byte
///   floor, zero padding — trips TYPE-001 only;
/// - `Loose` (char/long/char): aligned estimate 24 vs packed sum 10 wastes
///   14 bytes ≥ floor 8 — trips TYPE-002 only.
#[must_use]
fn kitchen_structs(path: &str) -> Vec<StructDef> {
    vec![
        StructDef {
            name: "Big".to_string(),
            path: path.to_string(),
            span: sp(45, 5, 6),
            members: vec![
                StructMember {
                    name: "buf".to_string(),
                    type_text: "char".to_string(),
                    array_dim_literal: Some(32),
                    is_pointer: false,
                },
                StructMember {
                    name: "tag".to_string(),
                    type_text: "long".to_string(),
                    array_dim_literal: None,
                    is_pointer: false,
                },
            ],
            kind: StructKind::Struct,
        },
        StructDef {
            name: "Loose".to_string(),
            path: path.to_string(),
            span: sp(50, 5, 6),
            members: vec![
                StructMember {
                    name: "a".to_string(),
                    type_text: "char".to_string(),
                    array_dim_literal: None,
                    is_pointer: false,
                },
                StructMember {
                    name: "l".to_string(),
                    type_text: "long".to_string(),
                    array_dim_literal: None,
                    is_pointer: false,
                },
                StructMember {
                    name: "c".to_string(),
                    type_text: "char".to_string(),
                    array_dim_literal: None,
                    is_pointer: false,
                },
            ],
            kind: StructKind::Struct,
        },
    ]
}

/// Two units: `src/kitchen.c` nests three loops and trips ALLOC-001/002/003,
/// STR-001/002/003, LOOP-001..008 (global-var store, indirect call, induced
/// subscript comparison, and deep access chain all sit in the innermost
/// loop), MEM-001 (pointer-param sizeof) + MEM-002 (non-literal memcpy
/// size), TYPE-001 (`struct Big b` by value) + TYPE-002 (`Loose` padding),
/// ARCH-001 (unresolved quoted include), and ARCH-003 (cross-file call to
/// `helper_fn`); `recurse` trips CALL-001 and CALL-002. The whole `src`
/// component (complexity_sum 22 ≥ 15) trips ARCH-002, anchored at the most
/// complex function (`recurse`).
#[must_use]
pub fn kitchen_units() -> Vec<FileUnit> {
    let path = "src/kitchen.c";
    let mut kitchen_calls = vec![
        call("malloc", &["n"], sp(5, 17, 27), 3, StatementRole::Nested),
        call("free", &["old"], sp(6, 17, 26), 3, StatementRole::Nested),
        call(
            "strcat",
            &["buf", "part"],
            sp(7, 17, 33),
            3,
            StatementRole::Nested,
        ),
        call("strlen", &["buf"], sp(8, 17, 27), 3, StatementRole::Nested),
        call(
            "strcpy",
            &["dst", "src"],
            sp(9, 17, 34),
            3,
            StatementRole::Nested,
        ),
        call("fgetc", &["fp"], sp(10, 17, 24), 3, StatementRole::Primary),
        // ALLOC-003: additive growth — the `+` binop below sits INSIDE
        // this call span (cols 28–38 within 17–45).
        call(
            "realloc",
            &["buf", "size"],
            sp(11, 17, 45),
            3,
            StatementRole::Nested,
        ),
        // LOOP-002: unguarded expensive I/O inside the loop nest.
        call("printf", &["fmt"], sp(12, 17, 29), 3, StatementRole::Nested),
        // ARCH-003: direct call whose name matches only src/helper.c.
        call("helper_fn", &[], sp(14, 17, 28), 3, StatementRole::Nested),
    ];
    // ALLOC-003 two-part proof requires the realloc SIZE arg (last) to be
    // a Binary shape — `realloc(buf, size + chunk)`.
    if let Some(realloc) = kitchen_calls
        .iter_mut()
        .find(|c| c.callee_name == "realloc")
        && let Some(size_arg) = realloc.args.last_mut()
    {
        size_arg.kind = ShapeKind::Binary;
    }
    // LOOP-006: INDIRECT call in the innermost loop (direct-only gates on
    // LOOP-002/003 and CALL-001 keep every other rule silent here).
    kitchen_calls.push(CallEvent {
        meta: meta(sp(15, 17, 26), 3),
        callee_name: "cb".to_string(),
        callee_kind: CalleeKind::Indirect,
        callee_path: None,
        is_macro: false,
        args: Vec::new(),
        statement_role: StatementRole::Nested,
        conditional_guarded: false,
        semantic: None,
    });
    // MEM-001: malloc(n * sizeof(p)) with `p` a pointer-typed kitchen
    // parameter — the flagship pointer_param_sizeof misuse class.
    kitchen_calls.push(CallEvent {
        meta: meta(sp(19, 17, 40), 3),
        callee_name: "malloc".to_string(),
        callee_kind: CalleeKind::Direct,
        callee_path: None,
        is_macro: false,
        args: vec![
            Shape::identifier("n"),
            Shape {
                kind: ShapeKind::Sizeof,
                sizeof_operand: Some("p".to_string()),
                ..Shape::default()
            },
        ],
        statement_role: StatementRole::Nested,
        conditional_guarded: false,
        semantic: None,
    });
    // MEM-002: memcpy with a non-literal byte count in the innermost loop.
    kitchen_calls.push(call(
        "memcpy",
        &["dst", "src", "len"],
        sp(20, 17, 42),
        3,
        StatementRole::Nested,
    ));
    let kitchen_binops = vec![
        // ALLOC-003 additivity proof: contained in the realloc call span.
        BinaryOpEvent {
            meta: meta(sp(11, 28, 38), 3),
            op: "+".to_string(),
            left: Shape::identifier("size"),
            right: Shape::identifier("chunk"),
        },
        // LOOP-004: variable-divisor division in the innermost loop
        // (depth 3 ⇒ derived is_innermost = true).
        binop("/", sp(13, 17, 24), "n", 3),
    ];
    // LOOP-005: file-scope store inside the innermost loop.
    let kitchen_assignments = vec![AssignmentEvent {
        meta: meta(sp(16, 17, 30), 3),
        target: Shape::identifier("g_sink"),
        scope_class: ScopeClass::Global,
    }];
    // LOOP-007: arr[j] == key at depth 3 with j an induction var of the
    // innermost loop (set below) — hand-rolled O(n²) lookup shape.
    let kitchen_comparisons = vec![ComparisonEvent {
        meta: meta(sp(17, 17, 31), 3),
        left: Shape {
            kind: ShapeKind::Subscript,
            name: Some("arr".to_string()),
            index_ident: Some("j".to_string()),
            ..Shape::default()
        },
        right: Shape::identifier("key"),
    }];
    // LOOP-008: depth-3 member chain off `p` in the innermost loop.
    let kitchen_chains = vec![AccessChainEvent {
        meta: meta(sp(18, 17, 40), 3),
        root: "p".to_string(),
        elements: vec![
            Shape {
                kind: ShapeKind::MemberAccess,
                name: Some("ctx".to_string()),
                depth: Some(1),
                ..Shape::default()
            },
            Shape {
                kind: ShapeKind::MemberAccess,
                name: Some("ops".to_string()),
                depth: Some(2),
                ..Shape::default()
            },
            Shape {
                kind: ShapeKind::MemberAccess,
                name: Some("flags".to_string()),
                depth: Some(3),
                ..Shape::default()
            },
        ],
    }];
    let mut kitchen_loops = vec![
        loop_node(
            LoopKind::For,
            1,
            Span {
                start_line: 2,
                start_col: 5,
                end_line: 40,
                end_col: 6,
            },
        ),
        loop_node(
            LoopKind::For,
            2,
            Span {
                start_line: 3,
                start_col: 9,
                end_line: 39,
                end_col: 10,
            },
        ),
        loop_node(
            LoopKind::For,
            3,
            Span {
                start_line: 4,
                start_col: 13,
                end_line: 38,
                end_col: 14,
            },
        ),
    ];
    // LOOP-007 needs `j` among the innermost loop's induction vars.
    kitchen_loops[2].induction_vars = vec!["j".to_string()];
    let recurse_calls = vec![
        call("recurse", &["a"], sp(43, 5, 15), 0, StatementRole::Primary),
        call("recurse", &["b"], sp(44, 5, 15), 0, StatementRole::Primary),
    ];
    vec![FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts::default(),
            function_count: 2,
            global_var_count: 1,
            include_counts: IncludeCounts::default(),
        },
        parse_health: ParseHealth::default(),
        // ARCH-001: unresolvable quoted include (to_path nulled by the
        // resolver — data, not error).
        includes: vec![IncludeEdge {
            from_path: path.to_string(),
            to_path: None,
            form: IncludeForm::Quoted,
            resolved: false,
            is_mod: false,
            span: sp(1, 1, 20),
        }],
        // LOOP-005 plausibility: the name-classified global exists.
        global_vars: vec![GlobalVar {
            name: "g_sink".to_string(),
            path: path.to_string(),
            span: sp(1, 22, 30),
            storage: StorageClass::None,
        }],
        // TYPE-001/002 inputs (see struct comments above).
        structs: kitchen_structs(path),
        macros: Vec::new(),
        functions: vec![
            {
                let mut kitchen = func(
                    "kitchen",
                    path,
                    Span {
                        start_line: 1,
                        start_col: 5,
                        end_line: 41,
                        end_col: 6,
                    },
                    kitchen_calls,
                    kitchen_loops,
                );
                kitchen.binary_ops = kitchen_binops;
                kitchen.assignments = kitchen_assignments;
                kitchen.comparisons = kitchen_comparisons;
                kitchen.access_chains = kitchen_chains;
                // MEM-001 flagship + TYPE-001 inputs: `p` is the pointer
                // param named by sizeof(p); `struct Big b` passes by value.
                kitchen.record.param_count = 3;
                kitchen.record.param_types = vec![
                    "int n".to_string(),
                    "char *p".to_string(),
                    "struct Big b".to_string(),
                ];
                kitchen
            },
            {
                // CALL-002: complexity far above the threshold of 15.
                let mut recurse = func(
                    "recurse",
                    path,
                    Span {
                        start_line: 42,
                        start_col: 5,
                        end_line: 45,
                        end_col: 6,
                    },
                    recurse_calls,
                    Vec::new(),
                );
                recurse.record.cyclomatic_complexity = 20;
                recurse
            },
        ],
    }]
}

/// ARCH-003 target: `helper_fn` is defined ONLY here, so kitchen's direct
/// call becomes a cross-file name-matched edge.
#[must_use]
pub fn helper_unit() -> FileUnit {
    let path = "src/helper.c";
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: "c".to_string(),
            loc: LocCounts::default(),
            function_count: 1,
            global_var_count: 0,
            include_counts: IncludeCounts::default(),
        },
        parse_health: ParseHealth::default(),
        includes: Vec::new(),
        global_vars: Vec::new(),
        structs: Vec::new(),
        macros: Vec::new(),
        functions: vec![func(
            "helper_fn",
            path,
            Span {
                start_line: 1,
                start_col: 5,
                end_line: 3,
                end_col: 6,
            },
            Vec::new(),
            Vec::new(),
        )],
    }
}
