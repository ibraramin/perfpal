//! Python negative / suppression IR fixtures + tests (plan `python-backend`
//! W4, lane 2).
//!
//! Parser-free IR documents under `tests/fixtures/ir/` (ADR-0001 guardrail 2)
//! lock the three negative classes for the Python tier, mirroring the shipped
//! Rust corpus (`tests/rust_negative_fixtures.rs` + `PERF-RS-*` fixtures):
//!
//! 1. **not-in-loop** — a family site outside every loop stays silent
//!    (`in_loop_depth == 0`) for every loop-gated `PERF-PY-*` rule;
//! 2. **downgrade hooks** — a finding that overlaps a `ParseHealth` error
//!    region still fires but is capped at MEDIUM with
//!    `error_node_overlap=true` (the shared `analyzer::apply_downgrade_hooks`,
//!    never per-rule). Python has no macros, so the macro-origin half of the
//!    hook is not exercised here — only the error-region path;
//! 3. **suppression** — rule-specific silent variants: the ALLOC-003
//!    rebuilt-receiver control, the LOOP-007 set/dict kind gate, the STR-001
//!    accumulator gate, ARCH-003 file-local-first, CALL-002 below-threshold,
//!    and ASYNC-001 non-`async def` / module-qualified `aiofiles.open`.
//!
//! Section 4 adds parser-free POSITIVE fixtures for every Python rule the IR
//! alone unambiguously drives, so each rule has an exact-count firing lock to
//! pair with its silence lock.
//!
//! Every assertion is an EXACT count (`== 0` or `== 1`) — never a range — so a
//! regression that silently drops or duplicates a finding fails loudly.

mod common;

use perfpal::analyzer::Severity;
use perfpal::analyzer::finding::{
    PY_ALLOC_001_RULE_ID, PY_ALLOC_003_RULE_ID, PY_ALLOC_004_RULE_ID, PY_ARCH_001_RULE_ID,
    PY_ARCH_002_RULE_ID, PY_ARCH_003_RULE_ID, PY_ASYNC_001_RULE_ID, PY_ASYNC_002_RULE_ID,
    PY_CALL_001_RULE_ID, PY_CALL_002_RULE_ID, PY_COMPREHENSION_001_RULE_ID,
    PY_EXECUTOR_001_RULE_ID, PY_LOOP_001_RULE_ID, PY_LOOP_002_RULE_ID, PY_LOOP_003_RULE_ID,
    PY_LOOP_007_RULE_ID, PY_LOOP_009_RULE_ID, PY_MEM_002_RULE_ID, PY_N1_001_RULE_ID,
    PY_PANDAS_001_RULE_ID, PY_PANDAS_002_RULE_ID, PY_STR_001_RULE_ID,
};
use perfpal::analyzer::{Confidence, analyze};

// ===========================================================================
// 1. not-in-loop — every loop-gated rule must stay silent outside every loop.
// ===========================================================================

#[test]
fn alloc_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ALLOC_001_RULE_ID).is_empty(),
        "a loop-invariant constructor outside any loop must not fire PY-ALLOC-001"
    );
}

#[test]
fn alloc_003_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-003__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ALLOC_003_RULE_ID).is_empty(),
        "list concat/append outside any loop must not fire PY-ALLOC-003"
    );
}

#[test]
fn alloc_004_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-004__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ALLOC_004_RULE_ID).is_empty(),
        "eager iterable materialisation outside any loop must not fire PY-ALLOC-004"
    );
}

#[test]
fn str_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-STR-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_STR_001_RULE_ID).is_empty(),
        "string accumulation outside any loop must not fire PY-STR-001"
    );
}

#[test]
fn mem_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-MEM-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_MEM_002_RULE_ID).is_empty(),
        "a container copy outside any loop must not fire PY-MEM-002"
    );
}

#[test]
fn loop_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-LOOP-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_LOOP_002_RULE_ID).is_empty(),
        "an expensive I/O call outside any loop must not fire PY-LOOP-002"
    );
}

