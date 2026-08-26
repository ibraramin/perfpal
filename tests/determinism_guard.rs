//! Behavioral guard for the byte-determinism decision (modules.md §6 row 3):
//! serde_json must run on DEFAULT features — `preserve_order` stays OFF so
//! object keys serialize in sorted (BTreeMap) order regardless of insertion
//! order.
//!
//! If anyone enables `preserve_order`, these tests fail immediately.

use serde_json::{Map, Value};

#[test]
fn object_keys_serialize_sorted_regardless_of_insertion_order() {
    let mut map = Map::new();
    map.insert("zebra".into(), Value::from(1));
    map.insert("alpha".into(), Value::from(2));
    map.insert("middle".into(), Value::from(3));

    let out = serde_json::to_string(&Value::Object(map)).expect("serialize");

    // Insertion order was zebra,alpha,middle; output MUST be alpha,middle,zebra.
    assert_eq!(out, r#"{"alpha":2,"middle":3,"zebra":1}"#);
}

#[test]
fn two_serialization_passes_are_byte_identical() {
    let mut map = Map::new();
    for key in ["q", "b", "k", "a", "z", "m"] {
        map.insert(key.into(), Value::from(key.len()));
    }
    let value = Value::Object(map);
    let first = serde_json::to_string(&value).expect("serialize pass 1");
    let second = serde_json::to_string(&value).expect("serialize pass 2");
    assert_eq!(first, second);
}

#[test]
fn nested_structures_stay_deterministic() {
    let inner: Value = serde_json::from_str(r#"{"y": [3, 1, {"n": 1, "m": 2}], "x": true}"#)
        .expect("inner literal");
    let outer = serde_json::json!({ "wrap": inner.clone(), "aa": [inner.clone(), inner] });
    let bytes: Vec<String> = (0..4)
        .map(|_| serde_json::to_string(&outer).expect("serialize"))
        .collect();
    assert!(
        bytes.windows(2).all(|w| w[0] == w[1]),
        "repeated serialization diverged"
    );
    // Keys sorted at every nesting level (aa<wrap, x<y, m<n) regardless of
    // the order they appeared in the source literals.
    assert_eq!(
        bytes[0],
        r#"{"aa":[{"x":true,"y":[3,1,{"m":2,"n":1}]},{"x":true,"y":[3,1,{"m":2,"n":1}]}],"wrap":{"x":true,"y":[3,1,{"m":2,"n":1}]}}"#
    );
}
