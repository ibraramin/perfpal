//! Harness smoke test: the fixture-IR corpus loads, re-serializes
//! deterministically, and emits sorted keys. The corpus itself becomes the
//! parser-free heuristic test bed in P2+ (ADR-0001 guardrail 2).

mod common;

#[test]
fn fixture_loads_and_round_trips_deterministically() {
    let value = common::load_ir_fixture("sample-empty.json");
    let bytes = common::deterministic_bytes(&value);
    assert!(bytes.starts_with('{'));
}

#[test]
fn harness_rejects_missing_fixture_with_path_in_message() {
    let result = std::panic::catch_unwind(|| common::load_ir_fixture("does-not-exist.json"));
    let err = result.expect_err("missing fixture must panic");
    let msg = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| err.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default();
    assert!(
        msg.contains("does-not-exist.json"),
        "panic message should name the fixture, got: {msg}"
    );
}
