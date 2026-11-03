//! metrics.json schema 1.9 lock (structural suite v1).
//!
//! Locks the MINOR bump v1.8 → v1.9: the three new function-level fields
//! (`loop_count`, `loop_body_loc`, `parse_health`) are OPTIONAL, every
//! `required` list is unchanged, both bundled copies are byte-identical, and a
//! 1.8-shaped document re-marked `"1.9"` still validates structurally.

use std::path::Path;

use perfpal::schema;

#[test]
fn version_and_artifact_name_are_1_9() {
    assert_eq!(schema::SCHEMA_VERSION, "1.9");
    assert_eq!(schema::SCHEMA_ARTIFACT_NAME, "metrics.schema.v1.9.json");
    assert_eq!(
        schema::SCHEMA_ARTIFACT_NAME,
        format!("metrics.schema.v{}.json", schema::SCHEMA_VERSION)
    );
}

#[test]
fn docs_and_embedded_v1_9_copies_are_byte_identical() {
    let docs = include_str!("../docs/schema/metrics.schema.v1.9.json");
    let src = include_str!("../src/schema/metrics.schema.v1.9.json");
    assert_eq!(docs, src, "src/schema v1.9 copy drifted from docs");
    assert_eq!(schema::EMBEDDED_SCHEMA, docs, "embedded schema drifted");
    // The emitted artifact name resolves to an actual bundled file.
    let artifact = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs/schema")
        .join(schema::SCHEMA_ARTIFACT_NAME);
    assert!(artifact.exists(), "artifacts must carry the full version");
}

#[test]
fn v1_9_is_additive_without_growing_required_lists() {
    let v19: serde_json::Value =
        serde_json::from_str(schema::EMBEDDED_SCHEMA).expect("v1.9 schema parses");
    let v18: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.8.json"))
            .expect("v1.8 schema parses");

    assert_eq!(v19["properties"]["schema_version"]["const"], "1.9");
    assert_eq!(
        v19["$id"],
        "https://perfpal.dev/schemas/metrics.schema.v1.9.json"
    );
    assert_eq!(v19["title"], "Perfpal metrics.json schema v1.9");

    // No required list anywhere grew (or shrank) relative to v1.8.
    assert_required_equal(&v18, &v19, "$");

    let fn_props = &v19["properties"]["stats"]["properties"]["functions"]["items"]["properties"];
    // New scalars are integers >= 0.
    assert_eq!(fn_props["loop_count"]["type"], "integer");
    assert_eq!(fn_props["loop_count"]["minimum"], 0);
    assert_eq!(fn_props["loop_body_loc"]["type"], "integer");
    assert_eq!(fn_props["loop_body_loc"]["minimum"], 0);
    // New parse_health object: closed status enum, required pair, no extras.
    let health = &fn_props["parse_health"];
    assert_eq!(health["type"], "object");
    assert_eq!(
        health["required"],
        serde_json::json!(["status", "overlapping_error_regions"])
    );
    assert_eq!(health["additionalProperties"], serde_json::json!(false));
    assert_eq!(
        health["properties"]["status"]["enum"],
        serde_json::json!(["ok", "partial", "failed"])
    );
    assert_eq!(
        health["properties"]["overlapping_error_regions"]["type"],
        "integer"
    );
    assert_eq!(
        health["properties"]["overlapping_error_regions"]["minimum"],
        0
    );

    // Stage 2 resource-action fields: integer >= 0 / array of closed-shape
    // site objects.
    assert_eq!(fn_props["alloc_sites_in_loops"]["type"], "integer");
    assert_eq!(fn_props["alloc_sites_in_loops"]["minimum"], 0);
    assert_eq!(fn_props["io_site_count"]["type"], "integer");
    assert_eq!(fn_props["io_sites_in_loops"]["type"], "integer");
    let sites = &fn_props["resource_sites"];
    assert_eq!(sites["type"], "array");
    let site = &sites["items"];
    assert_eq!(
        site["required"],
        serde_json::json!([
            "kind",
            "line",
            "column",
            "loop_depth",
            "cfg_gated",
            "language"
        ])
    );
    assert_eq!(site["additionalProperties"], serde_json::json!(false));
    assert_eq!(
        site["properties"]["kind"]["enum"],
        serde_json::json!(["alloc", "io"])
    );
    assert_eq!(site["properties"]["line"]["minimum"], 1);
    assert_eq!(site["properties"]["column"]["minimum"], 0);
    assert_eq!(site["properties"]["loop_depth"]["minimum"], 1);
    assert_eq!(
        site["properties"]["cfg_gated"]["type"],
        serde_json::json!(["boolean", "null"])
    );
    assert_eq!(
        site["properties"]["language"]["enum"],
        serde_json::json!(["c", "rust", "python", "javascript", "typescript"])
    );

    // The new function fields are NOT required (additive contract).
    let required = &v19["properties"]["stats"]["properties"]["functions"]["items"]["required"];
    for new_field in [
        "loop_count",
        "loop_body_loc",
        "parse_health",
        "alloc_sites_in_loops",
        "io_site_count",
        "io_sites_in_loops",
        "resource_sites",
    ] {
        assert!(
            !required
                .as_array()
                .expect("functions required[]")
                .iter()
                .any(|v| v == new_field),
            "{new_field} must remain optional"
        );
    }

    // additionalProperties: false stays on the function record.
    assert_eq!(
        v19["properties"]["stats"]["properties"]["functions"]["items"]["additionalProperties"],
        serde_json::json!(false)
    );

    // The previous artifact stays as retained history.
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/schema/metrics.schema.v1.8.json")
            .exists()
    );
}

