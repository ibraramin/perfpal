//! metrics.json schema 1.6 end-to-end lock (plan `map-build` I1a C + I1b
//! Rust + I1c Python + I2 cost-v2): the v1.5 entry/reachability fields plus
//! the ADDITIVE v1.6 `function_cost_v2` / `unknown_trip_loops` fields —
//! additive-only over 1.5/1.4, so every `required` list stays byte-equal to
//! the retained artifacts'. Retains the 1.4 locks below as history: the seven
//! v1.4 instrument fields + populated distributions (tests (6)–(7)), the
//! language enum + structural finding-id locks (tests (1)–(5)).
//!
//! History: the `"python"` language token arrived with the 1.2 → 1.3 bump
//! (plan `python-backend`); its locks below are RETAINED unchanged.
//! `scan.languages` derives from the scanned files (sorted-unique, supported
//! set on a zero-file scan) and `files[].language` admits `"python"`, all
//! asserted through the REAL pipeline (`cli::run_with`) over temp trees.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-schema13-{tag}-{}-{nanos}",
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

/// Run the real pipeline over `target` into a fresh report dir and return the
/// parsed metrics.json document.
fn run_pipeline_and_parse(tag: &str, target: &std::path::Path) -> serde_json::Value {
    let out = temp_dir(tag).join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        target.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "pipeline over {target:?} must exit 0");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    serde_json::from_str(&raw).expect("metrics.json parses as JSON")
}

/// (1) Mixed C+Rust tree → derived `["c","rust"]`; `files[]` carries both
/// language tokens.
#[test]
fn mixed_tree_derives_c_and_rust_languages() {
    let root = temp_dir("mixed");
    fs::write(root.join("a.c"), "int main(void) { return 0; }\n").expect("write a.c");
    fs::write(root.join("b.rs"), "fn main() {}\n").expect("write b.rs");

    let doc = run_pipeline_and_parse("mixed", &root);
    assert_eq!(doc["scan"]["languages"], serde_json::json!(["c", "rust"]));

    let files = doc["files"].as_array().expect("files[] present");
    let langs: Vec<&str> = files
        .iter()
        .map(|f| f["language"].as_str().expect("language token"))
        .collect();
    assert_eq!(langs, ["c", "rust"], "files sorted by path: a.c, b.rs");
}

/// (2) Rust-only tree → derived `["rust"]`; the sole file's language is
/// `"rust"`.
#[test]
fn rust_only_tree_derives_rust_language() {
    let root = temp_dir("rust-only");
    fs::write(root.join("lib.rs"), "pub fn helper() -> i32 { 1 }\n").expect("write lib.rs");

    let doc = run_pipeline_and_parse("rust-only", &root);
    assert_eq!(doc["scan"]["languages"], serde_json::json!(["rust"]));
    assert_eq!(doc["files"][0]["language"], "rust");
}

/// (3) Empty dir → exit 0 with the supported-set fallback (`minItems: 1`
/// preserved; semantics = "languages scanned for"). Since the 1.3 bump the
/// supported set is `["c","python","rust"]` (the `SUPPORTED_LANGUAGES` order);
// v1.4 retains it unchanged.
#[test]
fn empty_dir_scan_falls_back_to_supported_set() {
    let root = temp_dir("empty");
    let doc = run_pipeline_and_parse("empty", &root);
    assert_eq!(
        doc["scan"]["languages"],
        serde_json::json!(["c", "python", "rust"])
    );
    assert_eq!(doc["scan"]["file_count"], 0);
}

