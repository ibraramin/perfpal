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
