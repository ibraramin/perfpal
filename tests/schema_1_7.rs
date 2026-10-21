//! metrics.json schema 1.7 additive lock (plan `map-intelligibility` V4).
//!
//! Locks the two additive field groups — `files[].path_class` and
//! `stats.functions[].trip_count_source` + `known_trip_value` — the bundled
//! artifact version/byte-copy, and that no `required` list grew versus v1.6.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;
use perfpal::schema;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-schema17-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

#[test]
fn version_and_artifact_name_are_1_7() {
    assert_eq!(schema::SCHEMA_VERSION, "1.7");
    assert_eq!(schema::SCHEMA_ARTIFACT_NAME, "metrics.schema.v1.7.json");
    assert_eq!(
        schema::SCHEMA_ARTIFACT_NAME,
        format!("metrics.schema.v{}.json", schema::SCHEMA_VERSION)
    );
}

#[test]
fn docs_and_embedded_v1_7_copies_are_byte_identical() {
    let docs = include_str!("../docs/schema/metrics.schema.v1.7.json");
    let src = include_str!("../src/schema/metrics.schema.v1.7.json");
    assert_eq!(docs, src, "src/schema v1.7 copy drifted from docs");
    assert_eq!(schema::EMBEDDED_SCHEMA, docs, "embedded schema drifted");
}

#[test]
fn v1_7_declares_optional_fields_without_growing_required_lists() {
    let v17: serde_json::Value =
        serde_json::from_str(schema::EMBEDDED_SCHEMA).expect("v1.7 schema parses");
    let v16: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.6.json"))
            .expect("v1.6 schema parses");

    assert_eq!(v17["properties"]["schema_version"]["const"], "1.7");
    assert_eq!(
        v17["$id"],
        "https://perfpal.dev/schemas/metrics.schema.v1.7.json"
    );
    assert_eq!(v17["title"], "Perfpal metrics.json schema v1.7");

    // Additive properties only.
    let file_props = &v17["properties"]["files"]["items"]["properties"];
    let path_class = &file_props["path_class"];
    assert_eq!(
        path_class["enum"],
        serde_json::json!([
            "production",
            "test",
            "example",
            "bench",
            "fixture",
            "generated",
            "vendor",
            "unknown"
        ])
    );
    assert!(
        path_class["description"]
            .as_str()
            .expect("description")
            .contains("scripts/"),
        "tooling rule must be documented"
    );

    let fn_props = &v17["properties"]["stats"]["properties"]["functions"]["items"]["properties"];
    assert_eq!(
        fn_props["trip_count_source"]["enum"],
        serde_json::json!(["literal", "partial", "unknown"])
    );
    assert_eq!(fn_props["known_trip_value"]["type"], "integer");
    assert_eq!(fn_props["known_trip_value"]["minimum"], 0);

    // Additive: every required list matches v1.6 exactly.
    assert_required_equal(&v16, &v17, "$");
    // `additionalProperties: false` stays everywhere it was before.
    assert_eq!(v17["additionalProperties"], serde_json::json!(false));

    // Historical copies retained.
    for name in [
        "metrics.schema.v1.6.json",
        "metrics.schema.v1.5.json",
        "metrics.schema.v1.4.json",
    ] {
        assert!(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/schema")
                .join(name)
                .exists(),
            "{name} must remain as retained history"
        );
    }
}