/// (4) Schema lock: the embedded canonical v1.6 schema admits exactly
/// `["c","rust","python"]` at both language sites; the retained v1.4, v1.3,
/// v1.2 and v1.1 artifacts still pin their own version markers and language
/// values (proving the history copies are kept intact, not silently
/// overwritten).
#[test]
fn schema_1_6_admits_c_rust_and_python_language_values() {
    let schema: serde_json::Value =
        serde_json::from_str(perfpal::schema::EMBEDDED_SCHEMA).expect("embedded schema parses");

    let langs_enum =
        schema["properties"]["scan"]["properties"]["languages"]["items"]["enum"].clone();
    assert_eq!(langs_enum, serde_json::json!(["c", "rust", "python"]));

    let file_lang_enum =
        schema["properties"]["files"]["items"]["properties"]["language"]["enum"].clone();
    assert_eq!(file_lang_enum, serde_json::json!(["c", "rust", "python"]));

    // Retained history: v1.4 artifact intact (const "1.4", declaring the
    // seven v1.4 instrument fields but NOT the v1.5 entry fields).
    let v1_4: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.4.json"))
            .expect("v1.4 schema parses");
    assert_eq!(v1_4["properties"]["schema_version"]["const"], "1.4");
    let v1_4_props = &v1_4["properties"]["stats"]["properties"]["functions"]["items"]["properties"];
    assert!(v1_4_props.get("call_fanout").is_some());
    assert!(
        v1_4_props.get("entry_kind").is_none(),
        "v1.4 must not declare the v1.5 entry fields"
    );

    // Retained history: v1.3 artifact intact (const "1.3", enum
    // ["c","rust","python"]).
    let v1_3: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.3.json"))
            .expect("v1.3 schema parses");
    assert_eq!(v1_3["properties"]["schema_version"]["const"], "1.3");
    assert_eq!(
        v1_3["properties"]["files"]["items"]["properties"]["language"]["enum"],
        serde_json::json!(["c", "rust", "python"])
    );

    // Retained history: v1.2 artifact intact (enum ["c","rust"]).
    let v1_2: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.2.json"))
            .expect("v1.2 schema parses");
    assert_eq!(v1_2["properties"]["schema_version"]["const"], "1.2");
    assert_eq!(
        v1_2["properties"]["files"]["items"]["properties"]["language"]["enum"],
        serde_json::json!(["c", "rust"])
    );

    // Retained history: v1.1 artifact intact (const "c").
    let v1_1: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.1.json"))
            .expect("v1.1 schema parses");
    assert_eq!(v1_1["properties"]["schema_version"]["const"], "1.1");
    assert_eq!(
        v1_1["properties"]["files"]["items"]["properties"]["language"]["const"],
        "c"
    );
}

// ---------------------------------------------------------------------------
// (5) v1.6 bump lock (plan `map-build` I2): two additive cost-v2 fields on
// top of the v1.5 entry/reachability trio. No runtime JSON-Schema validator is
// available in-repo — Cargo.toml is dependency-frozen and carries no
// `jsonschema`/`valico`-style crate — so the v1.4 `rule_id`/`finding_id`
// patterns and the `files[].language` enum are asserted STRUCTURALLY here
// against the bundled artifact (retained verbatim from the v1.3 lock).
// ---------------------------------------------------------------------------

/// C source producing at least one `PERF-C-*` finding (alloc + free in loop).
const LOCK_C_SRC: &str = r#"#include <stdlib.h>

void churn(int n) {
    int i;
    for (i = 0; i < n; i++) {
        char *buf = malloc(16);
        free(buf);
    }
}
"#;

/// Rust source producing at least one `PERF-RS-*` finding (alloc in loop).
const LOCK_RS_SRC: &str = r#"pub fn churn(parts: &[&str]) -> String {
    let mut s = String::new();
    for p in parts {
        let tmp: Vec<u8> = Vec::new();
        s.push_str(p);
        let _ = tmp.len();
    }
    s
}
"#;

/// `[A-Z]+` run.
fn is_upper_run(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_uppercase())
}

/// `[0-9]+` run (finding-id line/col).
fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `[0-9]{3}` run (rule-id numeric suffix).
fn is_three_digits(s: &str) -> bool {
    s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit())
}