#[test]
fn loop_003_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-LOOP-003__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_LOOP_003_RULE_ID).is_empty(),
        "`range(len(seq))` outside any loop must not fire PY-LOOP-003"
    );
}

#[test]
fn loop_007_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-LOOP-007__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_LOOP_007_RULE_ID).is_empty(),
        "a list membership/index scan outside any loop must not fire PY-LOOP-007"
    );
}

#[test]
fn loop_009_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-LOOP-009__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_LOOP_009_RULE_ID).is_empty(),
        "a `try`/`except` outside any loop must not fire PY-LOOP-009"
    );
}

#[test]
fn async_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-ASYNC-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ASYNC_002_RULE_ID).is_empty(),
        "an `await` outside any loop must not fire PY-ASYNC-002"
    );
}

#[test]
fn executor_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-EXECUTOR-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_EXECUTOR_001_RULE_ID).is_empty(),
        "an executor constructed outside any loop must not fire PY-EXECUTOR-001"
    );
}

#[test]
fn n1_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-N1-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_N1_001_RULE_ID).is_empty(),
        "a DB/HTTP call outside any loop must not fire PY-N1-001"
    );
}

#[test]
fn pandas_002_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-PANDAS-002__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_PANDAS_002_RULE_ID).is_empty(),
        "`pd.concat` outside any loop must not fire PY-PANDAS-002"
    );
}

#[test]
fn comprehension_001_stays_silent_when_not_in_loop() {
    let units = common::load_units_fixture("PERF-PY-COMPREHENSION-001__not-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_COMPREHENSION_001_RULE_ID).is_empty(),
        "a single `append` outside any loop must not fire PY-COMPREHENSION-001"
    );
}

// ===========================================================================
// 2. Downgrade hooks — error-region overlap caps at MEDIUM, never drops.
// ===========================================================================

#[test]
fn loop_001_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-PY-LOOP-001__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a depth-3 nest inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps HIGH at MEDIUM"
    );
    assert!(
        f.error_node_overlap,
        "error region overlaps the finding span"
    );
    assert!(
        !f.macro_suspect,
        "Python has no macros; hooks fire independently"
    );
    assert_eq!(f.evidence.get("nest_depth").map(String::as_str), Some("3"));
}

#[test]
fn loop_003_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-PY-LOOP-003__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_003_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a `range(len(seq))` inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps HIGH at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
}

#[test]
fn call_001_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-PY-CALL-001__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_CALL_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a self-call inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps HIGH at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
}

#[test]
fn call_002_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-PY-CALL-002__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_CALL_002_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a complexity finding inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "error-region overlap caps HIGH at MEDIUM"
    );
    assert!(f.error_node_overlap);
    assert!(!f.macro_suspect, "hooks fire independently");
    assert_eq!(f.evidence.get("complexity").map(String::as_str), Some("15"));
}

#[test]
fn alloc_001_error_region_overlap_downgrades_to_medium_with_flag() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-001__error-region-overlap.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a loop-invariant constructor inside an error region is downgraded, never dropped"
    );
    let f = hits[0];
    assert_eq!(
        f.severity,
        Severity::Info,
        "severity band unchanged by hook"
    );
    assert_eq!(
        f.confidence,
        Confidence::Medium,
        "the rule's MEDIUM ceiling is unchanged by the hook"
    );
    assert!(
        f.error_node_overlap,
        "central hook flags error-region overlap"
    );
    assert!(!f.macro_suspect, "hooks fire independently");
    assert_eq!(f.evidence.get("callee").map(String::as_str), Some("open"));
}

// ===========================================================================
// 3. Suppression negatives.
// ===========================================================================

#[test]
fn alloc_003_receiver_rebuilt_in_loop_stays_silent() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-003__rebuilt-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ALLOC_003_RULE_ID).is_empty(),
        "a receiver rebuilt inside its loop (CTL-APPEND-LOCAL) must stay silent"
    );
}

