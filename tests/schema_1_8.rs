//! metrics.json schema 1.8 language-enum lock (plan `js-ts` B2).
//!
//! Locks the MINOR bump v1.7 → v1.8: both language sites widen to
//! `["c","js","python","rust","ts","tsx"]`, the bundled artifact
//! version/byte-copy advances, and no `required` list grew versus v1.7 — a
//! v1.7-shaped document re-marked `"1.8"` still validates structurally.
//! The v1.7 additive field groups (`files[].path_class`, trip provenance) are
//! retained as regression coverage from the previous lock.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-schema18-{tag}-{}-{nanos}",
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
fn v1_8_artifact_is_retained_as_history() {
    let v18: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.8.json"))
            .expect("v1.8 schema parses");
    assert_eq!(v18["properties"]["schema_version"]["const"], "1.8");
    assert_eq!(
        v18["$id"],
        "https://perfpal.dev/schemas/metrics.schema.v1.8.json"
    );
}

#[test]
fn docs_and_embedded_v1_8_copies_are_byte_identical() {
    let docs = include_str!("../docs/schema/metrics.schema.v1.8.json");
    let src = include_str!("../src/schema/metrics.schema.v1.8.json");
    assert_eq!(docs, src, "src/schema v1.8 copy drifted from docs");
}

#[test]
fn v1_8_widens_language_enums_without_growing_required_lists() {
    let v18: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.8.json"))
            .expect("v1.8 schema parses");
    let v17: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.7.json"))
            .expect("v1.7 schema parses");

    assert_eq!(v18["properties"]["schema_version"]["const"], "1.8");
    assert_eq!(
        v18["$id"],
        "https://perfpal.dev/schemas/metrics.schema.v1.8.json"
    );
    assert_eq!(v18["title"], "Perfpal metrics.json schema v1.8");

    // Both language sites admit exactly the six sorted tokens.
    let six = serde_json::json!(["c", "js", "python", "rust", "ts", "tsx"]);
    assert_eq!(
        v18["properties"]["scan"]["properties"]["languages"]["items"]["enum"],
        six
    );
    assert_eq!(
        v18["properties"]["files"]["items"]["properties"]["language"]["enum"],
        six
    );

    // The zero-file fallback wording stays generic (B2 emits four values,
    // B3 six), and the file-site description names the bump rule.
    let scan_desc = v18["properties"]["scan"]["properties"]["languages"]["description"]
        .as_str()
        .expect("scan.languages description");
    assert!(scan_desc.contains("sorted-unique set of files[].language"));
    assert!(scan_desc.contains("supported set"));
    assert!(
        !scan_desc.contains("tsx"),
        "scan.languages description must not hardcode the enum values"
    );
    let file_desc = v18["properties"]["files"]["items"]["properties"]["language"]["description"]
        .as_str()
        .expect("files[].language description");
    assert!(file_desc.contains("schema 1.8"));
    assert!(file_desc.contains("minor-bump"));

    // Retained v1.7 additive fields (regression coverage from the old lock).
    let file_props = &v18["properties"]["files"]["items"]["properties"];
    assert_eq!(
        file_props["path_class"]["enum"],
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
    let fn_props = &v18["properties"]["stats"]["properties"]["functions"]["items"]["properties"];
    assert_eq!(
        fn_props["trip_count_source"]["enum"],
        serde_json::json!(["literal", "partial", "unknown"])
    );
    assert_eq!(fn_props["known_trip_value"]["type"], "integer");

    // Additive: every required list matches v1.7 exactly.
    assert_required_equal(&v17, &v18, "$");
    // `additionalProperties: false` stays everywhere it was before — assert
    // the root and the two main nested object records explicitly (the full
    // recursive walk below covers every remaining node).
    assert_eq!(v18["additionalProperties"], serde_json::json!(false));
    assert_eq!(
        v18["properties"]["files"]["items"]["additionalProperties"],
        serde_json::json!(false),
        "nested files[].items must keep additionalProperties: false"
    );
    assert_eq!(
        v18["properties"]["stats"]["properties"]["functions"]["items"]["additionalProperties"],
        serde_json::json!(false),
        "nested stats.functions[].items must keep additionalProperties: false"
    );

    // Historical copies retained AND parseable.
    for (name, version) in [
        ("metrics.schema.v1.7.json", "1.7"),
        ("metrics.schema.v1.6.json", "1.6"),
        ("metrics.schema.v1.5.json", "1.5"),
        ("metrics.schema.v1.4.json", "1.4"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/schema")
            .join(name);
        assert!(path.exists(), "{name} must remain as retained history");
        let historical: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).expect("read historical artifact"))
                .expect("historical artifact parses");
        assert_eq!(
            historical["properties"]["schema_version"]["const"], version,
            "{name} must retain its own version marker"
        );
    }
}

