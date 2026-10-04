//! metrics.json schema 1.3 end-to-end lock (plan `python-backend`): the
//! `"python"` language token was admitted alongside `"c"` / `"rust"` (minor
//! bump 1.2 → 1.3, additive). `scan.languages` derives from the scanned files
//! (sorted-unique, supported set on a zero-file scan) and `files[].language`
//! admits `"python"`, all asserted through the REAL pipeline (`cli::run_with`)
//! over temp trees — NOT inside a language-specific backend test (whose header
//! pins the contract that the schema/writer path is not exercised there).

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
/// preserved; semantics = "languages scanned for"). With the 1.3 bump the
/// supported set is `["c","python","rust"]` (the `SUPPORTED_LANGUAGES` order).
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

/// (4) Schema lock: the embedded canonical v1.3 schema admits exactly
/// `["c","rust","python"]` at both language sites; the retained v1.2 and v1.1
/// artifacts still pin their own version markers and un-widened language
/// values (proving the history copies are kept intact, not silently
/// overwritten).
#[test]
fn schema_1_3_admits_c_rust_and_python_language_values() {
    let schema: serde_json::Value =
        serde_json::from_str(perfpal::schema::EMBEDDED_SCHEMA).expect("embedded schema parses");

    let langs_enum =
        schema["properties"]["scan"]["properties"]["languages"]["items"]["enum"].clone();
    assert_eq!(langs_enum, serde_json::json!(["c", "rust", "python"]));

    let file_lang_enum =
        schema["properties"]["files"]["items"]["properties"]["language"]["enum"].clone();
    assert_eq!(file_lang_enum, serde_json::json!(["c", "rust", "python"]));

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
// (5) v1.3 bump lock (plan `python-backend`): the Python language arrived as
// an additive minor bump 1.2 → 1.3. No runtime JSON-Schema validator is
// available in-repo — Cargo.toml is dependency-frozen and carries no
// `jsonschema`/`valico`-style crate — so the v1.3 `rule_id`/`finding_id`
// patterns and the `files[].language` enum are asserted STRUCTURALLY here
// against the bundled artifact.
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

/// Structural match for the v1.3 `rule_id` pattern
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

/// Structural match for the v1.3 `finding_id` pattern
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
fn v1_3_bump_and_findings_validate_structurally() {
    // v1.3 is an additive minor bump: the version marker and bundled artifact
    // name advance together, and v1.2 remains on disk as retained history.
    assert_eq!(perfpal::schema::SCHEMA_VERSION, "1.3");
    assert_eq!(
        perfpal::schema::SCHEMA_ARTIFACT_NAME,
        "metrics.schema.v1.3.json"
    );
    assert_eq!(
        perfpal::schema::SCHEMA_ARTIFACT_NAME,
        format!("metrics.schema.v{}.json", perfpal::schema::SCHEMA_VERSION),
        "artifact name must stay derived from SCHEMA_VERSION"
    );
    // The canonical v1.3 docs artifact is present and active.
    let canonical: serde_json::Value =
        serde_json::from_str(include_str!("../docs/schema/metrics.schema.v1.3.json"))
            .expect("canonical v1.3 schema parses");
    assert_eq!(canonical["properties"]["schema_version"]["const"], "1.3");
    // v1.2 is retained as history — the bump did not overwrite it.
    assert!(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/schema/metrics.schema.v1.2.json")
            .exists(),
        "v1.2 artifact must remain as retained history"
    );
    // The active v1.3 artifact file IS emitted.
    assert!(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/schema/metrics.schema.v1.3.json")
            .exists(),
        "the v1.3 artifact must be emitted by this bump"
    );

    // A REAL mixed-language document: Rust findings present so the structural
    // pattern lock exercises the new PERF-RS-* ids, not just PERF-C-*.
    let root = temp_dir("schema-lock");
    fs::write(root.join("c_src.c"), LOCK_C_SRC).expect("write c_src.c");
    fs::write(root.join("rs_src.rs"), LOCK_RS_SRC).expect("write rs_src.rs");
    let doc = run_pipeline_and_parse("schema-lock", &root);

    assert_eq!(doc["schema_version"], "1.3", "v1.3 bump in effect");

    // files[].language within the v1.3 enum.
    let files = doc["files"].as_array().expect("files[] present");
    for f in files {
        let lang = f["language"].as_str().expect("language token");
        assert!(
            matches!(lang, "c" | "rust" | "python"),
            "files[].language {lang:?} outside the v1.3 enum"
        );
    }
    // scan.languages within the same enum.
    for lang in doc["scan"]["languages"].as_array().expect("languages[]") {
        let lang = lang.as_str().expect("language token");
        assert!(
            matches!(lang, "c" | "rust" | "python"),
            "scan.languages entry {lang:?} outside the v1.3 enum"
        );
    }

    // Every finding must be a structurally valid v1.3 record AND carry the
    // language-matching id prefix; at least one PERF-RS-* must be present.
    let findings = doc["findings"].as_array().expect("findings[] present");
    assert!(!findings.is_empty(), "mixed lock tree must emit findings");
    let mut rs_seen = false;
    for f in findings {
        let rule_id = f["rule_id"].as_str().expect("rule_id");
        let finding_id = f["finding_id"].as_str().expect("finding_id");
        assert!(
            matches_rule_id(rule_id),
            "rule_id {rule_id:?} violates the v1.3 pattern"
        );
        assert!(
            matches_finding_id(finding_id),
            "finding_id {finding_id:?} violates the v1.3 pattern"
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