#[test]
fn loop_007_set_and_dict_containers_stay_silent() {
    let units = common::load_units_fixture("PERF-PY-LOOP-007__set-dict-container.json");
    assert!(
        common::by_rule(&analyze(&units), PY_LOOP_007_RULE_ID).is_empty(),
        "membership/index scans over set or dict containers must stay silent"
    );
}

#[test]
fn str_001_accumulator_gate_stays_silent() {
    let units = common::load_units_fixture("PERF-PY-STR-001__accumulator-gate.json");
    assert!(
        common::by_rule(&analyze(&units), PY_STR_001_RULE_ID).is_empty(),
        "a loop-invariant string read is not an accumulation and must stay silent"
    );
}

#[test]
fn arch_003_file_local_definition_suppresses_cross_module_edge() {
    let units = common::load_units_fixture("PERF-PY-ARCH-003__file-local-first.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ARCH_003_RULE_ID).is_empty(),
        "a same-file definition suppresses the cross-file hypothesis"
    );
}

#[test]
fn call_002_below_threshold_stays_silent() {
    let units = common::load_units_fixture("PERF-PY-CALL-002__below-threshold.json");
    assert!(
        common::by_rule(&analyze(&units), PY_CALL_002_RULE_ID).is_empty(),
        "cyclomatic complexity 14 is below the threshold of 15"
    );
}

#[test]
fn async_001_non_async_and_module_qualified_open_stay_silent() {
    let units = common::load_units_fixture("PERF-PY-ASYNC-001__non-async-aiofiles.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ASYNC_001_RULE_ID).is_empty(),
        "a blocking call in a sync function and a module-qualified `aiofiles.open` must stay silent"
    );
}

#[test]
fn alloc_004_literal_argument_stays_silent() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-004__literal-arg.json");
    assert!(
        common::by_rule(&analyze(&units), PY_ALLOC_004_RULE_ID).is_empty(),
        "a literal-arg constructor belongs to PY-ALLOC-001, not PY-ALLOC-004"
    );
}

#[test]
fn loop_002_print_in_loop_stays_silent() {
    let units = common::load_units_fixture("PERF-PY-LOOP-002__print-in-loop.json");
    assert!(
        common::by_rule(&analyze(&units), PY_LOOP_002_RULE_ID).is_empty(),
        "per-iteration `print` is deliberately excluded from the I/O family"
    );
}

// ===========================================================================
// 4. Positives — exact-count firing locks, one per IR-testable rule.
// ===========================================================================

#[test]
fn alloc_001_fires_on_loop_invariant_construction() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-001__constructor-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ALLOC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a literal-arg constructor in a loop fires once"
    );
    assert_eq!(hits[0].severity, Severity::Info);
    assert_eq!(hits[0].confidence, Confidence::Medium);
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("open")
    );
    assert_eq!(
        hits[0].evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
}

#[test]
fn alloc_003_fires_on_non_in_place_concat() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-003__concat-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ALLOC_003_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a non-in-place list concat in a loop fires once"
    );
    assert_eq!(hits[0].severity, Severity::Warn);
    assert_eq!(
        hits[0].evidence.get("binding_kind").map(String::as_str),
        Some("list")
    );
    assert_eq!(
        hits[0].evidence.get("growth").map(String::as_str),
        Some("concat")
    );
}

#[test]
fn alloc_004_fires_on_iterable_materialisation() {
    let units = common::load_units_fixture("PERF-PY-ALLOC-004__materialise-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ALLOC_004_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a non-literal `list(...)` in a loop fires once"
    );
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("list")
    );
}

