//! metrics.json schema 1.2 end-to-end lock (plan `rust-backend`, phase R2):
//! `scan.languages` derives from the scanned files (sorted-unique, supported
//! set on a zero-file scan) and `files[].language` admits `"rust"`, all
//! asserted through the REAL pipeline (`cli::run_with`) over temp trees —
//! NOT inside `tests/rust_backend.rs` (whose header pins the R1 contract that
//! the schema/writer path is not exercised over Rust).

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
        "perfpal-schema12-{tag}-{}-{nanos}",
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
/// preserved; semantics = "languages scanned for").
#[test]
fn empty_dir_scan_falls_back_to_supported_set() {
    let root = temp_dir("empty");
    let doc = run_pipeline_and_parse("empty", &root);
    assert_eq!(doc["scan"]["languages"], serde_json::json!(["c", "rust"]));
    assert_eq!(doc["scan"]["file_count"], 0);
}

/// (4) Schema lock: the embedded canonical v1.2 schema admits exactly
/// `["c","rust"]` at both language sites; the retained v1.1 artifact still
/// pins its own version marker and the un-widened `const "c"` language
/// (proving the history copy is kept intact, not silently overwritten).
#[test]
fn schema_1_2_admits_c_and_rust_language_values() {
    let schema: serde_json::Value =
        serde_json::from_str(perfpal::schema::EMBEDDED_SCHEMA).expect("embedded schema parses");

    let langs_enum =
        schema["properties"]["scan"]["properties"]["languages"]["items"]["enum"].clone();
    assert_eq!(langs_enum, serde_json::json!(["c", "rust"]));

    let file_lang_enum =
        schema["properties"]["files"]["items"]["properties"]["language"]["enum"].clone();
    assert_eq!(file_lang_enum, serde_json::json!(["c", "rust"]));

    // Historical artifact retained intact.
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
// (5) R3 no-schema-bump lock (plan `rust-backend`, R3 workstream E): the
// Rust language + 23 PERF-RS-* rules arrived within v1.2 (additive); no schema
// bump. No runtime JSON-Schema validator is available in-repo — Cargo.toml is
// dependency-frozen and carries no `jsonschema`/`valico`-style crate — so the
// v1.2 `rule_id`/`finding_id` patterns and the `files[].language` enum are
// asserted STRUCTURALLY here against the bundled artifact.
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

/// Structural match for the v1.2 `rule_id` pattern
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

/// Structural match for the v1.2 `finding_id` pattern
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
fn r3_no_schema_bump_and_rs_findings_validate_structurally() {
    // R3 is additive within v1.2: the version marker and bundled artifact name
    // are unchanged (no bump, no v1.3 artifact).
    assert_eq!(perfpal::schema::SCHEMA_VERSION, "1.2");
    assert_eq!(
        perfpal::schema::SCHEMA_ARTIFACT_NAME,
        "metrics.schema.v1.2.json"
    );
    assert_eq!(
        perfpal::schema::SCHEMA_ARTIFACT_NAME,
        format!("metrics.schema.v{}.json", perfpal::schema::SCHEMA_VERSION),
        "artifact name must stay derived from SCHEMA_VERSION"
    );
    // The canonical docs artifact is present and still v1.2.
    let canonical: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.2.json"))
            .expect("canonical v1.2 schema parses");
    assert_eq!(canonical["properties"]["schema_version"]["const"], "1.2");
    assert!(
        !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/schema/metrics.schema.v1.3.json")
            .exists(),
        "R3 must not have introduced a v1.3 artifact"
    );

    // A REAL mixed-language document: Rust findings present so the structural
    // pattern lock exercises the new PERF-RS-* ids, not just PERF-C-*.
    let root = temp_dir("schema-lock");
    fs::write(root.join("c_src.c"), LOCK_C_SRC).expect("write c_src.c");
    fs::write(root.join("rs_src.rs"), LOCK_RS_SRC).expect("write rs_src.rs");
    let doc = run_pipeline_and_parse("schema-lock", &root);

    assert_eq!(doc["schema_version"], "1.2", "no schema bump in R3");

    // files[].language within the v1.2 enum.
    let files = doc["files"].as_array().expect("files[] present");
    for f in files {
        let lang = f["language"].as_str().expect("language token");
        assert!(
            matches!(lang, "c" | "rust"),
            "files[].language {lang:?} outside the v1.2 enum"
        );
    }
    // scan.languages within the same enum.
    for lang in doc["scan"]["languages"].as_array().expect("languages[]") {
        let lang = lang.as_str().expect("language token");
        assert!(
            matches!(lang, "c" | "rust"),
            "scan.languages entry {lang:?} outside the v1.2 enum"
        );
    }

    // Every finding must be a structurally valid v1.2 record AND carry the
    // language-matching id prefix; at least one PERF-RS-* must be present.
    let findings = doc["findings"].as_array().expect("findings[] present");
    assert!(!findings.is_empty(), "mixed lock tree must emit findings");
    let mut rs_seen = false;
    for f in findings {
        let rule_id = f["rule_id"].as_str().expect("rule_id");
        let finding_id = f["finding_id"].as_str().expect("finding_id");
        assert!(
            matches_rule_id(rule_id),
            "rule_id {rule_id:?} violates the v1.2 pattern"
        );
        assert!(
            matches_finding_id(finding_id),
            "finding_id {finding_id:?} violates the v1.2 pattern"
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