/// A v1.7-shaped document carries only fields the v1.7 schema required (plus
/// none of the optional v1.7 additions); re-marking its `schema_version` to
/// `"1.8"` is the only change. It must still validate against the v1.8
/// artifact: same required lists, same types, language tokens inside the
/// widened enum. This mirrors the v1.6 → v1.7 additive proof.
#[test]
fn v1_7_document_re_marked_as_v1_8_still_validates() {
    let mut doc = serde_json::json!({
        "schema_version": "1.7",
        "tool": { "name": "perfpal", "version": "0.2.0" },
        "scan": {
            "root_path": ".",
            "started_at": "2026-10-24T00:00:00Z",
            "languages": ["c", "python"],
            "file_count": 1
        },
        "files": [{
            "path": "a.c",
            "language": "c",
            "loc": { "code": 1, "comment": 0, "blank": 0 },
            "function_count": 0,
            "global_var_count": 0,
            "include_counts": { "quoted": 0, "angle": 0 },
            "parse_health": { "status": "ok", "error_node_count": 0 }
        }],
        "stats": { "functions": [] },
        "findings": []
    });
    // v1.7's language vocabulary is a subset of v1.8's — the bump only widens.
    doc["schema_version"] = serde_json::json!("1.8");

    let v18: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.8.json"))
            .expect("v1.8 schema parses");
    assert_document_validates(&v18, &doc, "$");
}

/// Minimal structural validator (the repo carries no JSON-Schema engine):
/// required keys present, enum values admitted, and no unknown property where
/// `additionalProperties: false` holds. `$ref` is not exercised by the
/// minimal document above.
fn assert_document_validates(schema: &serde_json::Value, doc: &serde_json::Value, at: &str) {
    if let Some(required) = schema.get("required").and_then(serde_json::Value::as_array) {
        let obj = doc
            .as_object()
            .unwrap_or_else(|| panic!("{at} must be an object to satisfy required"));
        for key in required {
            let key = key.as_str().expect("required key is a string");
            assert!(obj.contains_key(key), "required key {at}.{key} missing");
        }
    }
    if let Some(enum_values) = schema.get("enum").and_then(serde_json::Value::as_array) {
        assert!(
            enum_values.contains(doc),
            "{at} value {doc} is outside the closed enum"
        );
    }
    if let Some(properties) = schema
        .get("properties")
        .and_then(serde_json::Value::as_object)
    {
        let obj = doc
            .as_object()
            .unwrap_or_else(|| panic!("{at} must be an object"));
        for (key, value) in obj {
            if let Some(sub) = properties.get(key) {
                assert_document_validates(sub, value, &format!("{at}.{key}"));
            } else {
                assert_ne!(
                    schema["additionalProperties"],
                    serde_json::json!(false),
                    "unknown property {at}.{key} is not admitted"
                );
            }
        }
    }
    if let (Some(items), Some(array)) = (schema.get("items"), doc.as_array()) {
        for (i, item) in array.iter().enumerate() {
            assert_document_validates(items, item, &format!("{at}[{i}]"));
        }
    }
}

/// Recursively assert every `required` array is identical between two schema
/// documents (v1.8 must be strictly additive).
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
            // New v1.8-only objects must not smuggle in a `required` list.
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

/// Recursively reject any `required` list inside a v1.8-only subtree.
fn assert_no_required(value: &serde_json::Value, at: &str) {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key("required") {
                panic!("new v1.8-only object at {at} introduces a required list");
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
fn required_checker_flags_new_v18_only_required() {
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
    assert_eq!(doc["schema_version"], "1.9");

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