/// Recursively assert every `required` array is identical between two schema
/// documents (v1.7 must be strictly additive).
fn assert_required_equal(old: &serde_json::Value, new: &serde_json::Value, at: &str) {
    match (old, new) {
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            match (a.get("required"), b.get("required")) {
                (Some(x), Some(y)) => assert_eq!(x, y, "required list drifted at {at}.required"),
                (None, None) => {}
                _ => panic!("required presence drifted at {at}.required"),
            }
            for (key, value) in a {
                if let Some(other) = b.get(key) {
                    assert_required_equal(value, other, &format!("{at}.{key}"));
                }
            }
            // New v1.7-only objects must not smuggle in a `required` list.
            for (key, value) in b {
                if !a.contains_key(key) {
                    assert_no_required(value, &format!("{at}.{key}"));
                }
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            let len = a.len().max(b.len());
            for i in 0..len {
                match (a.get(i), b.get(i)) {
                    (Some(x), Some(y)) => {
                        assert_required_equal(x, y, &format!("{at}[{i}]"));
                    }
                    (None, Some(y)) => assert_no_required(y, &format!("{at}[{i}]")),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Recursively reject any `required` list inside a v1.7-only subtree.
fn assert_no_required(value: &serde_json::Value, at: &str) {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key("required") {
                panic!("new v1.7-only object at {at} introduces a required list");
            }
            for (key, child) in map {
                assert_no_required(child, &format!("{at}.{key}"));
            }
        }
        serde_json::Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                assert_no_required(child, &format!("{at}[{i}]"));
            }
        }
        _ => {}
    }
}

#[test]
#[should_panic(expected = "introduces a required list")]
fn required_checker_flags_new_v17_only_required() {
    let old = serde_json::json!({"$defs": {"a": {}}});
    let new = serde_json::json!({"$defs": {"a": {}, "b": {"required": ["x"]}}});
    assert_required_equal(&old, &new, "$");
}

#[test]
fn end_to_end_scan_emits_path_class_and_trip_provenance() {
    let base = temp_dir("e2e");
    let root = base.join("repo");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::create_dir_all(root.join("examples")).unwrap();
    fs::create_dir_all(root.join("compat")).unwrap();
    fs::write(
        root.join("src/main.c"),
        "#include <stdlib.h>\n\
         int sum(void) {\n\
         \x20   int s = 0;\n\
         \x20   for (int i = 0; i < 10; i++) { s += i; }\n\
         \x20   return s;\n\
         }\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/test_main.c"),
        "int t(void) { return 0; }\n",
    )
    .unwrap();
    fs::write(root.join("examples/demo.c"), "int e(void) { return 0; }\n").unwrap();
    fs::write(root.join("compat/lib.c"), "int c(void) { return 0; }\n").unwrap();

    let out = base.join("out");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        root.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "scan must succeed");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");
    assert_eq!(doc["schema_version"], "1.7");

    let files = doc["files"].as_array().expect("files[]");
    assert!(!files.is_empty());
    for f in files {
        let path = f["path"].as_str().unwrap();
        let class = f["path_class"]
            .as_str()
            .unwrap_or_else(|| panic!("missing path_class for {path}"));
        assert_ne!(
            class, "unknown",
            "computed files never emit unknown: {path}"
        );
    }
    let class_of = |want: &str| {
        files
            .iter()
            .find(|f| f["path"] == want)
            .and_then(|f| f["path_class"].as_str())
            .unwrap_or_else(|| panic!("missing file {want}"))
            .to_string()
    };
    assert_eq!(class_of("src/main.c"), "production");
    assert_eq!(class_of("tests/test_main.c"), "test");
    assert_eq!(class_of("examples/demo.c"), "example");
    assert_eq!(class_of("compat/lib.c"), "production");

    let functions = doc["stats"]["functions"].as_array().expect("functions[]");
    let any_loop = functions
        .iter()
        .any(|f| f["max_loop_depth"].as_u64().unwrap_or(0) > 0);
    assert!(any_loop, "fixture must contain a loop-bearing function");
    for f in functions {
        if f["max_loop_depth"].as_u64().unwrap_or(0) > 0 {
            let source = f["trip_count_source"]
                .as_str()
                .unwrap_or_else(|| panic!("loop function without trip_count_source: {f}"));
            assert!(
                matches!(source, "literal" | "partial" | "unknown"),
                "closed trip_count_source: {source}"
            );
        }
    }
    let sum = functions
        .iter()
        .find(|f| f["name"] == "sum")
        .expect("sum() present");
    assert_eq!(sum["trip_count_source"], "literal");
    assert_eq!(sum["known_trip_value"], 10);
}
