//! CLI exit-code matrix (modules.md §8.2), driven end-to-end through
//! [`perfpal::cli::run_with`]. Each case cites its contract section.
//!
//! | # | Case                                        | Argv / setup                                          | Expected | Contract |
//! |---|---------------------------------------------|-------------------------------------------------------|----------|----------|
//! | 1 | Happy path                                  | scan tests/fixtures/e2e into fresh temp output         | 0        | §8.2 row 0 — report written; findings are data |
//! | 2 | Empty-dir scan (zero files)                 | empty temp dir as target                               | 0        | §8.2 row 0 + modules.md Principle 4 — valid empty report, `languages: ["c","rust"]` |
//! | 3 | `--help`                                    | `--help`                                               | 0        | §8.1 flag table — usage text, exit 0 |
//! | 4 | `--version`                                 | `--version`                                            | 0        | §8.1 flag table — version payload, exit 0 |
//! | 5 | Unknown flag                                | `--definitely-not-a-flag`                              | 1        | §8.2 row 1 — usage error (clap's default 2 is remapped) |
//! | 6 | Nonexistent root path                       | `/nonexistent/…` target                                | 1        | §8.2 row 1 — missing/unreadable target (scanner `UnreadableTarget` → `PipelineError::Usage`) |
//! | 7 | Safety-guard refusal                        | pre-created output dir WITHOUT metrics.json, no `--force` | 2     | §8.2 row 2 — safety-guard refusal (output-contract §2) |
//! | 8 | Safety-guard override                       | same setup + `--force`                                 | 0        | output-contract §2 — force replaces the non-report dir |
//! | 9 | Overwrite of a VALID existing report        | metrics.json present, no `--force`                     | 0        | output-contract §2 — atomic regeneration of own reports needs no force |
//! |10 | Panic → internal error                      | unit test `guarded_maps_ok_err_and_panic_to_contract_codes` in src/cli/mod.rs (via the `guarded` seam) | 3 | §8.2 row 3 — unexpected panic is always a bug |
//!
//! Case 10 lives as a `#[cfg(test)]` unit test inside src/cli/mod.rs because
//! the `guarded` seam is deliberately private (least public surface); the
//! panic cannot be triggered naturally through the pipeline. Cases 1–9 here
//! cover every externally reachable code through the public entry point.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-exit-matrix-{tag}-{}-{nanos}",
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

/// Best-effort cleanup; assertion failures must not mask the original code.
fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn case1_happy_path_exit_0() {
    // §8.2 row 0: success — report written; findings never affect the code.
    let root = temp_dir("happy");
    let out = root.join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);
    assert!(out.join("metrics.json").is_file());
    cleanup(&root);
}

#[test]
fn case2_empty_dir_scan_exit_0_with_valid_empty_report() {
    // §8.2 row 0 + Principle 4: a zero-file scan succeeds with a valid
    // empty report (`languages: ["c","rust"]` — the supported set).
    let root = temp_dir("empty");
    let target = root.join("no-c-files-here");
    fs::create_dir_all(&target).expect("empty scan target");
    let out = root.join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        target.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "zero-file scan must succeed (Principle 4)");
    let doc: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("metrics.json")).unwrap())
            .expect("valid empty report");
    assert_eq!(doc["scan"]["languages"], serde_json::json!(["c", "rust"]));
    assert_eq!(doc["scan"]["file_count"], 0);
    cleanup(&root);
}

#[test]
fn case3_help_exit_0() {
    // §8.1: usage text, exit 0 (NOT clap's default 2).
    assert_eq!(run_with(argv(&["--help"])), 0);
}

#[test]
fn case4_version_exit_0() {
    // §8.1: version payload, exit 0.
    assert_eq!(run_with(argv(&["--version"])), 0);
}

#[test]
fn case5_unknown_flag_exit_1() {
    // §8.2 row 1: unknown flag is a usage error; clap's default exit 2 is
    // remapped to 1 so output errors own exit 2 exclusively.
    let root = temp_dir("flag");
    let code = run_with(argv(&[
        "--definitely-not-a-flag",
        "--output",
        root.join("r").to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 1);
    cleanup(&root);
}

#[test]
fn case6_nonexistent_root_exit_1() {
    // §8.2 row 1: "missing/unreadable target path". Verified contract: the
    // scanner raises ScanError::UnreadableTarget and run_pipeline maps
    // scanner errors to PipelineError::Usage ⇒ exit 1. No discrepancy.
    let root = temp_dir("missing");
    let code = run_with(argv(&[
        "--output",
        root.join("r").to_string_lossy().as_ref(),
        "/nonexistent/perfpal-exit-matrix-target-xyz",
    ]));
    assert_eq!(code, 1);
    cleanup(&root);
}

#[test]
fn case7_safety_guard_refusal_exit_2() {
    // §8.2 row 2 / output-contract §2: an existing directory without
    // metrics.json is never silently destroyed without --force.
    let root = temp_dir("guard");
    let out = root.join("occupied");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("user-data.txt"), "precious\n").unwrap();
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 2, "safety-guard refusal without --force ⇒ exit 2");
    assert_eq!(
        fs::read_to_string(out.join("user-data.txt")).unwrap(),
        "precious\n",
        "guarded directory must be untouched"
    );
    cleanup(&root);
}

#[test]
fn case8_safety_guard_override_with_force_exit_0() {
    // output-contract §2: --force overrides the guard over a non-report dir.
    let root = temp_dir("force");
    let out = root.join("not-a-report");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("user-data.txt"), "precious\n").unwrap();
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        "--force",
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0);
    assert!(out.join("metrics.json").is_file());
    assert!(!out.join("user-data.txt").exists());
    cleanup(&root);
}

#[test]
fn case9_overwrite_valid_report_without_force_exit_0() {
    // output-contract §2: re-running over a VALID existing report (own
    // metrics.json present) is atomic regeneration — no --force needed.
    let root = temp_dir("overwrite");
    let out = root.join("perfpal-report");
    let out_str = out.to_string_lossy().into_owned();
    let code = run_with(argv(&[
        "--output",
        &out_str,
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "seeding run must succeed");
    let code = run_with(argv(&[
        "--output",
        &out_str,
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "overwrite of a valid report needs no --force");
    assert!(out.join("metrics.json").is_file());
    cleanup(&root);
}