/// A minimal 1.8-shaped document (no v1.9 fields at all) re-marked `"1.9"`
/// validates against the v1.9 schema: no new field is required.
#[test]
fn v1_8_document_re_marked_as_v1_9_still_validates() {
    let mut doc = serde_json::json!({
        "schema_version": "1.8",
        "tool": { "name": "perfpal", "version": "0.4.0" },
        "scan": {
            "root_path": ".",
            "started_at": "2026-11-04T00:00:00Z",
            "languages": ["c", "python"],
            "file_count": 1
        },
        "files": [{
            "path": "a.c",
            "language": "c",
            "loc": { "code": 1, "comment": 0, "blank": 0 },
            "function_count": 1,
            "global_var_count": 0,
            "include_counts": { "quoted": 0, "angle": 0 },
            "parse_health": { "status": "ok", "error_node_count": 0 }
        }],
        "stats": {
            "functions": [{
                "id": "a.c#f#1:1",
                "name": "f",
                "path": "a.c",
                "span": { "start_line": 1, "start_col": 1, "end_line": 1, "end_col": 2 },
                "param_count": 0,
                "cyclomatic_complexity": 1,
                "cognitive_complexity": 0,
                "max_loop_depth": 0,
                "alloc_site_count": 0,
                "length_lines": 1
            }]
        },
        "findings": []
    });
    doc["schema_version"] = serde_json::json!("1.9");

    let v19: serde_json::Value =
        serde_json::from_str(schema::EMBEDDED_SCHEMA).expect("v1.9 schema parses");
    assert_document_validates(&v19, &doc, "$");
}

/// A document carrying the new fields also validates (types/enums correct).
#[test]
fn v1_9_fields_validate_when_present() {
    let doc = serde_json::json!({
        "schema_version": "1.9",
        "tool": { "name": "perfpal", "version": "0.5.0" },
        "scan": {
            "root_path": ".",
            "started_at": "2026-11-04T00:00:00Z",
            "languages": ["c"],
            "file_count": 1
        },
        "files": [{
            "path": "a.c",
            "language": "c",
            "loc": { "code": 1, "comment": 0, "blank": 0 },
            "function_count": 1,
            "global_var_count": 0,
            "include_counts": { "quoted": 0, "angle": 0 },
            "parse_health": { "status": "ok", "error_node_count": 0 }
        }],
        "stats": {
            "functions": [{
                "id": "a.c#f#1:1",
                "name": "f",
                "path": "a.c",
                "span": { "start_line": 1, "start_col": 1, "end_line": 4, "end_col": 2 },
                "param_count": 0,
                "cyclomatic_complexity": 2,
                "cognitive_complexity": 1,
                "max_loop_depth": 1,
                "loop_count": 1,
                "loop_body_loc": 2,
                "parse_health": { "status": "partial", "overlapping_error_regions": 1 },
                "alloc_sites_in_loops": 1,
                "io_site_count": 2,
                "io_sites_in_loops": 1,
                "resource_sites": [
                    {
                        "kind": "alloc",
                        "line": 2,
                        "column": 4,
                        "loop_depth": 1,
                        "cfg_gated": true,
                        "language": "c"
                    },
                    {
                        "kind": "io",
                        "line": 3,
                        "column": 4,
                        "loop_depth": 1,
                        "cfg_gated": null,
                        "language": "rust"
                    }
                ],
                "alloc_site_count": 1,
                "length_lines": 4
            }]
        },
        "findings": []
    });
    let v19: serde_json::Value =
        serde_json::from_str(schema::EMBEDDED_SCHEMA).expect("v1.9 schema parses");
    assert_document_validates(&v19, &doc, "$");
}

/// Minimal structural validator (the repo carries no JSON-Schema engine):
/// required keys present, enum values admitted, and no unknown property where
/// `additionalProperties: false` holds. `$ref` is not exercised here.
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
/// documents (v1.9 must be strictly additive).
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
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            let len = a.len().max(b.len());
            for i in 0..len {
                if let (Some(x), Some(y)) = (a.get(i), b.get(i)) {
                    assert_required_equal(x, y, &format!("{at}[{i}]"));
                }
            }
        }
        _ => {}
    }
}