#[test]
fn str_001_fires_on_string_accumulation() {
    let units = common::load_units_fixture("PERF-PY-STR-001__concat-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_STR_001_RULE_ID);
    assert_eq!(hits.len(), 1, "a str accumulation in a loop fires once");
    assert_eq!(hits[0].severity, Severity::Warn);
    assert_eq!(
        hits[0].evidence.get("binding_kind").map(String::as_str),
        Some("str")
    );
    assert_eq!(
        hits[0].evidence.get("form").map(String::as_str),
        Some("concat")
    );
}

#[test]
fn mem_002_fires_on_container_copy_in_loop() {
    let units = common::load_units_fixture("PERF-PY-MEM-002__copy-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_MEM_002_RULE_ID);
    assert_eq!(hits.len(), 1, "a container copy in a loop fires once");
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("copy.deepcopy")
    );
}

#[test]
fn loop_001_fires_only_at_depth_three() {
    let units = common::load_units_fixture("PERF-PY-LOOP-001__deep-nest.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_001_RULE_ID);
    assert_eq!(hits.len(), 1, "exactly the depth-3 ancestor loop fires");
    assert_eq!(
        hits[0].evidence.get("nest_depth").map(String::as_str),
        Some("3")
    );
    assert_eq!(
        hits[0].evidence.get("max_depth").map(String::as_str),
        Some("3")
    );
}

#[test]
fn loop_002_fires_on_expressive_io_in_loop() {
    let units = common::load_units_fixture("PERF-PY-LOOP-002__io-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_002_RULE_ID);
    assert_eq!(hits.len(), 1, "a subprocess I/O call in a loop fires once");
    assert_eq!(hits[0].severity, Severity::Warn);
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("subprocess.Popen")
    );
}

#[test]
fn loop_003_fires_on_range_len() {
    let units = common::load_units_fixture("PERF-PY-LOOP-003__range-len.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_003_RULE_ID);
    assert_eq!(hits.len(), 1, "`range(len(...))` in a loop fires once");
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("range")
    );
}

#[test]
fn loop_007_fires_on_list_membership_scan() {
    let units = common::load_units_fixture("PERF-PY-LOOP-007__membership-scan.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_007_RULE_ID);
    assert_eq!(hits.len(), 1, "a list membership test in a loop fires once");
    assert_eq!(hits[0].severity, Severity::Warn);
    assert_eq!(
        hits[0].evidence.get("operator").map(String::as_str),
        Some("in")
    );
    assert_eq!(
        hits[0].evidence.get("root").map(String::as_str),
        Some("seq")
    );
}

#[test]
fn loop_007_fires_on_list_index_scan() {
    let units = common::load_units_fixture("PERF-PY-LOOP-007__index-scan.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_007_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a list index scan by induction var fires once"
    );
    assert_eq!(
        hits[0].evidence.get("index_ident").map(String::as_str),
        Some("i")
    );
    assert_eq!(
        hits[0].evidence.get("root").map(String::as_str),
        Some("seq")
    );
}

#[test]
fn loop_009_fires_on_try_inside_loop() {
    let units = common::load_units_fixture("PERF-PY-LOOP-009__try-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_LOOP_009_RULE_ID);
    assert_eq!(hits.len(), 1, "a `try`/`except` inside a loop fires once");
    assert_eq!(
        hits[0].evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
}

#[test]
fn async_001_fires_on_blocking_call_in_async_def() {
    let units = common::load_units_fixture("PERF-PY-ASYNC-001__blocking-in-async.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ASYNC_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a blocking `requests.get` in an `async def` fires once"
    );
    assert_eq!(hits[0].severity, Severity::Warn);
    assert_eq!(
        hits[0].evidence.get("receiver_root").map(String::as_str),
        Some("requests")
    );
}

#[test]
fn async_002_fires_on_await_inside_loop() {
    let units = common::load_units_fixture("PERF-PY-ASYNC-002__await-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ASYNC_002_RULE_ID);
    assert_eq!(hits.len(), 1, "an `await` inside a loop fires once");
    assert_eq!(
        hits[0].evidence.get("loop_depth").map(String::as_str),
        Some("1")
    );
}

#[test]
fn call_001_fires_on_direct_recursion() {
    let units = common::load_units_fixture("PERF-PY-CALL-001__direct-recursion.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_CALL_001_RULE_ID);
    assert_eq!(hits.len(), 1, "a direct self-call fires once");
    assert_eq!(hits[0].confidence, Confidence::High);
    assert_eq!(
        hits[0].evidence.get("function").map(String::as_str),
        Some("walk")
    );
}

#[test]
fn call_002_fires_at_threshold() {
    let units = common::load_units_fixture("PERF-PY-CALL-002__high-complexity.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_CALL_002_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "complexity 15 reaches the threshold and fires once"
    );
    assert_eq!(hits[0].confidence, Confidence::High);
    assert_eq!(
        hits[0].evidence.get("complexity").map(String::as_str),
        Some("15")
    );
}

#[test]
fn comprehension_001_fires_on_single_append() {
    let units = common::load_units_fixture("PERF-PY-COMPREHENSION-001__single-append.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_COMPREHENSION_001_RULE_ID);
    assert_eq!(hits.len(), 1, "a single-statement append loop fires once");
    assert_eq!(
        hits[0].evidence.get("receiver_root").map(String::as_str),
        Some("out")
    );
}

#[test]
fn executor_001_fires_on_executor_in_loop() {
    let units = common::load_units_fixture("PERF-PY-EXECUTOR-001__executor-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_EXECUTOR_001_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "an executor constructed in a loop fires once"
    );
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("ThreadPoolExecutor")
    );
}

#[test]
fn n1_001_fires_on_db_call_in_loop() {
    let units = common::load_units_fixture("PERF-PY-N1-001__db-call-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_N1_001_RULE_ID);
    assert_eq!(hits.len(), 1, "a DB call in a loop fires once");
    assert_eq!(hits[0].severity, Severity::Warn);
    assert_eq!(
        hits[0].evidence.get("family").map(String::as_str),
        Some("db")
    );
    assert_eq!(
        hits[0].evidence.get("receiver_root").map(String::as_str),
        Some("conn")
    );
}

#[test]
fn pandas_001_fires_on_iterrows() {
    let units = common::load_units_fixture("PERF-PY-PANDAS-001__iterrows.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_PANDAS_001_RULE_ID);
    assert_eq!(hits.len(), 1, "row-wise `iterrows` fires once");
    assert_eq!(
        hits[0].evidence.get("callee").map(String::as_str),
        Some("df.iterrows")
    );
}

#[test]
fn pandas_002_fires_on_concat_in_loop() {
    let units = common::load_units_fixture("PERF-PY-PANDAS-002__concat-in-loop.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_PANDAS_002_RULE_ID);
    assert_eq!(hits.len(), 1, "`pd.concat` in a loop fires once");
    assert_eq!(
        hits[0].evidence.get("receiver_root").map(String::as_str),
        Some("pd")
    );
}

#[test]
fn arch_001_fires_on_import_cycle() {
    let units = common::load_units_fixture("PERF-PY-ARCH-001__cycle.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ARCH_001_RULE_ID);
    assert_eq!(hits.len(), 1, "a two-module import cycle fires once");
    assert_eq!(
        hits[0].evidence.get("kind").map(String::as_str),
        Some("cycle")
    );
}

#[test]
fn arch_002_fires_on_hot_component_above_floor() {
    let units = common::load_units_fixture("PERF-PY-ARCH-002__hot-component.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ARCH_002_RULE_ID);
    assert_eq!(
        hits.len(),
        1,
        "a directory aggregating complexity 50 fires once"
    );
    assert_eq!(
        hits[0].evidence.get("complexity_sum").map(String::as_str),
        Some("50")
    );
}

#[test]
fn arch_003_emits_low_confidence_cross_module_edge() {
    let units = common::load_units_fixture("PERF-PY-ARCH-003__cross-module.json");
    let findings = analyze(&units);
    let hits = common::by_rule(&findings, PY_ARCH_003_RULE_ID);
    assert_eq!(hits.len(), 1, "a single other-file name match fires once");
    assert_eq!(hits[0].confidence, Confidence::Low);
    assert_eq!(
        hits[0].evidence.get("callee_name").map(String::as_str),
        Some("helper")
    );
    assert_eq!(
        hits[0].evidence.get("matched_path").map(String::as_str),
        Some("pos/arch_helper.py")
    );
}
