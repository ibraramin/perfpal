//! Shared fixture-IR harness (parser-free heuristic-test support).
//!
//! ADR-0001 guardrail 2: heuristics are tested against a parser-free fixture
//! corpus under `tests/fixtures/ir/` — no tree-sitter binding required. The
//! corpus holds serialized IR documents; strict typing arrives with the IR
//! types in P1, until then fixtures are handled as `serde_json::Value`.
//!
//! Byte-determinism rule (modules.md §6 row 3): any JSON we emit must be
//! byte-identical across runs — object keys sorted, struct fields in
//! declaration order.

// Each integration test binary links this module and uses a subset of it.
#![allow(dead_code)]

use std::fs;
use std::path::Path;

use perfpal::scanner::FileUnit;

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