/// Structural match for the v1.4 `rule_id` pattern
/// `^PERF-[A-Z]+(-[A-Z]+)?-[0-9]{3}$`.
fn matches_rule_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("PERF-") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('-').collect();
    match parts.as_slice() {
        [lang, num] => is_upper_run(lang) && is_three_digits(num),
        [lang, family, num] => is_upper_run(lang) && is_upper_run(family) && is_three_digits(num),
        _ => false,
    }
}

/// Structural match for the v1.4 `finding_id` pattern
/// `^PERF-[A-Z]+(-[A-Z]+)?-[0-9]{3}:.+:[0-9]+:[0-9]+$`. Parsed from the RIGHT
/// because paths may legally contain colons.
fn matches_finding_id(s: &str) -> bool {
    let mut parts = s.rsplitn(3, ':');
    let (Some(col), Some(line), Some(head)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if !is_digits(col) || !is_digits(line) {
        return false;
    }
    let Some((rule_id, path)) = head.split_once(':') else {
        return false;
    };
    !path.is_empty() && matches_rule_id(rule_id)
}

#[test]
fn v1_6_bump_and_findings_validate_structurally() {
    // v1.7 is an additive minor bump: the version marker and bundled artifact
    // name advance together, and v1.6/v1.5/v1.4 remain on disk as retained history.
    assert_eq!(perfpal::schema::SCHEMA_VERSION, "1.7");
    assert_eq!(
        perfpal::schema::SCHEMA_ARTIFACT_NAME,
        "metrics.schema.v1.7.json"
    );
    assert_eq!(
        perfpal::schema::SCHEMA_ARTIFACT_NAME,
        format!("metrics.schema.v{}.json", perfpal::schema::SCHEMA_VERSION),
        "artifact name must stay derived from SCHEMA_VERSION"
    );
    // The canonical v1.7 docs artifact is present and active.
    let canonical: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.7.json"))
            .expect("canonical v1.7 schema parses");
    assert_eq!(canonical["properties"]["schema_version"]["const"], "1.7");
    // v1.6/v1.5/v1.4 are retained as history — the bump did not overwrite them.
    for retained in [
        "metrics.schema.v1.6.json",
        "metrics.schema.v1.5.json",
        "metrics.schema.v1.4.json",
    ] {
        assert!(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("docs/schema")
                .join(retained)
                .exists(),
            "{retained} must remain as retained history"
        );
    }
    // The active v1.7 artifact file IS emitted.
    assert!(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/schema/metrics.schema.v1.7.json")
            .exists(),
        "the v1.7 artifact must be emitted by this bump"
    );

    // A REAL mixed-language document: Rust findings present so the structural
    // pattern lock exercises the new PERF-RS-* ids, not just PERF-C-*.
    let root = temp_dir("schema-lock");
    fs::write(root.join("c_src.c"), LOCK_C_SRC).expect("write c_src.c");
    fs::write(root.join("rs_src.rs"), LOCK_RS_SRC).expect("write rs_src.rs");
    let doc = run_pipeline_and_parse("schema-lock", &root);

    assert_eq!(doc["schema_version"], "1.7", "v1.7 bump in effect");

    // files[].language within the v1.4 enum.
    let files = doc["files"].as_array().expect("files[] present");
    for f in files {
        let lang = f["language"].as_str().expect("language token");
        assert!(
            matches!(lang, "c" | "rust" | "python"),
            "files[].language {lang:?} outside the v1.4 enum"
        );
    }
    // scan.languages within the same enum.
    for lang in doc["scan"]["languages"].as_array().expect("languages[]") {
        let lang = lang.as_str().expect("language token");
        assert!(
            matches!(lang, "c" | "rust" | "python"),
            "scan.languages entry {lang:?} outside the v1.4 enum"
        );
    }

    // Every finding must be a structurally valid v1.4 record AND carry the
    // language-matching id prefix; at least one PERF-RS-* must be present.
    let findings = doc["findings"].as_array().expect("findings[] present");
    assert!(!findings.is_empty(), "mixed lock tree must emit findings");
    let mut rs_seen = false;
    for f in findings {
        let rule_id = f["rule_id"].as_str().expect("rule_id");
        let finding_id = f["finding_id"].as_str().expect("finding_id");
        assert!(
            matches_rule_id(rule_id),
            "rule_id {rule_id:?} violates the v1.4 pattern"
        );
        assert!(
            matches_finding_id(finding_id),
            "finding_id {finding_id:?} violates the v1.4 pattern"
        );
        assert!(
            finding_id.starts_with(rule_id),
            "finding_id {finding_id:?} must be prefixed by rule_id {rule_id:?}"
        );
        let path = f["path"].as_str().expect("path");
        if path.ends_with(".rs") {
            assert!(rule_id.starts_with("PERF-RS-"), "{rule_id} on {path}");
            rs_seen = true;
        } else {
            assert!(rule_id.starts_with("PERF-C-"), "{rule_id} on {path}");
        }
    }
    assert!(
        rs_seen,
        "mixed document must carry PERF-RS-* findings for the structural lock"
    );

    fs::remove_dir_all(&root).ok();
}

/// (6) v1.6 additive-only lock: every `required` list in the v1.6 artifact is
/// byte-equal to the corresponding v1.5, v1.4 AND v1.3 lists — the two cost-v2
/// fields arrive as additions with a serde default, so v1.5/v1.4/v1.3
/// metrics.json documents still parse unchanged.
#[test]
fn v1_6_required_lists_match_v1_5_v1_4_and_v1_3_exactly() {
    let v1_6: serde_json::Value =
        serde_json::from_str(perfpal::schema::EMBEDDED_SCHEMA).expect("embedded schema parses");
    let v1_5: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.5.json"))
            .expect("v1.5 schema parses");
    let v1_4: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.4.json"))
            .expect("v1.4 schema parses");
    let v1_3: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.3.json"))
            .expect("v1.3 schema parses");
    assert_required_equal(&v1_5, &v1_6, "$");
    assert_required_equal(&v1_4, &v1_6, "$");
    assert_required_equal(&v1_3, &v1_6, "$");

    // The v1.5 entry trio AND the two v1.6 cost-v2 fields ARE declared — as
    // optional properties of stats.functions items.
    let props = &v1_6["properties"]["stats"]["properties"]["functions"]["items"]["properties"];
    for field in [
        "entry_kind",
        "entry_confidence",
        "reachable_from_entry",
        "function_cost_v2",
        "unknown_trip_loops",
    ] {
        assert!(
            props.get(field).is_some(),
            "v1.6 must declare optional stats.functions[] field {field}"
        );
    }
    // Closed value sets on the wire (`script` joined in I1c for Python
    // guarded modules — retained in the v1.6 artifact).
    assert_eq!(
        props["entry_kind"]["enum"],
        serde_json::json!(["main", "script", "handler", "lib-root", "unknown-entry"])
    );
    assert_eq!(
        props["entry_confidence"]["enum"],
        serde_json::json!(["HIGH", "MEDIUM", "LOW"])
    );
}

/// Recursive parallel walk: wherever the old schema has a `required` array,
/// the new schema must carry the identical array.
fn assert_required_equal(old: &serde_json::Value, new: &serde_json::Value, pointer: &str) {
    match (old, new) {
        (serde_json::Value::Object(old_map), serde_json::Value::Object(new_map)) => {
            if let Some(old_required) = old_map.get("required") {
                assert_eq!(
                    new_map.get("required"),
                    Some(old_required),
                    "required list changed at {pointer} — minor bumps are additive-only"
                );
            }
            for (key, old_value) in old_map {
                if let Some(new_value) = new_map.get(key) {
                    assert_required_equal(old_value, new_value, &format!("{pointer}/{key}"));
                }
            }
        }
        (serde_json::Value::Array(old_items), serde_json::Value::Array(new_items)) => {
            for (i, (o, n)) in old_items.iter().zip(new_items.iter()).enumerate() {
                assert_required_equal(o, n, &format!("{pointer}[{i}]"));
            }
        }
        _ => {}
    }
}

/// (7) v1.4 STEP 2 emission lock (plan `divergence-revamp` — supersedes the
/// step-1 skeleton "omitted" lock): on a non-empty scan every
/// `stats.functions[]` entry CARRIES the seven instrument fields with
/// schema-legal shapes — call-shape always `Some` (whole-program pass),
/// `idiom_hash` always `Some` (source-local), `error_path` always `Some`,
/// `alloc_family` present iff the function has alloc-family sites — while
/// `stats.distributions[]` stays populated as in step 1.
#[test]
fn v1_4_step2_populates_instrument_fields() {
    let root = temp_dir("skeleton");
    fs::write(root.join("c_src.c"), LOCK_C_SRC).expect("write c_src.c");
    fs::write(root.join("rs_src.rs"), LOCK_RS_SRC).expect("write rs_src.rs");
    let doc = run_pipeline_and_parse("skeleton", &root);

    let functions = doc["stats"]["functions"]
        .as_array()
        .expect("stats.functions[] present");
    assert!(!functions.is_empty());
    let is_hex16 = |v: &serde_json::Value| {
        v.as_str().is_some_and(|s| {
            s.len() == 16
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
    };
    for f in functions {
        for present in [
            "call_fanout",
            "call_fanin",
            "call_depth",
            "is_recursive",
            "idiom_hash",
            "error_path",
        ] {
            assert!(
                f.get(present).is_some(),
                "step-2 field {present} must be populated, got {f:?}"
            );
        }
        assert!(
            f["call_fanout"].as_u64().is_some(),
            "call_fanout must be a non-negative integer"
        );
        assert!(f["call_fanin"].as_u64().is_some());
        assert!(f["call_depth"].as_u64().is_some());
        assert!(f["is_recursive"].as_bool().is_some());
        assert!(
            is_hex16(&f["idiom_hash"]),
            "idiom_hash must be 16 lowercase hex"
        );
        assert!(f["error_path"].as_bool().is_some());
        if let Some(family) = f.get("alloc_family") {
            assert!(
                ["real_alloc", "empty_ctor", "serialize"].contains(&family.as_str().unwrap_or("")),
                "alloc_family outside the closed tag set: {family:?}"
            );
        }
    }

    // Lock-tree value pins: the C churn allocates via malloc (real_alloc,
    // no error vocabulary ⇒ error_path false, no callees); the Rust churn
    // builds only empty ctors (String::new + Vec::new ⇒ empty_ctor).
    let by_path = |path: &str| {
        functions
            .iter()
            .find(|f| f["path"] == path)
            .unwrap_or_else(|| panic!("function in {path} present"))
    };
    let c = by_path("c_src.c");
    assert_eq!(c["alloc_family"], "real_alloc");
    assert_eq!(c["error_path"], false);
    assert_eq!(c["call_fanout"], 0);
    assert_eq!(c["call_fanin"], 0);
    assert_eq!(c["is_recursive"], false);
    let rs = by_path("rs_src.rs");
    assert_eq!(rs["alloc_family"], "empty_ctor");
    assert_eq!(rs["error_path"], false);

    let entries = doc["stats"]["distributions"]
        .as_array()
        .expect("v1.4 populates stats.distributions[] on a non-empty scan");
    assert_eq!(entries.len(), 13, "6 complexity + 7 depth buckets");
    let keys: Vec<(String, String)> = entries
        .iter()
        .map(|e| {
            (
                e["metric"].as_str().expect("metric").to_string(),
                e["bucket"].as_str().expect("bucket").to_string(),
            )
        })
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "(metric, bucket) ascending");
    for metric in ["cyclomatic_complexity", "max_loop_depth"] {
        let sum: u64 = entries
            .iter()
            .filter(|e| e["metric"] == metric)
            .map(|e| e["count"].as_u64().expect("count"))
            .sum();
        assert_eq!(
            sum,
            functions.len() as u64,
            "per-metric sum for {metric} must equal the function count"
        );
    }

    fs::remove_dir_all(&root).ok();
}
